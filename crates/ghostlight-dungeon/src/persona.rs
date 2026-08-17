use crate::model::{ModelPort, ModelStageRequest, run_validated_stage};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use ghostlight_persona_projection::{
    InterpreterPrompt, PersonaPrompt, ProjectorPrompt, build_interpreter_prompt,
    build_persona_prompt, build_projector_prompt, narrative_stream_is_clean,
};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PermittedActorSlice {
    pub actor_id: String,
    pub location_id: String,
    pub snapshot_binding: String,
    pub identity_experience: Vec<String>,
    pub memories: Vec<String>,
    pub perceived_events: Vec<String>,
    pub perceived_actors: std::collections::BTreeMap<String, String>,
    pub relationships: Vec<String>,
    pub goals: Vec<String>,
    pub knowledge: Vec<String>,
    pub capabilities: Vec<String>,
    pub pressures: Vec<String>,
    pub affordances: Vec<String>,
    pub source_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LivedNarrativeStream {
    pub text: String,
    pub snapshot_binding: String,
    pub projector_receipt: crate::model::ModelStageReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PersonaProposalBundle {
    pub private_delta: crate::domain::ActorStateDelta,
    pub speech: Option<String>,
    pub reaction_priority: i16,
    pub world_actions: Vec<crate::domain::WorldActionProposal>,
}

#[derive(Clone, Debug)]
pub struct PersonaTerminalBundle {
    pub lived_stream: LivedNarrativeStream,
    pub persona_output: String,
    pub proposals: PersonaProposalBundle,
    pub stage_receipts: Vec<crate::model::ModelStageReceipt>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CellConstituentSlice {
    pub subject_id: String,
    pub subject_kind: crate::domain::AgencySubjectKind,
    pub name: String,
    pub collective_authority_id: Option<String>,
    pub location_ids: BTreeSet<String>,
    pub knowledge: BTreeSet<String>,
    pub capabilities: BTreeSet<String>,
    pub resources: BTreeSet<String>,
    pub information_channels: BTreeSet<String>,
    pub permitted_state_references: BTreeSet<String>,
    pub reachable_destination_ids: BTreeSet<String>,
    pub goals: Vec<String>,
    pub pressures: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PermittedCellSlice {
    pub cell_id: String,
    pub mode: crate::domain::SimulationCellMode,
    pub world_revision: u64,
    pub resolution_epoch: u64,
    pub snapshot_binding: String,
    pub constituents: Vec<CellConstituentSlice>,
    pub shared_knowledge: BTreeSet<String>,
    pub shared_capabilities: BTreeSet<String>,
    pub perceived_events: Vec<String>,
    pub world_clock_pressure: Vec<String>,
    pub detail_focus_subject_id: Option<String>,
    pub max_actions: usize,
    pub source_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CellTerminalBundle {
    pub lived_stream: LivedNarrativeStream,
    pub persona_output: String,
    pub appraisal: crate::domain::CellAppraisal,
    pub stage_receipts: Vec<crate::model::ModelStageReceipt>,
}

#[async_trait]
pub trait ExecutionPermit: Send + Sync {
    async fn require(&self, actor_id: &str, snapshot_binding: &str, stage: &str) -> Result<()>;
}

#[derive(Clone)]
pub struct AllowAllPermit;
#[async_trait]
impl ExecutionPermit for AllowAllPermit {
    async fn require(&self, _: &str, _: &str, _: &str) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct PersonaProjectionEngine {
    pub model: Arc<dyn ModelPort>,
    pub permit: Arc<dyn ExecutionPermit>,
    pub projector_model: String,
    pub persona_model: String,
    pub interpreter_model: String,
}

impl PersonaProjectionEngine {
    pub async fn execute(&self, slice: PermittedActorSlice) -> Result<PersonaTerminalBundle> {
        self.permit
            .require(&slice.actor_id, &slice.snapshot_binding, "projector")
            .await?;
        let typed_context = serde_json::to_string(&slice)?;
        let visible_stimulus = slice.perceived_events.join("\n");
        let projector_prompt = build_projector_prompt(&ProjectorPrompt {
            identity: &slice.actor_id,
            typed_context: &typed_context,
            visible_stimulus: &visible_stimulus,
            domain_guidance: "The runtime is a persistent fictional world. Distances, knowledge, capabilities, relationships, and custody remain bounded by the supplied slice.",
        });
        let projected = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "projector".into(),
                model: self.projector_model.clone(),
                snapshot_binding: slice.snapshot_binding.clone(),
                lived_stream: projector_prompt,
                output_schema: None,
                source_receipt_ids: slice.source_receipt_ids.clone(),
            },
        )
        .await?;
        if !narrative_stream_is_clean(&projected.narrative) {
            return Err(anyhow!("projector violated lived-narrative membrane"));
        }
        let lived = LivedNarrativeStream {
            text: projected.narrative.clone(),
            snapshot_binding: slice.snapshot_binding.clone(),
            projector_receipt: projected.receipt.clone(),
        };

        self.permit
            .require(&slice.actor_id, &slice.snapshot_binding, "persona")
            .await?;
        // This is the membrane: the Persona's domain context is exactly one lived stream.
        let persona = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "persona".into(),
                model: self.persona_model.clone(),
                snapshot_binding: slice.snapshot_binding.clone(),
                lived_stream: build_persona_prompt(&PersonaPrompt {
                    identity: &slice.actor_id,
                    lived_stream: &lived.text,
                    domain_guidance: "Respond as a situated character. Speech and attempted effects are distinct; the world kernel resolves consequences.",
                }),
                output_schema: None,
                source_receipt_ids: vec![],
            },
        )
        .await?;

        self.permit
            .require(&slice.actor_id, &slice.snapshot_binding, "interpreter")
            .await?;
        let mut schema = serde_json::to_value(schema_for!(PersonaProposalBundle))?;
        constrain_interpreter_schema(&mut schema, &slice)?;
        let interpreted = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "interpreter".into(),
                model: self.interpreter_model.clone(),
                snapshot_binding: slice.snapshot_binding.clone(),
                lived_stream: build_interpreter_prompt(&InterpreterPrompt {
                    identity: &slice.actor_id,
                    typed_context: &typed_context,
                    lived_stream: &lived.text,
                    persona_output: &persona.narrative,
                    output_schema: &serde_json::to_string_pretty(&schema)?,
                    domain_guidance: "Record only private changes supported by the lived stream and typed context. World actions are attempts, not completed effects. Use only exact actor ids and state-reference tokens admitted by the output schema.",
                }),
                output_schema: Some(schema),
                source_receipt_ids: slice.source_receipt_ids,
            },
        )
        .await?;
        self.permit
            .require(&slice.actor_id, &slice.snapshot_binding, "terminal")
            .await?;
        let proposals: PersonaProposalBundle = serde_json::from_value(
            interpreted
                .structured
                .clone()
                .ok_or_else(|| anyhow!("interpreter produced no typed proposal"))?,
        )?;
        Ok(PersonaTerminalBundle {
            lived_stream: lived,
            persona_output: persona.narrative,
            proposals,
            stage_receipts: vec![projected.receipt, persona.receipt, interpreted.receipt],
        })
    }
}

#[derive(Clone)]
pub struct CellProjectionEngine {
    pub model: Arc<dyn ModelPort>,
    pub permit: Arc<dyn ExecutionPermit>,
    pub projector_model: String,
    pub persona_model: String,
    pub interpreter_model: String,
}

impl CellProjectionEngine {
    pub async fn execute(&self, slice: PermittedCellSlice) -> Result<CellTerminalBundle> {
        self.permit
            .require(&slice.cell_id, &slice.snapshot_binding, "cell_projector")
            .await?;
        let typed_context = serde_json::to_string(&slice)?;
        let visible_stimulus = slice.perceived_events.join("\n");
        let mode_guidance = match slice.mode {
            crate::domain::SimulationCellMode::Cohesive => {
                "This cell has real collective authority. Render a plural lived perspective from genuinely shared knowledge and capability only; describe constituent exceptions as exceptions."
            }
            crate::domain::SimulationCellMode::Arena => {
                "This cell is an arena, never a person or faction. Render an attributed polyphonic situation. Never union secrets, knowledge, resources, intentions, authority, or voice between constituents."
            }
        };
        let projected = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "cell_projector".into(),
                model: self.projector_model.clone(),
                snapshot_binding: slice.snapshot_binding.clone(),
                lived_stream: build_projector_prompt(&ProjectorPrompt {
                    identity: &slice.cell_id,
                    typed_context: &typed_context,
                    visible_stimulus: &visible_stimulus,
                    domain_guidance: mode_guidance,
                }),
                output_schema: None,
                source_receipt_ids: slice.source_receipt_ids.clone(),
            },
        )
        .await?;
        if !narrative_stream_is_clean(&projected.narrative) {
            return Err(anyhow!("cell projector violated lived-narrative membrane"));
        }
        let lived = LivedNarrativeStream {
            text: projected.narrative.clone(),
            snapshot_binding: slice.snapshot_binding.clone(),
            projector_receipt: projected.receipt.clone(),
        };
        self.permit
            .require(&slice.cell_id, &slice.snapshot_binding, "cell_persona")
            .await?;
        let persona = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "cell_persona".into(),
                model: self.persona_model.clone(),
                snapshot_binding: slice.snapshot_binding.clone(),
                lived_stream: build_persona_prompt(&PersonaPrompt {
                    identity: &slice.cell_id,
                    lived_stream: &lived.text,
                    domain_guidance: match slice.mode {
                        crate::domain::SimulationCellMode::Cohesive => {
                            "Appraise the strategic horizon as a real collective. Action is optional, but inaction must be intentional. Do not invent completed consequences."
                        }
                        crate::domain::SimulationCellMode::Arena => {
                            "Appraise the strategic horizon polyphonically. Attribute every intention to a constituent. The arena itself cannot speak, know, decide, or act. Action is optional, but inaction must be explicit."
                        }
                    },
                }),
                output_schema: None,
                source_receipt_ids: vec![],
            },
        )
        .await?;
        self.permit
            .require(&slice.cell_id, &slice.snapshot_binding, "cell_interpreter")
            .await?;
        let mut schema = serde_json::to_value(schema_for!(crate::domain::CellAppraisal))?;
        constrain_cell_schema(&mut schema, &slice)?;
        let mut request = ModelStageRequest {
            stage: "cell_interpreter".into(),
            model: self.interpreter_model.clone(),
            snapshot_binding: slice.snapshot_binding.clone(),
            lived_stream: build_interpreter_prompt(&InterpreterPrompt {
                identity: &slice.cell_id,
                typed_context: &typed_context,
                lived_stream: &lived.text,
                persona_output: &persona.narrative,
                output_schema: &serde_json::to_string_pretty(&schema)?,
                domain_guidance: "Emit only exact constituent-attributed attempts supported by that constituent's typed state. The cell id is not an actor id. Use an empty actions array plus a concrete inaction_reason when nobody acts.",
            }),
            output_schema: Some(schema),
            source_receipt_ids: slice.source_receipt_ids.clone(),
        };
        let mut stage_receipts = vec![projected.receipt, persona.receipt];
        for attempt in 0..2 {
            let mut interpreted = run_validated_stage(self.model.as_ref(), &request).await?;
            let appraisal = interpreted
                .structured
                .clone()
                .ok_or_else(|| anyhow!("cell interpreter produced no typed appraisal"))
                .and_then(|value| serde_json::from_value(value).map_err(Into::into));
            match appraisal.and_then(|appraisal| {
                validate_cell_appraisal(&slice, &appraisal)?;
                Ok(appraisal)
            }) {
                Ok(appraisal) => {
                    stage_receipts.push(interpreted.receipt);
                    self.permit
                        .require(&slice.cell_id, &slice.snapshot_binding, "cell_terminal")
                        .await?;
                    return Ok(CellTerminalBundle {
                        lived_stream: lived,
                        persona_output: persona.narrative,
                        appraisal,
                        stage_receipts,
                    });
                }
                Err(error) if attempt == 0 => {
                    interpreted.receipt.validation_result = "semantic_invalid".into();
                    stage_receipts.push(interpreted.receipt);
                    request.lived_stream.push_str(&format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS APPRAISAL: {error}\nReturn one corrected complete appraisal against the same snapshot, lived stream, and Persona turn."
                    ));
                }
                Err(error) => {
                    return Err(anyhow!(
                        "cell interpreter failed semantic validation after one correction: {error}"
                    ));
                }
            }
        }
        unreachable!()
    }
}

fn validate_cell_appraisal(
    slice: &PermittedCellSlice,
    appraisal: &crate::domain::CellAppraisal,
) -> Result<()> {
    let expected: BTreeSet<_> = slice
        .constituents
        .iter()
        .map(|value| value.subject_id.clone())
        .collect();
    if appraisal.schema != "ghostlight.cell_appraisal.v1"
        || appraisal.cell_id != slice.cell_id
        || appraisal.world_revision != slice.world_revision
        || appraisal.resolution_epoch != slice.resolution_epoch
        || appraisal.considered_subject_ids != expected
        || appraisal.actions.len() > slice.max_actions
        || (appraisal.actions.is_empty()
            && appraisal
                .inaction_reason
                .as_deref()
                .is_none_or(str::is_empty))
    {
        return Err(anyhow!(
            "appraisal does not bind the complete permitted cell"
        ));
    }
    for action in &appraisal.actions {
        let subject = slice
            .constituents
            .iter()
            .find(|value| value.subject_id == action.subject_id)
            .ok_or_else(|| anyhow!("action is attributed outside the cell"))?;
        if action.intent.trim().is_empty()
            || action.intended_effect.trim().is_empty()
            || action
                .state_references
                .iter()
                .any(|reference| !subject.permitted_state_references.contains(reference))
            || action
                .public_channels
                .iter()
                .any(|channel| !subject.information_channels.contains(channel))
        {
            return Err(anyhow!(
                "action borrows state or information across constituents"
            ));
        }
        match &action.effect {
            crate::domain::StrategicCellEffect::Institution {
                institution_id,
                location_ids,
                ..
            } if subject.subject_kind == crate::domain::AgencySubjectKind::Institution
                && institution_id == &subject.subject_id
                && location_ids
                    .iter()
                    .all(|location| subject.location_ids.contains(location)) => {}
            crate::domain::StrategicCellEffect::Gestalt { gestalt_id, .. }
                if subject.subject_kind == crate::domain::AgencySubjectKind::Gestalt
                    && gestalt_id == &subject.subject_id => {}
            crate::domain::StrategicCellEffect::ActorMove {
                actor_id,
                destination_id,
            } if subject.subject_kind == crate::domain::AgencySubjectKind::Actor
                && actor_id == &subject.subject_id
                && subject.reachable_destination_ids.contains(destination_id) => {}
            _ => {
                return Err(anyhow!(
                    "action effect exceeds its exact constituent authority"
                ));
            }
        }
    }
    Ok(())
}

fn constrain_cell_schema(schema: &mut serde_json::Value, slice: &PermittedCellSlice) -> Result<()> {
    let properties = schema
        .pointer_mut("/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("cell appraisal schema has no properties"))?;
    properties.insert(
        "schema".into(),
        serde_json::json!({"const":"ghostlight.cell_appraisal.v1"}),
    );
    properties.insert("cell_id".into(), serde_json::json!({"const":slice.cell_id}));
    properties.insert(
        "world_revision".into(),
        serde_json::json!({"const":slice.world_revision}),
    );
    properties.insert(
        "resolution_epoch".into(),
        serde_json::json!({"const":slice.resolution_epoch}),
    );
    properties.insert(
        "considered_subject_ids".into(),
        serde_json::json!({
            "type":"array",
            "uniqueItems":true,
            "minItems":slice.constituents.len(),
            "maxItems":slice.constituents.len(),
            "items":{"type":"string","enum":slice.constituents.iter().map(|value| &value.subject_id).collect::<Vec<_>>()}
        }),
    );
    let actions = properties
        .get_mut("actions")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("cell appraisal schema has no action array"))?;
    actions.insert("maxItems".into(), slice.max_actions.into());
    let proposal = schema
        .pointer_mut("/$defs/CellActionProposal/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("cell appraisal schema has no proposal properties"))?;
    proposal.insert(
        "subject_id".into(),
        serde_json::json!({"type":"string","enum":slice.constituents.iter().map(|value| &value.subject_id).collect::<Vec<_>>() }),
    );
    Ok(())
}

fn constrain_interpreter_schema(
    schema: &mut serde_json::Value,
    slice: &PermittedActorSlice,
) -> Result<()> {
    let allowed_references = slice
        .capabilities
        .iter()
        .map(|value| format!("capability:{value}"))
        .chain(
            slice
                .knowledge
                .iter()
                .map(|value| format!("knowledge:{value}")),
        )
        .chain(
            slice
                .affordances
                .iter()
                .map(|value| format!("equipment:{value}")),
        )
        .chain(std::iter::once(format!("location:{}", slice.location_id)))
        .collect::<Vec<_>>();
    let world_action = schema
        .pointer_mut("/$defs/WorldActionProposal/properties")
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| anyhow!("Persona proposal schema has no world action properties"))?;
    world_action.insert(
        "actor_id".into(),
        serde_json::json!({"const":slice.actor_id}),
    );
    world_action.insert(
        "state_references".into(),
        serde_json::json!({"type":"array","items":{"type":"string","enum":allowed_references}}),
    );
    let relationship_updates = schema
        .pointer_mut("/$defs/ActorStateDelta/properties/relationship_updates")
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| anyhow!("Persona proposal schema has no relationship updates"))?;
    relationship_updates.insert(
        "propertyNames".into(),
        serde_json::json!({"enum":slice.perceived_actors.keys().collect::<Vec<_>>()}),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FixtureModel;
    #[tokio::test]
    async fn persona_receives_only_projected_stream() {
        let engine = PersonaProjectionEngine {
            model: Arc::new(FixtureModel),
            permit: Arc::new(AllowAllPermit),
            projector_model: "flash".into(),
            persona_model: "pro".into(),
            interpreter_model: "flash".into(),
        };
        let slice = PermittedActorSlice {
            actor_id: "npc".into(),
            location_id: "room".into(),
            snapshot_binding: "campaign:1".into(),
            identity_experience: vec!["A tired navigator".into()],
            memories: vec![],
            perceived_events: vec![],
            perceived_actors: std::collections::BTreeMap::from([(
                "player".into(),
                "Player".into(),
            )]),
            relationships: vec![],
            goals: vec![],
            knowledge: vec![],
            capabilities: vec![],
            pressures: vec![],
            affordances: vec![],
            source_receipt_ids: vec![],
        };
        let result = engine.execute(slice).await.unwrap();
        assert_eq!(result.stage_receipts.len(), 3);
        assert_eq!(result.proposals.reaction_priority, 0);
    }
}

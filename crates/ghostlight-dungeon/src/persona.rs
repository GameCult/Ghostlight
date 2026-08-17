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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellAppraisalProposal {
    actions: Vec<crate::domain::CellActionProposal>,
    inaction_reason: Option<String>,
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
            word_budget: 140,
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
                temperature: Some(0.0),
                max_output_tokens: Some(256),
            },
        )
        .await?;
        if !narrative_stream_is_clean(&projected.narrative) {
            return Err(anyhow!("projector violated lived-narrative membrane"));
        }
        let lived = LivedNarrativeStream {
            text: ground_actor_lived_stream(&slice, &projected.narrative),
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
                    domain_guidance: "Respond as a situated character. Answer direct questions at human conversational length. Speech and attempted effects are distinct; the world kernel resolves consequences. Asking, inviting, persuading, threatening, or demanding completes only your own speech: never supply the other person's answer, choice, consent, belief, disclosure, or obedience.",
                    word_budget: 160,
                }),
                output_schema: None,
                source_receipt_ids: vec![],
                temperature: Some(0.7),
                max_output_tokens: Some(256),
            },
        )
        .await?;

        self.permit
            .require(&slice.actor_id, &slice.snapshot_binding, "interpreter")
            .await?;
        let prompt_schema = serde_json::to_value(schema_for!(PersonaProposalBundle))?;
        let mut schema = prompt_schema.clone();
        constrain_interpreter_schema(&mut schema, &slice)?;
        let permission_guidance = actor_interpreter_guidance(&slice);
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
                    output_schema: &serde_json::to_string(&prompt_schema)?,
                    domain_guidance: &permission_guidance,
                }),
                output_schema: Some(schema),
                source_receipt_ids: slice.source_receipt_ids,
                temperature: Some(0.0),
                max_output_tokens: Some(768),
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

fn actor_interpreter_guidance(slice: &PermittedActorSlice) -> String {
    format!(
        "Record only private changes supported by the lived stream and typed context. World actions are attempts, not completed effects. Speech is extracted separately and is already complete. Do not emit a world action merely to make another actor answer, choose, consent, believe, disclose, feel, or obey; the other actor retains agency and any requested response remains unresolved. actor_id must be {:?}. Exact allowed state references are {:?}. Relationship update keys may only be {:?}.",
        slice.actor_id,
        allowed_actor_references(slice),
        slice.perceived_actors.keys().collect::<Vec<_>>()
    )
}

fn ground_actor_lived_stream(slice: &PermittedActorSlice, projection: &str) -> String {
    let reliable_knowledge = if slice.knowledge.is_empty() {
        "no additional external fact beyond what is happening in front of you".to_owned()
    } else {
        slice.knowledge.join("; ")
    };
    let visible_now = if slice.perceived_events.is_empty() {
        "nothing new beyond the immediate situation".to_owned()
    } else {
        slice.perceived_events.join("; ")
    };
    let people_now = if slice.perceived_actors.is_empty() {
        "no one else clearly perceived".to_owned()
    } else {
        slice
            .perceived_actors
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{projection}\n\nYour reliable footing in this moment is narrow. What you know as external fact: {reliable_knowledge}. What is happening now: {visible_now}. People you can presently perceive: {people_now}. Everything else in your impressions is feeling, inference, uncertainty, or possibility—not a remembered or witnessed fact."
    )
}

fn cell_projector_context(slice: &PermittedCellSlice) -> serde_json::Value {
    serde_json::json!({
        "cell_id": slice.cell_id,
        "mode": slice.mode,
        "world_revision": slice.world_revision,
        "resolution_epoch": slice.resolution_epoch,
        "constituents": slice.constituents.iter().map(|subject| serde_json::json!({
            "subject_id": subject.subject_id,
            "subject_kind": subject.subject_kind,
            "name": subject.name,
            "collective_authority_id": subject.collective_authority_id,
            "location_ids": subject.location_ids,
            "knowledge": subject.knowledge,
            "capabilities": subject.capabilities,
            "resources": subject.resources,
            "information_channels": subject.information_channels,
            "reachable_destination_ids": subject.reachable_destination_ids,
            "goals": subject.goals,
            "pressures": subject.pressures,
        })).collect::<Vec<_>>(),
        "shared_knowledge": slice.shared_knowledge,
        "shared_capabilities": slice.shared_capabilities,
        "world_clock_pressure": slice.world_clock_pressure,
        "detail_focus_subject_id": slice.detail_focus_subject_id,
        "max_actions": slice.max_actions,
    })
}

fn cell_interpreter_context(slice: &PermittedCellSlice) -> serde_json::Value {
    serde_json::json!({
        "cell_id": slice.cell_id,
        "mode": slice.mode,
        "world_revision": slice.world_revision,
        "resolution_epoch": slice.resolution_epoch,
        "detail_focus_subject_id": slice.detail_focus_subject_id,
        "max_actions": slice.max_actions,
        "exact_permissions": slice.constituents.iter().map(|subject| serde_json::json!({
            "subject_id": subject.subject_id,
            "subject_kind": subject.subject_kind,
            "collective_authority_id": subject.collective_authority_id,
            "location_ids": subject.location_ids,
            "information_channels": subject.information_channels,
            "permitted_state_references": subject.permitted_state_references,
            "reachable_destination_ids": subject.reachable_destination_ids,
        })).collect::<Vec<_>>(),
    })
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
        let projector_context = serde_json::to_string(&cell_projector_context(&slice))?;
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
                    typed_context: &projector_context,
                    visible_stimulus: &visible_stimulus,
                    domain_guidance: mode_guidance,
                    word_budget: (120 + 45 * slice.constituents.len()).min(360),
                }),
                output_schema: None,
                source_receipt_ids: slice.source_receipt_ids.clone(),
                temperature: Some(0.0),
                max_output_tokens: Some(640),
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
                    word_budget: (160 + 30 * slice.constituents.len()).min(320),
                }),
                output_schema: None,
                source_receipt_ids: vec![],
                temperature: Some(0.7),
                max_output_tokens: Some(512),
            },
        )
        .await?;
        self.permit
            .require(&slice.cell_id, &slice.snapshot_binding, "cell_interpreter")
            .await?;
        let prompt_schema = serde_json::to_value(schema_for!(CellAppraisalProposal))?;
        let mut schema = prompt_schema.clone();
        constrain_cell_proposal_schema(&mut schema, &slice)?;
        let interpreter_context = serde_json::to_string(&cell_interpreter_context(&slice))?;
        let permission_guidance = format!(
            "Emit at most {} exact constituent-attributed attempts supported by that constituent's permission references. The runtime, not you, binds cell identity, revisions, and complete membership. The cell id is not an actor id. Use an empty actions array plus a concrete inaction_reason when nobody acts.",
            slice.max_actions
        );
        let mut request = ModelStageRequest {
            stage: "cell_interpreter".into(),
            model: self.interpreter_model.clone(),
            snapshot_binding: slice.snapshot_binding.clone(),
            lived_stream: build_interpreter_prompt(&InterpreterPrompt {
                identity: &slice.cell_id,
                typed_context: &interpreter_context,
                lived_stream: &lived.text,
                persona_output: &persona.narrative,
                output_schema: &serde_json::to_string(&prompt_schema)?,
                domain_guidance: &permission_guidance,
            }),
            output_schema: Some(schema),
            source_receipt_ids: slice.source_receipt_ids.clone(),
            temperature: Some(0.0),
            max_output_tokens: Some(1_600),
        };
        let mut stage_receipts = vec![projected.receipt, persona.receipt];
        for attempt in 0..2 {
            let mut interpreted = run_validated_stage(self.model.as_ref(), &request).await?;
            let proposal = interpreted
                .structured
                .clone()
                .ok_or_else(|| anyhow!("cell interpreter produced no typed proposal"))
                .and_then(|value| serde_json::from_value(value).map_err(Into::into));
            match proposal.and_then(|proposal: CellAppraisalProposal| {
                let appraisal = bind_cell_appraisal(&slice, proposal);
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
                    let rejected_appraisal = interpreted
                        .structured
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()?
                        .unwrap_or_else(|| "null".into());
                    interpreted.receipt.validation_result = "semantic_invalid".into();
                    interpreted.receipt.local_validation_error =
                        Some(error.to_string().chars().take(1_000).collect());
                    stage_receipts.push(interpreted.receipt);
                    request.lived_stream.push_str(&format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS APPRAISAL: {error}\nPREVIOUS_REJECTED_APPRAISAL:\n{rejected_appraisal}\nReturn one corrected complete appraisal against the same snapshot, lived stream, Persona turn, and exact permission context. Change or remove every action that exceeds its attributed constituent's permissions; explicit inaction is valid."
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

fn bind_cell_appraisal(
    slice: &PermittedCellSlice,
    proposal: CellAppraisalProposal,
) -> crate::domain::CellAppraisal {
    crate::domain::CellAppraisal {
        schema: "ghostlight.cell_appraisal.v1".into(),
        cell_id: slice.cell_id.clone(),
        world_revision: slice.world_revision,
        resolution_epoch: slice.resolution_epoch,
        considered_subject_ids: slice
            .constituents
            .iter()
            .map(|subject| subject.subject_id.clone())
            .collect(),
        actions: proposal.actions,
        inaction_reason: proposal.inaction_reason,
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
    {
        return Err(anyhow!(
            "appraisal has a stale or incomplete runtime binding"
        ));
    }
    if appraisal.actions.len() > slice.max_actions {
        return Err(anyhow!(
            "appraisal emitted {} actions but this cell permits at most {}",
            appraisal.actions.len(),
            slice.max_actions
        ));
    }
    if appraisal.actions.is_empty()
        && appraisal
            .inaction_reason
            .as_deref()
            .is_none_or(|reason| reason.trim().is_empty())
    {
        return Err(anyhow!(
            "an appraisal with no actions requires one concrete non-empty inaction_reason"
        ));
    }
    for action in &appraisal.actions {
        let subject = slice
            .constituents
            .iter()
            .find(|value| value.subject_id == action.subject_id)
            .ok_or_else(|| anyhow!("action is attributed outside the cell"))?;
        if action.intent.trim().is_empty() || action.intended_effect.trim().is_empty() {
            return Err(anyhow!(
                "action for subject {} requires non-empty intent and intended_effect",
                subject.subject_id
            ));
        }
        let invalid_references = action
            .state_references
            .iter()
            .filter(|reference| !subject.permitted_state_references.contains(*reference))
            .collect::<Vec<_>>();
        let invalid_channels = action
            .public_channels
            .iter()
            .filter(|channel| !subject.information_channels.contains(*channel))
            .collect::<Vec<_>>();
        if !invalid_references.is_empty() || !invalid_channels.is_empty() {
            return Err(anyhow!(
                "action for subject {} borrowed forbidden state references {:?} or information channels {:?}",
                subject.subject_id,
                invalid_references,
                invalid_channels
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
                    "action for subject {} has effect {:?}, which exceeds exact authority for kind {:?}, locations {:?}, and reachable destinations {:?}",
                    subject.subject_id,
                    action.effect,
                    subject.subject_kind,
                    subject.location_ids,
                    subject.reachable_destination_ids
                ));
            }
        }
    }
    Ok(())
}

fn constrain_cell_proposal_schema(
    schema: &mut serde_json::Value,
    slice: &PermittedCellSlice,
) -> Result<()> {
    let properties = schema
        .pointer_mut("/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("cell proposal schema has no properties"))?;
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
    let allowed_references = allowed_actor_references(slice);
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
    use crate::{
        domain::{AgencySubjectKind, SimulationCellMode, StrategicCellEffect},
        model::{FixtureModel, ModelStageRequest},
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct CorrectingCellModel {
        interpreter_calls: AtomicUsize,
        saw_rejected_appraisal: AtomicBool,
    }

    #[async_trait]
    impl ModelPort for CorrectingCellModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "cell_projector" => {
                    Ok("Faction Six sees the public deadline and reviews its own mandate.".into())
                }
                "cell_persona" => Ok(
                    "Faction Six will publish a bounded position using its bulletin access.".into(),
                ),
                "cell_interpreter" => {
                    let call = self.interpreter_calls.fetch_add(1, Ordering::SeqCst);
                    if call == 0 {
                        return Ok(serde_json::json!({
                            "actions":[{
                                "subject_id":"faction-06",
                                "intent":"publish a position",
                                "intended_effect":"move a person instead",
                                "priority":5,
                                "state_references":["institution:faction-06"],
                                "public_channels":["public bulletin"],
                                "effect":{"type":"actor_move","actor_id":"faction-06","destination_id":"forum"}
                            }],
                            "inaction_reason":null
                        }).to_string());
                    }
                    self.saw_rejected_appraisal.store(
                        request.lived_stream.contains("PREVIOUS_REJECTED_APPRAISAL")
                            && request.lived_stream.contains("actor_move")
                            && request.lived_stream.contains("faction-06"),
                        Ordering::SeqCst,
                    );
                    Ok(serde_json::json!({
                        "actions":[{
                            "subject_id":"faction-06",
                            "intent":"publish a position",
                            "intended_effect":"state its bounded institutional posture",
                            "priority":5,
                            "state_references":["institution:faction-06"],
                            "public_channels":["public bulletin"],
                            "effect":{"type":"institution","institution_id":"faction-06","posture":"published a bounded position","location_ids":["forum"]}
                        }],
                        "inaction_reason":null
                    }).to_string())
                }
                stage => Err(anyhow!("unexpected fixture stage {stage}")),
            }
        }

        fn provider(&self) -> &'static str {
            "correcting-cell-fixture"
        }
    }

    fn fixture_cell_slice() -> PermittedCellSlice {
        PermittedCellSlice {
            cell_id: "cell:test".into(),
            mode: SimulationCellMode::Arena,
            world_revision: 4,
            resolution_epoch: 2,
            snapshot_binding: "campaign:4:2".into(),
            constituents: vec![CellConstituentSlice {
                subject_id: "faction-06".into(),
                subject_kind: AgencySubjectKind::Institution,
                name: "Faction Six".into(),
                collective_authority_id: None,
                location_ids: BTreeSet::from(["forum".into()]),
                knowledge: BTreeSet::from(["the public deadline".into()]),
                capabilities: BTreeSet::new(),
                resources: BTreeSet::from(["bulletin access".into()]),
                information_channels: BTreeSet::from(["public bulletin".into()]),
                permitted_state_references: BTreeSet::from(["institution:faction-06".into()]),
                reachable_destination_ids: BTreeSet::new(),
                goals: vec!["publish a position".into()],
                pressures: vec!["the vote is near".into()],
            }],
            shared_knowledge: BTreeSet::new(),
            shared_capabilities: BTreeSet::new(),
            perceived_events: vec!["The final vote is public.".into()],
            world_clock_pressure: vec!["vote 5/6".into()],
            detail_focus_subject_id: Some("faction-06".into()),
            max_actions: 1,
            source_receipt_ids: vec![],
        }
    }

    #[tokio::test]
    async fn cell_semantic_retry_receives_the_rejected_appraisal() {
        let model = Arc::new(CorrectingCellModel {
            interpreter_calls: AtomicUsize::new(0),
            saw_rejected_appraisal: AtomicBool::new(false),
        });
        let engine = CellProjectionEngine {
            model: model.clone(),
            permit: Arc::new(AllowAllPermit),
            projector_model: "flash".into(),
            persona_model: "flash".into(),
            interpreter_model: "flash".into(),
        };
        let output = engine.execute(fixture_cell_slice()).await.unwrap();
        assert!(model.saw_rejected_appraisal.load(Ordering::SeqCst));
        assert_eq!(output.stage_receipts.len(), 4);
        assert_eq!(
            output.stage_receipts[2].validation_result,
            "semantic_invalid"
        );
        assert!(matches!(
            output.appraisal.actions[0].effect,
            StrategicCellEffect::Institution { .. }
        ));
    }

    #[test]
    fn empty_cell_appraisal_names_the_missing_inaction_reason() {
        let slice = fixture_cell_slice();
        let appraisal = bind_cell_appraisal(
            &slice,
            CellAppraisalProposal {
                actions: vec![],
                inaction_reason: Some("   ".into()),
            },
        );
        let error = validate_cell_appraisal(&slice, &appraisal).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("concrete non-empty inaction_reason")
        );
    }

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
        let guidance = actor_interpreter_guidance(&slice);
        assert!(guidance.contains("other actor retains agency"));
        assert!(guidance.contains("requested response remains unresolved"));
        let result = engine.execute(slice).await.unwrap();
        assert_eq!(result.stage_receipts.len(), 3);
        assert_eq!(result.proposals.reaction_priority, 0);
    }
}

fn allowed_actor_references(slice: &PermittedActorSlice) -> Vec<String> {
    slice
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
        .collect()
}

use crate::model::{ModelPort, ModelStageRequest, run_validated_stage};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use ghostlight_persona_projection::{
    InterpreterPrompt, MEMBRANE_SCHEMA, PersonaPrompt, ProjectorPrompt, build_interpreter_prompt,
    build_persona_prompt, build_projector_prompt, narrative_stream_is_clean,
};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
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
    pub migration_destinations: BTreeMap<String, String>,
    pub activity_target_ids: BTreeSet<String>,
    pub goals: Vec<String>,
    pub current_posture: Option<String>,
    pub pressures: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CellMemberSlice {
    pub subject_id: String,
    pub member_id: String,
    pub name: String,
    pub source_gestalt_id: String,
    pub source_location_id: String,
    pub knowledge: BTreeSet<String>,
    pub capabilities: BTreeSet<String>,
    pub resources: BTreeSet<String>,
    pub information_channels: BTreeSet<String>,
    pub permitted_state_references: BTreeSet<String>,
    pub migration_destinations: BTreeMap<String, String>,
    pub activity_target_ids: BTreeSet<String>,
    pub goals: Vec<String>,
    pub pressures: Vec<String>,
    pub relationships: BTreeMap<String, String>,
    pub memories: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CellPerceivedEventSlice {
    pub event_id: String,
    pub summary: String,
    pub perceived_by_subject_ids: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PermittedCellSlice {
    pub cell_id: String,
    pub mode: crate::domain::SimulationCellMode,
    pub world_revision: u64,
    pub resolution_epoch: u64,
    pub snapshot_binding: String,
    pub constituents: Vec<CellConstituentSlice>,
    pub member_exceptions: Vec<CellMemberSlice>,
    pub shared_knowledge: BTreeSet<String>,
    pub shared_capabilities: BTreeSet<String>,
    pub perceived_events: Vec<CellPerceivedEventSlice>,
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
    actions: Vec<CellActionCandidate>,
    inaction_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellActionCandidate {
    subject_id: String,
    intent: String,
    intended_effect: String,
    priority: i16,
    state_references: Vec<String>,
    public_channels: Vec<String>,
    effect: CellEffectCandidate,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum CellEffectCandidate {
    Institution {
        posture: String,
        location_ids: Vec<String>,
    },
    Gestalt {
        #[serde(default)]
        pressure_additions: Vec<String>,
        #[serde(default)]
        pressure_resolutions: Vec<String>,
    },
    GestaltActivity {
        activity: crate::domain::StrategicActivityKind,
        #[serde(default)]
        target_subject_ids: Vec<String>,
        #[serde(default)]
        location_ids: Vec<String>,
    },
    GestaltMigration {
        destination_gestalt_id: String,
    },
    ActorMove {
        destination_id: String,
    },
    MemberActivity {
        activity: crate::domain::StrategicActivityKind,
        #[serde(default)]
        target_subject_ids: Vec<String>,
        #[serde(default)]
        location_ids: Vec<String>,
    },
    MemberMigration {
        destination_gestalt_id: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellEffectVerification {
    verdicts: Vec<CellActionEffectVerdict>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellActionEffectVerdict {
    action_index: usize,
    result: CellEffectMatchResult,
    mismatch_kind: Option<CellEffectMismatchKind>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
enum CellEffectMatchResult {
    Match,
    Mismatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
enum CellEffectMismatchKind {
    SubjectSwap,
    EffectOmission,
    EffectReversal,
    TargetSubstitution,
    InventedOutcome,
    WrongEffectKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellProjectionProposal {
    segments: Vec<CellPerspectiveSegment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellPerspectiveSegment {
    subject_id: String,
    narrative: String,
}

const CELL_PROJECTION_OUTPUT_CONTRACT: &str = r#"{
  "segments":[
    {"subject_id":"one exact supplied perspective owner","narrative":"private lived prose for only that subject"}
  ]
}"#;

const CELL_APPRAISAL_OUTPUT_CONTRACT: &str = r#"{
  "type":"object",
  "required":["actions","inaction_reason"],
  "properties":{
    "actions":{"type":"array","items":{"type":"object","required":["subject_id","intent","intended_effect","priority","state_references","public_channels","effect"],"properties":{
      "subject_id":{"type":"string"},"intent":{"type":"string"},"intended_effect":{"type":"string"},"priority":{"type":"integer"},
      "state_references":{"type":"array","items":{"type":"string"}},"public_channels":{"type":"array","items":{"type":"string"}},
      "effect":{"oneOf":[
        {"type":"object","required":["type","posture","location_ids"],"properties":{"type":{"const":"institution"},"posture":{"type":"string"},"location_ids":{"type":"array","items":{"type":"string"}}}},
        {"type":"object","required":["type","pressure_additions","pressure_resolutions"],"properties":{"type":{"const":"gestalt"},"pressure_additions":{"type":"array","maxItems":4,"items":{"type":"string"}},"pressure_resolutions":{"type":"array","maxItems":4,"items":{"type":"string"}}}},
        {"type":"object","required":["type","activity","target_subject_ids","location_ids"],"properties":{"type":{"const":"gestalt_activity"},"activity":{"enum":["prepare","coordinate","investigate","recruit","obstruct","trade","communicate"]},"target_subject_ids":{"type":"array","maxItems":4,"items":{"type":"string"}},"location_ids":{"type":"array","maxItems":4,"items":{"type":"string"}}}},
        {"type":"object","required":["type","destination_gestalt_id"],"properties":{"type":{"const":"gestalt_migration"},"destination_gestalt_id":{"type":"string"}}},
        {"type":"object","required":["type","destination_id"],"properties":{"type":{"const":"actor_move"},"destination_id":{"type":"string"}}},
        {"type":"object","required":["type","activity","target_subject_ids","location_ids"],"properties":{"type":{"const":"member_activity"},"activity":{"enum":["prepare","coordinate","investigate","recruit","obstruct","trade","communicate"]},"target_subject_ids":{"type":"array","maxItems":4,"items":{"type":"string"}},"location_ids":{"type":"array","maxItems":1,"items":{"type":"string"}}}},
        {"type":"object","required":["type","destination_gestalt_id"],"properties":{"type":{"const":"member_migration"},"destination_gestalt_id":{"type":"string"}}}
      ]}
    }}},
    "inaction_reason":{"type":["string","null"]}
  }
}"#;

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
            "migration_destinations": subject.migration_destinations,
            "activity_target_ids": subject.activity_target_ids,
            "goals": subject.goals,
            "current_posture": subject.current_posture,
            "pressures": subject.pressures,
        })).collect::<Vec<_>>(),
        "member_exceptions": slice.member_exceptions.iter().map(|member| serde_json::json!({
            "subject_id": member.subject_id,
            "member_id": member.member_id,
            "name": member.name,
            "source_gestalt_id": member.source_gestalt_id,
            "source_location_id": member.source_location_id,
            "knowledge": member.knowledge,
            "capabilities": member.capabilities,
            "resources": member.resources,
            "migration_destinations": member.migration_destinations,
            "activity_target_ids": member.activity_target_ids,
            "goals": member.goals,
            "pressures": member.pressures,
            "relationships": member.relationships,
            "memories": member.memories,
        })).collect::<Vec<_>>(),
        "shared_knowledge": slice.shared_knowledge,
        "shared_capabilities": slice.shared_capabilities,
        "perceived_events": slice.perceived_events,
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
            "allowed_effect_type": allowed_effect_type(&subject.subject_kind),
            "collective_authority_id": subject.collective_authority_id,
            "location_ids": subject.location_ids,
            "allowed_public_channels": subject.information_channels,
            "permitted_state_references": subject.permitted_state_references,
            "reachable_destination_ids": subject.reachable_destination_ids,
            "migration_destinations": subject.migration_destinations,
            "activity_target_ids": subject.activity_target_ids,
            "current_posture": subject.current_posture,
            "current_pressures": subject.pressures,
        })).collect::<Vec<_>>(),
        "member_permissions": slice.member_exceptions.iter().map(|member| serde_json::json!({
            "subject_id": member.subject_id,
            "member_id": member.member_id,
            "allowed_effect_type": "member_migration_or_member_activity",
            "source_gestalt_id": member.source_gestalt_id,
            "source_location_id": member.source_location_id,
            "allowed_public_channels": member.information_channels,
            "permitted_state_references": member.permitted_state_references,
            "migration_destinations": member.migration_destinations,
            "activity_target_ids": member.activity_target_ids,
        })).collect::<Vec<_>>(),
    })
}

fn cell_scene_boundaries(slice: &PermittedCellSlice) -> String {
    let mut by_location = BTreeMap::<String, BTreeSet<String>>::new();
    let mut unlocated = BTreeSet::new();
    let mut perspective_owners = BTreeSet::new();
    for subject in &slice.constituents {
        perspective_owners.insert(subject.name.clone());
        if subject.location_ids.is_empty() {
            unlocated.insert(subject.name.clone());
        } else {
            for location_id in &subject.location_ids {
                by_location
                    .entry(location_id.clone())
                    .or_default()
                    .insert(subject.name.clone());
            }
        }
    }
    for member in &slice.member_exceptions {
        perspective_owners.insert(member.name.clone());
        by_location
            .entry(member.source_location_id.clone())
            .or_default()
            .insert(member.name.clone());
    }
    let mut lines = vec![
        "Scene boundaries are reliable footing, not conjecture. Subjects listed at different locations are in simultaneous remote scenes and cannot directly see, hear, address, or answer one another without an explicitly perceived communication channel.".to_owned(),
    ];
    lines.extend(by_location.into_iter().map(|(location_id, names)| {
        format!(
            "At location {location_id}: {}.",
            names.into_iter().collect::<Vec<_>>().join(", ")
        )
    }));
    if !unlocated.is_empty() {
        lines.push(format!(
            "No co-presence is established for these distributed or unlocated perspectives: {}.",
            unlocated.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    lines.push(format!(
        "Only these cell-owned perspectives may make choices in this turn: {}.",
        perspective_owners
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ")
    ));
    lines.join("\n")
}

fn constrain_cell_projection_schema(
    schema: &mut serde_json::Value,
    slice: &PermittedCellSlice,
) -> Result<()> {
    let segments = schema
        .pointer_mut("/properties/segments")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("cell projection schema has no segment array"))?;
    segments.insert("minItems".into(), 1.into());
    segments.insert("maxItems".into(), slice.max_actions.max(1).into());
    let segment = schema
        .pointer_mut("/$defs/CellPerspectiveSegment/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("cell projection schema has no segment properties"))?;
    let mut subject_ids = slice
        .constituents
        .iter()
        .map(|subject| subject.subject_id.as_str())
        .chain(
            slice
                .member_exceptions
                .iter()
                .map(|member| member.subject_id.as_str()),
        )
        .collect::<Vec<_>>();
    if let Some(focus) = slice.detail_focus_subject_id.as_deref()
        && let Some(index) = subject_ids
            .iter()
            .position(|subject_id| *subject_id == focus)
    {
        subject_ids.swap(0, index);
    }
    segment.insert(
        "subject_id".into(),
        serde_json::json!({"type":"string","enum":subject_ids}),
    );
    Ok(())
}

fn bind_cell_projection(
    slice: &PermittedCellSlice,
    proposal: CellProjectionProposal,
) -> Result<String> {
    if proposal.segments.is_empty() || proposal.segments.len() > slice.max_actions.max(1) {
        return Err(anyhow!(
            "cell Projector emitted an invalid perspective count"
        ));
    }
    let mut segments = BTreeMap::new();
    for segment in proposal.segments {
        if segment.narrative.trim().is_empty() || !narrative_stream_is_clean(&segment.narrative) {
            return Err(anyhow!(
                "cell Projector emitted an empty or non-narrative perspective"
            ));
        }
        let owner_exists = slice
            .constituents
            .iter()
            .any(|subject| subject.subject_id == segment.subject_id)
            || slice
                .member_exceptions
                .iter()
                .any(|member| member.subject_id == segment.subject_id);
        if !owner_exists
            || segments
                .insert(segment.subject_id, segment.narrative)
                .is_some()
        {
            return Err(anyhow!(
                "cell Projector invented or duplicated a perspective owner"
            ));
        }
    }
    if let Some(focus) = slice.detail_focus_subject_id.as_deref()
        && (slice
            .constituents
            .iter()
            .any(|subject| subject.subject_id == focus)
            || slice
                .member_exceptions
                .iter()
                .any(|member| member.subject_id == focus))
        && !segments.contains_key(focus)
    {
        return Err(anyhow!(
            "cell Projector omitted the debt-selected perspective owner"
        ));
    }
    let mut lowered = Vec::with_capacity(segments.len());
    for (subject_id, narrative) in segments {
        if let Some(subject) = slice
            .constituents
            .iter()
            .find(|subject| subject.subject_id == subject_id)
        {
            let footing = if subject.location_ids.is_empty() {
                "at an established but undisclosed location".to_owned()
            } else {
                format!(
                    "at {}",
                    subject
                        .location_ids
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            lowered.push(format!(
                "{} — {}:\n{}",
                subject.name,
                footing,
                narrative.trim()
            ));
        } else {
            let member = slice
                .member_exceptions
                .iter()
                .find(|member| member.subject_id == subject_id)
                .expect("projection owner was validated");
            lowered.push(format!(
                "{} — at {}:\n{}",
                member.name,
                member.source_location_id,
                narrative.trim()
            ));
        }
    }
    Ok(lowered.join("\n\n"))
}

fn allowed_effect_type(kind: &crate::domain::AgencySubjectKind) -> &'static str {
    match kind {
        crate::domain::AgencySubjectKind::Actor => "actor_move",
        crate::domain::AgencySubjectKind::Institution => "institution",
        crate::domain::AgencySubjectKind::Gestalt => {
            "gestalt_pressure_or_gestalt_activity_or_gestalt_migration"
        }
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
        let projector_context = serde_json::to_string(&cell_projector_context(&slice))?;
        let visible_stimulus = slice
            .perceived_events
            .iter()
            .map(|event| {
                format!(
                    "Perceived by [{}]: {}",
                    event
                        .perceived_by_subject_ids
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                    event.summary
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mode_guidance = cell_projector_mode_guidance(&slice.mode);
        let mode_guidance = format!(
            "{mode_guidance} Each perceived event names the exact constituents that can perceive it; do not teach it to anyone else. Only supplied constituents and member_exceptions may own an internal perspective or choice. A person merely mentioned in an event is external observation when absent from those lists: never voice them. Every supplied member_exception was selected because that person has an actionable decision in this horizon. Render each selected person explicitly by name, with only their own footing and choices."
        );
        let word_budget = (120 + 45 * slice.constituents.len()).min(360);
        let perspective_limit = slice.max_actions.max(1);
        let mut projection_schema = serde_json::to_value(schema_for!(CellProjectionProposal))?;
        constrain_cell_projection_schema(&mut projection_schema, &slice)?;
        let mut projection_request = ModelStageRequest {
            stage: "cell_projector".into(),
            model: self.projector_model.clone(),
            snapshot_binding: slice.snapshot_binding.clone(),
            lived_stream: format!(
                "<!-- membrane:{MEMBRANE_SCHEMA}:cell-projector -->\nYou are a private cell Projector. Convert only the permitted typed context and visible stimulus into compact lived narrative segments. Each segment belongs to exactly one supplied subject_id and contains only that subject's perceptions, memories, wants, fears, knowledge, and explicit uncertainty. Mentioned outsiders remain external observations: never give them an internal viewpoint. Do not choose actions or claim world effects. Omit decorative recap. Return between 1 and {perspective_limit} unique segments; do not narrate every supplied subject. If detail_focus_subject_id is present, include it first. Spend the remaining slots only on subjects facing a materially different decision in this horizon.\n\nReturn exactly one JSON object matching this stable shape:\n{CELL_PROJECTION_OUTPUT_CONTRACT}\n\nDomain guidance:\n{mode_guidance}\n\nIdentity:\n{}\n\nPermitted typed context:\n{projector_context}\n\nVisible stimulus:\n{visible_stimulus}\n\nUse no more than {word_budget} narrative words across all segments.",
                slice.cell_id
            ),
            output_schema: Some(projection_schema),
            source_receipt_ids: slice.source_receipt_ids.clone(),
            temperature: Some(0.0),
            max_output_tokens: Some(768),
        };
        let mut projector_receipts = Vec::new();
        let (projected_narrative, projector_receipt) = loop {
            let mut projected =
                run_validated_stage(self.model.as_ref(), &projection_request).await?;
            let proposal = projected
                .structured
                .clone()
                .ok_or_else(|| anyhow!("cell Projector produced no typed segments"))
                .and_then(|value| serde_json::from_value(value).map_err(Into::into));
            match proposal.and_then(|proposal| bind_cell_projection(&slice, proposal)) {
                Ok(narrative) => {
                    let receipt = projected.receipt.clone();
                    projector_receipts.push(projected.receipt);
                    break (narrative, receipt);
                }
                Err(error) if projector_receipts.is_empty() => {
                    projected.receipt.validation_result = "semantic_invalid".into();
                    projected.receipt.local_validation_error =
                        Some(error.to_string().chars().take(1_000).collect());
                    projector_receipts.push(projected.receipt);
                    projection_request.lived_stream.push_str(&format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS SEGMENTS: {error}\nReturn one corrected complete JSON object against the same snapshot and contract."
                    ));
                }
                Err(error) => {
                    return Err(anyhow!(
                        "cell Projector failed perspective binding after one correction: {error}"
                    ));
                }
            }
        };
        let lived = LivedNarrativeStream {
            text: format!(
                "{}\n\n{}",
                cell_scene_boundaries(&slice),
                projected_narrative
            ),
            snapshot_binding: slice.snapshot_binding.clone(),
            projector_receipt,
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
                    domain_guidance: cell_persona_mode_guidance(&slice.mode),
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
        let mut schema = serde_json::to_value(schema_for!(CellAppraisalProposal))?;
        constrain_cell_proposal_schema(&mut schema, &slice)?;
        let interpreter_context = serde_json::to_string(&cell_interpreter_context(&slice))?;
        let permission_guidance = format!(
            concat!(
                "Emit at most {} exact constituent- or named-member-attributed attempts. Priority is an urgency score from 0 to 100 where higher numbers resolve first. ",
                "Copy each subject's allowed_effect_type: institution -> institution, gestalt -> gestalt pressure transition, gestalt_activity, or gestalt_migration, actor -> actor_move, named member -> member_migration or member_activity. ",
                "Use gestalt_activity or member_activity for a concrete attempt that does not itself change pressure. Map attempts narrowly: communicate means speak, send, offer, ask, or notify; coordinate means arrange a joint attempt; prepare means the subject's own concrete work; investigate means seek information; recruit means invite; trade means offer an exchange; obstruct means attempt interference. ",
                "target_subject_ids and location_ids must come from that exact subject's permissions. A member_activity uses exactly the member's source_location_id. Internal work is prepare with no targets. A local investigate may have no target and use the exact current location to seek information from the environment or an unnamed ordinary role; asking an unnamed clerk or dock master for facts maps here and records only the inquiry, never a reply or discovery. A local communicate may likewise have no target at the exact current location when the Persona speaks, sends, offers, asks permission, or notifies an unnamed ordinary role; it records only the source's outgoing attempt, never a listener, reply, acceptance, or outcome. Communication with a canonical subject requires that exact target ID. Never substitute a containing population, related institution, or merely permitted ID for an unnamed role. ",
                "Write intended_effect as the attempted act, never its hoped-for outcome or target response. Institution posture must be a specific new commitment or withholding. Gestalt pressure_resolutions copy exact current_pressures; additions are new unresolved constraints, never completed actions. Use only permitted state references and public channels. ",
                "A population that chooses to board, depart, or relocate together to one supplied migration_destinations key emits gestalt_migration; do not reduce it to prepare. It relocates only that exact population leaf and never implies a named member traveled. A named member who chooses to board, depart, travel, or join a supplied destination emits member_migration; use prepare only while departure remains unchosen. ",
                "A population or arena cannot migrate a person. Runtime binds identity and effect owner IDs from subject_id. Do not emit institution_id, gestalt_id, actor_id, or member_id inside effect. Use an empty actions array plus a concrete inaction_reason when nobody acts."
            ),
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
                output_schema: CELL_APPRAISAL_OUTPUT_CONTRACT,
                domain_guidance: &permission_guidance,
            }),
            output_schema: Some(schema),
            source_receipt_ids: slice.source_receipt_ids.clone(),
            temperature: Some(0.0),
            max_output_tokens: Some(1_600),
        };
        let mut stage_receipts = projector_receipts;
        stage_receipts.push(persona.receipt);
        for attempt in 0..2 {
            let mut interpreted = run_validated_stage(self.model.as_ref(), &request).await?;
            let proposal = interpreted
                .structured
                .clone()
                .ok_or_else(|| anyhow!("cell interpreter produced no typed proposal"))
                .and_then(|value| serde_json::from_value(value).map_err(Into::into));
            match proposal.and_then(|proposal: CellAppraisalProposal| {
                let appraisal = bind_cell_appraisal(&slice, proposal)?;
                validate_cell_appraisal(&slice, &appraisal)?;
                Ok(appraisal)
            }) {
                Ok(appraisal) => {
                    stage_receipts.push(interpreted.receipt);
                    if !appraisal.actions.is_empty() {
                        self.permit
                            .require(
                                &slice.cell_id,
                                &slice.snapshot_binding,
                                "cell_effect_verifier",
                            )
                            .await?;
                        let verifier_context = serde_json::json!({
                            "local_attempt_contract":"A targetless local communicate at the source's exact current location faithfully records speech, an offer, a permission request, or a notice directed to an unnamed ordinary role. It records no listener, reply, acceptance, or outcome and must not be rejected merely because target_subject_ids is empty.",
                            "lived_stream":lived.text,
                            "persona_turn":persona.narrative,
                            "candidate_actions":appraisal.actions.iter().enumerate().map(|(index, action)| serde_json::json!({
                                "index":index,
                                "subject_id":action.subject_id,
                                "intent":action.intent,
                                "intended_effect":action.intended_effect,
                                "typed_effect":action.effect,
                            })).collect::<Vec<_>>(),
                            "subject_names":slice.constituents.iter().map(|subject|(&subject.subject_id, &subject.name)).chain(slice.member_exceptions.iter().map(|member|(&member.subject_id, &member.name))).collect::<BTreeMap<_,_>>(),
                        });
                        let verifier_binding = cell_effect_verification_binding(
                            &slice.snapshot_binding,
                            &appraisal.actions,
                        )?;
                        let verifier_schema = cell_effect_verifier_schema(appraisal.actions.len())?;
                        let mut verified = run_validated_stage(
                            self.model.as_ref(),
                            &ModelStageRequest {
                                stage: "cell_effect_verifier".into(),
                                model: self.interpreter_model.clone(),
                                snapshot_binding: verifier_binding,
                                lived_stream: format!(
                                    "You are the private semantic verifier between an Interpreter and the world kernel. Judge each candidate typed effect independently against the exact attributed subject's choice in the Persona turn. Structural permissions were already checked. Return exactly one verdict for every supplied action_index, in the same order, with no omissions or duplicates. Never reject one action merely because another action is wrong. A gestalt_migration means that exact population leaf chooses to travel together to the supplied destination within the strategic horizon; loading, waiting, giving away passage, sending only some other subject, or merely considering travel does not entail it. Conversely, when the population chooses to board, depart, or relocate together, reject gestalt_activity prepare that erases the chosen journey. Gestalt migration never entails that a named member moved. A member_migration means that named member personally chooses to travel to the destination. Boarding a transport whose supplied destination is unambiguous in the lived stream is a chosen journey; the Persona need not repeat the place name. Giving away a berth, sending somebody else, waiting, or merely considering travel does not entail migration. Conversely, when the member chooses to board, depart, travel, or join the supplied destination, reject member_activity that reduces that commitment to preparing, queuing, or approaching. A member_activity belongs only to that exact named person's stated attempt; it cannot be reassigned to their population. Communication targets must be the exact canonical subjects actually addressed in the Persona turn. If the Persona addresses an unnamed clerk, dock master, passerby, or local environment, reject any effect that substitutes a containing population, related institution, or merely permitted ID. A targetless local investigate at the subject's exact current location is the faithful supported shape for seeking information from an unnamed role or the environment; its empty target list is intentional and must not itself be grounds for rejection. An institution posture must express its stated commitment or withholding. A gestalt pressure resolution must be causally supported by its stated attempt, and an added pressure must be a resulting unresolved condition rather than completed-action prose. An activity records only the exact attempt—never successful preparation, coordination, discovery, recruitment, obstruction, exchange, delivery, persuasion, acceptance, or target response. Reject omissions, reversals, subject swaps, wishful outcomes, and effects that the Persona did not choose. Be concise. Return exactly one JSON object. Each verdict uses result \"match\" with null mismatch_kind when the typed effect faithfully records the attempt. Otherwise use result \"mismatch\" and exactly one mismatch_kind: \"subject_swap\", \"effect_omission\", \"effect_reversal\", \"target_substitution\", \"invented_outcome\", or \"wrong_effect_kind\". Shape: {{\"verdicts\":[{{\"action_index\":0,\"result\":\"match\",\"mismatch_kind\":null}}]}}.\n\nCONTEXT:\n{}",
                                    serde_json::to_string(&verifier_context)?
                                ),
                                output_schema: Some(verifier_schema),
                                source_receipt_ids: slice.source_receipt_ids.clone(),
                                temperature: Some(0.0),
                                max_output_tokens: Some(384),
                            },
                        )
                        .await?;
                        let verification: CellEffectVerification =
                            serde_json::from_value(verified.structured.clone().ok_or_else(
                                || anyhow!("cell effect verifier produced no typed verdict"),
                            )?)?;
                        let rejected_action_indices =
                            validate_effect_verification(&verification, appraisal.actions.len())?;
                        if rejected_action_indices.is_empty() {
                            stage_receipts.push(verified.receipt);
                        } else {
                            let rejection_rationale = verification
                                .verdicts
                                .iter()
                                .filter_map(|verdict| {
                                    let CellEffectMatchResult::Mismatch = verdict.result else {
                                        return None;
                                    };
                                    let mismatch_kind = verdict
                                        .mismatch_kind
                                        .as_ref()
                                        .expect("validated mismatch kind");
                                    format!("action {}: {:?}", verdict.action_index, mismatch_kind)
                                        .into()
                                })
                                .collect::<Vec<_>>()
                                .join("; ");
                            let error = anyhow!(
                                "effect verifier rejected action indices {:?}: {}",
                                rejected_action_indices,
                                rejection_rationale
                            );
                            verified.receipt.validation_result = "semantic_invalid".into();
                            verified.receipt.local_validation_error =
                                Some(error.to_string().chars().take(1_000).collect());
                            stage_receipts.push(verified.receipt);
                            if attempt == 0 {
                                append_cell_correction(
                                    &mut request,
                                    &error,
                                    &serde_json::to_string(&appraisal.actions)?,
                                );
                                continue;
                            }
                            return Err(anyhow!(
                                "cell effect verifier rejected the corrected appraisal: {error}"
                            ));
                        }
                    }
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
                    append_cell_correction(&mut request, &error, &rejected_appraisal);
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

fn cell_projector_mode_guidance(mode: &crate::domain::SimulationCellMode) -> &'static str {
    match mode {
        crate::domain::SimulationCellMode::Cohesive => {
            "This cell has real collective authority. Render a plural lived perspective from genuinely shared knowledge and capability only; describe constituent and named-member exceptions as separately attributed exceptions. A population cannot decide for a named member."
        }
        crate::domain::SimulationCellMode::Arena => {
            "This cell is an arena, never a person or faction. Render an attributed polyphonic situation. Name each subject before its perspective; never use an unmarked first-person voice or a narrator that belongs to no constituent. This arena may contain simultaneous views from remote locations: state each subject's supplied location, keep remote scenes separate, and never stage shared sight, speech, or response unless exact co-presence or a perceived communication channel establishes it. An activity target is permission to attempt contact, not evidence that contact already exists. Never union secrets, knowledge, resources, intentions, authority, or voice between constituents or named-member exceptions."
        }
    }
}

fn cell_persona_mode_guidance(mode: &crate::domain::SimulationCellMode) -> &'static str {
    match mode {
        crate::domain::SimulationCellMode::Cohesive => {
            "Appraise the strategic horizon as a real collective. End this turn with a present-tense choice: describe the concrete attempt the collective now makes, or explicitly choose to hold or wait. Deliberating, asking for a future decision, considering an option, or saying what could be done is intentional inaction unless the choice to act is actually made. Do not invent completed consequences."
        }
        crate::domain::SimulationCellMode::Arena => {
            "Appraise the strategic horizon polyphonically. Name the constituent responsible for every perspective and decision; never speak as the arena or use an unmarked first-person voice. Only subjects already given an attributed internal perspective in the lived stream may choose; people merely observed or mentioned remain external. The lived stream may contain simultaneous remote scenes: preserve every stated location boundary, and never make one constituent see, hear, address, or answer another unless the stream explicitly establishes co-presence or a communication channel. Do not invent an available person, office, route, resource, or response absent from the lived stream. A constituent may choose to seek something unknown, but cannot claim contact with it. For each voiced constituent, end with a present-tense choice: a concrete attempt now, or an explicit choice to hold or wait. Deliberating, asking for a future decision, considering an option, or saying what could be done is inaction unless that constituent actually chooses to act."
        }
    }
}

fn append_cell_correction(
    request: &mut ModelStageRequest,
    error: &anyhow::Error,
    rejected_appraisal: &str,
) {
    request.lived_stream.push_str(&format!(
        "\n\nCORRECTION TASK—THE PREVIOUS APPRAISAL WAS REJECTED.\nREJECTION: {error}\nPREVIOUS_REJECTED_APPRAISAL:\n{rejected_appraisal}\nReturn one corrected complete appraisal against the same snapshot, lived stream, Persona turn, and exact permission context. A rejected action is forbidden unchanged: remove it or replace it with a different, faithful, permitted typed consequence. Do not repeat its subject, intended_effect, and typed effect together. Every retained action must still carry a valid non-empty typed transition under the original contract. If the Persona chose travel but that subject has no exact permitted destination in reachable_destination_ids or migration_destinations, no movement transition is available: remove that action and use explicit inaction. If an attempted preparation, inspection, request, or deliberation has no permitted typed consequence, remove it and use explicit inaction; never emit an empty transition or upgrade consideration into a completed consequence."
    ));
}

fn validate_effect_verification(
    verification: &CellEffectVerification,
    action_count: usize,
) -> Result<Vec<usize>> {
    if verification.verdicts.len() != action_count {
        return Err(anyhow!(
            "cell effect verifier returned {} verdicts for {action_count} actions",
            verification.verdicts.len()
        ));
    }
    let mut rejected = Vec::new();
    for (expected_index, verdict) in verification.verdicts.iter().enumerate() {
        if verdict.action_index != expected_index {
            return Err(anyhow!(
                "cell effect verifier returned an incoherent verdict for action {expected_index}"
            ));
        }
        match (&verdict.result, &verdict.mismatch_kind) {
            (CellEffectMatchResult::Match, None) => {}
            (CellEffectMatchResult::Mismatch, Some(_)) => rejected.push(expected_index),
            _ => {
                return Err(anyhow!(
                    "cell effect verifier returned an incoherent result discriminator for action {expected_index}"
                ));
            }
        }
    }
    Ok(rejected)
}

fn cell_effect_verifier_schema(action_count: usize) -> Result<serde_json::Value> {
    if action_count == 0 {
        return Err(anyhow!(
            "cell effect verifier schema requires at least one action"
        ));
    }
    let mut schema = serde_json::to_value(schema_for!(CellEffectVerification))?;
    let verdicts = schema
        .pointer_mut("/properties/verdicts")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("cell effect verifier schema has no verdicts property"))?;
    verdicts.insert("minItems".into(), serde_json::json!(action_count));
    verdicts.insert("maxItems".into(), serde_json::json!(action_count));
    let action_index = schema
        .pointer_mut("/$defs/CellActionEffectVerdict/properties/action_index")
        .ok_or_else(|| anyhow!("cell effect verifier schema has no action index property"))?;
    *action_index = serde_json::json!({
        "type":"integer",
        "minimum":0,
        "maximum":action_count - 1
    });
    Ok(schema)
}

pub fn cell_effect_verification_binding(
    cell_snapshot_binding: &str,
    actions: &[crate::domain::CellActionProposal],
) -> Result<String> {
    let payload = rmp_serde::to_vec_named(actions)?;
    Ok(format!(
        "{cell_snapshot_binding}:effects:sha256:{:x}",
        Sha256::digest(payload)
    ))
}

fn bind_cell_appraisal(
    slice: &PermittedCellSlice,
    proposal: CellAppraisalProposal,
) -> Result<crate::domain::CellAppraisal> {
    let actions = proposal
        .actions
        .into_iter()
        .map(|candidate| {
            let effect = match candidate.effect {
                CellEffectCandidate::Institution {
                    posture,
                    location_ids,
                } => crate::domain::StrategicCellEffect::Institution {
                    institution_id: candidate.subject_id.clone(),
                    posture,
                    location_ids,
                },
                CellEffectCandidate::Gestalt {
                    pressure_additions,
                    pressure_resolutions,
                } => crate::domain::StrategicCellEffect::Gestalt {
                    gestalt_id: candidate.subject_id.clone(),
                    pressure_additions,
                    pressure_resolutions,
                },
                CellEffectCandidate::GestaltActivity {
                    activity,
                    target_subject_ids,
                    location_ids,
                } => crate::domain::StrategicCellEffect::GestaltActivity {
                    gestalt_id: candidate.subject_id.clone(),
                    activity,
                    target_subject_ids,
                    location_ids,
                },
                CellEffectCandidate::GestaltMigration {
                    destination_gestalt_id,
                } => crate::domain::StrategicCellEffect::GestaltMigration {
                    destination_gestalt_id,
                },
                CellEffectCandidate::ActorMove { destination_id } => {
                    crate::domain::StrategicCellEffect::ActorMove {
                        actor_id: candidate.subject_id.clone(),
                        destination_id,
                    }
                }
                CellEffectCandidate::MemberActivity {
                    activity,
                    target_subject_ids,
                    location_ids,
                } => {
                    let member_id = slice
                        .member_exceptions
                        .iter()
                        .find(|member| member.subject_id == candidate.subject_id)
                        .map(|member| member.member_id.clone())
                        .ok_or_else(|| {
                            anyhow!(
                                "member_activity subject {} is not a selected member exception",
                                candidate.subject_id
                            )
                        })?;
                    crate::domain::StrategicCellEffect::MemberActivity {
                        member_id,
                        activity,
                        target_subject_ids,
                        location_ids,
                    }
                }
                CellEffectCandidate::MemberMigration {
                    destination_gestalt_id,
                } => crate::domain::StrategicCellEffect::MemberMigration {
                    destination_gestalt_id,
                },
            };
            Ok(crate::domain::CellActionProposal {
                subject_id: candidate.subject_id,
                intent: candidate.intent,
                intended_effect: candidate.intended_effect,
                priority: candidate.priority,
                state_references: candidate.state_references,
                public_channels: candidate.public_channels,
                effect,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(crate::domain::CellAppraisal {
        schema: "ghostlight.cell_appraisal.v1".into(),
        cell_id: slice.cell_id.clone(),
        world_revision: slice.world_revision,
        resolution_epoch: slice.resolution_epoch,
        considered_subject_ids: slice
            .constituents
            .iter()
            .map(|subject| subject.subject_id.clone())
            .collect(),
        actions,
        inaction_reason: proposal.inaction_reason,
    })
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
        if action.intent.trim().is_empty() || action.intended_effect.trim().is_empty() {
            return Err(anyhow!(
                "action for subject {} requires non-empty intent and intended_effect",
                action.subject_id
            ));
        }
        if let Some(subject) = slice
            .constituents
            .iter()
            .find(|value| value.subject_id == action.subject_id)
        {
            validate_action_permissions(
                action,
                &subject.permitted_state_references,
                &subject.information_channels,
            )?;
            validate_constituent_effect(subject, &action.effect)?;
        } else if let Some(member) = slice
            .member_exceptions
            .iter()
            .find(|value| value.subject_id == action.subject_id)
        {
            validate_action_permissions(
                action,
                &member.permitted_state_references,
                &member.information_channels,
            )?;
            match &action.effect {
                crate::domain::StrategicCellEffect::MemberActivity {
                    member_id,
                    activity,
                    target_subject_ids,
                    location_ids,
                } => {
                    let unique_targets = target_subject_ids.iter().collect::<BTreeSet<_>>();
                    let needs_target = !activity.allows_targetless_local_attempt();
                    if needs_target && target_subject_ids.is_empty() {
                        return Err(anyhow!(
                            "named member {} activity {:?} requires one or more exact target IDs; no anonymous or unsupplied target can be encoded. Remove the action unless the Persona explicitly attempted one of {:?}",
                            member.member_id,
                            activity,
                            member.activity_target_ids
                        ));
                    }
                    if member_id != &member.member_id
                        || target_subject_ids.len() > 4
                        || unique_targets.len() != target_subject_ids.len()
                        || target_subject_ids
                            .iter()
                            .any(|target| !member.activity_target_ids.contains(target))
                        || location_ids.len() != 1
                        || location_ids[0] != member.source_location_id
                    {
                        return Err(anyhow!(
                            "named member {} proposed {:?} toward {:?} at {:?}; exact allowed targets are {:?} and location is {:?}",
                            member.member_id,
                            activity,
                            target_subject_ids,
                            location_ids,
                            member.activity_target_ids,
                            member.source_location_id
                        ));
                    }
                }
                crate::domain::StrategicCellEffect::MemberMigration {
                    destination_gestalt_id,
                } if member
                    .migration_destinations
                    .contains_key(destination_gestalt_id) => {}
                _ => {
                    return Err(anyhow!(
                        "action for named member {} exceeds exact personal authority; effect={:?}; allowed destination gestalt IDs={:?}",
                        member.member_id,
                        action.effect,
                        member.migration_destinations.keys().collect::<Vec<_>>()
                    ));
                }
            }
        } else {
            return Err(anyhow!("action is attributed outside the cell"));
        }
    }
    Ok(())
}

fn validate_constituent_effect(
    subject: &CellConstituentSlice,
    effect: &crate::domain::StrategicCellEffect,
) -> Result<()> {
    match effect {
        crate::domain::StrategicCellEffect::Institution {
            institution_id,
            posture,
            location_ids,
        } => {
            if subject.subject_kind != crate::domain::AgencySubjectKind::Institution
                || institution_id != &subject.subject_id
            {
                return Err(anyhow!(
                    "subject {} has kind {:?} and may not emit an institution effect for {}",
                    subject.subject_id,
                    subject.subject_kind,
                    institution_id
                ));
            }
            if location_ids
                .iter()
                .any(|location| !subject.location_ids.contains(location))
            {
                return Err(anyhow!(
                    "institution {} used locations {:?}; exact allowed locations are {:?}",
                    subject.subject_id,
                    location_ids,
                    subject.location_ids
                ));
            }
            let current = subject.current_posture.as_deref().ok_or_else(|| {
                anyhow!(
                    "institution {} is missing its exact current posture",
                    subject.subject_id
                )
            })?;
            if !crate::resolution::substantive_text_change(current, posture) {
                return Err(anyhow!(
                    "institution {} proposed posture {:?}, but its exact current posture is {:?}; emit a specific different commitment or choose inaction",
                    subject.subject_id,
                    posture,
                    current
                ));
            }
        }
        crate::domain::StrategicCellEffect::Gestalt {
            gestalt_id,
            pressure_additions,
            pressure_resolutions,
        } => {
            if subject.subject_kind != crate::domain::AgencySubjectKind::Gestalt
                || gestalt_id != &subject.subject_id
            {
                return Err(anyhow!(
                    "subject {} has kind {:?} and may not emit a gestalt effect for {}",
                    subject.subject_id,
                    subject.subject_kind,
                    gestalt_id
                ));
            }
            crate::resolution::validate_gestalt_pressure_transition(
                &subject.pressures,
                pressure_additions,
                pressure_resolutions,
            )
            .map_err(|error| {
                anyhow!(
                    "gestalt {} proposed additions {:?} and resolutions {:?} against exact current pressures {:?}: {}",
                    subject.subject_id,
                    pressure_additions,
                    pressure_resolutions,
                    subject.pressures,
                    error
                )
            })?;
        }
        crate::domain::StrategicCellEffect::GestaltActivity {
            gestalt_id,
            activity,
            target_subject_ids,
            location_ids,
        } => {
            let unique_targets = target_subject_ids.iter().collect::<BTreeSet<_>>();
            let unique_locations = location_ids.iter().collect::<BTreeSet<_>>();
            let needs_target = !activity.allows_targetless_local_attempt();
            if needs_target && target_subject_ids.is_empty() {
                return Err(anyhow!(
                    "gestalt {} activity {:?} requires one or more exact target IDs; no anonymous or unsupplied target can be encoded. Remove the action unless the Persona explicitly attempted one of {:?}",
                    subject.subject_id,
                    activity,
                    subject.activity_target_ids
                ));
            }
            if subject.subject_kind != crate::domain::AgencySubjectKind::Gestalt
                || gestalt_id != &subject.subject_id
                || target_subject_ids.len() > 4
                || unique_targets.len() != target_subject_ids.len()
                || target_subject_ids
                    .iter()
                    .any(|target| !subject.activity_target_ids.contains(target))
                || location_ids.len() > 4
                || unique_locations.len() != location_ids.len()
                || location_ids
                    .iter()
                    .any(|location| !subject.location_ids.contains(location))
            {
                return Err(anyhow!(
                    "gestalt {} proposed {:?} toward {:?} at {:?}; exact allowed targets are {:?} and locations are {:?}",
                    subject.subject_id,
                    activity,
                    target_subject_ids,
                    location_ids,
                    subject.activity_target_ids,
                    subject.location_ids
                ));
            }
        }
        crate::domain::StrategicCellEffect::GestaltMigration {
            destination_gestalt_id,
        } => {
            if subject.subject_kind != crate::domain::AgencySubjectKind::Gestalt
                || !subject
                    .migration_destinations
                    .contains_key(destination_gestalt_id)
            {
                return Err(anyhow!(
                    "gestalt {} may not migrate to {}; exact allowed population destinations are {:?}",
                    subject.subject_id,
                    destination_gestalt_id,
                    subject.migration_destinations.keys().collect::<Vec<_>>()
                ));
            }
        }
        crate::domain::StrategicCellEffect::ActorMove {
            actor_id,
            destination_id,
        } => {
            if subject.subject_kind != crate::domain::AgencySubjectKind::Actor
                || actor_id != &subject.subject_id
                || !subject.reachable_destination_ids.contains(destination_id)
            {
                return Err(anyhow!(
                    "subject {} has kind {:?}; actor movement requested for {} to {:?}, while exact reachable destinations are {:?}",
                    subject.subject_id,
                    subject.subject_kind,
                    actor_id,
                    destination_id,
                    subject.reachable_destination_ids
                ));
            }
        }
        crate::domain::StrategicCellEffect::MemberMigration { .. } => {
            return Err(anyhow!(
                "constituent {} is not a named member and may not emit member_migration",
                subject.subject_id
            ));
        }
        crate::domain::StrategicCellEffect::MemberActivity { .. } => {
            return Err(anyhow!(
                "constituent {} is not a named member and may not emit member_activity",
                subject.subject_id
            ));
        }
    }
    Ok(())
}

fn validate_action_permissions(
    action: &crate::domain::CellActionProposal,
    permitted_state_references: &BTreeSet<String>,
    information_channels: &BTreeSet<String>,
) -> Result<()> {
    let invalid_references = action
        .state_references
        .iter()
        .filter(|reference| !permitted_state_references.contains(*reference))
        .collect::<Vec<_>>();
    let invalid_channels = action
        .public_channels
        .iter()
        .filter(|channel| !information_channels.contains(*channel))
        .collect::<Vec<_>>();
    if !invalid_references.is_empty() || !invalid_channels.is_empty() {
        return Err(anyhow!(
            "action for subject {} borrowed forbidden state references {:?} or information channels {:?}",
            action.subject_id,
            invalid_references,
            invalid_channels
        ));
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
        .pointer_mut("/$defs/CellActionCandidate/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("cell appraisal schema has no proposal properties"))?;
    let subject_ids = slice
        .constituents
        .iter()
        .map(|value| value.subject_id.as_str())
        .chain(
            slice
                .member_exceptions
                .iter()
                .map(|value| value.subject_id.as_str()),
        )
        .collect::<Vec<_>>();
    proposal.insert(
        "subject_id".into(),
        serde_json::json!({"type":"string","enum":subject_ids}),
    );
    proposal.insert(
        "priority".into(),
        serde_json::json!({"type":"integer","minimum":0,"maximum":100}),
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
                    assert!(
                        request
                            .lived_stream
                            .contains("between 1 and 1 unique segments")
                    );
                    assert!(request.lived_stream.contains("include it first"));
                    Ok(serde_json::json!({
                        "segments":[{
                            "subject_id":"faction-06",
                            "narrative":"The public deadline is visible, and the mandate remains ours to review."
                        }]
                    })
                    .to_string())
                }
                "cell_persona" => {
                    assert!(
                        request
                            .lived_stream
                            .contains("At location forum: Faction Six.")
                    );
                    assert!(
                        request
                            .lived_stream
                            .contains("Only these cell-owned perspectives may make choices")
                    );
                    Ok(
                        "Faction Six will publish a bounded position using its bulletin access."
                            .into(),
                    )
                }
                "cell_interpreter" => {
                    let call = self.interpreter_calls.fetch_add(1, Ordering::SeqCst);
                    assert!(
                        request
                            .lived_stream
                            .contains("local communicate may likewise have no target")
                    );
                    assert!(
                        request
                            .lived_stream
                            .contains("chooses to board, depart, travel, or join")
                    );
                    assert!(
                        request.lived_stream.contains(
                            "Never substitute a containing population, related institution"
                        )
                    );
                    if call == 0 {
                        return Ok(serde_json::json!({
                            "actions":[{
                                "subject_id":"faction-06",
                                "intent":"publish a position",
                                "intended_effect":"move a person instead",
                                "priority":5,
                                "state_references":["institution:faction-06"],
                                "public_channels":["public bulletin"],
                                "effect":{"type":"actor_move","destination_id":"forum"}
                            }],
                            "inaction_reason":null
                        })
                        .to_string());
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
                            "effect":{"type":"institution","posture":"published a bounded position","location_ids":["forum"]}
                        }],
                        "inaction_reason":null
                    }).to_string())
                }
                "cell_effect_verifier" => {
                    assert!(request.lived_stream.contains("JSON object"));
                    assert!(
                        request
                            .lived_stream
                            .contains("targetless local communicate")
                    );
                    assert!(
                        request
                            .lived_stream
                            .contains("reject member_activity that reduces that commitment")
                    );
                    Ok(serde_json::json!({
                        "verdicts":[{
                            "action_index":0,
                            "result":"match",
                            "mismatch_kind":null
                        }]
                    })
                    .to_string())
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
                migration_destinations: BTreeMap::new(),
                activity_target_ids: BTreeSet::new(),
                goals: vec!["publish a position".into()],
                current_posture: Some("weighing whether to publish a position".into()),
                pressures: vec!["the vote is near".into()],
            }],
            member_exceptions: vec![],
            shared_knowledge: BTreeSet::new(),
            shared_capabilities: BTreeSet::new(),
            perceived_events: vec![CellPerceivedEventSlice {
                event_id: "event:vote".into(),
                summary: "The final vote is public.".into(),
                perceived_by_subject_ids: BTreeSet::from(["house-a".into(), "house-b".into()]),
            }],
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
        assert_eq!(output.stage_receipts.len(), 5);
        assert_eq!(
            output.stage_receipts[2].validation_result,
            "semantic_invalid"
        );
        assert!(matches!(
            output.appraisal.actions[0].effect,
            StrategicCellEffect::Institution { .. }
        ));
    }

    struct SemanticallyCorrectingCellModel {
        interpreter_calls: AtomicUsize,
        verifier_calls: AtomicUsize,
        saw_verifier_rejection: AtomicBool,
    }

    #[async_trait]
    impl ModelPort for SemanticallyCorrectingCellModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "cell_projector" => Ok(serde_json::json!({
                    "segments":[{
                        "subject_id":"faction-06",
                        "narrative":"The deadline is visible, but evidence for an immediate release is absent."
                    }]
                })
                .to_string()),
                "cell_persona" => Ok(
                    "We will withhold the reserve commitment until the public count is verified."
                        .into(),
                ),
                "cell_interpreter" => {
                    let correction = self.interpreter_calls.fetch_add(1, Ordering::SeqCst) > 0;
                    if correction {
                        self.saw_verifier_rejection.store(
                            request.lived_stream.contains("effect verifier rejected")
                                && request.lived_stream.contains("releases the reserve"),
                            Ordering::SeqCst,
                        );
                        assert!(request.lived_stream.contains("forbidden unchanged"));
                        assert!(request
                            .lived_stream
                            .contains("no exact permitted destination"));
                    }
                    Ok(serde_json::json!({
                        "actions":[{
                            "subject_id":"faction-06",
                            "intent":"state the reserve decision",
                            "intended_effect":if correction {"withhold release pending a verified count"} else {"release the reserve immediately"},
                            "priority":5,
                            "state_references":["institution:faction-06"],
                            "public_channels":["public bulletin"],
                            "effect":{
                                "type":"institution",
                                "posture":if correction {"withholding reserve commitment pending a verified public count"} else {"releases the reserve immediately"},
                                "location_ids":["forum"]
                            }
                        }],
                        "inaction_reason":null
                    }).to_string())
                }
                "cell_effect_verifier" => {
                    assert!(
                        request
                            .lived_stream
                            .contains("reject any effect that substitutes a containing population")
                    );
                    assert!(request
                        .lived_stream
                        .contains("empty target list is intentional"));
                    assert!(request.lived_stream.contains("At location forum"));
                    let correction = self.verifier_calls.fetch_add(1, Ordering::SeqCst) > 0;
                    Ok(serde_json::json!({
                        "verdicts":[{
                            "action_index":0,
                            "result":if correction { "match" } else { "mismatch" },
                            "mismatch_kind":if correction {
                                None::<&str>
                            } else {
                                Some("effect_reversal")
                            }
                        }]
                    })
                    .to_string())
                }
                stage => Err(anyhow!("unexpected fixture stage {stage}")),
            }
        }

        fn provider(&self) -> &'static str {
            "semantic-correction-fixture"
        }
    }

    #[tokio::test]
    async fn effect_verifier_rejects_a_reversed_decision_before_terminal_output() {
        let model = Arc::new(SemanticallyCorrectingCellModel {
            interpreter_calls: AtomicUsize::new(0),
            verifier_calls: AtomicUsize::new(0),
            saw_verifier_rejection: AtomicBool::new(false),
        });
        let output = CellProjectionEngine {
            model: model.clone(),
            permit: Arc::new(AllowAllPermit),
            projector_model: "flash".into(),
            persona_model: "flash".into(),
            interpreter_model: "flash".into(),
        }
        .execute(fixture_cell_slice())
        .await
        .unwrap();
        assert!(model.saw_verifier_rejection.load(Ordering::SeqCst));
        assert_eq!(output.stage_receipts.len(), 6);
        assert_eq!(output.stage_receipts[3].stage, "cell_effect_verifier");
        assert_eq!(
            output.stage_receipts[3].validation_result,
            "semantic_invalid"
        );
        let StrategicCellEffect::Institution { posture, .. } = &output.appraisal.actions[0].effect
        else {
            panic!("corrected effect changed type")
        };
        assert!(posture.contains("withholding reserve"));
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
        )
        .unwrap();
        let error = validate_cell_appraisal(&slice, &appraisal).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("concrete non-empty inaction_reason")
        );
    }

    #[test]
    fn compact_cell_prompt_contract_is_valid_json() {
        serde_json::from_str::<serde_json::Value>(CELL_APPRAISAL_OUTPUT_CONTRACT).unwrap();
        assert!(!CELL_APPRAISAL_OUTPUT_CONTRACT.contains("\"institution_id\""));
        assert!(!CELL_APPRAISAL_OUTPUT_CONTRACT.contains("\"gestalt_id\""));
        assert!(!CELL_APPRAISAL_OUTPUT_CONTRACT.contains("\"actor_id\""));
        assert!(!CELL_APPRAISAL_OUTPUT_CONTRACT.contains("\"member_id\""));
    }

    #[test]
    fn effect_verifier_binding_changes_with_the_exact_action_bundle() {
        let first = crate::domain::CellActionProposal {
            subject_id: "faction-06".into(),
            intent: "withhold the reserve".into(),
            intended_effect: "wait for a verified count".into(),
            priority: 1,
            state_references: vec![],
            public_channels: vec![],
            effect: StrategicCellEffect::Institution {
                institution_id: "faction-06".into(),
                posture: "withholding pending verification".into(),
                location_ids: vec!["forum".into()],
            },
        };
        let mut second = first.clone();
        second.intended_effect = "release immediately".into();
        assert_eq!(
            cell_effect_verification_binding("snapshot", std::slice::from_ref(&first)).unwrap(),
            cell_effect_verification_binding("snapshot", std::slice::from_ref(&first)).unwrap()
        );
        assert_ne!(
            cell_effect_verification_binding("snapshot", &[first]).unwrap(),
            cell_effect_verification_binding("snapshot", &[second]).unwrap()
        );
    }

    #[test]
    fn effect_verifier_requires_one_ordered_verdict_per_action() {
        let verification = CellEffectVerification {
            verdicts: vec![
                CellActionEffectVerdict {
                    action_index: 0,
                    result: CellEffectMatchResult::Match,
                    mismatch_kind: None,
                },
                CellActionEffectVerdict {
                    action_index: 1,
                    result: CellEffectMatchResult::Mismatch,
                    mismatch_kind: Some(CellEffectMismatchKind::TargetSubstitution),
                },
            ],
        };
        assert_eq!(
            validate_effect_verification(&verification, 2).unwrap(),
            vec![1]
        );

        let mut duplicate = verification;
        duplicate.verdicts[1].action_index = 0;
        assert!(
            validate_effect_verification(&duplicate, 2)
                .unwrap_err()
                .to_string()
                .contains("action 1")
        );
    }

    #[test]
    fn effect_verifier_schema_binds_the_exact_batch_size() {
        let schema = cell_effect_verifier_schema(4).unwrap();
        assert_eq!(
            schema.pointer("/properties/verdicts/minItems"),
            Some(&serde_json::json!(4))
        );
        assert_eq!(
            schema.pointer("/properties/verdicts/maxItems"),
            Some(&serde_json::json!(4))
        );
        assert_eq!(
            schema.pointer("/$defs/CellActionEffectVerdict/properties/action_index/maximum"),
            Some(&serde_json::json!(3))
        );
    }

    #[test]
    fn gestalt_pressure_shape_is_rejected_before_the_world_wave() {
        let mut slice = fixture_cell_slice();
        let subject = &mut slice.constituents[0];
        subject.subject_id = "crowd".into();
        subject.subject_kind = AgencySubjectKind::Gestalt;
        subject.permitted_state_references = BTreeSet::from(["gestalt:crowd".into()]);
        let appraisal = bind_cell_appraisal(
            &slice,
            CellAppraisalProposal {
                actions: vec![CellActionCandidate {
                    subject_id: "crowd".into(),
                    intent: "respond to the pressure".into(),
                    intended_effect: "change the collective situation".into(),
                    priority: 1,
                    state_references: vec!["gestalt:crowd".into()],
                    public_channels: vec![],
                    effect: CellEffectCandidate::Gestalt {
                        pressure_additions: vec![],
                        pressure_resolutions: vec![],
                    },
                }],
                inaction_reason: None,
            },
        )
        .unwrap();
        assert!(
            validate_cell_appraisal(&slice, &appraisal)
                .unwrap_err()
                .to_string()
                .contains("must change one to four markers")
        );
    }

    #[test]
    fn gestalt_activity_requires_an_exact_permitted_target_and_location() {
        let mut slice = fixture_cell_slice();
        let subject = &mut slice.constituents[0];
        subject.subject_id = "refugees".into();
        subject.subject_kind = AgencySubjectKind::Gestalt;
        subject.permitted_state_references = BTreeSet::from(["gestalt:refugees".into()]);
        subject.activity_target_ids = BTreeSet::from(["dockers".into()]);
        let valid = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Coordinate,
            target_subject_ids: vec!["dockers".into()],
            location_ids: vec!["forum".into()],
        };
        validate_constituent_effect(subject, &valid).unwrap();

        let invented_target = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Coordinate,
            target_subject_ids: vec!["unseen-ministry".into()],
            location_ids: vec!["forum".into()],
        };
        assert!(
            validate_constituent_effect(subject, &invented_target)
                .unwrap_err()
                .to_string()
                .contains("exact allowed targets")
        );

        let targetless_obstruction = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Obstruct,
            target_subject_ids: vec![],
            location_ids: vec!["forum".into()],
        };
        let error = validate_constituent_effect(subject, &targetless_obstruction).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("no anonymous or unsupplied target")
        );
        assert!(error.to_string().contains("Remove the action"));

        let internal_preparation = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Prepare,
            target_subject_ids: vec![],
            location_ids: vec!["forum".into()],
        };
        validate_constituent_effect(subject, &internal_preparation).unwrap();

        let local_investigation = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Investigate,
            target_subject_ids: vec![],
            location_ids: vec!["forum".into()],
        };
        validate_constituent_effect(subject, &local_investigation).unwrap();

        let local_communication = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Communicate,
            target_subject_ids: vec![],
            location_ids: vec!["forum".into()],
        };
        validate_constituent_effect(subject, &local_communication).unwrap();
    }

    #[test]
    fn gestalt_migration_is_bound_to_the_exact_population_destination() {
        let mut slice = fixture_cell_slice();
        let subject = &mut slice.constituents[0];
        subject.subject_id = "refugees".into();
        subject.subject_kind = AgencySubjectKind::Gestalt;
        subject.migration_destinations =
            BTreeMap::from([("harbor-neighbors".into(), "south-harbor".into())]);
        let valid = StrategicCellEffect::GestaltMigration {
            destination_gestalt_id: "harbor-neighbors".into(),
        };
        validate_constituent_effect(subject, &valid).unwrap();

        let invented = StrategicCellEffect::GestaltMigration {
            destination_gestalt_id: "palace-court".into(),
        };
        assert!(
            validate_constituent_effect(subject, &invented)
                .unwrap_err()
                .to_string()
                .contains("exact allowed population destinations")
        );
    }

    #[test]
    fn named_member_activity_stays_attributed_to_the_person() {
        let mut slice = fixture_cell_slice();
        slice.member_exceptions.push(CellMemberSlice {
            subject_id: "member:mira".into(),
            member_id: "mira".into(),
            name: "Mira".into(),
            source_gestalt_id: "refugees".into(),
            source_location_id: "forum".into(),
            knowledge: BTreeSet::new(),
            capabilities: BTreeSet::new(),
            resources: BTreeSet::new(),
            information_channels: BTreeSet::new(),
            permitted_state_references: BTreeSet::from(["member:mira".into()]),
            migration_destinations: BTreeMap::new(),
            activity_target_ids: BTreeSet::from(["refugees".into()]),
            goals: vec![],
            pressures: vec![],
            relationships: BTreeMap::new(),
            memories: vec![],
        });
        let appraisal = bind_cell_appraisal(
            &slice,
            CellAppraisalProposal {
                actions: vec![CellActionCandidate {
                    subject_id: "member:mira".into(),
                    intent: "offer to help repair the shelter".into(),
                    intended_effect: "make the offer to the refugees".into(),
                    priority: 70,
                    state_references: vec!["member:mira".into()],
                    public_channels: vec![],
                    effect: CellEffectCandidate::MemberActivity {
                        activity: crate::domain::StrategicActivityKind::Communicate,
                        target_subject_ids: vec!["refugees".into()],
                        location_ids: vec!["forum".into()],
                    },
                }],
                inaction_reason: None,
            },
        )
        .unwrap();
        validate_cell_appraisal(&slice, &appraisal).unwrap();

        let mut stolen = appraisal;
        let StrategicCellEffect::MemberActivity { member_id, .. } = &mut stolen.actions[0].effect
        else {
            unreachable!()
        };
        *member_id = "somebody-else".into();
        assert!(validate_cell_appraisal(&slice, &stolen).is_err());
    }

    #[test]
    fn repeated_institution_posture_reports_the_exact_missing_change() {
        let slice = fixture_cell_slice();
        let error = validate_constituent_effect(
            &slice.constituents[0],
            &StrategicCellEffect::Institution {
                institution_id: "faction-06".into(),
                posture: "weighing whether to publish a position".into(),
                location_ids: vec!["forum".into()],
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("exact current posture"));
        assert!(
            error
                .to_string()
                .contains("weighing whether to publish a position")
        );
        let projector_context = cell_projector_context(&slice);
        assert_eq!(
            projector_context["constituents"][0]["current_posture"],
            "weighing whether to publish a position"
        );
        assert_eq!(
            projector_context["constituents"][0]["pressures"][0],
            "the vote is near"
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

    #[test]
    fn semantic_correction_restates_non_empty_effect_contract() {
        let mut request = ModelStageRequest {
            stage: "cell_interpreter".into(),
            model: "flash".into(),
            snapshot_binding: "campaign:one:revision:2".into(),
            lived_stream: "original contract".into(),
            output_schema: None,
            source_receipt_ids: vec![],
            temperature: Some(0.0),
            max_output_tokens: Some(100),
        };
        append_cell_correction(
            &mut request,
            &anyhow::anyhow!("the typed effect was unsupported"),
            r#"{"actions":[{"effect":{"type":"gestalt"}}]}"#,
        );
        assert!(
            request
                .lived_stream
                .contains("valid non-empty typed transition")
        );
        assert!(
            request
                .lived_stream
                .contains("preparation, inspection, request, or deliberation")
        );
        assert!(request.lived_stream.contains("use explicit inaction"));
    }

    #[test]
    fn cell_guidance_preserves_attribution_and_ends_on_a_decision() {
        let arena_projector = cell_projector_mode_guidance(&SimulationCellMode::Arena);
        let arena_persona = cell_persona_mode_guidance(&SimulationCellMode::Arena);
        let cohesive_persona = cell_persona_mode_guidance(&SimulationCellMode::Cohesive);
        assert!(arena_projector.contains("Name each subject"));
        assert!(arena_projector.contains("never use an unmarked first-person voice"));
        assert!(arena_projector.contains("remote locations"));
        assert!(arena_projector.contains("never stage shared sight, speech, or response"));
        assert!(arena_persona.contains("Name the constituent"));
        assert!(arena_persona.contains("present-tense choice"));
        assert!(arena_persona.contains("simultaneous remote scenes"));
        assert!(arena_persona.contains("preserve every stated location boundary"));
        assert!(arena_persona.contains("merely observed or mentioned remain external"));
        assert!(arena_persona.contains("cannot claim contact"));
        assert!(cohesive_persona.contains("present-tense choice"));
        assert!(cohesive_persona.contains("asking for a future decision"));
    }

    #[test]
    fn cell_priority_schema_uses_a_bounded_higher_wins_score() {
        let mut schema = serde_json::to_value(schema_for!(CellAppraisalProposal)).unwrap();
        constrain_cell_proposal_schema(&mut schema, &fixture_cell_slice()).unwrap();
        assert_eq!(
            schema.pointer("/$defs/CellActionCandidate/properties/priority/minimum"),
            Some(&serde_json::json!(0))
        );
        assert_eq!(
            schema.pointer("/$defs/CellActionCandidate/properties/priority/maximum"),
            Some(&serde_json::json!(100))
        );
    }

    #[test]
    fn cell_projection_binds_exact_unique_perspective_owners_into_prose() {
        let slice = fixture_cell_slice();
        let narrative = bind_cell_projection(
            &slice,
            CellProjectionProposal {
                segments: vec![CellPerspectiveSegment {
                    subject_id: "faction-06".into(),
                    narrative: "The deadline presses against our undecided mandate.".into(),
                }],
            },
        )
        .unwrap();
        assert!(narrative.contains("Faction Six — at forum"));
        assert!(!narrative.contains("faction-06"));

        let invented = bind_cell_projection(
            &slice,
            CellProjectionProposal {
                segments: vec![CellPerspectiveSegment {
                    subject_id: "mentioned-outsider".into(),
                    narrative: "I take over the scene.".into(),
                }],
            },
        )
        .unwrap_err();
        assert!(invented.to_string().contains("invented or duplicated"));

        let mut duplicate_slice = slice.clone();
        duplicate_slice.max_actions = 2;
        let duplicate = bind_cell_projection(
            &duplicate_slice,
            CellProjectionProposal {
                segments: vec![
                    CellPerspectiveSegment {
                        subject_id: "faction-06".into(),
                        narrative: "First voice.".into(),
                    },
                    CellPerspectiveSegment {
                        subject_id: "faction-06".into(),
                        narrative: "Second voice.".into(),
                    },
                ],
            },
        )
        .unwrap_err();
        assert!(duplicate.to_string().contains("invented or duplicated"));
    }

    #[test]
    fn cell_projection_must_include_the_debt_selected_subject() {
        let mut slice = fixture_cell_slice();
        let mut other = slice.constituents[0].clone();
        other.subject_id = "faction-07".into();
        other.name = "Faction Seven".into();
        slice.constituents.push(other);
        slice.detail_focus_subject_id = Some("faction-06".into());
        let error = bind_cell_projection(
            &slice,
            CellProjectionProposal {
                segments: vec![CellPerspectiveSegment {
                    subject_id: "faction-07".into(),
                    narrative: "Our own horizon remains quiet.".into(),
                }],
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("debt-selected"));
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

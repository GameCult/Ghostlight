use crate::agent::{
    ModelAgentFailure, ModelAgentSpec, ModelAgentTool, ModelAgentToolContext,
    ModelAgentToolOutcome, causal_source_ids, run_model_agent,
};
use crate::model::{ModelPort, ModelStageOutput, ModelStageRequest, run_validated_stage};
use crate::session_zero::{AggregatedBoundary, CampaignContract};
use anyhow::{Context, Result, anyhow};
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorInteractionRole {
    DirectResponseExpected,
    PresentObserver,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersonaSubjectKind {
    IndividualActor,
    CohesiveGestalt,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PermittedActorSlice {
    pub actor_id: String,
    pub location_id: String,
    pub subject_kind: PersonaSubjectKind,
    pub snapshot_binding: String,
    pub interaction_role: ActorInteractionRole,
    pub identity_experience: Vec<String>,
    /// Established identifiers owned by other people in the actor's immediate
    /// scene or source population. This is social context, not a namespace
    /// registry: the kernel uses it only to prevent a newly adopted handle
    /// from silently impersonating an already-durable person.
    pub reserved_public_identities: BTreeSet<String>,
    pub memories: Vec<String>,
    /// Exact recent public responses already committed for this stable subject.
    /// These are event-history witnesses, not private memory or new world fact.
    pub recent_self_authored_turns: Vec<String>,
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
    /// Typed refusal lane for a directly addressed actor. WorldKernel lowers
    /// it deterministically; this field cannot carry free-form effects.
    #[serde(default)]
    pub deliberate_silence: bool,
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
pub struct CellActivityTargetSlice {
    pub name: String,
    pub locations: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CellMigrationDestinationSlice {
    pub population_name: String,
    pub location_id: String,
    pub location_name: String,
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
    pub reachable_destinations: BTreeMap<String, String>,
    pub migration_destinations: BTreeMap<String, CellMigrationDestinationSlice>,
    pub activity_targets: BTreeMap<String, CellActivityTargetSlice>,
    pub goals: Vec<String>,
    pub relationships: BTreeMap<String, String>,
    pub memories: Vec<String>,
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
    pub migration_destinations: BTreeMap<String, CellMigrationDestinationSlice>,
    pub activity_targets: BTreeMap<String, CellActivityTargetSlice>,
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
pub struct CellCausalFollowThroughSlice {
    pub anchor_reference: String,
    pub responder_subject_id: String,
    pub summary: String,
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
    /// Scheduler-selected historical or durable pressure that owns an exact
    /// response window in this cell. It is context and provenance, never a
    /// prescribed action or outcome.
    pub causal_follow_through: Vec<CellCausalFollowThroughSlice>,
    pub world_clock_pressure: Vec<String>,
    /// Canonical names for the exact current locations already represented by
    /// this slice. This is identity context only; it grants no movement,
    /// activity, or mutation authority.
    pub canonical_locations: BTreeMap<String, String>,
    pub detail_focus_subject_id: Option<String>,
    /// Exact subjects that own this cell's bounded decision slots for the
    /// current strategic wave. Resolution chooses these before inference;
    /// Projector output may describe them but may not decide who participates.
    pub decision_owner_ids: BTreeSet<String>,
    pub max_actions: usize,
    pub source_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CellTerminalBundle {
    pub lived_stream: LivedNarrativeStream,
    pub persona_output: String,
    pub appraisal: crate::domain::CellAppraisal,
    pub stage_receipts: Vec<crate::model::ModelStageReceipt>,
}

#[derive(Clone, Debug)]
pub struct CellPipelineFailure {
    pub diagnostic: String,
    pub stage_receipts: Vec<crate::model::ModelStageReceipt>,
}

impl std::fmt::Display for CellPipelineFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.diagnostic)
    }
}

impl std::error::Error for CellPipelineFailure {}

fn cell_pipeline_failure(
    error: anyhow::Error,
    mut prior_stage_receipts: Vec<crate::model::ModelStageReceipt>,
) -> anyhow::Error {
    if let Some(failure) = error.downcast_ref::<CellPipelineFailure>() {
        prior_stage_receipts.extend(failure.stage_receipts.clone());
    } else if let Some(omission) = error.downcast_ref::<MissingExplicitCellDecision>() {
        prior_stage_receipts.extend(omission.stage_receipts.clone());
    }
    anyhow::Error::new(CellPipelineFailure {
        diagnostic: format!("{error:#}").chars().take(4_000).collect(),
        stage_receipts: distinct_stage_receipts(prior_stage_receipts),
    })
}

fn distinct_stage_receipts(
    receipts: Vec<crate::model::ModelStageReceipt>,
) -> Vec<crate::model::ModelStageReceipt> {
    let mut seen = BTreeSet::new();
    receipts
        .into_iter()
        .filter(|receipt| seen.insert(receipt.storage_key().to_owned()))
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellAppraisalProposal {
    actions: Vec<CellActionCandidate>,
    inactions: Vec<crate::domain::CellInaction>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CellInterpreterAgentCommand {
    Submit {
        decisions: BTreeMap<String, serde_json::Value>,
    },
    UpsertDecision {
        subject_id: String,
        decision: serde_json::Value,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellInterpreterAgentAction {
    command: CellInterpreterAgentCommand,
}

#[derive(Debug)]
enum CellInterpreterAgentOutput {
    Appraisal(crate::domain::CellAppraisal),
    MissingPersonaDecision { subject_ids: Vec<String> },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CellInterpreterFinding {
    DraftProgress {
        decision_subject_ids: Vec<String>,
        missing_subject_ids: Vec<String>,
    },
    UnknownDecisionOwner {
        subject_id: String,
        allowed_subject_ids: Vec<String>,
    },
    SubmitRequiresEmptyDraft {
        repair_subject_ids: Vec<String>,
    },
    DecisionNotRepairable {
        subject_id: String,
        repair_subject_ids: Vec<String>,
    },
    LocalValidation {
        diagnostic: String,
        decision_subject_ids: Vec<String>,
    },
    EffectMismatch {
        rejected: Vec<CellInterpreterEffectFinding>,
    },
}

#[derive(Clone, Debug, Serialize)]
struct CellInterpreterEffectFinding {
    subject_id: String,
    mismatch_kind: CellEffectMismatchKind,
    repair_guidance: String,
}

#[derive(Debug)]
struct MissingExplicitCellDecision {
    cell_id: String,
    stage_receipts: Vec<crate::model::ModelStageReceipt>,
    lived_stream: LivedNarrativeStream,
    active_subject_ids: BTreeSet<String>,
    projector_receipts: Vec<crate::model::ModelStageReceipt>,
}

impl std::fmt::Display for MissingExplicitCellDecision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "cell {} Persona supplied no explicit attributed action or inaction",
            self.cell_id
        )
    }
}

impl std::error::Error for MissingExplicitCellDecision {}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellActionCandidate {
    subject_id: String,
    intent: String,
    intended_effect: String,
    priority: i16,
    state_references: Vec<String>,
    public_channels: Vec<String>,
    effects: CellEffectBundleCandidate,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellEffectBundleCandidate {
    institution: Option<CellInstitutionEffectCandidate>,
    gestalt_pressure: Option<CellGestaltPressureEffectCandidate>,
    gestalt_activities: Option<CellActivitySetCandidate>,
    gestalt_migration: Option<CellMigrationEffectCandidate>,
    actor_move: Option<CellActorMoveEffectCandidate>,
    actor_activities: Option<CellActivitySetCandidate>,
    member_activities: Option<CellActivitySetCandidate>,
    member_migration: Option<CellMigrationEffectCandidate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellInstitutionEffectCandidate {
    #[schemars(length(min = 1, max = 240))]
    posture: String,
    location_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellGestaltPressureEffectCandidate {
    #[serde(default)]
    pressure_additions: Vec<String>,
    #[serde(default)]
    pressure_resolutions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellActivityScopeCandidate {
    #[serde(default)]
    target_subject_ids: Vec<String>,
    #[serde(default)]
    location_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellActivitySetCandidate {
    prepare: Option<Vec<CellActivityScopeCandidate>>,
    coordinate: Option<Vec<CellActivityScopeCandidate>>,
    investigate: Option<Vec<CellActivityScopeCandidate>>,
    recruit: Option<Vec<CellActivityScopeCandidate>>,
    obstruct: Option<Vec<CellActivityScopeCandidate>>,
    trade: Option<Vec<CellActivityScopeCandidate>>,
    communicate: Option<Vec<CellActivityScopeCandidate>>,
}

impl CellActivitySetCandidate {
    fn into_effects(
        self,
    ) -> impl Iterator<
        Item = (
            crate::domain::StrategicActivityKind,
            CellActivityScopeCandidate,
        ),
    > {
        [
            (crate::domain::StrategicActivityKind::Prepare, self.prepare),
            (
                crate::domain::StrategicActivityKind::Coordinate,
                self.coordinate,
            ),
            (
                crate::domain::StrategicActivityKind::Investigate,
                self.investigate,
            ),
            (crate::domain::StrategicActivityKind::Recruit, self.recruit),
            (
                crate::domain::StrategicActivityKind::Obstruct,
                self.obstruct,
            ),
            (crate::domain::StrategicActivityKind::Trade, self.trade),
            (
                crate::domain::StrategicActivityKind::Communicate,
                self.communicate,
            ),
        ]
        .into_iter()
        .flat_map(|(activity, scopes)| {
            scopes
                .into_iter()
                .flatten()
                .map(move |scope| (activity.clone(), scope))
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellMigrationEffectCandidate {
    destination_gestalt_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellActorMoveEffectCandidate {
    destination_id: String,
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
    findings: Vec<CellEffectMismatchFinding>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct CellEffectMismatchFinding {
    mismatch_kind: CellEffectMismatchKind,
    repair_guidance: String,
}

struct CellActionVerificationRun {
    action_index: usize,
    output: ModelStageOutput,
    verdict: CellActionEffectVerdict,
}

#[derive(Debug)]
struct CellEffectVerifierWaveFailure {
    diagnostics: Vec<String>,
    completed_stage_receipts: Vec<crate::model::ModelStageReceipt>,
}

impl std::fmt::Display for CellEffectVerifierWaveFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "cell effect verifier wave failed: {}",
            self.diagnostics.join("; ")
        )
    }
}

impl std::error::Error for CellEffectVerifierWaveFailure {}

#[derive(Debug)]
struct CellEffectVerifierTaskFailure {
    diagnostic: String,
    completed_stage_receipt: Option<crate::model::ModelStageReceipt>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
enum CellEffectMatchResult {
    Match,
    Mismatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
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

const CELL_EFFECT_VERIFIER_INSTRUCTIONS: &str = "You are the private semantic verifier between an Interpreter and the world kernel. Judge this one candidate action's typed effects as one composition against the exact attributed subject's choice in the Persona turn. Structural permissions were already checked. Use exact_subject_permission as the sole map of canonical subjects, locations, and destinations for this actor. canonical_locations is sibling identity context: compare it with exact_subject_permission.location_ids, and never treat a name as granting locality, co-presence, reach, publication, movement, target, effect, or mutation authority. A matching current-location name denotes that place or its unnamed local public, not an omitted canonical subject. activity_targets supplies each canonical target's exact name and current locations; reachable_destinations supplies exact actor-movement destination IDs and names; migration_destinations supplies exact population names and locations. Preserve every distinct affirmative means the subject actually chooses. Do not invent another means from a purpose, refusal, restraint, condition to preserve, desired social norm, or hoped-for state. Keeping someone's choice open, declining to coerce them, leaving state unchanged, respecting autonomy, or waiting for another subject's decision requires no additional typed effect unless the Persona separately chooses an observable act to do it. Communication can therefore be faithfully combined with restraint without implying coordinate, recruit, posture, or pressure effects. Reject effect omission only when the Persona explicitly undertakes another observable act. When one choice contains relocation, an activity at the subject's exact snapshot location occurs before relocation and an activity at the exact admitted destination occurs after arrival; activities within one location phase are an unordered atomic set. Array and object field order are not chronology. When the Persona chooses to go to a canonical target, actor_move must use that target's actual different reachable location. If actor and target are already co-located, reject movement to some other place; a local communicate, coordinate, or prepare may encode the stated attempt instead. A place named only in prose and absent from reachable_destinations and migration_destinations is local texture inside the supplied activity location; walking to it cannot justify rejecting a concrete local prepare or repair as omitted travel. Return exactly one verdict with action_index 0. A gestalt_migration means that exact population leaf chooses to travel together to the supplied destination within the strategic horizon; loading, waiting, giving away passage, sending only some other subject, or merely considering travel does not entail it. Conversely, when the population chooses to board, depart, or relocate together, reject gestalt_activity prepare that erases the chosen journey. Gestalt migration never entails that a named member moved. A member_migration means that named member personally chooses to travel to the destination. Boarding a transport whose supplied destination is unambiguous in the lived stream is a chosen journey; the Persona need not repeat the place name. Giving away a berth, sending somebody else, waiting, or merely considering travel does not entail migration. Conversely, when the member chooses to board, depart, travel, or join the supplied destination, reject member_activity that reduces that commitment to preparing, queuing, or approaching. A member_activity belongs only to that exact named person's stated attempt; it cannot be reassigned to their population. Communication targets must be the exact canonical subjects actually addressed in the Persona turn. An exact activity_targets entry is sufficient authority to attempt direct communication with that named subject; allowed_persistent_publication_channels governs only durable public publication and is never an additional requirement for direct contact. One communicate activity is also the complete supported composition when the same utterance addresses exact canonical targets and an unnamed public audience: target_subject_ids names the canonical addressees and candidate_action.public_channels names the simultaneous public reach. A call to unnamed people at a canonical location is such public or local audience, not a missing activity target. Do not demand a second targetless communicate for that same utterance. Use a targetless communicate only when the communication has no exact canonical addressee. Internal-population coordination is owner-specific: apply only coordination_target_contract.rule for this exact attributed subject, never a rule belonging to another subject kind. If the Persona addresses an unnamed clerk, dock master, passerby, or local environment, reject any effect that substitutes a containing population, related institution, or merely permitted ID. A targetless local investigate at the subject's exact current or paired movement destination is the faithful supported shape for seeking information from an unnamed role or the environment; its empty target list is intentional and must not itself be grounds for rejection. An institution posture must express its stated commitment or withholding. A gestalt pressure resolution must be causally supported by its stated attempt, and an added pressure must be a resulting unresolved condition rather than completed-action prose. An activity records only the exact attempt—never successful preparation, coordination, discovery, recruitment, obstruction, exchange, delivery, persuasion, acceptance, or target response. Reject omissions, reversals, subject swaps, wrong destinations, wishful outcomes, and effects that the Persona did not choose. Be concise. Return exactly one JSON object. A faithful verdict uses result \"match\" and an empty findings array. Otherwise use result \"mismatch\" and return the complete bounded set of distinct mismatches you can identify in this same action; do not reveal one defect per pass. Every finding needs mismatch_kind (\"subject_swap\", \"effect_omission\", \"effect_reversal\", \"target_substitution\", \"invented_outcome\", or \"wrong_effect_kind\") and one concrete repair_guidance sentence of at most 240 characters. Name the exact omitted choice, substituted target, or wrong destination. When no supplied typed effect composition can faithfully encode the choice, explicitly say to remove the action rather than downgrade or redirect it. Shape: {\"verdicts\":[{\"action_index\":0,\"result\":\"match\",\"findings\":[]}]}";

const CELL_ACTIVITY_CLASSIFICATION_GUIDANCE: &str = "Classify the chosen means by what the subject actually does. communicate means speak, send, offer, ask, or notify; coordinate means arrange a joint attempt; investigate means inspect, examine, diagnose, measure, test, or assess an existing condition in order to learn; recruit means invite; trade means offer an exchange; obstruct means attempt interference. prepare means materially repair, build, arrange, or ready a bounded resource, capability, or plan. Merely inspecting a handcart, regulator, record, route, patient, or other existing condition before deciding what to do is investigate, not prepare. Only actual repair or readiness work is prepare.";

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
                    domain_guidance: "Respond as a situated character. When the lived stream says a response is expected from you, make your answer, refusal, or deliberate silence observable now. Speech and attempted effects are distinct; the world kernel resolves consequences. Asking, inviting, persuading, threatening, or demanding completes only your own speech: never supply the other person's answer, choice, consent, belief, disclosure, or obedience.",
                    word_budget: 160,
                }),
                output_schema: None,
                source_receipt_ids: causal_source_ids(
                    &slice.source_receipt_ids,
                    std::slice::from_ref(&projected.receipt),
                ),
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
        let mut request = ModelStageRequest {
            stage: "interpreter".into(),
            model: self.interpreter_model.clone(),
            snapshot_binding: slice.snapshot_binding.clone(),
            lived_stream: build_interpreter_prompt(&InterpreterPrompt {
                identity: &slice.actor_id,
                typed_context: &typed_context,
                lived_stream: &lived.text,
                persona_output: &persona.narrative,
                output_schema: Some(&serde_json::to_string(&prompt_schema)?),
                domain_guidance: &permission_guidance,
            }),
            output_schema: Some(schema),
            source_receipt_ids: causal_source_ids(
                &slice.source_receipt_ids,
                &[projected.receipt.clone(), persona.receipt.clone()],
            ),
            temperature: Some(0.0),
            max_output_tokens: Some(768),
        };
        let mut interpreter_receipts = Vec::new();
        let proposals = loop {
            let mut interpreted = run_validated_stage(self.model.as_ref(), &request).await?;
            let structured = interpreted
                .structured
                .clone()
                .ok_or_else(|| anyhow!("interpreter produced no typed proposal"))?;
            let proposals: PersonaProposalBundle = serde_json::from_value(structured.clone())?;
            match validate_actor_proposals(&slice, &proposals) {
                Ok(()) => {
                    interpreter_receipts.push(interpreted.receipt);
                    break proposals;
                }
                Err(error) if interpreter_receipts.is_empty() => {
                    interpreted.receipt.validation_result = "semantic_invalid".into();
                    interpreted.receipt.local_validation_error =
                        Some(error.to_string().chars().take(1_000).collect());
                    interpreter_receipts.push(interpreted.receipt);
                    self.permit
                        .require(&slice.actor_id, &slice.snapshot_binding, "interpreter")
                        .await?;
                    append_actor_correction(&mut request, &error, &structured);
                }
                Err(error) => {
                    return Err(anyhow!(
                        "actor interpreter failed semantic validation after one correction: {error}"
                    ));
                }
            }
        };
        self.permit
            .require(&slice.actor_id, &slice.snapshot_binding, "terminal")
            .await?;
        let mut stage_receipts = vec![projected.receipt, persona.receipt];
        stage_receipts.extend(interpreter_receipts);
        Ok(PersonaTerminalBundle {
            lived_stream: lived,
            persona_output: persona.narrative,
            proposals,
            stage_receipts,
        })
    }
}

fn actor_interpreter_guidance(slice: &PermittedActorSlice) -> String {
    let response_guidance = match slice.interaction_role {
        ActorInteractionRole::DirectResponseExpected => {
            "This actor is an exact direct addressee. Extract an explicit spoken response, or set deliberate_silence true only when the Persona deliberately refuses or remains silent. Do not erase the Persona's response into null fields."
        }
        ActorInteractionRole::PresentObserver => {
            "This actor is present but was not directly asked to respond. Speech or observable action remains optional and must follow only from this actor's projected choice."
        }
    };
    let ownership_guidance = match slice.subject_kind {
        PersonaSubjectKind::IndividualActor => {
            "Record only private changes supported by the lived stream and typed context. identity_adoption is null unless the Persona explicitly adopts or presents one public self-identifier in its own speech; when set, copy the exact spoken handle and nothing else. World actions are attempts, not completed effects."
        }
        PersonaSubjectKind::CohesiveGestalt => {
            "This is a cohesive population appraisal. Extract only collective speech or deliberate silence. Every actor-private delta must remain empty, identity_adoption must be null, and world_actions must be empty; strategic population action uses the cell-resolution path, not foreground dialogue."
        }
    };
    format!(
        "{response_guidance} {ownership_guidance} Speech is extracted separately and is already complete. Do not emit a world action merely to make another actor answer, choose, consent, believe, disclose, feel, or obey; the other actor retains agency and any requested response remains unresolved. actor_id must be {:?}. Exact allowed state references are {:?}. Relationship update keys may only be {:?}.",
        slice.actor_id,
        allowed_actor_references(slice),
        slice.perceived_actors.keys().collect::<Vec<_>>()
    )
}

fn ground_actor_lived_stream(slice: &PermittedActorSlice, projection: &str) -> String {
    let identity = if slice.identity_experience.is_empty() {
        "no more specific self-identity than your exact actor identity".to_owned()
    } else {
        slice.identity_experience.join("; ")
    };
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
    let recent_memory_start = slice.memories.len().saturating_sub(8);
    let remembered_experience = if slice.memories.is_empty() {
        "no additional autobiographical memory is active in this moment".to_owned()
    } else {
        slice.memories[recent_memory_start..].join("; ")
    };
    let prior_public_response = if slice.recent_self_authored_turns.is_empty() {
        "no earlier public response by this exact subject is active in this moment".to_owned()
    } else {
        slice.recent_self_authored_turns.join("; ")
    };
    let interaction = match slice.interaction_role {
        ActorInteractionRole::DirectResponseExpected => {
            "You are an exact direct addressee of the current speech. A response is expected from you now. You retain the agency to answer, refuse, or remain silent, but refusal or silence must be made observable rather than disappearing from the scene."
        }
        ActorInteractionRole::PresentObserver => {
            "You are present and perceive the current event, but you were not directly asked to respond. You may react from your own goals and pressures; you need not seize conversational focus."
        }
    };
    let established_peer_identities = if slice.reserved_public_identities.is_empty() {
        "no other established public self-identifiers are active in your immediate social context"
            .to_owned()
    } else {
        format!(
            "these public self-identifiers already belong to other durable people in your immediate social context: {}",
            slice
                .reserved_public_identities
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let subject_grounding = match slice.subject_kind {
        PersonaSubjectKind::IndividualActor => {
            "You are one situated person. Speak and choose only for yourself."
        }
        PersonaSubjectKind::CohesiveGestalt => {
            "You are a cohesive population Persona. You may speak plurally only from genuinely shared state. Do not invent unanimity about exceptions, speak for named individuals, or claim authority outside the supplied collective."
        }
    };
    format!(
        "{projection}\n\n{subject_grounding} Your active self-identity: {identity}. In your social world, {established_peer_identities}. {interaction} Your reliable footing in this moment is narrow. What you know as external fact: {reliable_knowledge}. What you remember experiencing or being told: {remembered_experience}. These are your attributed recollections, not omniscient proof. Your exact recent public response history: {prior_public_response}. This response history records what you previously made observable; it does not prove that the content of those statements was true. What is happening now: {visible_now}. People you can presently perceive: {people_now}. Everything else in your impressions is feeling, inference, uncertainty, or possibility—not a remembered or witnessed fact."
    )
}

fn validate_actor_proposals(
    slice: &PermittedActorSlice,
    proposals: &PersonaProposalBundle,
) -> Result<()> {
    if matches!(slice.subject_kind, PersonaSubjectKind::CohesiveGestalt)
        && (proposals.private_delta != crate::domain::ActorStateDelta::default()
            || !proposals.world_actions.is_empty())
    {
        return Err(anyhow!(
            "cohesive foreground Gestalt appraisal may emit only speech or deliberate silence"
        ));
    }
    if matches!(
        slice.interaction_role,
        ActorInteractionRole::DirectResponseExpected
    ) && proposals
        .speech
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
        && !proposals.deliberate_silence
    {
        return Err(anyhow!(
            "directly addressed Persona produced no observable response"
        ));
    }
    if proposals
        .speech
        .as_deref()
        .is_some_and(|value| value.trim().is_empty() || value.chars().count() > 1_000)
    {
        return Err(anyhow!("Persona speech must contain 1 to 1000 characters"));
    }
    if let Some(identity) = proposals.private_delta.identity_adoption.as_deref() {
        let identity = identity.trim();
        if identity.is_empty() || identity.chars().count() > 160 {
            return Err(anyhow!(
                "Persona identity adoption must contain 1 to 160 characters"
            ));
        }
        let speech = proposals
            .speech
            .as_deref()
            .ok_or_else(|| anyhow!("Persona identity adoption requires public speech"))?;
        if !speech.to_lowercase().contains(&identity.to_lowercase()) {
            return Err(anyhow!(
                "Persona identity adoption must copy an exact spoken handle"
            ));
        }
        if slice
            .reserved_public_identities
            .iter()
            .any(|reserved| reserved.trim().to_lowercase() == identity.to_lowercase())
        {
            return Err(anyhow!(
                "Persona identity adoption conflicts with an established peer identity"
            ));
        }
    }
    Ok(())
}

fn append_actor_correction(
    request: &mut ModelStageRequest,
    error: &anyhow::Error,
    rejected: &serde_json::Value,
) {
    request.lived_stream.push_str(&format!(
        "\n\nCORRECTION TASK—THE PREVIOUS INTERPRETATION WAS REJECTED.\nREJECTION: {error}\nPREVIOUS_REJECTED_INTERPRETATION:\n{}\nReturn one corrected complete interpretation against the same snapshot, lived stream, Persona output, and exact permissions. Preserve supported speech and private changes. Do not invent speech to justify a delta. If the Persona did not explicitly adopt or present a public self-identifier in its own speech, identity_adoption must be null. If the Persona explicitly claimed an established peer identity, do not hide the conflict by nulling identity_adoption; preserve the extracted claim so the unchanged Persona turn remains rejected.",
        serde_json::to_string(rejected).unwrap_or_else(|_| "null".into())
    ));
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
            "reachable_destinations": subject.reachable_destinations,
            "migration_destinations": subject.migration_destinations,
            "activity_targets": subject.activity_targets,
            "goals": subject.goals,
            "relationships": subject.relationships,
            "memories": subject.memories,
            "already_committed_posture": subject.current_posture,
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
            "activity_targets": member.activity_targets,
            "goals": member.goals,
            "pressures": member.pressures,
            "relationships": member.relationships,
            "memories": member.memories,
        })).collect::<Vec<_>>(),
        "shared_knowledge": slice.shared_knowledge,
        "shared_capabilities": slice.shared_capabilities,
        "perceived_events": slice.perceived_events,
        "causal_follow_through": slice.causal_follow_through,
        "world_clock_pressure": slice.world_clock_pressure,
        "canonical_locations": slice.canonical_locations,
        "detail_focus_subject_id": slice.detail_focus_subject_id,
        "max_actions": slice.max_actions,
    })
}

fn cell_interpreter_context(
    slice: &PermittedCellSlice,
    active_subject_ids: &BTreeSet<String>,
) -> serde_json::Value {
    serde_json::json!({
        "cell_id": slice.cell_id,
        "mode": slice.mode,
        "world_revision": slice.world_revision,
        "resolution_epoch": slice.resolution_epoch,
        "canonical_locations": slice.canonical_locations,
        "detail_focus_subject_id": slice.detail_focus_subject_id,
        "causal_follow_through": slice.causal_follow_through,
        "max_actions": slice.max_actions,
        "exact_permissions": slice.constituents.iter().filter(|subject| active_subject_ids.contains(&subject.subject_id)).map(constituent_permission_context).collect::<Vec<_>>(),
        "member_permissions": slice.member_exceptions.iter().filter(|member| active_subject_ids.contains(&member.subject_id)).map(member_permission_context).collect::<Vec<_>>(),
    })
}

fn constituent_permission_context(subject: &CellConstituentSlice) -> serde_json::Value {
    serde_json::json!({
        "subject_id": subject.subject_id,
        "subject_kind": subject.subject_kind,
        "name": subject.name,
        "allowed_effect_types": allowed_constituent_effect_types(subject),
        "collective_authority_id": subject.collective_authority_id,
        "location_ids": subject.location_ids,
        "allowed_persistent_publication_channels": subject.information_channels,
        "permitted_state_references": subject.permitted_state_references,
        "reachable_destinations": subject.reachable_destinations,
        "migration_destinations": subject.migration_destinations,
        "activity_targets": subject.activity_targets,
        "already_committed_posture": subject.current_posture,
        "current_pressures": subject.pressures,
    })
}

fn member_permission_context(member: &CellMemberSlice) -> serde_json::Value {
    serde_json::json!({
        "subject_id": member.subject_id,
        "subject_kind": "gestalt_member",
        "member_id": member.member_id,
        "name": member.name,
        "allowed_effect_types": allowed_member_effect_types(member),
        "source_gestalt_id": member.source_gestalt_id,
        "source_location_id": member.source_location_id,
        "allowed_persistent_publication_channels": member.information_channels,
        "permitted_state_references": member.permitted_state_references,
        "migration_destinations": member.migration_destinations,
        "activity_targets": member.activity_targets,
    })
}

fn cell_action_verifier_permission(
    slice: &PermittedCellSlice,
    subject_id: &str,
) -> Result<serde_json::Value> {
    if let Some(subject) = slice
        .constituents
        .iter()
        .find(|subject| subject.subject_id == subject_id)
    {
        return Ok(constituent_permission_context(subject));
    }
    if let Some(member) = slice
        .member_exceptions
        .iter()
        .find(|member| member.subject_id == subject_id)
    {
        return Ok(member_permission_context(member));
    }
    Err(anyhow!(
        "cell effect verifier cannot find exact authority for {subject_id}"
    ))
}

fn coordination_target_contract(exact_subject_permission: &serde_json::Value) -> serde_json::Value {
    if let Some(source_gestalt_id) = exact_subject_permission
        .get("source_gestalt_id")
        .and_then(serde_json::Value::as_str)
    {
        return serde_json::json!({
            "owner_kind":"gestalt_member",
            "internal_population_target_subject_ids":[source_gestalt_id],
            "rule":format!("For this named member, coordinating their source population is encoded by targeting exactly {source_gestalt_id}. A targetless coordinate would omit that chosen population target.")
        });
    }
    if exact_subject_permission.get("subject_kind") == Some(&serde_json::json!("gestalt")) {
        return serde_json::json!({
            "owner_kind":"gestalt",
            "internal_population_target_subject_ids":[],
            "rule":"For this cohesive Gestalt owner, coordinating its own unnamed members is encoded as a targetless local coordinate. Named external targets remain explicit."
        });
    }
    serde_json::json!({
        "owner_kind":exact_subject_permission.get("subject_kind"),
        "internal_population_target_subject_ids":null,
        "rule":"This subject has no special internal-population coordination encoding. Preserve only exact addressed activity targets."
    })
}

fn cell_scene_boundaries(
    slice: &PermittedCellSlice,
    active_subject_ids: &BTreeSet<String>,
) -> String {
    let mut by_location = BTreeMap::<String, BTreeSet<String>>::new();
    let mut unlocated = BTreeSet::new();
    let mut perspective_owners = BTreeSet::new();
    for subject in slice
        .constituents
        .iter()
        .filter(|subject| active_subject_ids.contains(&subject.subject_id))
    {
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
    for member in slice
        .member_exceptions
        .iter()
        .filter(|member| active_subject_ids.contains(&member.subject_id))
    {
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
        "Only these projected perspectives may speak, choose, or receive an action or inaction record in this turn: {}. Every other constituent remains canonically represented by the cell but is inactive in this inference wave.",
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
    let required_count = required_projection_subject_ids(slice).len();
    if required_count == 0 || required_count > slice.max_actions {
        return Err(anyhow!(
            "resolution selected an invalid number of cell decision owners"
        ));
    }
    segments.insert("minItems".into(), required_count.into());
    segments.insert("maxItems".into(), required_count.into());
    let segment = schema
        .pointer_mut("/$defs/CellPerspectiveSegment/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("cell projection schema has no segment properties"))?;
    let mut subject_ids = slice
        .decision_owner_ids
        .iter()
        .map(String::as_str)
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
) -> Result<(String, BTreeSet<String>)> {
    let required_subject_ids = required_projection_subject_ids(slice);
    if proposal.segments.is_empty()
        || proposal.segments.len() > slice.max_actions.max(required_subject_ids.len()).max(1)
    {
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
    for subject_id in required_subject_ids {
        if !segments.contains_key(&subject_id) {
            return Err(anyhow!(
                "cell Projector omitted required perspective owner {subject_id}"
            ));
        }
    }
    let active_subject_ids = segments.keys().cloned().collect::<BTreeSet<_>>();
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
                "{} — {}:\n{}\n\n{}",
                subject.name,
                footing,
                narrative.trim(),
                constituent_agency_footing(subject)
            ));
        } else {
            let member = slice
                .member_exceptions
                .iter()
                .find(|member| member.subject_id == subject_id)
                .expect("projection owner was validated");
            lowered.push(format!(
                "{} — at {}:\n{}\n\n{}",
                member.name,
                member.source_location_id,
                narrative.trim(),
                member_agency_footing(member)
            ));
        }
    }
    Ok((lowered.join("\n\n"), active_subject_ids))
}

fn constituent_agency_footing(subject: &CellConstituentSlice) -> String {
    let destinations = subject
        .reachable_destinations
        .values()
        .cloned()
        .chain(subject.migration_destinations.values().map(|destination| {
            format!(
                "{} at {}",
                destination.population_name, destination.location_name
            )
        }))
        .collect::<Vec<_>>();
    agency_footing(
        &subject.name,
        &destinations,
        &subject.activity_targets,
        &subject.information_channels,
    )
}

fn member_agency_footing(member: &CellMemberSlice) -> String {
    let destinations = member
        .migration_destinations
        .values()
        .map(|destination| {
            format!(
                "{} at {}",
                destination.population_name, destination.location_name
            )
        })
        .collect::<Vec<_>>();
    agency_footing(
        &member.name,
        &destinations,
        &member.activity_targets,
        &member.information_channels,
    )
}

fn agency_footing(
    subject_name: &str,
    destinations: &[String],
    targets: &BTreeMap<String, CellActivityTargetSlice>,
    publication_channels: &BTreeSet<String>,
) -> String {
    let destinations = if destinations.is_empty() {
        "No distinct travel destination is presently established. You may seek a route from where you are, but you cannot assume arrival somewhere unnamed."
            .to_owned()
    } else {
        format!(
            "The distinct destinations you can presently choose to travel to are: {}. Somewhere else would first require finding a route from where you are.",
            destinations.join(", ")
        )
    };
    let targets = if targets.is_empty() {
        "No named distant person or body is presently available to contact. You may still speak to an ordinary unnamed person nearby or work on the local environment."
            .to_owned()
    } else {
        format!(
            "The named people or bodies you can presently try to reach are: {}. You may still speak to an ordinary unnamed person nearby or work on the local environment.",
            targets
                .values()
                .map(|target| {
                    let locations = target.locations.values().cloned().collect::<Vec<_>>();
                    if locations.is_empty() {
                        target.name.clone()
                    } else {
                        format!("{} at {}", target.name, locations.join(", "))
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let channels = if publication_channels.is_empty() {
        "You have no established channel for publishing this attempt beyond its immediate witnesses."
            .to_owned()
    } else {
        format!(
            "Your established channels for publishing an attempt are: {}.",
            publication_channels
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!("{subject_name}'s grounded agency: {destinations} {targets} {channels}")
}

struct CellInterpreterWorkbench {
    model: Arc<dyn ModelPort>,
    permit: Arc<dyn ExecutionPermit>,
    interpreter_model: String,
    slice: PermittedCellSlice,
    active_subject_ids: BTreeSet<String>,
    lived_stream: String,
    persona_turn: String,
    campaign_policy: String,
    appraisal_schema: serde_json::Value,
    draft: BTreeMap<String, serde_json::Value>,
    repair_subject_ids: BTreeSet<String>,
    accepted_verifier_bindings: BTreeSet<String>,
}

impl CellInterpreterWorkbench {
    fn progress(&self) -> CellInterpreterFinding {
        let decision_subject_ids = self.draft.keys().cloned().collect::<Vec<_>>();
        let decided = decision_subject_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        CellInterpreterFinding::DraftProgress {
            decision_subject_ids,
            missing_subject_ids: self
                .active_subject_ids
                .difference(&decided)
                .cloned()
                .collect(),
        }
    }

    async fn compile_draft(
        &mut self,
        source_receipt_ids: &[String],
    ) -> ModelAgentToolOutcome<CellInterpreterAgentOutput, CellInterpreterFinding> {
        let undecided_subject_ids = self
            .draft
            .iter()
            .filter_map(|(subject_id, decision)| {
                decision
                    .get("undecided")
                    .is_some()
                    .then_some(subject_id.clone())
            })
            .collect::<Vec<_>>();
        if !undecided_subject_ids.is_empty() {
            return ModelAgentToolOutcome::Accepted {
                output: CellInterpreterAgentOutput::MissingPersonaDecision {
                    subject_ids: undecided_subject_ids,
                },
                receipts: Vec::new(),
            };
        }
        if !self
            .active_subject_ids
            .iter()
            .all(|subject_id| self.draft.contains_key(subject_id))
        {
            self.repair_subject_ids = self
                .active_subject_ids
                .iter()
                .filter(|subject_id| !self.draft.contains_key(*subject_id))
                .cloned()
                .collect();
            return ModelAgentToolOutcome::Continue {
                observation: self.progress(),
                receipts: Vec::new(),
            };
        }

        let value = serde_json::json!({"decisions":self.draft});
        let appraisal =
            decode_cell_appraisal_proposal(&self.slice.cell_id, value).and_then(|proposal| {
                let appraisal =
                    bind_cell_appraisal(&self.slice, &self.active_subject_ids, proposal)?;
                validate_cell_appraisal(&self.slice, &appraisal)?;
                validate_active_decision_owners(&self.active_subject_ids, &appraisal)?;
                Ok(appraisal)
            });
        let appraisal = match appraisal {
            Ok(appraisal) => appraisal,
            Err(error) => {
                if self.repair_subject_ids.is_empty() {
                    self.repair_subject_ids = self.active_subject_ids.clone();
                }
                return ModelAgentToolOutcome::Rejected {
                    finding: CellInterpreterFinding::LocalValidation {
                        diagnostic: error.to_string().chars().take(1_000).collect(),
                        decision_subject_ids: self.repair_subject_ids.iter().cloned().collect(),
                    },
                    receipts: Vec::new(),
                };
            }
        };

        let mut pending_actions = Vec::new();
        for action in &appraisal.actions {
            let binding = match cell_effect_verification_binding(
                &self.slice.snapshot_binding,
                std::slice::from_ref(action),
            ) {
                Ok(binding) => binding,
                Err(error) => {
                    return ModelAgentToolOutcome::Failed {
                        message: error.to_string(),
                        receipts: Vec::new(),
                    };
                }
            };
            if !self.accepted_verifier_bindings.contains(&binding) {
                pending_actions.push(action.clone());
            }
        }
        if pending_actions.is_empty() {
            self.repair_subject_ids.clear();
            return ModelAgentToolOutcome::Accepted {
                output: CellInterpreterAgentOutput::Appraisal(appraisal),
                receipts: Vec::new(),
            };
        }

        if let Err(error) = self
            .permit
            .require(
                &self.slice.cell_id,
                &self.slice.snapshot_binding,
                "cell_effect_verifier",
            )
            .await
        {
            return ModelAgentToolOutcome::Failed {
                message: error.to_string(),
                receipts: Vec::new(),
            };
        }
        let mut verifications = match run_cell_effect_verifier_wave(
            self.model.clone(),
            &self.interpreter_model,
            &self.slice,
            &self.lived_stream,
            &self.persona_turn,
            &self.campaign_policy,
            &pending_actions,
            source_receipt_ids,
        )
        .await
        {
            Ok(verifications) => verifications,
            Err(error) => {
                let receipts = error
                    .downcast_ref::<CellEffectVerifierWaveFailure>()
                    .map(|failure| failure.completed_stage_receipts.clone())
                    .unwrap_or_default();
                return ModelAgentToolOutcome::Failed {
                    message: error.to_string(),
                    receipts,
                };
            }
        };
        let mut rejected = Vec::new();
        let mut receipts = Vec::with_capacity(verifications.len());
        for verification in &mut verifications {
            let action = &pending_actions[verification.action_index];
            let binding = cell_effect_verification_binding(
                &self.slice.snapshot_binding,
                std::slice::from_ref(action),
            )
            .expect("pending action binding was already computed");
            if matches!(verification.verdict.result, CellEffectMatchResult::Match) {
                self.accepted_verifier_bindings.insert(binding);
            } else {
                let repair_summary = verification
                    .verdict
                    .findings
                    .iter()
                    .map(|finding| finding.repair_guidance.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                verification.output.receipt.validation_result = "semantic_invalid".into();
                verification.output.receipt.local_validation_error =
                    Some(repair_summary.chars().take(1_000).collect());
                rejected.extend(verification.verdict.findings.iter().map(|finding| {
                    CellInterpreterEffectFinding {
                        subject_id: action.subject_id.clone(),
                        mismatch_kind: finding.mismatch_kind.clone(),
                        repair_guidance: finding.repair_guidance.clone(),
                    }
                }));
            }
            receipts.push(verification.output.receipt.clone());
        }
        if rejected.is_empty() {
            self.repair_subject_ids.clear();
            ModelAgentToolOutcome::Accepted {
                output: CellInterpreterAgentOutput::Appraisal(appraisal),
                receipts,
            }
        } else {
            self.repair_subject_ids = rejected
                .iter()
                .map(|finding| finding.subject_id.clone())
                .collect();
            ModelAgentToolOutcome::Rejected {
                finding: CellInterpreterFinding::EffectMismatch { rejected },
                receipts,
            }
        }
    }
}

#[async_trait]
impl ModelAgentTool for CellInterpreterWorkbench {
    type Action = CellInterpreterAgentAction;
    type Output = CellInterpreterAgentOutput;
    type Finding = CellInterpreterFinding;

    fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
        let state = if self.draft.is_empty() {
            CellInterpreterSchemaState::Initial
        } else if self.repair_subject_ids.is_empty() {
            return Err("nonterminal Interpreter draft has no repair owner".into());
        } else {
            CellInterpreterSchemaState::Repair(&self.repair_subject_ids)
        };
        cell_interpreter_agent_schema(&self.appraisal_schema, state)
            .map_err(|error| error.to_string())
    }

    async fn invoke(
        &mut self,
        action: Self::Action,
        context: &ModelAgentToolContext,
    ) -> ModelAgentToolOutcome<Self::Output, Self::Finding> {
        match action.command {
            CellInterpreterAgentCommand::Submit { decisions } => {
                if !self.draft.is_empty() {
                    return ModelAgentToolOutcome::Rejected {
                        finding: CellInterpreterFinding::SubmitRequiresEmptyDraft {
                            repair_subject_ids: self.repair_subject_ids.iter().cloned().collect(),
                        },
                        receipts: Vec::new(),
                    };
                }
                self.repair_subject_ids = decisions.keys().cloned().collect();
                self.draft = decisions;
                self.compile_draft(&context.source_receipt_ids).await
            }
            CellInterpreterAgentCommand::UpsertDecision {
                subject_id,
                decision,
            } => {
                if !self.active_subject_ids.contains(&subject_id) {
                    return ModelAgentToolOutcome::Rejected {
                        finding: CellInterpreterFinding::UnknownDecisionOwner {
                            subject_id,
                            allowed_subject_ids: self.active_subject_ids.iter().cloned().collect(),
                        },
                        receipts: Vec::new(),
                    };
                }
                if self.draft.contains_key(&subject_id)
                    && !self.repair_subject_ids.contains(&subject_id)
                {
                    return ModelAgentToolOutcome::Rejected {
                        finding: CellInterpreterFinding::DecisionNotRepairable {
                            subject_id,
                            repair_subject_ids: self.repair_subject_ids.iter().cloned().collect(),
                        },
                        receipts: Vec::new(),
                    };
                }
                self.draft.insert(subject_id, decision);
                self.compile_draft(&context.source_receipt_ids).await
            }
        }
    }
}

fn required_projection_subject_ids(slice: &PermittedCellSlice) -> BTreeSet<String> {
    slice.decision_owner_ids.clone()
}

fn allowed_constituent_effect_types(subject: &CellConstituentSlice) -> Vec<&'static str> {
    match subject.subject_kind {
        crate::domain::AgencySubjectKind::Actor => {
            let mut types = vec!["actor_activities"];
            if !subject.reachable_destinations.is_empty() {
                types.push("actor_move");
            }
            types
        }
        crate::domain::AgencySubjectKind::Institution => vec!["institution"],
        crate::domain::AgencySubjectKind::Gestalt => {
            let mut types = vec!["gestalt_pressure", "gestalt_activities"];
            if !subject.migration_destinations.is_empty() {
                types.push("gestalt_migration");
            }
            types
        }
    }
}

fn allowed_member_effect_types(member: &CellMemberSlice) -> Vec<&'static str> {
    let mut types = vec!["member_activities"];
    if !member.migration_destinations.is_empty() {
        types.push("member_migration");
    }
    types
}

#[derive(Clone)]
pub struct CellProjectionEngine {
    pub model: Arc<dyn ModelPort>,
    pub permit: Arc<dyn ExecutionPermit>,
    pub projector_model: String,
    pub persona_model: String,
    pub interpreter_model: String,
    pub campaign_contract: Option<CampaignContract>,
    pub aggregate_boundaries: Vec<AggregatedBoundary>,
}

#[derive(Clone)]
struct CellProjectedMoment {
    lived_stream: LivedNarrativeStream,
    active_subject_ids: BTreeSet<String>,
    projector_receipts: Vec<crate::model::ModelStageReceipt>,
    causal_receipts: Vec<crate::model::ModelStageReceipt>,
}

impl CellProjectionEngine {
    pub async fn execute(&self, slice: PermittedCellSlice) -> Result<CellTerminalBundle> {
        match self.execute_once(slice.clone(), false, None).await {
            Ok(bundle) => Ok(bundle),
            Err(error) => {
                let Some(omission) = error.downcast_ref::<MissingExplicitCellDecision>() else {
                    return Err(cell_pipeline_failure(error, Vec::new()));
                };
                let mut prior_receipts = omission.stage_receipts.clone();
                let retry_projection = CellProjectedMoment {
                    lived_stream: omission.lived_stream.clone(),
                    active_subject_ids: omission.active_subject_ids.clone(),
                    projector_receipts: omission.projector_receipts.clone(),
                    causal_receipts: omission.stage_receipts.clone(),
                };
                let mut bundle = match self.execute_once(slice, true, Some(retry_projection)).await
                {
                    Ok(bundle) => bundle,
                    Err(error) => {
                        return Err(cell_pipeline_failure(
                            error.context(
                                "cell Persona supplied no explicit decision after one same-snapshot retry",
                            ),
                            prior_receipts,
                        ));
                    }
                };
                prior_receipts.append(&mut bundle.stage_receipts);
                bundle.stage_receipts = distinct_stage_receipts(prior_receipts);
                Ok(bundle)
            }
        }
    }

    async fn execute_once(
        &self,
        slice: PermittedCellSlice,
        require_explicit_decision: bool,
        projection: Option<CellProjectedMoment>,
    ) -> Result<CellTerminalBundle> {
        let campaign_policy = serde_json::to_string(&serde_json::json!({
            "campaign_contract":self.campaign_contract,
            "aggregate_content_boundaries":self.aggregate_boundaries,
        }))?;
        let CellProjectedMoment {
            lived_stream: lived,
            active_subject_ids,
            projector_receipts,
            causal_receipts,
        } = if let Some(projection) = projection {
            projection
        } else {
            self.permit
                .require(&slice.cell_id, &slice.snapshot_binding, "cell_projector")
                .await?;
            let projector_context = serde_json::to_string(&cell_projector_context(&slice))?;
            let visible_stimulus = slice
                .causal_follow_through
                .iter()
                .map(|focus| {
                    format!(
                        "Causal response window for {} [{}]: {}",
                        focus.responder_subject_id, focus.anchor_reference, focus.summary
                    )
                })
                .chain(slice.perceived_events.iter().map(|event| {
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
                }))
                .collect::<Vec<_>>()
                .join("\n");
            let mode_guidance = cell_projector_mode_guidance(&slice.mode);
            let mode_guidance = format!(
                "{mode_guidance} Treat already_committed_posture as an institutional course already in force, not a pressure, option, or fresh decision. Continuing it is holding steady; only a materially different commitment is a new posture choice. Each perceived event names the exact constituents that can perceive it; do not teach it to anyone else. A causal response window means the scheduler selected that exact committed anchor as decision-relevant for that exact responder; render the responder confronting it without prescribing action, emotion, success, or another subject's reply. Only supplied constituents and member_exceptions may own an internal perspective or choice. A person merely mentioned in an event is external observation when absent from those lists: never voice them. Every supplied member_exception was selected because that person has an actionable decision in this horizon. Render each selected person explicitly by name, with only their own footing and choices."
            );
            let required_projection_subject_ids = required_projection_subject_ids(&slice);
            let word_budget =
                (120 + 45 * (slice.constituents.len() + slice.member_exceptions.len())).min(360);
            let perspective_limit = slice
                .max_actions
                .max(required_projection_subject_ids.len())
                .max(1);
            let mut projection_schema = serde_json::to_value(schema_for!(CellProjectionProposal))?;
            constrain_cell_projection_schema(&mut projection_schema, &slice)?;
            let mut projection_request = ModelStageRequest {
                stage: "cell_projector".into(),
                model: self.projector_model.clone(),
                snapshot_binding: slice.snapshot_binding.clone(),
                lived_stream: format!(
                    "<!-- membrane:{MEMBRANE_SCHEMA}:cell-projector -->\nYou are a private cell Projector. Convert only the permitted typed context and visible stimulus into compact lived narrative segments. Each segment belongs to exactly one supplied subject_id and contains only that subject's perceptions, memories, wants, fears, knowledge, and explicit uncertainty. Mentioned outsiders remain external observations: never give them an internal viewpoint. Campaign policy constrains what may become simulation content, but is never actor knowledge: omit line topics, keep veil topics off-screen, and introduce no ask_first topic. Do not narrativize or reveal the policy itself. Do not choose actions or claim world effects. Omit decorative recap. Return between {} and {perspective_limit} unique segments; do not narrate every ordinary constituent. Include every exact subject ID in REQUIRED PERSPECTIVE OWNERS, because each named member exception or debt focus has an actionable decision that must not disappear inside the aggregate. Put detail_focus_subject_id first when present. Spend any remaining slots only on subjects facing a materially different decision in this horizon.\n\nREQUIRED PERSPECTIVE OWNERS:\n{}\n\nReturn exactly one JSON object matching this stable shape:\n{CELL_PROJECTION_OUTPUT_CONTRACT}\n\nCAMPAIGN POLICY:\n{campaign_policy}\n\nDomain guidance:\n{mode_guidance}\n\nIdentity:\n{}\n\nPermitted typed context:\n{projector_context}\n\nVisible stimulus:\n{visible_stimulus}\n\nUse no more than {word_budget} narrative words across all segments.",
                    required_projection_subject_ids.len().max(1),
                    serde_json::to_string(&required_projection_subject_ids)?,
                    slice.cell_id
                ),
                output_schema: Some(projection_schema),
                source_receipt_ids: slice.source_receipt_ids.clone(),
                temperature: Some(0.0),
                max_output_tokens: Some(768),
            };
            let mut projector_receipts = Vec::new();
            let (projected_narrative, active_subject_ids, projector_receipt) = loop {
                let mut projected =
                    match run_validated_stage(self.model.as_ref(), &projection_request)
                        .await
                        .context("cell projector model stage failed")
                    {
                        Ok(projected) => projected,
                        Err(error) => return Err(cell_pipeline_failure(error, projector_receipts)),
                    };
                let proposal = projected
                    .structured
                    .clone()
                    .ok_or_else(|| anyhow!("cell Projector produced no typed segments"))
                    .and_then(|value| serde_json::from_value(value).map_err(Into::into));
                match proposal.and_then(|proposal| bind_cell_projection(&slice, proposal)) {
                    Ok((narrative, active_subject_ids)) => {
                        let receipt = projected.receipt.clone();
                        projector_receipts.push(projected.receipt);
                        break (narrative, active_subject_ids, receipt);
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
                        projected.receipt.validation_result = "semantic_invalid".into();
                        projected.receipt.local_validation_error =
                            Some(error.to_string().chars().take(1_000).collect());
                        projector_receipts.push(projected.receipt);
                        return Err(cell_pipeline_failure(
                            anyhow!(
                                "cell Projector failed perspective binding after one correction: {error}"
                            ),
                            projector_receipts,
                        ));
                    }
                }
            };
            let lived_stream = LivedNarrativeStream {
                text: format!(
                    "{}\n\n{}",
                    cell_scene_boundaries(&slice, &active_subject_ids),
                    projected_narrative
                ),
                snapshot_binding: slice.snapshot_binding.clone(),
                projector_receipt,
            };
            CellProjectedMoment {
                lived_stream,
                active_subject_ids,
                causal_receipts: projector_receipts.clone(),
                projector_receipts,
            }
        };
        self.permit
            .require(&slice.cell_id, &slice.snapshot_binding, "cell_persona")
            .await
            .map_err(|error| cell_pipeline_failure(error, projector_receipts.clone()))?;
        let persona_domain_guidance = if require_explicit_decision {
            format!(
                "{} Your previous response supplied no explicit strategic decision. Respond naturally again from this same lived moment, but make every voiced constituent end with either one concrete present-tense attempt or an explicit choice to hold or wait. Do not invent new perceptions, permissions, contacts, resources, or completed consequences.",
                cell_persona_mode_guidance(&slice.mode)
            )
        } else {
            cell_persona_mode_guidance(&slice.mode).to_owned()
        };
        let persona = match run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "cell_persona".into(),
                model: self.persona_model.clone(),
                snapshot_binding: slice.snapshot_binding.clone(),
                lived_stream: build_persona_prompt(&PersonaPrompt {
                    identity: &slice.cell_id,
                    lived_stream: &lived.text,
                    domain_guidance: &persona_domain_guidance,
                    word_budget: (160 + 30 * slice.constituents.len()).min(320),
                }),
                output_schema: None,
                source_receipt_ids: causal_source_ids(&slice.source_receipt_ids, &causal_receipts),
                temperature: Some(0.7),
                max_output_tokens: Some(512),
            },
        )
        .await
        .context("cell Persona model stage failed")
        {
            Ok(persona) => persona,
            Err(error) => return Err(cell_pipeline_failure(error, projector_receipts)),
        };
        let mut stage_receipts = projector_receipts.clone();
        stage_receipts.push(persona.receipt.clone());
        self.permit
            .require(&slice.cell_id, &slice.snapshot_binding, "cell_interpreter")
            .await
            .map_err(|error| cell_pipeline_failure(error, stage_receipts.clone()))?;
        let mut schema = serde_json::to_value(schema_for!(CellAppraisalProposal))
            .map_err(|error| cell_pipeline_failure(error.into(), stage_receipts.clone()))?;
        constrain_cell_proposal_schema(&mut schema, &slice, &active_subject_ids)
            .map_err(|error| cell_pipeline_failure(error, stage_receipts.clone()))?;
        let interpreter_context =
            serde_json::to_string(&cell_interpreter_context(&slice, &active_subject_ids))
                .map_err(|error| cell_pipeline_failure(error.into(), stage_receipts.clone()))?;
        let permission_guidance = format!(
            concat!(
                "Emit at most {} exact constituent- or named-member-attributed attempts. Priority is an urgency score from 0 to 100 where higher numbers resolve first. ",
                "Every subject in exact_permissions owns one schema-keyed decision slot. Fill that exact slot with one action or one inaction. Use undecided only when the Persona turn supplied no explicit choice or hold; runtime will retry the Persona rather than let the Interpreter invent one. Do not let a voiced constituent vanish during interpretation. ",
                "Each action carries an effects object whose exact subject-specific lane keys are supplied by the schema. Across scalar effects and every expanded activity scope, an action has one to four exact effects total. Scalar lanes contain one typed effect. An activities lane is one object keyed by chosen activity kinds; each non-null kind contains an array of one to four separate exact target-and-location scopes. Preserve repeated uses of one activity kind as separate scopes when their targets or locations differ; never union scopes across distinct locations or audiences. Top-level lane names are stable across subjects: use null for a null-only unavailable lane and for any schema-required optional lane or activity key the subject does not use. A non-null lane or activity key absent from its exact schema is structurally unavailable: do not emit it and do not invent a destination. Pressure and migration lanes each have one slot. Preserve every means of one chosen course in one action. With relocation, activities at the exact snapshot location occur before departure and activities at the exact admitted destination occur after arrival; activity effects inside each location phase are an atomic set. Field and array order is not chronology. Never split one subject's single choice into multiple actions. ",
                "Use gestalt_activities or member_activities for concrete attempts that do not themselves change pressure. A cohesive Gestalt coordinating its own unnamed internal members uses coordinate with an empty target_subject_ids list; do not invent its containing population, a distant population, or another canonical subject as the target of internal coordination. Cite the smallest exact set of state_references that materially supports each attempt; the permission list is an upper bound, not a checklist to echo. ",
                "target_subject_ids and location_ids must come from that exact subject's permissions. activity_targets is the exact canonical target map: each key is the authoritative ID and each value supplies the target's name and current canonical locations. Use an ID only when the Persona addresses that named target, never merely because the ID is permitted. If an addressed person or role has no matching activity_targets entry, it is not a canonical target in this slice. reachable_destinations maps exact actor-movement destination IDs to names. migration_destinations maps exact population destination IDs to names and locations. When the Persona chooses to go to a canonical target, compare the target's current locations with the acting subject's current location and exact reachable destinations; never guess a destination from an opaque ID. Every activity has at most four unique target_subject_ids; choose the four most causally relevant when more permitted subjects are involved. A member activity uses exactly the member's source_location_id. Internal work is prepare with no targets. A local investigate may have no target and use the exact current location to seek information from the environment or an unnamed ordinary role; asking an unnamed clerk or dock master for facts maps here and records only the inquiry, never a reply or discovery. A local communicate may likewise have no target at the exact current location when the Persona speaks, sends, offers, asks permission, or notifies an unnamed ordinary role; it records only the source's outgoing attempt, never a listener, reply, acceptance, or outcome. Communication with a canonical subject requires that exact target ID. When one utterance addresses canonical subjects and an unnamed public audience, emit one communicate activity with those exact target IDs and place its admitted public reach in public_channels; do not invent a second communicate lane. Never substitute a containing population, related institution, or merely permitted ID for an unnamed role. ",
                "Write intended_effect as the affirmative attempted acts, never the purpose, restraint, condition to preserve, hoped-for outcome, or target response. Keep purpose and restraint in intent. Respecting choice, declining coercion, leaving state unchanged, or waiting for another subject does not add a typed effect unless the Persona separately chooses an observable act. Merely waiting, watching, staying, holding position, or remaining ready is attributed inaction, not prepare. prepare requires concrete work on a bounded arrangement, repair, resource, or capability-backed readiness change. Institution posture must be a specific materially new commitment or withholding of at most {} characters. already_committed_posture is state already in force: maintaining, continuing, or restating it is inaction and must not emit an institution action. Gestalt pressure_resolutions copy exact current_pressures; additions are new unresolved constraints, never completed actions. Use only permitted state references. public_channels means durable publication of this attempt through exact allowed_persistent_publication_channels; it is not a perception method or ordinary local speech. Use [] when that exact list is empty. ",
                "A population that chooses to board, depart, or relocate together to one supplied migration_destinations key emits gestalt_migration; do not reduce it to prepare. It relocates only that exact population leaf and never implies a named member traveled. A named member who chooses to board, depart, travel, or join a supplied destination emits member_migration; use prepare only while departure remains unchosen. ",
                "A population or arena cannot migrate a person. Runtime binds identity and effect owner IDs from subject_id. Do not emit institution_id, gestalt_id, actor_id, or member_id inside effect. An inaction means that exact subject takes no strategic action in this horizon. Record only a subject that explicitly holds, waits without making another attempt, or merely continues already_committed_posture in the Persona turn as an inaction, using its exact subject_id and a concrete reason of at most 160 characters. Waiting for the result of an action, withholding a different possible action, or declining one option after choosing another does not make the chosen action an inaction. Never invent an inaction for an unvoiced subject or use absence of a Persona decision as a reason. Inactions share the same count limit stated for actions. A subject cannot appear in both actions and inactions. When nobody acts, actions is empty and inactions must still contain at least one exact attributed decision from the Persona turn."
            ),
            slice.max_actions,
            crate::domain::MAX_POSTURE_CHARS
        );
        let permission_guidance =
            format!("{permission_guidance} {CELL_ACTIVITY_CLASSIFICATION_GUIDANCE}");
        let permission_guidance = format!(
            "{permission_guidance} A targetless local obstruct at the exact supplied location records attempted interference with unnamed infrastructure, terrain, traffic, or another local feature. It records only the source's attempt—never damage, disruption, or a target response. Never substitute a merely permitted canonical subject for that unnamed local feature."
        );
        let permission_guidance = format!(
            "{permission_guidance} causal_follow_through assigns a committed anchor to one exact decision owner without prescribing their choice. If that owner acts in response, include the supplied anchor_reference in state_references so the causal basis survives admission. An attributed inaction remains legal and needs no invented effect."
        );
        let permission_guidance = format!(
            "{permission_guidance} Campaign policy is a hard output boundary, not actor knowledge. Emit no action, inaction rationale, pressure, migration, posture, or activity that introduces a line topic, depicts a veil topic on-screen, or introduces an ask_first topic. Never reveal boundary attribution. CAMPAIGN POLICY: {campaign_policy}"
        );
        let instructions = build_interpreter_prompt(&InterpreterPrompt {
            identity: &slice.cell_id,
            typed_context: &interpreter_context,
            lived_stream: &lived.text,
            persona_output: &persona.narrative,
            output_schema: None,
            domain_guidance: &format!(
                "{permission_guidance} Operate the private Interpreter workbench. Its current typed action schema is authoritative and exposes only the transition legal now: one complete submit initially, then exact named upsert_decision repairs after rejection. A rejected submit preserves its draft. Only the deterministic workbench can accept the appraisal."
            ),
        });
        let mut interpreter_causal_receipts = causal_receipts;
        interpreter_causal_receipts.push(persona.receipt.clone());
        let source_receipt_ids =
            causal_source_ids(&slice.source_receipt_ids, &interpreter_causal_receipts);
        let spec = ModelAgentSpec {
            stage: "cell_interpreter".into(),
            model: self.interpreter_model.clone(),
            snapshot_binding: slice.snapshot_binding.clone(),
            instructions,
            source_receipt_ids,
            temperature: Some(0.0),
            max_output_tokens: Some(1_600),
            max_steps: active_subject_ids.len().saturating_add(4).clamp(4, 8),
        };
        let mut workbench = CellInterpreterWorkbench {
            model: self.model.clone(),
            permit: self.permit.clone(),
            interpreter_model: self.interpreter_model.clone(),
            slice: slice.clone(),
            active_subject_ids: active_subject_ids.clone(),
            lived_stream: lived.text.clone(),
            persona_turn: persona.narrative.clone(),
            campaign_policy: campaign_policy.clone(),
            appraisal_schema: schema,
            draft: BTreeMap::new(),
            repair_subject_ids: BTreeSet::new(),
            accepted_verifier_bindings: BTreeSet::new(),
        };
        let run = match run_model_agent(self.model.as_ref(), &spec, &mut workbench).await {
            Ok(run) => run,
            Err(ModelAgentFailure { message, receipts }) => {
                stage_receipts.extend(receipts);
                return Err(cell_pipeline_failure(
                    anyhow!("cell Interpreter agent failed: {message}"),
                    stage_receipts,
                ));
            }
        };
        stage_receipts.extend(run.receipts);
        let appraisal = match run.output {
            CellInterpreterAgentOutput::Appraisal(appraisal) => appraisal,
            CellInterpreterAgentOutput::MissingPersonaDecision { subject_ids } => {
                return Err(anyhow::Error::new(MissingExplicitCellDecision {
                    cell_id: format!("{} ({})", slice.cell_id, subject_ids.join(", ")),
                    stage_receipts: distinct_stage_receipts(stage_receipts),
                    lived_stream: lived,
                    active_subject_ids,
                    projector_receipts,
                }));
            }
        };
        self.permit
            .require(&slice.cell_id, &slice.snapshot_binding, "cell_terminal")
            .await
            .map_err(|error| cell_pipeline_failure(error, stage_receipts.clone()))?;
        return Ok(CellTerminalBundle {
            lived_stream: lived,
            persona_output: persona.narrative,
            appraisal,
            stage_receipts,
        });
    }
}

async fn run_cell_effect_verifier_wave(
    model: Arc<dyn ModelPort>,
    interpreter_model: &str,
    slice: &PermittedCellSlice,
    lived_stream: &str,
    persona_turn: &str,
    campaign_policy: &str,
    actions: &[crate::domain::CellActionProposal],
    source_receipt_ids: &[String],
) -> Result<Vec<CellActionVerificationRun>> {
    let campaign_policy = serde_json::from_str::<serde_json::Value>(campaign_policy)?;
    let verifier_schema = cell_effect_verifier_schema(1)?;
    let mut jobs = tokio::task::JoinSet::new();
    for (action_index, action) in actions.iter().enumerate() {
        let exact_subject_permission = cell_action_verifier_permission(slice, &action.subject_id)?;
        let coordination_target_contract = coordination_target_contract(&exact_subject_permission);
        let verifier_context = serde_json::json!({
            "effect_order_contract":{
                "activity_effects_within_one_location":"unordered_atomic_set",
                "cross_location_order":"snapshot_location_activity_then_relocation_then_destination_activity",
                "field_order":"not_chronology"
            },
            "local_attempt_contract":"A targetless local communicate at the source's exact current location faithfully records speech, an offer, a permission request, or a notice directed to an unnamed ordinary role. A targetless local obstruct there faithfully records attempted interference with unnamed infrastructure, terrain, traffic, or another local feature. Both record only the source's attempt—never a listener, reply, damage, disruption, acceptance, or outcome—and must not be rejected merely because target_subject_ids is empty.",
            "spatial_effect_contract":"A prepare, investigate, or other activity may include incidental walking, approaching, queuing, carrying, or repositioning around an unnamed local feature while the source remains inside the effect's supplied canonical location. The activity records the attempt and need not serialize every footstep. Reject omitted movement only when the Persona clearly commits the subject to a different supplied canonical location or population destination; local texture does not create topology or establish arrival.",
            "coordination_target_contract":coordination_target_contract,
            "exact_subject_permission":exact_subject_permission,
            "canonical_locations":slice.canonical_locations,
            "lived_stream":lived_stream,
            "persona_turn":persona_turn,
            "candidate_action":{
                "action_index":0,
                "subject_id":action.subject_id,
                "intent":action.intent,
                "intended_effect":action.intended_effect,
                "state_references":action.state_references,
                "public_channels":action.public_channels,
                "typed_effects":action.effects,
            },
            "campaign_policy":campaign_policy,
        });
        let request = ModelStageRequest {
            stage: "cell_effect_verifier".into(),
            model: interpreter_model.to_owned(),
            snapshot_binding: cell_effect_verification_binding(
                &slice.snapshot_binding,
                std::slice::from_ref(action),
            )?,
            lived_stream: format!(
                "{CELL_EFFECT_VERIFIER_INSTRUCTIONS}\n\nCOORDINATION TARGET CONTRACT: coordination_target_contract.internal_population_target_subject_ids is the exact owner-specific target array for internal-population coordination. Preserve it; never apply a Gestalt owner's empty array to a named member.\n\nEFFECT ORDER CONTRACT: Exact activity locations derive the course. Snapshot-location activity precedes relocation; exact-destination activity follows arrival. Distinct activity effects inside one location phase are an unordered atomic set. Serialized field and array order is not chronology and cannot support an effect_reversal verdict.\n\nCONTEXT:\n{}",
                serde_json::to_string(&verifier_context)?
            ),
            output_schema: Some(verifier_schema.clone()),
            source_receipt_ids: source_receipt_ids.to_vec(),
            temperature: Some(0.0),
            max_output_tokens: Some(1_200),
        };
        let model = model.clone();
        jobs.spawn(async move {
            let output = match run_validated_stage(model.as_ref(), &request).await {
                Ok(output) => output,
                Err(error) => {
                    return Err(CellEffectVerifierTaskFailure {
                        diagnostic: format!(
                            "cell effect verifier action {action_index} failed: {error}"
                        ),
                        completed_stage_receipt: None,
                    });
                }
            };
            let verification = output
                .structured
                .clone()
                .ok_or_else(|| anyhow!("cell effect verifier produced no typed verdict"))
                .and_then(|value| serde_json::from_value::<CellEffectVerification>(value).map_err(Into::into))
                .and_then(|verification| {
                    validate_effect_verification(&verification, 1)?;
                    Ok(verification)
                })
                .map_err(|error: anyhow::Error| {
                    let diagnostic = format!(
                        "cell effect verifier action {action_index} failed local validation: {error}"
                    );
                    let mut receipt = output.receipt.clone();
                    receipt.validation_result = "semantic_invalid".into();
                    receipt.local_validation_error =
                        Some(diagnostic.chars().take(1_000).collect());
                    CellEffectVerifierTaskFailure {
                        diagnostic,
                        completed_stage_receipt: Some(receipt),
                    }
                })?;
            let verdict = verification
                .verdicts
                .into_iter()
                .next()
                .expect("one-action verifier schema and validator require one verdict");
            Ok::<_, CellEffectVerifierTaskFailure>(CellActionVerificationRun {
                action_index,
                output,
                verdict,
            })
        });
    }

    let mut verified = Vec::with_capacity(actions.len());
    let mut diagnostics = Vec::new();
    let mut failed_stage_receipts = Vec::new();
    while let Some(result) = jobs.join_next().await {
        match result {
            Ok(Ok(verification)) => verified.push(verification),
            Ok(Err(failure)) => {
                diagnostics.push(failure.diagnostic);
                failed_stage_receipts.extend(failure.completed_stage_receipt);
            }
            Err(error) => diagnostics.push(format!("cell effect verifier task failed: {error}")),
        }
    }
    if !diagnostics.is_empty() {
        return Err(anyhow::Error::new(CellEffectVerifierWaveFailure {
            diagnostics,
            completed_stage_receipts: verified
                .into_iter()
                .map(|verification| verification.output.receipt)
                .chain(failed_stage_receipts)
                .collect(),
        }));
    }
    verified.sort_by_key(|verification| verification.action_index);
    Ok(verified)
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
            "Appraise the strategic horizon as a real collective. End this turn with a present-tense choice: describe the concrete attempt the collective now makes, or explicitly choose to hold or wait. An institution continuing its already committed posture is holding steady, not choosing that posture again. Deliberating, asking for a future decision, considering an option, or saying what could be done is intentional inaction unless the choice to act is actually made. Do not invent completed consequences."
        }
        crate::domain::SimulationCellMode::Arena => {
            "Appraise the strategic horizon polyphonically. Name the constituent responsible for every perspective and decision; never speak as the arena or use an unmarked first-person voice. Only subjects already given an attributed internal perspective in the lived stream may choose; people merely observed or mentioned remain external. The lived stream may contain simultaneous remote scenes: preserve every stated location boundary, and never make one constituent see, hear, address, or answer another unless the stream explicitly establishes co-presence or a communication channel. Do not invent an available person, office, route, resource, or response absent from the lived stream. A constituent may choose to seek something unknown, but cannot claim contact with it. For each voiced constituent, end with a present-tense choice: a concrete attempt now, or an explicit choice to hold or wait. An institution continuing its already committed posture is holding steady, not choosing that posture again. Deliberating, asking for a future decision, considering an option, or saying what could be done is inaction unless that constituent actually chooses to act."
        }
    }
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
        match &verdict.result {
            CellEffectMatchResult::Match if verdict.findings.is_empty() => {}
            CellEffectMatchResult::Mismatch
                if !verdict.findings.is_empty()
                    && verdict.findings.len() <= 6
                    && verdict.findings.iter().all(|finding| {
                        !finding.repair_guidance.trim().is_empty()
                            && finding.repair_guidance.trim() == finding.repair_guidance
                            && finding.repair_guidance.chars().count() <= 240
                    })
                    && verdict
                        .findings
                        .iter()
                        .map(|finding| &finding.mismatch_kind)
                        .collect::<BTreeSet<_>>()
                        .len()
                        == verdict.findings.len() =>
            {
                rejected.push(expected_index);
            }
            _ => {
                return Err(anyhow!(
                    "cell effect verifier returned an incoherent result or findings set for action {expected_index}"
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
    let verdict = schema
        .pointer_mut("/$defs/CellActionEffectVerdict")
        .ok_or_else(|| anyhow!("cell effect verifier schema has no verdict definition"))?;
    let finding_schema = serde_json::json!({
        "type":"object",
        "additionalProperties":false,
        "required":["mismatch_kind", "repair_guidance"],
        "properties":{
            "mismatch_kind":{
                "enum":[
                    "subject_swap",
                    "effect_omission",
                    "effect_reversal",
                    "target_substitution",
                    "invented_outcome",
                    "wrong_effect_kind"
                ]
            },
            "repair_guidance":{
                "type":"string",
                "minLength":1,
                "maxLength":240
            }
        }
    });
    *verdict = serde_json::json!({
        "oneOf":[
            {
                "type":"object",
                "additionalProperties":false,
                "required":["action_index", "result", "findings"],
                "properties":{
                    "action_index":{
                        "type":"integer",
                        "minimum":0,
                        "maximum":action_count - 1
                    },
                    "result":{"const":"match"},
                    "findings":{
                        "type":"array",
                        "maxItems":0,
                        "items":finding_schema.clone()
                    }
                }
            },
            {
                "type":"object",
                "additionalProperties":false,
                "required":["action_index", "result", "findings"],
                "properties":{
                    "action_index":{
                        "type":"integer",
                        "minimum":0,
                        "maximum":action_count - 1
                    },
                    "result":{"const":"mismatch"},
                    "findings":{
                        "type":"array",
                        "minItems":1,
                        "maxItems":6,
                        "items":finding_schema
                    }
                }
            }
        ]
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
    active_subject_ids: &BTreeSet<String>,
    proposal: CellAppraisalProposal,
) -> Result<crate::domain::CellAppraisal> {
    let actions = proposal
        .actions
        .into_iter()
        .map(|candidate| {
            let CellActionCandidate {
                subject_id,
                intent,
                intended_effect,
                priority,
                state_references,
                public_channels,
                effects: candidate_effects,
            } = candidate;
            let member_id = if candidate_effects.member_activities.is_some()
                || candidate_effects.member_migration.is_some()
            {
                Some(
                    slice
                        .member_exceptions
                        .iter()
                        .find(|member| member.subject_id == subject_id)
                        .map(|member| member.member_id.clone())
                        .ok_or_else(|| {
                            anyhow!(
                                "member effect subject {} is not a selected member exception",
                                subject_id
                            )
                        })?,
                )
            } else {
                None
            };
            let mut effects = Vec::with_capacity(4);
            if let Some(effect) = candidate_effects.actor_move {
                effects.push(crate::domain::StrategicCellEffect::ActorMove {
                    actor_id: subject_id.clone(),
                    destination_id: effect.destination_id,
                });
            }
            if let Some(effect) = candidate_effects.gestalt_migration {
                effects.push(crate::domain::StrategicCellEffect::GestaltMigration {
                    destination_gestalt_id: effect.destination_gestalt_id,
                });
            }
            if let Some(effect) = candidate_effects.member_migration {
                effects.push(crate::domain::StrategicCellEffect::MemberMigration {
                    destination_gestalt_id: effect.destination_gestalt_id,
                });
            }
            if let Some(effect) = candidate_effects.institution {
                effects.push(crate::domain::StrategicCellEffect::Institution {
                    institution_id: subject_id.clone(),
                    posture: effect.posture,
                    location_ids: effect.location_ids,
                });
            }
            if let Some(effect) = candidate_effects.gestalt_pressure {
                effects.push(crate::domain::StrategicCellEffect::Gestalt {
                    gestalt_id: subject_id.clone(),
                    pressure_additions: effect.pressure_additions,
                    pressure_resolutions: effect.pressure_resolutions,
                });
            }
            if let Some(activities) = candidate_effects.gestalt_activities {
                for (activity, effect) in activities.into_effects() {
                    effects.push(crate::domain::StrategicCellEffect::GestaltActivity {
                        gestalt_id: subject_id.clone(),
                        activity,
                        target_subject_ids: effect.target_subject_ids,
                        location_ids: effect.location_ids,
                    });
                }
            }
            if let Some(activities) = candidate_effects.actor_activities {
                for (activity, effect) in activities.into_effects() {
                    effects.push(crate::domain::StrategicCellEffect::ActorActivity {
                        actor_id: subject_id.clone(),
                        activity,
                        target_subject_ids: effect.target_subject_ids,
                        location_ids: effect.location_ids,
                    });
                }
            }
            if let Some(activities) = candidate_effects.member_activities {
                for (activity, effect) in activities.into_effects() {
                    effects.push(crate::domain::StrategicCellEffect::MemberActivity {
                        member_id: member_id
                            .clone()
                            .expect("member effect resolved an exact member"),
                        activity,
                        target_subject_ids: effect.target_subject_ids,
                        location_ids: effect.location_ids,
                    });
                }
            }
            if effects.is_empty() || effects.len() > 4 {
                return Err(anyhow!(
                    "action for subject {} requires one to four exact effects",
                    subject_id
                ));
            }
            Ok(crate::domain::CellActionProposal {
                subject_id,
                intent,
                intended_effect,
                priority,
                state_references,
                public_channels,
                effects,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(crate::domain::CellAppraisal {
        schema: "ghostlight.cell_appraisal.v1".into(),
        cell_id: slice.cell_id.clone(),
        world_revision: slice.world_revision,
        resolution_epoch: slice.resolution_epoch,
        considered_subject_ids: active_subject_ids.clone(),
        actions,
        inactions: proposal.inactions,
    })
}

fn decode_cell_appraisal_proposal(
    cell_id: &str,
    value: serde_json::Value,
) -> Result<CellAppraisalProposal> {
    let decisions = value
        .get("decisions")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| anyhow!("cell interpreter proposal has no exact decision map"))?;
    let mut actions = Vec::new();
    let mut inactions = Vec::new();
    for (subject_id, decision) in decisions {
        let decision = decision
            .as_object()
            .ok_or_else(|| anyhow!("decision for subject {subject_id} is not an object"))?;
        match (
            decision.get("action"),
            decision.get("inaction"),
            decision.get("undecided"),
        ) {
            (Some(action), None, None) => {
                let action: CellActionCandidate = serde_json::from_value(action.clone())?;
                if action.subject_id != *subject_id {
                    return Err(anyhow!(
                        "decision key {subject_id} does not match action owner {}",
                        action.subject_id
                    ));
                }
                actions.push(action);
            }
            (None, Some(inaction), None) => {
                let inaction: crate::domain::CellInaction =
                    serde_json::from_value(inaction.clone())?;
                if inaction.subject_id != *subject_id {
                    return Err(anyhow!(
                        "decision key {subject_id} does not match inaction owner {}",
                        inaction.subject_id
                    ));
                }
                inactions.push(inaction);
            }
            (None, None, Some(_)) => {
                return Err(anyhow!(
                    "cell {cell_id} decision for subject {subject_id} remained undecided"
                ));
            }
            _ => {
                return Err(anyhow!(
                    "decision for subject {subject_id} must contain exactly one decision kind"
                ));
            }
        }
    }
    Ok(CellAppraisalProposal { actions, inactions })
}

fn validate_cell_appraisal(
    slice: &PermittedCellSlice,
    appraisal: &crate::domain::CellAppraisal,
) -> Result<()> {
    if appraisal.schema != "ghostlight.cell_appraisal.v1"
        || appraisal.cell_id != slice.cell_id
        || appraisal.world_revision != slice.world_revision
        || appraisal.resolution_epoch != slice.resolution_epoch
    {
        return Err(anyhow!(
            "appraisal has a stale or incomplete runtime binding"
        ));
    }
    if appraisal.actions.is_empty() && appraisal.inactions.is_empty() {
        return Err(anyhow!(
            "an appraisal with no actions requires one exact attributed inaction"
        ));
    }
    let permitted_subject_ids = slice
        .constituents
        .iter()
        .map(|subject| subject.subject_id.as_str())
        .chain(
            slice
                .member_exceptions
                .iter()
                .map(|member| member.subject_id.as_str()),
        )
        .collect::<BTreeSet<_>>();
    if appraisal.considered_subject_ids.is_empty()
        || appraisal.considered_subject_ids.len() > slice.max_actions
        || appraisal
            .considered_subject_ids
            .iter()
            .any(|subject_id| !permitted_subject_ids.contains(subject_id.as_str()))
    {
        return Err(anyhow!(
            "appraisal considered subjects are empty, over quota, or outside this cell's exact decision owners"
        ));
    }
    let mut action_subject_ids = BTreeSet::new();
    for action in &appraisal.actions {
        if !appraisal
            .considered_subject_ids
            .contains(&action.subject_id)
        {
            return Err(anyhow!(
                "action subject {} was not an exact projected decision owner",
                action.subject_id
            ));
        }
        if !action_subject_ids.insert(action.subject_id.as_str()) {
            return Err(anyhow!(
                "subject {} has duplicate strategic actions; combine every chosen movement and activity means into one composed action",
                action.subject_id
            ));
        }
    }
    let mut inaction_subject_ids = BTreeSet::new();
    for inaction in &appraisal.inactions {
        if inaction.reason.trim().is_empty() {
            return Err(anyhow!(
                "inaction for subject {} requires a non-empty reason",
                inaction.subject_id
            ));
        }
        if inaction.reason.chars().count() > 240 {
            return Err(anyhow!(
                "inaction reason for subject {} is {} characters but permits at most 240",
                inaction.subject_id,
                inaction.reason.chars().count()
            ));
        }
        if !permitted_subject_ids.contains(inaction.subject_id.as_str()) {
            return Err(anyhow!(
                "inaction subject {} is outside this cell's exact constituents and member exceptions",
                inaction.subject_id
            ));
        }
        if !appraisal
            .considered_subject_ids
            .contains(&inaction.subject_id)
        {
            return Err(anyhow!(
                "inaction subject {} was not an exact projected decision owner",
                inaction.subject_id
            ));
        }
        if action_subject_ids.contains(inaction.subject_id.as_str()) {
            return Err(anyhow!(
                "subject {} appears in both actions and inactions; retain exactly one of those decisions",
                inaction.subject_id
            ));
        }
        if !inaction_subject_ids.insert(inaction.subject_id.as_str()) {
            return Err(anyhow!(
                "subject {} has duplicate attributed inactions; retain exactly one",
                inaction.subject_id
            ));
        }
    }
    for action in &appraisal.actions {
        validate_cell_action(slice, action)?;
    }
    let decided_subject_ids = action_subject_ids
        .into_iter()
        .chain(inaction_subject_ids)
        .collect::<BTreeSet<_>>();
    let missing = appraisal
        .considered_subject_ids
        .iter()
        .map(String::as_str)
        .filter(|subject_id| !decided_subject_ids.contains(subject_id))
        .collect::<BTreeSet<_>>();
    if !missing.is_empty() {
        return Err(anyhow!(
            "appraisal omitted explicit strategic decisions for projected subjects {missing:?}"
        ));
    }
    Ok(())
}

fn validate_cell_action(
    slice: &PermittedCellSlice,
    action: &crate::domain::CellActionProposal,
) -> Result<()> {
    if action.intent.trim().is_empty() || action.intended_effect.trim().is_empty() {
        return Err(anyhow!(
            "action for subject {} requires non-empty intent and intended_effect",
            action.subject_id
        ));
    }
    let required_causal_references = slice
        .causal_follow_through
        .iter()
        .filter(|focus| focus.responder_subject_id == action.subject_id)
        .map(|focus| focus.anchor_reference.as_str())
        .collect::<BTreeSet<_>>();
    if !required_causal_references.is_empty()
        && action
            .state_references
            .iter()
            .all(|reference| !required_causal_references.contains(reference.as_str()))
    {
        return Err(anyhow!(
            "action for causal responder {} omits its exact scheduler anchor {:?}",
            action.subject_id,
            required_causal_references
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
        validate_effect_bundle(&action.effects)?;
        let prospective_location = action.effects.iter().find_map(|effect| match effect {
            crate::domain::StrategicCellEffect::ActorMove { destination_id, .. } => {
                Some(destination_id.as_str())
            }
            crate::domain::StrategicCellEffect::GestaltMigration {
                destination_gestalt_id,
            } => subject
                .migration_destinations
                .get(destination_gestalt_id)
                .map(|destination| destination.location_id.as_str()),
            _ => None,
        });
        for effect in &action.effects {
            validate_constituent_effect(subject, effect, prospective_location)?;
        }
        return Ok(());
    }
    let Some(member) = slice
        .member_exceptions
        .iter()
        .find(|value| value.subject_id == action.subject_id)
    else {
        return Err(anyhow!("action is attributed outside the cell"));
    };
    validate_action_permissions(
        action,
        &member.permitted_state_references,
        &member.information_channels,
    )?;
    validate_effect_bundle(&action.effects)?;
    let prospective_location = action.effects.iter().find_map(|effect| match effect {
        crate::domain::StrategicCellEffect::MemberMigration {
            destination_gestalt_id,
        } => member
            .migration_destinations
            .get(destination_gestalt_id)
            .map(|destination| destination.location_id.as_str()),
        _ => None,
    });
    for effect in &action.effects {
        match effect {
            crate::domain::StrategicCellEffect::MemberActivity {
                member_id,
                activity,
                target_subject_ids,
                location_ids,
            } => {
                let unique_targets = target_subject_ids.iter().collect::<BTreeSet<_>>();
                let needs_target = !activity.allows_targetless_local_attempt();
                let activity_location = location_ids.first().map(String::as_str);
                if member_id != &member.member_id
                    || target_subject_ids.len() > 4
                    || unique_targets.len() != target_subject_ids.len()
                    || target_subject_ids
                        .iter()
                        .any(|target| !member.activity_targets.contains_key(target))
                    || (needs_target && target_subject_ids.is_empty())
                    || location_ids.len() != 1
                    || (activity_location != Some(member.source_location_id.as_str())
                        && activity_location != prospective_location)
                {
                    return Err(anyhow!(
                        "named member {} proposed {:?} toward {:?} at {:?}; exact allowed targets are {:?}, current location is {}, and a paired migration may establish {:?}",
                        member.member_id,
                        activity,
                        target_subject_ids,
                        location_ids,
                        member.activity_targets.keys().collect::<Vec<_>>(),
                        member.source_location_id,
                        prospective_location
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
                    "action for named member {} exceeds exact personal authority; effects={:?}; allowed destination gestalt IDs={:?}",
                    member.member_id,
                    action.effects,
                    member.migration_destinations.keys().collect::<Vec<_>>()
                ));
            }
        }
    }
    Ok(())
}

fn validate_effect_bundle(effects: &[crate::domain::StrategicCellEffect]) -> Result<()> {
    if effects.is_empty() || effects.len() > 4 {
        return Err(anyhow!(
            "a strategic action requires one to four exact effects"
        ));
    }
    let mut lanes = BTreeSet::new();
    for (index, effect) in effects.iter().enumerate() {
        if effects[..index].contains(effect) {
            return Err(anyhow!(
                "a strategic action cannot repeat an exact typed effect"
            ));
        }
        let lane = match effect {
            crate::domain::StrategicCellEffect::GestaltActivity { .. }
            | crate::domain::StrategicCellEffect::ActorActivity { .. }
            | crate::domain::StrategicCellEffect::MemberActivity { .. } => None,
            _ => Some(effect.lane()),
        };
        if lane.is_some_and(|lane| !lanes.insert(lane)) {
            return Err(anyhow!(
                "one strategic action may use each scalar effect lane at most once"
            ));
        }
    }
    Ok(())
}

fn validate_active_decision_owners(
    active_subject_ids: &BTreeSet<String>,
    appraisal: &crate::domain::CellAppraisal,
) -> Result<()> {
    if appraisal.considered_subject_ids != *active_subject_ids {
        return Err(anyhow!(
            "appraisal decision owners do not match the exact projected perspectives"
        ));
    }
    let inactive = appraisal
        .actions
        .iter()
        .map(|action| action.subject_id.as_str())
        .chain(
            appraisal
                .inactions
                .iter()
                .map(|inaction| inaction.subject_id.as_str()),
        )
        .filter(|subject_id| !active_subject_ids.contains(*subject_id))
        .collect::<BTreeSet<_>>();
    if !inactive.is_empty() {
        return Err(anyhow!(
            "appraisal assigned decisions to inactive unprojected subjects {inactive:?}"
        ));
    }
    Ok(())
}

fn validate_constituent_effect(
    subject: &CellConstituentSlice,
    effect: &crate::domain::StrategicCellEffect,
    prospective_location: Option<&str>,
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
            if posture.trim().is_empty() {
                return Err(anyhow!(
                    "institution {} proposed an empty posture",
                    subject.subject_id
                ));
            }
            if posture.chars().count() > crate::domain::MAX_POSTURE_CHARS {
                return Err(anyhow!(
                    "institution {} proposed a posture of {} characters; the exact maximum is {}",
                    subject.subject_id,
                    posture.chars().count(),
                    crate::domain::MAX_POSTURE_CHARS
                ));
            }
            if current.trim().eq_ignore_ascii_case(posture.trim()) {
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
            let needs_target = activity.requires_explicit_target_for_gestalt();
            if needs_target && target_subject_ids.is_empty() {
                return Err(anyhow!(
                    "gestalt {} activity {:?} requires one or more exact target IDs; no anonymous or unsupplied target can be encoded. Remove the action unless the Persona explicitly attempted one of {:?}",
                    subject.subject_id,
                    activity,
                    subject.activity_targets.keys().collect::<Vec<_>>()
                ));
            }
            if subject.subject_kind != crate::domain::AgencySubjectKind::Gestalt
                || gestalt_id != &subject.subject_id
                || target_subject_ids.len() > 4
                || unique_targets.len() != target_subject_ids.len()
                || target_subject_ids
                    .iter()
                    .any(|target| !subject.activity_targets.contains_key(target))
                || location_ids.len() > 4
                || unique_locations.len() != location_ids.len()
                || location_ids.iter().any(|location| {
                    !subject.location_ids.contains(location)
                        && Some(location.as_str()) != prospective_location
                })
            {
                return Err(anyhow!(
                    "gestalt {} proposed {:?} toward {:?} at {:?}; exact allowed targets (choose at most four unique IDs) are {:?} and exact locations (choose at most four unique IDs) are {:?}",
                    subject.subject_id,
                    activity,
                    target_subject_ids,
                    location_ids,
                    subject.activity_targets.keys().collect::<Vec<_>>(),
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
                || !subject.reachable_destinations.contains_key(destination_id)
            {
                return Err(anyhow!(
                    "subject {} has kind {:?}; actor movement requested for {} to {:?}, while exact reachable destinations are {:?}",
                    subject.subject_id,
                    subject.subject_kind,
                    actor_id,
                    destination_id,
                    subject.reachable_destinations
                ));
            }
        }
        crate::domain::StrategicCellEffect::ActorActivity {
            actor_id,
            activity,
            target_subject_ids,
            location_ids,
        } => {
            let unique_targets = target_subject_ids.iter().collect::<BTreeSet<_>>();
            let needs_target = !activity.allows_targetless_local_attempt();
            if subject.subject_kind != crate::domain::AgencySubjectKind::Actor
                || actor_id != &subject.subject_id
                || target_subject_ids.len() > 4
                || unique_targets.len() != target_subject_ids.len()
                || target_subject_ids
                    .iter()
                    .any(|target| !subject.activity_targets.contains_key(target))
                || (needs_target && target_subject_ids.is_empty())
                || location_ids.len() != 1
                || (!subject.location_ids.contains(&location_ids[0])
                    && Some(location_ids[0].as_str()) != prospective_location)
            {
                return Err(anyhow!(
                    "actor {} proposed {:?} toward {:?} at {:?}; exact allowed targets are {:?} and exact locations are {:?}",
                    subject.subject_id,
                    activity,
                    target_subject_ids,
                    location_ids,
                    subject.activity_targets.keys().collect::<Vec<_>>(),
                    subject.location_ids
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
    active_subject_ids: &BTreeSet<String>,
) -> Result<()> {
    let action_candidate = schema
        .pointer("/$defs/CellActionCandidate")
        .cloned()
        .ok_or_else(|| anyhow!("cell appraisal schema has no action candidate"))?;
    let exact_subject_actions = slice
        .constituents
        .iter()
        .filter(|subject| active_subject_ids.contains(&subject.subject_id))
        .map(|subject| {
            Ok((
                subject.subject_id.clone(),
                exact_cell_action_schema(
                    action_candidate.clone(),
                    &subject.subject_id,
                    exact_constituent_effect_bundle_schema(subject),
                    &subject.permitted_state_references,
                    &subject.information_channels,
                )?,
            ))
        })
        .chain(
            slice
                .member_exceptions
                .iter()
                .filter(|member| active_subject_ids.contains(&member.subject_id))
                .map(|member| {
                    Ok((
                        member.subject_id.clone(),
                        exact_cell_action_schema(
                            action_candidate.clone(),
                            &member.subject_id,
                            exact_member_effect_bundle_schema(member),
                            &member.permitted_state_references,
                            &member.information_channels,
                        )?,
                    ))
                }),
        )
        .collect::<Result<BTreeMap<_, _>>>()?;
    if exact_subject_actions.is_empty() {
        return Err(anyhow!(
            "cell proposal schema requires at least one exact decision owner"
        ));
    }
    if exact_subject_actions.len() > slice.max_actions {
        return Err(anyhow!("cell proposal schema exceeds its decision budget"));
    }
    let mut decision_properties = serde_json::Map::new();
    let mut required_subjects = Vec::new();
    for (subject_id, action_schema) in exact_subject_actions {
        required_subjects.push(subject_id.clone());
        decision_properties.insert(
            subject_id.clone(),
            serde_json::json!({
                "anyOf":[
                    {
                        "type":"object",
                        "additionalProperties":false,
                        "required":["action"],
                        "properties":{"action":action_schema}
                    },
                    {
                        "type":"object",
                        "additionalProperties":false,
                        "required":["inaction"],
                        "properties":{"inaction":{
                            "type":"object",
                            "additionalProperties":false,
                            "required":["subject_id","reason"],
                            "properties":{
                                "subject_id":{"type":"string","const":subject_id},
                                "reason":{"type":"string","minLength":1,"maxLength":240}
                            }
                        }}
                    },
                    {
                        "type":"object",
                        "additionalProperties":false,
                        "required":["undecided"],
                        "properties":{"undecided":{
                            "type":"object",
                            "additionalProperties":false,
                            "required":["reason"],
                            "properties":{"reason":{"type":"string","minLength":1,"maxLength":240}}
                        }}
                    }
                ]
            }),
        );
    }
    *schema = serde_json::json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "type":"object",
        "additionalProperties":false,
        "required":["decisions"],
        "properties":{
            "decisions":{
                "type":"object",
                "additionalProperties":false,
                "required":required_subjects,
                "properties":decision_properties
            }
        }
    });
    Ok(())
}

enum CellInterpreterSchemaState<'a> {
    Initial,
    Repair(&'a BTreeSet<String>),
}

fn cell_interpreter_agent_schema(
    appraisal_schema: &serde_json::Value,
    state: CellInterpreterSchemaState<'_>,
) -> Result<serde_json::Value> {
    let decisions_schema = appraisal_schema
        .pointer("/properties/decisions")
        .cloned()
        .ok_or_else(|| anyhow!("cell appraisal schema has no exact decisions map"))?;
    let decision_properties = decisions_schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| anyhow!("cell appraisal schema has no exact decision owners"))?;
    let commands = match state {
        CellInterpreterSchemaState::Initial => vec![serde_json::json!({
            "type":"object",
            "additionalProperties":false,
            "required":["kind","decisions"],
            "properties":{
                "kind":{"const":"submit"},
                "decisions":decisions_schema
            }
        })],
        CellInterpreterSchemaState::Repair(repair_subject_ids) => {
            if repair_subject_ids.is_empty() {
                return Err(anyhow!("Interpreter repair schema requires a subject"));
            }
            repair_subject_ids
                .iter()
                .map(|subject_id| {
                    let decision_schema = decision_properties.get(subject_id).ok_or_else(|| {
                        anyhow!("Interpreter repair subject {subject_id} has no decision schema")
                    })?;
                    Ok(serde_json::json!({
                        "type":"object",
                        "additionalProperties":false,
                        "required":["kind","subject_id","decision"],
                        "properties":{
                            "kind":{"const":"upsert_decision"},
                            "subject_id":{"const":subject_id},
                            "decision":decision_schema
                        }
                    }))
                })
                .collect::<Result<Vec<_>>>()?
        }
    };
    Ok(serde_json::json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "type":"object",
        "additionalProperties":false,
        "required":["command"],
        "properties":{
            "command":{"oneOf":commands}
        }
    }))
}

fn exact_cell_action_schema(
    mut action_schema: serde_json::Value,
    subject_id: &str,
    effect_schema: serde_json::Value,
    permitted_state_references: &BTreeSet<String>,
    information_channels: &BTreeSet<String>,
) -> Result<serde_json::Value> {
    let properties = action_schema
        .pointer_mut("/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("cell action candidate schema has no properties"))?;
    properties.insert(
        "subject_id".into(),
        serde_json::json!({"type":"string","const":subject_id}),
    );
    properties.insert(
        "priority".into(),
        serde_json::json!({"type":"integer","minimum":0,"maximum":100}),
    );
    properties.insert(
        "state_references".into(),
        exact_string_array_schema(
            permitted_state_references,
            0,
            permitted_state_references.len(),
        ),
    );
    properties.insert(
        "public_channels".into(),
        exact_string_array_schema(information_channels, 0, 8),
    );
    properties.insert("effects".into(), effect_schema);
    Ok(action_schema)
}

fn exact_constituent_effect_bundle_schema(subject: &CellConstituentSlice) -> serde_json::Value {
    let activity_target_ids = subject
        .activity_targets
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let activity_location_ids = subject
        .location_ids
        .iter()
        .cloned()
        .chain(subject.reachable_destinations.keys().cloned())
        .chain(
            subject
                .migration_destinations
                .values()
                .map(|destination| destination.location_id.clone()),
        )
        .collect::<BTreeSet<_>>();
    let mut properties = null_cell_effect_lane_properties();
    match subject.subject_kind {
        crate::domain::AgencySubjectKind::Institution => {
            properties.insert(
                "institution".into(),
                nullable_effect_schema(serde_json::json!({
                    "type":"object",
                    "additionalProperties":false,
                    "required":["posture","location_ids"],
                    "properties":{
                        "posture":{"type":"string","minLength":1,"maxLength":crate::domain::MAX_POSTURE_CHARS},
                        "location_ids":exact_string_array_schema(&subject.location_ids, 0, 4)
                    }
                })),
            );
        }
        crate::domain::AgencySubjectKind::Gestalt => {
            properties.insert(
                "gestalt_pressure".into(),
                nullable_effect_schema(serde_json::json!({
                    "type":"object",
                    "additionalProperties":false,
                    "required":["pressure_additions","pressure_resolutions"],
                    "properties":{
                        "pressure_additions":{"type":"array","uniqueItems":true,"maxItems":4,"items":{"type":"string"}},
                        "pressure_resolutions":exact_string_slice_array_schema(&subject.pressures, 0, 4)
                    }
                })),
            );
            properties.insert(
                "gestalt_activities".into(),
                exact_activity_effects_schema(
                    &activity_target_ids,
                    &activity_location_ids,
                    0,
                    true,
                ),
            );
            if !subject.migration_destinations.is_empty() {
                properties.insert(
                    "gestalt_migration".into(),
                    nullable_effect_schema(exact_migration_effect_schema(
                        subject.migration_destinations.keys(),
                    )),
                );
            }
        }
        crate::domain::AgencySubjectKind::Actor => {
            if !subject.reachable_destinations.is_empty() {
                properties.insert(
                    "actor_move".into(),
                    nullable_effect_schema(serde_json::json!({
                        "type":"object",
                        "additionalProperties":false,
                        "required":["destination_id"],
                        "properties":{"destination_id":{"type":"string","enum":subject.reachable_destinations.keys().collect::<Vec<_>>()}}
                    })),
                );
            }
            properties.insert(
                "actor_activities".into(),
                exact_activity_effects_schema(
                    &activity_target_ids,
                    &activity_location_ids,
                    1,
                    false,
                ),
            );
        }
    }
    serde_json::json!({
        "type":"object",
        "additionalProperties":false,
        "minProperties":1,
        "properties":properties
    })
}

fn exact_member_effect_bundle_schema(member: &CellMemberSlice) -> serde_json::Value {
    let activity_target_ids = member
        .activity_targets
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let activity_location_ids = std::iter::once(member.source_location_id.clone())
        .chain(
            member
                .migration_destinations
                .values()
                .map(|destination| destination.location_id.clone()),
        )
        .collect::<BTreeSet<_>>();
    let mut properties = null_cell_effect_lane_properties();
    properties.insert(
        "member_activities".into(),
        exact_activity_effects_schema(&activity_target_ids, &activity_location_ids, 1, false),
    );
    if !member.migration_destinations.is_empty() {
        properties.insert(
            "member_migration".into(),
            nullable_effect_schema(exact_migration_effect_schema(
                member.migration_destinations.keys(),
            )),
        );
    }
    serde_json::json!({
        "type":"object",
        "additionalProperties":false,
        "minProperties":1,
        "properties":properties
    })
}

fn nullable_effect_schema(effect: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"anyOf":[effect,{"type":"null"}]})
}

fn null_cell_effect_lane_properties() -> serde_json::Map<String, serde_json::Value> {
    [
        "institution",
        "gestalt_pressure",
        "gestalt_activities",
        "gestalt_migration",
        "actor_move",
        "actor_activities",
        "member_activities",
        "member_migration",
    ]
    .into_iter()
    .map(|lane| (lane.to_owned(), serde_json::json!({"type":"null"})))
    .collect()
}

fn exact_migration_effect_schema<'a>(
    destinations: impl Iterator<Item = &'a String>,
) -> serde_json::Value {
    serde_json::json!({
        "type":"object",
        "additionalProperties":false,
        "required":["destination_gestalt_id"],
        "properties":{
            "destination_gestalt_id":{"type":"string","enum":destinations.collect::<Vec<_>>()}
        }
    })
}

fn exact_activity_scope_schema(
    target_ids: &BTreeSet<String>,
    location_ids: &BTreeSet<String>,
    minimum_locations: usize,
    minimum_targets: usize,
) -> serde_json::Value {
    serde_json::json!({
        "type":"object",
        "additionalProperties":false,
        "required":["target_subject_ids","location_ids"],
        "properties":{
            "target_subject_ids":exact_string_array_schema(target_ids, minimum_targets, 4),
            "location_ids":exact_string_array_schema(location_ids, minimum_locations, if minimum_locations == 1 { 1 } else { 4 })
        }
    })
}

fn exact_activity_effects_schema(
    target_ids: &BTreeSet<String>,
    location_ids: &BTreeSet<String>,
    minimum_locations: usize,
    allow_internal_coordination: bool,
) -> serde_json::Value {
    let local_scope = exact_activity_scope_schema(target_ids, location_ids, minimum_locations, 0);
    let relational_scope =
        exact_activity_scope_schema(target_ids, location_ids, minimum_locations, 1);
    let repeated = |scope| {
        nullable_effect_schema(serde_json::json!({
            "type":"array",
            "minItems":1,
            "maxItems":4,
            "items":scope
        }))
    };
    let mut properties: serde_json::Map<String, serde_json::Value> = serde_json::Map::from_iter([
        ("prepare".into(), repeated(local_scope.clone())),
        ("investigate".into(), repeated(local_scope.clone())),
        ("obstruct".into(), repeated(local_scope.clone())),
        ("communicate".into(), repeated(local_scope.clone())),
    ]);
    if allow_internal_coordination {
        properties.insert("coordinate".into(), repeated(local_scope));
    } else if !target_ids.is_empty() {
        properties.insert("coordinate".into(), repeated(relational_scope.clone()));
    }
    if !target_ids.is_empty() {
        properties.insert("recruit".into(), repeated(relational_scope.clone()));
        properties.insert("trade".into(), repeated(relational_scope));
    }
    serde_json::json!({
        "type":"object",
        "additionalProperties":false,
        "minProperties":1,
        "properties":properties
    })
}

fn exact_string_array_schema(
    values: &BTreeSet<String>,
    min_items: usize,
    max_items: usize,
) -> serde_json::Value {
    if values.is_empty() {
        serde_json::json!({
            "type":"array",
            "minItems":min_items,
            "maxItems":0,
            "items":{"type":"string"}
        })
    } else {
        serde_json::json!({
            "type":"array",
            "uniqueItems":true,
            "minItems":min_items,
            "maxItems":max_items,
            "items":{"type":"string","enum":values}
        })
    }
}

fn exact_string_slice_array_schema(
    values: &[String],
    min_items: usize,
    max_items: usize,
) -> serde_json::Value {
    exact_string_array_schema(&values.iter().cloned().collect(), min_items, max_items)
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
    let memories_add = schema
        .pointer_mut("/$defs/ActorStateDelta/properties/memories_add")
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| anyhow!("Persona proposal schema has no memory additions"))?;
    memories_add.insert("maxItems".into(), serde_json::json!(0));
    let identity_adoption = schema
        .pointer_mut("/$defs/ActorStateDelta/properties/identity_adoption")
        .ok_or_else(|| anyhow!("Persona proposal schema has no identity adoption"))?;
    *identity_adoption =
        serde_json::json!({"type":["string","null"],"minLength":1,"maxLength":160});
    if matches!(slice.subject_kind, PersonaSubjectKind::CohesiveGestalt) {
        for field in [
            "memories_add",
            "conditions_add",
            "conditions_remove",
            "goals_add",
        ] {
            let value = schema
                .pointer_mut(&format!("/$defs/ActorStateDelta/properties/{field}"))
                .ok_or_else(|| anyhow!("Persona proposal schema has no {field}"))?;
            *value = serde_json::json!({"type":"array","maxItems":0,"items":{"type":"string"}});
        }
        *schema
            .pointer_mut("/$defs/ActorStateDelta/properties/relationship_updates")
            .ok_or_else(|| anyhow!("Persona proposal schema has no relationship updates"))? =
            serde_json::json!({"type":"object","maxProperties":0,"additionalProperties":false});
        *schema
            .pointer_mut("/$defs/ActorStateDelta/properties/identity_adoption")
            .ok_or_else(|| anyhow!("Persona proposal schema has no identity adoption"))? =
            serde_json::json!({"type":"null"});
    }
    let root = schema
        .as_object_mut()
        .ok_or_else(|| anyhow!("Persona proposal schema is not an object"))?;
    let properties = root
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("Persona proposal schema has no root properties"))?;
    properties.insert(
        "speech".into(),
        serde_json::json!({"type":["string","null"],"minLength":1,"maxLength":1000}),
    );
    properties.insert(
        "deliberate_silence".into(),
        serde_json::json!({"type":"boolean"}),
    );
    if matches!(slice.subject_kind, PersonaSubjectKind::CohesiveGestalt) {
        properties.insert(
            "world_actions".into(),
            serde_json::json!({"type":"array","maxItems":0,"items":{"$ref":"#/$defs/WorldActionProposal"}}),
        );
    }
    root.entry("allOf")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .ok_or_else(|| anyhow!("Persona proposal schema allOf is not an array"))?
        .push(serde_json::json!({
            "anyOf":[
                {"required":["speech"],"properties":{"speech":{"type":"string","minLength":1,"maxLength":1000}}},
                {"required":["speech","private_delta"],"properties":{
                    "speech":{"type":"null"},
                    "private_delta":{"properties":{"identity_adoption":{"type":"null"}}}
                }}
            ]
        }));
    if matches!(
        slice.interaction_role,
        ActorInteractionRole::DirectResponseExpected
    ) {
        root.entry("allOf")
            .or_insert_with(|| serde_json::json!([]))
            .as_array_mut()
            .ok_or_else(|| anyhow!("Persona proposal schema allOf is not an array"))?
            .push(serde_json::json!({
                "anyOf":[
                    {"required":["speech"],"properties":{"speech":{"type":"string","minLength":1,"maxLength":1000}}},
                    {"required":["deliberate_silence"],"properties":{"deliberate_silence":{"const":true}}}
                ]
            }));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            AgencySubjectKind, SimulationCellMode, StrategicActivityKind, StrategicCellEffect,
        },
        model::{FixtureModel, ModelStageRequest},
    };
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tokio::sync::Barrier;

    #[test]
    fn interpreter_composes_distinct_activity_means_but_rejects_duplicates() {
        let first_activity = StrategicCellEffect::ActorActivity {
            actor_id: "actor:director".into(),
            activity: StrategicActivityKind::Investigate,
            target_subject_ids: vec![],
            location_ids: vec!["clinic".into()],
        };
        let second_activity = StrategicCellEffect::ActorActivity {
            actor_id: "actor:director".into(),
            activity: StrategicActivityKind::Communicate,
            target_subject_ids: vec!["institution:garrison".into()],
            location_ids: vec!["clinic".into()],
        };
        let movement = StrategicCellEffect::ActorMove {
            actor_id: "actor:director".into(),
            destination_id: "garrison".into(),
        };

        assert!(
            validate_effect_bundle(&[movement.clone(), first_activity.clone()]).is_ok(),
            "different orthogonal lanes must remain composable"
        );
        assert!(
            validate_effect_bundle(&[first_activity.clone(), second_activity]).is_ok(),
            "one strategic choice may preserve several distinct means"
        );
        assert!(
            validate_effect_bundle(&[first_activity.clone(), first_activity])
                .unwrap_err()
                .to_string()
                .contains("cannot repeat an exact typed effect")
        );
    }

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
                    assert!(
                        request
                            .lived_stream
                            .contains("Put detail_focus_subject_id first")
                    );
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
                            .contains("Only these projected perspectives may speak, choose")
                    );
                    Ok(
                        "Faction Six will publish a bounded position using its bulletin access."
                            .into(),
                    )
                }
                "cell_interpreter" => {
                    let call = self.interpreter_calls.fetch_add(1, Ordering::SeqCst);
                    let action_schema = request
                        .output_schema
                        .as_ref()
                        .expect("Interpreter step has a tool-owned schema");
                    let action_schema_text = serde_json::to_string(action_schema).unwrap();
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
                    assert!(request.lived_stream.contains("unvoiced subject"));
                    assert!(request.lived_stream.contains("at most 160 characters"));
                    assert!(request.lived_stream.contains(
                        "Merely inspecting a handcart, regulator, record, route, patient"
                    ));
                    assert!(request.lived_stream.contains("is investigate, not prepare"));
                    if call == 0 {
                        assert!(action_schema_text.contains("submit"));
                        assert!(!action_schema_text.contains("upsert_decision"));
                        return Ok(serde_json::json!({
                            "command":{
                                "kind":"submit",
                                "decisions":{"faction-06":{"action":{
                                    "subject_id":"faction-06",
                                    "intent":"continue weighing the position",
                                    "intended_effect":"retain the posture already in force",
                                    "priority":5,
                                    "state_references":["institution:faction-06"],
                                    "public_channels":["public bulletin"],
                                    "effects":{"institution":{"posture":"weighing whether to publish a position","location_ids":["forum"]}}
                                }}}
                            }
                        })
                        .to_string());
                    }
                    let repeated = serde_json::json!({
                        "command":{
                            "kind":"upsert_decision",
                            "subject_id":"faction-06",
                            "decision":{"action":{
                                "subject_id":"faction-06",
                                "intent":"continue weighing the position",
                                "intended_effect":"retain the posture already in force",
                                "priority":5,
                                "state_references":["institution:faction-06"],
                                "public_channels":["public bulletin"],
                                "effects":{"institution":{
                                    "posture":"weighing whether to publish a position",
                                    "location_ids":["forum"]
                                }}
                            }}
                        }
                    });
                    assert!(
                        jsonschema::validator_for(action_schema)
                            .unwrap()
                            .is_valid(&repeated),
                        "the correction schema must admit the exact rejected owner"
                    );
                    assert!(!action_schema_text.contains("submit"));
                    assert!(action_schema_text.contains("upsert_decision"));
                    assert!(!action_schema_text.contains("inspect_draft"));
                    self.saw_rejected_appraisal.store(
                        request.lived_stream.contains("local_validation")
                            && request
                                .lived_stream
                                .contains("weighing whether to publish a position")
                            && request.lived_stream.contains("faction-06"),
                        Ordering::SeqCst,
                    );
                    Ok(serde_json::json!({
                        "command":{
                            "kind":"upsert_decision",
                            "subject_id":"faction-06",
                            "decision":{"action":{
                                "subject_id":"faction-06",
                                "intent":"publish a position",
                                "intended_effect":"state its bounded institutional posture",
                                "priority":5,
                                "state_references":["institution:faction-06"],
                                "public_channels":["public bulletin"],
                                "effects":{"institution":{"posture":"published a bounded position","location_ids":["forum"]}}
                            }}
                        }
                    }).to_string())
                }
                "cell_effect_verifier" => {
                    assert!(request.lived_stream.contains("JSON object"));
                    assert!(request.lived_stream.contains("exact_subject_permission"));
                    assert!(
                        request
                            .lived_stream
                            .contains("\"reachable_destinations\":{}")
                    );
                    assert!(
                        request
                            .lived_stream
                            .contains("targetless local communicate")
                    );
                    assert!(request.lived_stream.contains("incidental walking"));
                    assert!(
                        request
                            .lived_stream
                            .contains("reject member_activity that reduces that commitment")
                    );
                    Ok(serde_json::json!({
                        "verdicts":[{
                            "action_index":0,
                            "result":"match",
                            "findings":[]
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
                reachable_destinations: BTreeMap::new(),
                migration_destinations: BTreeMap::new(),
                activity_targets: BTreeMap::new(),
                goals: vec!["publish a position".into()],
                relationships: BTreeMap::new(),
                memories: vec![],
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
            causal_follow_through: vec![],
            world_clock_pressure: vec!["vote 5/6".into()],
            canonical_locations: BTreeMap::from([("forum".into(), "Forum".into())]),
            detail_focus_subject_id: Some("faction-06".into()),
            decision_owner_ids: BTreeSet::from(["faction-06".into()]),
            max_actions: 1,
            source_receipt_ids: vec![],
        }
    }

    struct MissingDecisionRetryModel {
        projector_calls: AtomicUsize,
        persona_calls: AtomicUsize,
        interpreter_calls: AtomicUsize,
        saw_retry_guidance: AtomicBool,
        request_sources: Mutex<Vec<(String, Vec<String>)>>,
    }

    #[async_trait]
    impl ModelPort for MissingDecisionRetryModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            self.request_sources
                .lock()
                .unwrap()
                .push((request.stage.clone(), request.source_receipt_ids.clone()));
            match request.stage.as_str() {
                "cell_projector" => {
                    self.projector_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(serde_json::json!({
                        "segments":[{
                            "subject_id":"faction-06",
                            "narrative":"The deadline is visible, but the institution has not yet chosen a new course."
                        }]
                    })
                    .to_string())
                }
                "cell_persona" => {
                    let call = self.persona_calls.fetch_add(1, Ordering::SeqCst);
                    if call == 0 {
                        return Ok(
                            "Faction Six weighs the deadline and its existing posture.".into()
                        );
                    }
                    self.saw_retry_guidance.store(
                        request
                            .lived_stream
                            .contains("previous response supplied no explicit strategic decision"),
                        Ordering::SeqCst,
                    );
                    Ok("Faction Six explicitly chooses to hold its existing posture for this horizon.".into())
                }
                "cell_interpreter" => {
                    let call = self.interpreter_calls.fetch_add(1, Ordering::SeqCst);
                    if call == 0 {
                        return Ok(serde_json::json!({
                            "command":{
                                "kind":"submit",
                                "decisions":{"faction-06":{"undecided":{
                                    "reason":"The Persona supplied no explicit action or hold."
                                }}}
                            }
                        })
                        .to_string());
                    }
                    Ok(serde_json::json!({
                        "command":{
                            "kind":"submit",
                            "decisions":{"faction-06":{"inaction":{
                                "subject_id":"faction-06",
                                "reason":"Faction Six explicitly holds its existing posture for this horizon."
                            }}}
                        }
                    })
                    .to_string())
                }
                stage => Err(anyhow!("unexpected fixture stage {stage}")),
            }
        }

        fn provider(&self) -> &'static str {
            "missing-decision-retry-fixture"
        }
    }

    #[tokio::test]
    async fn missing_persona_decision_retries_the_lived_turn_not_the_interpreter_invention() {
        let model = Arc::new(MissingDecisionRetryModel {
            projector_calls: AtomicUsize::new(0),
            persona_calls: AtomicUsize::new(0),
            interpreter_calls: AtomicUsize::new(0),
            saw_retry_guidance: AtomicBool::new(false),
            request_sources: Mutex::new(Vec::new()),
        });
        let engine = CellProjectionEngine {
            model: model.clone(),
            permit: Arc::new(AllowAllPermit),
            projector_model: "flash".into(),
            persona_model: "flash".into(),
            interpreter_model: "flash".into(),
            campaign_contract: None,
            aggregate_boundaries: vec![],
        };

        let output = engine.execute(fixture_cell_slice()).await.unwrap();

        assert_eq!(model.projector_calls.load(Ordering::SeqCst), 1);
        assert_eq!(model.persona_calls.load(Ordering::SeqCst), 2);
        assert_eq!(model.interpreter_calls.load(Ordering::SeqCst), 2);
        assert!(model.saw_retry_guidance.load(Ordering::SeqCst));
        assert!(output.appraisal.actions.is_empty());
        assert_eq!(output.appraisal.inactions.len(), 1);
        assert_eq!(output.appraisal.inactions[0].subject_id, "faction-06");
        assert_eq!(output.stage_receipts.len(), 5);
        assert!(
            output
                .stage_receipts
                .iter()
                .all(|receipt| receipt.validation_result == "valid")
        );
        let receipt_ids = output
            .stage_receipts
            .iter()
            .map(|receipt| receipt.storage_key().to_owned())
            .collect::<Vec<_>>();
        let request_sources = model.request_sources.lock().unwrap();
        let persona_sources = request_sources
            .iter()
            .filter(|(stage, _)| stage == "cell_persona")
            .map(|(_, sources)| sources)
            .collect::<Vec<_>>();
        let interpreter_sources = request_sources
            .iter()
            .filter(|(stage, _)| stage == "cell_interpreter")
            .map(|(_, sources)| sources)
            .collect::<Vec<_>>();
        assert!(persona_sources[0].contains(&receipt_ids[0]));
        assert!(interpreter_sources[0].contains(&receipt_ids[0]));
        assert!(interpreter_sources[0].contains(&receipt_ids[1]));
        assert!(persona_sources[1].contains(&receipt_ids[0]));
        assert!(persona_sources[1].contains(&receipt_ids[1]));
        assert!(persona_sources[1].contains(&receipt_ids[2]));
        assert!(interpreter_sources[1].contains(&receipt_ids[3]));
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
            campaign_contract: None,
            aggregate_boundaries: vec![],
        };
        let output = engine.execute(fixture_cell_slice()).await.unwrap();
        assert!(model.saw_rejected_appraisal.load(Ordering::SeqCst));
        assert_eq!(output.stage_receipts.len(), 5);
        assert_eq!(
            output.stage_receipts[2].validation_result,
            "semantic_invalid"
        );
        assert!(matches!(
            output.appraisal.actions[0].effects[0],
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
                    "We will withhold the reserve commitment until the public count is verified, and notify the ward clerks of the delay."
                        .into(),
                ),
                "cell_interpreter" => {
                    let call = self.interpreter_calls.fetch_add(1, Ordering::SeqCst);
                    if call > 0 {
                        self.saw_verifier_rejection.store(
                            request.lived_stream.contains("effect_mismatch")
                                && request.lived_stream.contains("releases the reserve"),
                            Ordering::SeqCst,
                        );
                        assert!(request
                            .lived_stream
                            .contains("Preserve the Persona's exact withholding decision"));
                        assert!(request
                            .lived_stream
                            .contains("include the explicit notice to ward clerks"));
                    }
                    let decision = serde_json::json!({"action":{
                            "subject_id":"faction-06",
                            "intent":"state the reserve decision",
                            "intended_effect":if call == 0 {
                                "release the reserve immediately"
                            } else {
                                "withhold release pending a verified count and notify the ward clerks"
                            },
                            "priority":5,
                            "state_references":["institution:faction-06"],
                            "public_channels":["public bulletin"],
                            "effects":{"institution":{
                                "posture":if call == 0 {
                                    "releases the reserve immediately"
                                } else {
                                    "withhold reserve pending a verified public count; notify ward clerks of the delay"
                                },
                                "location_ids":["forum"]
                            }}
                        }});
                    Ok(if call == 0 {
                        serde_json::json!({
                            "command":{
                                "kind":"submit",
                                "decisions":{"faction-06":decision}
                            }
                        })
                    } else {
                        serde_json::json!({
                            "command":{
                                "kind":"upsert_decision",
                                "subject_id":"faction-06",
                                "decision":decision
                            }
                        })
                    }.to_string())
                }
                "cell_effect_verifier" => {
                    assert!(
                        request
                            .lived_stream
                            .contains("reject any effect that substitutes a containing population")
                    );
                    assert!(request
                        .lived_stream
                        .contains("sole map of canonical subjects, locations, and destinations"));
                    assert!(request.lived_stream.contains("exact_subject_permission"));
                    assert!(request
                        .lived_stream
                        .contains("empty target list is intentional"));
                    assert!(request.lived_stream.contains("At location forum"));
                    let call = self.verifier_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(serde_json::json!({
                        "verdicts":[{
                            "action_index":0,
                            "result":if call > 0 { "match" } else { "mismatch" },
                            "findings":if call > 0 {
                                Vec::<serde_json::Value>::new()
                            } else {
                                vec![
                                    serde_json::json!({
                                        "mismatch_kind":"effect_reversal",
                                        "repair_guidance":"Preserve the Persona's exact withholding decision rather than reversing it into release."
                                    }),
                                    serde_json::json!({
                                        "mismatch_kind":"effect_omission",
                                        "repair_guidance":"Rewrite the posture compactly to include the explicit notice to ward clerks as well as the withholding decision."
                                    })
                                ]
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
    async fn effect_verifier_delivers_multiple_findings_to_one_bounded_correction() {
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
            campaign_contract: None,
            aggregate_boundaries: vec![],
        }
        .execute(fixture_cell_slice())
        .await
        .unwrap();
        assert!(model.saw_verifier_rejection.load(Ordering::SeqCst));
        assert_eq!(model.interpreter_calls.load(Ordering::SeqCst), 2);
        assert_eq!(model.verifier_calls.load(Ordering::SeqCst), 2);
        assert_eq!(output.stage_receipts.len(), 6);
        assert_eq!(output.stage_receipts[3].stage, "cell_effect_verifier");
        assert_eq!(
            output.stage_receipts[3].validation_result,
            "semantic_invalid"
        );
        let StrategicCellEffect::Institution { posture, .. } =
            &output.appraisal.actions[0].effects[0]
        else {
            panic!("corrected effect changed type")
        };
        assert!(posture.contains("notify ward clerks"));
    }

    struct CachingVerifierModel {
        calls: AtomicUsize,
        faction_seven_calls: AtomicUsize,
    }

    #[async_trait]
    impl ModelPort for CachingVerifierModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            assert_eq!(request.stage, "cell_effect_verifier");
            self.calls.fetch_add(1, Ordering::SeqCst);
            if request.lived_stream.contains("steady-seven") {
                self.faction_seven_calls.fetch_add(1, Ordering::SeqCst);
            }
            let mismatch = request.lived_stream.contains("reverse-six");
            Ok(serde_json::json!({
                "verdicts":[{
                    "action_index":0,
                    "result":if mismatch { "mismatch" } else { "match" },
                    "findings":if mismatch {
                        vec![serde_json::json!({
                            "mismatch_kind":"effect_reversal",
                            "repair_guidance":"Preserve Faction Six's stated withholding rather than reversing it."
                        })]
                    } else {
                        Vec::<serde_json::Value>::new()
                    }
                }]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "caching-verifier-fixture"
        }
    }

    fn institution_decision(
        subject_id: &str,
        intended_effect: &str,
        posture: &str,
        state_reference: &str,
    ) -> serde_json::Value {
        serde_json::json!({"action":{
            "subject_id":subject_id,
            "intent":"state a bounded institutional course",
            "intended_effect":intended_effect,
            "priority":50,
            "state_references":[state_reference],
            "public_channels":["public bulletin"],
            "effects":{"institution":{"posture":posture,"location_ids":["forum"]}}
        }})
    }

    #[tokio::test]
    async fn interpreter_workbench_reuses_unchanged_valid_effect_verification() {
        let mut slice = fixture_cell_slice();
        let mut second = slice.constituents[0].clone();
        second.subject_id = "faction-07".into();
        second.name = "Faction Seven".into();
        second.permitted_state_references = BTreeSet::from(["institution:faction-07".into()]);
        second.current_posture = Some("observing the count".into());
        slice.constituents.push(second);
        slice.decision_owner_ids.insert("faction-07".into());
        slice.max_actions = 2;
        let active_subject_ids = slice.decision_owner_ids.clone();
        let model = Arc::new(CachingVerifierModel {
            calls: AtomicUsize::new(0),
            faction_seven_calls: AtomicUsize::new(0),
        });
        let mut appraisal_schema =
            serde_json::to_value(schema_for!(CellAppraisalProposal)).unwrap();
        constrain_cell_proposal_schema(&mut appraisal_schema, &slice, &active_subject_ids).unwrap();
        let mut workbench = CellInterpreterWorkbench {
            model: model.clone(),
            permit: Arc::new(AllowAllPermit),
            interpreter_model: "flash".into(),
            slice,
            active_subject_ids,
            lived_stream: "At location forum: Faction Six and Faction Seven.".into(),
            persona_turn: "Faction Six withholds; Faction Seven publishes.".into(),
            campaign_policy: "{}".into(),
            appraisal_schema,
            draft: BTreeMap::new(),
            repair_subject_ids: BTreeSet::new(),
            accepted_verifier_bindings: BTreeSet::new(),
        };
        let context = ModelAgentToolContext {
            source_receipt_ids: vec!["persona:one".into()],
            current_model_receipt: None,
        };
        let initial_schema = workbench.action_schema().unwrap();
        let initial_schema_text = serde_json::to_string(&initial_schema).unwrap();
        assert!(initial_schema_text.contains("submit"));
        assert!(!initial_schema_text.contains("upsert_decision"));
        let first = workbench
            .invoke(
                CellInterpreterAgentAction {
                    command: CellInterpreterAgentCommand::Submit {
                        decisions: BTreeMap::from([
                            (
                                "faction-06".into(),
                                institution_decision(
                                    "faction-06",
                                    "reverse-six",
                                    "release immediately",
                                    "institution:faction-06",
                                ),
                            ),
                            (
                                "faction-07".into(),
                                institution_decision(
                                    "faction-07",
                                    "steady-seven",
                                    "publish the verified count",
                                    "institution:faction-07",
                                ),
                            ),
                        ]),
                    },
                },
                &context,
            )
            .await;
        assert!(matches!(
            first,
            ModelAgentToolOutcome::Rejected {
                finding: CellInterpreterFinding::EffectMismatch { .. },
                ..
            }
        ));
        assert_eq!(model.calls.load(Ordering::SeqCst), 2);
        assert_eq!(model.faction_seven_calls.load(Ordering::SeqCst), 1);
        let repair_schema = workbench.action_schema().unwrap();
        let repair_schema_text = serde_json::to_string(&repair_schema).unwrap();
        assert!(!repair_schema_text.contains("submit"));
        assert!(repair_schema_text.contains("upsert_decision"));
        assert!(repair_schema_text.contains("faction-06"));
        assert!(!repair_schema_text.contains("faction-07"));
        assert!(!repair_schema_text.contains("inspect_draft"));
        assert!(!repair_schema_text.contains("remove_decision"));

        let wholesale_resubmit = workbench
            .invoke(
                CellInterpreterAgentAction {
                    command: CellInterpreterAgentCommand::Submit {
                        decisions: BTreeMap::new(),
                    },
                },
                &context,
            )
            .await;
        assert!(matches!(
            wholesale_resubmit,
            ModelAgentToolOutcome::Rejected {
                finding: CellInterpreterFinding::SubmitRequiresEmptyDraft { .. },
                ..
            }
        ));
        let unrelated_repair = workbench
            .invoke(
                CellInterpreterAgentAction {
                    command: CellInterpreterAgentCommand::UpsertDecision {
                        subject_id: "faction-07".into(),
                        decision: institution_decision(
                            "faction-07",
                            "rewrite-seven",
                            "replace the already accepted course",
                            "institution:faction-07",
                        ),
                    },
                },
                &context,
            )
            .await;
        assert!(matches!(
            unrelated_repair,
            ModelAgentToolOutcome::Rejected {
                finding: CellInterpreterFinding::DecisionNotRepairable { .. },
                ..
            }
        ));
        assert_eq!(model.calls.load(Ordering::SeqCst), 2);

        let second = workbench
            .invoke(
                CellInterpreterAgentAction {
                    command: CellInterpreterAgentCommand::UpsertDecision {
                        subject_id: "faction-06".into(),
                        decision: institution_decision(
                            "faction-06",
                            "withhold-six",
                            "withhold pending the verified count",
                            "institution:faction-06",
                        ),
                    },
                },
                &context,
            )
            .await;
        assert!(matches!(
            second,
            ModelAgentToolOutcome::Accepted {
                output: CellInterpreterAgentOutput::Appraisal(_),
                ..
            }
        ));
        assert_eq!(model.calls.load(Ordering::SeqCst), 3);
        assert_eq!(
            model.faction_seven_calls.load(Ordering::SeqCst),
            1,
            "the unchanged accepted action must not repay its semantic verifier"
        );
    }

    #[tokio::test]
    async fn interpreter_repair_keeps_other_unadmitted_owners_repairable() {
        let mut slice = fixture_cell_slice();
        let mut second = slice.constituents[0].clone();
        second.subject_id = "faction-07".into();
        second.name = "Faction Seven".into();
        second.permitted_state_references = BTreeSet::from(["institution:faction-07".into()]);
        slice.constituents.push(second);
        slice.decision_owner_ids.insert("faction-07".into());
        slice.max_actions = 2;
        let active_subject_ids = slice.decision_owner_ids.clone();
        let mut appraisal_schema =
            serde_json::to_value(schema_for!(CellAppraisalProposal)).unwrap();
        constrain_cell_proposal_schema(&mut appraisal_schema, &slice, &active_subject_ids).unwrap();
        let mut workbench = CellInterpreterWorkbench {
            model: Arc::new(CachingVerifierModel {
                calls: AtomicUsize::new(0),
                faction_seven_calls: AtomicUsize::new(0),
            }),
            permit: Arc::new(AllowAllPermit),
            interpreter_model: "flash".into(),
            slice,
            active_subject_ids,
            lived_stream: "At location forum: Faction Six and Faction Seven.".into(),
            persona_turn: "Both factions hold for the count.".into(),
            campaign_policy: "{}".into(),
            appraisal_schema,
            draft: BTreeMap::from([
                (
                    "faction-06".into(),
                    serde_json::json!({"inaction":{
                        "subject_id":"faction-06",
                        "reason":""
                    }}),
                ),
                (
                    "faction-07".into(),
                    serde_json::json!({"inaction":{
                        "subject_id":"faction-07",
                        "reason":""
                    }}),
                ),
            ]),
            repair_subject_ids: BTreeSet::from(["faction-06".into(), "faction-07".into()]),
            accepted_verifier_bindings: BTreeSet::new(),
        };
        let outcome = workbench
            .invoke(
                CellInterpreterAgentAction {
                    command: CellInterpreterAgentCommand::UpsertDecision {
                        subject_id: "faction-06".into(),
                        decision: serde_json::json!({"inaction":{
                            "subject_id":"faction-06",
                            "reason":"Faction Six deliberately holds for the verified count."
                        }}),
                    },
                },
                &ModelAgentToolContext {
                    source_receipt_ids: vec!["persona:one".into()],
                    current_model_receipt: None,
                },
            )
            .await;

        assert!(matches!(
            outcome,
            ModelAgentToolOutcome::Rejected {
                finding: CellInterpreterFinding::LocalValidation { .. },
                ..
            }
        ));
        let repair_schema_text =
            serde_json::to_string(&workbench.action_schema().unwrap()).unwrap();
        assert!(repair_schema_text.contains("faction-07"));
        assert!(!repair_schema_text.contains("submit"));
    }

    #[test]
    fn empty_cell_appraisal_requires_exact_attributed_inaction() {
        let mut slice = fixture_cell_slice();
        let mut appraisal = bind_cell_appraisal(
            &slice,
            &BTreeSet::from(["faction-06".into()]),
            CellAppraisalProposal {
                actions: vec![],
                inactions: vec![crate::domain::CellInaction {
                    subject_id: "faction-06".into(),
                    reason: "The institution deliberately holds its current position.".into(),
                }],
            },
        )
        .unwrap();
        validate_cell_appraisal(&slice, &appraisal).unwrap();
        appraisal.inactions[0].reason = "   ".into();
        let error = validate_cell_appraisal(&slice, &appraisal).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("inaction for subject faction-06 requires a non-empty reason")
        );

        appraisal.inactions[0].reason = "The institution holds.".into();
        appraisal.actions.push(crate::domain::CellActionProposal {
            subject_id: "faction-06".into(),
            intent: "publish a new position".into(),
            intended_effect: "adopt a bounded commitment".into(),
            priority: 50,
            state_references: vec!["institution:faction-06".into()],
            public_channels: vec!["public bulletin".into()],
            effects: vec![StrategicCellEffect::Institution {
                institution_id: "faction-06".into(),
                posture: "publishing a bounded new commitment".into(),
                location_ids: vec!["forum".into()],
            }],
        });
        assert!(
            validate_cell_appraisal(&slice, &appraisal)
                .unwrap_err()
                .to_string()
                .contains("subject faction-06 appears in both actions and inactions")
        );

        appraisal.inactions.clear();
        slice.max_actions = 2;
        appraisal.actions.push(appraisal.actions[0].clone());
        assert!(
            validate_cell_appraisal(&slice, &appraisal)
                .unwrap_err()
                .to_string()
                .contains("subject faction-06 has duplicate strategic actions")
        );
    }

    #[test]
    fn every_projected_decision_owner_requires_an_action_or_explicit_inaction() {
        let mut slice = fixture_cell_slice();
        let mut second = slice.constituents[0].clone();
        second.subject_id = "faction-07".into();
        second.name = "Faction Seven".into();
        second.permitted_state_references = BTreeSet::from(["institution:faction-07".into()]);
        slice.constituents.push(second);
        slice.max_actions = 2;
        let active = BTreeSet::from(["faction-06".into(), "faction-07".into()]);
        let mut appraisal = bind_cell_appraisal(
            &slice,
            &active,
            CellAppraisalProposal {
                actions: vec![],
                inactions: vec![crate::domain::CellInaction {
                    subject_id: "faction-06".into(),
                    reason: "Faction Six explicitly holds its current course.".into(),
                }],
            },
        )
        .unwrap();

        let error = validate_cell_appraisal(&slice, &appraisal).unwrap_err();
        assert!(error.to_string().contains("faction-07"));

        appraisal.inactions.push(crate::domain::CellInaction {
            subject_id: "faction-07".into(),
            reason: "Faction Seven explicitly waits for the vote count.".into(),
        });
        validate_cell_appraisal(&slice, &appraisal).unwrap();
        validate_active_decision_owners(&active, &appraisal).unwrap();
    }

    #[test]
    fn compact_cell_prompt_contract_is_valid_json() {
        let mut schema = serde_json::to_value(schema_for!(CellAppraisalProposal)).unwrap();
        constrain_cell_proposal_schema(
            &mut schema,
            &fixture_cell_slice(),
            &BTreeSet::from(["faction-06".into()]),
        )
        .unwrap();
        let schema_text = serde_json::to_string(&schema).unwrap();
        assert!(schema_text.contains("\"maxLength\":240"));
        assert!(schema.get("$defs").is_none());
        assert!(!schema_text.contains("\"$ref\""));
        assert!(schema.pointer("/properties/decisions").is_some());
        assert!(schema.pointer("/properties/actions").is_none());
        assert!(schema.pointer("/properties/inactions").is_none());
        let initial_schema =
            cell_interpreter_agent_schema(&schema, CellInterpreterSchemaState::Initial).unwrap();
        let initial_schema_text = serde_json::to_string(&initial_schema).unwrap();
        assert_eq!(initial_schema["type"], "object");
        assert!(initial_schema.get("oneOf").is_none());
        assert!(
            initial_schema
                .pointer("/properties/command/oneOf")
                .is_some()
        );
        assert!(initial_schema_text.contains("submit"));
        assert!(!initial_schema_text.contains("upsert_decision"));
        assert!(!initial_schema_text.contains("inspect_draft"));
        assert!(!initial_schema_text.contains("remove_decision"));
        assert!(initial_schema_text.contains("\"maxLength\":240"));
        assert!(
            jsonschema::validator_for(&initial_schema)
                .unwrap()
                .is_valid(&serde_json::json!({
                    "command":{
                        "kind":"submit",
                        "decisions":{
                            "faction-06":{
                                "inaction":{
                                    "subject_id":"faction-06",
                                    "reason":"Faction Six deliberately waits for the count."
                                }
                            }
                        }
                    }
                }))
        );
        assert!(
            !jsonschema::validator_for(&initial_schema)
                .unwrap()
                .is_valid(&serde_json::json!({
                    "command":{
                        "kind":"submit",
                        "decisions":{},
                        "decision":{"inaction":{"reason":"illegal mixed payload"}}
                    }
                }))
        );
        let mut provider_schema = initial_schema;
        crate::model_connector::project_strict_responses_schema(&mut provider_schema).unwrap();
        assert_eq!(provider_schema["type"], "object");
        assert!(
            provider_schema
                .pointer("/properties/command/anyOf")
                .is_some()
        );
        assert!(
            jsonschema::validator_for(&provider_schema)
                .unwrap()
                .is_valid(&serde_json::json!({
                    "command":{
                        "kind":"submit",
                        "decisions":{
                            "faction-06":{
                                "inaction":{
                                    "subject_id":"faction-06",
                                    "reason":"Faction Six deliberately waits for the count."
                                }
                            }
                        }
                    }
                }))
        );
        let repair_subject_ids = BTreeSet::from(["faction-06".to_owned()]);
        let repair_schema = cell_interpreter_agent_schema(
            &schema,
            CellInterpreterSchemaState::Repair(&repair_subject_ids),
        )
        .unwrap();
        let repair_schema_text = serde_json::to_string(&repair_schema).unwrap();
        assert!(repair_schema_text.contains("upsert_decision"));
        assert!(!repair_schema_text.contains("submit"));
        assert!(
            jsonschema::validator_for(&repair_schema)
                .unwrap()
                .is_valid(&serde_json::json!({
                    "command":{
                        "kind":"upsert_decision",
                        "subject_id":"faction-06",
                        "decision":{
                            "inaction":{
                                "subject_id":"faction-06",
                                "reason":"Faction Six deliberately waits for the count."
                            }
                        }
                    }
                }))
        );
        let mut provider_repair_schema = repair_schema;
        crate::model_connector::project_strict_responses_schema(&mut provider_repair_schema)
            .unwrap();
        assert!(
            jsonschema::validator_for(&provider_repair_schema)
                .unwrap()
                .is_valid(&serde_json::json!({
                    "command":{
                        "kind":"upsert_decision",
                        "subject_id":"faction-06",
                        "decision":{
                            "inaction":{
                                "subject_id":"faction-06",
                                "reason":"Faction Six deliberately waits for the count."
                            }
                        }
                    }
                }))
        );
        assert_eq!(
            exact_constituent_effect_bundle_schema(&fixture_cell_slice().constituents[0])
                .pointer("/properties/institution/anyOf/0/properties/posture/maxLength"),
            Some(&serde_json::json!(crate::domain::MAX_POSTURE_CHARS))
        );
    }

    #[test]
    fn cell_schema_keeps_unavailable_effect_lanes_null_only() {
        let mut slice = fixture_cell_slice();
        let actor_id = "relationship-anchor:reed".to_owned();
        {
            let actor = &mut slice.constituents[0];
            actor.subject_id = actor_id.clone();
            actor.subject_kind = AgencySubjectKind::Actor;
            actor.name = "Reed".into();
            actor.permitted_state_references =
                BTreeSet::from(["subject:relationship-anchor:reed".into()]);
            actor.information_channels.clear();
            actor.current_posture = None;
            actor.reachable_destinations.clear();
            actor.migration_destinations.clear();
            actor.activity_targets = BTreeMap::from([(
                "inst_zhestokost".into(),
                CellActivityTargetSlice {
                    name: "Zhestokost".into(),
                    locations: BTreeMap::from([("forum".into(), "Forum".into())]),
                },
            )]);
        }
        slice.detail_focus_subject_id = Some(actor_id.clone());
        let active = BTreeSet::from([actor_id]);
        let mut schema = serde_json::to_value(schema_for!(CellAppraisalProposal)).unwrap();
        constrain_cell_proposal_schema(&mut schema, &slice, &active).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let action = |effects| {
            serde_json::json!({
                "decisions":{"relationship-anchor:reed":{"action":{
                    "subject_id":"relationship-anchor:reed",
                    "intent":"keep the patients together",
                    "intended_effect":"make one bounded local attempt",
                    "priority":80,
                    "state_references":["subject:relationship-anchor:reed"],
                    "public_channels":[],
                    "effects":effects
                }}}
            })
        };

        assert!(validator.is_valid(&action(serde_json::json!({
            "actor_activities":{"prepare":[{
                "target_subject_ids":[],
                "location_ids":["forum"]
            }]}
        }))));
        assert!(!validator.is_valid(&serde_json::json!({"decisions":{}})));
        let mut conflicting_decision = action(serde_json::json!({
            "actor_activities":{"prepare":[{
                "target_subject_ids":[],
                "location_ids":["forum"]
            }]}
        }));
        conflicting_decision
            .pointer_mut("/decisions/relationship-anchor:reed")
            .and_then(serde_json::Value::as_object_mut)
            .unwrap()
            .insert(
                "inaction".into(),
                serde_json::json!({
                    "subject_id":"relationship-anchor:reed",
                    "reason":"Reed explicitly waits."
                }),
            );
        assert!(
            !validator.is_valid(&conflicting_decision),
            "one exact subject slot cannot encode both action and inaction"
        );
        assert!(validator.is_valid(&action(serde_json::json!({
            "actor_activities":{
                "investigate":[{
                    "target_subject_ids":[],
                    "location_ids":["forum"]
                }],
                "prepare":[{
                    "target_subject_ids":[],
                    "location_ids":["forum"]
                }]
            }
        }))));
        assert!(!validator.is_valid(&action(serde_json::json!({
            "actor_activities":{
                "investigate":[{
                    "target_subject_ids":[],
                    "location_ids":["forum"]
                }],
                "invented_duplicate_investigate":{
                    "target_subject_ids":["inst_zhestokost"],
                    "location_ids":["forum"]
                }
            }
        }))));
        assert!(!validator.is_valid(&action(serde_json::json!({
            "actor_move":{"destination_id":"forum"}
        }))));
        assert!(!validator.is_valid(&action(serde_json::json!({
            "gestalt_migration":{"destination_gestalt_id":"loc_water_ice_rail_corridor"}
        }))));
        assert!(!validator.is_valid(&action(serde_json::json!({
            "actor_activities":{"coordinate":[{
                "target_subject_ids":[],
                "location_ids":["forum"]
            }]}
        }))));
        assert!(validator.is_valid(&action(serde_json::json!({
            "actor_activities":{"obstruct":[{
                "target_subject_ids":[],
                "location_ids":["forum"]
            }]}
        }))));
        assert!(validator.is_valid(&action(serde_json::json!({
            "actor_activities":{"coordinate":[{
                "target_subject_ids":["inst_zhestokost"],
                "location_ids":["forum"]
            }]}
        }))));
        assert!(!validator.is_valid(&action(serde_json::json!({
            "actor_activities":{"coordinate":[{
                "target_subject_ids":["invented-target"],
                "location_ids":["forum"]
            }]}
        }))));
        assert!(!validator.is_valid(&action(serde_json::json!({
            "actor_activities":{"prepare":[{
                "target_subject_ids":[],
                "location_ids":["invented-location"]
            }]}
        }))));
        assert_eq!(
            allowed_constituent_effect_types(&slice.constituents[0]),
            vec!["actor_activities"]
        );
        let strict_shaped_action = action(serde_json::json!({
            "institution":null,
            "gestalt_pressure":null,
            "gestalt_activities":null,
            "gestalt_migration":null,
            "actor_move":null,
            "actor_activities":{
                "prepare":[{
                    "target_subject_ids":[],
                    "location_ids":["forum"]
                }],
                "coordinate":null,
                "investigate":null,
                "recruit":null,
                "obstruct":null,
                "trade":null,
                "communicate":null
            },
            "member_activities":null,
            "member_migration":null
        }));
        assert!(
            validator.is_valid(&strict_shaped_action),
            "the canonical return-path schema must accept the strict provider projection"
        );
        let mut strict_schema = schema;
        crate::model_connector::project_strict_responses_schema(&mut strict_schema).unwrap();
        let strict_effect_properties = strict_schema
            .pointer("/properties/decisions/properties/relationship-anchor:reed/anyOf/0/properties/action/properties/effects/properties")
            .and_then(serde_json::Value::as_object)
            .unwrap();
        assert_eq!(strict_effect_properties.len(), 8);
        let strict_validator = jsonschema::validator_for(&strict_schema).unwrap();
        assert!(strict_validator.is_valid(&strict_shaped_action));
        let mut unauthorized_move = strict_shaped_action;
        *unauthorized_move
            .pointer_mut("/decisions/relationship-anchor:reed/action/effects/actor_move")
            .unwrap() = serde_json::json!({"destination_id":"forum"});
        assert!(!strict_validator.is_valid(&unauthorized_move));
    }

    #[test]
    fn interpreter_context_disambiguates_target_location_from_reachable_destination() {
        let mut slice = fixture_cell_slice();
        let actor = &mut slice.constituents[0];
        actor.subject_id = "clinic-director".into();
        actor.subject_kind = AgencySubjectKind::Actor;
        actor.location_ids = BTreeSet::from(["junction".into()]);
        actor.current_posture = None;
        actor.reachable_destinations =
            BTreeMap::from([("garrison".into(), "Garrison Outpost".into())]);
        actor.activity_targets = BTreeMap::from([(
            "reed".into(),
            CellActivityTargetSlice {
                name: "Reed".into(),
                locations: BTreeMap::from([("junction".into(), "Kostolom Junction".into())]),
            },
        )]);
        let context = cell_interpreter_context(&slice, &BTreeSet::from(["clinic-director".into()]));

        assert_eq!(
            context.pointer("/exact_permissions/0/reachable_destinations/garrison"),
            Some(&serde_json::json!("Garrison Outpost"))
        );
        assert_eq!(
            context.pointer("/exact_permissions/0/activity_targets/reed/name"),
            Some(&serde_json::json!("Reed"))
        );
        assert_eq!(
            context.pointer("/exact_permissions/0/activity_targets/reed/locations/junction"),
            Some(&serde_json::json!("Kostolom Junction"))
        );
    }

    #[test]
    fn cell_schema_does_not_reject_a_large_exact_reference_slice() {
        let mut slice = fixture_cell_slice();
        let subject_id = slice.constituents[0].subject_id.clone();
        slice.constituents[0].permitted_state_references = (0..21)
            .map(|index| format!("reference:{index:02}"))
            .collect();
        let active = BTreeSet::from([subject_id]);
        let mut schema = serde_json::to_value(schema_for!(CellAppraisalProposal)).unwrap();
        constrain_cell_proposal_schema(&mut schema, &slice, &active).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let exact_references = slice.constituents[0]
            .permitted_state_references
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let subject_id = slice.constituents[0].subject_id.clone();
        let appraisal = serde_json::json!({
            "decisions":{(subject_id.clone()):{"action":{
                "subject_id":subject_id,
                "intent":"hold the exact supplied footing",
                "intended_effect":"prepare locally",
                "priority":80,
                "state_references":exact_references,
                "public_channels":[],
                "effects":{"institution":{
                    "posture":"withhold action pending an exact count",
                    "location_ids":[]
                }}
            }}}
        });

        assert!(validator.is_valid(&appraisal));
    }

    #[test]
    fn exact_empty_string_arrays_remain_valid_strict_response_schemas() {
        let schema = exact_string_array_schema(&BTreeSet::new(), 0, 8);
        assert_eq!(schema["maxItems"], 0);
        assert_eq!(schema["items"]["type"], "string");
        assert!(
            jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&serde_json::json!([]))
        );
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
            effects: vec![StrategicCellEffect::Institution {
                institution_id: "faction-06".into(),
                posture: "withholding pending verification".into(),
                location_ids: vec!["forum".into()],
            }],
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
    fn effect_verifier_separates_direct_contact_from_publication_authority() {
        assert!(CELL_EFFECT_VERIFIER_INSTRUCTIONS.contains(
            "An exact activity_targets entry is sufficient authority to attempt direct communication"
        ));
        assert!(CELL_EFFECT_VERIFIER_INSTRUCTIONS.contains(
            "allowed_persistent_publication_channels governs only durable public publication"
        ));
        assert!(CELL_EFFECT_VERIFIER_INSTRUCTIONS.contains(
            "apply only coordination_target_contract.rule for this exact attributed subject"
        ));
        assert!(CELL_EFFECT_VERIFIER_INSTRUCTIONS.contains(
            "Communication can therefore be faithfully combined with restraint without implying coordinate"
        ));
        assert!(CELL_EFFECT_VERIFIER_INSTRUCTIONS.contains(
            "Reject effect omission only when the Persona explicitly undertakes another observable act"
        ));
        assert!(CELL_EFFECT_VERIFIER_INSTRUCTIONS.contains(
            "target_subject_ids names the canonical addressees and candidate_action.public_channels names the simultaneous public reach"
        ));
        assert!(
            CELL_EFFECT_VERIFIER_INSTRUCTIONS
                .contains("Do not demand a second targetless communicate for that same utterance")
        );
        assert!(CELL_EFFECT_VERIFIER_INSTRUCTIONS.contains(
            "A matching current-location name denotes that place or its unnamed local public"
        ));
        let mut slice = fixture_cell_slice();
        slice.canonical_locations = BTreeMap::from([("yard".into(), "Thornweald Assembly".into())]);
        let permission = cell_action_verifier_permission(&slice, "faction-06").unwrap();
        assert!(permission.get("canonical_locations").is_none());
        assert!(
            permission["activity_targets"]
                .as_object()
                .unwrap()
                .is_empty()
        );
        let unauthorized = validate_constituent_effect(
            &slice.constituents[0],
            &StrategicCellEffect::Institution {
                institution_id: "faction-06".into(),
                posture: "moving the public deadline to the assembly".into(),
                location_ids: vec!["yard".into()],
            },
            None,
        )
        .unwrap_err();
        assert!(unauthorized.to_string().contains("exact allowed locations"));
    }

    #[test]
    fn effect_verifier_distinguishes_member_coordination_from_gestalt_coordination() {
        let member = coordination_target_contract(&serde_json::json!({
            "subject_kind":"gestalt_member",
            "source_gestalt_id":"raincross-households"
        }));
        assert_eq!(
            member.get("internal_population_target_subject_ids"),
            Some(&serde_json::json!(["raincross-households"]))
        );
        assert!(
            member["rule"]
                .as_str()
                .unwrap()
                .contains("targeting exactly raincross-households")
        );

        let qualified_member = coordination_target_contract(&serde_json::json!({
            "subject_kind":"gestalt_member",
            "source_gestalt_id":"gestalt:raincross-households"
        }));
        assert_eq!(
            qualified_member.get("internal_population_target_subject_ids"),
            Some(&serde_json::json!(["gestalt:raincross-households"]))
        );

        let gestalt = coordination_target_contract(&serde_json::json!({
            "subject_kind":"gestalt"
        }));
        assert_eq!(
            gestalt.get("internal_population_target_subject_ids"),
            Some(&serde_json::json!([]))
        );
        assert!(gestalt["rule"].as_str().unwrap().contains("targetless"));
    }

    #[test]
    fn effect_verifier_requires_one_ordered_verdict_per_action() {
        let verification = CellEffectVerification {
            verdicts: vec![
                CellActionEffectVerdict {
                    action_index: 0,
                    result: CellEffectMatchResult::Match,
                    findings: Vec::new(),
                },
                CellActionEffectVerdict {
                    action_index: 1,
                    result: CellEffectMatchResult::Mismatch,
                    findings: vec![CellEffectMismatchFinding {
                        mismatch_kind: CellEffectMismatchKind::TargetSubstitution,
                        repair_guidance: "Remove the substituted target and retain only the exact addressed subject."
                            .into(),
                    }],
                },
            ],
        };
        assert_eq!(
            validate_effect_verification(&verification, 2).unwrap(),
            vec![1]
        );

        let mut missing_guidance = verification.clone();
        missing_guidance.verdicts[1].findings[0]
            .repair_guidance
            .clear();
        assert!(
            validate_effect_verification(&missing_guidance, 2)
                .unwrap_err()
                .to_string()
                .contains("findings set")
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
        assert!(
            schema
                .pointer("/$defs/CellActionEffectVerdict/oneOf/0/properties/findings/items")
                .is_some(),
            "even a zero-length findings branch needs an items schema at the provider boundary"
        );
        let validator = jsonschema::validator_for(&schema).unwrap();
        let faithful = serde_json::json!({
            "verdicts":[
                {"action_index":0,"result":"match","findings":[]},
                {"action_index":1,"result":"mismatch","findings":[{"mismatch_kind":"target_substitution","repair_guidance":"Use the exact addressed subject."}]},
                {"action_index":2,"result":"match","findings":[]},
                {"action_index":3,"result":"match","findings":[]}
            ]
        });
        assert!(validator.is_valid(&faithful));

        let long_but_semantically_coherent_guidance = serde_json::json!({
            "verdicts":[
                {"action_index":0,"result":"mismatch","findings":[{"mismatch_kind":"wrong_effect_kind","repair_guidance":"The Persona chose a concrete journey, so preserve that exact decision with the supplied migration effect rather than downgrading it to coordination; if no supplied effect can represent the destination, remove this action and let the world retain the attributed choice without committing an invented route."}]},
                {"action_index":1,"result":"match","findings":[]},
                {"action_index":2,"result":"match","findings":[]},
                {"action_index":3,"result":"match","findings":[]}
            ]
        });
        assert!(!validator.is_valid(&long_but_semantically_coherent_guidance));

        let incoherent = serde_json::json!({
            "verdicts":[
                {"action_index":0,"result":"match","findings":[{"mismatch_kind":"invented_outcome","repair_guidance":"Do not claim this succeeded."}]},
                {"action_index":1,"result":"match","findings":[]},
                {"action_index":2,"result":"match","findings":[]},
                {"action_index":3,"result":"match","findings":[]}
            ]
        });
        assert!(!validator.is_valid(&incoherent));
    }

    struct ParallelActionVerifierModel {
        barrier: Arc<Barrier>,
        prompts: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl ModelPort for ParallelActionVerifierModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            assert_eq!(request.stage, "cell_effect_verifier");
            self.prompts
                .lock()
                .unwrap()
                .push(request.lived_stream.clone());
            self.barrier.wait().await;
            Ok(serde_json::json!({
                "verdicts":[{
                    "action_index":0,
                    "result":"match",
                    "findings":[]
                }]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "parallel-action-verifier-fixture"
        }
    }

    #[tokio::test]
    async fn effect_verifier_wave_is_parallel_and_subject_scoped() {
        let mut slice = fixture_cell_slice();
        let mut second = slice.constituents[0].clone();
        second.subject_id = "faction-07".into();
        second.name = "Faction Seven".into();
        second.permitted_state_references = BTreeSet::from(["institution:faction-07".into()]);
        second.current_posture = Some("holding a separate position".into());
        slice.constituents.push(second);
        let actions = [
            crate::domain::CellActionProposal {
                subject_id: "faction-06".into(),
                intent: "publish its position".into(),
                intended_effect: "state a bounded commitment".into(),
                priority: 10,
                state_references: vec!["institution:faction-06".into()],
                public_channels: vec!["public bulletin".into()],
                effects: vec![StrategicCellEffect::Institution {
                    institution_id: "faction-06".into(),
                    posture: "publishing a bounded position".into(),
                    location_ids: vec!["forum".into()],
                }],
            },
            crate::domain::CellActionProposal {
                subject_id: "faction-07".into(),
                intent: "publish its separate position".into(),
                intended_effect: "state a distinct bounded commitment".into(),
                priority: 9,
                state_references: vec!["institution:faction-07".into()],
                public_channels: vec!["public bulletin".into()],
                effects: vec![StrategicCellEffect::Institution {
                    institution_id: "faction-07".into(),
                    posture: "publishing a separate bounded position".into(),
                    location_ids: vec!["forum".into()],
                }],
            },
        ];
        let prompts = Arc::new(Mutex::new(Vec::new()));
        let model = Arc::new(ParallelActionVerifierModel {
            barrier: Arc::new(Barrier::new(2)),
            prompts: prompts.clone(),
        });

        let verified = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            run_cell_effect_verifier_wave(
                model,
                "flash",
                &slice,
                "Two separately attributed perspectives are active.",
                "Each institution chooses its own public statement.",
                "{}",
                &actions,
                &[],
            ),
        )
        .await
        .expect("parallel verifier wave deadlocked")
        .unwrap();

        assert_eq!(
            verified
                .iter()
                .map(|result| result.action_index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        for verification in &verified {
            assert_eq!(
                verification.output.receipt.snapshot_binding,
                cell_effect_verification_binding(
                    &slice.snapshot_binding,
                    std::slice::from_ref(&actions[verification.action_index])
                )
                .unwrap()
            );
        }
        for prompt in prompts.lock().unwrap().iter() {
            assert!(prompt.contains(
                "Snapshot-location activity precedes relocation; exact-destination activity follows arrival"
            ));
            assert!(prompt.contains(
                "\"cross_location_order\":\"snapshot_location_activity_then_relocation_then_destination_activity\""
            ));
            assert!(prompt.contains("\"field_order\":\"not_chronology\""));
            let context: serde_json::Value = serde_json::from_str(
                prompt
                    .split("CONTEXT:\n")
                    .nth(1)
                    .expect("verifier prompt contains one context document"),
            )
            .unwrap();
            assert_eq!(context["canonical_locations"]["forum"], "Forum");
            assert!(
                context["exact_subject_permission"]
                    .get("canonical_locations")
                    .is_none()
            );
        }
        let prompts = prompts.lock().unwrap();
        assert_eq!(prompts.len(), 2);
        let first = prompts
            .iter()
            .find(|prompt| prompt.contains("faction-06"))
            .unwrap();
        let second = prompts
            .iter()
            .find(|prompt| prompt.contains("faction-07"))
            .unwrap();
        assert!(!first.contains("faction-07"));
        assert!(!second.contains("faction-06"));
        assert!(first.contains("exact_subject_permission"));
        assert!(first.contains("\"state_references\":[\"institution:faction-06\"]"));
        assert!(first.contains("\"public_channels\":[\"public bulletin\"]"));
        assert!(!first.contains("exact_typed_permissions"));
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
            &BTreeSet::from(["crowd".into()]),
            CellAppraisalProposal {
                actions: vec![CellActionCandidate {
                    subject_id: "crowd".into(),
                    intent: "respond to the pressure".into(),
                    intended_effect: "change the collective situation".into(),
                    priority: 1,
                    state_references: vec!["gestalt:crowd".into()],
                    public_channels: vec![],
                    effects: CellEffectBundleCandidate {
                        gestalt_pressure: Some(CellGestaltPressureEffectCandidate {
                            pressure_additions: vec![],
                            pressure_resolutions: vec![],
                        }),
                        ..Default::default()
                    },
                }],
                inactions: vec![],
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
        subject.activity_targets = BTreeMap::from([(
            "dockers".into(),
            CellActivityTargetSlice {
                name: "Dockers".into(),
                locations: BTreeMap::from([("forum".into(), "Forum".into())]),
            },
        )]);
        let valid = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Coordinate,
            target_subject_ids: vec!["dockers".into()],
            location_ids: vec!["forum".into()],
        };
        validate_constituent_effect(subject, &valid, None).unwrap();

        let internal_coordination = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Coordinate,
            target_subject_ids: vec![],
            location_ids: vec!["forum".into()],
        };
        validate_constituent_effect(subject, &internal_coordination, None).unwrap();

        let invented_target = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Coordinate,
            target_subject_ids: vec!["unseen-ministry".into()],
            location_ids: vec!["forum".into()],
        };
        assert!(
            validate_constituent_effect(subject, &invented_target, None)
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
        validate_constituent_effect(subject, &targetless_obstruction, None).unwrap();

        let internal_preparation = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Prepare,
            target_subject_ids: vec![],
            location_ids: vec!["forum".into()],
        };
        validate_constituent_effect(subject, &internal_preparation, None).unwrap();

        let local_investigation = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Investigate,
            target_subject_ids: vec![],
            location_ids: vec!["forum".into()],
        };
        validate_constituent_effect(subject, &local_investigation, None).unwrap();

        let local_communication = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees".into(),
            activity: crate::domain::StrategicActivityKind::Communicate,
            target_subject_ids: vec![],
            location_ids: vec!["forum".into()],
        };
        validate_constituent_effect(subject, &local_communication, None).unwrap();
    }

    #[test]
    fn gestalt_migration_is_bound_to_the_exact_population_destination() {
        let mut slice = fixture_cell_slice();
        let subject = &mut slice.constituents[0];
        subject.subject_id = "refugees".into();
        subject.subject_kind = AgencySubjectKind::Gestalt;
        subject.migration_destinations = BTreeMap::from([(
            "harbor-neighbors".into(),
            CellMigrationDestinationSlice {
                population_name: "Harbor Neighbors".into(),
                location_id: "south-harbor".into(),
                location_name: "South Harbor".into(),
            },
        )]);
        let valid = StrategicCellEffect::GestaltMigration {
            destination_gestalt_id: "harbor-neighbors".into(),
        };
        validate_constituent_effect(subject, &valid, None).unwrap();

        let invented = StrategicCellEffect::GestaltMigration {
            destination_gestalt_id: "palace-court".into(),
        };
        assert!(
            validate_constituent_effect(subject, &invented, None)
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
            activity_targets: BTreeMap::from([(
                "refugees".into(),
                CellActivityTargetSlice {
                    name: "Refugee Convoy".into(),
                    locations: BTreeMap::from([("forum".into(), "Forum".into())]),
                },
            )]),
            goals: vec![],
            pressures: vec![],
            relationships: BTreeMap::new(),
            memories: vec![],
        });
        let appraisal = bind_cell_appraisal(
            &slice,
            &BTreeSet::from(["member:mira".into()]),
            CellAppraisalProposal {
                actions: vec![CellActionCandidate {
                    subject_id: "member:mira".into(),
                    intent: "offer to help repair the shelter".into(),
                    intended_effect: "make the offer to the refugees".into(),
                    priority: 70,
                    state_references: vec!["member:mira".into()],
                    public_channels: vec![],
                    effects: CellEffectBundleCandidate {
                        member_activities: Some(CellActivitySetCandidate {
                            communicate: Some(vec![CellActivityScopeCandidate {
                                target_subject_ids: vec!["refugees".into()],
                                location_ids: vec!["forum".into()],
                            }]),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                }],
                inactions: vec![],
            },
        )
        .unwrap();
        validate_cell_appraisal(&slice, &appraisal).unwrap();

        let mut targetless_coordination = appraisal.clone();
        let StrategicCellEffect::MemberActivity {
            activity,
            target_subject_ids,
            ..
        } = &mut targetless_coordination.actions[0].effects[0]
        else {
            unreachable!()
        };
        *activity = crate::domain::StrategicActivityKind::Coordinate;
        target_subject_ids.clear();
        assert!(validate_cell_appraisal(&slice, &targetless_coordination).is_err());

        let mut stolen = appraisal;
        let StrategicCellEffect::MemberActivity { member_id, .. } =
            &mut stolen.actions[0].effects[0]
        else {
            unreachable!()
        };
        *member_id = "somebody-else".into();
        assert!(validate_cell_appraisal(&slice, &stolen).is_err());
    }

    #[test]
    fn canonical_actor_activity_stays_attributed_to_the_actor() {
        let mut slice = fixture_cell_slice();
        let subject = &mut slice.constituents[0];
        subject.subject_id = "actor:liaison".into();
        subject.subject_kind = AgencySubjectKind::Actor;
        subject.name = "Liaison".into();
        subject.collective_authority_id = None;
        subject.location_ids = BTreeSet::from(["forum".into()]);
        subject.permitted_state_references = BTreeSet::from(["subject:actor:liaison".into()]);
        subject.activity_targets = BTreeMap::from([(
            "clinic".into(),
            CellActivityTargetSlice {
                name: "Clinic".into(),
                locations: BTreeMap::from([("forum".into(), "Forum".into())]),
            },
        )]);
        subject.current_posture = None;

        let appraisal = bind_cell_appraisal(
            &slice,
            &BTreeSet::from(["actor:liaison".into()]),
            CellAppraisalProposal {
                actions: vec![CellActionCandidate {
                    subject_id: "actor:liaison".into(),
                    intent: "ask the clinic for its shortage count".into(),
                    intended_effect: "send the request without inventing a reply".into(),
                    priority: 70,
                    state_references: vec!["subject:actor:liaison".into()],
                    public_channels: vec![],
                    effects: CellEffectBundleCandidate {
                        actor_activities: Some(CellActivitySetCandidate {
                            communicate: Some(vec![CellActivityScopeCandidate {
                                target_subject_ids: vec!["clinic".into()],
                                location_ids: vec!["forum".into()],
                            }]),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                }],
                inactions: vec![],
            },
        )
        .unwrap();
        validate_cell_appraisal(&slice, &appraisal).unwrap();
        assert!(matches!(
            &appraisal.actions[0].effects[0],
            StrategicCellEffect::ActorActivity { actor_id, .. } if actor_id == "actor:liaison"
        ));

        let wrong_lane = bind_cell_appraisal(
            &slice,
            &BTreeSet::from(["actor:liaison".into()]),
            CellAppraisalProposal {
                actions: vec![CellActionCandidate {
                    subject_id: "actor:liaison".into(),
                    intent: "ask the clinic for its shortage count".into(),
                    intended_effect: "send the request without inventing a reply".into(),
                    priority: 70,
                    state_references: vec!["subject:actor:liaison".into()],
                    public_channels: vec![],
                    effects: CellEffectBundleCandidate {
                        member_activities: Some(CellActivitySetCandidate {
                            communicate: Some(vec![CellActivityScopeCandidate {
                                target_subject_ids: vec!["clinic".into()],
                                location_ids: vec!["forum".into()],
                            }]),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                }],
                inactions: vec![],
            },
        )
        .unwrap_err();
        assert!(
            wrong_lane
                .to_string()
                .contains("is not a selected member exception")
        );
    }

    #[test]
    fn actor_can_compose_exact_travel_and_destination_activity() {
        let mut slice = fixture_cell_slice();
        let subject = &mut slice.constituents[0];
        subject.subject_id = "actor:director".into();
        subject.subject_kind = AgencySubjectKind::Actor;
        subject.name = "Clinic Director".into();
        subject.collective_authority_id = None;
        subject.location_ids = BTreeSet::from(["garrison".into()]);
        subject.reachable_destinations =
            BTreeMap::from([("encampment".into(), "Refugee Encampment".into())]);
        subject.permitted_state_references =
            BTreeSet::from(["subject:actor:director".into(), "location:garrison".into()]);
        subject.activity_targets = BTreeMap::from([
            (
                "workers".into(),
                CellActivityTargetSlice {
                    name: "Kiln Hands".into(),
                    locations: BTreeMap::from([("garrison".into(), "Garrison".into())]),
                },
            ),
            (
                "board".into(),
                CellActivityTargetSlice {
                    name: "Water Board".into(),
                    locations: BTreeMap::from([("encampment".into(), "Refugee Encampment".into())]),
                },
            ),
        ]);
        subject.current_posture = None;

        let proposal = bind_cell_appraisal(
            &slice,
            &BTreeSet::from(["actor:director".into()]),
            CellAppraisalProposal {
                actions: vec![CellActionCandidate {
                    subject_id: "actor:director".into(),
                    intent: "travel to the encampment and ask what supplies are needed".into(),
                    intended_effect: "arrive there and make the request without inventing a reply"
                        .into(),
                    priority: 70,
                    state_references: vec!["subject:actor:director".into()],
                    public_channels: vec![],
                    effects: CellEffectBundleCandidate {
                        actor_move: Some(CellActorMoveEffectCandidate {
                            destination_id: "encampment".into(),
                        }),
                        actor_activities: Some(CellActivitySetCandidate {
                            communicate: Some(vec![
                                CellActivityScopeCandidate {
                                    target_subject_ids: vec!["workers".into()],
                                    location_ids: vec!["garrison".into()],
                                },
                                CellActivityScopeCandidate {
                                    target_subject_ids: vec!["board".into()],
                                    location_ids: vec!["encampment".into()],
                                },
                            ]),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                }],
                inactions: vec![],
            },
        )
        .unwrap();
        validate_cell_appraisal(&slice, &proposal).unwrap();
        assert_eq!(proposal.actions[0].effects.len(), 3);

        let mut omitted_travel = proposal;
        omitted_travel.actions[0].effects.remove(0);
        assert!(validate_cell_appraisal(&slice, &omitted_travel).is_err());
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
            None,
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
            projector_context["constituents"][0]["already_committed_posture"],
            "weighing whether to publish a position"
        );
        assert!(projector_context["constituents"][0]["current_posture"].is_null());
        assert_eq!(
            projector_context["constituents"][0]["pressures"][0],
            "the vote is near"
        );

        validate_constituent_effect(
            &slice.constituents[0],
            &StrategicCellEffect::Institution {
                institution_id: "faction-06".into(),
                posture: "x".repeat(crate::domain::MAX_POSTURE_CHARS),
                location_ids: vec!["forum".into()],
            },
            None,
        )
        .unwrap();
        let error = validate_constituent_effect(
            &slice.constituents[0],
            &StrategicCellEffect::Institution {
                institution_id: "faction-06".into(),
                posture: "x".repeat(crate::domain::MAX_POSTURE_CHARS + 1),
                location_ids: vec!["forum".into()],
            },
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("exact maximum is 460"));
    }

    struct CausalFixtureModel {
        request_sources: Mutex<Vec<(String, Vec<String>)>>,
    }

    #[async_trait]
    impl ModelPort for CausalFixtureModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            self.request_sources
                .lock()
                .unwrap()
                .push((request.stage.clone(), request.source_receipt_ids.clone()));
            FixtureModel.run(request).await
        }

        fn provider(&self) -> &'static str {
            "causal-fixture"
        }
    }

    #[tokio::test]
    async fn persona_receives_only_projected_stream() {
        let model = Arc::new(CausalFixtureModel {
            request_sources: Mutex::new(Vec::new()),
        });
        let engine = PersonaProjectionEngine {
            model: model.clone(),
            permit: Arc::new(AllowAllPermit),
            projector_model: "flash".into(),
            persona_model: "pro".into(),
            interpreter_model: "flash".into(),
        };
        let slice = PermittedActorSlice {
            actor_id: "npc".into(),
            location_id: "room".into(),
            subject_kind: PersonaSubjectKind::IndividualActor,
            snapshot_binding: "campaign:1".into(),
            interaction_role: ActorInteractionRole::PresentObserver,
            identity_experience: vec!["A tired navigator".into()],
            reserved_public_identities: BTreeSet::new(),
            memories: vec!["Proposed the eastern trail evacuation at dusk.".into()],
            recent_self_authored_turns: vec![
                "At world revision 3, your committed public response was exactly: Use the eastern trail.".into(),
            ],
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
        let grounded = ground_actor_lived_stream(&slice, "The crowd turns toward you.");
        assert!(grounded.contains("Your active self-identity: A tired navigator"));
        assert!(grounded.contains("What you remember experiencing or being told"));
        assert!(grounded.contains("Proposed the eastern trail evacuation at dusk."));
        assert!(grounded.contains("Use the eastern trail."));
        assert!(grounded.contains("does not prove that the content"));
        assert!(grounded.contains("not omniscient proof"));
        let result = engine.execute(slice).await.unwrap();
        assert_eq!(result.stage_receipts.len(), 3);
        assert_eq!(result.proposals.reaction_priority, 0);
        let receipt_ids = result
            .stage_receipts
            .iter()
            .map(|receipt| receipt.storage_key().to_owned())
            .collect::<Vec<_>>();
        let request_sources = model.request_sources.lock().unwrap();
        let persona_sources = request_sources
            .iter()
            .find(|(stage, _)| stage == "persona")
            .map(|(_, sources)| sources)
            .unwrap();
        let interpreter_sources = request_sources
            .iter()
            .find(|(stage, _)| stage == "interpreter")
            .map(|(_, sources)| sources)
            .unwrap();
        assert!(persona_sources.contains(&receipt_ids[0]));
        assert!(interpreter_sources.contains(&receipt_ids[0]));
        assert!(interpreter_sources.contains(&receipt_ids[1]));
    }

    #[test]
    fn persona_identity_adoption_must_be_spoken_exactly() {
        let mut slice = PermittedActorSlice {
            actor_id: "npc".into(),
            location_id: "room".into(),
            subject_kind: PersonaSubjectKind::IndividualActor,
            snapshot_binding: "campaign:1".into(),
            interaction_role: ActorInteractionRole::DirectResponseExpected,
            identity_experience: vec!["You are an unnamed patient.".into()],
            reserved_public_identities: BTreeSet::new(),
            memories: vec![],
            recent_self_authored_turns: vec![],
            perceived_events: vec!["The player asks your name.".into()],
            perceived_actors: BTreeMap::from([("player".into(), "Player".into())]),
            relationships: vec![],
            goals: vec![],
            knowledge: vec![],
            capabilities: vec![],
            pressures: vec![],
            affordances: vec![],
            source_receipt_ids: vec![],
        };
        let mut proposals = PersonaProposalBundle {
            private_delta: crate::domain::ActorStateDelta {
                identity_adoption: Some("Taren".into()),
                ..Default::default()
            },
            speech: Some("Call me Rook.".into()),
            deliberate_silence: false,
            reaction_priority: 0,
            world_actions: vec![],
        };
        assert!(
            validate_actor_proposals(&slice, &proposals)
                .unwrap_err()
                .to_string()
                .contains("exact spoken handle")
        );
        proposals.speech = Some("My name is Taren.".into());
        validate_actor_proposals(&slice, &proposals).unwrap();
        slice.reserved_public_identities.insert(" tArEn ".into());
        assert!(
            validate_actor_proposals(&slice, &proposals)
                .unwrap_err()
                .to_string()
                .contains("established peer identity")
        );
    }

    #[test]
    fn actor_interpreter_schema_forbids_silent_identity_adoption() {
        let slice = PermittedActorSlice {
            actor_id: "npc".into(),
            location_id: "room".into(),
            subject_kind: PersonaSubjectKind::IndividualActor,
            snapshot_binding: "campaign:1".into(),
            interaction_role: ActorInteractionRole::PresentObserver,
            identity_experience: vec!["You are an unnamed patient.".into()],
            reserved_public_identities: BTreeSet::new(),
            memories: vec![],
            recent_self_authored_turns: vec![],
            perceived_events: vec!["A regulator is inspected.".into()],
            perceived_actors: BTreeMap::from([("player".into(), "Player".into())]),
            relationships: vec![],
            goals: vec![],
            knowledge: vec![],
            capabilities: vec![],
            pressures: vec![],
            affordances: vec![],
            source_receipt_ids: vec![],
        };
        let mut schema = serde_json::to_value(schema_for!(PersonaProposalBundle)).unwrap();
        constrain_interpreter_schema(&mut schema, &slice).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let silent_adoption = serde_json::to_value(PersonaProposalBundle {
            private_delta: crate::domain::ActorStateDelta {
                identity_adoption: Some("Taren".into()),
                ..Default::default()
            },
            speech: None,
            deliberate_silence: false,
            reaction_priority: 0,
            world_actions: vec![],
        })
        .unwrap();
        assert!(!validator.is_valid(&silent_adoption));
    }

    struct CorrectingActorModel {
        interpreter_calls: AtomicUsize,
        saw_rejected_interpretation: AtomicBool,
    }

    #[async_trait]
    impl ModelPort for CorrectingActorModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "projector" => Ok("The player asks you to identify yourself.".into()),
                "persona" => Ok("My name is Taren.".into()),
                "interpreter" => {
                    let correction = self.interpreter_calls.fetch_add(1, Ordering::SeqCst) > 0;
                    if correction {
                        self.saw_rejected_interpretation.store(
                            request
                                .lived_stream
                                .contains("PREVIOUS_REJECTED_INTERPRETATION")
                                && request.lived_stream.contains("Rook")
                                && request.lived_stream.contains("exact spoken handle"),
                            Ordering::SeqCst,
                        );
                    }
                    Ok(serde_json::to_string(&PersonaProposalBundle {
                        private_delta: crate::domain::ActorStateDelta {
                            identity_adoption: Some(
                                if correction { "Taren" } else { "Rook" }.into(),
                            ),
                            ..Default::default()
                        },
                        speech: Some("My name is Taren.".into()),
                        deliberate_silence: false,
                        reaction_priority: 10,
                        world_actions: vec![],
                    })?)
                }
                stage => Err(anyhow!("unexpected fixture stage {stage}")),
            }
        }

        fn provider(&self) -> &'static str {
            "correcting-actor-fixture"
        }
    }

    #[tokio::test]
    async fn actor_semantic_retry_preserves_same_snapshot_and_receipts() {
        let model = Arc::new(CorrectingActorModel {
            interpreter_calls: AtomicUsize::new(0),
            saw_rejected_interpretation: AtomicBool::new(false),
        });
        let engine = PersonaProjectionEngine {
            model: model.clone(),
            permit: Arc::new(AllowAllPermit),
            projector_model: "flash".into(),
            persona_model: "pro".into(),
            interpreter_model: "flash".into(),
        };
        let output = engine
            .execute(PermittedActorSlice {
                actor_id: "npc".into(),
                location_id: "room".into(),
                subject_kind: PersonaSubjectKind::IndividualActor,
                snapshot_binding: "campaign:1:revision:4".into(),
                interaction_role: ActorInteractionRole::DirectResponseExpected,
                identity_experience: vec!["You are an unnamed patient.".into()],
                reserved_public_identities: BTreeSet::new(),
                memories: vec![],
                recent_self_authored_turns: vec![],
                perceived_events: vec!["The player asks your name.".into()],
                perceived_actors: BTreeMap::from([("player".into(), "Player".into())]),
                relationships: vec![],
                goals: vec![],
                knowledge: vec![],
                capabilities: vec![],
                pressures: vec![],
                affordances: vec![],
                source_receipt_ids: vec![],
            })
            .await
            .unwrap();
        assert!(model.saw_rejected_interpretation.load(Ordering::SeqCst));
        assert_eq!(output.stage_receipts.len(), 4);
        assert_eq!(
            output.stage_receipts[2].validation_result,
            "semantic_invalid"
        );
        assert_eq!(
            output.proposals.private_delta.identity_adoption.as_deref(),
            Some("Taren")
        );
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
        constrain_cell_proposal_schema(
            &mut schema,
            &fixture_cell_slice(),
            &BTreeSet::from(["faction-06".into()]),
        )
        .unwrap();
        assert_eq!(
            schema.pointer("/properties/decisions/properties/faction-06/anyOf/0/properties/action/properties/priority/minimum"),
            Some(&serde_json::json!(0))
        );
        assert_eq!(
            schema.pointer("/properties/decisions/properties/faction-06/anyOf/0/properties/action/properties/priority/maximum"),
            Some(&serde_json::json!(100))
        );
    }

    #[test]
    fn activity_schema_omits_relation_dependent_choices_without_an_exact_target() {
        let no_target = exact_activity_effects_schema(
            &BTreeSet::new(),
            &BTreeSet::from(["village".into()]),
            0,
            false,
        );
        let no_target_properties = no_target["properties"].as_object().unwrap();
        assert_eq!(
            no_target_properties
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["communicate", "investigate", "obstruct", "prepare"])
        );

        let exact_target = exact_activity_effects_schema(
            &BTreeSet::from(["refugee-convoy".into()]),
            &BTreeSet::from(["village".into()]),
            0,
            false,
        );
        let exact_target_properties = exact_target["properties"].as_object().unwrap();
        assert!(exact_target_properties.contains_key("coordinate"));
        assert!(exact_target_properties.contains_key("recruit"));
        assert!(exact_target_properties.contains_key("trade"));
    }

    #[test]
    fn cohesive_gestalt_activity_can_coordinate_its_own_unnamed_members() {
        let schema = exact_activity_effects_schema(
            &BTreeSet::new(),
            &BTreeSet::from(["village".into()]),
            0,
            true,
        );
        let validator = jsonschema::validator_for(&schema).unwrap();

        assert!(validator.is_valid(&serde_json::json!({
            "coordinate":[{
                "target_subject_ids":[],
                "location_ids":["village"]
            }]
        })));
        assert!(!validator.is_valid(&serde_json::json!({
            "coordinate":[{
                "target_subject_ids":["unadmitted-external-population"],
                "location_ids":["village"]
            }]
        })));
    }

    #[test]
    fn cell_projection_binds_exact_unique_perspective_owners_into_prose() {
        let slice = fixture_cell_slice();
        let (narrative, active_subject_ids) = bind_cell_projection(
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
        assert!(narrative.contains("Faction Six's grounded agency"));
        assert!(narrative.contains("No distinct travel destination is presently established"));
        assert!(narrative.contains("No named distant person or body is presently available"));
        assert!(
            narrative
                .contains("established channels for publishing an attempt are: public bulletin")
        );
        assert!(!narrative.contains("faction-06"));
        assert_eq!(active_subject_ids, BTreeSet::from(["faction-06".into()]));

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
    fn projected_actor_receives_exact_lived_travel_and_contact_footing() {
        let mut slice = fixture_cell_slice();
        let actor = &mut slice.constituents[0];
        actor.subject_kind = AgencySubjectKind::Actor;
        actor.name = "Clinic Director".into();
        actor.information_channels.clear();
        actor.reachable_destinations =
            BTreeMap::from([("junction".into(), "Kostolom Junction".into())]);
        actor.activity_targets = BTreeMap::from([(
            "commander".into(),
            CellActivityTargetSlice {
                name: "Commander Voss".into(),
                locations: BTreeMap::from([("junction".into(), "Kostolom Junction".into())]),
            },
        )]);
        let (narrative, _) = bind_cell_projection(
            &slice,
            CellProjectionProposal {
                segments: vec![CellPerspectiveSegment {
                    subject_id: "faction-06".into(),
                    narrative: "The clinic's needs still press for an answer.".into(),
                }],
            },
        )
        .unwrap();

        assert!(narrative.contains("choose to travel to are: Kostolom Junction"));
        assert!(narrative.contains("try to reach are: Commander Voss at Kostolom Junction"));
        assert!(narrative.contains("Somewhere else would first require finding a route"));
        assert!(narrative.contains("no established channel for publishing this attempt"));
    }

    #[test]
    fn projected_subjects_are_the_only_persona_and_interpreter_decision_owners() {
        let mut slice = fixture_cell_slice();
        let mut inactive = slice.constituents[0].clone();
        inactive.subject_id = "faction-07".into();
        inactive.name = "Faction Seven".into();
        slice.constituents.push(inactive);
        slice.max_actions = 2;
        let active = BTreeSet::from(["faction-06".into()]);

        let boundaries = cell_scene_boundaries(&slice, &active);
        assert!(boundaries.contains("Faction Six"));
        assert!(!boundaries.contains("Faction Seven"));
        let context = cell_interpreter_context(&slice, &active);
        let projector_context = cell_projector_context(&slice);
        assert_eq!(projector_context["canonical_locations"]["forum"], "Forum");
        assert_eq!(context["canonical_locations"]["forum"], "Forum");
        assert_eq!(context["exact_permissions"].as_array().unwrap().len(), 1);
        assert_eq!(context["exact_permissions"][0]["subject_id"], "faction-06");

        let mut schema = serde_json::to_value(schema_for!(CellAppraisalProposal)).unwrap();
        constrain_cell_proposal_schema(&mut schema, &slice, &active).unwrap();
        assert_eq!(
            schema
                .pointer("/properties/decisions/properties/faction-06/anyOf/0/properties/action/properties/subject_id/const")
                .unwrap(),
            &serde_json::json!("faction-06")
        );
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
        assert!(error.to_string().contains("required perspective owner"));
    }

    #[test]
    fn cell_projection_cannot_erase_a_selected_member_exception() {
        let mut slice = fixture_cell_slice();
        slice.max_actions = 2;
        slice.decision_owner_ids.insert("member:mira".into());
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
            activity_targets: BTreeMap::new(),
            goals: vec!["choose whether to leave".into()],
            pressures: vec!["the final ferry is boarding".into()],
            relationships: BTreeMap::new(),
            memories: vec![],
        });

        let error = bind_cell_projection(
            &slice,
            CellProjectionProposal {
                segments: vec![CellPerspectiveSegment {
                    subject_id: "faction-06".into(),
                    narrative: "The institution watches the ferry clock.".into(),
                }],
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("member:mira"));
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

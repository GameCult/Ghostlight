use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Campaign {
    pub schema: String,
    pub id: Uuid,
    pub name: String,
    pub revision: u64,
    pub branch_origin: BranchOrigin,
    pub world_time: DateTime<Utc>,
    pub tick_hours: u32,
    pub player_actor_id: String,
    pub locations: BTreeMap<String, Location>,
    pub actors: BTreeMap<String, ActorState>,
    pub institutions: BTreeMap<String, InstitutionState>,
    pub clocks: BTreeMap<String, WorldClock>,
    pub facts: BTreeMap<String, WorldFact>,
    pub transcript: Vec<NarrativeTurn>,
    pub last_player_activity: DateTime<Utc>,
    pub pending_ticks: u8,
    #[serde(default)]
    pub away_ticks_processed: u8,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub news: Vec<NewsIssue>,
    #[serde(default)]
    pub canon_candidates: BTreeMap<String, CanonCandidate>,
    #[serde(default)]
    pub gestalts: BTreeMap<String, GestaltPersonaState>,
    #[serde(default)]
    pub gestalt_members: BTreeMap<String, GestaltMemberDelta>,
    #[serde(default)]
    pub pending_world_proposals: Vec<WorldActionProposal>,
    #[serde(default)]
    pub agency_profiles: BTreeMap<String, AgencyProfile>,
    #[serde(default)]
    pub agency_relations: BTreeMap<String, AgencyRelation>,
    #[serde(default)]
    pub gestalt_lineages: BTreeMap<String, GestaltLineage>,
    #[serde(default)]
    pub resolution_policy: ResolutionPolicy,
    #[serde(default)]
    pub resolution_pins: BTreeMap<String, ResolutionPin>,
    #[serde(default)]
    pub resolution_cover: Option<ResolutionCover>,
    #[serde(default)]
    pub strategic_tick_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct BranchOrigin {
    pub canon_cutoff: String,
    pub evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VaultManifest {
    pub schema: String,
    pub provider: String,
    pub source_ids: BTreeSet<String>,
    pub authority_lanes: BTreeSet<String>,
    pub temporal_scopes: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Location {
    pub id: String,
    pub name: String,
    pub container_id: Option<String>,
    pub routes: BTreeMap<String, Route>,
    pub persistent_features: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Route {
    pub destination_id: String,
    pub distance: String,
    #[schemars(range(min = 1))]
    pub travel_minutes: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ActorState {
    pub id: String,
    pub name: String,
    pub location_id: String,
    pub capabilities: BTreeSet<String>,
    pub knowledge: BTreeSet<String>,
    pub equipment: BTreeSet<String>,
    pub conditions: BTreeSet<String>,
    pub obligations: BTreeSet<String>,
    pub relationships: BTreeMap<String, String>,
    pub goals: Vec<String>,
    #[serde(default)]
    pub memories: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RelationshipState {
    pub schema: String,
    pub actor_id: String,
    pub other_actor_id: String,
    pub description: String,
    pub source_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltPersonaState {
    pub schema: String,
    pub id: String,
    pub name: String,
    pub version: u64,
    pub home_location_id: String,
    pub shared_capabilities: BTreeSet<String>,
    pub shared_knowledge: BTreeSet<String>,
    pub resources: BTreeSet<String>,
    pub goals: Vec<String>,
    pub pressures: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltMemberDelta {
    pub schema: String,
    pub id: String,
    pub gestalt_id: String,
    pub version: u64,
    pub name: String,
    pub capability_additions: BTreeSet<String>,
    pub capability_removals: BTreeSet<String>,
    pub knowledge_additions: BTreeSet<String>,
    pub knowledge_removals: BTreeSet<String>,
    pub equipment: BTreeSet<String>,
    pub conditions: BTreeSet<String>,
    pub obligations: BTreeSet<String>,
    pub relationships: BTreeMap<String, String>,
    pub goals: Vec<String>,
    pub memories: Vec<String>,
    pub last_location_id: Option<String>,
    pub materialized_actor_id: Option<String>,
    #[serde(default)]
    pub last_relevant_revision: u64,
    #[serde(default)]
    pub relevance_lease_until_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct GestaltAggregateDelta {
    pub knowledge_additions: BTreeSet<String>,
    pub resource_additions: BTreeSet<String>,
    pub pressures: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltPromotion {
    pub gestalt_id: String,
    pub expected_gestalt_version: u64,
    pub member_id: String,
    pub expected_member_version: u64,
    pub location_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltIndividuation {
    pub gestalt_id: String,
    pub expected_gestalt_version: u64,
    pub member: GestaltMemberDelta,
    pub location_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltDemotion {
    pub actor_id: String,
    #[serde(default)]
    pub aggregate_delta: GestaltAggregateDelta,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct GestaltPresencePlan {
    #[serde(default)]
    pub individuations: Vec<GestaltIndividuation>,
    pub promotions: Vec<GestaltPromotion>,
    pub demotions: Vec<GestaltDemotion>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltMaterializationReceipt {
    pub schema: String,
    pub campaign_id: Uuid,
    pub previous_revision: u64,
    pub revision: u64,
    pub reason: String,
    pub changes: Vec<GestaltPresenceChange>,
    pub committed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltPresenceChange {
    pub operation: String,
    pub gestalt_id: String,
    pub member_id: String,
    pub actor_id: String,
    pub gestalt_version: u64,
    pub member_version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AgencySubjectKind {
    Actor,
    Institution,
    Gestalt,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AgencyAxis {
    Geography,
    Ideology,
    Authority,
    EconomyRole,
    SpeciesBody,
    Information,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgencyProfile {
    pub schema: String,
    pub id: String,
    pub subject_id: String,
    pub subject_kind: AgencySubjectKind,
    pub profile_version: u64,
    pub collective_authority_id: Option<String>,
    pub parent_subject_id: Option<String>,
    pub active_leaf: bool,
    #[serde(default = "default_true")]
    pub simulation_eligible: bool,
    pub facets: BTreeMap<AgencyAxis, BTreeSet<String>>,
    pub location_ids: BTreeSet<String>,
    pub information_channels: BTreeSet<String>,
    pub detail_debt: u64,
    pub last_detail_tick: u64,
    pub evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgencyRelationKind {
    Containment,
    Command,
    Membership,
    Alliance,
    Rivalry,
    Trade,
    Migration,
    Communication,
    Coercion,
    SharedLocation,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgencyRelation {
    pub schema: String,
    pub id: String,
    pub from_subject_id: String,
    pub to_subject_id: String,
    pub kind: AgencyRelationKind,
    #[schemars(range(min = 1, max = 100))]
    pub strength: u8,
    pub active: bool,
    pub evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltLineage {
    pub schema: String,
    pub parent_gestalt_id: String,
    pub child_gestalt_ids: Vec<String>,
    pub partition_axis: AgencyAxis,
    pub partition_values: BTreeMap<String, String>,
    pub residual_child_id: String,
    pub source_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ResolutionPolicy {
    pub schema: String,
    pub resolution_epoch: u64,
    #[serde(default)]
    pub provider_configuration_epoch: u64,
    pub active_cell_budget: u8,
    pub pending_active_cell_budget: Option<u8>,
    pub provider_parallelism: u8,
}

impl Default for ResolutionPolicy {
    fn default() -> Self {
        Self {
            schema: "ghostlight.resolution_policy.v1".into(),
            resolution_epoch: 0,
            provider_configuration_epoch: 0,
            active_cell_budget: 8,
            pending_active_cell_budget: None,
            provider_parallelism: 8,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionPinKind {
    KeepTogether,
    KeepSeparate,
    MinimumIndividualDetail,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ResolutionPin {
    pub schema: String,
    pub id: String,
    pub kind: ResolutionPinKind,
    pub subject_ids: BTreeSet<String>,
    pub reason: String,
    pub created_world_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ResolutionDemand {
    pub schema: String,
    pub campaign_id: Uuid,
    pub world_revision: u64,
    pub resolution_epoch: u64,
    pub axis_weights: BTreeMap<AgencyAxis, f32>,
    pub focal_subject_ids: BTreeSet<String>,
    pub horizon_minutes: u32,
    pub rationale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SimulationCellMode {
    Cohesive,
    Arena,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct MergeLoss {
    pub facet_divergence: f32,
    pub hidden_boundary_mass: f32,
    pub information_divergence: f32,
    pub spatial_divergence: f32,
    pub clock_obligation_divergence: f32,
    pub salience_burial: f32,
    pub partition_churn: f32,
    pub total: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SimulationCell {
    pub schema: String,
    pub id: String,
    pub mode: SimulationCellMode,
    pub subject_ids: BTreeSet<String>,
    pub merge_loss: MergeLoss,
    pub rationale: String,
    pub lease_until_world_revision: u64,
    pub lease_until_strategic_tick: u64,
    pub detail_focus_subject_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ResolutionCover {
    pub schema: String,
    pub campaign_id: Uuid,
    pub world_revision: u64,
    pub resolution_epoch: u64,
    pub configured_budget: u8,
    pub effective_budget: u8,
    pub mandatory_overage: u8,
    pub cells: Vec<SimulationCell>,
    pub demand: ResolutionDemand,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ResolutionPlanReceipt {
    pub schema: String,
    pub campaign_id: Uuid,
    pub world_revision: u64,
    pub resolution_epoch: u64,
    pub configured_budget: u8,
    pub effective_budget: u8,
    pub cell_ids: Vec<String>,
    pub mandatory_overage: u8,
    pub preserved_cell_ids: Vec<String>,
    pub collapsed_boundaries: Vec<String>,
    pub merge_losses: BTreeMap<String, MergeLoss>,
    pub rationale: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ResolutionControlReceipt {
    pub schema: String,
    pub campaign_id: Uuid,
    pub world_revision: u64,
    pub previous_resolution_epoch: u64,
    pub resolution_epoch: u64,
    #[serde(default)]
    pub provider_configuration_epoch: u64,
    pub operation: String,
    pub active_cell_budget: u8,
    pub pin_ids: Vec<String>,
    pub committed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StrategicCellEffect {
    Institution {
        institution_id: String,
        posture: String,
        location_ids: Vec<String>,
    },
    Gestalt {
        gestalt_id: String,
        pressure_additions: Vec<String>,
    },
    ActorMove {
        actor_id: String,
        destination_id: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CellActionProposal {
    pub subject_id: String,
    pub intent: String,
    pub intended_effect: String,
    pub priority: i16,
    pub state_references: Vec<String>,
    pub public_channels: Vec<String>,
    pub effect: StrategicCellEffect,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CellAppraisal {
    pub schema: String,
    pub cell_id: String,
    pub world_revision: u64,
    pub resolution_epoch: u64,
    pub considered_subject_ids: BTreeSet<String>,
    pub actions: Vec<CellActionProposal>,
    pub inaction_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ResolutionWaveCommit {
    pub schema: String,
    pub world_revision: u64,
    pub resolution_epoch: u64,
    pub cover: ResolutionCover,
    pub plan_receipt: ResolutionPlanReceipt,
    pub appraisals: Vec<CellAppraisal>,
    pub model_receipt_hashes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltFissionPreview {
    pub schema: String,
    pub campaign_id: Uuid,
    pub expected_world_revision: u64,
    pub parent_gestalt_id: String,
    pub partition_axis: AgencyAxis,
    pub children: Vec<GestaltPersonaState>,
    pub child_partition_values: BTreeMap<String, String>,
    pub residual_child_id: String,
    #[serde(default)]
    pub member_child_assignments: BTreeMap<String, String>,
    pub evidence_receipt_ids: Vec<String>,
    pub gaps: Vec<String>,
    #[serde(default)]
    pub canon_candidates: Vec<CanonCandidate>,
    pub requires_approval: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct ActorStateDelta {
    pub memories_add: Vec<String>,
    pub conditions_add: BTreeSet<String>,
    pub conditions_remove: BTreeSet<String>,
    pub goals_add: Vec<String>,
    pub relationship_updates: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldActionProposal {
    pub actor_id: String,
    pub intent: String,
    pub intended_effect: String,
    pub priority: i16,
    pub state_references: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ActorReaction {
    pub actor_id: String,
    pub speech: Option<String>,
    pub private_delta: ActorStateDelta,
    pub action_proposals: Vec<WorldActionProposal>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct InstitutionState {
    pub id: String,
    pub name: String,
    pub resources: Vec<String>,
    pub goals: Vec<String>,
    pub posture: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldClock {
    pub id: String,
    pub label: String,
    pub progress: u8,
    #[schemars(range(min = 1))]
    pub threshold: u8,
    pub consequence: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldFact {
    pub id: String,
    pub statement: String,
    pub scope: FactScope,
    pub evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Event {
    pub id: String,
    pub at: DateTime<Utc>,
    pub kind: String,
    pub summary: String,
    pub actor_ids: Vec<String>,
    pub institution_ids: Vec<String>,
    pub location_ids: Vec<String>,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct NewsIssue {
    pub id: String,
    pub at: DateTime<Utc>,
    pub channel: String,
    pub headline: String,
    pub event_ids: Vec<String>,
    pub reliability: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct StrategicTickPlan {
    pub institution_actions: Vec<StrategicInstitutionAction>,
    pub gestalt_actions: Vec<StrategicGestaltAction>,
    pub actor_moves: Vec<StrategicActorMove>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicInstitutionAction {
    pub institution_id: String,
    pub posture: String,
    pub summary: String,
    pub location_ids: Vec<String>,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicGestaltAction {
    pub gestalt_id: String,
    pub summary: String,
    pub pressure_additions: Vec<String>,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicActorMove {
    pub actor_id: String,
    pub destination_id: String,
    pub summary: String,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicTickReceipt {
    pub schema: String,
    pub campaign_id: Uuid,
    pub previous_revision: u64,
    pub revision: u64,
    pub source: TickSource,
    pub model_receipt_hash: Option<String>,
    #[serde(default)]
    pub model_receipt_hashes: Vec<String>,
    #[serde(default)]
    pub resolution_epoch: Option<u64>,
    #[serde(default)]
    pub resolution_cover_id: Option<String>,
    pub event_ids: Vec<String>,
    pub committed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CanonCandidate {
    pub schema: String,
    pub id: String,
    pub originating_campaign_id: Uuid,
    pub gap: String,
    pub evidence_receipt_ids: Vec<String>,
    pub conflicts: Vec<String>,
    pub proposed_wording: String,
    pub affected_vault_sources: Vec<String>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FactScope {
    CanonBaseline,
    BranchLocal,
    ProvisionalLocal,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct NarrativeTurn {
    pub revision: u64,
    pub at: DateTime<Utc>,
    pub speaker: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct NarrationProjection {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub source_revision: u64,
    pub text: String,
    pub event_ids: Vec<String>,
    pub model_receipt_hash: String,
    pub published_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CampaignLifecycleReceipt {
    pub schema: String,
    pub campaign_id: Uuid,
    pub operation: String,
    pub parent_campaign_id: Option<Uuid>,
    pub parent_revision: Option<u64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RejectedProposalReceipt {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub revision: u64,
    pub command_kind: String,
    pub reason: String,
    pub rejected_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ActionIntent {
    pub actor_id: String,
    pub description: String,
    pub intended_effect: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorldCommand {
    CreateCampaign {
        campaign: Campaign,
        evidence_receipts: Vec<VaultEvidenceReceipt>,
        #[serde(default)]
        model_stage_receipts: Vec<crate::model::ModelStageReceipt>,
    },
    Speak {
        expected_revision: u64,
        actor_id: String,
        text: String,
        intended_effect: Option<String>,
    },
    Assess {
        expected_revision: u64,
        intent: ActionIntent,
        #[serde(default)]
        proposal: Option<ActionAssessment>,
    },
    Attempt {
        assessment_digest: String,
    },
    Wait {
        expected_revision: u64,
        minutes: u32,
    },
    AdvanceStrategicTick {
        expected_revision: u64,
        source: TickSource,
        #[serde(default)]
        plan: Option<StrategicTickPlan>,
        #[serde(default)]
        model_receipt_hash: Option<String>,
        #[serde(default)]
        resolution_wave: Option<ResolutionWaveCommit>,
    },
    SetResolutionBudget {
        expected_revision: u64,
        expected_resolution_epoch: u64,
        active_cell_budget: u8,
    },
    SetProviderParallelism {
        expected_revision: u64,
        expected_provider_configuration_epoch: u64,
        provider_parallelism: u8,
    },
    ReplaceResolutionPins {
        expected_revision: u64,
        expected_resolution_epoch: u64,
        pins: Vec<ResolutionPin>,
    },
    FissionGestalt {
        expected_revision: u64,
        preview: GestaltFissionPreview,
        evidence_receipts: Vec<VaultEvidenceReceipt>,
        #[serde(default)]
        model_stage_receipts: Vec<crate::model::ModelStageReceipt>,
    },
    ExpandRegion {
        expected_revision: u64,
        expansion: RegionExpansion,
        evidence_receipts: Vec<VaultEvidenceReceipt>,
        canon_candidates: Vec<CanonCandidate>,
        #[serde(default)]
        model_stage_receipts: Vec<crate::model::ModelStageReceipt>,
    },
    MaterializeGestaltMember {
        expected_revision: u64,
        gestalt_id: String,
        expected_gestalt_version: u64,
        member_id: String,
        expected_member_version: u64,
        location_id: String,
    },
    IndividuateGestaltMember {
        expected_revision: u64,
        individuation: GestaltIndividuation,
    },
    DematerializeGestaltMember {
        expected_revision: u64,
        actor_id: String,
        aggregate_delta: GestaltAggregateDelta,
    },
    ReconcileGestaltPresence {
        expected_revision: u64,
        reason: String,
        plan: GestaltPresencePlan,
    },
    ResolveReactionWave {
        expected_revision: u64,
        event_summary: String,
        reactions: Vec<ActorReaction>,
    },
    ResolveNpcAction {
        expected_revision: u64,
        proposal: WorldActionProposal,
        assessment: ActionAssessment,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RegionExpansion {
    pub origin_location_id: String,
    pub locations: Vec<Location>,
    pub facts: Vec<WorldFact>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RegionExpansionPreview {
    pub schema: String,
    pub campaign_id: Uuid,
    pub expected_revision: u64,
    pub expansion: RegionExpansion,
    pub evidence_receipts: Vec<VaultEvidenceReceipt>,
    pub gaps: Vec<String>,
    pub canon_candidates: Vec<CanonCandidate>,
    pub requires_approval: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TickSource {
    Scheduler,
    ReturnCatchUp,
    PlayerWait,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ContextModifier {
    pub label: String,
    pub value: i8,
    pub references: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct ConditionDelta {
    pub add: BTreeSet<String>,
    pub remove: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct WorldEffectDelta {
    pub actor_conditions: BTreeMap<String, ConditionDelta>,
    /// Exact player-readable natural-language findings learned by each actor.
    /// Values are declarative statements, never fact IDs, labels, or keys.
    #[serde(default)]
    pub actor_knowledge_additions: BTreeMap<String, BTreeSet<String>>,
    pub actor_relationship_updates: BTreeMap<String, BTreeMap<String, String>>,
    pub actor_moves: BTreeMap<String, String>,
    pub clock_advances: BTreeMap<String, u8>,
    pub institution_postures: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ActionAssessment {
    pub schema: String,
    pub campaign_id: Uuid,
    pub revision: u64,
    pub intent: ActionIntent,
    pub admissible: bool,
    pub missing_permission: Option<String>,
    pub dc: u8,
    pub modifiers: Vec<ContextModifier>,
    pub modifier_total: i8,
    pub effect_ceiling: String,
    pub success_stake: String,
    pub mixed_stake: String,
    pub failure_stake: String,
    #[serde(default)]
    pub strong_effect: WorldEffectDelta,
    #[serde(default)]
    pub success_effect: WorldEffectDelta,
    #[serde(default)]
    pub mixed_effect: WorldEffectDelta,
    #[serde(default)]
    pub failure_effect: WorldEffectDelta,
    pub bargains: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeBand {
    Failure,
    Mixed,
    Success,
    StrongSuccess,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RollReceipt {
    pub schema: String,
    pub assessment_digest: String,
    pub d20: u8,
    pub modifier_total: i8,
    pub total: i16,
    pub dc: u8,
    pub outcome: OutcomeBand,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldCommitReceipt {
    pub schema: String,
    pub campaign_id: Uuid,
    pub previous_revision: u64,
    pub revision: u64,
    pub command_kind: String,
    pub committed_at: DateTime<Utc>,
    pub roll: Option<RollReceipt>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VaultEvidenceReceipt {
    pub schema: String,
    pub id: String,
    pub provider: String,
    pub query_hash: String,
    pub witnesses: Vec<SourceWitness>,
    pub retrieved_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SourceWitness {
    pub source_id: String,
    pub exact_locator: String,
    pub content_hash: String,
    pub excerpt: String,
    pub authority_lane: String,
    pub temporal_scope: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldCompilePreview {
    pub schema: String,
    pub title: String,
    pub campaign: Campaign,
    pub evidence_receipts: Vec<VaultEvidenceReceipt>,
    #[serde(default)]
    pub evidence_coverage: Vec<EvidenceCoverage>,
    pub gaps: Vec<String>,
    pub branch_assumptions: Vec<String>,
    pub requires_approval: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceUseLane {
    DirectSeed,
    SettingBackground,
    Excluded,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct EvidenceCoverage {
    pub source_id: String,
    pub lane: EvidenceUseLane,
    pub rationale: String,
}

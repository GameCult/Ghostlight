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
    #[serde(default)]
    pub civic_systems: BTreeMap<String, CivicSystemManifest>,
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
    /// Durable record of exact causal attention windows already served by
    /// Nemesis. WorldKernel is the sole writer; the scheduler reads it only to
    /// avoid repeatedly assigning the same anchor to the same responder.
    #[serde(default)]
    pub nemesis_attention_history: Vec<NemesisAttentionRecord>,
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
    #[schemars(
        description = "Geometric containment only. This does not create an implicit movement edge."
    )]
    pub container_id: Option<String>,
    #[schemars(
        description = "Explicit directed movement edges keyed by stable route ID. Containment never substitutes for a route; a playable opening must provide route chains from the player location to every supplied location and back."
    )]
    pub routes: BTreeMap<String, Route>,
    pub persistent_features: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Route {
    #[schemars(
        description = "Exact ID of another supplied location reached by this directed route."
    )]
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
    #[serde(alias = "other_actor_id")]
    pub other_subject_id: String,
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

/// Gestalt member records own a local identity. The `member:` namespace is a
/// world-subject projection and must never become part of that stored local ID.
/// Repeated prefixes are accepted only at model and legacy-state boundaries so
/// every downstream organ converges on one canonical person.
pub fn canonical_gestalt_member_local_id(value: &str) -> String {
    let mut value = value.trim();
    while let Some(unprefixed) = value.strip_prefix("member:") {
        value = unprefixed;
    }
    value.to_owned()
}

pub fn gestalt_member_subject_id(member_id: &str) -> String {
    format!("member:{}", canonical_gestalt_member_local_id(member_id))
}

/// State-reference namespaces wrap a canonical world ID exactly once. World
/// compilers may choose already-qualified IDs (for example
/// `gestalt:raincross_households`), while older fixtures use local IDs. Both
/// must project to the same evidence handle rather than accumulating prefixes.
pub fn gestalt_state_reference(gestalt_id: &str) -> String {
    let gestalt_id = gestalt_id.trim();
    if gestalt_id.starts_with("gestalt:") {
        gestalt_id.to_owned()
    } else {
        format!("gestalt:{gestalt_id}")
    }
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
pub struct StrategicGestaltIndividuation {
    pub schema: String,
    /// Digest of the selected Gestalt-owned action whose pressure made this
    /// individual strategically consequential.
    pub action_digest: String,
    pub rationale: String,
    pub individuation: GestaltIndividuation,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltDemotion {
    pub actor_id: String,
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
    #[serde(default)]
    pub previous_resolution_epoch: u64,
    #[serde(default)]
    pub resolution_epoch: u64,
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

impl AgencyRelation {
    pub const SCHEMA: &'static str = "ghostlight.agency_relation.v1";
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
    /// Scheduler-owned reaction windows for already committed world pressure.
    /// These bindings decide who must receive a decision slot, never what that
    /// subject decides or which consequence the kernel commits.
    #[serde(default)]
    pub causal_follow_through: Vec<CausalFollowThroughAssignment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CausalFollowThroughAssignment {
    /// Exact opaque handle from the scheduler's frozen anchor catalog.
    pub anchor_reference: String,
    /// One exact autonomous actor, institution, population, or dormant member
    /// that owns a decision window for this anchor in the current cover.
    pub responder_subject_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct NemesisAttentionRecord {
    pub anchor_reference: String,
    pub responder_subject_id: String,
    pub served_world_revision: u64,
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
        #[serde(default)]
        pressure_additions: Vec<String>,
        #[serde(default)]
        pressure_resolutions: Vec<String>,
    },
    GestaltActivity {
        gestalt_id: String,
        activity: StrategicActivityKind,
        #[serde(default)]
        target_subject_ids: Vec<String>,
        #[serde(default)]
        location_ids: Vec<String>,
    },
    GestaltMigration {
        destination_gestalt_id: String,
    },
    ActorMove {
        actor_id: String,
        destination_id: String,
    },
    ActorActivity {
        actor_id: String,
        activity: StrategicActivityKind,
        #[serde(default)]
        target_subject_ids: Vec<String>,
        #[serde(default)]
        location_ids: Vec<String>,
    },
    MemberActivity {
        member_id: String,
        activity: StrategicActivityKind,
        #[serde(default)]
        target_subject_ids: Vec<String>,
        #[serde(default)]
        location_ids: Vec<String>,
    },
    MemberMigration {
        destination_gestalt_id: String,
    },
}

impl StrategicCellEffect {
    pub(crate) fn lane(&self) -> &'static str {
        match self {
            Self::Institution { .. } => "institution",
            Self::Gestalt { .. } => "gestalt_pressure",
            Self::GestaltActivity { .. } => "gestalt_activity",
            Self::GestaltMigration { .. } => "gestalt_migration",
            Self::ActorMove { .. } => "actor_move",
            Self::ActorActivity { .. } => "actor_activity",
            Self::MemberActivity { .. } => "member_activity",
            Self::MemberMigration { .. } => "member_migration",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrategicActivityKind {
    Prepare,
    Coordinate,
    Investigate,
    Recruit,
    Obstruct,
    Trade,
    Communicate,
}

impl StrategicActivityKind {
    pub fn allows_targetless_local_attempt(&self) -> bool {
        matches!(
            self,
            Self::Prepare | Self::Investigate | Self::Obstruct | Self::Communicate
        )
    }

    pub fn requires_explicit_target_for_gestalt(&self) -> bool {
        !self.allows_targetless_local_attempt() && !matches!(self, Self::Coordinate)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CellActionProposal {
    pub subject_id: String,
    #[schemars(length(min = 1, max = 460))]
    pub intent: String,
    #[schemars(length(min = 1, max = 460))]
    pub intended_effect: String,
    pub priority: i16,
    pub state_references: Vec<String>,
    pub public_channels: Vec<String>,
    #[serde(default)]
    #[schemars(length(min = 1, max = 4))]
    pub effects: Vec<StrategicCellEffect>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CellInaction {
    pub subject_id: String,
    #[schemars(length(min = 1, max = 240))]
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CellAppraisal {
    pub schema: String,
    pub cell_id: String,
    pub world_revision: u64,
    pub resolution_epoch: u64,
    /// Exact projected decision owners for this wave. The resolution cover
    /// separately proves representation of every canonical cell constituent;
    /// each ID here must own exactly one action or explicit inaction.
    pub considered_subject_ids: BTreeSet<String>,
    pub actions: Vec<CellActionProposal>,
    pub inactions: Vec<CellInaction>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ResolutionWaveCommit {
    pub schema: String,
    pub world_revision: u64,
    pub resolution_epoch: u64,
    pub cover: ResolutionCover,
    pub plan_receipt: ResolutionPlanReceipt,
    pub appraisals: Vec<CellAppraisal>,
    #[serde(default)]
    pub activity_outcomes: Vec<StrategicActivityOutcome>,
    #[serde(default)]
    pub strategic_individuations: Vec<StrategicGestaltIndividuation>,
    pub model_receipt_hashes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrategicOutcomeBand {
    Success,
    Mixed,
    Failure,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicActivityOutcome {
    pub schema: String,
    pub action_digest: String,
    pub source_subject_id: String,
    pub band: StrategicOutcomeBand,
    #[schemars(length(min = 1, max = 240))]
    pub summary: String,
    #[serde(default)]
    pub supporting_state_references: Vec<String>,
    pub effect: StrategicOutcomeEffect,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StrategicOutcomeEffect {
    NoMaterialChange {
        reason: String,
    },
    ResourceCreated {
        owner_subject_id: String,
        resource: String,
    },
    ResourceConsumed {
        owner_subject_id: String,
        resource: String,
    },
    ResourceTransferred {
        from_subject_id: String,
        to_subject_id: String,
        resource: String,
    },
    GestaltPressure {
        gestalt_id: String,
        #[serde(default)]
        pressure_additions: Vec<String>,
        #[serde(default)]
        pressure_resolutions: Vec<String>,
    },
    AgencyRelationShift {
        relation_id: String,
        strength_delta: i16,
    },
    MemberMemory {
        member_id: String,
        memory: String,
    },
    MemberObligation {
        member_id: String,
        obligation: String,
    },
    MemberRelationship {
        member_id: String,
        other_subject_id: String,
        description: String,
    },
    KnowledgeLearned {
        owner_subject_id: String,
        fact_id: String,
    },
    KnowledgeCommunicated {
        from_subject_id: String,
        #[schemars(length(min = 1, max = 4))]
        to_subject_ids: Vec<String>,
        fact_id: String,
    },
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
    /// Exact scarce-resource custody after the split. Every resource owned by
    /// the parent appears once and names one child; it is never inherited by
    /// every child as population baseline.
    #[serde(default)]
    pub resource_child_assignments: BTreeMap<String, String>,
    pub evidence_receipt_ids: Vec<String>,
    pub gaps: Vec<String>,
    #[serde(default)]
    pub canon_candidates: Vec<CanonCandidate>,
    pub requires_approval: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct ActorStateDelta {
    pub memories_add: Vec<String>,
    /// One public self-identifier explicitly adopted in the actor's own
    /// speech. WorldKernel binds the subject and derives the identity handle;
    /// the model never supplies either authority value.
    #[serde(default)]
    pub identity_adoption: Option<String>,
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
    /// A typed, actor-owned refusal to answer. The kernel lowers this to a
    /// deterministic visible transcript turn, so an Interpreter cannot smuggle
    /// an unassessed physical effect through free-form reaction prose.
    #[serde(default)]
    pub deliberate_silence: bool,
    pub private_delta: ActorStateDelta,
    pub action_proposals: Vec<WorldActionProposal>,
}

/// A foreground appraisal owned by one cohesive population subject. Unlike an
/// arena cell, a Gestalt has genuine collective authority and may speak in the
/// plural. Its foreground reaction cannot mutate actor-private state or smuggle
/// an unassessed strategic action through dialogue.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltReaction {
    pub gestalt_id: String,
    pub speech: Option<String>,
    #[serde(default)]
    pub deliberate_silence: bool,
}

pub const MAX_POSTURE_CHARS: usize = 460;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct InstitutionState {
    pub id: String,
    pub name: String,
    pub resources: Vec<String>,
    pub goals: Vec<String>,
    pub posture: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default)]
#[schemars(inline)]
pub struct WorldEventScope {
    pub actor_ids: Vec<String>,
    pub institution_ids: Vec<String>,
    pub gestalt_ids: Vec<String>,
    pub location_ids: Vec<String>,
    pub public_channels: Vec<String>,
}

impl Default for WorldEventScope {
    fn default() -> Self {
        Self {
            actor_ids: Vec::new(),
            institution_ids: Vec::new(),
            gestalt_ids: Vec::new(),
            location_ids: Vec::new(),
            public_channels: Vec::new(),
        }
    }
}

impl WorldEventScope {
    pub fn is_unbound(&self) -> bool {
        self.actor_ids.is_empty()
            && self.institution_ids.is_empty()
            && self.gestalt_ids.is_empty()
            && self.location_ids.is_empty()
            && self.public_channels.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldClock {
    pub id: String,
    pub label: String,
    pub progress: u8,
    #[schemars(range(min = 1))]
    pub threshold: u8,
    pub consequence: String,
    /// Exact subjects, places, and information routes affected when this clock
    /// first reaches its threshold. Legacy snapshots may be unbound until an
    /// admitted reconciliation pass supplies this scope.
    #[serde(default)]
    pub consequence_scope: WorldEventScope,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldFact {
    pub id: String,
    pub statement: String,
    pub scope: FactScope,
    pub evidence_receipt_ids: Vec<String>,
    /// Locations whose immediate environment can expose this fact through an
    /// admitted informational attempt. Empty means the fact is not directly
    /// discoverable from occupancy alone.
    #[serde(default)]
    pub discoverable_at_location_ids: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Event {
    pub id: String,
    pub at: DateTime<Utc>,
    pub kind: String,
    pub summary: String,
    pub actor_ids: Vec<String>,
    pub institution_ids: Vec<String>,
    #[serde(default)]
    pub gestalt_ids: Vec<String>,
    pub location_ids: Vec<String>,
    pub public_channels: Vec<String>,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum PublicEventAssertionStatus {
    AttemptCommittedOutcomeUnknown,
    CourseCommittedEmbeddedActionsNotCompleted,
    PublicDeclaration,
    MaterialChangeCommitted,
    PublicAccountStatusUnspecified,
}

impl Event {
    pub fn public_assertion_status(&self) -> PublicEventAssertionStatus {
        match self.kind.as_str() {
            "actor_activity" | "gestalt_activity" | "gestalt_member_activity" => {
                PublicEventAssertionStatus::AttemptCommittedOutcomeUnknown
            }
            "institution_action" => {
                PublicEventAssertionStatus::CourseCommittedEmbeddedActionsNotCompleted
            }
            "gestalt_action" | "public_notice" => PublicEventAssertionStatus::PublicDeclaration,
            "strategic_activity_outcome"
            | "actor_movement"
            | "gestalt_migration"
            | "gestalt_member_migration"
            | "clock_consequence"
            | "group_travel" => PublicEventAssertionStatus::MaterialChangeCommitted,
            _ => PublicEventAssertionStatus::PublicAccountStatusUnspecified,
        }
    }
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

pub const MAX_PUBLIC_EVENT_SUMMARY_CHARS: usize = 240;

pub fn committed_news_headline(summary: &str) -> String {
    const MAX_HEADLINE_CHARS: usize = 96;
    let summary = summary.trim();
    if summary.chars().count() <= MAX_HEADLINE_CHARS {
        return summary.to_owned();
    }
    let mut headline = summary.chars().take(MAX_HEADLINE_CHARS).collect::<String>();
    let semantic_cut = headline
        .rfind(|character: char| matches!(character, ';' | ':' | ',' | '—'))
        .filter(|cut| *cut >= MAX_HEADLINE_CHARS / 2);
    let word_cut = headline
        .rfind(char::is_whitespace)
        .filter(|cut| *cut >= MAX_HEADLINE_CHARS / 2);
    if let Some(cut) = semantic_cut.or(word_cut) {
        headline.truncate(cut);
    }
    headline = headline
        .trim_end_matches(|character: char| character.is_whitespace() || character == '.')
        .to_owned();
    headline.push('…');
    headline
}

pub fn append_event_with_publications(campaign: &mut Campaign, event: Event) {
    for channel in &event.public_channels {
        campaign.news.push(NewsIssue {
            id: event_publication_id(&event.id, channel),
            at: event.at,
            channel: channel.clone(),
            headline: committed_news_headline(&event.summary),
            event_ids: vec![event.id.clone()],
            reliability: "committed public channel".into(),
        });
    }
    campaign.events.push(event);
}

pub fn event_publication_id(event_id: &str, channel: &str) -> String {
    use sha2::{Digest, Sha256};

    let channel_id = format!("{:x}", Sha256::digest(channel.as_bytes()))[..12].to_owned();
    format!("news:{event_id}:{channel_id}")
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct StrategicTickPlan {
    /// Canonical admitted strategic choices. The action arrays below are
    /// deterministic projections used by the legacy event and transition
    /// lowering path; production callers rebuild them from this list.
    #[serde(default)]
    pub selected_actions: Vec<CellActionProposal>,
    pub institution_actions: Vec<StrategicInstitutionAction>,
    pub gestalt_actions: Vec<StrategicGestaltAction>,
    #[serde(default)]
    pub gestalt_activities: Vec<StrategicGestaltActivity>,
    #[serde(default)]
    pub gestalt_migrations: Vec<StrategicGestaltMigration>,
    pub actor_moves: Vec<StrategicActorMove>,
    #[serde(default)]
    pub actor_activities: Vec<StrategicActorActivity>,
    #[serde(default)]
    pub member_migrations: Vec<StrategicMemberMigration>,
    #[serde(default)]
    pub member_activities: Vec<StrategicMemberActivity>,
    #[serde(default)]
    pub activity_outcomes: Vec<StrategicActivityOutcome>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicInstitutionAction {
    pub institution_id: String,
    pub posture: String,
    pub location_ids: Vec<String>,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicGestaltAction {
    pub gestalt_id: String,
    pub pressure_additions: Vec<String>,
    pub pressure_resolutions: Vec<String>,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicGestaltActivity {
    pub action_digest: String,
    pub gestalt_id: String,
    pub activity: StrategicActivityKind,
    pub target_subject_ids: Vec<String>,
    pub location_ids: Vec<String>,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicGestaltMigration {
    pub gestalt_id: String,
    pub destination_gestalt_id: String,
    pub destination_location_id: String,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicActorMove {
    pub actor_id: String,
    pub destination_id: String,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicActorActivity {
    pub action_digest: String,
    pub actor_id: String,
    pub activity: StrategicActivityKind,
    pub target_subject_ids: Vec<String>,
    pub location_ids: Vec<String>,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicMemberMigration {
    pub member_id: String,
    pub source_gestalt_id: String,
    pub destination_gestalt_id: String,
    pub destination_location_id: String,
    pub public_channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StrategicMemberActivity {
    pub action_digest: String,
    pub member_id: String,
    pub source_gestalt_id: String,
    pub activity: StrategicActivityKind,
    pub target_subject_ids: Vec<String>,
    pub location_ids: Vec<String>,
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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
    /// Exact model-controlled Persona subject IDs whom this committed speech
    /// asks to respond. The legacy field name includes actors, folded named
    /// members, and cohesive Gestalts. Other present subjects still perceive
    /// and appraise the turn but are not response-bound. Human-controlled
    /// actors never appear here.
    #[serde(default)]
    pub persona_response_actor_ids: BTreeSet<String>,
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
    ApplyExternalSubjectSnapshot {
        snapshot: crate::consumer::ExternalSubjectSnapshot,
    },
    Speak {
        expected_revision: u64,
        actor_id: String,
        text: String,
        intended_effect: Option<String>,
        /// Filled by the server-side scene-address resolver from the exact
        /// present-actor catalog. Player boundaries must not accept authority
        /// IDs in this field.
        #[serde(default)]
        persona_response_actor_ids: BTreeSet<String>,
    },
    Assess {
        expected_revision: u64,
        intent: ActionIntent,
        #[serde(default)]
        proposal: Option<ActionAssessment>,
    },
    Attempt {
        actor_id: String,
        assessment_digest: String,
    },
    Wait {
        expected_revision: u64,
        minutes: u32,
    },
    ProposeTimeAdvance {
        expected_revision: u64,
        member_id: String,
        minutes: u32,
    },
    ApproveTimeAdvance {
        expected_revision: u64,
        proposal_id: String,
        member_id: String,
    },
    ProposeGroupTravel {
        expected_revision: u64,
        member_id: String,
        destination_location_id: String,
    },
    ApproveGroupTravel {
        expected_revision: u64,
        proposal_id: String,
        member_id: String,
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
    ProposeResolutionBudget {
        expected_revision: u64,
        expected_resolution_epoch: u64,
        member_id: String,
        active_cell_budget: u8,
    },
    ApproveResolutionBudget {
        expected_revision: u64,
        proposal_id: String,
        member_id: String,
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
    ReconcileFissionCivicBindings {
        expected_revision: u64,
    },
    ExpandRegion {
        expected_revision: u64,
        expansion: RegionExpansion,
        evidence_receipts: Vec<VaultEvidenceReceipt>,
        canon_candidates: Vec<CanonCandidate>,
        #[serde(default)]
        model_stage_receipts: Vec<crate::model::ModelStageReceipt>,
    },
    ElaborateLocality {
        expected_revision: u64,
        elaboration: LocalityElaboration,
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
        #[serde(default)]
        gestalt_reactions: Vec<GestaltReaction>,
    },
    ResolveNpcAction {
        expected_revision: u64,
        proposal: WorldActionProposal,
        assessment: ActionAssessment,
    },
    BindClockConsequences {
        expected_revision: u64,
        admission: crate::clock::ClockConsequenceBindingAdmission,
        model_stage_receipts: Vec<crate::model::ModelStageReceipt>,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RegionExpansion {
    pub origin_location_id: String,
    /// Exact outbound routes added to the already-materialized origin. Keeping
    /// these separate from `locations` makes the existing-place mutation
    /// explicit in the approval preview instead of deriving a hidden reverse
    /// route after commit.
    #[serde(default)]
    pub origin_routes: BTreeMap<String, Route>,
    pub locations: Vec<Location>,
    pub facts: Vec<WorldFact>,
    /// Optional population leaves that make an inhabited destination part of
    /// the agency graph. Destination compilation admits these subjects; it
    /// never moves an existing population or named member into them.
    #[serde(default)]
    pub populations: Vec<GestaltPersonaState>,
    /// Exact agency inputs for `populations`. Keeping these in the approved
    /// expansion makes the resolution cover reproducible without making the
    /// cover itself canonical.
    #[serde(default)]
    pub population_profiles: Vec<AgencyProfile>,
    /// Directed population routes admitted with the destination. A migration
    /// remains a later strategic choice validated against one of these exact
    /// relations and the physical topology.
    #[serde(default)]
    pub migration_relations: Vec<AgencyRelation>,
    /// Institutions admitted with the region or locality. Institutions remain
    /// distinct strategic subjects; population leaves may know public civic
    /// facts without inheriting an institution's goals, resources, or voice.
    #[serde(default)]
    pub institutions: Vec<InstitutionState>,
    /// Exact agency inputs for `institutions`.
    #[serde(default)]
    pub institution_profiles: Vec<AgencyProfile>,
    /// Political and administrative relations among newly admitted local
    /// institutions and populations. Migration remains in its separate lane.
    #[serde(default)]
    pub local_relations: Vec<AgencyRelation>,
    /// Structural receipt proving that an inhabited locality exposes enough
    /// committed public state for ordinary civic questions. The facts may say
    /// that no mayor or election exists; a player's presupposition never gets
    /// to choose the answer.
    #[serde(default)]
    pub civic_system: Option<CivicSystemManifest>,
}

#[derive(JsonSchema)]
#[allow(dead_code)]
pub(crate) enum CivicSystemManifestSchemaV1 {
    #[schemars(rename = "ghostlight.civic_system_manifest.v1")]
    V1,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CivicSystemManifest {
    #[schemars(with = "CivicSystemManifestSchemaV1")]
    pub schema: String,
    #[serde(default)]
    pub version: u64,
    pub jurisdiction_location_id: String,
    pub governing_institution_ids: BTreeSet<String>,
    pub resident_population_ids: BTreeSet<String>,
    pub public_authority_fact_ids: BTreeSet<String>,
    pub public_selection_fact_ids: BTreeSet<String>,
    pub public_resource_fact_ids: BTreeSet<String>,
    pub public_redress_fact_ids: BTreeSet<String>,
    pub political_relation_ids: BTreeSet<String>,
    /// Exact locally rebound verifier receipt that covers this complete civic
    /// candidate. The compiler supplies it after inference; models never own
    /// this binding.
    #[serde(default)]
    pub semantic_verification_receipt_id: String,
}

/// Admission of bounded child detail beneath one exact canonical coarse place.
/// The target identity is immutable; the nested expansion contains only new
/// subjects, facts, child places, and edges admitted beneath it.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct LocalityElaboration {
    pub target_location_id: String,
    pub expansion: RegionExpansion,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RegionExpansionPreview {
    pub schema: String,
    pub campaign_id: Uuid,
    pub expected_revision: u64,
    pub expansion: RegionExpansion,
    pub evidence_receipts: Vec<VaultEvidenceReceipt>,
    /// Consequential game-scale detail synthesized for this campaign because
    /// the Vault constrains the destination without exhaustively specifying a
    /// playable map. These assumptions are reviewable branch state, not canon
    /// gaps or canon-candidate proposals.
    #[serde(default)]
    pub branch_assumptions: Vec<String>,
    pub gaps: Vec<String>,
    pub canon_candidates: Vec<CanonCandidate>,
    pub requires_approval: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct LocalityElaborationPreview {
    pub schema: String,
    pub campaign_id: Uuid,
    pub expected_revision: u64,
    pub elaboration: LocalityElaboration,
    pub evidence_receipts: Vec<VaultEvidenceReceipt>,
    #[serde(default)]
    pub branch_assumptions: Vec<String>,
    pub gaps: Vec<String>,
    pub canon_candidates: Vec<CanonCandidate>,
    pub requires_approval: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", content = "preview", rename_all = "snake_case")]
pub enum DestinationCompilationPreview {
    RegionExpansion(RegionExpansionPreview),
    LocalityElaboration(LocalityElaborationPreview),
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
pub struct CommitmentDelta {
    pub goals_add: BTreeSet<String>,
    pub goals_retire: BTreeSet<String>,
    pub obligations_add: BTreeSet<String>,
    pub obligations_retire: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(default)]
pub struct WorldEffectDelta {
    pub actor_conditions: BTreeMap<String, ConditionDelta>,
    /// Bounded changes to goals or obligations owned by exact present actors.
    /// A social outcome may create a voluntary commitment; it may not transfer
    /// custody of the actor or guarantee behavior beyond that commitment.
    #[serde(default)]
    pub actor_commitments: BTreeMap<String, CommitmentDelta>,
    /// Exact player-readable natural-language findings learned by each actor.
    /// Values are declarative statements, never fact IDs, labels, or keys.
    #[serde(default)]
    pub actor_knowledge_additions: BTreeMap<String, BTreeSet<String>>,
    /// New branch-local propositions established by the acting actor's exact
    /// means in this outcome. These are observations, measurements, or test
    /// results—not pre-existing facts selected from the campaign catalog. Each
    /// value is the concrete proposition learned, never a report that an
    /// inquiry occurred or an unresolved placeholder for a later resolver.
    #[serde(default)]
    pub actor_observations: BTreeMap<String, BTreeSet<String>>,
    pub actor_relationship_updates: BTreeMap<String, BTreeMap<String, String>>,
    pub actor_moves: BTreeMap<String, String>,
    pub clock_advances: BTreeMap<String, u8>,
    /// Bounded progress removed from an existing world clock. This is the
    /// persistent inverse of an advance for repairs, relief, and obstruction.
    #[serde(default)]
    pub clock_reductions: BTreeMap<String, u8>,
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

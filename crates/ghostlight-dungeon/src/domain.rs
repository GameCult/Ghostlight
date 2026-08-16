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
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct BranchOrigin {
    pub canon_cutoff: String,
    pub evidence_receipt_ids: Vec<String>,
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
pub struct GestaltDemotion {
    pub actor_id: String,
    #[serde(default)]
    pub aggregate_delta: GestaltAggregateDelta,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct GestaltPresencePlan {
    pub promotions: Vec<GestaltPromotion>,
    pub demotions: Vec<GestaltDemotion>,
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
    },
    ExpandRegion {
        expected_revision: u64,
        expansion: RegionExpansion,
        evidence_receipts: Vec<VaultEvidenceReceipt>,
        canon_candidates: Vec<CanonCandidate>,
    },
    MaterializeGestaltMember {
        expected_revision: u64,
        gestalt_id: String,
        expected_gestalt_version: u64,
        member_id: String,
        expected_member_version: u64,
        location_id: String,
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
    BeginNpcAction {
        expected_revision: u64,
        proposal: WorldActionProposal,
    },
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
    pub gaps: Vec<String>,
    pub branch_assumptions: Vec<String>,
    pub requires_approval: bool,
}

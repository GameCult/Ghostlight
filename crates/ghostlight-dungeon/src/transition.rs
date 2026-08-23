use crate::domain::{OutcomeBand, StrategicOutcomeBand};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    Campaign,
    Actor,
    Population,
    Institution,
    Place,
    Resource,
    Pressure,
    Proposition,
    Channel,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct SubjectRef {
    pub kind: SubjectKind,
    pub id: String,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum WorldComponentKind {
    Identity,
    Occupancy,
    Custody,
    ResourceState,
    Capability,
    Condition,
    Knowledge,
    Memory,
    Relationship,
    Commitment,
    Pressure,
    Posture,
    PopulationMembership,
    PopulationLineage,
    Topology,
    Lifecycle,
    WorldTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldComponentRef {
    pub subject: SubjectRef,
    pub component: WorldComponentKind,
    pub entry_id: Option<String>,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ActionMeans {
    pub schema: String,
    pub actor: SubjectRef,
    #[schemars(length(min = 1, max = 4000))]
    pub description: String,
    #[serde(default)]
    pub targets: Vec<SubjectRef>,
    #[serde(default)]
    pub instruments: Vec<SubjectRef>,
    #[serde(default)]
    pub places: Vec<SubjectRef>,
    #[serde(default)]
    pub route_ids: BTreeSet<String>,
    #[serde(default)]
    pub channels: Vec<SubjectRef>,
    #[serde(default)]
    #[schemars(length(max = 4000))]
    pub speech: Option<String>,
    #[serde(default)]
    pub state_references: Vec<WorldComponentRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MutationIntent {
    pub schema: String,
    pub component: WorldComponentKind,
    #[serde(default)]
    pub targets: Vec<SubjectRef>,
    #[schemars(length(min = 1, max = 1000))]
    pub desired_change: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MutationProcedure {
    ForegroundAttempt,
    DirectCommand,
    NpcAttempt,
    ReactionAppraisal,
    StrategicOutcome,
    Governance,
    CompilerAdmission,
    Lifecycle,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum MutationOutcomeBinding {
    Foreground(OutcomeBand),
    Strategic(StrategicOutcomeBand),
    StrategicWave(Vec<StrategicOutcomeSourceBinding>),
    Deterministic,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct StrategicOutcomeSourceBinding {
    pub action_digest: String,
    pub band: StrategicOutcomeBand,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum WorldMutationOperation {
    Relocate,
    TransferCustody,
    ResourceCreate,
    ResourceTransform,
    ResourceConsume,
    ResourceDamage,
    ResourceRepair,
    ResourceSplit,
    ResourceCombine,
    CapabilityGrant,
    CapabilityAlter,
    CapabilitySuspend,
    CapabilityRetire,
    ConditionApply,
    ConditionAlter,
    ConditionClear,
    CommitmentCreate,
    CommitmentAlter,
    CommitmentFulfill,
    CommitmentDefault,
    CommitmentRetire,
    RelationshipCreate,
    RelationshipAlter,
    RelationshipRetire,
    PressureCreate,
    PressureAdvance,
    PressureReduce,
    PressureResolve,
    PressureRetire,
    KnowledgeAcquire,
    KnowledgeCommunicate,
    KnowledgeConceal,
    KnowledgeCorrect,
    KnowledgeInvalidate,
    MemoryRecord,
    MemoryRevise,
    MemoryRetire,
    PostureChange,
    PopulationJoin,
    PopulationLeave,
    PopulationTransfer,
    PopulationSplit,
    PopulationMerge,
    IdentityAdopt,
    IdentityDisclose,
    IdentityRestrict,
    IdentityRetire,
    TopologyAdd,
    TopologyAlter,
    TopologyOpen,
    TopologyClose,
    TopologyRetire,
    AdmitEntity,
    RetireEntity,
    AdvanceWorldTime,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum MutationSubjectRole {
    Subject,
    Actor,
    Owner,
    Counterparty,
    OriginPlace,
    DestinationPlace,
    Resource,
    RelatedResource,
    SourceCustodian,
    DestinationCustodian,
    Pressure,
    Knower,
    Speaker,
    Recipient,
    Proposition,
    Channel,
    SourcePopulation,
    DestinationPopulation,
    ParentPopulation,
    ChildPopulation,
    RelationshipSource,
    RelationshipTarget,
    TopologyOrigin,
    TopologyDestination,
    Entity,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum MutationStringRole {
    RouteId,
    ResourceKind,
    ResourceLabel,
    RecipeId,
    Description,
    Summary,
    RetirementReason,
    CapabilityId,
    ConditionId,
    CommitmentId,
    RelationshipId,
    MemoryId,
    EventId,
    PressureLabel,
    Posture,
    IdentityHandleId,
    IdentityHandleValue,
    TopologyEdgeId,
    AdmissionReceiptId,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum MutationIntegerRole {
    Quantity,
    Integrity,
    Severity,
    RelationshipStrengthDelta,
    PressureAmount,
    TravelMinutes,
    WorldMinutes,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct IntegerBounds {
    pub minimum: i64,
    pub maximum: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct StringConstraint {
    #[serde(default)]
    pub allowed_values: BTreeSet<String>,
    #[schemars(range(min = 1))]
    pub minimum_length: u16,
    #[schemars(range(min = 1))]
    pub maximum_length: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MutationSubjectBinding {
    pub role: MutationSubjectRole,
    pub allowed_subjects: BTreeSet<SubjectRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MutationPermit {
    pub id: String,
    pub operation: WorldMutationOperation,
    pub subject_bindings: Vec<MutationSubjectBinding>,
    #[serde(default)]
    pub string_constraints: BTreeMap<MutationStringRole, StringConstraint>,
    #[serde(default)]
    pub integer_bounds: BTreeMap<MutationIntegerRole, IntegerBounds>,
    #[schemars(range(min = 1))]
    pub maximum_uses: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MutationAuthorityEnvelope {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub world_revision: u64,
    pub resolution_epoch: Option<u64>,
    pub procedure: MutationProcedure,
    pub source_subject: Option<SubjectRef>,
    pub outcome: MutationOutcomeBinding,
    #[schemars(length(min = 1, max = 1000))]
    pub effect_ceiling: String,
    pub permits: Vec<MutationPermit>,
    #[serde(default)]
    pub authority_receipt_ids: BTreeSet<String>,
    pub expires_at: DateTime<Utc>,
    pub digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceMutationOperation {
    Create,
    Transform,
    Consume,
    Damage,
    Repair,
    Split,
    Combine,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityMutationOperation {
    Grant,
    Alter,
    Suspend,
    Retire,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionMutationOperation {
    Apply,
    Alter,
    Clear,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentKind {
    Goal,
    Obligation,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentMutationOperation {
    Create,
    Alter,
    Fulfill,
    Default,
    Retire,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipMutationOperation {
    Create,
    Alter,
    Retire,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PressureMutationOperation {
    Create,
    Advance,
    Reduce,
    Resolve,
    Retire,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeMutationOperation {
    Acquire,
    Communicate,
    Conceal,
    Correct,
    Invalidate,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryMutationOperation {
    Record,
    Revise,
    Retire,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PopulationMembershipOperation {
    Join,
    Leave,
    Transfer,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PopulationLineageOperation {
    Split,
    Merge,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityMutationOperation {
    Adopt,
    Disclose,
    Restrict,
    Retire,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TopologyMutationOperation {
    Add,
    Alter,
    Open,
    Close,
    Retire,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorldMutation {
    Relocate {
        subject: SubjectRef,
        from_place: SubjectRef,
        to_place: SubjectRef,
        route_id: String,
    },
    TransferCustody {
        resource: SubjectRef,
        from_custodian: SubjectRef,
        to_custodian: SubjectRef,
    },
    MutateResource {
        resource: SubjectRef,
        operation: ResourceMutationOperation,
        custodian: Option<SubjectRef>,
        #[serde(default)]
        related_resources: Vec<SubjectRef>,
        resource_kind: Option<String>,
        resource_label: Option<String>,
        recipe_id: Option<String>,
        quantity: Option<i64>,
        integrity: Option<i64>,
    },
    ChangeCapability {
        subject: SubjectRef,
        operation: CapabilityMutationOperation,
        capability_id: String,
        #[schemars(length(max = 1000))]
        description: Option<String>,
    },
    ChangeCondition {
        subject: SubjectRef,
        operation: ConditionMutationOperation,
        condition_id: String,
        #[schemars(length(max = 1000))]
        description: Option<String>,
        severity: Option<i64>,
    },
    ChangeCommitment {
        subject: SubjectRef,
        operation: CommitmentMutationOperation,
        kind: CommitmentKind,
        commitment_id: String,
        counterparty: Option<SubjectRef>,
        #[schemars(length(max = 1000))]
        description: Option<String>,
    },
    ChangeRelationship {
        source: SubjectRef,
        target: SubjectRef,
        operation: RelationshipMutationOperation,
        relationship_id: String,
        #[schemars(length(max = 1000))]
        description: Option<String>,
        strength_delta: Option<i64>,
    },
    ChangePressure {
        pressure: SubjectRef,
        owner: SubjectRef,
        operation: PressureMutationOperation,
        amount: Option<i64>,
        #[schemars(length(max = 240))]
        label: Option<String>,
    },
    ChangeKnowledge {
        operation: KnowledgeMutationOperation,
        proposition: SubjectRef,
        knower: Option<SubjectRef>,
        speaker: Option<SubjectRef>,
        #[serde(default)]
        recipients: Vec<SubjectRef>,
        channel: Option<SubjectRef>,
    },
    ChangeMemory {
        subject: SubjectRef,
        operation: MemoryMutationOperation,
        memory_id: String,
        event_id: Option<String>,
        #[schemars(length(max = 1000))]
        summary: Option<String>,
    },
    ChangePosture {
        subject: SubjectRef,
        posture: String,
    },
    ChangePopulationMembership {
        actor: SubjectRef,
        operation: PopulationMembershipOperation,
        source_population: Option<SubjectRef>,
        destination_population: Option<SubjectRef>,
    },
    ChangePopulationLineage {
        operation: PopulationLineageOperation,
        #[serde(default)]
        parent_populations: Vec<SubjectRef>,
        #[serde(default)]
        child_populations: Vec<SubjectRef>,
        remainder_population: Option<SubjectRef>,
    },
    ChangeIdentity {
        subject: SubjectRef,
        operation: IdentityMutationOperation,
        handle_id: String,
        handle_value: Option<String>,
        #[serde(default)]
        audience: Vec<SubjectRef>,
    },
    ChangeTopology {
        operation: TopologyMutationOperation,
        edge_id: String,
        from_place: SubjectRef,
        to_place: SubjectRef,
        travel_minutes: Option<i64>,
    },
    AdmitEntity {
        subject: SubjectRef,
        initial_components: BTreeSet<WorldComponentKind>,
        admission_receipt_id: String,
    },
    RetireEntity {
        subject: SubjectRef,
        #[schemars(length(min = 1, max = 1000))]
        reason: String,
    },
    AdvanceWorldTime {
        campaign: SubjectRef,
        minutes: i64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PermittedWorldMutation {
    pub permit_id: String,
    pub mutation: WorldMutation,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldMutationBatch {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub expected_world_revision: u64,
    pub expected_resolution_epoch: Option<u64>,
    pub authority_envelope_digest: String,
    pub source_receipt_id: String,
    pub means_digest: Option<String>,
    pub intended_effect_digest: Option<String>,
    #[serde(default)]
    pub mutations: Vec<PermittedWorldMutation>,
    pub digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ComponentVersionChange {
    pub component: WorldComponentRef,
    pub previous_version: Option<u64>,
    pub version: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldMutationReceipt {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub batch_digest: String,
    pub authority_envelope_digest: String,
    pub previous_world_revision: u64,
    pub world_revision: u64,
    pub mutation_digests: Vec<String>,
    pub component_versions: Vec<ComponentVersionChange>,
    pub committed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct IdentityHandleState {
    pub schema: String,
    pub id: String,
    pub subject: SubjectRef,
    pub value: String,
    pub active: bool,
    #[serde(default)]
    pub known_by: BTreeSet<SubjectRef>,
    #[serde(default)]
    pub restricted_to: BTreeSet<SubjectRef>,
    pub source_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStatus {
    Admitted,
    Active,
    Consumed,
    Retired,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct TypedSubject {
    pub schema: String,
    pub subject: SubjectRef,
    pub lifecycle: LifecycleStatus,
    #[serde(default)]
    pub admitted_components: BTreeSet<WorldComponentKind>,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ResourceComponentState {
    pub schema: String,
    pub resource: SubjectRef,
    pub resource_kind: String,
    pub label: String,
    pub quantity: i64,
    pub integrity: i64,
    #[serde(default)]
    pub qualities: BTreeSet<String>,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubjectEntryKey {
    pub subject: SubjectRef,
    pub entry_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CapabilityComponentState {
    pub description: String,
    pub suspended: bool,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ConditionComponentState {
    pub description: String,
    pub severity: Option<i64>,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CommitmentComponentState {
    pub kind: CommitmentKind,
    pub description: String,
    pub counterparty: Option<SubjectRef>,
    pub status: String,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RelationshipComponentState {
    pub source: SubjectRef,
    pub target: SubjectRef,
    pub description: String,
    pub strength: Option<i64>,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PressureComponentState {
    pub pressure: SubjectRef,
    pub owner: SubjectRef,
    pub label: String,
    pub progress: i64,
    pub threshold: i64,
    pub resolved: bool,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
pub struct KnowledgeKey {
    pub knower: SubjectRef,
    pub proposition: SubjectRef,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct KnowledgeComponentState {
    pub status: String,
    pub source: Option<SubjectRef>,
    pub channel: Option<SubjectRef>,
    #[serde(default)]
    pub concealed_from: BTreeSet<SubjectRef>,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MemoryComponentState {
    pub event_id: Option<String>,
    pub summary: String,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
pub struct MembershipKey {
    pub actor: SubjectRef,
    pub population: SubjectRef,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PopulationMembershipState {
    pub active: bool,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PopulationLineageState {
    pub id: String,
    pub operation: PopulationLineageOperation,
    pub parent_populations: Vec<SubjectRef>,
    pub child_populations: Vec<SubjectRef>,
    pub remainder_population: Option<SubjectRef>,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct TopologyComponentState {
    pub id: String,
    pub from_place: SubjectRef,
    pub to_place: SubjectRef,
    pub travel_minutes: i64,
    pub open: bool,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ComponentWorldState {
    pub schema: String,
    pub campaign_id: Uuid,
    pub revision: u64,
    pub resolution_epoch: u64,
    pub world_time: DateTime<Utc>,
    pub subjects: BTreeMap<SubjectRef, TypedSubject>,
    pub occupancy: BTreeMap<SubjectRef, SubjectRef>,
    pub custody: BTreeMap<SubjectRef, SubjectRef>,
    pub resources: BTreeMap<SubjectRef, ResourceComponentState>,
    pub capabilities: BTreeMap<SubjectEntryKey, CapabilityComponentState>,
    pub conditions: BTreeMap<SubjectEntryKey, ConditionComponentState>,
    pub commitments: BTreeMap<SubjectEntryKey, CommitmentComponentState>,
    pub relationships: BTreeMap<String, RelationshipComponentState>,
    pub pressures: BTreeMap<SubjectRef, PressureComponentState>,
    pub knowledge: BTreeMap<KnowledgeKey, KnowledgeComponentState>,
    pub memories: BTreeMap<SubjectEntryKey, MemoryComponentState>,
    pub postures: BTreeMap<SubjectRef, String>,
    pub memberships: BTreeMap<MembershipKey, PopulationMembershipState>,
    pub population_lineages: BTreeMap<String, PopulationLineageState>,
    pub identities: BTreeMap<String, IdentityHandleState>,
    pub topology: BTreeMap<String, TopologyComponentState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationApplication {
    pub state: ComponentWorldState,
    pub receipt: WorldMutationReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgencyAdmissionExpectation {
    Admissible,
    Impossible,
    BargainRequired,
    MissingPrimitive,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AgencyResolutionLane {
    Foreground,
    Npc,
    Reaction,
    StrategicActor,
    StrategicPopulation,
    StrategicInstitution,
    ArenaConstituent,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AgencyAttemptCase {
    pub schema: String,
    pub id: String,
    pub domain: String,
    pub scenario: String,
    pub world_fixture_ids: Vec<String>,
    pub means: ActionMeans,
    pub intended_effects: Vec<MutationIntent>,
    pub expected_admission: AgencyAdmissionExpectation,
    #[serde(default)]
    pub expected_mutation_operations: BTreeSet<WorldMutationOperation>,
    #[serde(default)]
    pub forbidden_mutation_operations: BTreeSet<WorldMutationOperation>,
    #[serde(default)]
    pub expected_bargains: Vec<String>,
    #[serde(default)]
    pub equivalent_lanes: BTreeSet<AgencyResolutionLane>,
    #[serde(default)]
    pub invariant_witnesses: Vec<String>,
    pub review_status: String,
    pub missing_primitive: Option<String>,
}

pub fn envelope_digest(envelope: &MutationAuthorityEnvelope) -> Result<String> {
    let mut value = envelope.clone();
    value.digest.clear();
    Ok(sha256(&rmp_serde::to_vec_named(&value)?))
}

pub fn mutation_batch_digest(batch: &WorldMutationBatch) -> Result<String> {
    let mut value = batch.clone();
    value.digest.clear();
    Ok(sha256(&rmp_serde::to_vec_named(&value)?))
}

pub fn mutation_digest(mutation: &PermittedWorldMutation) -> Result<String> {
    Ok(sha256(&rmp_serde::to_vec_named(mutation)?))
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn apply_component_world_batch(
    state: &ComponentWorldState,
    envelope: &MutationAuthorityEnvelope,
    batch: &WorldMutationBatch,
    now: DateTime<Utc>,
) -> Result<MutationApplication> {
    validate_batch_structure(envelope, batch, now)?;
    if state.campaign_id != batch.campaign_id
        || state.revision != batch.expected_world_revision
        || batch
            .expected_resolution_epoch
            .is_some_and(|epoch| state.resolution_epoch != epoch)
    {
        return Err(anyhow!(
            "component world state does not match the mutation snapshot"
        ));
    }

    let mut next = state.clone();
    for proposed in &batch.mutations {
        apply_component_mutation(&mut next, &proposed.mutation)?;
    }
    validate_component_world(&next)?;

    let previous_revision = state.revision;
    next.revision = next.revision.saturating_add(1);
    let mutation_digests = batch
        .mutations
        .iter()
        .map(mutation_digest)
        .collect::<Result<Vec<_>>>()?;
    let component_versions = batch
        .mutations
        .iter()
        .flat_map(|proposed| mutation_component_refs(&proposed.mutation))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|(subject, component, entry_id)| ComponentVersionChange {
            component: WorldComponentRef {
                subject,
                component,
                entry_id,
                version: next.revision,
            },
            previous_version: Some(previous_revision),
            version: Some(next.revision),
        })
        .collect();
    Ok(MutationApplication {
        state: next,
        receipt: WorldMutationReceipt {
            schema: "ghostlight.world_mutation_receipt.v1".into(),
            id: format!("mutation:{}", batch.id),
            campaign_id: batch.campaign_id,
            batch_digest: batch.digest.clone(),
            authority_envelope_digest: envelope.digest.clone(),
            previous_world_revision: previous_revision,
            world_revision: previous_revision.saturating_add(1),
            mutation_digests,
            component_versions,
            committed_at: now,
        },
    })
}

fn active_subject<'a>(
    state: &'a ComponentWorldState,
    subject: &SubjectRef,
) -> Result<&'a TypedSubject> {
    state
        .subjects
        .get(subject)
        .filter(|record| {
            matches!(
                record.lifecycle,
                LifecycleStatus::Admitted | LifecycleStatus::Active
            )
        })
        .ok_or_else(|| anyhow!("mutation subject is absent or retired: {}", subject.id))
}

fn require_kind(subject: &SubjectRef, kind: SubjectKind) -> Result<()> {
    if subject.kind != kind {
        return Err(anyhow!(
            "subject {} has kind {:?}, expected {:?}",
            subject.id,
            subject.kind,
            kind
        ));
    }
    Ok(())
}

fn next_component_version(state: &ComponentWorldState) -> u64 {
    state.revision.saturating_add(1)
}

fn apply_component_mutation(
    state: &mut ComponentWorldState,
    mutation: &WorldMutation,
) -> Result<()> {
    use WorldMutation::*;
    match mutation {
        Relocate {
            subject,
            from_place,
            to_place,
            route_id,
        } => {
            active_subject(state, subject)?;
            active_subject(state, from_place)?;
            active_subject(state, to_place)?;
            require_kind(from_place, SubjectKind::Place)?;
            require_kind(to_place, SubjectKind::Place)?;
            let route = state
                .topology
                .get(route_id)
                .ok_or_else(|| anyhow!("relocation route does not exist"))?;
            if !route.open || route.from_place != *from_place || route.to_place != *to_place {
                return Err(anyhow!(
                    "relocation route does not admit the exact origin and destination"
                ));
            }
            if state.occupancy.get(subject) != Some(from_place) {
                return Err(anyhow!(
                    "relocation origin does not match canonical occupancy"
                ));
            }
            state.occupancy.insert(subject.clone(), to_place.clone());
        }
        TransferCustody {
            resource,
            from_custodian,
            to_custodian,
        } => {
            active_subject(state, resource)?;
            active_subject(state, from_custodian)?;
            active_subject(state, to_custodian)?;
            require_kind(resource, SubjectKind::Resource)?;
            if state.custody.get(resource) != Some(from_custodian) {
                return Err(anyhow!("custody source does not own the exact resource"));
            }
            state.custody.insert(resource.clone(), to_custodian.clone());
        }
        MutateResource {
            resource,
            operation,
            custodian,
            related_resources,
            resource_kind,
            resource_label,
            recipe_id,
            quantity,
            integrity,
        } => apply_resource_mutation(
            state,
            resource,
            operation,
            custodian.as_ref(),
            related_resources,
            resource_kind.as_deref(),
            resource_label.as_deref(),
            recipe_id.as_deref(),
            *quantity,
            *integrity,
        )?,
        ChangeCapability {
            subject,
            operation,
            capability_id,
            description,
        } => {
            active_subject(state, subject)?;
            let key = SubjectEntryKey {
                subject: subject.clone(),
                entry_id: capability_id.clone(),
            };
            let version = next_component_version(state);
            match operation {
                CapabilityMutationOperation::Grant => {
                    if state.capabilities.contains_key(&key) {
                        return Err(anyhow!("capability already exists"));
                    }
                    state.capabilities.insert(
                        key,
                        CapabilityComponentState {
                            description: description
                                .clone()
                                .unwrap_or_else(|| capability_id.clone()),
                            suspended: false,
                            version,
                        },
                    );
                }
                CapabilityMutationOperation::Alter => {
                    let value = state
                        .capabilities
                        .get_mut(&key)
                        .ok_or_else(|| anyhow!("capability does not exist"))?;
                    value.description = description
                        .clone()
                        .ok_or_else(|| anyhow!("capability alteration lacks a description"))?;
                    value.version = version;
                }
                CapabilityMutationOperation::Suspend => {
                    let value = state
                        .capabilities
                        .get_mut(&key)
                        .ok_or_else(|| anyhow!("capability does not exist"))?;
                    value.suspended = true;
                    value.version = version;
                }
                CapabilityMutationOperation::Retire => {
                    if state.capabilities.remove(&key).is_none() {
                        return Err(anyhow!("capability does not exist"));
                    }
                }
            }
        }
        ChangeCondition {
            subject,
            operation,
            condition_id,
            description,
            severity,
        } => {
            active_subject(state, subject)?;
            let key = SubjectEntryKey {
                subject: subject.clone(),
                entry_id: condition_id.clone(),
            };
            let version = next_component_version(state);
            match operation {
                ConditionMutationOperation::Apply => {
                    if state.conditions.contains_key(&key) {
                        return Err(anyhow!("condition already exists"));
                    }
                    state.conditions.insert(
                        key,
                        ConditionComponentState {
                            description: description
                                .clone()
                                .unwrap_or_else(|| condition_id.clone()),
                            severity: *severity,
                            version,
                        },
                    );
                }
                ConditionMutationOperation::Alter => {
                    let value = state
                        .conditions
                        .get_mut(&key)
                        .ok_or_else(|| anyhow!("condition does not exist"))?;
                    if let Some(description) = description {
                        value.description = description.clone();
                    }
                    if severity.is_some() {
                        value.severity = *severity;
                    }
                    value.version = version;
                }
                ConditionMutationOperation::Clear => {
                    if state.conditions.remove(&key).is_none() {
                        return Err(anyhow!("condition does not exist"));
                    }
                }
            }
        }
        ChangeCommitment {
            subject,
            operation,
            kind,
            commitment_id,
            counterparty,
            description,
        } => apply_commitment_mutation(
            state,
            subject,
            operation,
            kind,
            commitment_id,
            counterparty.as_ref(),
            description.as_deref(),
        )?,
        ChangeRelationship {
            source,
            target,
            operation,
            relationship_id,
            description,
            strength_delta,
        } => apply_relationship_mutation(
            state,
            source,
            target,
            operation,
            relationship_id,
            description.as_deref(),
            *strength_delta,
        )?,
        ChangePressure {
            pressure,
            owner,
            operation,
            amount,
            label,
        } => apply_pressure_mutation(state, pressure, owner, operation, *amount, label.as_deref())?,
        ChangeKnowledge {
            operation,
            proposition,
            knower,
            speaker,
            recipients,
            channel,
        } => apply_knowledge_mutation(
            state,
            operation,
            proposition,
            knower.as_ref(),
            speaker.as_ref(),
            recipients,
            channel.as_ref(),
        )?,
        ChangeMemory {
            subject,
            operation,
            memory_id,
            event_id,
            summary,
        } => apply_memory_mutation(
            state,
            subject,
            operation,
            memory_id,
            event_id.as_deref(),
            summary.as_deref(),
        )?,
        ChangePosture { subject, posture } => {
            active_subject(state, subject)?;
            state.postures.insert(subject.clone(), posture.clone());
        }
        ChangePopulationMembership {
            actor,
            operation,
            source_population,
            destination_population,
        } => apply_membership_mutation(
            state,
            actor,
            operation,
            source_population.as_ref(),
            destination_population.as_ref(),
        )?,
        ChangePopulationLineage {
            operation,
            parent_populations,
            child_populations,
            remainder_population,
        } => apply_lineage_mutation(
            state,
            operation,
            parent_populations,
            child_populations,
            remainder_population.as_ref(),
        )?,
        ChangeIdentity {
            subject,
            operation,
            handle_id,
            handle_value,
            audience,
        } => apply_identity_mutation(
            state,
            subject,
            operation,
            handle_id,
            handle_value.as_deref(),
            audience,
        )?,
        ChangeTopology {
            operation,
            edge_id,
            from_place,
            to_place,
            travel_minutes,
        } => apply_topology_mutation(
            state,
            operation,
            edge_id,
            from_place,
            to_place,
            *travel_minutes,
        )?,
        AdmitEntity {
            subject,
            initial_components,
            ..
        } => {
            if subject.kind == SubjectKind::Resource {
                return Err(anyhow!("resources are admitted by resource creation"));
            }
            if state.subjects.contains_key(subject) {
                return Err(anyhow!("entity already exists"));
            }
            state.subjects.insert(
                subject.clone(),
                TypedSubject {
                    schema: "ghostlight.typed_subject.v1".into(),
                    subject: subject.clone(),
                    lifecycle: LifecycleStatus::Admitted,
                    admitted_components: initial_components.clone(),
                    version: next_component_version(state),
                },
            );
        }
        RetireEntity { subject, .. } => retire_entity(state, subject)?,
        AdvanceWorldTime { campaign, minutes } => {
            require_kind(campaign, SubjectKind::Campaign)?;
            active_subject(state, campaign)?;
            state.world_time += chrono::Duration::minutes(*minutes);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_resource_mutation(
    state: &mut ComponentWorldState,
    resource: &SubjectRef,
    operation: &ResourceMutationOperation,
    custodian: Option<&SubjectRef>,
    related_resources: &[SubjectRef],
    resource_kind: Option<&str>,
    resource_label: Option<&str>,
    recipe_id: Option<&str>,
    quantity: Option<i64>,
    integrity: Option<i64>,
) -> Result<()> {
    require_kind(resource, SubjectKind::Resource)?;
    let version = next_component_version(state);
    if !matches!(operation, ResourceMutationOperation::Create) {
        let custodian =
            custodian.ok_or_else(|| anyhow!("resource mutation lacks exact custody authority"))?;
        active_subject(state, custodian)?;
        if matches!(operation, ResourceMutationOperation::Combine) {
            if related_resources
                .iter()
                .any(|input| state.custody.get(input) != Some(custodian))
            {
                return Err(anyhow!("resource combination exceeds exact custody"));
            }
        } else if state.custody.get(resource) != Some(custodian) {
            return Err(anyhow!("resource mutation exceeds exact custody"));
        }
    }
    match operation {
        ResourceMutationOperation::Create => {
            if state.subjects.contains_key(resource) || state.resources.contains_key(resource) {
                return Err(anyhow!("resource already exists"));
            }
            let custodian = custodian.ok_or_else(|| anyhow!("resource creation lacks custody"))?;
            active_subject(state, custodian)?;
            let quantity = quantity.ok_or_else(|| anyhow!("resource creation lacks quantity"))?;
            if quantity <= 0 {
                return Err(anyhow!("resource quantity must be positive"));
            }
            let resource_kind = resource_kind
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("resource creation lacks a kind"))?;
            let resource_label = resource_label
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("resource creation lacks a label"))?;
            state.subjects.insert(
                resource.clone(),
                TypedSubject {
                    schema: "ghostlight.typed_subject.v1".into(),
                    subject: resource.clone(),
                    lifecycle: LifecycleStatus::Active,
                    admitted_components: BTreeSet::from([
                        WorldComponentKind::ResourceState,
                        WorldComponentKind::Custody,
                        WorldComponentKind::Lifecycle,
                    ]),
                    version,
                },
            );
            state.resources.insert(
                resource.clone(),
                ResourceComponentState {
                    schema: "ghostlight.resource_component.v1".into(),
                    resource: resource.clone(),
                    resource_kind: resource_kind.into(),
                    label: resource_label.into(),
                    quantity,
                    integrity: integrity.unwrap_or(100),
                    qualities: BTreeSet::new(),
                    version,
                },
            );
            state.custody.insert(resource.clone(), custodian.clone());
        }
        ResourceMutationOperation::Transform => {
            let _recipe =
                recipe_id.ok_or_else(|| anyhow!("resource transformation lacks recipe"))?;
            let owner = custodian.expect("non-create custody checked").clone();
            for input in related_resources {
                active_subject(state, input)?;
                require_kind(input, SubjectKind::Resource)?;
                if state.custody.get(input) != Some(&owner) {
                    return Err(anyhow!("resource transformation crosses custody"));
                }
            }
            for input in related_resources {
                consume_entire_resource(state, input)?;
            }
            let value = state
                .resources
                .get_mut(resource)
                .ok_or_else(|| anyhow!("resource does not exist"))?;
            if let Some(kind) = resource_kind {
                value.resource_kind = kind.into();
            }
            if let Some(label) = resource_label {
                value.label = label.into();
            }
            if quantity.is_some_and(|quantity| quantity != value.quantity) {
                return Err(anyhow!(
                    "resource transformation cannot alter quantity; compose consumption or creation"
                ));
            }
            if let Some(integrity) = integrity {
                value.integrity = integrity;
            }
            value.version = version;
        }
        ResourceMutationOperation::Consume => {
            active_subject(state, resource)?;
            let amount = quantity.ok_or_else(|| anyhow!("resource consumption lacks quantity"))?;
            if amount <= 0 {
                return Err(anyhow!("resource consumption must be positive"));
            }
            let value = state
                .resources
                .get_mut(resource)
                .ok_or_else(|| anyhow!("resource does not exist"))?;
            if value.quantity < amount {
                return Err(anyhow!("resource consumption exceeds available quantity"));
            }
            value.quantity -= amount;
            value.version = version;
            if value.quantity == 0 {
                state.custody.remove(resource);
                state.resources.remove(resource);
                state
                    .subjects
                    .get_mut(resource)
                    .expect("active resource has a subject")
                    .lifecycle = LifecycleStatus::Consumed;
            }
        }
        ResourceMutationOperation::Damage | ResourceMutationOperation::Repair => {
            active_subject(state, resource)?;
            let amount = integrity.ok_or_else(|| anyhow!("integrity mutation lacks amount"))?;
            if amount <= 0 {
                return Err(anyhow!("integrity mutation must be positive"));
            }
            let value = state
                .resources
                .get_mut(resource)
                .ok_or_else(|| anyhow!("resource does not exist"))?;
            value.integrity = if matches!(operation, ResourceMutationOperation::Damage) {
                value.integrity.saturating_sub(amount)
            } else {
                value.integrity.saturating_add(amount).min(100)
            };
            value.version = version;
        }
        ResourceMutationOperation::Split => {
            active_subject(state, resource)?;
            let amount = quantity.ok_or_else(|| anyhow!("resource split lacks quantity"))?;
            let [child] = related_resources else {
                return Err(anyhow!(
                    "resource split requires exactly one reserved child"
                ));
            };
            require_kind(child, SubjectKind::Resource)?;
            if state.subjects.contains_key(child) {
                return Err(anyhow!("resource split child already exists"));
            }
            let owner = custodian.expect("non-create custody checked").clone();
            let source = state
                .resources
                .get_mut(resource)
                .ok_or_else(|| anyhow!("resource does not exist"))?;
            if amount <= 0 || amount >= source.quantity {
                return Err(anyhow!(
                    "resource split must preserve positive source and child lots"
                ));
            }
            source.quantity -= amount;
            source.version = version;
            let child_state = ResourceComponentState {
                schema: source.schema.clone(),
                resource: child.clone(),
                resource_kind: source.resource_kind.clone(),
                label: resource_label.unwrap_or(&source.label).into(),
                quantity: amount,
                integrity: source.integrity,
                qualities: source.qualities.clone(),
                version,
            };
            state.subjects.insert(
                child.clone(),
                TypedSubject {
                    schema: "ghostlight.typed_subject.v1".into(),
                    subject: child.clone(),
                    lifecycle: LifecycleStatus::Active,
                    admitted_components: BTreeSet::from([
                        WorldComponentKind::ResourceState,
                        WorldComponentKind::Custody,
                        WorldComponentKind::Lifecycle,
                    ]),
                    version,
                },
            );
            state.resources.insert(child.clone(), child_state);
            state.custody.insert(child.clone(), owner);
        }
        ResourceMutationOperation::Combine => {
            if state.subjects.contains_key(resource) {
                return Err(anyhow!("combined resource target already exists"));
            }
            if related_resources.len() < 2 {
                return Err(anyhow!("resource combination requires at least two inputs"));
            }
            let _recipe = recipe_id.ok_or_else(|| anyhow!("resource combination lacks recipe"))?;
            let first = related_resources
                .first()
                .expect("length checked before first");
            let owner = custodian.expect("non-create custody checked").clone();
            let first_state = state
                .resources
                .get(first)
                .cloned()
                .ok_or_else(|| anyhow!("combined input does not exist"))?;
            let mut total = 0i64;
            for input in related_resources {
                active_subject(state, input)?;
                if state.custody.get(input) != Some(&owner) {
                    return Err(anyhow!("resource combination crosses custody"));
                }
                total = total.saturating_add(
                    state
                        .resources
                        .get(input)
                        .ok_or_else(|| anyhow!("combined input does not exist"))?
                        .quantity,
                );
            }
            for input in related_resources {
                consume_entire_resource(state, input)?;
            }
            state.subjects.insert(
                resource.clone(),
                TypedSubject {
                    schema: "ghostlight.typed_subject.v1".into(),
                    subject: resource.clone(),
                    lifecycle: LifecycleStatus::Active,
                    admitted_components: BTreeSet::from([
                        WorldComponentKind::ResourceState,
                        WorldComponentKind::Custody,
                        WorldComponentKind::Lifecycle,
                    ]),
                    version,
                },
            );
            if quantity.is_some_and(|quantity| quantity != total) {
                return Err(anyhow!(
                    "resource combination must conserve the exact input quantity"
                ));
            }
            state.resources.insert(
                resource.clone(),
                ResourceComponentState {
                    schema: first_state.schema,
                    resource: resource.clone(),
                    resource_kind: resource_kind.unwrap_or(&first_state.resource_kind).into(),
                    label: resource_label.unwrap_or(&first_state.label).into(),
                    quantity: total,
                    integrity: integrity.unwrap_or(first_state.integrity),
                    qualities: first_state.qualities,
                    version,
                },
            );
            state.custody.insert(resource.clone(), owner);
        }
    }
    Ok(())
}

fn consume_entire_resource(state: &mut ComponentWorldState, resource: &SubjectRef) -> Result<()> {
    state
        .resources
        .remove(resource)
        .ok_or_else(|| anyhow!("resource input does not exist"))?;
    state.custody.remove(resource);
    state
        .subjects
        .get_mut(resource)
        .ok_or_else(|| anyhow!("resource subject does not exist"))?
        .lifecycle = LifecycleStatus::Consumed;
    Ok(())
}

fn apply_commitment_mutation(
    state: &mut ComponentWorldState,
    subject: &SubjectRef,
    operation: &CommitmentMutationOperation,
    kind: &CommitmentKind,
    commitment_id: &str,
    counterparty: Option<&SubjectRef>,
    description: Option<&str>,
) -> Result<()> {
    active_subject(state, subject)?;
    if let Some(counterparty) = counterparty {
        active_subject(state, counterparty)?;
    }
    let key = SubjectEntryKey {
        subject: subject.clone(),
        entry_id: commitment_id.into(),
    };
    let version = next_component_version(state);
    match operation {
        CommitmentMutationOperation::Create => {
            if state.commitments.contains_key(&key) {
                return Err(anyhow!("commitment already exists"));
            }
            state.commitments.insert(
                key,
                CommitmentComponentState {
                    kind: kind.clone(),
                    description: description.unwrap_or(commitment_id).into(),
                    counterparty: counterparty.cloned(),
                    status: "active".into(),
                    version,
                },
            );
        }
        CommitmentMutationOperation::Alter => {
            let value = state
                .commitments
                .get_mut(&key)
                .ok_or_else(|| anyhow!("commitment does not exist"))?;
            if let Some(description) = description {
                value.description = description.into();
            }
            value.counterparty = counterparty.cloned().or(value.counterparty.clone());
            value.version = version;
        }
        CommitmentMutationOperation::Fulfill => {
            let value = state
                .commitments
                .get_mut(&key)
                .ok_or_else(|| anyhow!("commitment does not exist"))?;
            value.status = "fulfilled".into();
            value.version = version;
        }
        CommitmentMutationOperation::Default => {
            let value = state
                .commitments
                .get_mut(&key)
                .ok_or_else(|| anyhow!("commitment does not exist"))?;
            value.status = "defaulted".into();
            value.version = version;
        }
        CommitmentMutationOperation::Retire => {
            if state.commitments.remove(&key).is_none() {
                return Err(anyhow!("commitment does not exist"));
            }
        }
    }
    Ok(())
}

fn apply_relationship_mutation(
    state: &mut ComponentWorldState,
    source: &SubjectRef,
    target: &SubjectRef,
    operation: &RelationshipMutationOperation,
    relationship_id: &str,
    description: Option<&str>,
    strength_delta: Option<i64>,
) -> Result<()> {
    active_subject(state, source)?;
    active_subject(state, target)?;
    let version = next_component_version(state);
    match operation {
        RelationshipMutationOperation::Create => {
            if state.relationships.contains_key(relationship_id) {
                return Err(anyhow!("relationship already exists"));
            }
            if strength_delta.is_some_and(|value| !(0..=100).contains(&value)) {
                return Err(anyhow!("relationship strength exceeds bounds"));
            }
            state.relationships.insert(
                relationship_id.into(),
                RelationshipComponentState {
                    source: source.clone(),
                    target: target.clone(),
                    description: description
                        .filter(|value| !value.trim().is_empty())
                        .ok_or_else(|| anyhow!("relationship creation lacks a description"))?
                        .into(),
                    strength: strength_delta,
                    version,
                },
            );
        }
        RelationshipMutationOperation::Alter => {
            let value = state
                .relationships
                .get_mut(relationship_id)
                .ok_or_else(|| anyhow!("relationship does not exist"))?;
            if value.source != *source || value.target != *target {
                return Err(anyhow!("relationship endpoints do not match"));
            }
            if let Some(description) = description.filter(|value| !value.trim().is_empty()) {
                value.description = description.into();
            }
            if let Some(delta) = strength_delta {
                let current = value
                    .strength
                    .ok_or_else(|| anyhow!("relationship has no numeric strength"))?;
                let next = current
                    .checked_add(delta)
                    .filter(|next| (0..=100).contains(next))
                    .ok_or_else(|| anyhow!("relationship strength exceeds bounds"))?;
                value.strength = Some(next);
            }
            if description.is_none() && strength_delta.is_none() {
                return Err(anyhow!("relationship alteration is empty"));
            }
            value.version = version;
        }
        RelationshipMutationOperation::Retire => {
            let value = state
                .relationships
                .get(relationship_id)
                .ok_or_else(|| anyhow!("relationship does not exist"))?;
            if value.source != *source || value.target != *target {
                return Err(anyhow!("relationship endpoints do not match"));
            }
            state.relationships.remove(relationship_id);
        }
    }
    Ok(())
}

fn apply_pressure_mutation(
    state: &mut ComponentWorldState,
    pressure: &SubjectRef,
    owner: &SubjectRef,
    operation: &PressureMutationOperation,
    amount: Option<i64>,
    label: Option<&str>,
) -> Result<()> {
    require_kind(pressure, SubjectKind::Pressure)?;
    active_subject(state, owner)?;
    let version = next_component_version(state);
    match operation {
        PressureMutationOperation::Create => {
            if state.subjects.contains_key(pressure) || state.pressures.contains_key(pressure) {
                return Err(anyhow!("pressure already exists"));
            }
            let threshold = amount.unwrap_or(4);
            if threshold <= 0 {
                return Err(anyhow!("pressure threshold must be positive"));
            }
            state.subjects.insert(
                pressure.clone(),
                TypedSubject {
                    schema: "ghostlight.typed_subject.v1".into(),
                    subject: pressure.clone(),
                    lifecycle: LifecycleStatus::Active,
                    admitted_components: BTreeSet::from([
                        WorldComponentKind::Pressure,
                        WorldComponentKind::Lifecycle,
                    ]),
                    version,
                },
            );
            state.pressures.insert(
                pressure.clone(),
                PressureComponentState {
                    pressure: pressure.clone(),
                    owner: owner.clone(),
                    label: label
                        .filter(|label| !label.trim().is_empty())
                        .ok_or_else(|| anyhow!("pressure creation lacks a label"))?
                        .into(),
                    progress: 0,
                    threshold,
                    resolved: false,
                    version,
                },
            );
        }
        PressureMutationOperation::Advance | PressureMutationOperation::Reduce => {
            let amount = amount.ok_or_else(|| anyhow!("pressure change lacks an amount"))?;
            if amount <= 0 {
                return Err(anyhow!("pressure change must be positive"));
            }
            let value = state
                .pressures
                .get_mut(pressure)
                .ok_or_else(|| anyhow!("pressure does not exist"))?;
            if value.owner != *owner || value.resolved {
                return Err(anyhow!("pressure owner or lifecycle does not match"));
            }
            value.progress = if matches!(operation, PressureMutationOperation::Advance) {
                value.progress.saturating_add(amount).min(value.threshold)
            } else {
                value.progress.saturating_sub(amount)
            };
            value.version = version;
        }
        PressureMutationOperation::Resolve => {
            let value = state
                .pressures
                .get_mut(pressure)
                .ok_or_else(|| anyhow!("pressure does not exist"))?;
            if value.owner != *owner || value.resolved {
                return Err(anyhow!("pressure owner or lifecycle does not match"));
            }
            value.resolved = true;
            value.version = version;
        }
        PressureMutationOperation::Retire => {
            let value = state
                .pressures
                .get(pressure)
                .ok_or_else(|| anyhow!("pressure does not exist"))?;
            if value.owner != *owner {
                return Err(anyhow!("pressure owner does not match"));
            }
            state.pressures.remove(pressure);
            state
                .subjects
                .get_mut(pressure)
                .expect("pressure component has subject")
                .lifecycle = LifecycleStatus::Retired;
        }
    }
    Ok(())
}

fn apply_knowledge_mutation(
    state: &mut ComponentWorldState,
    operation: &KnowledgeMutationOperation,
    proposition: &SubjectRef,
    knower: Option<&SubjectRef>,
    speaker: Option<&SubjectRef>,
    recipients: &[SubjectRef],
    channel: Option<&SubjectRef>,
) -> Result<()> {
    active_subject(state, proposition)?;
    require_kind(proposition, SubjectKind::Proposition)?;
    if let Some(channel) = channel {
        active_subject(state, channel)?;
        require_kind(channel, SubjectKind::Channel)?;
    }
    let version = next_component_version(state);
    match operation {
        KnowledgeMutationOperation::Acquire => {
            let knower = knower.ok_or_else(|| anyhow!("knowledge acquisition lacks a knower"))?;
            active_subject(state, knower)?;
            let key = KnowledgeKey {
                knower: knower.clone(),
                proposition: proposition.clone(),
            };
            if state
                .knowledge
                .get(&key)
                .is_some_and(|knowledge| knowledge.status == "known")
            {
                return Err(anyhow!("knower already has the proposition"));
            }
            state.knowledge.insert(
                key,
                KnowledgeComponentState {
                    status: "known".into(),
                    source: None,
                    channel: channel.cloned(),
                    concealed_from: BTreeSet::new(),
                    version,
                },
            );
        }
        KnowledgeMutationOperation::Communicate => {
            let speaker = speaker.ok_or_else(|| anyhow!("communication lacks a speaker"))?;
            active_subject(state, speaker)?;
            let speaker_key = KnowledgeKey {
                knower: speaker.clone(),
                proposition: proposition.clone(),
            };
            if !state
                .knowledge
                .get(&speaker_key)
                .is_some_and(|knowledge| knowledge.status == "known")
            {
                return Err(anyhow!("speaker does not know the proposition"));
            }
            if recipients.is_empty() {
                return Err(anyhow!("communication lacks exact recipients"));
            }
            for recipient in recipients {
                active_subject(state, recipient)?;
                let key = KnowledgeKey {
                    knower: recipient.clone(),
                    proposition: proposition.clone(),
                };
                state.knowledge.insert(
                    key,
                    KnowledgeComponentState {
                        status: "known".into(),
                        source: Some(speaker.clone()),
                        channel: channel.cloned(),
                        concealed_from: BTreeSet::new(),
                        version,
                    },
                );
            }
        }
        KnowledgeMutationOperation::Conceal => {
            let knower = knower.ok_or_else(|| anyhow!("concealment lacks a knower"))?;
            let value = state
                .knowledge
                .get_mut(&KnowledgeKey {
                    knower: knower.clone(),
                    proposition: proposition.clone(),
                })
                .ok_or_else(|| anyhow!("knower does not hold the proposition"))?;
            value.concealed_from.extend(recipients.iter().cloned());
            value.version = version;
        }
        KnowledgeMutationOperation::Correct | KnowledgeMutationOperation::Invalidate => {
            let knower = knower.ok_or_else(|| anyhow!("knowledge change lacks a knower"))?;
            let value = state
                .knowledge
                .get_mut(&KnowledgeKey {
                    knower: knower.clone(),
                    proposition: proposition.clone(),
                })
                .ok_or_else(|| anyhow!("knower does not hold the proposition"))?;
            value.status = if matches!(operation, KnowledgeMutationOperation::Correct) {
                "corrected"
            } else {
                "invalidated"
            }
            .into();
            value.version = version;
        }
    }
    Ok(())
}

fn apply_memory_mutation(
    state: &mut ComponentWorldState,
    subject: &SubjectRef,
    operation: &MemoryMutationOperation,
    memory_id: &str,
    event_id: Option<&str>,
    summary: Option<&str>,
) -> Result<()> {
    active_subject(state, subject)?;
    let key = SubjectEntryKey {
        subject: subject.clone(),
        entry_id: memory_id.into(),
    };
    let version = next_component_version(state);
    match operation {
        MemoryMutationOperation::Record => {
            if state.memories.contains_key(&key) {
                return Err(anyhow!("memory already exists"));
            }
            state.memories.insert(
                key,
                MemoryComponentState {
                    event_id: event_id.map(str::to_owned),
                    summary: summary
                        .filter(|summary| !summary.trim().is_empty())
                        .ok_or_else(|| anyhow!("memory record lacks a summary"))?
                        .into(),
                    version,
                },
            );
        }
        MemoryMutationOperation::Revise => {
            let value = state
                .memories
                .get_mut(&key)
                .ok_or_else(|| anyhow!("memory does not exist"))?;
            value.summary = summary
                .filter(|summary| !summary.trim().is_empty())
                .ok_or_else(|| anyhow!("memory revision lacks a summary"))?
                .into();
            value.event_id = event_id.map(str::to_owned).or(value.event_id.clone());
            value.version = version;
        }
        MemoryMutationOperation::Retire => {
            if state.memories.remove(&key).is_none() {
                return Err(anyhow!("memory does not exist"));
            }
        }
    }
    Ok(())
}

fn apply_membership_mutation(
    state: &mut ComponentWorldState,
    actor: &SubjectRef,
    operation: &PopulationMembershipOperation,
    source_population: Option<&SubjectRef>,
    destination_population: Option<&SubjectRef>,
) -> Result<()> {
    active_subject(state, actor)?;
    require_kind(actor, SubjectKind::Actor)?;
    if let Some(population) = source_population {
        active_subject(state, population)?;
        require_kind(population, SubjectKind::Population)?;
    }
    if let Some(population) = destination_population {
        active_subject(state, population)?;
        require_kind(population, SubjectKind::Population)?;
    }
    let version = next_component_version(state);
    let source_key = source_population.map(|population| MembershipKey {
        actor: actor.clone(),
        population: population.clone(),
    });
    let destination_key = destination_population.map(|population| MembershipKey {
        actor: actor.clone(),
        population: population.clone(),
    });
    match operation {
        PopulationMembershipOperation::Join => {
            let key =
                destination_key.ok_or_else(|| anyhow!("population join lacks destination"))?;
            if state
                .memberships
                .get(&key)
                .is_some_and(|membership| membership.active)
            {
                return Err(anyhow!("actor already belongs to destination population"));
            }
            state.memberships.insert(
                key,
                PopulationMembershipState {
                    active: true,
                    version,
                },
            );
        }
        PopulationMembershipOperation::Leave => {
            let key = source_key.ok_or_else(|| anyhow!("population leave lacks source"))?;
            let value = state
                .memberships
                .get_mut(&key)
                .filter(|membership| membership.active)
                .ok_or_else(|| anyhow!("actor does not belong to source population"))?;
            value.active = false;
            value.version = version;
        }
        PopulationMembershipOperation::Transfer => {
            let source = source_key.ok_or_else(|| anyhow!("population transfer lacks source"))?;
            let destination =
                destination_key.ok_or_else(|| anyhow!("population transfer lacks destination"))?;
            if source == destination {
                return Err(anyhow!(
                    "population transfer source and destination are identical"
                ));
            }
            if state
                .memberships
                .get(&destination)
                .is_some_and(|membership| membership.active)
            {
                return Err(anyhow!("actor already belongs to destination population"));
            }
            let source_value = state
                .memberships
                .get_mut(&source)
                .filter(|membership| membership.active)
                .ok_or_else(|| anyhow!("actor does not belong to source population"))?;
            source_value.active = false;
            source_value.version = version;
            state.memberships.insert(
                destination,
                PopulationMembershipState {
                    active: true,
                    version,
                },
            );
        }
    }
    Ok(())
}

fn apply_lineage_mutation(
    state: &mut ComponentWorldState,
    operation: &PopulationLineageOperation,
    parents: &[SubjectRef],
    children: &[SubjectRef],
    remainder: Option<&SubjectRef>,
) -> Result<()> {
    for population in parents.iter().chain(children).chain(remainder) {
        active_subject(state, population)?;
        require_kind(population, SubjectKind::Population)?;
    }
    let parent_set = parents.iter().cloned().collect::<BTreeSet<_>>();
    let child_set = children.iter().cloned().collect::<BTreeSet<_>>();
    if parent_set.len() != parents.len()
        || child_set.len() != children.len()
        || !parent_set.is_disjoint(&child_set)
    {
        return Err(anyhow!("population lineage repeats or overlaps subjects"));
    }
    match operation {
        PopulationLineageOperation::Split => {
            if parents.len() != 1 || children.is_empty() {
                return Err(anyhow!("population split shape is invalid"));
            }
            let remainder = remainder.ok_or_else(|| anyhow!("population split lacks remainder"))?;
            if !child_set.contains(remainder) {
                return Err(anyhow!("population split remainder is not a child"));
            }
        }
        PopulationLineageOperation::Merge => {
            if parents.len() < 2 || children.len() != 1 || remainder.is_some() {
                return Err(anyhow!("population merge shape is invalid"));
            }
        }
    }
    let encoded = rmp_serde::to_vec_named(&(operation, parents, children, remainder))?;
    let id = format!("lineage:{}", &sha256(&encoded)[7..23]);
    if state.population_lineages.contains_key(&id) {
        return Err(anyhow!("population lineage already exists"));
    }
    state.population_lineages.insert(
        id.clone(),
        PopulationLineageState {
            id,
            operation: operation.clone(),
            parent_populations: parents.to_vec(),
            child_populations: children.to_vec(),
            remainder_population: remainder.cloned(),
            version: next_component_version(state),
        },
    );
    Ok(())
}

fn apply_identity_mutation(
    state: &mut ComponentWorldState,
    subject: &SubjectRef,
    operation: &IdentityMutationOperation,
    handle_id: &str,
    handle_value: Option<&str>,
    audience: &[SubjectRef],
) -> Result<()> {
    active_subject(state, subject)?;
    for observer in audience {
        active_subject(state, observer)?;
    }
    let version = next_component_version(state);
    match operation {
        IdentityMutationOperation::Adopt => {
            if state.identities.contains_key(handle_id) {
                return Err(anyhow!("identity handle already exists"));
            }
            state.identities.insert(
                handle_id.into(),
                IdentityHandleState {
                    schema: "ghostlight.identity_handle.v1".into(),
                    id: handle_id.into(),
                    subject: subject.clone(),
                    value: handle_value
                        .filter(|value| !value.trim().is_empty())
                        .ok_or_else(|| anyhow!("identity adoption lacks a handle value"))?
                        .into(),
                    active: true,
                    known_by: BTreeSet::from([subject.clone()]),
                    restricted_to: BTreeSet::new(),
                    source_revision: version,
                },
            );
        }
        IdentityMutationOperation::Disclose => {
            let value = state
                .identities
                .get_mut(handle_id)
                .filter(|handle| handle.active && handle.subject == *subject)
                .ok_or_else(|| anyhow!("active identity handle does not exist for subject"))?;
            if handle_value.is_some_and(|candidate| candidate != value.value) {
                return Err(anyhow!(
                    "identity disclosure value does not match the handle"
                ));
            }
            value.known_by.extend(audience.iter().cloned());
            value.source_revision = version;
        }
        IdentityMutationOperation::Restrict => {
            let value = state
                .identities
                .get_mut(handle_id)
                .filter(|handle| handle.active && handle.subject == *subject)
                .ok_or_else(|| anyhow!("active identity handle does not exist for subject"))?;
            value.restricted_to = audience.iter().cloned().collect();
            value.source_revision = version;
        }
        IdentityMutationOperation::Retire => {
            let value = state
                .identities
                .get_mut(handle_id)
                .filter(|handle| handle.active && handle.subject == *subject)
                .ok_or_else(|| anyhow!("active identity handle does not exist for subject"))?;
            value.active = false;
            value.source_revision = version;
        }
    }
    Ok(())
}

fn apply_topology_mutation(
    state: &mut ComponentWorldState,
    operation: &TopologyMutationOperation,
    edge_id: &str,
    from_place: &SubjectRef,
    to_place: &SubjectRef,
    travel_minutes: Option<i64>,
) -> Result<()> {
    active_subject(state, from_place)?;
    active_subject(state, to_place)?;
    require_kind(from_place, SubjectKind::Place)?;
    require_kind(to_place, SubjectKind::Place)?;
    if from_place == to_place {
        return Err(anyhow!("topology edge cannot connect a place to itself"));
    }
    let version = next_component_version(state);
    match operation {
        TopologyMutationOperation::Add => {
            if state.topology.contains_key(edge_id) {
                return Err(anyhow!("topology edge already exists"));
            }
            let travel_minutes = travel_minutes
                .filter(|minutes| *minutes > 0)
                .ok_or_else(|| anyhow!("topology edge requires positive travel time"))?;
            state.topology.insert(
                edge_id.into(),
                TopologyComponentState {
                    id: edge_id.into(),
                    from_place: from_place.clone(),
                    to_place: to_place.clone(),
                    travel_minutes,
                    open: true,
                    version,
                },
            );
        }
        TopologyMutationOperation::Alter => {
            let value = state
                .topology
                .get_mut(edge_id)
                .ok_or_else(|| anyhow!("topology edge does not exist"))?;
            if value.from_place != *from_place || value.to_place != *to_place {
                return Err(anyhow!("topology endpoints do not match"));
            }
            value.travel_minutes = travel_minutes
                .filter(|minutes| *minutes > 0)
                .ok_or_else(|| anyhow!("topology alteration requires positive travel time"))?;
            value.version = version;
        }
        TopologyMutationOperation::Open | TopologyMutationOperation::Close => {
            let value = state
                .topology
                .get_mut(edge_id)
                .ok_or_else(|| anyhow!("topology edge does not exist"))?;
            if value.from_place != *from_place || value.to_place != *to_place {
                return Err(anyhow!("topology endpoints do not match"));
            }
            let next_open = matches!(operation, TopologyMutationOperation::Open);
            if value.open == next_open {
                return Err(anyhow!("topology operation is a no-op"));
            }
            value.open = next_open;
            value.version = version;
        }
        TopologyMutationOperation::Retire => {
            let value = state
                .topology
                .get(edge_id)
                .ok_or_else(|| anyhow!("topology edge does not exist"))?;
            if value.from_place != *from_place || value.to_place != *to_place {
                return Err(anyhow!("topology endpoints do not match"));
            }
            state.topology.remove(edge_id);
        }
    }
    Ok(())
}

fn retire_entity(state: &mut ComponentWorldState, subject: &SubjectRef) -> Result<()> {
    active_subject(state, subject)?;
    if state.occupancy.contains_key(subject)
        || state.custody.contains_key(subject)
        || state.custody.values().any(|owner| owner == subject)
        || state.memberships.iter().any(|(key, value)| {
            value.active && (&key.actor == subject || &key.population == subject)
        })
        || state
            .topology
            .values()
            .any(|edge| &edge.from_place == subject || &edge.to_place == subject)
        || state
            .relationships
            .values()
            .any(|relationship| &relationship.source == subject || &relationship.target == subject)
        || state.commitments.iter().any(|(key, commitment)| {
            &key.subject == subject || commitment.counterparty.as_ref() == Some(subject)
        })
    {
        return Err(anyhow!(
            "entity retirement would leave active component references"
        ));
    }
    let version = next_component_version(state);
    let record = state
        .subjects
        .get_mut(subject)
        .expect("active subject was resolved");
    record.lifecycle = LifecycleStatus::Retired;
    record.version = version;
    Ok(())
}

pub fn validate_component_world(state: &ComponentWorldState) -> Result<()> {
    for (subject, record) in &state.subjects {
        if &record.subject != subject || subject.id.trim().is_empty() {
            return Err(anyhow!("typed subject identity is malformed"));
        }
    }
    for (subject, place) in &state.occupancy {
        active_subject(state, subject)?;
        active_subject(state, place)?;
        require_kind(place, SubjectKind::Place)?;
    }
    for (resource, custodian) in &state.custody {
        active_subject(state, resource)?;
        require_kind(resource, SubjectKind::Resource)?;
        active_subject(state, custodian)?;
        if !state.resources.contains_key(resource) {
            return Err(anyhow!("custody references a missing resource component"));
        }
    }
    for (resource, value) in &state.resources {
        active_subject(state, resource)?;
        require_kind(resource, SubjectKind::Resource)?;
        if &value.resource != resource
            || value.label.trim().is_empty()
            || value.quantity <= 0
            || value.integrity < 0
            || value.integrity > 100
            || !state.custody.contains_key(resource)
        {
            return Err(anyhow!(
                "resource component violates conservation or bounds"
            ));
        }
    }
    for key in state
        .capabilities
        .keys()
        .chain(state.conditions.keys())
        .chain(state.commitments.keys())
        .chain(state.memories.keys())
    {
        active_subject(state, &key.subject)?;
        if key.entry_id.trim().is_empty() {
            return Err(anyhow!("component entry id is empty"));
        }
    }
    for relationship in state.relationships.values() {
        active_subject(state, &relationship.source)?;
        active_subject(state, &relationship.target)?;
    }
    for (pressure, value) in &state.pressures {
        active_subject(state, pressure)?;
        require_kind(pressure, SubjectKind::Pressure)?;
        active_subject(state, &value.owner)?;
        if &value.pressure != pressure
            || value.progress < 0
            || value.progress > value.threshold
            || value.threshold <= 0
        {
            return Err(anyhow!("pressure component violates bounds"));
        }
    }
    for (key, value) in &state.knowledge {
        active_subject(state, &key.knower)?;
        active_subject(state, &key.proposition)?;
        require_kind(&key.proposition, SubjectKind::Proposition)?;
        if let Some(source) = &value.source {
            active_subject(state, source)?;
        }
        if let Some(channel) = &value.channel {
            active_subject(state, channel)?;
            require_kind(channel, SubjectKind::Channel)?;
        }
        for observer in &value.concealed_from {
            active_subject(state, observer)?;
        }
    }
    for subject in state.postures.keys() {
        active_subject(state, subject)?;
    }
    for (key, value) in &state.memberships {
        active_subject(state, &key.actor)?;
        active_subject(state, &key.population)?;
        require_kind(&key.actor, SubjectKind::Actor)?;
        require_kind(&key.population, SubjectKind::Population)?;
        if !value.active && value.version == 0 {
            return Err(anyhow!("inactive membership lacks history"));
        }
    }
    for lineage in state.population_lineages.values() {
        for population in lineage
            .parent_populations
            .iter()
            .chain(&lineage.child_populations)
            .chain(lineage.remainder_population.as_ref())
        {
            active_subject(state, population)?;
            require_kind(population, SubjectKind::Population)?;
        }
    }
    for handle in state.identities.values() {
        active_subject(state, &handle.subject)?;
        if handle.id.trim().is_empty() || handle.value.trim().is_empty() {
            return Err(anyhow!("identity handle is malformed"));
        }
        for observer in handle.known_by.iter().chain(&handle.restricted_to) {
            active_subject(state, observer)?;
        }
    }
    for edge in state.topology.values() {
        active_subject(state, &edge.from_place)?;
        active_subject(state, &edge.to_place)?;
        require_kind(&edge.from_place, SubjectKind::Place)?;
        require_kind(&edge.to_place, SubjectKind::Place)?;
        if edge.travel_minutes <= 0 || edge.from_place == edge.to_place {
            return Err(anyhow!("topology edge is malformed"));
        }
    }
    Ok(())
}

fn mutation_component_refs(
    mutation: &WorldMutation,
) -> Vec<(SubjectRef, WorldComponentKind, Option<String>)> {
    use WorldComponentKind as Component;
    use WorldMutation::*;
    match mutation {
        Relocate { subject, .. } => vec![(subject.clone(), Component::Occupancy, None)],
        TransferCustody { resource, .. } => vec![(resource.clone(), Component::Custody, None)],
        MutateResource {
            resource,
            related_resources,
            ..
        } => std::iter::once(resource)
            .chain(related_resources)
            .map(|resource| (resource.clone(), Component::ResourceState, None))
            .collect(),
        ChangeCapability {
            subject,
            capability_id,
            ..
        } => vec![(
            subject.clone(),
            Component::Capability,
            Some(capability_id.clone()),
        )],
        ChangeCondition {
            subject,
            condition_id,
            ..
        } => vec![(
            subject.clone(),
            Component::Condition,
            Some(condition_id.clone()),
        )],
        ChangeCommitment {
            subject,
            commitment_id,
            ..
        } => vec![(
            subject.clone(),
            Component::Commitment,
            Some(commitment_id.clone()),
        )],
        ChangeRelationship {
            source,
            relationship_id,
            ..
        } => vec![(
            source.clone(),
            Component::Relationship,
            Some(relationship_id.clone()),
        )],
        ChangePressure { pressure, .. } => vec![(pressure.clone(), Component::Pressure, None)],
        ChangeKnowledge {
            proposition,
            knower,
            recipients,
            ..
        } => knower
            .iter()
            .chain(recipients)
            .map(|subject| {
                (
                    subject.clone(),
                    Component::Knowledge,
                    Some(proposition.id.clone()),
                )
            })
            .collect(),
        ChangeMemory {
            subject, memory_id, ..
        } => vec![(subject.clone(), Component::Memory, Some(memory_id.clone()))],
        ChangePosture { subject, .. } => vec![(subject.clone(), Component::Posture, None)],
        ChangePopulationMembership { actor, .. } => {
            vec![(actor.clone(), Component::PopulationMembership, None)]
        }
        ChangePopulationLineage {
            parent_populations,
            child_populations,
            ..
        } => parent_populations
            .iter()
            .chain(child_populations)
            .map(|population| (population.clone(), Component::PopulationLineage, None))
            .collect(),
        ChangeIdentity {
            subject, handle_id, ..
        } => vec![(
            subject.clone(),
            Component::Identity,
            Some(handle_id.clone()),
        )],
        ChangeTopology {
            from_place,
            edge_id,
            ..
        } => vec![(
            from_place.clone(),
            Component::Topology,
            Some(edge_id.clone()),
        )],
        AdmitEntity { subject, .. } | RetireEntity { subject, .. } => {
            vec![(subject.clone(), Component::Lifecycle, None)]
        }
        AdvanceWorldTime { campaign, .. } => vec![(campaign.clone(), Component::WorldTime, None)],
    }
}

pub fn validate_batch_structure(
    envelope: &MutationAuthorityEnvelope,
    batch: &WorldMutationBatch,
    now: DateTime<Utc>,
) -> Result<()> {
    if envelope.schema != "ghostlight.mutation_authority_envelope.v1"
        || batch.schema != "ghostlight.world_mutation_batch.v1"
    {
        return Err(anyhow!("mutation document uses an unsupported schema"));
    }
    if envelope.digest != envelope_digest(envelope)? {
        return Err(anyhow!("mutation authority envelope digest is invalid"));
    }
    if batch.digest != mutation_batch_digest(batch)? {
        return Err(anyhow!("world mutation batch digest is invalid"));
    }
    if envelope.campaign_id != batch.campaign_id
        || envelope.world_revision != batch.expected_world_revision
        || envelope.resolution_epoch != batch.expected_resolution_epoch
        || envelope.digest != batch.authority_envelope_digest
    {
        return Err(anyhow!(
            "world mutation batch is not bound to its authority snapshot"
        ));
    }
    if envelope.expires_at < now {
        return Err(anyhow!("mutation authority envelope is expired"));
    }
    if batch.id.trim().is_empty() || batch.source_receipt_id.trim().is_empty() {
        return Err(anyhow!(
            "world mutation batch lacks exact identity or source"
        ));
    }

    let permits = envelope
        .permits
        .iter()
        .map(|permit| (permit.id.as_str(), permit))
        .collect::<BTreeMap<_, _>>();
    if permits.len() != envelope.permits.len() {
        return Err(anyhow!("mutation authority envelope repeats a permit id"));
    }
    let mut uses = BTreeMap::<&str, u16>::new();
    for proposed in &batch.mutations {
        let permit = permits
            .get(proposed.permit_id.as_str())
            .ok_or_else(|| anyhow!("world mutation cites an unknown permit"))?;
        let count = uses.entry(proposed.permit_id.as_str()).or_default();
        *count = count.saturating_add(1);
        if *count > permit.maximum_uses {
            return Err(anyhow!("world mutation exceeds permit use count"));
        }
        validate_permitted_mutation(permit, &proposed.mutation)?;
    }
    Ok(())
}

pub fn mutation_batch_json_schema(
    envelope: &MutationAuthorityEnvelope,
) -> Result<serde_json::Value> {
    let mut schema = serde_json::to_value(schema_for!(WorldMutationBatch))?;
    let variants = schema
        .pointer("/$defs/WorldMutation/oneOf")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .ok_or_else(|| anyhow!("world mutation schema omitted tagged variants"))?;
    let branches = envelope
        .permits
        .iter()
        .map(|permit| permit_schema_branch(permit, &variants))
        .collect::<Result<Vec<_>>>()?;
    let mutation_items = schema
        .pointer_mut("/$defs/PermittedWorldMutation")
        .ok_or_else(|| anyhow!("world mutation schema omitted permitted mutation definition"))?;
    *mutation_items = serde_json::json!({"oneOf": branches});
    schema
        .pointer_mut("/$defs")
        .and_then(serde_json::Value::as_object_mut)
        .expect("schemars emits an object definition table")
        .remove("WorldMutation");
    Ok(schema)
}

fn permit_schema_branch(
    permit: &MutationPermit,
    variants: &[serde_json::Value],
) -> Result<serde_json::Value> {
    let (mutation_type, operation) = mutation_schema_tags(permit.operation);
    let mut mutation = variants
        .iter()
        .find(|variant| {
            variant
                .pointer("/properties/type/const")
                .and_then(serde_json::Value::as_str)
                == Some(mutation_type)
        })
        .cloned()
        .ok_or_else(|| anyhow!("world mutation schema omitted {mutation_type}"))?;
    if let Some(operation) = operation {
        mutation["properties"]["operation"] = serde_json::json!({"const": operation});
    }
    constrain_permit_schema(&mut mutation, permit)?;
    Ok(serde_json::json!({
        "type":"object",
        "additionalProperties":false,
        "required":["permit_id","mutation"],
        "properties":{
            "permit_id":{"const":permit.id},
            "mutation":mutation
        }
    }))
}

#[derive(Clone, Copy)]
enum FieldShape {
    Required,
    Optional,
    Array,
}

fn constrain_permit_schema(
    mutation: &mut serde_json::Value,
    permit: &MutationPermit,
) -> Result<()> {
    for (role, field, shape) in subject_schema_fields(permit.operation) {
        let allowed = permit
            .subject_bindings
            .iter()
            .find(|binding| binding.role == role)
            .map(|binding| &binding.allowed_subjects);
        constrain_subject_field(mutation, field, shape, allowed)?;
    }
    for (role, field, shape) in string_schema_fields(permit.operation) {
        let constraint = permit.string_constraints.get(&role);
        constrain_string_field(mutation, field, shape, constraint)?;
    }
    for (role, field, shape) in integer_schema_fields(permit.operation) {
        let bounds = permit.integer_bounds.get(&role);
        constrain_integer_field(mutation, field, shape, bounds)?;
    }
    Ok(())
}

fn constrain_subject_field(
    mutation: &mut serde_json::Value,
    field: &str,
    shape: FieldShape,
    allowed: Option<&BTreeSet<SubjectRef>>,
) -> Result<()> {
    let target = mutation
        .pointer_mut(&format!("/properties/{field}"))
        .ok_or_else(|| anyhow!("mutation schema omitted subject field {field}"))?;
    let values = allowed
        .into_iter()
        .flatten()
        .map(serde_json::to_value)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    *target = match shape {
        FieldShape::Required if values.is_empty() => {
            return Err(anyhow!("permit omitted required subject role for {field}"));
        }
        FieldShape::Required => serde_json::json!({"enum":values}),
        FieldShape::Optional if values.is_empty() => serde_json::json!({"type":"null"}),
        FieldShape::Optional => {
            serde_json::json!({"anyOf":[{"enum":values},{"type":"null"}]})
        }
        FieldShape::Array if values.is_empty() => {
            serde_json::json!({"type":"array","maxItems":0})
        }
        FieldShape::Array => serde_json::json!({
            "type":"array","items":{"enum":values},"uniqueItems":true
        }),
    };
    Ok(())
}

fn constrain_string_field(
    mutation: &mut serde_json::Value,
    field: &str,
    shape: FieldShape,
    constraint: Option<&StringConstraint>,
) -> Result<()> {
    let target = mutation
        .pointer_mut(&format!("/properties/{field}"))
        .ok_or_else(|| anyhow!("mutation schema omitted string field {field}"))?;
    let Some(constraint) = constraint else {
        return match shape {
            FieldShape::Required => Err(anyhow!("permit omitted required string role for {field}")),
            FieldShape::Optional => {
                *target = serde_json::json!({"type":"null"});
                Ok(())
            }
            FieldShape::Array => Err(anyhow!("string arrays are not supported in permits")),
        };
    };
    let mut string = serde_json::json!({
        "type":"string",
        "minLength":constraint.minimum_length,
        "maxLength":constraint.maximum_length
    });
    if !constraint.allowed_values.is_empty() {
        string["enum"] = serde_json::to_value(&constraint.allowed_values)?;
    }
    *target = match shape {
        FieldShape::Required => string,
        FieldShape::Optional => serde_json::json!({"anyOf":[string,{"type":"null"}]}),
        FieldShape::Array => return Err(anyhow!("string arrays are not supported in permits")),
    };
    Ok(())
}

fn constrain_integer_field(
    mutation: &mut serde_json::Value,
    field: &str,
    shape: FieldShape,
    bounds: Option<&IntegerBounds>,
) -> Result<()> {
    let target = mutation
        .pointer_mut(&format!("/properties/{field}"))
        .ok_or_else(|| anyhow!("mutation schema omitted integer field {field}"))?;
    let Some(bounds) = bounds else {
        return match shape {
            FieldShape::Required => {
                Err(anyhow!("permit omitted required integer role for {field}"))
            }
            FieldShape::Optional => {
                *target = serde_json::json!({"type":"null"});
                Ok(())
            }
            FieldShape::Array => Err(anyhow!("integer arrays are not supported in permits")),
        };
    };
    let integer = serde_json::json!({
        "type":"integer","minimum":bounds.minimum,"maximum":bounds.maximum
    });
    *target = match shape {
        FieldShape::Required => integer,
        FieldShape::Optional => serde_json::json!({"anyOf":[integer,{"type":"null"}]}),
        FieldShape::Array => return Err(anyhow!("integer arrays are not supported in permits")),
    };
    Ok(())
}

fn subject_schema_fields(
    operation: WorldMutationOperation,
) -> Vec<(MutationSubjectRole, &'static str, FieldShape)> {
    use FieldShape::{Array, Optional, Required};
    use MutationSubjectRole::*;
    use WorldMutationOperation::*;
    match operation {
        Relocate => vec![
            (Subject, "subject", Required),
            (OriginPlace, "from_place", Required),
            (DestinationPlace, "to_place", Required),
        ],
        TransferCustody => vec![
            (Resource, "resource", Required),
            (SourceCustodian, "from_custodian", Required),
            (DestinationCustodian, "to_custodian", Required),
        ],
        ResourceCreate => vec![
            (Resource, "resource", Required),
            (Owner, "custodian", Required),
            (RelatedResource, "related_resources", Array),
        ],
        ResourceTransform | ResourceConsume | ResourceDamage | ResourceRepair | ResourceSplit
        | ResourceCombine => vec![
            (Resource, "resource", Required),
            (Owner, "custodian", Optional),
            (RelatedResource, "related_resources", Array),
        ],
        CapabilityGrant | CapabilityAlter | CapabilitySuspend | CapabilityRetire
        | ConditionApply | ConditionAlter | ConditionClear | MemoryRecord | MemoryRevise
        | MemoryRetire | PostureChange => vec![(Subject, "subject", Required)],
        CommitmentCreate | CommitmentAlter | CommitmentFulfill | CommitmentDefault
        | CommitmentRetire => vec![
            (Subject, "subject", Required),
            (Counterparty, "counterparty", Optional),
        ],
        RelationshipCreate | RelationshipAlter | RelationshipRetire => vec![
            (RelationshipSource, "source", Required),
            (RelationshipTarget, "target", Required),
        ],
        PressureCreate | PressureAdvance | PressureReduce | PressureResolve | PressureRetire => {
            vec![(Pressure, "pressure", Required), (Owner, "owner", Required)]
        }
        KnowledgeAcquire | KnowledgeConceal | KnowledgeCorrect | KnowledgeInvalidate => vec![
            (Proposition, "proposition", Required),
            (Knower, "knower", Required),
            (Speaker, "speaker", Optional),
            (Recipient, "recipients", Array),
            (Channel, "channel", Optional),
        ],
        KnowledgeCommunicate => vec![
            (Proposition, "proposition", Required),
            (Knower, "knower", Optional),
            (Speaker, "speaker", Required),
            (Recipient, "recipients", Array),
            (Channel, "channel", Optional),
        ],
        PopulationJoin | PopulationLeave | PopulationTransfer => vec![
            (Actor, "actor", Required),
            (SourcePopulation, "source_population", Optional),
            (DestinationPopulation, "destination_population", Optional),
        ],
        PopulationSplit | PopulationMerge => vec![
            (ParentPopulation, "parent_populations", Array),
            (ChildPopulation, "child_populations", Array),
            (ChildPopulation, "remainder_population", Optional),
        ],
        IdentityAdopt | IdentityDisclose | IdentityRestrict | IdentityRetire => vec![
            (Subject, "subject", Required),
            (Recipient, "audience", Array),
        ],
        TopologyAdd | TopologyAlter | TopologyOpen | TopologyClose | TopologyRetire => vec![
            (TopologyOrigin, "from_place", Required),
            (TopologyDestination, "to_place", Required),
        ],
        AdmitEntity | RetireEntity => vec![(Entity, "subject", Required)],
        AdvanceWorldTime => vec![(Subject, "campaign", Required)],
    }
}

fn string_schema_fields(
    operation: WorldMutationOperation,
) -> Vec<(MutationStringRole, &'static str, FieldShape)> {
    use FieldShape::{Optional, Required};
    use MutationStringRole::*;
    use WorldMutationOperation::*;
    match operation {
        Relocate => vec![(RouteId, "route_id", Required)],
        ResourceCreate => vec![
            (ResourceKind, "resource_kind", Required),
            (ResourceLabel, "resource_label", Required),
            (RecipeId, "recipe_id", Optional),
        ],
        ResourceTransform | ResourceSplit | ResourceCombine => vec![
            (ResourceKind, "resource_kind", Optional),
            (ResourceLabel, "resource_label", Optional),
            (RecipeId, "recipe_id", Required),
        ],
        ResourceConsume | ResourceDamage | ResourceRepair => vec![
            (ResourceKind, "resource_kind", Optional),
            (ResourceLabel, "resource_label", Optional),
            (RecipeId, "recipe_id", Optional),
        ],
        CapabilityGrant | CapabilityAlter | CapabilitySuspend | CapabilityRetire => vec![
            (CapabilityId, "capability_id", Required),
            (Description, "description", Optional),
        ],
        ConditionApply | ConditionAlter | ConditionClear => vec![
            (ConditionId, "condition_id", Required),
            (Description, "description", Optional),
        ],
        CommitmentCreate | CommitmentAlter | CommitmentFulfill | CommitmentDefault
        | CommitmentRetire => vec![
            (CommitmentId, "commitment_id", Required),
            (Description, "description", Optional),
        ],
        RelationshipCreate | RelationshipAlter | RelationshipRetire => vec![
            (RelationshipId, "relationship_id", Required),
            (Description, "description", Optional),
        ],
        PressureCreate => vec![(PressureLabel, "label", Required)],
        PressureAdvance | PressureReduce | PressureResolve | PressureRetire => {
            vec![(PressureLabel, "label", Optional)]
        }
        MemoryRecord | MemoryRevise | MemoryRetire => vec![
            (MemoryId, "memory_id", Required),
            (EventId, "event_id", Optional),
            (Summary, "summary", Optional),
        ],
        PostureChange => vec![(Posture, "posture", Required)],
        IdentityAdopt => vec![
            (IdentityHandleId, "handle_id", Required),
            (IdentityHandleValue, "handle_value", Required),
        ],
        IdentityDisclose | IdentityRestrict | IdentityRetire => vec![
            (IdentityHandleId, "handle_id", Required),
            (IdentityHandleValue, "handle_value", Optional),
        ],
        TopologyAdd | TopologyAlter | TopologyOpen | TopologyClose | TopologyRetire => {
            vec![(TopologyEdgeId, "edge_id", Required)]
        }
        AdmitEntity => vec![(AdmissionReceiptId, "admission_receipt_id", Required)],
        RetireEntity => vec![(RetirementReason, "reason", Required)],
        TransferCustody | KnowledgeAcquire | KnowledgeCommunicate | KnowledgeConceal
        | KnowledgeCorrect | KnowledgeInvalidate | PopulationJoin | PopulationLeave
        | PopulationTransfer | PopulationSplit | PopulationMerge | AdvanceWorldTime => vec![],
    }
}

fn integer_schema_fields(
    operation: WorldMutationOperation,
) -> Vec<(MutationIntegerRole, &'static str, FieldShape)> {
    use FieldShape::{Optional, Required};
    use MutationIntegerRole::*;
    use WorldMutationOperation::*;
    match operation {
        ResourceCreate | ResourceConsume | ResourceSplit => vec![
            (Quantity, "quantity", Required),
            (Integrity, "integrity", Optional),
        ],
        ResourceDamage | ResourceRepair => vec![
            (Quantity, "quantity", Optional),
            (Integrity, "integrity", Required),
        ],
        ResourceTransform | ResourceCombine => vec![
            (Quantity, "quantity", Optional),
            (Integrity, "integrity", Optional),
        ],
        ConditionApply | ConditionAlter | ConditionClear => {
            vec![(Severity, "severity", Optional)]
        }
        RelationshipCreate | RelationshipAlter => {
            vec![(RelationshipStrengthDelta, "strength_delta", Optional)]
        }
        PressureAdvance | PressureReduce => vec![(PressureAmount, "amount", Required)],
        PressureCreate | PressureResolve | PressureRetire => {
            vec![(PressureAmount, "amount", Optional)]
        }
        TopologyAdd | TopologyAlter => vec![(TravelMinutes, "travel_minutes", Required)],
        TopologyOpen | TopologyClose | TopologyRetire => {
            vec![(TravelMinutes, "travel_minutes", Optional)]
        }
        AdvanceWorldTime => vec![(WorldMinutes, "minutes", Required)],
        Relocate | TransferCustody | CapabilityGrant | CapabilityAlter | CapabilitySuspend
        | CapabilityRetire | CommitmentCreate | CommitmentAlter | CommitmentFulfill
        | CommitmentDefault | CommitmentRetire | RelationshipRetire | KnowledgeAcquire
        | KnowledgeCommunicate | KnowledgeConceal | KnowledgeCorrect | KnowledgeInvalidate
        | MemoryRecord | MemoryRevise | MemoryRetire | PostureChange | PopulationJoin
        | PopulationLeave | PopulationTransfer | PopulationSplit | PopulationMerge
        | IdentityAdopt | IdentityDisclose | IdentityRestrict | IdentityRetire | AdmitEntity
        | RetireEntity => vec![],
    }
}

fn mutation_schema_tags(operation: WorldMutationOperation) -> (&'static str, Option<&'static str>) {
    use WorldMutationOperation::*;
    match operation {
        Relocate => ("relocate", None),
        TransferCustody => ("transfer_custody", None),
        ResourceCreate => ("mutate_resource", Some("create")),
        ResourceTransform => ("mutate_resource", Some("transform")),
        ResourceConsume => ("mutate_resource", Some("consume")),
        ResourceDamage => ("mutate_resource", Some("damage")),
        ResourceRepair => ("mutate_resource", Some("repair")),
        ResourceSplit => ("mutate_resource", Some("split")),
        ResourceCombine => ("mutate_resource", Some("combine")),
        CapabilityGrant => ("change_capability", Some("grant")),
        CapabilityAlter => ("change_capability", Some("alter")),
        CapabilitySuspend => ("change_capability", Some("suspend")),
        CapabilityRetire => ("change_capability", Some("retire")),
        ConditionApply => ("change_condition", Some("apply")),
        ConditionAlter => ("change_condition", Some("alter")),
        ConditionClear => ("change_condition", Some("clear")),
        CommitmentCreate => ("change_commitment", Some("create")),
        CommitmentAlter => ("change_commitment", Some("alter")),
        CommitmentFulfill => ("change_commitment", Some("fulfill")),
        CommitmentDefault => ("change_commitment", Some("default")),
        CommitmentRetire => ("change_commitment", Some("retire")),
        RelationshipCreate => ("change_relationship", Some("create")),
        RelationshipAlter => ("change_relationship", Some("alter")),
        RelationshipRetire => ("change_relationship", Some("retire")),
        PressureCreate => ("change_pressure", Some("create")),
        PressureAdvance => ("change_pressure", Some("advance")),
        PressureReduce => ("change_pressure", Some("reduce")),
        PressureResolve => ("change_pressure", Some("resolve")),
        PressureRetire => ("change_pressure", Some("retire")),
        KnowledgeAcquire => ("change_knowledge", Some("acquire")),
        KnowledgeCommunicate => ("change_knowledge", Some("communicate")),
        KnowledgeConceal => ("change_knowledge", Some("conceal")),
        KnowledgeCorrect => ("change_knowledge", Some("correct")),
        KnowledgeInvalidate => ("change_knowledge", Some("invalidate")),
        MemoryRecord => ("change_memory", Some("record")),
        MemoryRevise => ("change_memory", Some("revise")),
        MemoryRetire => ("change_memory", Some("retire")),
        PostureChange => ("change_posture", None),
        PopulationJoin => ("change_population_membership", Some("join")),
        PopulationLeave => ("change_population_membership", Some("leave")),
        PopulationTransfer => ("change_population_membership", Some("transfer")),
        PopulationSplit => ("change_population_lineage", Some("split")),
        PopulationMerge => ("change_population_lineage", Some("merge")),
        IdentityAdopt => ("change_identity", Some("adopt")),
        IdentityDisclose => ("change_identity", Some("disclose")),
        IdentityRestrict => ("change_identity", Some("restrict")),
        IdentityRetire => ("change_identity", Some("retire")),
        TopologyAdd => ("change_topology", Some("add")),
        TopologyAlter => ("change_topology", Some("alter")),
        TopologyOpen => ("change_topology", Some("open")),
        TopologyClose => ("change_topology", Some("close")),
        TopologyRetire => ("change_topology", Some("retire")),
        AdmitEntity => ("admit_entity", None),
        RetireEntity => ("retire_entity", None),
        AdvanceWorldTime => ("advance_world_time", None),
    }
}

fn validate_permitted_mutation(permit: &MutationPermit, mutation: &WorldMutation) -> Result<()> {
    if mutation.operation() != permit.operation {
        return Err(anyhow!(
            "world mutation operation does not match its permit"
        ));
    }
    let roles = mutation.subject_roles();
    for binding in &permit.subject_bindings {
        let actual = roles.get(&binding.role).cloned().unwrap_or_default();
        if actual.is_empty() || !actual.is_subset(&binding.allowed_subjects) {
            return Err(anyhow!(
                "world mutation exceeds the subjects allowed for role {:?}",
                binding.role
            ));
        }
    }
    for (role, actual) in &roles {
        if !permit
            .subject_bindings
            .iter()
            .any(|binding| binding.role == *role && actual.is_subset(&binding.allowed_subjects))
        {
            return Err(anyhow!(
                "world mutation uses an unbound subject role {:?}",
                role
            ));
        }
    }
    for (role, value) in mutation.string_roles() {
        let constraint = permit.string_constraints.get(&role).ok_or_else(|| {
            anyhow!(
                "world mutation string for role {:?} has no permit constraint",
                role
            )
        })?;
        let length = value.chars().count();
        if length < usize::from(constraint.minimum_length)
            || length > usize::from(constraint.maximum_length)
            || (!constraint.allowed_values.is_empty()
                && !constraint.allowed_values.contains(&value))
        {
            return Err(anyhow!(
                "world mutation string for role {:?} is not permitted",
                role
            ));
        }
    }
    for (role, value) in mutation.integer_roles() {
        let bounds = permit.integer_bounds.get(&role).ok_or_else(|| {
            anyhow!(
                "world mutation integer for role {:?} has no permit bounds",
                role
            )
        })?;
        if value < bounds.minimum || value > bounds.maximum {
            return Err(anyhow!(
                "world mutation integer for role {:?} is outside permit bounds",
                role
            ));
        }
    }
    mutation.validate_local_shape()
}

impl WorldMutation {
    pub fn operation(&self) -> WorldMutationOperation {
        use WorldMutation::*;
        match self {
            Relocate { .. } => WorldMutationOperation::Relocate,
            TransferCustody { .. } => WorldMutationOperation::TransferCustody,
            MutateResource { operation, .. } => match operation {
                ResourceMutationOperation::Create => WorldMutationOperation::ResourceCreate,
                ResourceMutationOperation::Transform => WorldMutationOperation::ResourceTransform,
                ResourceMutationOperation::Consume => WorldMutationOperation::ResourceConsume,
                ResourceMutationOperation::Damage => WorldMutationOperation::ResourceDamage,
                ResourceMutationOperation::Repair => WorldMutationOperation::ResourceRepair,
                ResourceMutationOperation::Split => WorldMutationOperation::ResourceSplit,
                ResourceMutationOperation::Combine => WorldMutationOperation::ResourceCombine,
            },
            ChangeCapability { operation, .. } => match operation {
                CapabilityMutationOperation::Grant => WorldMutationOperation::CapabilityGrant,
                CapabilityMutationOperation::Alter => WorldMutationOperation::CapabilityAlter,
                CapabilityMutationOperation::Suspend => WorldMutationOperation::CapabilitySuspend,
                CapabilityMutationOperation::Retire => WorldMutationOperation::CapabilityRetire,
            },
            ChangeCondition { operation, .. } => match operation {
                ConditionMutationOperation::Apply => WorldMutationOperation::ConditionApply,
                ConditionMutationOperation::Alter => WorldMutationOperation::ConditionAlter,
                ConditionMutationOperation::Clear => WorldMutationOperation::ConditionClear,
            },
            ChangeCommitment { operation, .. } => match operation {
                CommitmentMutationOperation::Create => WorldMutationOperation::CommitmentCreate,
                CommitmentMutationOperation::Alter => WorldMutationOperation::CommitmentAlter,
                CommitmentMutationOperation::Fulfill => WorldMutationOperation::CommitmentFulfill,
                CommitmentMutationOperation::Default => WorldMutationOperation::CommitmentDefault,
                CommitmentMutationOperation::Retire => WorldMutationOperation::CommitmentRetire,
            },
            ChangeRelationship { operation, .. } => match operation {
                RelationshipMutationOperation::Create => WorldMutationOperation::RelationshipCreate,
                RelationshipMutationOperation::Alter => WorldMutationOperation::RelationshipAlter,
                RelationshipMutationOperation::Retire => WorldMutationOperation::RelationshipRetire,
            },
            ChangePressure { operation, .. } => match operation {
                PressureMutationOperation::Create => WorldMutationOperation::PressureCreate,
                PressureMutationOperation::Advance => WorldMutationOperation::PressureAdvance,
                PressureMutationOperation::Reduce => WorldMutationOperation::PressureReduce,
                PressureMutationOperation::Resolve => WorldMutationOperation::PressureResolve,
                PressureMutationOperation::Retire => WorldMutationOperation::PressureRetire,
            },
            ChangeKnowledge { operation, .. } => match operation {
                KnowledgeMutationOperation::Acquire => WorldMutationOperation::KnowledgeAcquire,
                KnowledgeMutationOperation::Communicate => {
                    WorldMutationOperation::KnowledgeCommunicate
                }
                KnowledgeMutationOperation::Conceal => WorldMutationOperation::KnowledgeConceal,
                KnowledgeMutationOperation::Correct => WorldMutationOperation::KnowledgeCorrect,
                KnowledgeMutationOperation::Invalidate => {
                    WorldMutationOperation::KnowledgeInvalidate
                }
            },
            ChangeMemory { operation, .. } => match operation {
                MemoryMutationOperation::Record => WorldMutationOperation::MemoryRecord,
                MemoryMutationOperation::Revise => WorldMutationOperation::MemoryRevise,
                MemoryMutationOperation::Retire => WorldMutationOperation::MemoryRetire,
            },
            ChangePosture { .. } => WorldMutationOperation::PostureChange,
            ChangePopulationMembership { operation, .. } => match operation {
                PopulationMembershipOperation::Join => WorldMutationOperation::PopulationJoin,
                PopulationMembershipOperation::Leave => WorldMutationOperation::PopulationLeave,
                PopulationMembershipOperation::Transfer => {
                    WorldMutationOperation::PopulationTransfer
                }
            },
            ChangePopulationLineage { operation, .. } => match operation {
                PopulationLineageOperation::Split => WorldMutationOperation::PopulationSplit,
                PopulationLineageOperation::Merge => WorldMutationOperation::PopulationMerge,
            },
            ChangeIdentity { operation, .. } => match operation {
                IdentityMutationOperation::Adopt => WorldMutationOperation::IdentityAdopt,
                IdentityMutationOperation::Disclose => WorldMutationOperation::IdentityDisclose,
                IdentityMutationOperation::Restrict => WorldMutationOperation::IdentityRestrict,
                IdentityMutationOperation::Retire => WorldMutationOperation::IdentityRetire,
            },
            ChangeTopology { operation, .. } => match operation {
                TopologyMutationOperation::Add => WorldMutationOperation::TopologyAdd,
                TopologyMutationOperation::Alter => WorldMutationOperation::TopologyAlter,
                TopologyMutationOperation::Open => WorldMutationOperation::TopologyOpen,
                TopologyMutationOperation::Close => WorldMutationOperation::TopologyClose,
                TopologyMutationOperation::Retire => WorldMutationOperation::TopologyRetire,
            },
            AdmitEntity { .. } => WorldMutationOperation::AdmitEntity,
            RetireEntity { .. } => WorldMutationOperation::RetireEntity,
            AdvanceWorldTime { .. } => WorldMutationOperation::AdvanceWorldTime,
        }
    }

    fn subject_roles(&self) -> BTreeMap<MutationSubjectRole, BTreeSet<SubjectRef>> {
        let mut roles = BTreeMap::<MutationSubjectRole, BTreeSet<SubjectRef>>::new();
        let mut add = |role, subject: &SubjectRef| {
            roles.entry(role).or_default().insert(subject.clone());
        };
        use WorldMutation::*;
        match self {
            Relocate {
                subject,
                from_place,
                to_place,
                ..
            } => {
                add(MutationSubjectRole::Subject, subject);
                add(MutationSubjectRole::OriginPlace, from_place);
                add(MutationSubjectRole::DestinationPlace, to_place);
            }
            TransferCustody {
                resource,
                from_custodian,
                to_custodian,
            } => {
                add(MutationSubjectRole::Resource, resource);
                add(MutationSubjectRole::SourceCustodian, from_custodian);
                add(MutationSubjectRole::DestinationCustodian, to_custodian);
            }
            MutateResource {
                resource,
                custodian,
                related_resources,
                ..
            } => {
                add(MutationSubjectRole::Resource, resource);
                if let Some(custodian) = custodian {
                    add(MutationSubjectRole::Owner, custodian);
                }
                for related in related_resources {
                    add(MutationSubjectRole::RelatedResource, related);
                }
            }
            ChangeCapability { subject, .. }
            | ChangeCondition { subject, .. }
            | ChangeMemory { subject, .. }
            | ChangePosture { subject, .. } => add(MutationSubjectRole::Subject, subject),
            ChangeCommitment {
                subject,
                counterparty,
                ..
            } => {
                add(MutationSubjectRole::Subject, subject);
                if let Some(counterparty) = counterparty {
                    add(MutationSubjectRole::Counterparty, counterparty);
                }
            }
            ChangeRelationship { source, target, .. } => {
                add(MutationSubjectRole::RelationshipSource, source);
                add(MutationSubjectRole::RelationshipTarget, target);
            }
            ChangePressure {
                pressure, owner, ..
            } => {
                add(MutationSubjectRole::Pressure, pressure);
                add(MutationSubjectRole::Owner, owner);
            }
            ChangeKnowledge {
                proposition,
                knower,
                speaker,
                recipients,
                channel,
                ..
            } => {
                add(MutationSubjectRole::Proposition, proposition);
                if let Some(knower) = knower {
                    add(MutationSubjectRole::Knower, knower);
                }
                if let Some(speaker) = speaker {
                    add(MutationSubjectRole::Speaker, speaker);
                }
                for recipient in recipients {
                    add(MutationSubjectRole::Recipient, recipient);
                }
                if let Some(channel) = channel {
                    add(MutationSubjectRole::Channel, channel);
                }
            }
            ChangePopulationMembership {
                actor,
                source_population,
                destination_population,
                ..
            } => {
                add(MutationSubjectRole::Actor, actor);
                if let Some(population) = source_population {
                    add(MutationSubjectRole::SourcePopulation, population);
                }
                if let Some(population) = destination_population {
                    add(MutationSubjectRole::DestinationPopulation, population);
                }
            }
            ChangePopulationLineage {
                parent_populations,
                child_populations,
                remainder_population,
                ..
            } => {
                for population in parent_populations {
                    add(MutationSubjectRole::ParentPopulation, population);
                }
                for population in child_populations {
                    add(MutationSubjectRole::ChildPopulation, population);
                }
                if let Some(population) = remainder_population {
                    add(MutationSubjectRole::ChildPopulation, population);
                }
            }
            ChangeIdentity {
                subject, audience, ..
            } => {
                add(MutationSubjectRole::Subject, subject);
                for recipient in audience {
                    add(MutationSubjectRole::Recipient, recipient);
                }
            }
            ChangeTopology {
                from_place,
                to_place,
                ..
            } => {
                add(MutationSubjectRole::TopologyOrigin, from_place);
                add(MutationSubjectRole::TopologyDestination, to_place);
            }
            AdmitEntity { subject, .. } | RetireEntity { subject, .. } => {
                add(MutationSubjectRole::Entity, subject)
            }
            AdvanceWorldTime { campaign, .. } => add(MutationSubjectRole::Subject, campaign),
        }
        roles
    }

    fn string_roles(&self) -> Vec<(MutationStringRole, String)> {
        use WorldMutation::*;
        let mut values = Vec::new();
        match self {
            Relocate { route_id, .. } => {
                values.push((MutationStringRole::RouteId, route_id.clone()))
            }
            MutateResource {
                resource_kind,
                resource_label,
                recipe_id,
                ..
            } => {
                if let Some(value) = resource_kind {
                    values.push((MutationStringRole::ResourceKind, value.clone()));
                }
                if let Some(value) = resource_label {
                    values.push((MutationStringRole::ResourceLabel, value.clone()));
                }
                if let Some(value) = recipe_id {
                    values.push((MutationStringRole::RecipeId, value.clone()));
                }
            }
            ChangeCapability {
                capability_id,
                description,
                ..
            } => {
                values.push((MutationStringRole::CapabilityId, capability_id.clone()));
                if let Some(value) = description {
                    values.push((MutationStringRole::Description, value.clone()));
                }
            }
            ChangeCondition {
                condition_id,
                description,
                ..
            } => {
                values.push((MutationStringRole::ConditionId, condition_id.clone()));
                if let Some(value) = description {
                    values.push((MutationStringRole::Description, value.clone()));
                }
            }
            ChangeCommitment {
                commitment_id,
                description,
                ..
            } => {
                values.push((MutationStringRole::CommitmentId, commitment_id.clone()));
                if let Some(value) = description {
                    values.push((MutationStringRole::Description, value.clone()));
                }
            }
            ChangeRelationship {
                relationship_id,
                description,
                ..
            } => {
                values.push((MutationStringRole::RelationshipId, relationship_id.clone()));
                if let Some(value) = description {
                    values.push((MutationStringRole::Description, value.clone()));
                }
            }
            ChangePressure { label, .. } => {
                if let Some(value) = label {
                    values.push((MutationStringRole::PressureLabel, value.clone()));
                }
            }
            ChangeMemory {
                memory_id,
                event_id,
                summary,
                ..
            } => {
                values.push((MutationStringRole::MemoryId, memory_id.clone()));
                if let Some(value) = event_id {
                    values.push((MutationStringRole::EventId, value.clone()));
                }
                if let Some(value) = summary {
                    values.push((MutationStringRole::Summary, value.clone()));
                }
            }
            ChangePosture { posture, .. } => {
                values.push((MutationStringRole::Posture, posture.clone()))
            }
            ChangeIdentity {
                handle_id,
                handle_value,
                ..
            } => {
                values.push((MutationStringRole::IdentityHandleId, handle_id.clone()));
                if let Some(value) = handle_value {
                    values.push((MutationStringRole::IdentityHandleValue, value.clone()));
                }
            }
            ChangeTopology { edge_id, .. } => {
                values.push((MutationStringRole::TopologyEdgeId, edge_id.clone()))
            }
            AdmitEntity {
                admission_receipt_id,
                ..
            } => values.push((
                MutationStringRole::AdmissionReceiptId,
                admission_receipt_id.clone(),
            )),
            RetireEntity { reason, .. } => {
                values.push((MutationStringRole::RetirementReason, reason.clone()))
            }
            TransferCustody { .. }
            | ChangeKnowledge { .. }
            | ChangePopulationMembership { .. }
            | ChangePopulationLineage { .. }
            | AdvanceWorldTime { .. } => {}
        }
        values
    }

    fn integer_roles(&self) -> Vec<(MutationIntegerRole, i64)> {
        use WorldMutation::*;
        let mut values = Vec::new();
        match self {
            MutateResource {
                quantity,
                integrity,
                ..
            } => {
                if let Some(value) = quantity {
                    values.push((MutationIntegerRole::Quantity, *value));
                }
                if let Some(value) = integrity {
                    values.push((MutationIntegerRole::Integrity, *value));
                }
            }
            ChangeCondition { severity, .. } => {
                if let Some(value) = severity {
                    values.push((MutationIntegerRole::Severity, *value));
                }
            }
            ChangeRelationship { strength_delta, .. } => {
                if let Some(value) = strength_delta {
                    values.push((MutationIntegerRole::RelationshipStrengthDelta, *value));
                }
            }
            ChangePressure { amount, .. } => {
                if let Some(value) = amount {
                    values.push((MutationIntegerRole::PressureAmount, *value));
                }
            }
            ChangeTopology { travel_minutes, .. } => {
                if let Some(value) = travel_minutes {
                    values.push((MutationIntegerRole::TravelMinutes, *value));
                }
            }
            AdvanceWorldTime { minutes, .. } => {
                values.push((MutationIntegerRole::WorldMinutes, *minutes));
            }
            Relocate { .. }
            | TransferCustody { .. }
            | ChangeCapability { .. }
            | ChangeCommitment { .. }
            | ChangeKnowledge { .. }
            | ChangeMemory { .. }
            | ChangePosture { .. }
            | ChangePopulationMembership { .. }
            | ChangePopulationLineage { .. }
            | ChangeIdentity { .. }
            | AdmitEntity { .. }
            | RetireEntity { .. } => {}
        }
        values
    }

    fn validate_local_shape(&self) -> Result<()> {
        let nonempty = |label: &str, value: &str| -> Result<()> {
            if value.trim().is_empty() {
                Err(anyhow!("world mutation {label} is empty"))
            } else {
                Ok(())
            }
        };
        for subject in self.subject_roles().values().flatten() {
            nonempty("subject id", &subject.id)?;
        }
        use WorldMutation::*;
        match self {
            Relocate { route_id, .. } => nonempty("route id", route_id)?,
            MutateResource {
                operation,
                custodian,
                related_resources,
                resource_label,
                quantity,
                integrity,
                ..
            } => match operation {
                ResourceMutationOperation::Create
                | ResourceMutationOperation::Consume
                | ResourceMutationOperation::Split
                    if quantity.is_none() =>
                {
                    return Err(anyhow!("resource mutation requires an exact quantity"));
                }
                ResourceMutationOperation::Damage | ResourceMutationOperation::Repair
                    if integrity.is_none() =>
                {
                    return Err(anyhow!("resource mutation requires an integrity amount"));
                }
                ResourceMutationOperation::Transform
                | ResourceMutationOperation::Split
                | ResourceMutationOperation::Combine
                    if related_resources.is_empty() =>
                {
                    return Err(anyhow!("resource mutation requires related resources"));
                }
                ResourceMutationOperation::Create if custodian.is_none() => {
                    return Err(anyhow!("resource creation requires an exact custodian"));
                }
                ResourceMutationOperation::Create
                    if resource_label.as_deref().is_none_or(str::is_empty) =>
                {
                    return Err(anyhow!("resource creation requires an exact label"));
                }
                _ if custodian.is_none() => {
                    return Err(anyhow!(
                        "resource mutation requires exact custody authority"
                    ));
                }
                _ => {}
            },
            ChangeCapability { capability_id, .. } => nonempty("capability id", capability_id)?,
            ChangeCondition { condition_id, .. } => nonempty("condition id", condition_id)?,
            ChangeCommitment { commitment_id, .. } => nonempty("commitment id", commitment_id)?,
            ChangeRelationship {
                operation,
                relationship_id,
                description,
                strength_delta,
                ..
            } => {
                nonempty("relationship id", relationship_id)?;
                if matches!(operation, RelationshipMutationOperation::Create)
                    && description.as_deref().is_none_or(str::is_empty)
                {
                    return Err(anyhow!("relationship creation requires a description"));
                }
                if matches!(operation, RelationshipMutationOperation::Alter)
                    && description.is_none()
                    && strength_delta.is_none()
                {
                    return Err(anyhow!("relationship alteration is empty"));
                }
            }
            ChangePressure {
                operation, amount, ..
            } if matches!(
                operation,
                PressureMutationOperation::Advance | PressureMutationOperation::Reduce
            ) && amount.is_none() =>
            {
                return Err(anyhow!("pressure change requires an exact amount"));
            }
            ChangeKnowledge {
                operation,
                knower,
                speaker,
                recipients,
                ..
            } => match operation {
                KnowledgeMutationOperation::Acquire
                | KnowledgeMutationOperation::Conceal
                | KnowledgeMutationOperation::Correct
                | KnowledgeMutationOperation::Invalidate
                    if knower.is_none() =>
                {
                    return Err(anyhow!("knowledge mutation requires an exact knower"));
                }
                KnowledgeMutationOperation::Communicate
                    if speaker.is_none() || recipients.is_empty() =>
                {
                    return Err(anyhow!(
                        "knowledge communication requires a speaker and exact recipients"
                    ));
                }
                _ => {}
            },
            ChangeMemory { memory_id, .. } => nonempty("memory id", memory_id)?,
            ChangePosture { posture, .. } => nonempty("posture", posture)?,
            ChangePopulationMembership {
                operation,
                source_population,
                destination_population,
                ..
            } => match operation {
                PopulationMembershipOperation::Join if destination_population.is_none() => {
                    return Err(anyhow!("population join requires a destination"));
                }
                PopulationMembershipOperation::Leave if source_population.is_none() => {
                    return Err(anyhow!("population leave requires a source"));
                }
                PopulationMembershipOperation::Transfer
                    if source_population.is_none() || destination_population.is_none() =>
                {
                    return Err(anyhow!(
                        "population transfer requires source and destination"
                    ));
                }
                _ => {}
            },
            ChangePopulationLineage {
                operation,
                parent_populations,
                child_populations,
                remainder_population,
            } => match operation {
                PopulationLineageOperation::Split
                    if parent_populations.len() != 1
                        || child_populations.is_empty()
                        || remainder_population.is_none() =>
                {
                    return Err(anyhow!(
                        "population split requires one parent, children, and a remainder"
                    ));
                }
                PopulationLineageOperation::Merge
                    if parent_populations.len() < 2 || child_populations.len() != 1 =>
                {
                    return Err(anyhow!(
                        "population merge requires multiple parents and one child"
                    ));
                }
                _ => {}
            },
            ChangeIdentity {
                operation,
                handle_id,
                handle_value,
                audience,
                ..
            } => {
                nonempty("identity handle id", handle_id)?;
                if matches!(operation, IdentityMutationOperation::Adopt)
                    && handle_value.as_deref().is_none_or(str::is_empty)
                {
                    return Err(anyhow!("identity adoption requires a handle value"));
                }
                if matches!(
                    operation,
                    IdentityMutationOperation::Disclose | IdentityMutationOperation::Restrict
                ) && audience.is_empty()
                {
                    return Err(anyhow!(
                        "identity disclosure change requires an exact audience"
                    ));
                }
            }
            ChangeTopology {
                operation,
                edge_id,
                travel_minutes,
                ..
            } => {
                nonempty("topology edge id", edge_id)?;
                if matches!(
                    operation,
                    TopologyMutationOperation::Add | TopologyMutationOperation::Alter
                ) && travel_minutes.is_none()
                {
                    return Err(anyhow!(
                        "topology creation or alteration requires travel time"
                    ));
                }
            }
            AdmitEntity {
                initial_components,
                admission_receipt_id,
                ..
            } => {
                if initial_components.is_empty() {
                    return Err(anyhow!("entity admission requires initial components"));
                }
                nonempty("admission receipt id", admission_receipt_id)?;
            }
            RetireEntity { reason, .. } => nonempty("retirement reason", reason)?,
            AdvanceWorldTime { minutes, .. } if *minutes <= 0 => {
                return Err(anyhow!("world time must advance by a positive duration"));
            }
            TransferCustody { .. } | ChangePressure { .. } | AdvanceWorldTime { .. } => {}
        }
        Ok(())
    }
}

/// Compile a single-use permit that authorizes exactly one already-admitted
/// semantic mutation. This is the compatibility cut for legacy outcome
/// records: they may be lowered into the algebra, but they do not retain an
/// independent write policy.
pub fn exact_mutation_permit(
    id: impl Into<String>,
    mutation: &WorldMutation,
) -> Result<MutationPermit> {
    mutation.validate_local_shape()?;
    let subject_bindings = mutation
        .subject_roles()
        .into_iter()
        .map(|(role, allowed_subjects)| MutationSubjectBinding {
            role,
            allowed_subjects,
        })
        .collect();
    let mut string_constraints = BTreeMap::new();
    for (role, value) in mutation.string_roles() {
        let length = u16::try_from(value.chars().count())
            .map_err(|_| anyhow!("world mutation string exceeds permit capacity"))?;
        let constraint = string_constraints
            .entry(role)
            .or_insert_with(|| StringConstraint {
                allowed_values: BTreeSet::new(),
                minimum_length: 1,
                maximum_length: length.max(1),
            });
        constraint.maximum_length = constraint.maximum_length.max(length);
        constraint.allowed_values.insert(value);
    }
    let integer_bounds = mutation
        .integer_roles()
        .into_iter()
        .map(|(role, value)| {
            (
                role,
                IntegerBounds {
                    minimum: value,
                    maximum: value,
                },
            )
        })
        .collect();
    Ok(MutationPermit {
        id: id.into(),
        operation: mutation.operation(),
        subject_bindings,
        string_constraints,
        integer_bounds,
        maximum_uses: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn subject(kind: SubjectKind, id: &str) -> SubjectRef {
        SubjectRef {
            kind,
            id: id.into(),
        }
    }

    fn exact(values: &[&str]) -> StringConstraint {
        StringConstraint {
            allowed_values: values.iter().map(|value| (*value).into()).collect(),
            minimum_length: 1,
            maximum_length: 240,
        }
    }

    fn relocate_permit() -> MutationPermit {
        MutationPermit {
            id: "permit:move-east".into(),
            operation: WorldMutationOperation::Relocate,
            subject_bindings: vec![
                MutationSubjectBinding {
                    role: MutationSubjectRole::Subject,
                    allowed_subjects: BTreeSet::from([subject(SubjectKind::Actor, "actor:ash")]),
                },
                MutationSubjectBinding {
                    role: MutationSubjectRole::OriginPlace,
                    allowed_subjects: BTreeSet::from([subject(SubjectKind::Place, "place:camp")]),
                },
                MutationSubjectBinding {
                    role: MutationSubjectRole::DestinationPlace,
                    allowed_subjects: BTreeSet::from([subject(
                        SubjectKind::Place,
                        "place:east-trail",
                    )]),
                },
            ],
            string_constraints: BTreeMap::from([(
                MutationStringRole::RouteId,
                exact(&["route:camp-east"]),
            )]),
            integer_bounds: BTreeMap::new(),
            maximum_uses: 1,
        }
    }

    fn envelope() -> MutationAuthorityEnvelope {
        let mut envelope = MutationAuthorityEnvelope {
            schema: "ghostlight.mutation_authority_envelope.v1".into(),
            id: "authority:move-east".into(),
            campaign_id: Uuid::nil(),
            world_revision: 40,
            resolution_epoch: None,
            procedure: MutationProcedure::ForegroundAttempt,
            source_subject: Some(subject(SubjectKind::Actor, "actor:ash")),
            outcome: MutationOutcomeBinding::Foreground(OutcomeBand::Success),
            effect_ceiling: "Ash may reach the eastern trail; no other subject moves.".into(),
            permits: vec![relocate_permit()],
            authority_receipt_ids: BTreeSet::from(["assessment:move-east".into()]),
            expires_at: Utc::now() + Duration::minutes(5),
            digest: String::new(),
        };
        envelope.digest = envelope_digest(&envelope).unwrap();
        envelope
    }

    fn valid_batch(envelope: &MutationAuthorityEnvelope) -> WorldMutationBatch {
        let mut batch = WorldMutationBatch {
            schema: "ghostlight.world_mutation_batch.v1".into(),
            id: "batch:move-east".into(),
            campaign_id: envelope.campaign_id,
            expected_world_revision: envelope.world_revision,
            expected_resolution_epoch: envelope.resolution_epoch,
            authority_envelope_digest: envelope.digest.clone(),
            source_receipt_id: "assessment:move-east".into(),
            means_digest: Some("sha256:means".into()),
            intended_effect_digest: Some("sha256:intent".into()),
            mutations: vec![PermittedWorldMutation {
                permit_id: "permit:move-east".into(),
                mutation: WorldMutation::Relocate {
                    subject: subject(SubjectKind::Actor, "actor:ash"),
                    from_place: subject(SubjectKind::Place, "place:camp"),
                    to_place: subject(SubjectKind::Place, "place:east-trail"),
                    route_id: "route:camp-east".into(),
                },
            }],
            digest: String::new(),
        };
        batch.digest = mutation_batch_digest(&batch).unwrap();
        batch
    }

    #[test]
    fn exact_permit_accepts_only_its_bound_transition() {
        let envelope = envelope();
        let batch = valid_batch(&envelope);
        validate_batch_structure(&envelope, &batch, Utc::now()).unwrap();

        let mut stolen = batch.clone();
        let WorldMutation::Relocate { subject, .. } = &mut stolen.mutations[0].mutation else {
            panic!("fixture mutation changed kind");
        };
        *subject = subject_ref(SubjectKind::Actor, "actor:sable");
        stolen.digest = mutation_batch_digest(&stolen).unwrap();
        assert!(
            validate_batch_structure(&envelope, &stolen, Utc::now())
                .unwrap_err()
                .to_string()
                .contains("role Subject")
        );
    }

    #[test]
    fn exact_permit_binds_semantic_description_not_only_subject_and_operation() {
        let mutation = WorldMutation::ChangeRelationship {
            source: subject(SubjectKind::Actor, "actor:ash"),
            target: subject(SubjectKind::Actor, "actor:sable"),
            operation: RelationshipMutationOperation::Create,
            relationship_id: "relationship:ash:sable".into(),
            description: Some("Ash trusts Sable with the route.".into()),
            strength_delta: None,
        };
        let permit = exact_mutation_permit("permit:relationship", &mutation).unwrap();
        validate_permitted_mutation(&permit, &mutation).unwrap();

        let mut rewritten = mutation;
        let WorldMutation::ChangeRelationship { description, .. } = &mut rewritten else {
            unreachable!();
        };
        *description = Some("Ash owes Sable absolute obedience.".into());
        assert!(validate_permitted_mutation(&permit, &rewritten).is_err());
    }

    #[test]
    fn stale_or_partially_invalid_batch_has_no_structural_admission() {
        let envelope = envelope();
        let mut batch = valid_batch(&envelope);
        batch.mutations.push(batch.mutations[0].clone());
        batch.digest = mutation_batch_digest(&batch).unwrap();
        assert!(
            validate_batch_structure(&envelope, &batch, Utc::now())
                .unwrap_err()
                .to_string()
                .contains("use count")
        );

        let mut stale = valid_batch(&envelope);
        stale.expected_world_revision += 1;
        stale.digest = mutation_batch_digest(&stale).unwrap();
        assert!(
            validate_batch_structure(&envelope, &stale, Utc::now())
                .unwrap_err()
                .to_string()
                .contains("authority snapshot")
        );
    }

    #[test]
    fn action_specific_schema_omits_unavailable_mutation_variants() {
        let envelope = envelope();
        let schema = mutation_batch_json_schema(&envelope).unwrap();
        let rendered = serde_json::to_string(&schema).unwrap();
        assert!(rendered.contains("relocate"));
        assert!(!rendered.contains("transfer_custody"));
        assert!(!rendered.contains("change_identity"));

        let instance = serde_json::to_value(valid_batch(&envelope)).unwrap();
        assert!(
            jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&instance)
        );

        let mut stolen = valid_batch(&envelope);
        let WorldMutation::Relocate { to_place, .. } = &mut stolen.mutations[0].mutation else {
            unreachable!();
        };
        *to_place = subject(SubjectKind::Place, "place:unreachable");
        stolen.digest = mutation_batch_digest(&stolen).unwrap();
        let stolen = serde_json::to_value(stolen).unwrap();
        assert!(
            !jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&stolen)
        );
    }

    #[test]
    fn identity_disclosure_requires_exact_audience_authority() {
        let sable = subject(SubjectKind::Actor, "actor:sable");
        let ash = subject(SubjectKind::Actor, "actor:ash");
        let stranger = subject(SubjectKind::Actor, "actor:stranger");
        let permit = MutationPermit {
            id: "permit:sable-disclosure".into(),
            operation: WorldMutationOperation::IdentityDisclose,
            subject_bindings: vec![
                MutationSubjectBinding {
                    role: MutationSubjectRole::Subject,
                    allowed_subjects: BTreeSet::from([sable.clone()]),
                },
                MutationSubjectBinding {
                    role: MutationSubjectRole::Recipient,
                    allowed_subjects: BTreeSet::from([ash.clone()]),
                },
            ],
            string_constraints: BTreeMap::from([
                (
                    MutationStringRole::IdentityHandleId,
                    exact(&["identity:sable:given-handle"]),
                ),
                (MutationStringRole::IdentityHandleValue, exact(&["Sable"])),
            ]),
            integer_bounds: BTreeMap::new(),
            maximum_uses: 1,
        };
        let mutation = WorldMutation::ChangeIdentity {
            subject: sable,
            operation: IdentityMutationOperation::Disclose,
            handle_id: "identity:sable:given-handle".into(),
            handle_value: Some("Sable".into()),
            audience: vec![ash],
        };
        validate_permitted_mutation(&permit, &mutation).unwrap();

        let mut leaked = mutation;
        let WorldMutation::ChangeIdentity { audience, .. } = &mut leaked else {
            unreachable!();
        };
        audience.push(stranger);
        assert!(validate_permitted_mutation(&permit, &leaked).is_err());
    }

    fn component_world() -> ComponentWorldState {
        let campaign = subject(SubjectKind::Campaign, "campaign:test");
        let ash = subject(SubjectKind::Actor, "actor:ash");
        let sable = subject(SubjectKind::Actor, "actor:sable");
        let rival = subject(SubjectKind::Actor, "actor:rival");
        let clinic = subject(SubjectKind::Institution, "institution:clinic");
        let camp = subject(SubjectKind::Place, "place:camp");
        let trail = subject(SubjectKind::Place, "place:trail");
        let refugees = subject(SubjectKind::Population, "population:refugees");
        let medicine = subject(SubjectKind::Resource, "resource:medicine-lot");
        let proposition = subject(SubjectKind::Proposition, "proposition:east-trail");
        let mut subjects = BTreeMap::new();
        for value in [
            campaign.clone(),
            ash.clone(),
            sable.clone(),
            rival,
            clinic,
            camp.clone(),
            trail,
            refugees.clone(),
            medicine.clone(),
            proposition.clone(),
        ] {
            subjects.insert(
                value.clone(),
                TypedSubject {
                    schema: "ghostlight.typed_subject.v1".into(),
                    subject: value,
                    lifecycle: LifecycleStatus::Active,
                    admitted_components: BTreeSet::new(),
                    version: 4,
                },
            );
        }
        ComponentWorldState {
            schema: "ghostlight.component_world_state.v1".into(),
            campaign_id: Uuid::nil(),
            revision: 4,
            resolution_epoch: 2,
            world_time: Utc::now(),
            subjects,
            occupancy: BTreeMap::from([
                (ash.clone(), camp.clone()),
                (sable.clone(), camp.clone()),
                (campaign, camp.clone()),
            ]),
            custody: BTreeMap::from([(medicine.clone(), sable.clone())]),
            resources: BTreeMap::from([(
                medicine.clone(),
                ResourceComponentState {
                    schema: "ghostlight.resource_component.v1".into(),
                    resource: medicine,
                    resource_kind: "medicine".into(),
                    label: "medicine lot".into(),
                    quantity: 10,
                    integrity: 100,
                    qualities: BTreeSet::new(),
                    version: 4,
                },
            )]),
            capabilities: BTreeMap::new(),
            conditions: BTreeMap::new(),
            commitments: BTreeMap::new(),
            relationships: BTreeMap::new(),
            pressures: BTreeMap::new(),
            knowledge: BTreeMap::from([(
                KnowledgeKey {
                    knower: sable,
                    proposition,
                },
                KnowledgeComponentState {
                    status: "known".into(),
                    source: None,
                    channel: None,
                    concealed_from: BTreeSet::new(),
                    version: 4,
                },
            )]),
            memories: BTreeMap::new(),
            postures: BTreeMap::new(),
            memberships: BTreeMap::from([(
                MembershipKey {
                    actor: ash,
                    population: refugees,
                },
                PopulationMembershipState {
                    active: true,
                    version: 4,
                },
            )]),
            population_lineages: BTreeMap::new(),
            identities: BTreeMap::new(),
            topology: BTreeMap::from([(
                "route:camp-trail".into(),
                TopologyComponentState {
                    id: "route:camp-trail".into(),
                    from_place: camp,
                    to_place: subject(SubjectKind::Place, "place:trail"),
                    travel_minutes: 30,
                    open: true,
                    version: 4,
                },
            )]),
        }
    }

    fn permit(
        id: &str,
        operation: WorldMutationOperation,
        subjects: Vec<(MutationSubjectRole, BTreeSet<SubjectRef>)>,
        strings: Vec<(MutationStringRole, StringConstraint)>,
        integers: Vec<(MutationIntegerRole, IntegerBounds)>,
    ) -> MutationPermit {
        MutationPermit {
            id: id.into(),
            operation,
            subject_bindings: subjects
                .into_iter()
                .map(|(role, allowed_subjects)| MutationSubjectBinding {
                    role,
                    allowed_subjects,
                })
                .collect(),
            string_constraints: strings.into_iter().collect(),
            integer_bounds: integers.into_iter().collect(),
            maximum_uses: 1,
        }
    }

    fn envelope_for(
        state: &ComponentWorldState,
        permits: Vec<MutationPermit>,
    ) -> MutationAuthorityEnvelope {
        let mut envelope = MutationAuthorityEnvelope {
            schema: "ghostlight.mutation_authority_envelope.v1".into(),
            id: "authority:component-test".into(),
            campaign_id: state.campaign_id,
            world_revision: state.revision,
            resolution_epoch: Some(state.resolution_epoch),
            procedure: MutationProcedure::StrategicOutcome,
            source_subject: Some(subject(SubjectKind::Actor, "actor:sable")),
            outcome: MutationOutcomeBinding::Strategic(StrategicOutcomeBand::Success),
            effect_ceiling: "Only the exact permitted component transitions may commit.".into(),
            permits,
            authority_receipt_ids: BTreeSet::from(["outcome:test".into()]),
            expires_at: Utc::now() + Duration::minutes(5),
            digest: String::new(),
        };
        envelope.digest = envelope_digest(&envelope).unwrap();
        envelope
    }

    fn batch_for(
        state: &ComponentWorldState,
        envelope: &MutationAuthorityEnvelope,
        mutations: Vec<(&str, WorldMutation)>,
    ) -> WorldMutationBatch {
        let mut batch = WorldMutationBatch {
            schema: "ghostlight.world_mutation_batch.v1".into(),
            id: format!("batch:{}", Uuid::new_v4().simple()),
            campaign_id: state.campaign_id,
            expected_world_revision: state.revision,
            expected_resolution_epoch: Some(state.resolution_epoch),
            authority_envelope_digest: envelope.digest.clone(),
            source_receipt_id: "outcome:test".into(),
            means_digest: Some("sha256:means".into()),
            intended_effect_digest: Some("sha256:intent".into()),
            mutations: mutations
                .into_iter()
                .map(|(permit_id, mutation)| PermittedWorldMutation {
                    permit_id: permit_id.into(),
                    mutation,
                })
                .collect(),
            digest: String::new(),
        };
        batch.digest = mutation_batch_digest(&batch).unwrap();
        batch
    }

    #[test]
    fn component_batch_is_atomic_when_a_late_mutation_fails() {
        let state = component_world();
        let medicine = subject(SubjectKind::Resource, "resource:medicine-lot");
        let sable = subject(SubjectKind::Actor, "actor:sable");
        let clinic = subject(SubjectKind::Institution, "institution:clinic");
        let camp = subject(SubjectKind::Place, "place:camp");
        let trail = subject(SubjectKind::Place, "place:trail");
        let permits = vec![
            permit(
                "permit:transfer",
                WorldMutationOperation::TransferCustody,
                vec![
                    (
                        MutationSubjectRole::Resource,
                        BTreeSet::from([medicine.clone()]),
                    ),
                    (
                        MutationSubjectRole::SourceCustodian,
                        BTreeSet::from([sable.clone()]),
                    ),
                    (
                        MutationSubjectRole::DestinationCustodian,
                        BTreeSet::from([clinic.clone()]),
                    ),
                ],
                vec![],
                vec![],
            ),
            permit(
                "permit:move",
                WorldMutationOperation::Relocate,
                vec![
                    (
                        MutationSubjectRole::Subject,
                        BTreeSet::from([sable.clone()]),
                    ),
                    (
                        MutationSubjectRole::OriginPlace,
                        BTreeSet::from([camp.clone()]),
                    ),
                    (
                        MutationSubjectRole::DestinationPlace,
                        BTreeSet::from([trail.clone()]),
                    ),
                ],
                vec![(MutationStringRole::RouteId, exact(&["route:camp-trail"]))],
                vec![],
            ),
        ];
        let envelope = envelope_for(&state, permits);
        let batch = batch_for(
            &state,
            &envelope,
            vec![
                (
                    "permit:transfer",
                    WorldMutation::TransferCustody {
                        resource: medicine,
                        from_custodian: sable.clone(),
                        to_custodian: clinic,
                    },
                ),
                (
                    "permit:move",
                    WorldMutation::Relocate {
                        subject: sable,
                        from_place: camp,
                        to_place: trail,
                        route_id: "route:camp-trail".into(),
                    },
                ),
            ],
        );
        let mut invalid_state = state.clone();
        invalid_state
            .occupancy
            .remove(&subject(SubjectKind::Actor, "actor:sable"));
        assert!(
            apply_component_world_batch(&invalid_state, &envelope, &batch, Utc::now()).is_err()
        );
        assert_eq!(invalid_state.custody, state.custody);
        assert_eq!(invalid_state.revision, state.revision);
    }

    #[test]
    fn resource_split_and_transfer_conserve_quantity_and_exact_custody() {
        let state = component_world();
        let source = subject(SubjectKind::Resource, "resource:medicine-lot");
        let child = subject(SubjectKind::Resource, "resource:medicine-dose");
        let sable = subject(SubjectKind::Actor, "actor:sable");
        let clinic = subject(SubjectKind::Institution, "institution:clinic");
        let permits = vec![
            permit(
                "permit:split",
                WorldMutationOperation::ResourceSplit,
                vec![
                    (
                        MutationSubjectRole::Resource,
                        BTreeSet::from([source.clone()]),
                    ),
                    (
                        MutationSubjectRole::RelatedResource,
                        BTreeSet::from([child.clone()]),
                    ),
                    (MutationSubjectRole::Owner, BTreeSet::from([sable.clone()])),
                ],
                vec![(
                    MutationStringRole::RecipeId,
                    exact(&["recipe:measure-dose"]),
                )],
                vec![(
                    MutationIntegerRole::Quantity,
                    IntegerBounds {
                        minimum: 1,
                        maximum: 4,
                    },
                )],
            ),
            permit(
                "permit:transfer-dose",
                WorldMutationOperation::TransferCustody,
                vec![
                    (
                        MutationSubjectRole::Resource,
                        BTreeSet::from([child.clone()]),
                    ),
                    (
                        MutationSubjectRole::SourceCustodian,
                        BTreeSet::from([sable.clone()]),
                    ),
                    (
                        MutationSubjectRole::DestinationCustodian,
                        BTreeSet::from([clinic.clone()]),
                    ),
                ],
                vec![],
                vec![],
            ),
        ];
        let envelope = envelope_for(&state, permits);
        let batch = batch_for(
            &state,
            &envelope,
            vec![
                (
                    "permit:split",
                    WorldMutation::MutateResource {
                        resource: source.clone(),
                        operation: ResourceMutationOperation::Split,
                        custodian: Some(sable.clone()),
                        related_resources: vec![child.clone()],
                        resource_kind: None,
                        resource_label: None,
                        recipe_id: Some("recipe:measure-dose".into()),
                        quantity: Some(4),
                        integrity: None,
                    },
                ),
                (
                    "permit:transfer-dose",
                    WorldMutation::TransferCustody {
                        resource: child.clone(),
                        from_custodian: sable,
                        to_custodian: clinic.clone(),
                    },
                ),
            ],
        );
        let applied = apply_component_world_batch(&state, &envelope, &batch, Utc::now()).unwrap();
        assert_eq!(applied.state.resources[&source].quantity, 6);
        assert_eq!(applied.state.resources[&child].quantity, 4);
        assert_eq!(applied.state.custody[&child], clinic);
        assert_eq!(
            applied.state.resources[&source].quantity + applied.state.resources[&child].quantity,
            10
        );
        assert_eq!(applied.state.revision, 5);
    }

    #[test]
    fn component_state_batch_and_exact_proof_rows_commit_in_one_cultcache_cas() {
        let state = component_world();
        let campaign = subject(SubjectKind::Campaign, "campaign:test");
        let permit = permit(
            "permit:time",
            WorldMutationOperation::AdvanceWorldTime,
            vec![(
                MutationSubjectRole::Subject,
                BTreeSet::from([campaign.clone()]),
            )],
            vec![],
            vec![(
                MutationIntegerRole::WorldMinutes,
                IntegerBounds {
                    minimum: 30,
                    maximum: 30,
                },
            )],
        );
        let envelope = envelope_for(&state, vec![permit]);
        let batch = batch_for(
            &state,
            &envelope,
            vec![(
                "permit:time",
                WorldMutation::AdvanceWorldTime {
                    campaign,
                    minutes: 30,
                },
            )],
        );
        let applied = apply_component_world_batch(&state, &envelope, &batch, Utc::now()).unwrap();
        let path = std::env::temp_dir().join(format!(
            "ghostlight-transition-store-{}.cc",
            Uuid::new_v4().simple()
        ));
        let store = crate::persistence::CampaignStore::open(&path).unwrap();
        let expected = store.create_component_world_state(&state).unwrap();
        store
            .commit_world_mutation_batch(
                &expected,
                &applied.state,
                &envelope,
                &batch,
                &applied.receipt,
            )
            .unwrap();
        let persisted = store
            .load::<ComponentWorldState>("component_world_state.v1", &state.campaign_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted, applied.state);
        assert_eq!(
            store.keys("mutation_authority_envelope.v1").unwrap().len(),
            1
        );
        assert_eq!(store.keys("world_mutation_batch.v1").unwrap().len(), 1);
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);
        assert!(
            store
                .commit_world_mutation_batch(
                    &expected,
                    &applied.state,
                    &envelope,
                    &batch,
                    &applied.receipt,
                )
                .is_err()
        );
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);
        drop(store);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn communicated_knowledge_reaches_only_exact_recipients() {
        let state = component_world();
        let sable = subject(SubjectKind::Actor, "actor:sable");
        let ash = subject(SubjectKind::Actor, "actor:ash");
        let rival = subject(SubjectKind::Actor, "actor:rival");
        let proposition = subject(SubjectKind::Proposition, "proposition:east-trail");
        let permit = permit(
            "permit:tell-ash",
            WorldMutationOperation::KnowledgeCommunicate,
            vec![
                (
                    MutationSubjectRole::Proposition,
                    BTreeSet::from([proposition.clone()]),
                ),
                (
                    MutationSubjectRole::Speaker,
                    BTreeSet::from([sable.clone()]),
                ),
                (
                    MutationSubjectRole::Recipient,
                    BTreeSet::from([ash.clone()]),
                ),
            ],
            vec![],
            vec![],
        );
        let envelope = envelope_for(&state, vec![permit]);
        let batch = batch_for(
            &state,
            &envelope,
            vec![(
                "permit:tell-ash",
                WorldMutation::ChangeKnowledge {
                    operation: KnowledgeMutationOperation::Communicate,
                    proposition: proposition.clone(),
                    knower: None,
                    speaker: Some(sable),
                    recipients: vec![ash.clone()],
                    channel: None,
                },
            )],
        );
        let applied = apply_component_world_batch(&state, &envelope, &batch, Utc::now()).unwrap();
        assert!(applied.state.knowledge.contains_key(&KnowledgeKey {
            knower: ash,
            proposition: proposition.clone()
        }));
        assert!(!applied.state.knowledge.contains_key(&KnowledgeKey {
            knower: rival,
            proposition
        }));
    }

    #[test]
    fn sable_identity_survives_population_and_occupancy_changes_without_global_disclosure() {
        let mut state = component_world();
        let sable = subject(SubjectKind::Actor, "actor:sable");
        let ash = subject(SubjectKind::Actor, "actor:ash");
        let rival = subject(SubjectKind::Actor, "actor:rival");
        state.identities.insert(
            "identity:sable:given-handle".into(),
            IdentityHandleState {
                schema: "ghostlight.identity_handle.v1".into(),
                id: "identity:sable:given-handle".into(),
                subject: sable.clone(),
                value: "Sable".into(),
                active: true,
                known_by: BTreeSet::from([sable.clone()]),
                restricted_to: BTreeSet::new(),
                source_revision: 4,
            },
        );
        let permit = permit(
            "permit:disclose-sable",
            WorldMutationOperation::IdentityDisclose,
            vec![
                (
                    MutationSubjectRole::Subject,
                    BTreeSet::from([sable.clone()]),
                ),
                (
                    MutationSubjectRole::Recipient,
                    BTreeSet::from([ash.clone()]),
                ),
            ],
            vec![(
                MutationStringRole::IdentityHandleId,
                exact(&["identity:sable:given-handle"]),
            )],
            vec![],
        );
        let envelope = envelope_for(&state, vec![permit]);
        let batch = batch_for(
            &state,
            &envelope,
            vec![(
                "permit:disclose-sable",
                WorldMutation::ChangeIdentity {
                    subject: sable.clone(),
                    operation: IdentityMutationOperation::Disclose,
                    handle_id: "identity:sable:given-handle".into(),
                    handle_value: None,
                    audience: vec![ash.clone()],
                },
            )],
        );
        let applied = apply_component_world_batch(&state, &envelope, &batch, Utc::now()).unwrap();
        let handle = &applied.state.identities["identity:sable:given-handle"];
        assert!(handle.known_by.contains(&sable));
        assert!(handle.known_by.contains(&ash));
        assert!(!handle.known_by.contains(&rival));
        assert_eq!(handle.subject, sable);
    }

    fn subject_ref(kind: SubjectKind, id: &str) -> SubjectRef {
        subject(kind, id)
    }
}

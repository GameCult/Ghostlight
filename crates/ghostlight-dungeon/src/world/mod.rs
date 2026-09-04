//! Sealed replacement world owner under construction.
//!
//! The world owner is one deterministic authority: authenticated commands enter,
//! one reducer decides, and one journal atomically commits the resulting state.
//! Controllers may use models, but models never own lifecycle, scope, affordances,
//! opportunities, reduction, or persistence.

mod action;
mod controllers;
mod journal;
mod mailbox;
mod patch;

pub(crate) use action::ActionMismatch;
pub(crate) use controllers::{
    ControllerError, ControllerModels, ControllerOpenError, ControllerPendingReason,
    ControllerRunner, ControllerWorkCustody, NarrativeCapture, NarrativeDecision, NarrativePending,
    NarrativeRun, OperationalCapture, OperationalDecision, OperationalPending, OperationalRun,
    SourceRange, SubmissionDisposition, TranslationGapSummary,
};
pub(crate) use mailbox::{MailboxError, WorldMailbox};
pub(crate) use patch::{
    AccessKind, Affordance, AffordanceKindName, AuthorityGrant, AuthorityKindName, AuthorityTarget,
    Bounds, ComponentOpKind, Cost, Declaration, DependencyTarget, DraftHandle, EffectSlot,
    EntityDeclaration, EntityKind, EvidenceRef, Forum, GrievanceKindName, Mismatch, Office,
    OfficeName, OutcomeBand, PatchAnswer, Position, Precondition, Quantity, Ref, RefKind, Role,
    RoleSpec, SubjectDeclaration, WorldPatch,
};
#[cfg(test)]
use patch::{AffordanceDeclaration, ComponentOp, DependencyRef, RouteDeclaration};
#[cfg(test)]
pub(crate) use patch::{AuthorityGrantRef, AuthorityTargetRef};
use patch::{EdgeRecord, EntityRecord, LedgerDelta, ResolvedOp, ResolvedPatch};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

pub(crate) const STATE_SCHEMA: &str = "ghostlight.world_state.authority.v1";
pub(crate) const COMMIT_SCHEMA: &str = "ghostlight.world_commit.authority.v1";

/// Compatibility tag derived from [`STATE_SCHEMA`]: the trailing
/// `<family>-<version>` pair (e.g. `foundation-v1`). Callers that publish a
/// compatibility marker alongside the schema string must derive it from here
/// rather than hand-copying a second literal that can drift from the schema.
pub(crate) fn state_schema_compatibility_tag() -> String {
    let mut segments = STATE_SCHEMA.rsplit('.');
    let version = segments.next().unwrap_or_default();
    let family = segments.next().unwrap_or_default();
    format!("{family}-{version}")
}

macro_rules! opaque_uuid {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
        )]
        #[serde(transparent)]
        pub(crate) struct $name(Uuid);
    };
}

opaque_uuid!(WorldId);
opaque_uuid!(CommandId);
opaque_uuid!(SubjectId);
opaque_uuid!(EntityId);
opaque_uuid!(EdgeId);
opaque_uuid!(ControllerId);
opaque_uuid!(AffordanceId);
opaque_uuid!(EventId);

impl CommandId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }

    pub(crate) fn new() -> Self {
        Self::issue()
    }

    pub(crate) fn parse_uuid(value: &str) -> Result<Self, KernelError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| KernelError::InvalidCommandId)
    }

    fn key(self) -> String {
        self.0.to_string()
    }
}

impl WorldId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }

    fn key(self) -> String {
        self.0.to_string()
    }
}

// Subject, entity, controller, and affordance IDs are derived, never drawn.
// `patch::derive_id` is the only allocator; these fixtures exist so tests can
// name an ID that no partition holds.
#[cfg(test)]
impl SubjectId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }
}

#[cfg(test)]
impl EntityId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }
}

#[cfg(test)]
impl ControllerId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }
}

#[cfg(test)]
impl AffordanceId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }
}

impl EventId {
    fn for_command(command_id: CommandId) -> Self {
        Self(command_id.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct PrincipalId(String);

impl PrincipalId {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorldPhase {
    Draft,
    Active,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SubjectKind {
    Person,
    Institution,
    Population,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ControllerMode {
    Human,
    NarrativePersona,
    OperationalAgent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum NewController {
    Human { principal: PrincipalId },
    NarrativePersona,
    OperationalAgent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct CreateWorld {
    id: CommandId,
    owner: PrincipalId,
    title: String,
    patch: WorldPatch,
}

/// Unattributed creation intent. World ingress derives ownership, controller
/// identity, handles, kinds, and affordances from verified principal evidence.
#[derive(Clone, Debug)]
pub(crate) struct CreateWorldIntent {
    pub(crate) id: CommandId,
    pub(crate) title: String,
    pub(crate) human_subject_label: String,
    pub(crate) narrative_persona_label: Option<String>,
    pub(crate) operational_agent_label: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
enum CallerId {
    Principal(PrincipalId),
    Controller(ControllerId),
}

#[derive(Clone, Debug)]
struct AuthenticatedCaller {
    caller: CallerId,
}

impl AuthenticatedCaller {
    fn verified_principal(principal: PrincipalId) -> Self {
        Self {
            caller: CallerId::Principal(principal),
        }
    }

    fn verified_controller(controller: ControllerId) -> Self {
        Self {
            caller: CallerId::Controller(controller),
        }
    }

    #[cfg(test)]
    fn fixture(caller: CallerId) -> Self {
        Self { caller }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionScope {
    pub(crate) subject_id: SubjectId,
}

/// The digest of exactly the components a proposal's verification reads: the
/// scope's controller assignment, its affordance grants, its subject's Position,
/// and the routes incident to that place. Built through the one `digest()`
/// helper over ordered containers, so it is a pure function of committed state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct ScopeDigest(String);

impl ScopeDigest {
    fn as_str(&self) -> &str {
        &self.0
    }

    #[cfg(test)]
    pub(crate) fn fixture(value: &str) -> Self {
        Self(value.into())
    }
}

#[derive(Serialize)]
struct ScopePreimage<'a> {
    world_id: WorldId,
    subject_id: SubjectId,
    controller: &'a ControllerAssignment,
    /// The granted entries *and their definitions*: preconditions and ceilings
    /// are what admission reads, so a change to a granted entry must change the
    /// digest.
    affordances: BTreeMap<AffordanceId, &'a Affordance>,
    components: &'a ScopeComponents,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionOpportunity {
    pub(crate) world_id: WorldId,
    pub(crate) revision: u64,
    pub(crate) scope_digest: ScopeDigest,
    pub(crate) scope: DecisionScope,
    pub(crate) controller_id: ControllerId,
    pub(crate) controller_mode: ControllerMode,
    pub(crate) affordance_ids: Vec<AffordanceId>,
}

impl DecisionOpportunity {
    pub(crate) fn digest(&self) -> Result<String, KernelError> {
        digest(self)
    }
}

/// The referent a role is bound to. Canonical IDs only: an invocation happens
/// in Active, where no declaration exists, so a draft reference is
/// unrepresentable here and the action lane runs no draft resolution.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "target", content = "id", rename_all = "snake_case")]
pub(crate) enum Target {
    Subject(SubjectId),
    Entity(EntityId),
    Edge(EdgeId),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoleBinding {
    pub(crate) role: Role,
    pub(crate) target: Target,
}

/// Canonical nonempty text. Emptiness is one `ActionMismatch` inside the
/// complete set, not a kernel error of its own.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct Utterance(String);

impl Utterance {
    pub(crate) fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        if patch::is_canonical_text(&value) {
            Some(Self(value))
        } else {
            None
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "magnitude", content = "value", rename_all = "snake_case")]
pub(crate) enum Magnitude {
    None,
    Quantity(Quantity),
    Cost(Cost),
}

/// One proposal against one effect slot. It carries no referents: they all come
/// from the slot's roles resolved through the invocation's bindings, so an
/// invocation cannot name a target its affordance declared no role for and the
/// ceiling check is a comparison rather than a graph walk.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProposedEffect {
    pub(crate) slot: usize,
    pub(crate) magnitude: Magnitude,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionInvocation {
    pub(crate) affordance: AffordanceId,
    pub(crate) bindings: Vec<RoleBinding>,
    /// Exactly one entry per slot in the entry, in any order.
    pub(crate) proposed: Vec<ProposedEffect>,
    pub(crate) speech: Option<Utterance>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct CommandEnvelope {
    id: CommandId,
    world_id: WorldId,
    expected_revision: u64,
    caller: CallerId,
    body: CommandBody,
}

/// Unattributed human command intent. The mailbox derives the caller from
/// live app-session evidence before the reducer sees an envelope.
#[derive(Clone, Debug)]
pub(crate) struct PrincipalCommandIntent {
    pub(crate) id: CommandId,
    pub(crate) world_id: WorldId,
    pub(crate) expected_revision: u64,
    pub(crate) body: CommandBody,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum CommandBody {
    ApproveDraft,
    ActivateWorld,
    ExerciseDecision {
        opportunity: DecisionOpportunity,
        invocation: DecisionInvocation,
    },
    DeclineDecision {
        opportunity: DecisionOpportunity,
    },
    AdmitPatch {
        /// Structurally `None` until boundary answers exist: `PatchAnswer` is
        /// uninhabited.
        answers: Option<PatchAnswer>,
        patch: WorldPatch,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct SubjectState {
    label: String,
    kind: SubjectKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ControllerAssignment {
    Human {
        controller_id: ControllerId,
        principal: PrincipalId,
    },
    NarrativePersona {
        controller_id: ControllerId,
    },
    OperationalAgent {
        controller_id: ControllerId,
    },
}

impl ControllerAssignment {
    fn id(&self) -> ControllerId {
        match self {
            Self::Human { controller_id, .. }
            | Self::NarrativePersona { controller_id }
            | Self::OperationalAgent { controller_id } => *controller_id,
        }
    }

    fn mode(&self) -> ControllerMode {
        match self {
            Self::Human { .. } => ControllerMode::Human,
            Self::NarrativePersona { .. } => ControllerMode::NarrativePersona,
            Self::OperationalAgent { .. } => ControllerMode::OperationalAgent,
        }
    }

    fn expected_caller(&self) -> CallerId {
        match self {
            Self::Human { principal, .. } => CallerId::Principal(principal.clone()),
            Self::NarrativePersona { controller_id } | Self::OperationalAgent { controller_id } => {
                CallerId::Controller(*controller_id)
            }
        }
    }

    fn human_principal(&self) -> Option<&PrincipalId> {
        match self {
            Self::Human { principal, .. } => Some(principal),
            Self::NarrativePersona { .. } | Self::OperationalAgent { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DecisionEvent {
    pub(crate) id: EventId,
    pub(crate) revision: u64,
    pub(crate) scope: DecisionScope,
    pub(crate) controller_id: ControllerId,
    pub(crate) invocation: DecisionInvocation,
    /// Index into the entry's `outcome_bands`. Re-derived and compared at apply
    /// and at replay, never trusted.
    pub(crate) band: usize,
    /// The selected band's slots, lowered through the invocation's bindings.
    /// Empty when the selected band names no effects.
    pub(crate) effects: Vec<ResolvedOp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct WorldState {
    schema: String,
    world_id: WorldId,
    revision: u64,
    phase: WorldPhase,
    owner: PrincipalId,
    title: String,
    draft_approvals: BTreeSet<PrincipalId>,
    subjects: BTreeMap<SubjectId, SubjectState>,
    entities: BTreeMap<EntityId, EntityRecord>,
    edges: BTreeMap<EdgeId, EdgeRecord>,
    positions: BTreeMap<SubjectId, Position>,
    /// What each subject holds. Absence is zero at both levels: no stored
    /// `Quantity(0)` and no empty inner map, so a duplicate holding is
    /// unrepresentable and "nothing" has one shape.
    holdings: BTreeMap<SubjectId, BTreeMap<EntityId, Quantity>>,
    /// What each subject depends on. Never empty for a present key.
    dependencies: BTreeMap<SubjectId, BTreeSet<DependencyTarget>>,
    /// What ground each subject has jurisdiction over, and under which
    /// world-declared kind. Never empty for a present key. No subject holds two
    /// grants of one kind over overlapping ground, so nothing ever arbitrates
    /// between two sources of one permission.
    authority: BTreeMap<SubjectId, BTreeSet<AuthorityGrant>>,
    /// The offices each institution constitutes, and who sits in them. An
    /// incumbent never exercises its institution's opportunity:
    /// `controller_assignments` stays the sole answer to who may call, and an
    /// office answers the different question of what its incumbent has
    /// jurisdiction over.
    selection: BTreeMap<SubjectId, BTreeMap<OfficeName, Office>>,
    /// Where each kind of grievance goes, and who may bring it. Keyed by
    /// neither a subject nor a referent, so it enters no scope digest and is
    /// admission-only state.
    redress: BTreeMap<GrievanceKindName, Forum>,
    controller_assignments: BTreeMap<DecisionScope, ControllerAssignment>,
    /// What an affordance *is*. World-authored, Draft-only, written by
    /// `admit_resolved` alone.
    affordance_catalog: BTreeMap<AffordanceId, Affordance>,
    /// Who may exercise which entry. A grant carries no payload, so a duplicate
    /// grant is unrepresentable rather than checked.
    affordance_grants: BTreeMap<DecisionScope, BTreeSet<AffordanceId>>,
    events: Vec<DecisionEvent>,
    state_digest: String,
    last_commit_digest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorldEffect {
    WorldCreated {
        owner: PrincipalId,
        title: String,
        resolved: ResolvedPatch,
    },
    PatchAdmitted {
        resolved: ResolvedPatch,
    },
    DraftApproved {
        principal: PrincipalId,
    },
    WorldActivated,
    DecisionExercised {
        opportunity: DecisionOpportunity,
        event: DecisionEvent,
    },
    DecisionDeclined {
        opportunity: DecisionOpportunity,
    },
}

/// The exact raw command admitted by the world owner. This is the sole
/// persistent command authority: IDs, callers, retry identity, and effects are
/// derived from this value rather than copied into independently forgeable
/// commit fields.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "command", rename_all = "snake_case")]
enum CommittedCommand {
    CreateWorld(CreateWorld),
    WorldCommand(CommandEnvelope),
}

impl CommittedCommand {
    fn id(&self) -> CommandId {
        match self {
            Self::CreateWorld(command) => command.id,
            Self::WorldCommand(command) => command.id,
        }
    }

    fn caller(&self) -> CallerId {
        match self {
            Self::CreateWorld(command) => CallerId::Principal(command.owner.clone()),
            Self::WorldCommand(command) => command.caller.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct WorldCommit {
    schema: String,
    world_id: WorldId,
    command: CommittedCommand,
    previous_revision: Option<u64>,
    resulting_revision: u64,
    previous_state_digest: Option<String>,
    resulting_state_digest: String,
    previous_commit_digest: Option<String>,
    effect: WorldEffect,
    committed_at: DateTime<Utc>,
    digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SubjectSnapshot {
    pub(crate) id: SubjectId,
    pub(crate) label: String,
    pub(crate) kind: SubjectKind,
    pub(crate) controller_id: ControllerId,
    pub(crate) controller_mode: ControllerMode,
    pub(crate) human_controller: Option<PrincipalId>,
    pub(crate) affordances: BTreeSet<AffordanceId>,
    pub(crate) position: Option<EntityId>,
    /// Lowered from the one `scope_components` owner, so these are exactly what
    /// the scope digest binds: this subject's own holdings, dependencies, and
    /// grants, the routes incident to its place, and the offices it occupies
    /// with what each lends. A counterparty's components never enter.
    pub(crate) holdings: BTreeMap<EntityId, Quantity>,
    pub(crate) dependencies: BTreeSet<DependencyTarget>,
    pub(crate) incident_routes: Vec<EdgeId>,
    pub(crate) authority: BTreeSet<AuthorityGrant>,
    pub(crate) offices_held: Vec<OfficeSnapshot>,
    /// View-only, and covered by no digest: an institution's own offices, so
    /// its controller can see what it lends and to whom. No precondition reads
    /// it, and installing a warden therefore does not reject the institution's
    /// in-flight proposals — what an institution lends is not what it may do.
    pub(crate) offices_granted: Vec<OfficeSnapshot>,
    /// View-only: the forums whose standing covers this subject, through the
    /// same covering predicate `HasStanding` uses.
    pub(crate) redress: Vec<ForumSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OfficeSnapshot {
    pub(crate) institution: SubjectId,
    pub(crate) office: OfficeName,
    pub(crate) incumbent: Option<SubjectId>,
    /// What the office actually lends: the institution's live grants whose kind
    /// the office delegates.
    pub(crate) authority: BTreeSet<AuthorityGrant>,
}

/// A forum a subject may petition. It carries no standing: a subject learns
/// that it may bring a grievance, not the boundary of everyone else's standing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ForumSnapshot {
    pub(crate) grievance: GrievanceKindName,
    pub(crate) forum: SubjectId,
}

/// One catalog entry as consumers read it. The whole entry, because every
/// derived surface — tool schemas, signature prose, the typed view's permission
/// block — is a projection of it and a narrower snapshot would be a second
/// vocabulary to keep in step.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AffordanceSnapshot {
    pub(crate) id: AffordanceId,
    pub(crate) entry: Affordance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResourceSnapshot {
    pub(crate) id: EntityId,
    pub(crate) label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaceSnapshot {
    pub(crate) id: EntityId,
    pub(crate) label: String,
    pub(crate) container: Option<EntityId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RouteSnapshot {
    pub(crate) id: EdgeId,
    pub(crate) label: String,
    pub(crate) from: EntityId,
    pub(crate) to: EntityId,
    pub(crate) access: AccessKind,
    pub(crate) cost: Cost,
    pub(crate) open: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorldSnapshot {
    pub(crate) world_id: WorldId,
    pub(crate) revision: u64,
    pub(crate) phase: WorldPhase,
    pub(crate) owner: PrincipalId,
    pub(crate) title: String,
    pub(crate) draft_approvals: BTreeSet<PrincipalId>,
    pub(crate) required_approvers: BTreeSet<PrincipalId>,
    pub(crate) subjects: Vec<SubjectSnapshot>,
    pub(crate) affordances: Vec<AffordanceSnapshot>,
    pub(crate) places: Vec<PlaceSnapshot>,
    pub(crate) resources: Vec<ResourceSnapshot>,
    pub(crate) routes: Vec<RouteSnapshot>,
    pub(crate) events: Vec<DecisionEvent>,
    pub(crate) opportunities: Vec<DecisionOpportunity>,
    pub(crate) state_digest: String,
    pub(crate) last_commit_digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommitReceipt {
    pub(crate) command_id: CommandId,
    pub(crate) resulting_revision: u64,
    pub(crate) resulting_state_digest: String,
    pub(crate) commit_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CreationReceipt {
    pub(crate) command_id: CommandId,
    pub(crate) world_id: WorldId,
    pub(crate) resulting_state_digest: String,
    pub(crate) commit_digest: String,
}

impl CreationReceipt {
    fn from_commit(commit: &WorldCommit) -> Result<Self, KernelError> {
        let CommittedCommand::CreateWorld(command) = &commit.command else {
            return Err(KernelError::Invariant(
                "world genesis receipt does not point to a creation command".into(),
            ));
        };
        let WorldEffect::WorldCreated { .. } = &commit.effect else {
            return Err(KernelError::Invariant(
                "world genesis receipt does not point to genesis".into(),
            ));
        };
        Ok(Self {
            command_id: command.id,
            world_id: commit.world_id,
            resulting_state_digest: commit.resulting_state_digest.clone(),
            commit_digest: commit.digest.clone(),
        })
    }
}

impl From<&WorldCommit> for CommitReceipt {
    fn from(commit: &WorldCommit) -> Self {
        Self {
            command_id: commit.command.id(),
            resulting_revision: commit.resulting_revision,
            resulting_state_digest: commit.resulting_state_digest.clone(),
            commit_digest: commit.digest.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SubmitReceipt {
    Applied(CommitReceipt),
    AlreadyApplied(CommitReceipt),
}

#[derive(Debug, Error)]
pub(crate) enum KernelError {
    #[error("command ID must be a UUID")]
    InvalidCommandId,
    #[error("world title must not be empty")]
    EmptyTitle,
    #[error("world owner or human controller principal must be canonical and nonempty")]
    EmptyPrincipal,
    #[error("patch rejected: {0:?}")]
    PatchRejected(Vec<Mismatch>),
    #[error("action rejected: {0:?}")]
    ActionRejected(Vec<ActionMismatch>),
    #[error("command targets another world")]
    WorldMismatch,
    #[error("authenticated caller does not match the command")]
    AuthenticationMismatch,
    #[error("caller does not own this world")]
    Unauthorized,
    #[error("command requires phase {expected:?}, current phase is {actual:?}")]
    WrongPhase {
        expected: WorldPhase,
        actual: WorldPhase,
    },
    #[error("caller is not a required draft approver")]
    NotDraftApprover,
    #[error("caller has already approved this draft")]
    DraftAlreadyApproved,
    #[error("draft is missing required approvals: {0:?}")]
    MissingApprovals(Vec<PrincipalId>),
    #[error("this world derives no such decision opportunity")]
    OpportunityMismatch,
    #[error("decision scope changed since the proposal was bound")]
    ScopeChanged {
        scope: DecisionScope,
        expected: ScopeDigest,
        actual: ScopeDigest,
    },
    #[error("caller does not control this decision scope")]
    ControllerMismatch,
    #[error("decision affordance is not granted by the opportunity")]
    AffordanceDenied,
    #[error("expected revision {expected}, current revision {actual}")]
    RevisionMismatch { expected: u64, actual: u64 },
    #[error("command ID was reused with different content")]
    CommandIdConflict,
    #[error("world creation ID was reused with different content")]
    CreationConflict,
    #[error("world creation target already contains another world")]
    CreationTargetOccupied,
    #[error("world has not been created")]
    WorldNotCreated,
    #[error("opened store contains another world")]
    OpenedWorldMismatch,
    #[error("world ownership is uncertain after command {command_id:?}; drop and reopen")]
    RecoveryRequired { command_id: CommandId },
    #[error("world store path no longer names the owned database; drop and reopen explicitly")]
    OwnershipLost,
    #[error("canonical serialization failed: {0}")]
    Serialization(String),
    #[error("world store operation failed: {0}")]
    Store(String),
    #[error("world journal is corrupt: {0}")]
    CorruptJournal(String),
    #[error("private reducer invariant failed: {0}")]
    Invariant(String),
}

impl KernelError {
    fn requires_owner_restart(&self) -> bool {
        matches!(
            self,
            Self::RecoveryRequired { .. }
                | Self::OwnershipLost
                | Self::Store(_)
                | Self::CorruptJournal(_)
                | Self::Invariant(_)
        )
    }
}

impl From<journal::JournalError> for KernelError {
    fn from(error: journal::JournalError) -> Self {
        match error {
            journal::JournalError::WorldMismatch => Self::OpenedWorldMismatch,
            journal::JournalError::RecoveryRequired { command_id } => {
                Self::RecoveryRequired { command_id }
            }
            journal::JournalError::OwnershipLost => Self::OwnershipLost,
            journal::JournalError::Store(detail) => Self::Store(detail),
            journal::JournalError::Corrupt(detail) => Self::CorruptJournal(detail),
        }
    }
}

struct WorldKernel {
    state: WorldState,
    journal: journal::WorldJournal,
}

struct PreparedCreation {
    command: CreateWorld,
    world_id: WorldId,
    owner: PrincipalId,
    title: String,
    resolved: ResolvedPatch,
}

fn prepare_creation(
    input: CreateWorld,
    authenticated: &AuthenticatedCaller,
) -> Result<PreparedCreation, KernelError> {
    let CallerId::Principal(authenticated_principal) = &authenticated.caller else {
        return Err(KernelError::AuthenticationMismatch);
    };
    validate_principal(&input.owner)?;
    if &input.owner != authenticated_principal {
        return Err(KernelError::AuthenticationMismatch);
    }
    let title = normalize_title(&input.title)?;
    // The world's own identity is not world structure, and it feeds every
    // derived ID, so it is minted before resolution rather than by it.
    let world_id = WorldId::issue();
    let resolved = patch::resolve_patch(
        &WorldState::empty(world_id, input.owner.clone(), title.clone()),
        input.id,
        &input.patch,
    )
    .map_err(KernelError::PatchRejected)?;
    Ok(PreparedCreation {
        world_id,
        owner: input.owner.clone(),
        title,
        resolved,
        command: input,
    })
}

impl WorldKernel {
    fn initialize(
        empty: journal::EmptyWorldJournal,
        prepared: PreparedCreation,
    ) -> Result<(Self, CreationReceipt), KernelError> {
        let world_id = prepared.world_id;
        let effect = WorldEffect::WorldCreated {
            owner: prepared.owner,
            title: prepared.title,
            resolved: prepared.resolved,
        };
        let mut state = WorldState::genesis(world_id, &prepared.command, &effect)?;
        let mut genesis = WorldCommit {
            schema: COMMIT_SCHEMA.into(),
            world_id,
            command: CommittedCommand::CreateWorld(prepared.command),
            previous_revision: None,
            resulting_revision: 0,
            previous_state_digest: None,
            resulting_state_digest: state.state_digest.clone(),
            previous_commit_digest: None,
            effect,
            committed_at: Utc::now(),
            digest: String::new(),
        };
        genesis.digest = commit_digest(&genesis)?;
        state.last_commit_digest = Some(genesis.digest.clone());
        let receipt = CreationReceipt::from_commit(&genesis)?;
        let journal = empty.initialize(&state, &genesis)?;
        Ok((Self { state, journal }, receipt))
    }

    fn retry_creation(&self, prepared: &PreparedCreation) -> Result<CreationReceipt, KernelError> {
        self.journal.ensure_healthy()?;
        let Some(commit) = self.journal.commit_for(prepared.command.id) else {
            return Err(KernelError::CreationTargetOccupied);
        };
        if commit.command != CommittedCommand::CreateWorld(prepared.command.clone())
            || !matches!(&commit.effect, WorldEffect::WorldCreated { .. })
        {
            return Err(KernelError::CreationConflict);
        }
        CreationReceipt::from_commit(commit)
    }

    #[cfg(test)]
    fn create(
        path: impl AsRef<Path>,
        input: CreateWorld,
        authenticated: &AuthenticatedCaller,
    ) -> Result<(Self, CreationReceipt), KernelError> {
        let prepared = prepare_creation(input, authenticated)?;
        match journal::WorldJournal::open_owner(path.as_ref())? {
            journal::JournalOpen::Empty(empty) => Self::initialize(empty, prepared),
            journal::JournalOpen::Live { journal, state } => {
                let kernel = Self { state, journal };
                let receipt = kernel.retry_creation(&prepared)?;
                Ok((kernel, receipt))
            }
        }
    }

    #[cfg(test)]
    fn open(path: impl AsRef<Path>, expected_world_id: WorldId) -> Result<Self, KernelError> {
        let (journal, state) = journal::WorldJournal::open(path.as_ref(), expected_world_id)?;
        Ok(Self { state, journal })
    }

    fn snapshot(&self) -> Result<WorldSnapshot, KernelError> {
        self.journal.ensure_healthy()?;
        snapshot(&self.state)
    }

    fn submit(
        &mut self,
        command: CommandEnvelope,
        authenticated: &AuthenticatedCaller,
    ) -> Result<SubmitReceipt, KernelError> {
        let committed_at = Utc::now();
        self.journal.ensure_healthy()?;
        if command.world_id != self.state.world_id {
            return Err(KernelError::WorldMismatch);
        }
        if command.caller != authenticated.caller {
            return Err(KernelError::AuthenticationMismatch);
        }
        // Retry identity is caller plus body. The expected revision is stamped by
        // the mailbox owner rather than supplied by the caller, so it is not part
        // of what makes a command the same command.
        if let Some(commit) = self.committed_command(command.id) {
            let CommittedCommand::WorldCommand(committed) = &commit.command else {
                return Err(KernelError::CommandIdConflict);
            };
            return if committed.caller == command.caller && committed.body == command.body {
                Ok(SubmitReceipt::AlreadyApplied(CommitReceipt::from(commit)))
            } else {
                Err(KernelError::CommandIdConflict)
            };
        }
        if command.expected_revision != self.state.revision {
            return Err(KernelError::RevisionMismatch {
                expected: command.expected_revision,
                actual: self.state.revision,
            });
        }

        let effect = reduce(&self.state, &command)?;
        let mut candidate = self.state.clone();
        apply_effect(&mut candidate, command.id, &command.caller, &effect)?;
        candidate.revision = candidate
            .revision
            .checked_add(1)
            .ok_or_else(|| KernelError::Serialization("world revision overflow".into()))?;
        candidate.state_digest = state_digest(&candidate)?;

        let mut commit = WorldCommit {
            schema: COMMIT_SCHEMA.into(),
            world_id: self.state.world_id,
            command: CommittedCommand::WorldCommand(command),
            previous_revision: Some(self.state.revision),
            resulting_revision: candidate.revision,
            previous_state_digest: Some(self.state.state_digest.clone()),
            resulting_state_digest: candidate.state_digest.clone(),
            previous_commit_digest: self.state.last_commit_digest.clone(),
            effect,
            committed_at,
            digest: String::new(),
        };
        commit.digest = commit_digest(&commit)?;
        candidate.last_commit_digest = Some(commit.digest.clone());

        self.journal.commit(&candidate, &commit)?;
        self.state = candidate;
        Ok(SubmitReceipt::Applied(CommitReceipt::from(&commit)))
    }

    fn committed_command(&self, command_id: CommandId) -> Option<&WorldCommit> {
        self.journal.commit_for(command_id)
    }

    fn controller_receipt(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
        invocation: &DecisionInvocation,
    ) -> Result<Option<CommitReceipt>, KernelError> {
        self.journal.ensure_healthy()?;
        let Some(commit) = self.committed_command(command_id) else {
            return Ok(None);
        };
        let CommittedCommand::WorldCommand(CommandEnvelope {
            body:
                CommandBody::ExerciseDecision {
                    opportunity: committed_opportunity,
                    invocation: committed_invocation,
                },
            ..
        }) = &commit.command
        else {
            return Err(KernelError::CommandIdConflict);
        };
        if committed_opportunity != opportunity || committed_invocation != invocation {
            return Err(KernelError::CommandIdConflict);
        }
        Ok(Some(CommitReceipt::from(commit)))
    }

    fn controller_decline_receipt(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
    ) -> Result<Option<CommitReceipt>, KernelError> {
        self.journal.ensure_healthy()?;
        let Some(commit) = self.committed_command(command_id) else {
            return Ok(None);
        };
        let CommittedCommand::WorldCommand(CommandEnvelope {
            body:
                CommandBody::DeclineDecision {
                    opportunity: committed_opportunity,
                },
            ..
        }) = &commit.command
        else {
            return Err(KernelError::CommandIdConflict);
        };
        if committed_opportunity != opportunity {
            return Err(KernelError::CommandIdConflict);
        }
        Ok(Some(CommitReceipt::from(commit)))
    }
}

fn reduce(state: &WorldState, command: &CommandEnvelope) -> Result<WorldEffect, KernelError> {
    match &command.body {
        CommandBody::ApproveDraft => {
            require_phase(state, WorldPhase::Draft)?;
            let CallerId::Principal(principal) = &command.caller else {
                return Err(KernelError::NotDraftApprover);
            };
            if !required_approvers(state).contains(principal) {
                return Err(KernelError::NotDraftApprover);
            }
            if state.draft_approvals.contains(principal) {
                return Err(KernelError::DraftAlreadyApproved);
            }
            Ok(WorldEffect::DraftApproved {
                principal: principal.clone(),
            })
        }
        CommandBody::ActivateWorld => {
            require_owner(state, &command.caller)?;
            require_phase(state, WorldPhase::Draft)?;
            let missing: Vec<_> = required_approvers(state)
                .difference(&state.draft_approvals)
                .cloned()
                .collect();
            if !missing.is_empty() {
                return Err(KernelError::MissingApprovals(missing));
            }
            Ok(WorldEffect::WorldActivated)
        }
        CommandBody::ExerciseDecision {
            opportunity,
            invocation,
        } => {
            require_phase(state, WorldPhase::Active)?;
            let current = exact_opportunity(state, opportunity)?;
            let assignment = state
                .controller_assignments
                .get(&current.scope)
                .ok_or(KernelError::OpportunityMismatch)?;
            if assignment.expected_caller() != command.caller {
                return Err(KernelError::ControllerMismatch);
            }
            let granted = require_granted(state, &current, invocation.affordance)?;
            let event = action::exercise(state, command.id, &current, &granted, invocation)?;
            Ok(WorldEffect::DecisionExercised {
                opportunity: current,
                event,
            })
        }
        CommandBody::DeclineDecision { opportunity } => {
            require_phase(state, WorldPhase::Active)?;
            let current = exact_opportunity(state, opportunity)?;
            let assignment = state
                .controller_assignments
                .get(&current.scope)
                .ok_or(KernelError::OpportunityMismatch)?;
            if assignment.expected_caller() != command.caller {
                return Err(KernelError::ControllerMismatch);
            }
            Ok(WorldEffect::DecisionDeclined {
                opportunity: current,
            })
        }
        CommandBody::AdmitPatch { answers: _, patch } => {
            require_owner(state, &command.caller)?;
            // Draft admits declarations, evidence, and operations. Active admits
            // operations only, so it never mints a canonical ID.
            if state.phase == WorldPhase::Active
                && !(patch.declarations.is_empty() && patch.evidence.is_empty())
            {
                return Err(KernelError::WrongPhase {
                    expected: WorldPhase::Draft,
                    actual: WorldPhase::Active,
                });
            }
            let resolved = patch::resolve_patch(state, command.id, patch)
                .map_err(KernelError::PatchRejected)?;
            Ok(WorldEffect::PatchAdmitted { resolved })
        }
    }
}

/// A human principal joins `required_approvers`, so only the lane that builds
/// revision 0 from nothing may bind one. The predicate reads state, never the
/// caller. It takes the two fields it inspects rather than a whole
/// `WorldState` so the genesis lane, which resolves declarations before any
/// `WorldState` exists, can call the same owner instead of hand-copying the
/// literal it would otherwise always evaluate to.
fn admits_human(revision: u64, subjects: &BTreeMap<SubjectId, SubjectState>) -> bool {
    revision == 0 && subjects.is_empty()
}

impl WorldState {
    /// The world before any structure. Genesis resolves its own patch against
    /// this value, so the genesis lane and `AdmitPatch` share one resolver
    /// signature instead of hand-passing empty partitions.
    fn empty(world_id: WorldId, owner: PrincipalId, title: String) -> Self {
        Self {
            schema: STATE_SCHEMA.into(),
            world_id,
            revision: 0,
            phase: WorldPhase::Draft,
            owner,
            title,
            draft_approvals: BTreeSet::new(),
            subjects: BTreeMap::new(),
            entities: BTreeMap::new(),
            edges: BTreeMap::new(),
            positions: BTreeMap::new(),
            holdings: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            authority: BTreeMap::new(),
            selection: BTreeMap::new(),
            redress: BTreeMap::new(),
            controller_assignments: BTreeMap::new(),
            affordance_catalog: BTreeMap::new(),
            affordance_grants: BTreeMap::new(),
            events: Vec::new(),
            state_digest: String::new(),
            last_commit_digest: None,
        }
    }

    fn genesis(
        world_id: WorldId,
        command: &CreateWorld,
        effect: &WorldEffect,
    ) -> Result<Self, KernelError> {
        let WorldEffect::WorldCreated {
            owner,
            title,
            resolved,
        } = effect
        else {
            return Err(KernelError::Invariant(
                "genesis state requires a world-created effect".into(),
            ));
        };
        validate_principal(&command.owner)?;
        let expected_title = normalize_title(&command.title)?;
        if owner != &command.owner || title != &expected_title {
            return Err(KernelError::Invariant(
                "genesis effect does not match the admitted creation command".into(),
            ));
        }
        // The same re-derive-and-compare that `apply_committed_command` runs for
        // every other command. Deterministic allocation is what lets one
        // equality replace a field-by-field binding zip.
        let mut state = Self::empty(world_id, owner.clone(), title.clone());
        let expected = patch::resolve_patch(&state, command.id, &command.patch)
            .map_err(KernelError::PatchRejected)?;
        if &expected != resolved {
            return Err(KernelError::Invariant(
                "genesis effect does not derive from its creation command".into(),
            ));
        }
        admit_resolved(&mut state, resolved)?;
        state.state_digest = state_digest(&state)?;
        Ok(state)
    }
}

/// The only writer of every ontology partition. Both admission lanes mutate
/// through it in one fixed order — entities, routes, subjects, operations — so a
/// patch may relocate along a route it declares. It re-derives every structural
/// claim from `state`, so an effect that skipped resolution dies here.
fn admit_resolved(state: &mut WorldState, resolved: &ResolvedPatch) -> Result<(), KernelError> {
    if resolved.declares_nothing() && resolved.operations.is_empty() {
        return Err(KernelError::Invariant(
            "admitted patch carries no canonical change".into(),
        ));
    }
    let humans_admitted = admits_human(state.revision, &state.subjects);
    for entity in &resolved.entities {
        if !patch::is_canonical_text(&entity.entity.label) {
            return Err(KernelError::Invariant(
                "admitted entity label is not canonical".into(),
            ));
        }
        if let Some(container) = entity.entity.container
            && (entity.entity.kind != EntityKind::Place
                || state
                    .entities
                    .get(&container)
                    .is_none_or(|record| record.kind != EntityKind::Place))
        {
            return Err(KernelError::Invariant(
                "admitted container does not name a canonical place".into(),
            ));
        }
        if state
            .entities
            .insert(entity.entity_id, entity.entity.clone())
            .is_some()
        {
            return Err(KernelError::Invariant(
                "admitted entity ID collision".into(),
            ));
        }
        if !patch::containment_terminates(entity.entity_id, &state.entities) {
            return Err(KernelError::Invariant(
                "admitted place contains itself".into(),
            ));
        }
    }
    for route in &resolved.routes {
        let (from, to) = route.edge.endpoints();
        let endpoint_is_place = |entity_id: &EntityId| {
            state
                .entities
                .get(entity_id)
                .is_some_and(|record| record.kind == EntityKind::Place)
        };
        if !patch::is_canonical_text(route.edge.label())
            || !endpoint_is_place(&from)
            || !endpoint_is_place(&to)
            || from == to
            || !patch::is_valid_cost(route.edge.cost())
        {
            return Err(KernelError::Invariant(
                "admitted route is noncanonical or does not join two places".into(),
            ));
        }
        if state
            .edges
            .insert(route.edge_id, route.edge.clone())
            .is_some()
        {
            return Err(KernelError::Invariant("admitted route ID collision".into()));
        }
    }
    let mut controller_ids: BTreeSet<ControllerId> = state
        .controller_assignments
        .values()
        .map(ControllerAssignment::id)
        .collect();
    for entry in &resolved.affordances {
        if state
            .affordance_catalog
            .insert(entry.affordance_id, entry.affordance.clone())
            .is_some()
        {
            return Err(KernelError::Invariant(
                "admitted affordance ID collision".into(),
            ));
        }
    }
    for subject in &resolved.subjects {
        let scope = DecisionScope {
            subject_id: subject.subject_id,
        };
        if !patch::is_canonical_text(&subject.subject.label) {
            return Err(KernelError::Invariant(
                "admitted subject label is not canonical".into(),
            ));
        }
        validate_assignment(&subject.controller)?;
        if matches!(subject.controller, ControllerAssignment::Human { .. }) && !humans_admitted {
            return Err(KernelError::Invariant(
                "only world genesis may bind a human controller".into(),
            ));
        }
        if subject.affordances.is_empty() {
            return Err(KernelError::Invariant(
                "admitted subject has no affordance".into(),
            ));
        }
        if let Some(position) = subject.position
            && state
                .entities
                .get(&position.place)
                .is_none_or(|entity| entity.kind != EntityKind::Place)
        {
            return Err(KernelError::Invariant(
                "admitted position does not name a canonical place".into(),
            ));
        }
        if !controller_ids.insert(subject.controller.id()) {
            return Err(KernelError::Invariant(
                "admitted controller ID collision".into(),
            ));
        }
        if state
            .subjects
            .insert(subject.subject_id, subject.subject.clone())
            .is_some()
            || state
                .controller_assignments
                .insert(scope, subject.controller.clone())
                .is_some()
        {
            return Err(KernelError::Invariant(
                "admitted subject or scope ID collision".into(),
            ));
        }
        if let Some(position) = subject.position
            && state
                .positions
                .insert(subject.subject_id, position)
                .is_some()
        {
            return Err(KernelError::Invariant(
                "admitted position collides with an existing one".into(),
            ));
        }
        for affordance_id in &subject.affordances {
            if !state.affordance_catalog.contains_key(affordance_id) {
                return Err(KernelError::Invariant(
                    "admitted grant names no catalog entry".into(),
                ));
            }
            state
                .affordance_grants
                .entry(scope)
                .or_default()
                .insert(*affordance_id);
        }
    }
    apply_operations(state, &resolved.operations, &resolved.evidence)
}

/// The component writer for every lane: snapshot the before-totals for every
/// resource the operations name, apply each operation, accumulate the ledger
/// deltas, and prove conservation once. Declaration admission calls it after its
/// declaration loops; an exercised affordance's lowered effects call it
/// directly. One writer, one conservation statement.
///
/// A forged effect reaches this function without ever passing the resolver, so
/// conservation is re-derived over the committed partitions, against the same
/// equation, with `before` read from state rather than from anything the effect
/// asserts.
fn apply_operations(
    state: &mut WorldState,
    operations: &[ResolvedOp],
    evidence: &[EvidenceRef],
) -> Result<(), KernelError> {
    let mut deltas: BTreeMap<EntityId, LedgerDelta> = BTreeMap::new();
    for operation in operations {
        for resource in operation_resources(operation) {
            deltas.entry(resource).or_default().before = resource_total(state, resource);
        }
    }
    for operation in operations {
        apply_operation(state, operation, evidence)?;
        match operation {
            ResolvedOp::Transform {
                from_resource,
                into_resource,
                qty,
                ..
            } => {
                deltas.entry(*from_resource).or_default().spent += u128::from(qty.0);
                deltas.entry(*into_resource).or_default().gained += u128::from(qty.0);
            }
            ResolvedOp::Consume { resource, qty, .. } => {
                deltas.entry(*resource).or_default().consumed += u128::from(qty.0);
            }
            ResolvedOp::Admit { resource, qty, .. } => {
                deltas.entry(*resource).or_default().admitted += u128::from(qty.0);
            }
            _ => {}
        }
    }
    for (resource, delta) in &mut deltas {
        delta.after = resource_total(state, *resource);
    }
    if patch::check_ledger(&deltas).is_some() {
        return Err(KernelError::Invariant("custody does not conserve".into()));
    }
    // Disjointness, re-derived over the committed partitions for the same
    // reason conservation is: a forged effect reaches this function without
    // passing the resolver.
    if operations.iter().any(|operation| {
        matches!(
            operation,
            ResolvedOp::GrantAuthority { .. }
                | ResolvedOp::OpenOffice { .. }
                | ResolvedOp::InstallIncumbent { .. }
        )
    }) && overlapping_holder(state).is_some()
    {
        return Err(KernelError::Invariant(
            "one subject holds two overlapping jurisdictions of one kind".into(),
        ));
    }
    Ok(())
}

/// Every resource an operation names, so `admit_resolved` can read its committed
/// total before touching it.
fn operation_resources(operation: &ResolvedOp) -> Vec<EntityId> {
    match operation {
        ResolvedOp::Transfer { resource, .. }
        | ResolvedOp::Consume { resource, .. }
        | ResolvedOp::Admit { resource, .. } => vec![*resource],
        ResolvedOp::Transform {
            from_resource,
            into_resource,
            ..
        } => vec![*from_resource, *into_resource],
        _ => Vec::new(),
    }
}

fn resource_total(state: &WorldState, resource: EntityId) -> u128 {
    state
        .holdings
        .values()
        .filter_map(|held| held.get(&resource))
        .map(|quantity| u128::from(quantity.0))
        .sum()
}

/// What a holder holds. Absence is zero.
fn held(state: &WorldState, holder: SubjectId, resource: EntityId) -> u64 {
    state
        .holdings
        .get(&holder)
        .and_then(|held| held.get(&resource))
        .map_or(0, |quantity| quantity.0)
}

/// Zero removes the resource key, and an emptied holder removes the holder key,
/// so `holdings` has exactly one representation of nothing.
fn set_held(state: &mut WorldState, holder: SubjectId, resource: EntityId, value: u64) {
    if value == 0 {
        let empty = if let Some(held) = state.holdings.get_mut(&holder) {
            held.remove(&resource);
            held.is_empty()
        } else {
            false
        };
        if empty {
            state.holdings.remove(&holder);
        }
    } else {
        state
            .holdings
            .entry(holder)
            .or_default()
            .insert(resource, Quantity(value));
    }
}

/// Whether a subject and a resource are both canonical and of the right kind.
fn custody_referents_exist(state: &WorldState, holder: SubjectId, resource: EntityId) -> bool {
    state.subjects.contains_key(&holder)
        && state
            .entities
            .get(&resource)
            .is_some_and(|record| record.kind == EntityKind::Resource)
}

fn dependency_target_exists(state: &WorldState, target: DependencyTarget) -> bool {
    match target {
        DependencyTarget::Resource(entity_id) => state
            .entities
            .get(&entity_id)
            .is_some_and(|record| record.kind == EntityKind::Resource),
        DependencyTarget::Route(edge_id) => state.edges.contains_key(&edge_id),
        DependencyTarget::Subject(subject_id) => state.subjects.contains_key(&subject_id),
    }
}

/// The component half of admission. Every precondition is re-derived from the
/// partitions, so a forged operation cannot assert a move the topology refuses.
fn apply_operation(
    state: &mut WorldState,
    operation: &ResolvedOp,
    evidence: &[EvidenceRef],
) -> Result<(), KernelError> {
    let insufficient = || KernelError::Invariant("holder does not hold enough".into());
    let overflow = || KernelError::Invariant("holding would overflow".into());
    let unknown = || KernelError::Invariant("custody operation names no canonical referent".into());
    let zero = || KernelError::Invariant("custody operation moves nothing".into());
    match operation {
        ResolvedOp::Relocate {
            subject_id,
            edge_id,
        } => {
            let route = state
                .edges
                .get(edge_id)
                .ok_or_else(|| KernelError::Invariant("relocation names no route".into()))?;
            let (from, to) = route.endpoints();
            let open = route.is_open();
            let access = route.access().clone();
            if !open
                || state.positions.get(subject_id) != Some(&Position { place: from })
                || !patch::route_admits(state, &subject_authority(state, *subject_id), &access, to)
            {
                return Err(KernelError::Invariant(
                    "relocation does not traverse an open route the subject may take from its place"
                        .into(),
                ));
            }
            state.positions.insert(*subject_id, Position { place: to });
        }
        ResolvedOp::OpenRoute { edge_id } | ResolvedOp::CloseRoute { edge_id } => {
            let open = matches!(operation, ResolvedOp::OpenRoute { .. });
            let route = state
                .edges
                .get_mut(edge_id)
                .ok_or_else(|| KernelError::Invariant("route operation names no route".into()))?;
            if route.is_open() == open {
                return Err(KernelError::Invariant(
                    "route operation changes nothing".into(),
                ));
            }
            route.set_open(open);
        }
        ResolvedOp::AlterCost { edge_id, cost } => {
            if !patch::is_valid_cost(*cost) {
                return Err(KernelError::Invariant(
                    "admitted cost is out of range".into(),
                ));
            }
            let route = state
                .edges
                .get_mut(edge_id)
                .ok_or_else(|| KernelError::Invariant("route operation names no route".into()))?;
            if route.cost() == *cost {
                return Err(KernelError::Invariant(
                    "route operation changes nothing".into(),
                ));
            }
            route.set_cost(*cost);
        }
        ResolvedOp::Transfer {
            from,
            to,
            resource,
            qty,
        } => {
            if !custody_referents_exist(state, *from, *resource) || !state.subjects.contains_key(to)
            {
                return Err(unknown());
            }
            if qty.0 == 0 || from == to {
                return Err(zero());
            }
            let remaining = held(state, *from, *resource)
                .checked_sub(qty.0)
                .ok_or_else(insufficient)?;
            let credited = held(state, *to, *resource)
                .checked_add(qty.0)
                .ok_or_else(overflow)?;
            set_held(state, *from, *resource, remaining);
            set_held(state, *to, *resource, credited);
        }
        ResolvedOp::Transform {
            holder,
            from_resource,
            into_resource,
            qty,
        } => {
            if !custody_referents_exist(state, *holder, *from_resource)
                || !custody_referents_exist(state, *holder, *into_resource)
            {
                return Err(unknown());
            }
            if qty.0 == 0 || from_resource == into_resource {
                return Err(zero());
            }
            let remaining = held(state, *holder, *from_resource)
                .checked_sub(qty.0)
                .ok_or_else(insufficient)?;
            let gained = held(state, *holder, *into_resource)
                .checked_add(qty.0)
                .ok_or_else(overflow)?;
            set_held(state, *holder, *from_resource, remaining);
            set_held(state, *holder, *into_resource, gained);
        }
        ResolvedOp::Consume {
            holder,
            resource,
            qty,
        } => {
            if !custody_referents_exist(state, *holder, *resource) {
                return Err(unknown());
            }
            if qty.0 == 0 {
                return Err(zero());
            }
            let remaining = held(state, *holder, *resource)
                .checked_sub(qty.0)
                .ok_or_else(insufficient)?;
            set_held(state, *holder, *resource, remaining);
        }
        ResolvedOp::Admit {
            holder,
            resource,
            qty,
            evidence: cited,
        } => {
            if !custody_referents_exist(state, *holder, *resource) {
                return Err(unknown());
            }
            if qty.0 == 0 {
                return Err(zero());
            }
            if !evidence.contains(cited) {
                return Err(KernelError::Invariant(
                    "admitted quantity cites no evidence in its patch".into(),
                ));
            }
            let admitted = held(state, *holder, *resource)
                .checked_add(qty.0)
                .ok_or_else(overflow)?;
            set_held(state, *holder, *resource, admitted);
        }
        ResolvedOp::Bind { subject, target } | ResolvedOp::Release { subject, target } => {
            let bind = matches!(operation, ResolvedOp::Bind { .. });
            if !state.subjects.contains_key(subject) || !dependency_target_exists(state, *target) {
                return Err(unknown());
            }
            if *target == DependencyTarget::Subject(*subject) {
                return Err(KernelError::Invariant(
                    "a subject cannot depend on itself".into(),
                ));
            }
            let bound = state
                .dependencies
                .get(subject)
                .is_some_and(|targets| targets.contains(target));
            if bound == bind {
                return Err(KernelError::Invariant(
                    "dependency operation changes nothing".into(),
                ));
            }
            if bind {
                state
                    .dependencies
                    .entry(*subject)
                    .or_default()
                    .insert(*target);
            } else {
                let empty = if let Some(targets) = state.dependencies.get_mut(subject) {
                    targets.remove(target);
                    targets.is_empty()
                } else {
                    false
                };
                if empty {
                    state.dependencies.remove(subject);
                }
            }
        }
        ResolvedOp::GrantAuthority { holder, grant }
        | ResolvedOp::RevokeAuthority { holder, grant } => {
            let granting = matches!(operation, ResolvedOp::GrantAuthority { .. });
            if !state.subjects.contains_key(holder)
                || !authority_referent_exists(state, grant.over)
                || !patch::is_civic_name(&grant.kind.0)
            {
                return Err(unknown());
            }
            let held = state
                .authority
                .get(holder)
                .is_some_and(|grants| grants.contains(grant));
            if held == granting {
                return Err(KernelError::Invariant(
                    "authority operation changes nothing".into(),
                ));
            }
            if granting {
                state
                    .authority
                    .entry(*holder)
                    .or_default()
                    .insert(grant.clone());
            } else {
                let empty = if let Some(grants) = state.authority.get_mut(holder) {
                    grants.remove(grant);
                    grants.is_empty()
                } else {
                    false
                };
                if empty {
                    state.authority.remove(holder);
                }
            }
        }
        ResolvedOp::OpenOffice {
            institution,
            office,
            delegated,
        } => {
            require_institution(state, *institution, office)?;
            if delegated.is_empty() || delegated.iter().any(|kind| !patch::is_civic_name(&kind.0)) {
                return Err(KernelError::Invariant(
                    "an office lends no canonical authority kind".into(),
                ));
            }
            let current = state
                .selection
                .get(institution)
                .and_then(|offices| offices.get(office));
            if current.is_some_and(|seat| &seat.delegated == delegated) {
                return Err(KernelError::Invariant(
                    "office operation changes nothing".into(),
                ));
            }
            // A sitting incumbent survives a reconstitution: clipping an
            // office's powers under its holder is a political act, not a
            // vacancy.
            let incumbent = current.and_then(|seat| seat.incumbent);
            state.selection.entry(*institution).or_default().insert(
                office.clone(),
                Office {
                    incumbent,
                    delegated: delegated.clone(),
                },
            );
        }
        ResolvedOp::CloseOffice {
            institution,
            office,
        }
        | ResolvedOp::VacateOffice {
            institution,
            office,
        } => {
            let closing = matches!(operation, ResolvedOp::CloseOffice { .. });
            require_institution(state, *institution, office)?;
            let no_office = || KernelError::Invariant("office operation names no office".into());
            let offices = state.selection.get_mut(institution).ok_or_else(no_office)?;
            let occupied = offices
                .get(office)
                .map(|seat| seat.incumbent.is_some())
                .ok_or_else(no_office)?;
            if closing {
                offices.remove(office);
                let empty = offices.is_empty();
                if empty {
                    state.selection.remove(institution);
                }
            } else if !occupied {
                return Err(KernelError::Invariant(
                    "office operation changes nothing".into(),
                ));
            } else {
                offices.get_mut(office).ok_or_else(no_office)?.incumbent = None;
            }
        }
        ResolvedOp::InstallIncumbent {
            institution,
            office,
            incumbent,
        } => {
            require_institution(state, *institution, office)?;
            if state.subjects.get(incumbent).map(|subject| subject.kind)
                != Some(SubjectKind::Person)
            {
                return Err(KernelError::Invariant(
                    "an office holder is not a person subject".into(),
                ));
            }
            let offices = state
                .selection
                .get_mut(institution)
                .ok_or_else(|| KernelError::Invariant("office operation names no office".into()))?;
            if offices
                .iter()
                .any(|(name, seat)| name != office && seat.incumbent == Some(*incumbent))
            {
                return Err(KernelError::Invariant(
                    "one person holds two offices of one institution".into(),
                ));
            }
            let seat = offices
                .get_mut(office)
                .ok_or_else(|| KernelError::Invariant("office operation names no office".into()))?;
            if seat.incumbent == Some(*incumbent) {
                return Err(KernelError::Invariant(
                    "office operation changes nothing".into(),
                ));
            }
            seat.incumbent = Some(*incumbent);
        }
        ResolvedOp::OpenForum {
            grievance,
            forum,
            standing,
        } => {
            if !state.subjects.contains_key(forum)
                || !authority_referent_exists(state, *standing)
                || !patch::is_civic_name(&grievance.0)
            {
                return Err(unknown());
            }
            let seat = Forum {
                forum: *forum,
                standing: *standing,
            };
            if state.redress.get(grievance) == Some(&seat) {
                return Err(KernelError::Invariant(
                    "forum operation changes nothing".into(),
                ));
            }
            state.redress.insert(grievance.clone(), seat);
        }
        ResolvedOp::CloseForum { grievance } => {
            if state.redress.remove(grievance).is_none() {
                return Err(KernelError::Invariant(
                    "forum operation names no forum".into(),
                ));
            }
        }
    }
    Ok(())
}

/// Whether an authority target names a live subject or a live place.
fn authority_referent_exists(state: &WorldState, target: AuthorityTarget) -> bool {
    match target {
        AuthorityTarget::Subject(subject_id) => state.subjects.contains_key(&subject_id),
        AuthorityTarget::PlaceSubtree(entity_id) => state
            .entities
            .get(&entity_id)
            .is_some_and(|record| record.kind == EntityKind::Place),
    }
}

/// The shared half of every office operation: a canonical office name on a live
/// institution subject.
fn require_institution(
    state: &WorldState,
    institution: SubjectId,
    office: &OfficeName,
) -> Result<(), KernelError> {
    if !patch::is_civic_name(&office.0) {
        return Err(KernelError::Invariant(
            "office name is not canonical".into(),
        ));
    }
    if state.subjects.get(&institution).map(|subject| subject.kind)
        != Some(SubjectKind::Institution)
    {
        return Err(KernelError::Invariant(
            "an office was opened on a subject that is not an institution".into(),
        ));
    }
    Ok(())
}

/// The subject, if any, whose effective authority holds two grants of one kind
/// over overlapping ground. Re-derived over the committed partitions after every
/// batch that could widen a jurisdiction, so a forged effect cannot install the
/// split-brained state the resolver refuses.
fn overlapping_holder(state: &WorldState) -> Option<SubjectId> {
    let holders: BTreeSet<SubjectId> = state
        .authority
        .keys()
        .copied()
        .chain(
            state
                .selection
                .values()
                .flat_map(|offices| offices.values())
                .filter_map(|office| office.incumbent),
        )
        .collect();
    holders.into_iter().find(|holder| {
        let effective: Vec<AuthorityGrant> =
            subject_authority(state, *holder).into_iter().collect();
        effective.iter().enumerate().any(|(index, one)| {
            effective[index + 1..].iter().any(|other| {
                one.kind == other.kind
                    && patch::targets_overlap(&state.entities, one.over, other.over)
            })
        })
    })
}

/// Exactly the components a subject's verification reads. One owner, consumed by
/// the scope digest and by the snapshot, so the two cannot drift. `routes` is
/// every edge whose `from` or `to` is the subject's place: an inbound route
/// decides who can arrive, and reading too little is a correctness hole while
/// reading too much only costs an extra rejection. `holdings` and `dependencies`
/// are the acting subject's own; a counterparty's holdings do not enter, because
/// a transfer changes both subjects' components and so changes both digests.
/// It also holds what the actor *is* authorized over, and nothing about the
/// world the actor reaches: the target's position, the target's container
/// chain, the forum's state, and any subordinate's components stay out. A
/// realm-wide authority whose preimage held its jurisdiction's occupancy would
/// conflict with every commit in the realm, which is whole-world binding
/// restored by the front door. A target that walks out of a jurisdiction
/// mid-flight therefore fails at `Authorized` rather than rebinding, while a
/// revoked grant is a `ScopeChanged`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(super) struct ScopeComponents {
    position: Option<Position>,
    routes: BTreeMap<EdgeId, EdgeRecord>,
    holdings: BTreeMap<EntityId, Quantity>,
    dependencies: BTreeSet<DependencyTarget>,
    /// This subject's own grants.
    authority: BTreeSet<AuthorityGrant>,
    /// What each held office lends: for every office whose incumbent is this
    /// subject, the institution's live grants whose kind that office delegates.
    /// The institution's authority is copied in through the office link, so one
    /// scope digest covers the grant, the office, and the institution's
    /// jurisdiction, and only delegated kinds enter — an institution's `judge`
    /// jurisdiction never churns a `levy`-only incumbent's proposals.
    delegated: BTreeMap<SubjectId, BTreeMap<OfficeName, BTreeSet<AuthorityGrant>>>,
}

/// Whether an authority target covers a referent. The one statement of
/// jurisdictional membership, reached by `Authorized`, `HasStanding`,
/// `route_admits`, the delegation monotonicity rule, and the snapshot's redress
/// projection.
///
/// A place subtree covers a subject standing anywhere under it, a place under
/// it, and a route with either endpoint under it — a city must be able to close
/// its own gate, and a gate's far end is outside the city by construction. A
/// resource, fact, or channel is covered by nothing; `RoleKindUnfit` refuses an
/// `Authorized` over such a role at declaration.
fn covers(state: &WorldState, target: AuthorityTarget, of: Target) -> bool {
    let under = |place: EntityId, root: EntityId| patch::covers_place(&state.entities, root, place);
    match (target, of) {
        (AuthorityTarget::Subject(holder), Target::Subject(subject_id)) => holder == subject_id,
        (AuthorityTarget::Subject(_), _) => false,
        (AuthorityTarget::PlaceSubtree(root), Target::Subject(subject_id)) => state
            .positions
            .get(&subject_id)
            .is_some_and(|position| under(position.place, root)),
        (AuthorityTarget::PlaceSubtree(root), Target::Entity(entity_id)) => state
            .entities
            .get(&entity_id)
            .is_some_and(|record| record.kind == EntityKind::Place && under(entity_id, root)),
        (AuthorityTarget::PlaceSubtree(root), Target::Edge(edge_id)) => {
            state.edges.get(&edge_id).is_some_and(|record| {
                let (from, to) = record.endpoints();
                under(from, root) || under(to, root)
            })
        }
    }
}

/// Own grants plus what every held office lends. The single statement of what a
/// subject may invoke authority over. There is no precedence between a direct
/// grant and a delegated one, because `OverlappingJurisdiction` makes it
/// impossible for both to answer for one kind over overlapping ground.
fn effective_authority(
    own: &BTreeSet<AuthorityGrant>,
    delegated: &BTreeMap<SubjectId, BTreeMap<OfficeName, BTreeSet<AuthorityGrant>>>,
) -> BTreeSet<AuthorityGrant> {
    let mut effective = own.clone();
    for lent in delegated.values().flat_map(|offices| offices.values()) {
        effective.extend(lent.iter().cloned());
    }
    effective
}

/// Every office this subject occupies, keyed by institution, with what each one
/// lends of that institution's live grants.
fn delegated_authority(
    state: &WorldState,
    subject_id: SubjectId,
) -> BTreeMap<SubjectId, BTreeMap<OfficeName, BTreeSet<AuthorityGrant>>> {
    let mut held: BTreeMap<SubjectId, BTreeMap<OfficeName, BTreeSet<AuthorityGrant>>> =
        BTreeMap::new();
    for (institution, offices) in &state.selection {
        for (name, office) in offices {
            if office.incumbent != Some(subject_id) {
                continue;
            }
            let lent: BTreeSet<AuthorityGrant> = state
                .authority
                .get(institution)
                .into_iter()
                .flatten()
                .filter(|grant| office.delegated.contains(&grant.kind))
                .cloned()
                .collect();
            held.entry(*institution)
                .or_default()
                .insert(name.clone(), lent);
        }
    }
    held
}

/// The effective authority of one subject read straight from the partitions,
/// for the reducer paths that do not already hold a `ScopeComponents`.
fn subject_authority(state: &WorldState, subject_id: SubjectId) -> BTreeSet<AuthorityGrant> {
    let empty = BTreeSet::new();
    effective_authority(
        state.authority.get(&subject_id).unwrap_or(&empty),
        &delegated_authority(state, subject_id),
    )
}

fn scope_components(state: &WorldState, subject_id: SubjectId) -> ScopeComponents {
    let position = state.positions.get(&subject_id).copied();
    let routes = position
        .map(|position| {
            state
                .edges
                .iter()
                .filter(|(_, record)| {
                    let (from, to) = record.endpoints();
                    from == position.place || to == position.place
                })
                .map(|(edge_id, record)| (*edge_id, record.clone()))
                .collect()
        })
        .unwrap_or_default();
    ScopeComponents {
        position,
        routes,
        holdings: state.holdings.get(&subject_id).cloned().unwrap_or_default(),
        dependencies: state
            .dependencies
            .get(&subject_id)
            .cloned()
            .unwrap_or_default(),
        authority: state
            .authority
            .get(&subject_id)
            .cloned()
            .unwrap_or_default(),
        delegated: delegated_authority(state, subject_id),
    }
}

fn snapshot(state: &WorldState) -> Result<WorldSnapshot, KernelError> {
    let subjects = state
        .subjects
        .iter()
        .map(|(subject_id, subject)| {
            let scope = DecisionScope {
                subject_id: *subject_id,
            };
            let controller = state.controller_assignments.get(&scope).ok_or_else(|| {
                KernelError::Invariant("subject has no controller assignment".into())
            })?;
            let affordances = state
                .affordance_grants
                .get(&scope)
                .cloned()
                .unwrap_or_default();
            let components = scope_components(state, *subject_id);
            let offices_held = components
                .delegated
                .iter()
                .flat_map(|(institution, offices)| {
                    offices
                        .iter()
                        .map(move |(office, authority)| OfficeSnapshot {
                            institution: *institution,
                            office: office.clone(),
                            incumbent: Some(*subject_id),
                            authority: authority.clone(),
                        })
                })
                .collect();
            let offices_granted = state
                .selection
                .get(subject_id)
                .into_iter()
                .flatten()
                .map(|(office, seat)| OfficeSnapshot {
                    institution: *subject_id,
                    office: office.clone(),
                    incumbent: seat.incumbent,
                    authority: state
                        .authority
                        .get(subject_id)
                        .into_iter()
                        .flatten()
                        .filter(|grant| seat.delegated.contains(&grant.kind))
                        .cloned()
                        .collect(),
                })
                .collect();
            let redress = state
                .redress
                .iter()
                .filter(|(_, forum)| covers(state, forum.standing, Target::Subject(*subject_id)))
                .map(|(grievance, forum)| ForumSnapshot {
                    grievance: grievance.clone(),
                    forum: forum.forum,
                })
                .collect();
            Ok(SubjectSnapshot {
                id: *subject_id,
                label: subject.label.clone(),
                kind: subject.kind,
                controller_id: controller.id(),
                controller_mode: controller.mode(),
                human_controller: controller.human_principal().cloned(),
                affordances,
                position: components.position.map(|position| position.place),
                holdings: components.holdings,
                dependencies: components.dependencies,
                incident_routes: components.routes.into_keys().collect(),
                authority: components.authority,
                offices_held,
                offices_granted,
                redress,
            })
        })
        .collect::<Result<Vec<_>, KernelError>>()?;
    let places = state
        .entities
        .iter()
        .filter(|(_, record)| record.kind == EntityKind::Place)
        .map(|(entity_id, record)| PlaceSnapshot {
            id: *entity_id,
            label: record.label.clone(),
            container: record.container,
        })
        .collect();
    let resources = state
        .entities
        .iter()
        .filter(|(_, record)| record.kind == EntityKind::Resource)
        .map(|(entity_id, record)| ResourceSnapshot {
            id: *entity_id,
            label: record.label.clone(),
        })
        .collect();
    let routes = state
        .edges
        .iter()
        .map(|(edge_id, record)| {
            let (from, to) = record.endpoints();
            RouteSnapshot {
                id: *edge_id,
                label: record.label().to_owned(),
                from,
                to,
                access: record.access().clone(),
                cost: record.cost(),
                open: record.is_open(),
            }
        })
        .collect();
    Ok(WorldSnapshot {
        world_id: state.world_id,
        revision: state.revision,
        phase: state.phase,
        owner: state.owner.clone(),
        title: state.title.clone(),
        draft_approvals: state.draft_approvals.clone(),
        required_approvers: required_approvers(state),
        subjects,
        affordances: state
            .affordance_catalog
            .iter()
            .map(|(affordance_id, entry)| AffordanceSnapshot {
                id: *affordance_id,
                entry: entry.clone(),
            })
            .collect(),
        places,
        resources,
        routes,
        events: state.events.clone(),
        opportunities: derive_opportunities(state)?,
        state_digest: state.state_digest.clone(),
        last_commit_digest: state.last_commit_digest.clone(),
    })
}

fn derive_opportunities(state: &WorldState) -> Result<Vec<DecisionOpportunity>, KernelError> {
    if state.phase != WorldPhase::Active {
        return Ok(Vec::new());
    }
    state
        .controller_assignments
        .iter()
        .map(|(scope, controller)| {
            let affordance_ids: Vec<_> = state
                .affordance_grants
                .get(scope)
                .into_iter()
                .flatten()
                .copied()
                .collect();
            if affordance_ids.is_empty() {
                return Err(KernelError::Invariant(
                    "active decision scope has no affordances".into(),
                ));
            }
            Ok(DecisionOpportunity {
                world_id: state.world_id,
                revision: state.revision,
                scope_digest: scope_digest(state, *scope)?,
                scope: *scope,
                controller_id: controller.id(),
                controller_mode: controller.mode(),
                affordance_ids,
            })
        })
        .collect()
}

/// The entries one scope is granted, paired with their definitions. A grant
/// naming no catalog entry is a corrupt kernel rather than a rejection: one
/// owner writes both partitions in one commit.
fn granted_entries(
    state: &WorldState,
    scope: DecisionScope,
) -> Result<BTreeMap<AffordanceId, &Affordance>, KernelError> {
    state
        .affordance_grants
        .get(&scope)
        .into_iter()
        .flatten()
        .map(|affordance_id| {
            state
                .affordance_catalog
                .get(affordance_id)
                .map(|entry| (*affordance_id, entry))
                .ok_or_else(|| KernelError::Invariant("granted affordance has no entry".into()))
        })
        .collect()
}

/// One affordance the kernel has already gated: the opportunity offers this
/// entry, the scope holds it, and `entry` is its catalog definition. It is the
/// only way to name an affordance to `action::exercise`, so a caller cannot
/// reach the invocation pipeline without passing the grant check first.
pub(super) struct GrantedAffordance<'a> {
    pub(super) id: AffordanceId,
    pub(super) entry: &'a Affordance,
}

/// The membership half of the affordance check, shared by `reduce` and
/// `apply_effect`: the opportunity offers the entry and the scope holds it. It
/// returns the entry rather than a unit, so the gate and the lookup are one act.
fn require_granted<'a>(
    state: &'a WorldState,
    current: &DecisionOpportunity,
    affordance: AffordanceId,
) -> Result<GrantedAffordance<'a>, KernelError> {
    if current.affordance_ids.contains(&affordance)
        && state
            .affordance_grants
            .get(&current.scope)
            .is_some_and(|granted| granted.contains(&affordance))
    {
        let entry = state
            .affordance_catalog
            .get(&affordance)
            .ok_or_else(|| KernelError::Invariant("granted affordance has no entry".into()))?;
        Ok(GrantedAffordance {
            id: affordance,
            entry,
        })
    } else {
        Err(KernelError::AffordanceDenied)
    }
}

/// The sole producer of a `ScopeDigest`. The components it reads come from
/// `scope_components`, which the snapshot reads too.
fn scope_digest(state: &WorldState, scope: DecisionScope) -> Result<ScopeDigest, KernelError> {
    let controller = state
        .controller_assignments
        .get(&scope)
        .ok_or_else(|| KernelError::Invariant("decision scope has no controller".into()))?;
    let affordances = granted_entries(state, scope)?;
    let components = scope_components(state, scope.subject_id);
    digest(&ScopePreimage {
        world_id: state.world_id,
        subject_id: scope.subject_id,
        controller,
        affordances,
        components: &components,
    })
    .map(ScopeDigest)
}

/// Binding, which is one of the two checks a committed invocation passes.
/// Binding is scope-digest equality: has what the proposal was made against
/// moved? Admission — preconditions, effect ceilings, and the band draw, all in
/// `action::exercise` — is the other, and it runs at commit against the same
/// revision the digest was verified at. A component that admission reads but
/// binding does not is therefore fail-closed by re-check at commit rather than
/// by rejecting the proposal when it changes.
fn exact_opportunity(
    state: &WorldState,
    claimed: &DecisionOpportunity,
) -> Result<DecisionOpportunity, KernelError> {
    if claimed.world_id != state.world_id {
        return Err(KernelError::OpportunityMismatch);
    }
    let current = derive_opportunities(state)?
        .into_iter()
        .find(|current| current.scope == claimed.scope)
        .ok_or(KernelError::OpportunityMismatch)?;
    if claimed.revision > state.revision {
        return Err(KernelError::OpportunityMismatch);
    }
    if current.scope_digest != claimed.scope_digest {
        return Err(KernelError::ScopeChanged {
            scope: claimed.scope,
            expected: claimed.scope_digest.clone(),
            actual: current.scope_digest,
        });
    }
    Ok(current)
}

fn required_approvers(state: &WorldState) -> BTreeSet<PrincipalId> {
    std::iter::once(state.owner.clone())
        .chain(
            state
                .controller_assignments
                .values()
                .filter_map(ControllerAssignment::human_principal)
                .cloned(),
        )
        .collect()
}

fn require_owner(state: &WorldState, caller: &CallerId) -> Result<(), KernelError> {
    if caller == &CallerId::Principal(state.owner.clone()) {
        Ok(())
    } else {
        Err(KernelError::Unauthorized)
    }
}

fn require_phase(state: &WorldState, expected: WorldPhase) -> Result<(), KernelError> {
    if state.phase == expected {
        Ok(())
    } else {
        Err(KernelError::WrongPhase {
            expected,
            actual: state.phase,
        })
    }
}

fn normalize_title(value: &str) -> Result<String, KernelError> {
    let value = value.trim();
    if value.is_empty() {
        Err(KernelError::EmptyTitle)
    } else {
        Ok(value.to_owned())
    }
}

fn validate_principal(value: &PrincipalId) -> Result<(), KernelError> {
    if value.0.trim().is_empty() || value.0.trim() != value.0 {
        Err(KernelError::EmptyPrincipal)
    } else {
        Ok(())
    }
}

fn validate_assignment(value: &ControllerAssignment) -> Result<(), KernelError> {
    if let ControllerAssignment::Human { principal, .. } = value {
        validate_principal(principal)?;
    }
    Ok(())
}

/// `command_id` is the band draw's only per-command term, so the arm that
/// re-derives an exercised decision needs the same one `reduce` drew from.
fn apply_effect(
    state: &mut WorldState,
    command_id: CommandId,
    caller: &CallerId,
    effect: &WorldEffect,
) -> Result<(), KernelError> {
    match effect {
        WorldEffect::WorldCreated { .. } => {
            return Err(KernelError::Invariant(
                "world genesis cannot be applied as a mutable effect".into(),
            ));
        }
        WorldEffect::PatchAdmitted { resolved } => {
            if caller != &CallerId::Principal(state.owner.clone())
                || (state.phase == WorldPhase::Active && !resolved.declares_nothing())
            {
                return Err(KernelError::Invariant(
                    "admitted patch does not satisfy admission authority".into(),
                ));
            }
            admit_resolved(state, resolved)?;
        }
        WorldEffect::DraftApproved { principal } => {
            if state.phase != WorldPhase::Draft
                || caller != &CallerId::Principal(principal.clone())
                || !required_approvers(state).contains(principal)
                || !state.draft_approvals.insert(principal.clone())
            {
                return Err(KernelError::Invariant(
                    "draft-approval effect is unauthorized or duplicate".into(),
                ));
            }
        }
        WorldEffect::WorldActivated => {
            if caller != &CallerId::Principal(state.owner.clone())
                || state.phase != WorldPhase::Draft
                || !required_approvers(state).is_subset(&state.draft_approvals)
            {
                return Err(KernelError::Invariant(
                    "activation does not satisfy lifecycle authority".into(),
                ));
            }
            state.phase = WorldPhase::Active;
        }
        WorldEffect::DecisionExercised { opportunity, event } => {
            let current = exact_opportunity(state, opportunity)?;
            let assignment = state
                .controller_assignments
                .get(&current.scope)
                .ok_or_else(|| {
                    KernelError::Invariant("decision scope lost its controller".into())
                })?;
            if state.phase != WorldPhase::Active || caller != &assignment.expected_caller() {
                return Err(KernelError::Invariant(
                    "decision effect does not match exact opportunity authority".into(),
                ));
            }
            let granted = require_granted(state, &current, event.invocation.affordance)?;
            // The whole event is re-derived by the function that produced the
            // honest one, so a forged band, operation, magnitude, or utterance
            // is one comparison rather than a clause apiece.
            let derived =
                action::exercise(state, command_id, &current, &granted, &event.invocation)?;
            if derived != *event {
                return Err(KernelError::Invariant(
                    "decision effect does not derive from its opportunity".into(),
                ));
            }
            state.events.push(event.clone());
            if !event.effects.is_empty() {
                apply_operations(state, &event.effects, &[])?;
            }
        }
        WorldEffect::DecisionDeclined { opportunity } => {
            let current = exact_opportunity(state, opportunity)?;
            let assignment = state
                .controller_assignments
                .get(&current.scope)
                .ok_or_else(|| {
                    KernelError::Invariant("decision scope lost its controller".into())
                })?;
            if state.phase != WorldPhase::Active || caller != &assignment.expected_caller() {
                return Err(KernelError::Invariant(
                    "decline effect does not match exact opportunity authority".into(),
                ));
            }
        }
    }
    Ok(())
}

fn digest<T: Serialize>(value: &T) -> Result<String, KernelError> {
    let bytes = rmp_serde::to_vec_named(value)
        .map_err(|error| KernelError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn state_digest(state: &WorldState) -> Result<String, KernelError> {
    let mut value = state.clone();
    value.state_digest.clear();
    value.last_commit_digest = None;
    digest(&value)
}

fn commit_digest(commit: &WorldCommit) -> Result<String, KernelError> {
    let mut value = commit.clone();
    value.digest.clear();
    digest(&value)
}

#[cfg(test)]
mod tests {
    use super::patch::kernel_speak_grant;
    use super::*;

    pub(super) fn owner() -> PrincipalId {
        PrincipalId::new("owner@example.test")
    }

    pub(super) fn player() -> PrincipalId {
        PrincipalId::new("player@example.test")
    }

    pub(super) fn auth_principal(principal: PrincipalId) -> AuthenticatedCaller {
        AuthenticatedCaller::fixture(CallerId::Principal(principal))
    }

    pub(super) fn subject(
        handle: &str,
        label: &str,
        kind: SubjectKind,
        controller: NewController,
    ) -> Declaration {
        Declaration::Subject(SubjectDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind,
            controller,
            affordances: kernel_speak_grant(),
            position: None,
        })
    }

    /// The committed Speak entry. A post-genesis declaration grants it by
    /// canonical reference, like every other structure a previous commit
    /// allocated: the kernel synthesizes the entry once, at genesis.
    pub(super) fn speak_entry(kernel: &WorldKernel) -> Ref<AffordanceId> {
        Ref::Existing(
            *kernel
                .state
                .affordance_catalog
                .iter()
                .find(|(_, entry)| entry.kind.0 == "speak")
                .map(|(affordance_id, _)| affordance_id)
                .expect("genesis admits the kernel Speak entry"),
        )
    }

    /// A catalog entry by kind name. An affordance id names an entry, not a
    /// subject's copy of one, so a test that needs an entry a scope does not
    /// hold asks for it by name rather than by list position.
    pub(super) fn affordance_named(snapshot: &WorldSnapshot, kind: &str) -> AffordanceId {
        snapshot
            .affordances
            .iter()
            .find(|entry| entry.entry.kind.0 == kind)
            .map(|entry| entry.id)
            .expect("the fixture world declares this affordance")
    }

    pub(super) fn creation(id: CommandId, title: &str) -> CreateWorld {
        CreateWorld {
            id,
            owner: owner(),
            title: title.into(),
            patch: WorldPatch {
                operations: Vec::new(),
                evidence: Vec::new(),
                declarations: vec![
                    // A second, world-declared entry granted to exactly one
                    // subject, so the fixture world has more than one verb and
                    // grant sets differ between scopes.
                    Declaration::Affordance(AffordanceDeclaration {
                        handle: DraftHandle::new("convene"),
                        kind: AffordanceKindName("convene".into()),
                        roles: Vec::new(),
                        preconditions: Vec::new(),
                        effect_slots: Vec::new(),
                        outcome_bands: vec![OutcomeBand {
                            weight: 1,
                            effects: Vec::new(),
                        }],
                        carries_speech: true,
                    }),
                    subject(
                        "player",
                        "The Player",
                        SubjectKind::Person,
                        NewController::Human {
                            principal: player(),
                        },
                    ),
                    subject(
                        "persona",
                        "The Witness",
                        SubjectKind::Person,
                        NewController::NarrativePersona,
                    ),
                    Declaration::Subject(SubjectDeclaration {
                        handle: DraftHandle::new("operator"),
                        label: "The Council".into(),
                        kind: SubjectKind::Institution,
                        controller: NewController::OperationalAgent,
                        affordances: kernel_speak_grant()
                            .into_iter()
                            .chain(std::iter::once(Ref::Draft(DraftHandle::new("convene"))))
                            .collect(),
                        position: None,
                    }),
                ],
            },
        }
    }

    pub(super) fn command(
        snapshot: &WorldSnapshot,
        id: CommandId,
        caller: CallerId,
        body: CommandBody,
    ) -> CommandEnvelope {
        CommandEnvelope {
            id,
            world_id: snapshot.world_id,
            expected_revision: snapshot.revision,
            caller,
            body,
        }
    }

    pub(super) fn submit_owner(
        kernel: &mut WorldKernel,
        snapshot: &WorldSnapshot,
        body: CommandBody,
    ) -> SubmitReceipt {
        kernel
            .submit(
                command(
                    snapshot,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    body,
                ),
                &auth_principal(owner()),
            )
            .unwrap()
    }

    pub(super) fn activate(kernel: &mut WorldKernel) -> WorldSnapshot {
        let genesis = kernel.snapshot().unwrap();
        submit_owner(kernel, &genesis, CommandBody::ApproveDraft);
        let after_owner = kernel.snapshot().unwrap();
        kernel
            .submit(
                command(
                    &after_owner,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(player()),
            )
            .unwrap();
        let approved = kernel.snapshot().unwrap();
        submit_owner(kernel, &approved, CommandBody::ActivateWorld);
        kernel.snapshot().unwrap()
    }

    /// The canonical IDs of the shared topology fixture: three places, four
    /// routes with every access and open combination the operations read, and one
    /// subject standing in the yard.
    pub(super) struct Topology {
        pub(super) yard: EntityId,
        pub(super) road: EntityId,
        pub(super) gate: EntityId,
        pub(super) ramp: EdgeId,
        pub(super) shutter: EdgeId,
        pub(super) toll: EdgeId,
        pub(super) span: EdgeId,
        pub(super) walker: SubjectId,
    }

    fn place(handle: &str, label: &str) -> Declaration {
        Declaration::Entity(EntityDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: EntityKind::Place,
            container: None,
        })
    }

    fn way(
        handle: &str,
        label: &str,
        from: &str,
        to: &str,
        access: AccessKind,
        cost: u32,
    ) -> Declaration {
        Declaration::Route(RouteDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            from: Ref::Draft(DraftHandle::new(from)),
            to: Ref::Draft(DraftHandle::new(to)),
            access,
            cost: Cost(cost),
        })
    }

    pub(super) fn topology_patch(speak: Ref<AffordanceId>) -> WorldPatch {
        WorldPatch {
            declarations: vec![
                place("yard", "The Cavity Yard"),
                place("road", "The Rhythm Road"),
                place("gate", "The Rain Gate"),
                way(
                    "ramp",
                    "The Yard Ramp",
                    "yard",
                    "road",
                    AccessKind::Public,
                    12,
                ),
                way(
                    "shutter",
                    "The Yard Shutter",
                    "yard",
                    "gate",
                    AccessKind::Public,
                    5,
                ),
                way(
                    "toll",
                    "The Toll Stair",
                    "yard",
                    "gate",
                    AccessKind::Restricted {
                        requires: AuthorityKindName(ADMIT_KIND.into()),
                    },
                    4,
                ),
                way(
                    "span",
                    "The Western Span",
                    "road",
                    "gate",
                    AccessKind::Public,
                    7,
                ),
                Declaration::Subject(SubjectDeclaration {
                    handle: DraftHandle::new("walker"),
                    label: "The Walker".into(),
                    kind: SubjectKind::Person,
                    controller: NewController::OperationalAgent,
                    affordances: BTreeSet::from([speak]),
                    position: Some(Ref::Draft(DraftHandle::new("yard"))),
                }),
            ],
            // A route that should start closed is declared and closed in one
            // patch, so `open` keeps a single writer family.
            operations: vec![ComponentOp::CloseRoute {
                route: Ref::Draft(DraftHandle::new("shutter")),
            }],
            evidence: Vec::new(),
        }
    }

    pub(super) fn admit_topology(kernel: &mut WorldKernel) -> Topology {
        let before = kernel.snapshot().unwrap();
        let receipt = submit_owner(
            kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: topology_patch(speak_entry(kernel)),
            },
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let entity = |label: &str| {
            *kernel
                .state
                .entities
                .iter()
                .find(|(_, record)| record.label == label)
                .expect("a declared place")
                .0
        };
        let edge = |label: &str| {
            *kernel
                .state
                .edges
                .iter()
                .find(|(_, record)| record.label() == label)
                .expect("a declared route")
                .0
        };
        Topology {
            yard: entity("The Cavity Yard"),
            road: entity("The Rhythm Road"),
            gate: entity("The Rain Gate"),
            ramp: edge("The Yard Ramp"),
            shutter: edge("The Yard Shutter"),
            toll: edge("The Toll Stair"),
            span: edge("The Western Span"),
            walker: *kernel
                .state
                .subjects
                .iter()
                .find(|(_, subject)| subject.label == "The Walker")
                .expect("the walker is admitted")
                .0,
        }
    }

    /// The canonical IDs of the custody fixture: two resources and two holder
    /// subjects standing in the topology, with the first holder carrying an
    /// evidenced opening balance.
    pub(super) struct Custody {
        pub(super) tithe: EntityId,
        pub(super) ingot: EntityId,
        pub(super) holder: SubjectId,
        pub(super) counterparty: SubjectId,
    }

    /// The seed world's authority kinds, offices, and grievance. They are world
    /// data: the kernel compares these strings and reads them no other way.
    pub(super) const LEVY_KIND: &str = "levy";
    pub(super) const ADMIT_KIND: &str = "admit";
    pub(super) const COMMAND_KIND: &str = "command";
    pub(super) const JUDGE_KIND: &str = "judge";
    pub(super) const WARDEN_OFFICE: &str = "warden";
    pub(super) const BAILIFF_OFFICE: &str = "bailiff";
    pub(super) const SEIZURE_GRIEVANCE: &str = "seizure";

    pub(super) const TITHE_RECEIPT: &str = "receipt:rhythm-tithe-census";
    pub(super) const OPENING_BALANCE: u64 = 7;

    fn resource(handle: &str, label: &str) -> Declaration {
        Declaration::Entity(EntityDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: EntityKind::Resource,
            container: None,
        })
    }

    fn holder(handle: &str, label: &str, place: EntityId, speak: Ref<AffordanceId>) -> Declaration {
        Declaration::Subject(SubjectDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: SubjectKind::Institution,
            controller: NewController::OperationalAgent,
            affordances: std::iter::once(speak)
                .chain(carry_affordances().into_iter().map(|declaration| {
                    let Declaration::Affordance(entry) = declaration else {
                        unreachable!("carry_affordances declares only affordances")
                    };
                    Ref::Draft(entry.handle)
                }))
                .collect(),
            position: Some(Ref::Existing(place)),
        })
    }

    pub(super) const CARRY_HANDLE: &str = "carry";

    /// The worked affordances for the action lane. Every one carries the same
    /// four roles and the same bounded `Transfer` slot, so a test varies exactly
    /// one thing — a precondition or a band table — by naming a different entry
    /// rather than by reaching into a committed catalog. The catalog is
    /// Draft-only, so authoring it is how a world gets a verb.
    fn carry_roles() -> Vec<RoleSpec> {
        vec![
            RoleSpec {
                role: Role("from".into()),
                kind: RefKind::Subject(None),
            },
            RoleSpec {
                role: Role("recipient".into()),
                kind: RefKind::Subject(None),
            },
            RoleSpec {
                role: Role("place".into()),
                kind: RefKind::Entity(EntityKind::Place),
            },
            RoleSpec {
                role: Role("resource".into()),
                kind: RefKind::Entity(EntityKind::Resource),
            },
        ]
    }

    /// Two slots on disjoint bands: band 0 names only the transfer, band 1 only
    /// the consume. Both slots are proposed on every invocation, so the entry
    /// separates what a proposer offered from what a draw admitted.
    fn split_variant() -> Declaration {
        Declaration::Affordance(AffordanceDeclaration {
            handle: DraftHandle::new("carry-split"),
            kind: AffordanceKindName("carry_split".into()),
            roles: carry_roles(),
            preconditions: Vec::new(),
            effect_slots: vec![
                EffectSlot {
                    op_kind: ComponentOpKind::Transfer,
                    roles: vec![
                        Role("from".into()),
                        Role("recipient".into()),
                        Role("resource".into()),
                    ],
                    bounds: Bounds::Quantity(Quantity(3)),
                },
                EffectSlot {
                    op_kind: ComponentOpKind::Consume,
                    roles: vec![Role("from".into()), Role("resource".into())],
                    bounds: Bounds::Quantity(Quantity(3)),
                },
            ],
            outcome_bands: vec![
                OutcomeBand {
                    weight: 1,
                    effects: vec![0],
                },
                OutcomeBand {
                    weight: 1,
                    effects: vec![1],
                },
            ],
            carries_speech: false,
        })
    }

    fn carry_variant(
        handle: &str,
        kind: &str,
        preconditions: Vec<Precondition>,
        outcome_bands: Vec<OutcomeBand>,
    ) -> Declaration {
        Declaration::Affordance(AffordanceDeclaration {
            handle: DraftHandle::new(handle),
            kind: AffordanceKindName(kind.into()),
            roles: carry_roles(),
            preconditions,
            effect_slots: vec![EffectSlot {
                op_kind: ComponentOpKind::Transfer,
                roles: vec![
                    Role("from".into()),
                    Role("recipient".into()),
                    Role("resource".into()),
                ],
                bounds: Bounds::Quantity(Quantity(3)),
            }],
            outcome_bands,
            carries_speech: false,
        })
    }

    fn certain_band() -> Vec<OutcomeBand> {
        vec![OutcomeBand {
            weight: 1,
            effects: vec![0],
        }]
    }

    fn place_role() -> Role {
        Role("place".into())
    }

    /// Carry, and five variants that each move one dial: a holding demand above
    /// the opening balance, a reach budget short of the two-hop path, one that
    /// covers it, a band that names no effect, and three equally weighted bands.
    pub(super) fn carry_affordances() -> Vec<Declaration> {
        vec![
            carry_variant(
                CARRY_HANDLE,
                "carry",
                vec![
                    Precondition::Present { at: place_role() },
                    Precondition::Holds {
                        resource: Role("resource".into()),
                        at_least: Quantity(1),
                    },
                ],
                certain_band(),
            ),
            carry_variant(
                "carry-greedy",
                "carry_greedy",
                vec![
                    Precondition::Present { at: place_role() },
                    Precondition::Holds {
                        resource: Role("resource".into()),
                        at_least: Quantity(OPENING_BALANCE + 1),
                    },
                ],
                certain_band(),
            ),
            carry_variant(
                "carry-holds",
                "carry_holds",
                vec![Precondition::Holds {
                    resource: Role("resource".into()),
                    at_least: Quantity(1),
                }],
                certain_band(),
            ),
            carry_variant(
                "carry-near",
                "carry_near",
                vec![Precondition::Reachable {
                    to: place_role(),
                    within: Cost(NEAR_REACH),
                }],
                certain_band(),
            ),
            carry_variant(
                "carry-far",
                "carry_far",
                vec![Precondition::Reachable {
                    to: place_role(),
                    within: Cost(FAR_REACH),
                }],
                certain_band(),
            ),
            carry_variant(
                "carry-idle",
                "carry_idle",
                Vec::new(),
                vec![OutcomeBand {
                    weight: 1,
                    effects: Vec::new(),
                }],
            ),
            carry_variant(
                "carry-chance",
                "carry_chance",
                Vec::new(),
                vec![
                    OutcomeBand {
                        weight: 1,
                        effects: vec![0],
                    },
                    OutcomeBand {
                        weight: 1,
                        effects: Vec::new(),
                    },
                    OutcomeBand {
                        weight: 1,
                        effects: Vec::new(),
                    },
                ],
            ),
            split_variant(),
        ]
    }

    /// Yard to gate over the open shutter alone.
    pub(super) const NEAR_REACH: u32 = 5;
    /// Yard to gate over the ramp and the span, which is the only open public
    /// path while the shutter is closed.
    pub(super) const FAR_REACH: u32 = 19;

    /// Declarations and evidence are Draft-only, so the resources, the holders,
    /// and the one evidenced `Admit` that creates the opening balance all land
    /// before activation. There is no holdings declaration field: quantity is
    /// created by `Admit` and by nothing else, in this lane as in every other.
    pub(super) fn custody_patch(topology: &Topology, speak: Ref<AffordanceId>) -> WorldPatch {
        WorldPatch {
            declarations: carry_affordances()
                .into_iter()
                .chain([
                    resource("tithe", "The Rhythm Tithe"),
                    resource("ingot", "The Cut Ingot"),
                    holder("clerk", "The Ledger Clerk", topology.yard, speak.clone()),
                    holder("keeper", "The Gate Keeper", topology.gate, speak),
                ])
                .collect(),
            operations: vec![ComponentOp::Admit {
                holder: Ref::Draft(DraftHandle::new("clerk")),
                resource: Ref::Draft(DraftHandle::new("tithe")),
                qty: Quantity(OPENING_BALANCE),
                evidence: EvidenceRef::new(TITHE_RECEIPT),
            }],
            evidence: vec![EvidenceRef::new(TITHE_RECEIPT)],
        }
    }

    pub(super) fn admit_custody(kernel: &mut WorldKernel, topology: &Topology) -> Custody {
        let before = kernel.snapshot().unwrap();
        let receipt = submit_owner(
            kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: custody_patch(topology, speak_entry(kernel)),
            },
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let entity = |label: &str| {
            *kernel
                .state
                .entities
                .iter()
                .find(|(_, record)| record.label == label)
                .expect("a declared resource")
                .0
        };
        let subject = |label: &str| {
            *kernel
                .state
                .subjects
                .iter()
                .find(|(_, record)| record.label == label)
                .expect("a declared holder")
                .0
        };
        Custody {
            tithe: entity("The Rhythm Tithe"),
            ingot: entity("The Cut Ingot"),
            holder: subject("The Ledger Clerk"),
            counterparty: subject("The Gate Keeper"),
        }
    }

    /// Topology, custody, then activation: everything a custody test needs, in
    /// the one order the phase rules allow.
    pub(super) fn custody_world(kernel: &mut WorldKernel) -> (Topology, Custody, WorldSnapshot) {
        let topology = admit_topology(kernel);
        let custody = admit_custody(kernel, &topology);
        let active = activate(kernel);
        (topology, custody, active)
    }

    /// The canonical IDs of the civic fixture: a hall containing a chamber, an
    /// institution holding jurisdiction over the hall, a person who holds that
    /// jurisdiction only through an office, a person and a resource inside the
    /// hall, and a person standing outside it.
    pub(super) struct Civic {
        pub(super) hall: EntityId,
        pub(super) chamber: EntityId,
        pub(super) passage: EdgeId,
        pub(super) causeway: EdgeId,
        pub(super) postern: EdgeId,
        pub(super) treasury: SubjectId,
        pub(super) reeve: SubjectId,
        pub(super) farmer: SubjectId,
        pub(super) outsider: SubjectId,
        pub(super) grain: EntityId,
    }

    pub(super) fn authority_kind(name: &str) -> AuthorityKindName {
        AuthorityKindName(name.into())
    }

    pub(super) fn office(name: &str) -> OfficeName {
        OfficeName(name.into())
    }

    pub(super) fn grievance(name: &str) -> GrievanceKindName {
        GrievanceKindName(name.into())
    }

    pub(super) fn over_place(place: EntityId) -> AuthorityTargetRef {
        AuthorityTargetRef::PlaceSubtree(Ref::Existing(place))
    }

    pub(super) fn over_subject(subject_id: SubjectId) -> AuthorityTargetRef {
        AuthorityTargetRef::Subject(Ref::Existing(subject_id))
    }

    pub(super) fn grant_of(kind: &str, over: AuthorityTargetRef) -> AuthorityGrantRef {
        AuthorityGrantRef {
            kind: authority_kind(kind),
            over,
        }
    }

    pub(super) fn grant_to(holder: SubjectId, kind: &str, over: AuthorityTargetRef) -> ComponentOp {
        ComponentOp::GrantAuthority {
            holder: Ref::Existing(holder),
            grant: grant_of(kind, over),
        }
    }

    /// Six world-authored civic entries. The kernel builds none of them; each
    /// is a worked example of what a seed author writes to make the political
    /// layer playable rather than administratively imposed.
    fn civic_affordances() -> Vec<Declaration> {
        let entry = |handle: &str,
                     kind: &str,
                     roles: Vec<RoleSpec>,
                     preconditions: Vec<Precondition>,
                     effect_slots: Vec<EffectSlot>,
                     outcome_bands: Vec<OutcomeBand>,
                     carries_speech: bool| {
            Declaration::Affordance(AffordanceDeclaration {
                handle: DraftHandle::new(handle),
                kind: AffordanceKindName(kind.into()),
                roles,
                preconditions,
                effect_slots,
                outcome_bands,
                carries_speech,
            })
        };
        let role = |name: &str, kind: RefKind| RoleSpec {
            role: Role(name.into()),
            kind,
        };
        let slot = |op_kind: ComponentOpKind, roles: Vec<&str>, bounds: Bounds| EffectSlot {
            op_kind,
            roles: roles.into_iter().map(|name| Role(name.into())).collect(),
            bounds,
        };
        let authorized = |over: &str, kind: &str| Precondition::Authorized {
            over: Role(over.into()),
            kind: authority_kind(kind),
        };
        vec![
            // The payee is the reserved `actor` role, so an authorized
            // collector cannot lawfully take a tax and send it to a friend.
            entry(
                "levy",
                "levy",
                vec![
                    role("payer", RefKind::Subject(None)),
                    role("resource", RefKind::Entity(EntityKind::Resource)),
                ],
                vec![authorized("payer", LEVY_KIND)],
                vec![slot(
                    ComponentOpKind::Transfer,
                    vec!["payer", "actor", "resource"],
                    Bounds::Quantity(Quantity(10)),
                )],
                certain_band(),
                false,
            ),
            entry(
                "delegate",
                "delegate",
                vec![
                    role("deputy", RefKind::Subject(None)),
                    role("ground", RefKind::Entity(EntityKind::Place)),
                ],
                // The precondition names one kind and the slot grants another,
                // so the monotonicity rule is the check that decides whether
                // the granted authority is one the granter holds.
                vec![authorized("ground", COMMAND_KIND)],
                vec![slot(
                    ComponentOpKind::GrantAuthority {
                        kind: authority_kind(LEVY_KIND),
                    },
                    vec!["deputy", "ground"],
                    Bounds::None,
                )],
                certain_band(),
                false,
            ),
            entry(
                "deploy",
                "deploy",
                vec![
                    role("subordinate", RefKind::Subject(None)),
                    role("via", RefKind::Edge(patch::EdgeKind::Route)),
                ],
                vec![authorized("subordinate", COMMAND_KIND)],
                vec![slot(
                    ComponentOpKind::Relocate,
                    vec!["subordinate", "via"],
                    Bounds::None,
                )],
                certain_band(),
                false,
            ),
            // The first affordance whose effect changes shared topology under a
            // legitimacy check; the second band is the interdiction that fails.
            entry(
                "sanction",
                "sanction",
                vec![role("road", RefKind::Edge(patch::EdgeKind::Route))],
                vec![authorized("road", COMMAND_KIND)],
                vec![slot(
                    ComponentOpKind::CloseRoute,
                    vec!["road"],
                    Bounds::None,
                )],
                vec![
                    OutcomeBand {
                        weight: 3,
                        effects: vec![0],
                    },
                    OutcomeBand {
                        weight: 1,
                        effects: Vec::new(),
                    },
                ],
                false,
            ),
            // Succession is an act someone performs under authority, not a
            // declared method string.
            entry(
                "appoint",
                "appoint",
                vec![
                    role("institution", RefKind::Subject(None)),
                    role("candidate", RefKind::Subject(None)),
                ],
                vec![authorized("institution", COMMAND_KIND)],
                vec![slot(
                    ComponentOpKind::InstallIncumbent {
                        office: office(BAILIFF_OFFICE),
                    },
                    vec!["institution", "candidate"],
                    Bounds::None,
                )],
                certain_band(),
                false,
            ),
            // The reader that keeps `Redress` from being decorative.
            entry(
                "petition",
                "petition",
                Vec::new(),
                vec![Precondition::HasStanding {
                    grievance: grievance(SEIZURE_GRIEVANCE),
                }],
                Vec::new(),
                vec![OutcomeBand {
                    weight: 1,
                    effects: Vec::new(),
                }],
                true,
            ),
        ]
    }

    fn civic_grants(handles: &[&str], speak: &Ref<AffordanceId>) -> BTreeSet<Ref<AffordanceId>> {
        std::iter::once(speak.clone())
            .chain(
                handles
                    .iter()
                    .map(|handle| Ref::Draft(DraftHandle::new(*handle))),
            )
            .collect()
    }

    pub(super) fn civic_patch(topology: &Topology, speak: Ref<AffordanceId>) -> WorldPatch {
        let person = |handle: &str, label: &str, place: Ref<EntityId>, handles: &[&str]| {
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(handle),
                label: label.into(),
                kind: SubjectKind::Person,
                controller: NewController::OperationalAgent,
                affordances: civic_grants(handles, &speak),
                position: Some(place),
            })
        };
        let declarations = civic_affordances()
            .into_iter()
            .chain([
                Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new("hall"),
                    label: "The Tithe Hall".into(),
                    kind: EntityKind::Place,
                    container: None,
                }),
                Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new("chamber"),
                    label: "The Counting Chamber".into(),
                    kind: EntityKind::Place,
                    container: Some(Ref::Draft(DraftHandle::new("hall"))),
                }),
                resource("grain", "The Winter Grain"),
                Declaration::Route(RouteDeclaration {
                    handle: DraftHandle::new("passage"),
                    label: "The Hall Passage".into(),
                    from: Ref::Draft(DraftHandle::new("chamber")),
                    to: Ref::Draft(DraftHandle::new("hall")),
                    access: AccessKind::Public,
                    cost: Cost(3),
                }),
                Declaration::Route(RouteDeclaration {
                    handle: DraftHandle::new("causeway"),
                    label: "The Hall Causeway".into(),
                    from: Ref::Draft(DraftHandle::new("hall")),
                    to: Ref::Existing(topology.road),
                    access: AccessKind::Public,
                    cost: Cost(6),
                }),
                Declaration::Route(RouteDeclaration {
                    handle: DraftHandle::new("postern"),
                    label: "The Hall Postern".into(),
                    from: Ref::Existing(topology.yard),
                    to: Ref::Draft(DraftHandle::new("chamber")),
                    access: AccessKind::Restricted {
                        requires: authority_kind(ADMIT_KIND),
                    },
                    cost: Cost(2),
                }),
                Declaration::Subject(SubjectDeclaration {
                    handle: DraftHandle::new("treasury"),
                    label: "The Tithe Treasury".into(),
                    kind: SubjectKind::Institution,
                    controller: NewController::OperationalAgent,
                    affordances: civic_grants(
                        &["levy", "delegate", "deploy", "sanction", "appoint"],
                        &speak,
                    ),
                    position: Some(Ref::Draft(DraftHandle::new("hall"))),
                }),
                Declaration::Subject(SubjectDeclaration {
                    handle: DraftHandle::new("reeve"),
                    label: "The Hall Reeve".into(),
                    kind: SubjectKind::Person,
                    controller: NewController::NarrativePersona,
                    affordances: civic_grants(
                        &["levy", "delegate", "petition", "sanction"],
                        &speak,
                    ),
                    position: Some(Ref::Draft(DraftHandle::new("chamber"))),
                }),
                person(
                    "farmer",
                    "The Winter Farmer",
                    Ref::Draft(DraftHandle::new("chamber")),
                    &["levy", "petition"],
                ),
                person(
                    "outsider",
                    "The Road Pedlar",
                    Ref::Existing(topology.road),
                    &["levy", "petition"],
                ),
            ])
            .collect();
        WorldPatch {
            declarations,
            operations: vec![
                ComponentOp::Admit {
                    holder: Ref::Draft(DraftHandle::new("farmer")),
                    resource: Ref::Draft(DraftHandle::new("grain")),
                    qty: Quantity(OPENING_BALANCE),
                    evidence: EvidenceRef::new(TITHE_RECEIPT),
                },
                ComponentOp::Admit {
                    holder: Ref::Draft(DraftHandle::new("outsider")),
                    resource: Ref::Draft(DraftHandle::new("grain")),
                    qty: Quantity(OPENING_BALANCE),
                    evidence: EvidenceRef::new(TITHE_RECEIPT),
                },
                ComponentOp::GrantAuthority {
                    holder: Ref::Draft(DraftHandle::new("treasury")),
                    grant: AuthorityGrantRef {
                        kind: authority_kind(LEVY_KIND),
                        over: AuthorityTargetRef::PlaceSubtree(Ref::Draft(DraftHandle::new(
                            "hall",
                        ))),
                    },
                },
                ComponentOp::GrantAuthority {
                    holder: Ref::Draft(DraftHandle::new("treasury")),
                    grant: AuthorityGrantRef {
                        kind: authority_kind(COMMAND_KIND),
                        over: AuthorityTargetRef::PlaceSubtree(Ref::Draft(DraftHandle::new(
                            "hall",
                        ))),
                    },
                },
                ComponentOp::OpenOffice {
                    institution: Ref::Draft(DraftHandle::new("treasury")),
                    office: office(WARDEN_OFFICE),
                    delegated: BTreeSet::from([authority_kind(LEVY_KIND)]),
                },
                ComponentOp::OpenOffice {
                    institution: Ref::Draft(DraftHandle::new("treasury")),
                    office: office(BAILIFF_OFFICE),
                    delegated: BTreeSet::from([authority_kind(JUDGE_KIND)]),
                },
                ComponentOp::InstallIncumbent {
                    institution: Ref::Draft(DraftHandle::new("treasury")),
                    office: office(WARDEN_OFFICE),
                    incumbent: Ref::Draft(DraftHandle::new("reeve")),
                },
                ComponentOp::OpenForum {
                    grievance: grievance(SEIZURE_GRIEVANCE),
                    forum: Ref::Draft(DraftHandle::new("treasury")),
                    standing: AuthorityTargetRef::PlaceSubtree(Ref::Draft(DraftHandle::new(
                        "chamber",
                    ))),
                },
            ],
            evidence: vec![EvidenceRef::new(TITHE_RECEIPT)],
        }
    }

    pub(super) fn admit_civic(kernel: &mut WorldKernel, topology: &Topology) -> Civic {
        let before = kernel.snapshot().unwrap();
        let receipt = submit_owner(
            kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: civic_patch(topology, speak_entry(kernel)),
            },
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let entity = |label: &str| {
            *kernel
                .state
                .entities
                .iter()
                .find(|(_, record)| record.label == label)
                .expect("a declared entity")
                .0
        };
        let edge = |label: &str| {
            *kernel
                .state
                .edges
                .iter()
                .find(|(_, record)| record.label() == label)
                .expect("a declared route")
                .0
        };
        let subject = |label: &str| {
            *kernel
                .state
                .subjects
                .iter()
                .find(|(_, record)| record.label == label)
                .expect("a declared subject")
                .0
        };
        Civic {
            hall: entity("The Tithe Hall"),
            chamber: entity("The Counting Chamber"),
            grain: entity("The Winter Grain"),
            passage: edge("The Hall Passage"),
            causeway: edge("The Hall Causeway"),
            postern: edge("The Hall Postern"),
            treasury: subject("The Tithe Treasury"),
            reeve: subject("The Hall Reeve"),
            farmer: subject("The Winter Farmer"),
            outsider: subject("The Road Pedlar"),
        }
    }

    /// Topology, the civic subgraph, then activation.
    pub(super) fn civic_world(kernel: &mut WorldKernel) -> (Topology, Civic, WorldSnapshot) {
        let topology = admit_topology(kernel);
        let civic = admit_civic(kernel, &topology);
        let active = activate(kernel);
        (topology, civic, active)
    }

    /// Submits as owner and returns the complete mismatch set.
    pub(super) fn reject_owner(
        kernel: &mut WorldKernel,
        snapshot: &WorldSnapshot,
        body: CommandBody,
    ) -> Vec<Mismatch> {
        let error = kernel
            .submit(
                command(
                    snapshot,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    body,
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch, got {error:?}");
        };
        mismatches
    }

    pub(super) fn operations(operations: Vec<ComponentOp>) -> CommandBody {
        CommandBody::AdmitPatch {
            answers: None,
            patch: WorldPatch {
                declarations: Vec::new(),
                operations,
                evidence: Vec::new(),
            },
        }
    }

    pub(super) fn opportunity_for(
        snapshot: &WorldSnapshot,
        subject_id: SubjectId,
    ) -> DecisionOpportunity {
        snapshot
            .opportunities
            .iter()
            .find(|value| value.scope.subject_id == subject_id)
            .expect("the subject has an opportunity")
            .clone()
    }

    fn opportunity(snapshot: &WorldSnapshot, mode: ControllerMode) -> DecisionOpportunity {
        snapshot
            .opportunities
            .iter()
            .find(|value| value.controller_mode == mode)
            .unwrap()
            .clone()
    }

    fn speak(opportunity: &DecisionOpportunity, text: &str) -> DecisionInvocation {
        DecisionInvocation {
            affordance: opportunity.affordance_ids[0],
            bindings: Vec::new(),
            proposed: Vec::new(),
            speech: Some(Utterance::new(text).unwrap()),
        }
    }

    #[test]
    fn external_opportunities_reject_nested_authority_claims() {
        let opportunity = DecisionOpportunity {
            world_id: WorldId::issue(),
            revision: 4,
            scope_digest: ScopeDigest::fixture("sha256:fixture"),
            scope: DecisionScope {
                subject_id: SubjectId::issue(),
            },
            controller_id: ControllerId::issue(),
            controller_mode: ControllerMode::NarrativePersona,
            affordance_ids: vec![AffordanceId::issue()],
        };
        let mut nested = serde_json::to_value(&opportunity).unwrap();
        nested["scope"]["caller"] = serde_json::json!("legacy-owner");
        assert!(serde_json::from_value::<DecisionOpportunity>(nested).is_err());

        let mut outer = serde_json::to_value(&opportunity).unwrap();
        outer["callerId"] = serde_json::json!("legacy-controller");
        assert!(serde_json::from_value::<DecisionOpportunity>(outer).is_err());
    }

    /// A commit elsewhere in the world does not discard a bound proposal: the
    /// binding is the scope digest, and this scope did not change.
    #[test]
    fn an_unchanged_scope_commits_at_a_later_revision() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "Unchanged"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let bound = opportunity_for(&active, topology.walker);

        let persona = opportunity(&active, ControllerMode::NarrativePersona);
        let persona_caller = CallerId::Controller(persona.controller_id);
        kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    persona_caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&persona, "The yard is quiet."),
                        opportunity: persona,
                    },
                ),
                &AuthenticatedCaller::fixture(persona_caller),
            )
            .unwrap();
        let later = kernel.snapshot().unwrap();
        assert_eq!(later.revision, active.revision + 1);

        let walker_caller = CallerId::Controller(bound.controller_id);
        let receipt = kernel
            .submit(
                command(
                    &later,
                    CommandId::new(),
                    walker_caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&bound, "I take the ramp."),
                        opportunity: bound.clone(),
                    },
                ),
                &AuthenticatedCaller::fixture(walker_caller),
            )
            .unwrap();
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let after = kernel.snapshot().unwrap();
        assert_eq!(after.events.len(), 2);
        assert_eq!(after.events[1].revision, bound.revision + 2);
    }

    /// Closing a route incident to the subject's place changes exactly what the
    /// proposal's verification reads, so the bound proposal dies at the digest.
    #[test]
    fn a_changed_scope_is_rejected_with_scope_changed() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "Changed"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let bound = opportunity_for(&active, topology.walker);

        submit_owner(
            &mut kernel,
            &active,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::CloseRoute {
                        route: Ref::Existing(topology.ramp),
                    }],
                    evidence: Vec::new(),
                },
            },
        );
        let closed = kernel.snapshot().unwrap();
        let walker_caller = CallerId::Controller(bound.controller_id);
        let error = kernel
            .submit(
                command(
                    &closed,
                    CommandId::new(),
                    walker_caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&bound, "I take the ramp."),
                        opportunity: bound.clone(),
                    },
                ),
                &AuthenticatedCaller::fixture(walker_caller),
            )
            .unwrap_err();
        let KernelError::ScopeChanged {
            scope,
            expected,
            actual,
        } = error
        else {
            panic!("expected a changed scope");
        };
        assert_eq!(scope, bound.scope);
        assert_eq!(expected, bound.scope_digest);
        assert_ne!(expected, actual);
        assert_eq!(kernel.snapshot().unwrap(), closed);
    }

    /// The digest covers the scope's controller, its grants, its Position, and
    /// its incident routes, and nothing else in the world.
    #[test]
    fn scope_digest_reads_exactly_its_components() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "Components"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        activate(&mut kernel);
        let scope = DecisionScope {
            subject_id: topology.walker,
        };
        let base = scope_digest(&kernel.state, scope).unwrap();

        let mut changed_controller = kernel.state.clone();
        let assignment = changed_controller
            .controller_assignments
            .get_mut(&scope)
            .unwrap();
        *assignment = ControllerAssignment::NarrativePersona {
            controller_id: assignment.id(),
        };
        assert_ne!(scope_digest(&changed_controller, scope).unwrap(), base);

        let mut changed_grants = kernel.state.clone();
        let extra = AffordanceId::issue();
        changed_grants
            .affordance_catalog
            .insert(extra, patch::kernel_speak_entry());
        changed_grants
            .affordance_grants
            .entry(scope)
            .or_default()
            .insert(extra);
        assert_ne!(scope_digest(&changed_grants, scope).unwrap(), base);

        // The digest reads the granted entries' definitions, not only their
        // ids: admission reads preconditions and ceilings, so altering an entry
        // must move the digest.
        let mut changed_entry = kernel.state.clone();
        let granted = *kernel.state.affordance_grants[&scope].first().unwrap();
        changed_entry
            .affordance_catalog
            .get_mut(&granted)
            .unwrap()
            .carries_speech = false;
        assert_ne!(scope_digest(&changed_entry, scope).unwrap(), base);

        let mut changed_position = kernel.state.clone();
        changed_position.positions.insert(
            topology.walker,
            Position {
                place: topology.road,
            },
        );
        assert_ne!(scope_digest(&changed_position, scope).unwrap(), base);

        let mut changed_route = kernel.state.clone();
        changed_route
            .edges
            .get_mut(&topology.ramp)
            .unwrap()
            .set_cost(Cost(99));
        assert_ne!(scope_digest(&changed_route, scope).unwrap(), base);

        // The span joins road and gate, so it is incident to neither the walker's
        // place nor its scope; an unrelated subject's decision is invisible too.
        let mut unrelated_route = kernel.state.clone();
        unrelated_route
            .edges
            .get_mut(&topology.span)
            .unwrap()
            .set_cost(Cost(99));
        assert_eq!(scope_digest(&unrelated_route, scope).unwrap(), base);

        let active = kernel.snapshot().unwrap();
        let persona = opportunity(&active, ControllerMode::NarrativePersona);
        let persona_caller = CallerId::Controller(persona.controller_id);
        kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    persona_caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&persona, "Unrelated."),
                        opportunity: persona,
                    },
                ),
                &AuthenticatedCaller::fixture(persona_caller),
            )
            .unwrap();
        assert_eq!(scope_digest(&kernel.state, scope).unwrap(), base);
    }

    #[test]
    fn draft_activation_player_and_autonomous_actions_share_one_reducer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "  Delvehold  "),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        assert_eq!(genesis.title, "Delvehold");
        assert_eq!(genesis.phase, WorldPhase::Draft);
        assert!(genesis.opportunities.is_empty());

        let active = activate(&mut kernel);
        assert_eq!(active.phase, WorldPhase::Active);
        assert_eq!(active.opportunities.len(), 3);

        let player_opportunity = opportunity(&active, ControllerMode::Human);
        kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&player_opportunity, "I open the door."),
                        opportunity: player_opportunity,
                    },
                ),
                &auth_principal(player()),
            )
            .unwrap();
        let after_player = kernel.snapshot().unwrap();
        assert_eq!(after_player.events.len(), 1);
        assert_eq!(
            after_player.events[0]
                .invocation
                .speech
                .as_ref()
                .map(Utterance::as_str),
            Some("I open the door.")
        );

        let persona_opportunity = opportunity(&after_player, ControllerMode::NarrativePersona);
        let persona_caller = CallerId::Controller(persona_opportunity.controller_id);
        kernel
            .submit(
                command(
                    &after_player,
                    CommandId::new(),
                    persona_caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&persona_opportunity, "Then mind the hinge."),
                        opportunity: persona_opportunity,
                    },
                ),
                &AuthenticatedCaller::fixture(persona_caller),
            )
            .unwrap();
        let after_persona = kernel.snapshot().unwrap();
        assert_eq!(after_persona.events.len(), 2);
        assert!(
            after_persona
                .opportunities
                .iter()
                .all(|value| value.revision == after_persona.revision)
        );
    }

    #[test]
    fn lifecycle_and_controller_authority_fail_closed_without_commits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Held"),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        assert!(matches!(
            kernel.submit(
                command(
                    &genesis,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::ActivateWorld,
                ),
                &auth_principal(owner())
            ),
            Err(KernelError::MissingApprovals(_))
        ));
        assert_eq!(kernel.journal.commit_count(), 1);

        submit_owner(&mut kernel, &genesis, CommandBody::ApproveDraft);
        let owner_approved = kernel.snapshot().unwrap();
        assert!(matches!(
            kernel.submit(
                command(
                    &owner_approved,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ActivateWorld,
                ),
                &auth_principal(player())
            ),
            Err(KernelError::Unauthorized)
        ));
        assert!(matches!(
            kernel.submit(
                command(
                    &owner_approved,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(owner())
            ),
            Err(KernelError::DraftAlreadyApproved)
        ));
        assert_eq!(kernel.journal.commit_count(), 2);

        kernel
            .submit(
                command(
                    &owner_approved,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(player()),
            )
            .unwrap();
        let approved = kernel.snapshot().unwrap();
        submit_owner(&mut kernel, &approved, CommandBody::ActivateWorld);
        let active = kernel.snapshot().unwrap();
        let persona = opportunity(&active, ControllerMode::NarrativePersona);
        let commits_before_forgery = kernel.journal.commit_count();
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&persona, "Puppet"),
                        opportunity: persona,
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::ControllerMismatch)
        ));
        assert_eq!(kernel.journal.commit_count(), commits_before_forgery);
        assert_eq!(kernel.snapshot().unwrap(), active);
    }

    #[test]
    fn a_tampered_or_unknown_opportunity_never_commits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Exact"),
            &auth_principal(owner()),
        )
        .unwrap();
        let active = activate(&mut kernel);
        let original = opportunity(&active, ControllerMode::Human);
        // An entry only the operational subject is granted: the human's scope
        // does not hold it, which is what denial means now that an affordance id
        // names a catalog entry rather than one subject's copy of one.
        let denied_invocation = DecisionInvocation {
            affordance: affordance_named(&active, "convene"),
            bindings: Vec::new(),
            proposed: Vec::new(),
            speech: Some(Utterance::new("No grant").unwrap()),
        };
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: denied_invocation,
                        opportunity: original.clone(),
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::AffordanceDenied)
        ));
        // A claimed affordance the world did not grant carries no authority: the
        // kernel exercises the opportunity it derives, never the one submitted.
        let forged_affordance = AffordanceId::issue();
        let mut tampered = original.clone();
        tampered.affordance_ids.push(forged_affordance);
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: DecisionInvocation {
                            affordance: forged_affordance,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Utterance::new("Forged").unwrap()),
                        },
                        opportunity: tampered,
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::AffordanceDenied)
        ));
        assert_eq!(kernel.snapshot().unwrap(), active);

        let mut forged_scope = original.clone();
        forged_scope.scope_digest = ScopeDigest::fixture("sha256:not-this-scope");
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&original, "No"),
                        opportunity: forged_scope,
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::ScopeChanged { .. })
        ));
        assert_eq!(kernel.snapshot().unwrap(), active);

        let unknown_scope = DecisionOpportunity {
            scope: DecisionScope {
                subject_id: SubjectId::issue(),
            },
            ..original.clone()
        };
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&original, "No"),
                        opportunity: unknown_scope,
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::OpportunityMismatch)
        ));
        assert_eq!(kernel.snapshot().unwrap(), active);
    }

    #[test]
    fn create_submit_restart_and_replay_are_exact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let creation = creation(CommandId::new(), "First");
        let (mut kernel, creation_receipt) =
            WorldKernel::create(&path, creation.clone(), &auth_principal(owner())).unwrap();
        let active = activate(&mut kernel);
        let decision = opportunity(&active, ControllerMode::OperationalAgent);
        let caller = CallerId::Controller(decision.controller_id);
        let command = command(
            &active,
            CommandId::new(),
            caller.clone(),
            CommandBody::ExerciseDecision {
                invocation: speak(&decision, "The vote carries."),
                opportunity: decision,
            },
        );
        let applied = kernel
            .submit(
                command.clone(),
                &AuthenticatedCaller::fixture(caller.clone()),
            )
            .unwrap();
        let SubmitReceipt::Applied(receipt) = applied else {
            panic!("expected applied receipt")
        };
        let accepted = kernel.snapshot().unwrap();
        drop(kernel);

        let (mut reopened, retried_creation_receipt) =
            WorldKernel::create(&path, creation, &auth_principal(owner())).unwrap();
        assert_eq!(retried_creation_receipt, creation_receipt);
        assert_eq!(reopened.snapshot().unwrap(), accepted);
        assert_eq!(
            reopened
                .submit(command, &AuthenticatedCaller::fixture(caller))
                .unwrap(),
            SubmitReceipt::AlreadyApplied(receipt)
        );
        drop(reopened);
        let reopened = WorldKernel::open(&path, accepted.world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), accepted);
    }

    #[test]
    fn controller_receipt_is_bound_to_the_exact_opportunity_and_invocation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Receipt Scope"),
            &auth_principal(owner()),
        )
        .unwrap();
        let active = activate(&mut kernel);
        let decision = opportunity(&active, ControllerMode::OperationalAgent);
        let other = opportunity(&active, ControllerMode::NarrativePersona);
        let command_id = CommandId::new();
        let caller = CallerId::Controller(decision.controller_id);
        let invocation = speak(&decision, "The record is exact.");
        kernel
            .submit(
                command(
                    &active,
                    command_id,
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: invocation.clone(),
                        opportunity: decision.clone(),
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .unwrap();

        let receipt = kernel
            .controller_receipt(command_id, &decision, &invocation)
            .unwrap()
            .expect("committed controller command has a receipt");
        assert_eq!(receipt.command_id, command_id);
        assert!(
            kernel
                .controller_receipt(CommandId::new(), &decision, &invocation)
                .unwrap()
                .is_none()
        );
        assert!(matches!(
            kernel.controller_receipt(command_id, &other, &invocation),
            Err(KernelError::CommandIdConflict)
        ));
        let altered_invocation = speak(&decision, "A different proposal.");
        assert!(matches!(
            kernel.controller_receipt(command_id, &decision, &altered_invocation),
            Err(KernelError::CommandIdConflict)
        ));
    }

    #[test]
    fn controller_decline_consumes_only_its_exact_opportunity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Decline Scope"),
            &auth_principal(owner()),
        )
        .unwrap();
        let active = activate(&mut kernel);
        let decision = opportunity(&active, ControllerMode::OperationalAgent);
        let other = opportunity(&active, ControllerMode::NarrativePersona);
        let caller = CallerId::Controller(decision.controller_id);

        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Controller(other.controller_id),
                    CommandBody::DeclineDecision {
                        opportunity: decision.clone(),
                    },
                ),
                &AuthenticatedCaller::fixture(CallerId::Controller(other.controller_id)),
            ),
            Err(KernelError::ControllerMismatch)
        ));
        assert_eq!(kernel.snapshot().unwrap(), active);

        let command_id = CommandId::new();
        let decline = command(
            &active,
            command_id,
            caller.clone(),
            CommandBody::DeclineDecision {
                opportunity: decision.clone(),
            },
        );
        let SubmitReceipt::Applied(receipt) = kernel
            .submit(
                decline.clone(),
                &AuthenticatedCaller::fixture(caller.clone()),
            )
            .unwrap()
        else {
            panic!("exact decline was not applied")
        };
        let after = kernel.snapshot().unwrap();
        assert_eq!(after.revision, active.revision + 1);
        assert!(after.events.is_empty());
        let refreshed = opportunity(&after, ControllerMode::OperationalAgent);
        assert_ne!(refreshed, decision);
        assert_eq!(
            kernel
                .controller_decline_receipt(command_id, &decision)
                .unwrap(),
            Some(receipt.clone())
        );
        assert!(matches!(
            kernel.controller_decline_receipt(command_id, &other),
            Err(KernelError::CommandIdConflict)
        ));
        assert!(matches!(
            kernel.controller_receipt(
                command_id,
                &decision,
                &speak(&decision, "A decline is not speech."),
            ),
            Err(KernelError::CommandIdConflict)
        ));
        assert_eq!(
            kernel
                .submit(decline, &AuthenticatedCaller::fixture(caller.clone()))
                .unwrap(),
            SubmitReceipt::AlreadyApplied(receipt.clone())
        );
        assert_eq!(kernel.snapshot().unwrap(), after);
        drop(kernel);
        let reopened = WorldKernel::open(&path, after.world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), after);
        assert_eq!(
            reopened
                .controller_decline_receipt(
                    command_id,
                    &opportunity(&active, ControllerMode::OperationalAgent),
                )
                .unwrap(),
            Some(receipt)
        );
    }

    #[test]
    fn duplicate_and_stale_commands_do_not_commit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Still"),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        assert!(matches!(
            submit_owner(&mut kernel, &genesis, CommandBody::ApproveDraft),
            SubmitReceipt::Applied(_)
        ));
        let after = kernel.snapshot().unwrap();
        assert!(matches!(
            kernel.submit(
                command(
                    &after,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(owner())
            ),
            Err(KernelError::DraftAlreadyApproved)
        ));
        let stale = command(
            &genesis,
            CommandId::new(),
            CallerId::Principal(owner()),
            CommandBody::ApproveDraft,
        );
        assert!(matches!(
            kernel.submit(stale, &auth_principal(owner())),
            Err(KernelError::RevisionMismatch { .. })
        ));
        assert_eq!(kernel.snapshot().unwrap(), after);
        assert_eq!(kernel.journal.commit_count(), 2);
    }

    #[test]
    fn durable_idempotency_precedes_revision_and_detects_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "A"),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        let id = CommandId::new();
        let first = command(
            &genesis,
            id,
            CallerId::Principal(owner()),
            CommandBody::ApproveDraft,
        );
        let first_receipt = kernel
            .submit(first.clone(), &auth_principal(owner()))
            .unwrap();
        let second_snapshot = kernel.snapshot().unwrap();
        kernel
            .submit(
                command(
                    &second_snapshot,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(player()),
            )
            .unwrap();
        let after_second = kernel.snapshot().unwrap();
        assert_eq!(
            kernel.submit(first, &auth_principal(owner())).unwrap(),
            match first_receipt {
                SubmitReceipt::Applied(value) => SubmitReceipt::AlreadyApplied(value),
                _ => unreachable!(),
            }
        );
        let conflict = command(
            &genesis,
            id,
            CallerId::Principal(owner()),
            CommandBody::ActivateWorld,
        );
        assert!(matches!(
            kernel.submit(conflict, &auth_principal(owner())),
            Err(KernelError::CommandIdConflict)
        ));
        assert_eq!(kernel.snapshot().unwrap(), after_second);
    }

    #[test]
    fn creation_retry_is_bound_to_the_exact_raw_command() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let id = CommandId::new();
        let raw = creation(id, "  Canonical Result  ");
        let (kernel, first) =
            WorldKernel::create(&path, raw.clone(), &auth_principal(owner())).unwrap();
        assert_eq!(kernel.snapshot().unwrap().title, "Canonical Result");
        drop(kernel);

        let (kernel, retry) =
            WorldKernel::create(&path, raw.clone(), &auth_principal(owner())).unwrap();
        assert_eq!(retry, first);
        drop(kernel);

        let mut merely_equivalent = raw;
        merely_equivalent.title = "Canonical Result".into();
        assert!(matches!(
            WorldKernel::create(&path, merely_equivalent, &auth_principal(owner())),
            Err(KernelError::CreationConflict)
        ));
    }

    #[test]
    fn invalid_seed_never_allocates_a_world() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let mut invalid = creation(CommandId::new(), "Nope");
        invalid.patch.declarations[2] = invalid.patch.declarations[1].clone();
        let Err(KernelError::PatchRejected(rejected)) =
            WorldKernel::create(&path, invalid, &auth_principal(owner()))
        else {
            panic!("expected a rejected creation patch");
        };
        assert_eq!(
            rejected,
            vec![Mismatch::DuplicateHandle {
                handle: DraftHandle::new("player")
            }]
        );
        assert!(
            WorldKernel::create(
                &path,
                creation(CommandId::new(), "Valid"),
                &auth_principal(owner())
            )
            .is_ok()
        );
    }

    #[test]
    fn a_second_live_owner_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let creation = creation(CommandId::new(), "Held");
        let (kernel, _) =
            WorldKernel::create(&path, creation.clone(), &auth_principal(owner())).unwrap();
        let world_id = kernel.snapshot().unwrap().world_id;
        assert!(matches!(
            WorldKernel::open(&path, world_id),
            Err(KernelError::Store(_))
        ));
        assert!(matches!(
            WorldKernel::create(&path, creation.clone(), &auth_principal(owner())),
            Err(KernelError::Store(_))
        ));
        drop(kernel);
        let kernel = WorldKernel::open(&path, world_id).unwrap();
        drop(kernel);
        assert!(matches!(
            WorldKernel::open(&path, WorldId::issue()),
            Err(KernelError::OpenedWorldMismatch)
        ));
    }

    #[test]
    fn lost_post_commit_ack_poisons_until_reopen_and_exact_retry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Certain"),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        let uncertain = CommandId::new();
        let attempted = command(
            &genesis,
            uncertain,
            CallerId::Principal(owner()),
            CommandBody::ApproveDraft,
        );
        kernel.journal.fail_after_durable_commit_for_test();
        assert!(matches!(
            kernel.submit(attempted.clone(), &auth_principal(owner())),
            Err(KernelError::RecoveryRequired { command_id }) if command_id == uncertain
        ));
        assert!(matches!(
            kernel.snapshot(),
            Err(KernelError::RecoveryRequired { command_id }) if command_id == uncertain
        ));
        drop(kernel);

        let mut reopened = WorldKernel::open(&path, genesis.world_id).unwrap();
        let durable = reopened.snapshot().unwrap();
        assert_eq!(durable.revision, 1);
        assert_eq!(durable.draft_approvals, BTreeSet::from([owner()]));
        assert!(matches!(
            reopened
                .submit(attempted, &auth_principal(owner()))
                .unwrap(),
            SubmitReceipt::AlreadyApplied(_)
        ));
    }

    #[test]
    fn lost_genesis_ack_reopens_as_the_same_world_and_exact_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let command_id = CommandId::new();
        let input = creation(command_id, "Durable Genesis");
        let authenticated = auth_principal(owner());
        let mut empty = match journal::WorldJournal::open_owner(&path).unwrap() {
            journal::JournalOpen::Empty(empty) => empty,
            journal::JournalOpen::Live { .. } => panic!("fixture store must be empty"),
        };
        empty.fail_after_durable_initialize_for_test();
        let prepared = prepare_creation(input.clone(), &authenticated).unwrap();
        assert!(matches!(
            WorldKernel::initialize(empty, prepared),
            Err(KernelError::RecoveryRequired { command_id: uncertain })
                if uncertain == command_id
        ));

        let (reopened, receipt) = WorldKernel::create(&path, input, &authenticated).unwrap();
        let snapshot = reopened.snapshot().unwrap();
        assert_eq!(receipt.command_id, command_id);
        assert_eq!(receipt.world_id, snapshot.world_id);
        assert_eq!(snapshot.revision, 0);
        assert_eq!(snapshot.title, "Durable Genesis");
    }

    #[test]
    fn replacing_the_store_path_revokes_the_live_owner() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let displaced = dir.path().join("displaced.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Pinned"),
            &auth_principal(owner()),
        )
        .unwrap();
        let snapshot = kernel.snapshot().unwrap();
        std::fs::rename(&path, &displaced).unwrap();
        std::fs::File::create(&path).unwrap();
        assert!(matches!(kernel.snapshot(), Err(KernelError::OwnershipLost)));
        assert!(matches!(
            kernel.submit(
                command(
                    &snapshot,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(owner())
            ),
            Err(KernelError::OwnershipLost)
        ));
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&displaced, &path).unwrap();
        assert!(matches!(kernel.snapshot(), Err(KernelError::OwnershipLost)));
        drop(kernel);
        let reopened = WorldKernel::open(&path, snapshot.world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), snapshot);
    }

    #[test]
    fn sealed_authentication_cannot_be_replaced_by_a_command_claim() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Owned"),
            &auth_principal(owner()),
        )
        .unwrap();
        let snapshot = kernel.snapshot().unwrap();
        let forged = command(
            &snapshot,
            CommandId::new(),
            CallerId::Principal(player()),
            CommandBody::ApproveDraft,
        );
        assert!(matches!(
            kernel.submit(forged, &auth_principal(owner())),
            Err(KernelError::AuthenticationMismatch)
        ));
        assert_eq!(kernel.snapshot().unwrap(), snapshot);
    }

    /// Soul falsification: the preimage must exclude the revision, the phase,
    /// and every component outside the scope. Each step below is a real commit
    /// that moves the world, not a hand-edited state.
    #[test]
    fn soul_scope_digest_excludes_revision_phase_and_everything_outside_the_scope() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "Preimage"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let scope = DecisionScope {
            subject_id: topology.walker,
        };
        let base = scope_digest(&kernel.state, scope).unwrap();
        let start_revision = kernel.state.revision;

        // A new place elsewhere, a route that touches neither the walker's
        // place nor it, and a second placed subject: all outside the scope.
        let speak = speak_entry(&kernel);
        let before = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: vec![
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("annex"),
                            label: "The Annex".into(),
                            kind: EntityKind::Place,
                            container: None,
                        }),
                        Declaration::Route(RouteDeclaration {
                            handle: DraftHandle::new("annexway"),
                            label: "The Annex Way".into(),
                            from: Ref::Existing(topology.road),
                            to: Ref::Draft(DraftHandle::new("annex")),
                            access: AccessKind::Public,
                            cost: Cost(3),
                        }),
                        Declaration::Subject(SubjectDeclaration {
                            handle: DraftHandle::new("runner"),
                            label: "The Runner".into(),
                            kind: SubjectKind::Person,
                            controller: NewController::OperationalAgent,
                            affordances: BTreeSet::from([speak.clone()]),
                            position: Some(Ref::Existing(topology.road)),
                        }),
                    ],
                    operations: Vec::new(),
                    evidence: Vec::new(),
                },
            },
        );
        assert_ne!(kernel.state.revision, start_revision);
        assert_eq!(scope_digest(&kernel.state, scope).unwrap(), base);

        // Activation changes the phase, the approvals, and the revision.
        let active = activate(&mut kernel);
        assert_eq!(active.phase, WorldPhase::Active);
        assert_eq!(scope_digest(&kernel.state, scope).unwrap(), base);

        // Another subject's Position moving is not this scope's business.
        let annexway = *kernel
            .state
            .edges
            .iter()
            .find(|(_, record)| record.label() == "The Annex Way")
            .expect("the declared route")
            .0;
        let runner = *kernel
            .state
            .subjects
            .iter()
            .find(|(_, subject)| subject.label == "The Runner")
            .expect("the declared subject")
            .0;
        let active = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &active,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::Relocate {
                        subject: Ref::Existing(runner),
                        via: Ref::Existing(annexway),
                    }],
                    evidence: Vec::new(),
                },
            },
        );
        assert_eq!(
            kernel.state.positions.get(&runner),
            Some(&Position {
                place: kernel
                    .state
                    .entities
                    .iter()
                    .find(|(_, record)| record.label == "The Annex")
                    .map(|(id, _)| *id)
                    .unwrap()
            })
        );
        assert_eq!(scope_digest(&kernel.state, scope).unwrap(), base);

        // A route incident to the walker's own place is inside the scope.
        let moved = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &moved,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::CloseRoute {
                        route: Ref::Existing(topology.ramp),
                    }],
                    evidence: Vec::new(),
                },
            },
        );
        assert_ne!(scope_digest(&kernel.state, scope).unwrap(), base);
    }

    /// Soul falsification: a claimed opportunity whose `affordance_ids` name
    /// grants this scope does not hold cannot widen authority. The kernel reads
    /// the derived list, so the forged one is ignored and the invocation is
    /// denied on the real grants.
    #[test]
    fn soul_a_forged_affordance_list_cannot_widen_authority() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "Affordances"),
            &auth_principal(owner()),
        )
        .unwrap();
        let active = activate(&mut kernel);
        let persona = opportunity(&active, ControllerMode::NarrativePersona);
        let operator = opportunity(&active, ControllerMode::OperationalAgent);

        // The persona claims the operator's entry and lists it as its own.
        let convened = affordance_named(&active, "convene");
        assert!(operator.affordance_ids.contains(&convened));
        assert!(!persona.affordance_ids.contains(&convened));
        let mut forged = persona.clone();
        forged.affordance_ids = vec![convened, persona.affordance_ids[0]];
        let caller = CallerId::Controller(persona.controller_id);
        let error = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        opportunity: forged,
                        invocation: DecisionInvocation {
                            affordance: convened,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Utterance::new("Not my grant.").unwrap()),
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .unwrap_err();
        assert!(matches!(error, KernelError::AffordanceDenied));
        assert_eq!(kernel.snapshot().unwrap(), active);
    }

    /// Soul: an `AffordanceId` now names a catalog entry, not a subject's copy
    /// of one — the three genesis subjects share Speak's single id. So the id
    /// cannot be what gates: grant membership for the invoking scope is. The
    /// same `convene` id commits for the scope that holds it and is denied for
    /// one that does not.
    #[test]
    fn soul_a_shared_catalog_id_is_gated_by_the_scope_grant() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "SharedCatalogIds"),
            &auth_principal(owner()),
        )
        .unwrap();
        let active = activate(&mut kernel);
        let speak = affordance_named(&active, "speak");
        let convened = affordance_named(&active, "convene");

        // One entry, one id, every genesis subject a grantee.
        assert_eq!(active.subjects.len(), 3);
        assert!(
            active
                .subjects
                .iter()
                .all(|subject| subject.affordances.contains(&speak))
        );
        assert!(
            active
                .opportunities
                .iter()
                .all(|opportunity| opportunity.affordance_ids.contains(&speak))
        );
        let holders: Vec<SubjectId> = active
            .subjects
            .iter()
            .filter(|subject| subject.affordances.contains(&convened))
            .map(|subject| subject.id)
            .collect();
        assert_eq!(holders.len(), 1);

        let invoke = |kernel: &mut WorldKernel, snapshot: &WorldSnapshot, subject: SubjectId| {
            let opportunity = opportunity_for(snapshot, subject);
            let caller = CallerId::Controller(opportunity.controller_id);
            kernel.submit(
                command(
                    snapshot,
                    CommandId::new(),
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        opportunity,
                        invocation: DecisionInvocation {
                            affordance: convened,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Utterance::new("The council convenes.").unwrap()),
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
        };

        // The grantee's scope commits with that id.
        assert!(matches!(
            invoke(&mut kernel, &active, holders[0]).unwrap(),
            SubmitReceipt::Applied(_)
        ));

        // A scope that does not hold the entry is denied the very same id, and
        // nothing commits.
        let after = kernel.snapshot().unwrap();
        // A controller-driven scope, so the denial is the grant check rather
        // than the human lane's caller check reached first.
        let stranger = after
            .subjects
            .iter()
            .find(|subject| {
                !subject.affordances.contains(&convened) && subject.human_controller.is_none()
            })
            .expect("a controller-driven subject without the grant")
            .id;
        assert!(matches!(
            invoke(&mut kernel, &after, stranger).unwrap_err(),
            KernelError::AffordanceDenied
        ));
        assert_eq!(kernel.snapshot().unwrap(), after);
    }

    #[test]
    fn a_stale_grant_rejects_but_a_distant_route_reaches_admission_instead() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "BindingAndAdmission"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let bound = opportunity_for(&active, topology.walker);
        let speak = affordance_named(&active, "speak");

        // Binding is scope-digest equality, and the digest reads the granted
        // entries' definitions: altering one moves the digest, so a proposal
        // bound before that change no longer binds.
        let mut altered = kernel.state.clone();
        altered
            .affordance_catalog
            .get_mut(&speak)
            .expect("the Speak entry")
            .carries_speech = false;
        assert_ne!(
            scope_digest(&altered, bound.scope).unwrap(),
            scope_digest(&kernel.state, bound.scope).unwrap()
        );

        // A route the actor does not stand beside is outside the digest, so
        // closing it leaves the binding intact. That is deliberate: admission
        // re-reads the live graph at commit, so nothing commits on stale
        // topology and a proposal is not invalidated by distant traffic.
        let before_digest = scope_digest(&kernel.state, bound.scope).unwrap();
        let before = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &before,
            operations(vec![ComponentOp::CloseRoute {
                route: Ref::Existing(topology.span),
            }]),
        );
        assert_eq!(
            scope_digest(&kernel.state, bound.scope).unwrap(),
            before_digest,
            "a route outside the actor's incident set does not rebind the proposal"
        );

        let snapshot = kernel.snapshot().unwrap();
        let receipt = kernel
            .submit(
                command(
                    &snapshot,
                    CommandId::new(),
                    CallerId::Controller(bound.controller_id),
                    CommandBody::ExerciseDecision {
                        opportunity: bound.clone(),
                        invocation: DecisionInvocation {
                            affordance: speak,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Utterance::new("Still here.").unwrap()),
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(CallerId::Controller(bound.controller_id)),
            )
            .expect("a distant route closing does not rebind this proposal");
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
    }

    /// The mvp doc's disjointness requirement: an institution with an
    /// operational organ and a person-shaped voice is two subjects joined by an
    /// office, not one subject with two controllers.
    #[test]
    fn an_institution_and_its_voice_are_two_subjects_joined_by_an_office() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Two");
        let (_, civic, active) = civic_world(&mut kernel);

        let treasury = opportunity_for(&active, civic.treasury);
        let reeve = opportunity_for(&active, civic.reeve);
        assert_ne!(treasury.controller_id, reeve.controller_id);
        assert_eq!(
            active
                .opportunities
                .iter()
                .filter(|value| value.scope.subject_id == civic.treasury)
                .count(),
            1
        );
        assert_eq!(
            active
                .opportunities
                .iter()
                .filter(|value| value.scope.subject_id == civic.reeve)
                .count(),
            1
        );

        // Both are authorized over the hall — one directly, one by delegation —
        // and neither appears in the other's components.
        let over_hall = AuthorityGrant {
            kind: authority_kind(LEVY_KIND),
            over: AuthorityTarget::PlaceSubtree(civic.hall),
        };
        assert!(subject_authority(&kernel.state, civic.treasury).contains(&over_hall));
        assert!(subject_authority(&kernel.state, civic.reeve).contains(&over_hall));
        assert!(kernel.state.authority.get(&civic.reeve).is_none());
        assert!(
            scope_components(&kernel.state, civic.treasury)
                .delegated
                .is_empty()
        );
        assert!(
            scope_components(&kernel.state, civic.reeve)
                .authority
                .is_empty()
        );
    }

    /// A levy from catalog to conservation: one lowered `Transfer`, the ledger
    /// total unchanged, and no other partition touched.
    #[test]
    fn a_levy_round_trips_from_catalog_to_conservation() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Levy");
        let (_, civic, active) = civic_world(&mut kernel);
        let before_total: u64 = kernel
            .state
            .holdings
            .values()
            .filter_map(|held| held.get(&civic.grain))
            .map(|quantity| quantity.0)
            .sum();
        let positions = kernel.state.positions.clone();
        let edges = kernel.state.edges.clone();
        let entities = kernel.state.entities.clone();

        let opportunity = opportunity_for(&active, civic.treasury);
        let caller = CallerId::Controller(opportunity.controller_id);
        let receipt = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        opportunity,
                        invocation: DecisionInvocation {
                            affordance: affordance_named(&active, "levy"),
                            bindings: vec![
                                RoleBinding {
                                    role: Role("payer".into()),
                                    target: Target::Subject(civic.farmer),
                                },
                                RoleBinding {
                                    role: Role("resource".into()),
                                    target: Target::Entity(civic.grain),
                                },
                            ],
                            proposed: vec![ProposedEffect {
                                slot: 0,
                                magnitude: Magnitude::Quantity(Quantity(3)),
                            }],
                            speech: None,
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .unwrap();
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));

        let held = |holder: SubjectId| {
            kernel
                .state
                .holdings
                .get(&holder)
                .and_then(|held| held.get(&civic.grain))
                .map_or(0, |quantity| quantity.0)
        };
        assert_eq!(held(civic.farmer), OPENING_BALANCE - 3);
        assert_eq!(held(civic.treasury), 3);
        let after_total: u64 = kernel
            .state
            .holdings
            .values()
            .filter_map(|held| held.get(&civic.grain))
            .map(|quantity| quantity.0)
            .sum();
        assert_eq!(before_total, after_total);
        assert_eq!(
            kernel.state.events.last().unwrap().effects,
            vec![ResolvedOp::Transfer {
                from: civic.farmer,
                to: civic.treasury,
                resource: civic.grain,
                qty: Quantity(3),
            }]
        );
        assert_eq!(kernel.state.positions, positions);
        assert_eq!(kernel.state.edges, edges);
        assert_eq!(kernel.state.entities, entities);
    }

    /// Representation is person-and-institution shaped, and that is the check
    /// that stops the two-subject shape from collapsing back into one.
    #[test]
    fn office_admission_is_person_and_institution_shaped() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Offices");
        let (_, civic, _) = civic_world(&mut kernel);
        let refuse = |kernel: &mut WorldKernel, ops: Vec<ComponentOp>| {
            let before = kernel.snapshot().unwrap();
            reject_owner(kernel, &before, operations(ops))
        };

        // An institution cannot occupy its own office.
        assert_eq!(
            refuse(
                &mut kernel,
                vec![ComponentOp::InstallIncumbent {
                    institution: Ref::Existing(civic.treasury),
                    office: office(BAILIFF_OFFICE),
                    incumbent: Ref::Existing(civic.treasury),
                }]
            ),
            vec![Mismatch::OfficeHolderNotPerson { operation: 0 }]
        );
        // An office cannot be opened on a person.
        assert_eq!(
            refuse(
                &mut kernel,
                vec![ComponentOp::OpenOffice {
                    institution: Ref::Existing(civic.reeve),
                    office: office(WARDEN_OFFICE),
                    delegated: BTreeSet::from([authority_kind(LEVY_KIND)]),
                }]
            ),
            vec![Mismatch::OfficeOnNonInstitution { operation: 0 }]
        );
        // One person, at most one office per institution.
        assert_eq!(
            refuse(
                &mut kernel,
                vec![ComponentOp::InstallIncumbent {
                    institution: Ref::Existing(civic.treasury),
                    office: office(BAILIFF_OFFICE),
                    incumbent: Ref::Existing(civic.reeve),
                }]
            ),
            vec![Mismatch::DuplicateIncumbency { operation: 0 }]
        );
    }

    /// Disjointness in both directions: overlap inside one subject's effective
    /// authority is refused, overlap between subjects is layered government.
    #[test]
    fn overlapping_jurisdiction_within_one_subject_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Overlap");
        let (_, civic, _) = civic_world(&mut kernel);
        let refuse = |kernel: &mut WorldKernel, ops: Vec<ComponentOp>| {
            let before = kernel.snapshot().unwrap();
            reject_owner(kernel, &before, operations(ops))
        };
        let admit = |kernel: &mut WorldKernel, ops: Vec<ComponentOp>| {
            let before = kernel.snapshot().unwrap();
            submit_owner(kernel, &before, operations(ops))
        };

        // A direct grant under what the reeve's office already lends.
        assert_eq!(
            refuse(
                &mut kernel,
                vec![grant_to(civic.reeve, LEVY_KIND, over_place(civic.chamber))]
            ),
            vec![Mismatch::OverlappingJurisdiction { operation: 0 }]
        );
        // Two direct grants of one kind, nested, in one patch.
        assert_eq!(
            refuse(
                &mut kernel,
                vec![
                    grant_to(civic.farmer, LEVY_KIND, over_place(civic.hall)),
                    grant_to(civic.farmer, LEVY_KIND, over_place(civic.chamber)),
                ]
            ),
            vec![Mismatch::OverlappingJurisdiction { operation: 1 }]
        );
        // An incumbency that would lend the farmer ground it already holds.
        admit(
            &mut kernel,
            vec![grant_to(civic.farmer, LEVY_KIND, over_place(civic.chamber))],
        );
        assert_eq!(
            refuse(
                &mut kernel,
                vec![ComponentOp::InstallIncumbent {
                    institution: Ref::Existing(civic.treasury),
                    office: office(WARDEN_OFFICE),
                    incumbent: Ref::Existing(civic.farmer),
                }]
            ),
            vec![Mismatch::OverlappingJurisdiction { operation: 0 }]
        );

        // A different kind over the same ground commits, and so does the same
        // kind held by a different subject: two holders authorized over one
        // target is not a contradiction, because nothing arbitrates.
        assert!(matches!(
            admit(
                &mut kernel,
                vec![
                    grant_to(civic.reeve, JUDGE_KIND, over_place(civic.chamber)),
                    grant_to(civic.outsider, LEVY_KIND, over_place(civic.chamber)),
                ]
            ),
            SubmitReceipt::Applied(_)
        ));
    }

    /// Contract 6 on the civic surface: one patch, the complete sorted set, no
    /// allocation, and repairing exactly those failures commits.
    #[test]
    fn civic_operations_are_rejected_with_the_complete_set() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "CivicSet");
        let (_, civic, _) = civic_world(&mut kernel);
        let before = kernel.snapshot().unwrap();
        let broken = vec![
            ComponentOp::OpenOffice {
                institution: Ref::Existing(civic.treasury),
                office: office("steward"),
                delegated: BTreeSet::new(),
            },
            ComponentOp::InstallIncumbent {
                institution: Ref::Existing(civic.treasury),
                office: office("provost"),
                incumbent: Ref::Existing(civic.farmer),
            },
            ComponentOp::CloseForum {
                grievance: grievance("eviction"),
            },
            ComponentOp::OpenOffice {
                institution: Ref::Existing(civic.treasury),
                office: office("Chief Steward"),
                delegated: BTreeSet::from([authority_kind(JUDGE_KIND)]),
            },
            ComponentOp::InstallIncumbent {
                institution: Ref::Existing(civic.treasury),
                office: office(BAILIFF_OFFICE),
                incumbent: Ref::Existing(civic.treasury),
            },
            grant_to(civic.reeve, LEVY_KIND, over_place(civic.chamber)),
        ];
        let mut expected = vec![
            Mismatch::EmptyDelegation { operation: 0 },
            Mismatch::UnknownOffice { operation: 1 },
            Mismatch::UnknownForum { operation: 2 },
            Mismatch::InvalidCivicName {
                site: patch::Site::Operation(3),
            },
            Mismatch::OfficeHolderNotPerson { operation: 4 },
            Mismatch::OverlappingJurisdiction { operation: 5 },
        ];
        expected.sort();
        assert_eq!(
            reject_owner(&mut kernel, &before, operations(broken)),
            expected
        );
        assert_eq!(kernel.snapshot().unwrap(), before);

        let repaired = vec![
            ComponentOp::OpenOffice {
                institution: Ref::Existing(civic.treasury),
                office: office("steward"),
                delegated: BTreeSet::from([authority_kind(JUDGE_KIND)]),
            },
            ComponentOp::InstallIncumbent {
                institution: Ref::Existing(civic.treasury),
                office: office("steward"),
                incumbent: Ref::Existing(civic.farmer),
            },
            ComponentOp::OpenForum {
                grievance: grievance("eviction"),
                forum: Ref::Existing(civic.treasury),
                standing: over_place(civic.hall),
            },
        ];
        assert!(matches!(
            submit_owner(&mut kernel, &before, operations(repaired)),
            SubmitReceipt::Applied(_)
        ));
    }

    /// A civic operation that changes nothing is not a canonical change.
    #[test]
    fn an_idempotent_civic_operation_is_no_change() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel =
            crate::world::custody_tests::custody_kernel(directory.path(), "Idempotent");
        let (_, civic, _) = civic_world(&mut kernel);
        for operation in [
            grant_to(civic.treasury, LEVY_KIND, over_place(civic.hall)),
            ComponentOp::RevokeAuthority {
                holder: Ref::Existing(civic.treasury),
                grant: grant_of(JUDGE_KIND, over_place(civic.hall)),
            },
            ComponentOp::VacateOffice {
                institution: Ref::Existing(civic.treasury),
                office: office(BAILIFF_OFFICE),
            },
            ComponentOp::OpenForum {
                grievance: grievance(SEIZURE_GRIEVANCE),
                forum: Ref::Existing(civic.treasury),
                standing: over_place(civic.chamber),
            },
            ComponentOp::OpenOffice {
                institution: Ref::Existing(civic.treasury),
                office: office(WARDEN_OFFICE),
                delegated: BTreeSet::from([authority_kind(LEVY_KIND)]),
            },
        ] {
            let before = kernel.snapshot().unwrap();
            assert_eq!(
                reject_owner(&mut kernel, &before, operations(vec![operation])),
                vec![Mismatch::NoOperationEffect { operation: 0 }]
            );
        }
    }

    /// The digest reads what the actor is authorized over and nothing about the
    /// world the actor reaches.
    #[test]
    fn scope_digest_reads_authority_and_delegation_but_not_occupancy() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Digest");
        let (topology, civic, _) = civic_world(&mut kernel);
        let scope = |subject_id| DecisionScope { subject_id };
        let digest_of = |kernel: &WorldKernel, subject_id| {
            scope_digest(&kernel.state, scope(subject_id)).unwrap()
        };
        let admit = |kernel: &mut WorldKernel, ops: Vec<ComponentOp>| {
            let before = kernel.snapshot().unwrap();
            submit_owner(kernel, &before, operations(ops));
        };

        // Moving an unrelated subject into and out of the jurisdiction, and
        // opening a forum, leave every digest alone.
        let reeve_before = digest_of(&kernel, civic.reeve);
        let treasury_before = digest_of(&kernel, civic.treasury);
        admit(
            &mut kernel,
            vec![
                ComponentOp::Relocate {
                    subject: Ref::Existing(topology.walker),
                    via: Ref::Existing(topology.ramp),
                },
                ComponentOp::OpenForum {
                    grievance: grievance("eviction"),
                    forum: Ref::Existing(civic.treasury),
                    standing: over_place(civic.hall),
                },
            ],
        );
        assert_eq!(digest_of(&kernel, civic.reeve), reeve_before);
        assert_eq!(digest_of(&kernel, civic.treasury), treasury_before);

        // Installing an office the subject does not hold leaves it alone too.
        admit(
            &mut kernel,
            vec![ComponentOp::InstallIncumbent {
                institution: Ref::Existing(civic.treasury),
                office: office(BAILIFF_OFFICE),
                incumbent: Ref::Existing(civic.farmer),
            }],
        );
        assert_eq!(digest_of(&kernel, civic.reeve), reeve_before);
        assert_eq!(digest_of(&kernel, civic.treasury), treasury_before);

        // Revoking the institution's grant for a lent kind moves the
        // incumbent's digest, because the office copies it in.
        admit(
            &mut kernel,
            vec![ComponentOp::RevokeAuthority {
                holder: Ref::Existing(civic.treasury),
                grant: grant_of(LEVY_KIND, over_place(civic.hall)),
            }],
        );
        assert_ne!(digest_of(&kernel, civic.reeve), reeve_before);
        assert_ne!(digest_of(&kernel, civic.treasury), treasury_before);

        // Granting the actor's own, and narrowing a held office, move it too.
        let reeve_before = digest_of(&kernel, civic.reeve);
        admit(
            &mut kernel,
            vec![grant_to(civic.reeve, LEVY_KIND, over_place(civic.chamber))],
        );
        assert_ne!(digest_of(&kernel, civic.reeve), reeve_before);
        let reeve_before = digest_of(&kernel, civic.reeve);
        admit(
            &mut kernel,
            vec![ComponentOp::OpenOffice {
                institution: Ref::Existing(civic.treasury),
                office: office(WARDEN_OFFICE),
                delegated: BTreeSet::from([authority_kind(COMMAND_KIND)]),
            }],
        );
        assert_ne!(digest_of(&kernel, civic.reeve), reeve_before);
        let reeve_before = digest_of(&kernel, civic.reeve);
        admit(
            &mut kernel,
            vec![ComponentOp::VacateOffice {
                institution: Ref::Existing(civic.treasury),
                office: office(WARDEN_OFFICE),
            }],
        );
        assert_ne!(digest_of(&kernel, civic.reeve), reeve_before);
    }

    /// The snapshot carries a subject's own civic state and no gazetteer: a
    /// forum is visible only to its standing, and no subject sees another's
    /// grants.
    #[test]
    fn a_forum_is_visible_only_to_its_standing() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Redress");
        let (_, civic, active) = civic_world(&mut kernel);
        let of = |subject_id: SubjectId| {
            active
                .subjects
                .iter()
                .find(|subject| subject.id == subject_id)
                .expect("a declared subject")
        };

        // The farmer stands in the chamber, which is the forum's standing.
        assert_eq!(
            of(civic.farmer).redress,
            vec![ForumSnapshot {
                grievance: grievance(SEIZURE_GRIEVANCE),
                forum: civic.treasury,
            }]
        );
        assert!(of(civic.outsider).redress.is_empty());

        // Grants are the subject's own; offices held and offices granted are
        // the two ends of one link and never merge.
        assert!(of(civic.reeve).authority.is_empty());
        assert_eq!(of(civic.reeve).offices_held.len(), 1);
        assert!(of(civic.reeve).offices_granted.is_empty());
        assert_eq!(of(civic.treasury).offices_granted.len(), 2);
        assert!(of(civic.treasury).offices_held.is_empty());
        assert!(of(civic.farmer).authority.is_empty());
        assert_eq!(
            of(civic.treasury).authority,
            BTreeSet::from([
                AuthorityGrant {
                    kind: authority_kind(LEVY_KIND),
                    over: AuthorityTarget::PlaceSubtree(civic.hall),
                },
                AuthorityGrant {
                    kind: authority_kind(COMMAND_KIND),
                    over: AuthorityTarget::PlaceSubtree(civic.hall),
                },
            ])
        );
    }

    /// Soul falsification of the candidate authority shadow's reach: a grant
    /// minted in the same patch resolves, but a *destination* declared in the
    /// same patch does not, because the shadow is projected to canonical
    /// referents before the covering predicate runs. A genesis patch therefore
    /// cannot declare a restricted room and walk anyone into it in one batch.
    #[test]
    fn a_same_patch_destination_is_covered_by_no_grant() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "DraftDoor");
        let topology = admit_topology(&mut kernel);
        let before = kernel.snapshot().unwrap();
        let patch = WorldPatch {
            declarations: vec![
                Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new("keep"),
                    label: "The Inner Keep".into(),
                    kind: EntityKind::Place,
                    container: None,
                }),
                Declaration::Route(RouteDeclaration {
                    handle: DraftHandle::new("keepgate"),
                    label: "The Keep Gate".into(),
                    from: Ref::Existing(topology.yard),
                    to: Ref::Draft(DraftHandle::new("keep")),
                    access: AccessKind::Restricted {
                        requires: authority_kind(ADMIT_KIND),
                    },
                    cost: Cost(2),
                }),
            ],
            operations: vec![
                ComponentOp::GrantAuthority {
                    holder: Ref::Existing(topology.walker),
                    grant: AuthorityGrantRef {
                        kind: authority_kind(ADMIT_KIND),
                        over: AuthorityTargetRef::PlaceSubtree(Ref::Draft(DraftHandle::new(
                            "keep",
                        ))),
                    },
                },
                ComponentOp::Relocate {
                    subject: Ref::Existing(topology.walker),
                    via: Ref::Draft(DraftHandle::new("keepgate")),
                },
            ],
            evidence: Vec::new(),
        };
        assert_eq!(
            reject_owner(
                &mut kernel,
                &before,
                CommandBody::AdmitPatch {
                    answers: None,
                    patch
                }
            ),
            vec![Mismatch::RouteAccessRestricted { operation: 1 }],
            "the key covers the door, but the door's far side is still a draft"
        );
    }
}

#[cfg(test)]
mod custody_tests {
    use super::tests::{
        OPENING_BALANCE, TITHE_RECEIPT, activate, admit_custody, admit_topology, auth_principal,
        command, creation, custody_world, operations, opportunity_for, owner, reject_owner,
        speak_entry, submit_owner,
    };
    use super::*;

    pub(super) fn custody_kernel(path: &Path, title: &str) -> WorldKernel {
        WorldKernel::create(
            path.join("world.cc"),
            creation(CommandId::new(), title),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0
    }

    fn holding(kernel: &WorldKernel, holder: SubjectId, resource: EntityId) -> u64 {
        held(&kernel.state, holder, resource)
    }

    /// A forged `PatchAdmitted` never passes the resolver, so `admit_resolved`
    /// re-derives every custody precondition from the committed partitions and
    /// commits nothing when one fails.
    ///
    /// Deviation from the spec's wording, decided here: `ResolvedOp::Transfer`
    /// carries one `qty`, so "a transfer whose debit and credit differ" is
    /// unrepresentable — which is the stronger outcome that shape was chosen
    /// for. The reachable forgery is a transfer the holder cannot cover, and
    /// the conservation equation itself is falsified directly beside it.
    #[test]
    fn a_non_conserving_transfer_effect_does_not_apply() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Forged");
        let (_, custody, _) = custody_world(&mut kernel);
        let before = kernel.state.holdings.clone();

        let mut candidate = kernel.state.clone();
        let forged = WorldEffect::PatchAdmitted {
            resolved: ResolvedPatch {
                subjects: Vec::new(),
                entities: Vec::new(),
                routes: Vec::new(),
                affordances: Vec::new(),
                operations: vec![ResolvedOp::Transfer {
                    from: custody.holder,
                    to: custody.counterparty,
                    resource: custody.tithe,
                    qty: Quantity(OPENING_BALANCE + 1),
                }],
                evidence: Vec::new(),
            },
        };
        let error = apply_effect(
            &mut candidate,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &forged,
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::Invariant(_)));
        assert_eq!(candidate.holdings, before);

        // The conservation equation is the one owner, and it refuses a ledger
        // that does not balance regardless of which operation produced it.
        let unbalanced = BTreeMap::from([(
            custody.tithe,
            LedgerDelta {
                before: 7,
                after: 9,
                admitted: 1,
                ..LedgerDelta::default()
            },
        )]);
        assert_eq!(patch::check_ledger(&unbalanced), Some(custody.tithe));
        let balanced = BTreeMap::from([(
            custody.tithe,
            LedgerDelta {
                before: 7,
                after: 8,
                admitted: 1,
                ..LedgerDelta::default()
            },
        )]);
        assert_eq!(patch::check_ledger(&balanced), None);
    }

    /// Creation of quantity is attributable: an `Admit` whose ref is not in the
    /// patch's own evidence list mints nothing and allocates no ID.
    #[test]
    fn an_admit_without_evidence_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Unevidenced");
        let topology = admit_topology(&mut kernel);
        let speak = speak_entry(&kernel);
        let before = kernel.snapshot().unwrap();
        let commits = kernel.journal.commit_count();

        let mismatches = reject_owner(
            &mut kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: vec![
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("tithe"),
                            label: "The Rhythm Tithe".into(),
                            kind: EntityKind::Resource,
                            container: None,
                        }),
                        Declaration::Subject(SubjectDeclaration {
                            handle: DraftHandle::new("clerk"),
                            label: "The Ledger Clerk".into(),
                            kind: SubjectKind::Institution,
                            controller: NewController::OperationalAgent,
                            affordances: BTreeSet::from([speak.clone()]),
                            position: Some(Ref::Existing(topology.yard)),
                        }),
                    ],
                    operations: vec![ComponentOp::Admit {
                        holder: Ref::Draft(DraftHandle::new("clerk")),
                        resource: Ref::Draft(DraftHandle::new("tithe")),
                        qty: Quantity(4),
                        evidence: EvidenceRef::new("receipt:never-listed"),
                    }],
                    evidence: Vec::new(),
                },
            },
        );

        assert_eq!(
            mismatches,
            vec![Mismatch::AdmitWithoutEvidence { operation: 0 }]
        );
        assert!(kernel.state.holdings.is_empty());
        assert_eq!(kernel.journal.commit_count(), commits);
        assert_eq!(kernel.snapshot().unwrap(), before);
    }

    #[test]
    fn a_transfer_moves_the_exact_quantity_and_nothing_else() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Transfer");
        let (_, custody, active) = custody_world(&mut kernel);
        let entities = kernel.state.entities.clone();
        let edges = kernel.state.edges.clone();
        let positions = kernel.state.positions.clone();

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Transfer {
                from: Ref::Existing(custody.holder),
                to: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(3),
            }]),
        );

        assert_eq!(holding(&kernel, custody.holder, custody.tithe), 4);
        assert_eq!(holding(&kernel, custody.counterparty, custody.tithe), 3);
        assert_eq!(resource_total(&kernel.state, custody.tithe), 7);
        assert_eq!(kernel.state.entities, entities);
        assert_eq!(kernel.state.edges, edges);
        assert_eq!(kernel.state.positions, positions);
    }

    /// Absence is zero at both levels, so emptying a holding removes the
    /// resource key and emptying a holder removes the holder key.
    #[test]
    fn consume_reduces_the_holding_and_emptying_removes_the_key() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Consume");
        let (_, custody, active) = custody_world(&mut kernel);

        let after_three = submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Consume {
                holder: Ref::Existing(custody.holder),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(3),
            }]),
        );
        assert!(matches!(after_three, SubmitReceipt::Applied(_)));
        assert_eq!(holding(&kernel, custody.holder, custody.tithe), 4);

        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::Consume {
                holder: Ref::Existing(custody.holder),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(4),
            }]),
        );
        assert!(!kernel.state.holdings.contains_key(&custody.holder));
        let subject = kernel
            .snapshot()
            .unwrap()
            .subjects
            .into_iter()
            .find(|subject| subject.id == custody.holder)
            .expect("the holder is in the snapshot");
        assert!(subject.holdings.is_empty());
    }

    #[test]
    fn a_transform_conserves_one_for_one() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Transform");
        let (_, custody, active) = custody_world(&mut kernel);

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Transform {
                holder: Ref::Existing(custody.holder),
                from_resource: Ref::Existing(custody.tithe),
                into_resource: Ref::Existing(custody.ingot),
                qty: Quantity(5),
            }]),
        );

        assert_eq!(holding(&kernel, custody.holder, custody.tithe), 2);
        assert_eq!(holding(&kernel, custody.holder, custody.ingot), 5);
        assert_eq!(resource_total(&kernel.state, custody.tithe), 2);
        assert_eq!(resource_total(&kernel.state, custody.ingot), 5);
    }

    #[test]
    fn spending_more_than_held_is_rejected_with_insufficient_custody() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Insufficient");
        let (_, custody, _) = custody_world(&mut kernel);
        let too_much = Quantity(OPENING_BALANCE + 1);
        let attempts = [
            ComponentOp::Transfer {
                from: Ref::Existing(custody.holder),
                to: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: too_much,
            },
            ComponentOp::Transform {
                holder: Ref::Existing(custody.holder),
                from_resource: Ref::Existing(custody.tithe),
                into_resource: Ref::Existing(custody.ingot),
                qty: too_much,
            },
            ComponentOp::Consume {
                holder: Ref::Existing(custody.holder),
                resource: Ref::Existing(custody.tithe),
                qty: too_much,
            },
        ];

        for attempt in attempts {
            let snapshot = kernel.snapshot().unwrap();
            let commits = kernel.journal.commit_count();
            let mismatches = reject_owner(&mut kernel, &snapshot, operations(vec![attempt]));
            assert_eq!(
                mismatches,
                vec![Mismatch::InsufficientCustody { operation: 0 }]
            );
            assert_eq!(kernel.journal.commit_count(), commits);
            assert_eq!(kernel.snapshot().unwrap(), snapshot);
        }
    }

    /// A holder with no entry holds zero: there is no second name for it and no
    /// zero entry anywhere in the snapshot.
    #[test]
    fn an_absent_holding_is_zero() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Absent");
        let (_, custody, active) = custody_world(&mut kernel);

        let mismatches = reject_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Consume {
                holder: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(1),
            }]),
        );
        assert_eq!(
            mismatches,
            vec![Mismatch::InsufficientCustody { operation: 0 }]
        );
        let counterparty = kernel
            .snapshot()
            .unwrap()
            .subjects
            .into_iter()
            .find(|subject| subject.id == custody.counterparty)
            .expect("the counterparty is in the snapshot");
        assert!(counterparty.holdings.is_empty());
    }

    #[test]
    fn a_zero_quantity_operation_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Zero");
        let topology = admit_topology(&mut kernel);
        let custody = admit_custody(&mut kernel, &topology);

        // `Admit` is Draft-only, because Active refuses a patch carrying
        // evidence, so its zero case is proven before activation.
        let draft = kernel.snapshot().unwrap();
        let mismatches = reject_owner(
            &mut kernel,
            &draft,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::Admit {
                        holder: Ref::Existing(custody.holder),
                        resource: Ref::Existing(custody.tithe),
                        qty: Quantity(0),
                        evidence: EvidenceRef::new(TITHE_RECEIPT),
                    }],
                    evidence: vec![EvidenceRef::new(TITHE_RECEIPT)],
                },
            },
        );
        assert_eq!(mismatches, vec![Mismatch::ZeroQuantity { operation: 0 }]);

        let active = activate(&mut kernel);
        for attempt in [
            ComponentOp::Transfer {
                from: Ref::Existing(custody.holder),
                to: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(0),
            },
            ComponentOp::Consume {
                holder: Ref::Existing(custody.holder),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(0),
            },
        ] {
            let snapshot = kernel.snapshot().unwrap();
            let mismatches = reject_owner(&mut kernel, &snapshot, operations(vec![attempt]));
            assert_eq!(mismatches, vec![Mismatch::ZeroQuantity { operation: 0 }]);
        }
        assert_eq!(kernel.snapshot().unwrap(), active);
    }

    #[test]
    fn dependency_bind_and_release_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Dependency");
        let (_, custody, active) = custody_world(&mut kernel);
        let before = kernel.state.dependencies.clone();

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Bind {
                subject: Ref::Existing(custody.holder),
                target: DependencyRef::Resource(Ref::Existing(custody.tithe)),
            }]),
        );
        assert_eq!(
            kernel.state.dependencies.get(&custody.holder),
            Some(&BTreeSet::from([DependencyTarget::Resource(custody.tithe)]))
        );

        let bound = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &bound,
            operations(vec![ComponentOp::Release {
                subject: Ref::Existing(custody.holder),
                target: DependencyRef::Resource(Ref::Existing(custody.tithe)),
            }]),
        );
        assert_eq!(kernel.state.dependencies, before);
        assert!(!kernel.state.dependencies.contains_key(&custody.holder));
    }

    /// A bind that changes nothing is `NoOperationEffect`, the name pass 2
    /// already owns; a subject bound to itself is `SelfDependency`.
    #[test]
    fn a_duplicate_dependency_bind_is_rejected_as_no_effect() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Duplicate");
        let (_, custody, active) = custody_world(&mut kernel);

        let bind = ComponentOp::Bind {
            subject: Ref::Existing(custody.holder),
            target: DependencyRef::Resource(Ref::Existing(custody.tithe)),
        };
        submit_owner(&mut kernel, &active, operations(vec![bind.clone()]));

        let bound = kernel.snapshot().unwrap();
        assert_eq!(
            reject_owner(&mut kernel, &bound, operations(vec![bind])),
            vec![Mismatch::NoOperationEffect { operation: 0 }]
        );
        assert_eq!(
            reject_owner(
                &mut kernel,
                &bound,
                operations(vec![ComponentOp::Release {
                    subject: Ref::Existing(custody.holder),
                    target: DependencyRef::Resource(Ref::Existing(custody.ingot)),
                }]),
            ),
            vec![Mismatch::NoOperationEffect { operation: 0 }]
        );
        assert_eq!(
            reject_owner(
                &mut kernel,
                &bound,
                operations(vec![ComponentOp::Bind {
                    subject: Ref::Existing(custody.holder),
                    target: DependencyRef::Subject(Ref::Existing(custody.holder)),
                }]),
            ),
            vec![Mismatch::SelfDependency { operation: 0 }]
        );
    }

    /// Pass 3 owns the representation of a dependency, not its consequence. A
    /// dependency on a closed route commits and shows up in the subject's
    /// components; nothing rejects it and nothing fires from it.
    #[test]
    fn a_dependency_on_a_closed_route_is_representable_and_visible() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Closed");
        let (topology, custody, active) = custody_world(&mut kernel);
        assert!(!kernel.state.edges[&topology.shutter].is_open());

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Bind {
                subject: Ref::Existing(custody.holder),
                target: DependencyRef::Route(Ref::Existing(topology.shutter)),
            }]),
        );

        let subject = kernel
            .snapshot()
            .unwrap()
            .subjects
            .into_iter()
            .find(|subject| subject.id == custody.holder)
            .expect("the holder is in the snapshot");
        assert_eq!(
            subject.dependencies,
            BTreeSet::from([DependencyTarget::Route(topology.shutter)])
        );
    }

    /// The digest reads the acting subject's own holdings and dependencies, and
    /// only its own: a counterparty's churn does not move it.
    #[test]
    fn scope_digest_reads_holdings_and_dependencies() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "ScopeCustody");
        let topology = admit_topology(&mut kernel);
        let custody = admit_custody(&mut kernel, &topology);
        activate(&mut kernel);
        let scope = DecisionScope {
            subject_id: custody.holder,
        };
        let base = scope_digest(&kernel.state, scope).unwrap();

        let mut spent = kernel.state.clone();
        set_held(&mut spent, custody.holder, custody.tithe, 1);
        assert_ne!(scope_digest(&spent, scope).unwrap(), base);

        let mut gained = kernel.state.clone();
        set_held(&mut gained, custody.holder, custody.ingot, 1);
        assert_ne!(scope_digest(&gained, scope).unwrap(), base);

        let mut bound = kernel.state.clone();
        bound
            .dependencies
            .entry(custody.holder)
            .or_default()
            .insert(DependencyTarget::Resource(custody.tithe));
        assert_ne!(scope_digest(&bound, scope).unwrap(), base);

        let mut elsewhere = kernel.state.clone();
        set_held(&mut elsewhere, custody.counterparty, custody.tithe, 9);
        elsewhere
            .dependencies
            .entry(custody.counterparty)
            .or_default()
            .insert(DependencyTarget::Resource(custody.ingot));
        assert_eq!(scope_digest(&elsewhere, scope).unwrap(), base);
    }

    /// A proposal bound while its subject held seven must not commit after
    /// someone took six. Custody churn rejects more bound proposals than route
    /// churn did; that is the binding working.
    #[test]
    fn a_spend_bound_to_a_stale_balance_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Stale");
        let (_, custody, active) = custody_world(&mut kernel);
        let bound = opportunity_for(&active, custody.holder);

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Transfer {
                from: Ref::Existing(custody.holder),
                to: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(6),
            }]),
        );

        let moved = kernel.snapshot().unwrap();
        let commits = kernel.journal.commit_count();
        let error = kernel
            .submit(
                command(
                    &moved,
                    CommandId::new(),
                    CallerId::Controller(bound.controller_id),
                    CommandBody::ExerciseDecision {
                        opportunity: bound.clone(),
                        invocation: DecisionInvocation {
                            affordance: bound.affordance_ids[0],
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Utterance::new("The tithe is short.").unwrap()),
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(CallerId::Controller(bound.controller_id)),
            )
            .unwrap_err();

        assert!(matches!(error, KernelError::ScopeChanged { .. }));
        assert_eq!(kernel.journal.commit_count(), commits);
        assert_eq!(kernel.snapshot().unwrap(), moved);
    }

    /// Soul, proof 4: the ledger equation stated as arithmetic over the
    /// committed partitions, across one patch that exercises every conserving
    /// operation at once. Per resource, `after == before + admitted + gained -
    /// consumed - spent`, and no third resource moves.
    #[test]
    fn soul_a_mixed_patch_conserves_every_resource_it_touches() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "MixedLedger");
        let (_, custody, active) = custody_world(&mut kernel);

        // The opening balance was created by `Admit` alone: before is zero,
        // admitted is the balance, and the total equals the sum of the two.
        assert_eq!(
            resource_total(&kernel.state, custody.tithe),
            u128::from(OPENING_BALANCE)
        );
        assert_eq!(resource_total(&kernel.state, custody.ingot), 0);

        let tithe_before = resource_total(&kernel.state, custody.tithe);
        let ingot_before = resource_total(&kernel.state, custody.ingot);

        let receipt = submit_owner(
            &mut kernel,
            &active,
            operations(vec![
                ComponentOp::Transfer {
                    from: Ref::Existing(custody.holder),
                    to: Ref::Existing(custody.counterparty),
                    resource: Ref::Existing(custody.tithe),
                    qty: Quantity(3),
                },
                ComponentOp::Transform {
                    holder: Ref::Existing(custody.holder),
                    from_resource: Ref::Existing(custody.tithe),
                    into_resource: Ref::Existing(custody.ingot),
                    qty: Quantity(2),
                },
                ComponentOp::Consume {
                    holder: Ref::Existing(custody.counterparty),
                    resource: Ref::Existing(custody.tithe),
                    qty: Quantity(1),
                },
            ]),
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));

        // Transfer contributes to no term; transform contributes one spent and
        // one gained; consume contributes one consumed. Nothing was admitted.
        assert_eq!(
            resource_total(&kernel.state, custody.tithe),
            tithe_before - 1 - 2
        );
        assert_eq!(
            resource_total(&kernel.state, custody.ingot),
            ingot_before + 2
        );
        assert_eq!(holding(&kernel, custody.holder, custody.tithe), 2);
        assert_eq!(holding(&kernel, custody.holder, custody.ingot), 2);
        assert_eq!(holding(&kernel, custody.counterparty, custody.tithe), 2);
        // Absence is zero: the counterparty never held an ingot, so there is no
        // row for one rather than a row holding nothing.
        assert!(!kernel.state.holdings[&custody.counterparty].contains_key(&custody.ingot));
    }

    /// Soul: is the apply-side `check_ledger` reachable?
    ///
    /// A forged `PatchAdmitted` can carry a *sequence* whose second operation
    /// exceeds what the first left. It dies at `apply_operation`'s own
    /// re-derived precondition, one operation early, so the ledger never speaks.
    /// A sequence in which every operation applies cannot break the equation,
    /// because every `ResolvedOp` arm debits and credits the same `qty` it
    /// records: the apply-side call is unreachable from the current closed set.
    #[test]
    fn soul_a_forged_sequence_dies_at_the_per_operation_check_not_the_ledger() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "ForgedSequence");
        let (_, custody, _) = custody_world(&mut kernel);
        let before = kernel.state.holdings.clone();

        let forge = |operations: Vec<ResolvedOp>| WorldEffect::PatchAdmitted {
            resolved: ResolvedPatch {
                subjects: Vec::new(),
                entities: Vec::new(),
                routes: Vec::new(),
                affordances: Vec::new(),
                operations,
                evidence: Vec::new(),
            },
        };
        let transfer = |qty: u64| ResolvedOp::Transfer {
            from: custody.holder,
            to: custody.counterparty,
            resource: custody.tithe,
            qty: Quantity(qty),
        };

        // Two transfers of five out of a balance of seven: the first leaves two,
        // the second cannot cover five.
        let mut candidate = kernel.state.clone();
        let error = apply_effect(
            &mut candidate,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &forge(vec![transfer(5), transfer(5)]),
        )
        .unwrap_err();
        assert!(
            matches!(&error, KernelError::Invariant(message) if message == "holder does not hold enough"),
            "the per-operation precondition speaks first, not the ledger: {error:?}"
        );
        // The candidate is a throwaway clone; the live state never moved.
        assert_eq!(kernel.state.holdings, before);

        // Every operation applying means the equation balances, so the same
        // forged lane commits a conserving sequence with no ledger complaint.
        let mut conserving = kernel.state.clone();
        apply_effect(
            &mut conserving,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &forge(vec![
                transfer(5),
                ResolvedOp::Transfer {
                    from: custody.counterparty,
                    to: custody.holder,
                    resource: custody.tithe,
                    qty: Quantity(5),
                },
            ]),
        )
        .expect("a conserving forged sequence applies");
        assert_eq!(conserving.holdings, before);

        // The degenerate shapes a forgery could otherwise use to mint or burn
        // are refused by the arms themselves, not by the equation.
        for degenerate in [
            ResolvedOp::Transfer {
                from: custody.holder,
                to: custody.holder,
                resource: custody.tithe,
                qty: Quantity(1),
            },
            ResolvedOp::Transform {
                holder: custody.holder,
                from_resource: custody.tithe,
                into_resource: custody.tithe,
                qty: Quantity(1),
            },
        ] {
            let mut candidate = kernel.state.clone();
            let error = apply_effect(
                &mut candidate,
                CommandId::issue(),
                &CallerId::Principal(owner()),
                &forge(vec![degenerate]),
            )
            .unwrap_err();
            assert!(
                matches!(&error, KernelError::Invariant(message) if message == "custody operation moves nothing"),
                "unexpected refusal: {error:?}"
            );
        }

        // An `Admit` in a forged effect still needs its receipt in the same
        // patch, so the apply lane mints no unattributable quantity either.
        let mut unevidenced = kernel.state.clone();
        let error = apply_effect(
            &mut unevidenced,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &forge(vec![ResolvedOp::Admit {
                holder: custody.holder,
                resource: custody.tithe,
                qty: Quantity(4),
                evidence: EvidenceRef::new(TITHE_RECEIPT),
            }]),
        )
        .unwrap_err();
        assert!(
            matches!(&error, KernelError::Invariant(message) if message == "admitted quantity cites no evidence in its patch"),
            "unexpected refusal: {error:?}"
        );
        assert_eq!(kernel.state.holdings, before);
    }

    /// Soul: `QuantityOverflow` is reachable, and the arithmetic that reaches it
    /// is `checked_add` rather than a silent clamp. Two admissions in one patch
    /// whose sum exceeds `u64::MAX` mint nothing and allocate no ID.
    #[test]
    fn soul_a_holding_cannot_overflow_silently() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Overflow");
        let topology = admit_topology(&mut kernel);
        let speak = speak_entry(&kernel);
        let before = kernel.snapshot().unwrap();
        let commits = kernel.journal.commit_count();

        let admit = |qty: u64| ComponentOp::Admit {
            holder: Ref::Draft(DraftHandle::new("clerk")),
            resource: Ref::Draft(DraftHandle::new("tithe")),
            qty: Quantity(qty),
            evidence: EvidenceRef::new(TITHE_RECEIPT),
        };
        let mismatches = reject_owner(
            &mut kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: vec![
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("tithe"),
                            label: "The Rhythm Tithe".into(),
                            kind: EntityKind::Resource,
                            container: None,
                        }),
                        Declaration::Subject(SubjectDeclaration {
                            handle: DraftHandle::new("clerk"),
                            label: "The Ledger Clerk".into(),
                            kind: SubjectKind::Institution,
                            controller: NewController::OperationalAgent,
                            affordances: BTreeSet::from([speak.clone()]),
                            position: Some(Ref::Existing(topology.yard)),
                        }),
                    ],
                    operations: vec![admit(u64::MAX), admit(1)],
                    evidence: vec![EvidenceRef::new(TITHE_RECEIPT)],
                },
            },
        );

        assert_eq!(
            mismatches,
            vec![Mismatch::QuantityOverflow { operation: 1 }],
            "the overflow is the complete verdict: no clamp, no second name"
        );
        assert!(kernel.state.holdings.is_empty());
        assert_eq!(kernel.journal.commit_count(), commits);
        assert_eq!(kernel.snapshot().unwrap(), before);
    }

    /// Soul, scope: a transfer changes the *receiver's* components too, so the
    /// counterparty's in-flight proposal is refused with `ScopeChanged` while an
    /// unrelated subject's bound proposal still commits at the later revision.
    #[test]
    fn soul_an_incoming_transfer_changes_only_the_counterpartys_scope() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "IncomingScope");
        let (topology, custody, active) = custody_world(&mut kernel);
        let receiver = opportunity_for(&active, custody.counterparty);
        let bystander = opportunity_for(&active, topology.walker);

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Transfer {
                from: Ref::Existing(custody.holder),
                to: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(3),
            }]),
        );
        let moved = kernel.snapshot().unwrap();

        let exercise = |opportunity: &DecisionOpportunity| CommandBody::ExerciseDecision {
            opportunity: opportunity.clone(),
            invocation: DecisionInvocation {
                affordance: opportunity.affordance_ids[0],
                bindings: Vec::new(),
                proposed: Vec::new(),
                speech: Some(Utterance::new("Counted before the tithe arrived.").unwrap()),
            },
        };

        let error = kernel
            .submit(
                command(
                    &moved,
                    CommandId::new(),
                    CallerId::Controller(receiver.controller_id),
                    exercise(&receiver),
                ),
                &AuthenticatedCaller::fixture(CallerId::Controller(receiver.controller_id)),
            )
            .unwrap_err();
        assert!(
            matches!(error, KernelError::ScopeChanged { .. }),
            "the receiver's holdings moved under its bound proposal: {error:?}"
        );

        let receipt = kernel
            .submit(
                command(
                    &moved,
                    CommandId::new(),
                    CallerId::Controller(bystander.controller_id),
                    exercise(&bystander),
                ),
                &AuthenticatedCaller::fixture(CallerId::Controller(bystander.controller_id)),
            )
            .expect("an unrelated subject's bound proposal is untouched by the transfer");
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
    }
}

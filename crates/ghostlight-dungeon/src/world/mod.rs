//! Sealed replacement world owner under construction.
//!
//! The world owner is one deterministic authority: authenticated commands enter,
//! one reducer decides, and one journal atomically commits the resulting state.
//! Controllers may use models, but models never own lifecycle, scope, affordances,
//! opportunities, reduction, or persistence.

mod controllers;
mod journal;
mod mailbox;
mod patch;

pub(crate) use controllers::{
    ControllerError, ControllerModels, ControllerOpenError, ControllerPendingReason,
    ControllerRunner, ControllerWorkCustody, NarrativeCapture, NarrativeDecision, NarrativePending,
    NarrativeRun, OperationalCapture, OperationalDecision, OperationalPending, OperationalRun,
    SourceRange, SubmissionDisposition, TranslationGapSummary,
};
pub(crate) use mailbox::{MailboxError, WorldMailbox};
pub(crate) use patch::{
    AccessKind, Cost, Declaration, DependencyTarget, DraftHandle, EntityDeclaration, EntityKind,
    EvidenceRef, Mismatch, PatchAnswer, Position, Quantity, Ref, SubjectDeclaration, WorldPatch,
};
#[cfg(test)]
use patch::{ComponentOp, DependencyRef, RouteDeclaration};
use patch::{EdgeRecord, EntityRecord, LedgerDelta, ResolvedOp, ResolvedPatch};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

pub(crate) const STATE_SCHEMA: &str = "ghostlight.world_state.custody.v1";
pub(crate) const COMMIT_SCHEMA: &str = "ghostlight.world_commit.custody.v1";

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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AffordanceKind {
    Speak,
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
    affordances: BTreeMap<AffordanceId, &'a AffordanceGrant>,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum DecisionAction {
    Speak { text: String },
}

impl DecisionAction {
    fn kind(&self) -> AffordanceKind {
        match self {
            Self::Speak { .. } => AffordanceKind::Speak,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionInvocation {
    pub(crate) affordance_id: AffordanceId,
    pub(crate) action: DecisionAction,
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
struct AffordanceGrant {
    scope: DecisionScope,
    kind: AffordanceKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DecisionEvent {
    pub(crate) id: EventId,
    pub(crate) revision: u64,
    pub(crate) scope: DecisionScope,
    pub(crate) controller_id: ControllerId,
    pub(crate) invocation: DecisionInvocation,
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
    controller_assignments: BTreeMap<DecisionScope, ControllerAssignment>,
    affordance_grants: BTreeMap<AffordanceId, AffordanceGrant>,
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
    pub(crate) affordances: BTreeMap<AffordanceKind, AffordanceId>,
    pub(crate) position: Option<EntityId>,
    /// This subject's own holdings and dependencies, and the routes incident to
    /// its place: exactly what its scope digest reads, lowered from the one
    /// `scope_components` owner.
    pub(crate) holdings: BTreeMap<EntityId, Quantity>,
    pub(crate) dependencies: BTreeSet<DependencyTarget>,
    pub(crate) incident_routes: Vec<EdgeId>,
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
    #[error("spoken action must not be empty")]
    EmptySpeech,
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
    #[error("decision action does not match its affordance")]
    AffordanceMismatch,
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
        apply_effect(&mut candidate, &command.caller, &effect)?;
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
            if !current.affordance_ids.contains(&invocation.affordance_id) {
                return Err(KernelError::AffordanceDenied);
            }
            let grant = state
                .affordance_grants
                .get(&invocation.affordance_id)
                .filter(|grant| grant.scope == current.scope)
                .ok_or(KernelError::AffordanceDenied)?;
            if grant.kind != invocation.action.kind() {
                return Err(KernelError::AffordanceMismatch);
            }
            let invocation = validated_invocation(invocation)?;
            let resulting_revision = state
                .revision
                .checked_add(1)
                .ok_or_else(|| KernelError::Serialization("world revision overflow".into()))?;
            let event = DecisionEvent {
                id: EventId::for_command(command.id),
                revision: resulting_revision,
                scope: current.scope,
                controller_id: current.controller_id,
                invocation: invocation.clone(),
            };
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
            controller_assignments: BTreeMap::new(),
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
    let mut scope_kinds: BTreeSet<(DecisionScope, AffordanceKind)> = state
        .affordance_grants
        .values()
        .map(|grant| (grant.scope, grant.kind))
        .collect();
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
        for (affordance_id, grant) in &subject.affordances {
            if grant.scope != scope
                || !scope_kinds.insert((scope, grant.kind))
                || state
                    .affordance_grants
                    .insert(*affordance_id, grant.clone())
                    .is_some()
            {
                return Err(KernelError::Invariant(
                    "admitted affordance is unscoped or collides".into(),
                ));
            }
        }
    }
    // A forged `PatchAdmitted` reaches this function without ever passing the
    // resolver, so conservation is re-derived here over the committed
    // partitions, against the same equation, with `before` read from state
    // rather than from anything the effect asserts.
    let mut deltas: BTreeMap<EntityId, LedgerDelta> = BTreeMap::new();
    for operation in &resolved.operations {
        for resource in operation_resources(operation) {
            deltas.entry(resource).or_default().before = resource_total(state, resource);
        }
    }
    for operation in &resolved.operations {
        apply_operation(state, operation, &resolved.evidence)?;
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
            if !route.is_open()
                || route.access() != AccessKind::Public
                || state.positions.get(subject_id) != Some(&Position { place: from })
            {
                return Err(KernelError::Invariant(
                    "relocation does not traverse an open public route from the subject's place"
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
    }
    Ok(())
}

/// Exactly the components a subject's verification reads. One owner, consumed by
/// the scope digest and by the snapshot, so the two cannot drift. `routes` is
/// every edge whose `from` or `to` is the subject's place: an inbound route
/// decides who can arrive, and reading too little is a correctness hole while
/// reading too much only costs an extra rejection. `holdings` and `dependencies`
/// are the acting subject's own; a counterparty's holdings do not enter, because
/// a transfer changes both subjects' components and so changes both digests.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(super) struct ScopeComponents {
    position: Option<Position>,
    routes: BTreeMap<EdgeId, EdgeRecord>,
    holdings: BTreeMap<EntityId, Quantity>,
    dependencies: BTreeSet<DependencyTarget>,
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
                .iter()
                .filter(|(_, grant)| grant.scope == scope)
                .map(|(affordance_id, grant)| (grant.kind, *affordance_id))
                .collect();
            let components = scope_components(state, *subject_id);
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
                access: record.access(),
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
                .iter()
                .filter(|(_, grant)| grant.scope == *scope)
                .map(|(affordance_id, _)| *affordance_id)
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

/// The sole producer of a `ScopeDigest`. The components it reads come from
/// `scope_components`, which the snapshot reads too.
fn scope_digest(state: &WorldState, scope: DecisionScope) -> Result<ScopeDigest, KernelError> {
    let controller = state
        .controller_assignments
        .get(&scope)
        .ok_or_else(|| KernelError::Invariant("decision scope has no controller".into()))?;
    let affordances = state
        .affordance_grants
        .iter()
        .filter(|(_, grant)| grant.scope == scope)
        .map(|(affordance_id, grant)| (*affordance_id, grant))
        .collect();
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

/// The one validity check for a bound proposal. Controller, mode, and affordance
/// IDs are inside the preimage, so the digest comparison is the whole check.
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

fn validated_invocation(value: &DecisionInvocation) -> Result<DecisionInvocation, KernelError> {
    let action = match &value.action {
        DecisionAction::Speak { text } if text.trim().is_empty() => {
            return Err(KernelError::EmptySpeech);
        }
        DecisionAction::Speak { text } => DecisionAction::Speak { text: text.clone() },
    };
    Ok(DecisionInvocation {
        affordance_id: value.affordance_id,
        action,
    })
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

fn apply_effect(
    state: &mut WorldState,
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
            let grant = state
                .affordance_grants
                .get(&event.invocation.affordance_id)
                .ok_or(KernelError::AffordanceDenied)?;
            let expected_revision = state
                .revision
                .checked_add(1)
                .ok_or_else(|| KernelError::Serialization("world revision overflow".into()))?;
            if state.phase != WorldPhase::Active
                || caller != &assignment.expected_caller()
                || event.revision != expected_revision
                || event.scope != current.scope
                || event.controller_id != current.controller_id
                || !current
                    .affordance_ids
                    .contains(&event.invocation.affordance_id)
                || grant.scope != current.scope
                || grant.kind != event.invocation.action.kind()
                || validated_invocation(&event.invocation)? != event.invocation
            {
                return Err(KernelError::Invariant(
                    "decision effect does not match exact opportunity authority".into(),
                ));
            }
            state.events.push(event.clone());
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
            affordances: BTreeSet::from([AffordanceKind::Speak]),
            position: None,
        })
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
                    subject(
                        "operator",
                        "The Council",
                        SubjectKind::Institution,
                        NewController::OperationalAgent,
                    ),
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

    pub(super) fn topology_patch() -> WorldPatch {
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
                    AccessKind::Restricted,
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
                    affordances: BTreeSet::from([AffordanceKind::Speak]),
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
                patch: topology_patch(),
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

    fn holder(handle: &str, label: &str, place: EntityId) -> Declaration {
        Declaration::Subject(SubjectDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: SubjectKind::Institution,
            controller: NewController::OperationalAgent,
            affordances: BTreeSet::from([AffordanceKind::Speak]),
            position: Some(Ref::Existing(place)),
        })
    }

    /// Declarations and evidence are Draft-only, so the resources, the holders,
    /// and the one evidenced `Admit` that creates the opening balance all land
    /// before activation. There is no holdings declaration field: quantity is
    /// created by `Admit` and by nothing else, in this lane as in every other.
    pub(super) fn custody_patch(topology: &Topology) -> WorldPatch {
        WorldPatch {
            declarations: vec![
                resource("tithe", "The Rhythm Tithe"),
                resource("ingot", "The Cut Ingot"),
                holder("clerk", "The Ledger Clerk", topology.yard),
                holder("keeper", "The Gate Keeper", topology.gate),
            ],
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
                patch: custody_patch(topology),
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
            affordance_id: opportunity.affordance_ids[0],
            action: DecisionAction::Speak { text: text.into() },
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
        changed_grants.affordance_grants.insert(
            AffordanceId::issue(),
            AffordanceGrant {
                scope,
                kind: AffordanceKind::Speak,
            },
        );
        assert_ne!(scope_digest(&changed_grants, scope).unwrap(), base);

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
                        invocation: speak(&player_opportunity, "  I open the door.  "),
                        opportunity: player_opportunity,
                    },
                ),
                &auth_principal(player()),
            )
            .unwrap();
        let after_player = kernel.snapshot().unwrap();
        assert_eq!(after_player.events.len(), 1);
        assert_eq!(
            after_player.events[0].invocation.action,
            DecisionAction::Speak {
                text: "  I open the door.  ".into()
            }
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
        let other_scope = opportunity(&active, ControllerMode::NarrativePersona);
        let denied_invocation = DecisionInvocation {
            affordance_id: other_scope.affordance_ids[0],
            action: DecisionAction::Speak {
                text: "No grant".into(),
            },
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
                            affordance_id: forged_affordance,
                            action: DecisionAction::Speak {
                                text: "Forged".into()
                            },
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
        invalid.patch.declarations[1] = invalid.patch.declarations[0].clone();
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
                            affordances: BTreeSet::from([AffordanceKind::Speak]),
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

        // The persona claims the operator's affordance and lists it as its own.
        let mut forged = persona.clone();
        forged.affordance_ids = vec![operator.affordance_ids[0], persona.affordance_ids[0]];
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
                            affordance_id: operator.affordance_ids[0],
                            action: DecisionAction::Speak {
                                text: "Not my grant.".into(),
                            },
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .unwrap_err();
        assert!(matches!(error, KernelError::AffordanceDenied));
        assert_eq!(kernel.snapshot().unwrap(), active);
    }
}

#[cfg(test)]
mod custody_tests {
    use super::tests::{
        OPENING_BALANCE, TITHE_RECEIPT, activate, admit_custody, admit_topology, auth_principal,
        command, creation, custody_world, operations, opportunity_for, owner, reject_owner,
        submit_owner,
    };
    use super::*;

    fn custody_kernel(path: &Path, title: &str) -> WorldKernel {
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
                operations: vec![ResolvedOp::Transfer {
                    from: custody.holder,
                    to: custody.counterparty,
                    resource: custody.tithe,
                    qty: Quantity(OPENING_BALANCE + 1),
                }],
                evidence: Vec::new(),
            },
        };
        let error =
            apply_effect(&mut candidate, &CallerId::Principal(owner()), &forged).unwrap_err();
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
                            affordances: BTreeSet::from([AffordanceKind::Speak]),
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
                            affordance_id: bound.affordance_ids[0],
                            action: DecisionAction::Speak {
                                text: "The tithe is short.".into(),
                            },
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
}

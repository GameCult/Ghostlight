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
    Declaration, DraftHandle, EntityDeclaration, EntityKind, Mismatch, PatchAnswer, Ref,
    SubjectDeclaration, WorldPatch,
};
use patch::{EdgeRecord, EntityRecord, ResolvedPatch};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

const STATE_SCHEMA: &str = "ghostlight.world_state.foundation.v1";
const COMMIT_SCHEMA: &str = "ghostlight.world_commit.foundation.v2";

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionOpportunity {
    pub(crate) world_id: WorldId,
    pub(crate) revision: u64,
    pub(crate) state_digest: String,
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
    authority_scope: Option<EntityId>,
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
    #[error("decision opportunity is stale or does not exactly derive from current state")]
    OpportunityMismatch,
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
    let resolved = patch::resolve_declarations(
        &BTreeMap::new(),
        &BTreeMap::new(),
        world_id,
        input.id,
        &input.patch,
        true,
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
        if let Some(commit) = self.committed_command(command.id) {
            return if commit.command == CommittedCommand::WorldCommand(command.clone()) {
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
            require_phase(state, WorldPhase::Draft)?;
            require_owner(state, &command.caller)?;
            let resolved = patch::resolve_declarations(
                &state.subjects,
                &state.entities,
                state.world_id,
                command.id,
                patch,
                admits_human(state),
            )
            .map_err(KernelError::PatchRejected)?;
            Ok(WorldEffect::PatchAdmitted { resolved })
        }
    }
}

/// A human principal joins `required_approvers`, so only the lane that builds
/// revision 0 from nothing may bind one. The predicate reads state, never the
/// caller.
fn admits_human(state: &WorldState) -> bool {
    state.revision == 0 && state.subjects.is_empty()
}

impl WorldState {
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
        let expected = patch::resolve_declarations(
            &BTreeMap::new(),
            &BTreeMap::new(),
            world_id,
            command.id,
            &command.patch,
            true,
        )
        .map_err(KernelError::PatchRejected)?;
        if &expected != resolved {
            return Err(KernelError::Invariant(
                "genesis effect does not derive from its creation command".into(),
            ));
        }

        let mut state = Self {
            schema: STATE_SCHEMA.into(),
            world_id,
            revision: 0,
            phase: WorldPhase::Draft,
            owner: owner.clone(),
            title: title.clone(),
            draft_approvals: BTreeSet::new(),
            subjects: BTreeMap::new(),
            entities: BTreeMap::new(),
            edges: BTreeMap::new(),
            controller_assignments: BTreeMap::new(),
            affordance_grants: BTreeMap::new(),
            events: Vec::new(),
            state_digest: String::new(),
            last_commit_digest: None,
        };
        admit_resolved(&mut state, resolved)?;
        state.state_digest = state_digest(&state)?;
        Ok(state)
    }
}

/// The only writer of the subject, entity, controller, and grant partitions.
/// Both admission lanes mutate through it, and it re-derives every structural
/// claim from `state`, so an effect that skipped resolution dies here.
fn admit_resolved(state: &mut WorldState, resolved: &ResolvedPatch) -> Result<(), KernelError> {
    if resolved.subjects.is_empty() && resolved.entities.is_empty() {
        return Err(KernelError::Invariant(
            "admitted patch carries no canonical change".into(),
        ));
    }
    let humans_admitted = admits_human(state);
    for entity in &resolved.entities {
        if !patch::is_canonical_text(&entity.entity.label) {
            return Err(KernelError::Invariant(
                "admitted entity label is not canonical".into(),
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
        if let Some(entity_id) = subject.subject.authority_scope
            && state
                .entities
                .get(&entity_id)
                .is_none_or(|entity| entity.kind != EntityKind::Place)
        {
            return Err(KernelError::Invariant(
                "admitted authority scope does not name a canonical place".into(),
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
    Ok(())
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
            Ok(SubjectSnapshot {
                id: *subject_id,
                label: subject.label.clone(),
                kind: subject.kind,
                controller_id: controller.id(),
                controller_mode: controller.mode(),
                human_controller: controller.human_principal().cloned(),
                affordances,
            })
        })
        .collect::<Result<Vec<_>, KernelError>>()?;
    Ok(WorldSnapshot {
        world_id: state.world_id,
        revision: state.revision,
        phase: state.phase,
        owner: state.owner.clone(),
        title: state.title.clone(),
        draft_approvals: state.draft_approvals.clone(),
        required_approvers: required_approvers(state),
        subjects,
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
                state_digest: state.state_digest.clone(),
                scope: *scope,
                controller_id: controller.id(),
                controller_mode: controller.mode(),
                affordance_ids,
            })
        })
        .collect()
}

fn exact_opportunity(
    state: &WorldState,
    claimed: &DecisionOpportunity,
) -> Result<DecisionOpportunity, KernelError> {
    if claimed.world_id != state.world_id
        || claimed.revision != state.revision
        || claimed.state_digest != state.state_digest
    {
        return Err(KernelError::OpportunityMismatch);
    }
    derive_opportunities(state)?
        .into_iter()
        .find(|current| current.scope == claimed.scope)
        .filter(|current| current == claimed)
        .ok_or(KernelError::OpportunityMismatch)
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
            if state.phase != WorldPhase::Draft
                || caller != &CallerId::Principal(state.owner.clone())
            {
                return Err(KernelError::Invariant(
                    "admitted patch does not satisfy draft authority".into(),
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
            authority_scope: None,
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
            state_digest: "sha256:fixture".into(),
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
    fn stale_or_tampered_opportunities_never_commit() {
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
        let mut tampered = original.clone();
        tampered.affordance_ids.push(AffordanceId::issue());
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&original, "No"),
                        opportunity: tampered,
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::OpportunityMismatch)
        ));

        kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&original, "Yes"),
                        opportunity: original.clone(),
                    },
                ),
                &auth_principal(player()),
            )
            .unwrap();
        let after = kernel.snapshot().unwrap();
        let stale_command = command(
            &after,
            CommandId::new(),
            CallerId::Principal(player()),
            CommandBody::ExerciseDecision {
                invocation: speak(&original, "Again"),
                opportunity: original,
            },
        );
        assert!(matches!(
            kernel.submit(stale_command, &auth_principal(player())),
            Err(KernelError::OpportunityMismatch)
        ));
        assert_eq!(kernel.snapshot().unwrap(), after);
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
        assert!(matches!(
            kernel.submit(
                command(
                    &after,
                    CommandId::new(),
                    caller.clone(),
                    CommandBody::DeclineDecision {
                        opportunity: decision,
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            ),
            Err(KernelError::OpportunityMismatch)
        ));
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
}

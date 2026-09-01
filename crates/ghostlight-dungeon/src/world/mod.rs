//! Sealed replacement world owner under construction.
//!
//! The world owner is one deterministic authority: authenticated commands enter,
//! one reducer decides, and one journal atomically commits the resulting state.
//! Controllers may use models, but models never own lifecycle, scope, affordances,
//! opportunities, reduction, or persistence.

mod journal;
mod mailbox;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use thiserror::Error;
use uuid::Uuid;

const STATE_SCHEMA: &str = "ghostlight.world_state.foundation.v0";
const COMMIT_SCHEMA: &str = "ghostlight.world_commit.foundation.v0";

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

impl SubjectId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }
}

impl ControllerId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }
}

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct DraftSubjectHandle(String);

impl DraftSubjectHandle {
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
pub(crate) struct NewDecisionSubject {
    pub(crate) handle: DraftSubjectHandle,
    pub(crate) label: String,
    pub(crate) kind: SubjectKind,
    pub(crate) controller: NewController,
    pub(crate) affordances: BTreeSet<AffordanceKind>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CreateWorld {
    pub(crate) id: CommandId,
    pub(crate) owner: PrincipalId,
    pub(crate) title: String,
    pub(crate) subjects: Vec<NewDecisionSubject>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub(crate) enum CallerId {
    Principal(PrincipalId),
    Controller(ControllerId),
}

/// Sealed identity evidence. Production construction belongs to the app-session
/// identity owner when runtime ingress moves onto this boundary.
#[derive(Clone, Debug)]
pub(crate) struct AuthenticatedCaller {
    caller: CallerId,
}

impl AuthenticatedCaller {
    #[cfg(test)]
    fn fixture(caller: CallerId) -> Self {
        Self { caller }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DecisionScope {
    pub(crate) subject_id: SubjectId,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DecisionOpportunity {
    pub(crate) world_id: WorldId,
    pub(crate) revision: u64,
    pub(crate) state_digest: String,
    pub(crate) scope: DecisionScope,
    pub(crate) controller_id: ControllerId,
    pub(crate) controller_mode: ControllerMode,
    pub(crate) affordance_ids: Vec<AffordanceId>,
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
pub(crate) struct DecisionInvocation {
    pub(crate) affordance_id: AffordanceId,
    pub(crate) action: DecisionAction,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CommandEnvelope {
    pub(crate) id: CommandId,
    pub(crate) world_id: WorldId,
    pub(crate) expected_revision: u64,
    pub(crate) caller: CallerId,
    pub(crate) body: CommandBody,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum CommandBody {
    SetTitle {
        title: String,
    },
    ApproveDraft,
    ActivateWorld,
    ExerciseDecision {
        opportunity: DecisionOpportunity,
        invocation: DecisionInvocation,
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
    controller_assignments: BTreeMap<DecisionScope, ControllerAssignment>,
    affordance_grants: BTreeMap<AffordanceId, AffordanceGrant>,
    events: Vec<DecisionEvent>,
    state_digest: String,
    last_commit_digest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct GenesisSubjectBinding {
    handle: DraftSubjectHandle,
    subject_id: SubjectId,
    subject: SubjectState,
    controller: ControllerAssignment,
    affordances: BTreeMap<AffordanceId, AffordanceGrant>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorldEffect {
    WorldCreated {
        owner: PrincipalId,
        title: String,
        bindings: Vec<GenesisSubjectBinding>,
    },
    TitleChanged {
        previous_title: String,
        resulting_title: String,
    },
    DraftApproved {
        principal: PrincipalId,
    },
    WorldActivated,
    DecisionExercised {
        opportunity: DecisionOpportunity,
        event: DecisionEvent,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct WorldCommit {
    schema: String,
    world_id: WorldId,
    command_id: CommandId,
    command_digest: String,
    previous_revision: Option<u64>,
    resulting_revision: u64,
    previous_state_digest: Option<String>,
    resulting_state_digest: String,
    previous_commit_digest: Option<String>,
    caller: CallerId,
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
pub(crate) struct CreatedSubject {
    pub(crate) handle: DraftSubjectHandle,
    pub(crate) subject_id: SubjectId,
    pub(crate) controller_id: ControllerId,
    pub(crate) affordances: BTreeMap<AffordanceKind, AffordanceId>,
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
    pub(crate) subjects: Vec<CreatedSubject>,
    pub(crate) resulting_state_digest: String,
    pub(crate) commit_digest: String,
}

impl CreationReceipt {
    fn from_commit(commit: &WorldCommit) -> Result<Self, KernelError> {
        let WorldEffect::WorldCreated { bindings, .. } = &commit.effect else {
            return Err(KernelError::Invariant(
                "world genesis receipt does not point to genesis".into(),
            ));
        };
        let subjects = bindings
            .iter()
            .map(|binding| CreatedSubject {
                handle: binding.handle.clone(),
                subject_id: binding.subject_id,
                controller_id: binding.controller.id(),
                affordances: binding
                    .affordances
                    .iter()
                    .map(|(affordance_id, grant)| (grant.kind, *affordance_id))
                    .collect(),
            })
            .collect();
        Ok(Self {
            command_id: commit.command_id,
            world_id: commit.world_id,
            subjects,
            resulting_state_digest: commit.resulting_state_digest.clone(),
            commit_digest: commit.digest.clone(),
        })
    }
}

impl From<&WorldCommit> for CommitReceipt {
    fn from(commit: &WorldCommit) -> Self {
        Self {
            command_id: commit.command_id,
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
    NoEffect(WorldSnapshot),
}

#[derive(Debug, Error)]
pub(crate) enum KernelError {
    #[error("world title must not be empty")]
    EmptyTitle,
    #[error("world owner or human controller principal must be canonical and nonempty")]
    EmptyPrincipal,
    #[error("world creation requires at least one decision subject")]
    NoSubjects,
    #[error("decision subject handle must be canonical and nonempty")]
    EmptySubjectHandle,
    #[error("decision subject label must not be empty")]
    EmptySubjectLabel,
    #[error("decision subject handles must be unique")]
    DuplicateSubjectHandle,
    #[error("every decision subject requires at least one affordance")]
    NoAffordances,
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
    #[error("draft title is locked after the first approval")]
    DraftLocked,
    #[error("caller is not a required draft approver")]
    NotDraftApprover,
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

impl From<journal::JournalError> for KernelError {
    fn from(error: journal::JournalError) -> Self {
        match error {
            journal::JournalError::NotEmpty => Self::CreationTargetOccupied,
            journal::JournalError::WorldMismatch => Self::OpenedWorldMismatch,
            journal::JournalError::CreationConflict => Self::CreationConflict,
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

impl WorldKernel {
    fn create(
        path: impl AsRef<Path>,
        input: CreateWorld,
        authenticated: &AuthenticatedCaller,
    ) -> Result<(Self, CreationReceipt), KernelError> {
        let CallerId::Principal(authenticated_principal) = &authenticated.caller else {
            return Err(KernelError::AuthenticationMismatch);
        };
        validate_principal(&input.owner)?;
        if &input.owner != authenticated_principal {
            return Err(KernelError::AuthenticationMismatch);
        }
        let canonical_subjects = canonicalize_subjects(&input.subjects)?;
        let title = normalize_title(&input.title)?;
        let creation_digest = digest(&input)?;
        let (journal, state) = match journal::WorldJournal::open_for_creation(
            path.as_ref(),
            input.id,
            &creation_digest,
        )? {
            journal::CreationOpen::Existing { journal, state } => (journal, state),
            journal::CreationOpen::Empty(empty) => {
                let world_id = WorldId::issue();
                let caller = CallerId::Principal(input.owner.clone());
                let effect = issue_genesis(input.owner, title, canonical_subjects);
                let mut state = WorldState::genesis(world_id, &caller, &effect)?;
                let mut genesis = WorldCommit {
                    schema: COMMIT_SCHEMA.into(),
                    world_id,
                    command_id: input.id,
                    command_digest: creation_digest,
                    previous_revision: None,
                    resulting_revision: 0,
                    previous_state_digest: None,
                    resulting_state_digest: state.state_digest.clone(),
                    previous_commit_digest: None,
                    caller,
                    effect,
                    committed_at: Utc::now(),
                    digest: String::new(),
                };
                genesis.digest = commit_digest(&genesis)?;
                state.last_commit_digest = Some(genesis.digest.clone());
                let journal = empty.initialize(&state, &genesis)?;
                (journal, state)
            }
        };
        let receipt = journal
            .commit_for(input.id)
            .ok_or_else(|| KernelError::Invariant("world genesis receipt is missing".into()))
            .and_then(CreationReceipt::from_commit)?;
        Ok((Self { state, journal }, receipt))
    }

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
        let command_digest = digest(&command)?;
        if let Some(commit) = self.committed_command(command.id) {
            return if commit.command_digest == command_digest {
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

        let Reduction::Apply(effect) = reduce(&self.state, &command)? else {
            return Ok(SubmitReceipt::NoEffect(snapshot(&self.state)?));
        };
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
            command_id: command.id,
            command_digest,
            previous_revision: Some(self.state.revision),
            resulting_revision: candidate.revision,
            previous_state_digest: Some(self.state.state_digest.clone()),
            resulting_state_digest: candidate.state_digest.clone(),
            previous_commit_digest: self.state.last_commit_digest.clone(),
            caller: command.caller,
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
}

enum Reduction {
    NoEffect,
    Apply(WorldEffect),
}

fn reduce(state: &WorldState, command: &CommandEnvelope) -> Result<Reduction, KernelError> {
    match &command.body {
        CommandBody::SetTitle { title } => {
            require_owner(state, &command.caller)?;
            require_phase(state, WorldPhase::Draft)?;
            let title = normalize_title(title)?;
            if title == state.title {
                return Ok(Reduction::NoEffect);
            }
            if !state.draft_approvals.is_empty() {
                return Err(KernelError::DraftLocked);
            }
            Ok(Reduction::Apply(WorldEffect::TitleChanged {
                previous_title: state.title.clone(),
                resulting_title: title,
            }))
        }
        CommandBody::ApproveDraft => {
            require_phase(state, WorldPhase::Draft)?;
            let CallerId::Principal(principal) = &command.caller else {
                return Err(KernelError::NotDraftApprover);
            };
            if !required_approvers(state).contains(principal) {
                return Err(KernelError::NotDraftApprover);
            }
            if state.draft_approvals.contains(principal) {
                return Ok(Reduction::NoEffect);
            }
            Ok(Reduction::Apply(WorldEffect::DraftApproved {
                principal: principal.clone(),
            }))
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
            Ok(Reduction::Apply(WorldEffect::WorldActivated))
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
            let invocation = canonical_invocation(invocation)?;
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
            Ok(Reduction::Apply(WorldEffect::DecisionExercised {
                opportunity: current,
                event,
            }))
        }
    }
}

impl WorldState {
    fn genesis(
        world_id: WorldId,
        caller: &CallerId,
        effect: &WorldEffect,
    ) -> Result<Self, KernelError> {
        let WorldEffect::WorldCreated {
            owner,
            title,
            bindings,
        } = effect
        else {
            return Err(KernelError::Invariant(
                "genesis state requires a world-created effect".into(),
            ));
        };
        if caller != &CallerId::Principal(owner.clone()) {
            return Err(KernelError::Invariant(
                "genesis creator does not match owner".into(),
            ));
        }
        validate_principal(owner)?;
        if normalize_title(title)? != *title {
            return Err(KernelError::Invariant(
                "genesis title is not canonical".into(),
            ));
        }
        if bindings.is_empty() {
            return Err(KernelError::NoSubjects);
        }

        let mut handles = BTreeSet::new();
        let mut subjects = BTreeMap::new();
        let mut assignments = BTreeMap::new();
        let mut controller_ids = BTreeSet::new();
        let mut grants = BTreeMap::new();
        let mut scope_kinds = BTreeSet::new();
        for binding in bindings {
            validate_handle(&binding.handle)?;
            if !handles.insert(binding.handle.clone()) {
                return Err(KernelError::DuplicateSubjectHandle);
            }
            if normalize_label(&binding.subject.label)? != binding.subject.label {
                return Err(KernelError::Invariant(
                    "genesis subject label is not canonical".into(),
                ));
            }
            let scope = DecisionScope {
                subject_id: binding.subject_id,
            };
            validate_assignment(&binding.controller)?;
            if !controller_ids.insert(binding.controller.id()) {
                return Err(KernelError::Invariant(
                    "genesis controller ID collision".into(),
                ));
            }
            if binding.affordances.is_empty() {
                return Err(KernelError::NoAffordances);
            }
            if subjects
                .insert(binding.subject_id, binding.subject.clone())
                .is_some()
                || assignments
                    .insert(scope, binding.controller.clone())
                    .is_some()
            {
                return Err(KernelError::Invariant(
                    "genesis subject or scope ID collision".into(),
                ));
            }
            for (affordance_id, grant) in &binding.affordances {
                if grant.scope != scope {
                    return Err(KernelError::Invariant(
                        "genesis affordance is bound to another scope".into(),
                    ));
                }
                if !scope_kinds.insert((scope, grant.kind))
                    || grants.insert(*affordance_id, grant.clone()).is_some()
                {
                    return Err(KernelError::Invariant(
                        "genesis affordance ID or kind collision".into(),
                    ));
                }
            }
        }

        let mut state = Self {
            schema: STATE_SCHEMA.into(),
            world_id,
            revision: 0,
            phase: WorldPhase::Draft,
            owner: owner.clone(),
            title: title.clone(),
            draft_approvals: BTreeSet::new(),
            subjects,
            controller_assignments: assignments,
            affordance_grants: grants,
            events: Vec::new(),
            state_digest: String::new(),
            last_commit_digest: None,
        };
        state.state_digest = state_digest(&state)?;
        Ok(state)
    }
}

fn canonicalize_subjects(
    inputs: &[NewDecisionSubject],
) -> Result<Vec<NewDecisionSubject>, KernelError> {
    if inputs.is_empty() {
        return Err(KernelError::NoSubjects);
    }
    let mut handles = BTreeSet::new();
    let mut result = Vec::with_capacity(inputs.len());
    for input in inputs {
        validate_handle(&input.handle)?;
        if !handles.insert(input.handle.clone()) {
            return Err(KernelError::DuplicateSubjectHandle);
        }
        if input.affordances.is_empty() {
            return Err(KernelError::NoAffordances);
        }
        if let NewController::Human { principal } = &input.controller {
            validate_principal(principal)?;
        }
        result.push(NewDecisionSubject {
            handle: input.handle.clone(),
            label: normalize_label(&input.label)?,
            kind: input.kind,
            controller: input.controller.clone(),
            affordances: input.affordances.clone(),
        });
    }
    Ok(result)
}

fn issue_genesis(
    owner: PrincipalId,
    title: String,
    subjects: Vec<NewDecisionSubject>,
) -> WorldEffect {
    let bindings = subjects
        .into_iter()
        .map(|input| {
            let subject_id = SubjectId::issue();
            let scope = DecisionScope { subject_id };
            let controller_id = ControllerId::issue();
            let controller = match input.controller {
                NewController::Human { principal } => ControllerAssignment::Human {
                    controller_id,
                    principal,
                },
                NewController::NarrativePersona => {
                    ControllerAssignment::NarrativePersona { controller_id }
                }
                NewController::OperationalAgent => {
                    ControllerAssignment::OperationalAgent { controller_id }
                }
            };
            let affordances = input
                .affordances
                .into_iter()
                .map(|kind| (AffordanceId::issue(), AffordanceGrant { scope, kind }))
                .collect();
            GenesisSubjectBinding {
                handle: input.handle,
                subject_id,
                subject: SubjectState {
                    label: input.label,
                    kind: input.kind,
                },
                controller,
                affordances,
            }
        })
        .collect();
    WorldEffect::WorldCreated {
        owner,
        title,
        bindings,
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

fn normalize_label(value: &str) -> Result<String, KernelError> {
    let value = value.trim();
    if value.is_empty() {
        Err(KernelError::EmptySubjectLabel)
    } else {
        Ok(value.to_owned())
    }
}

fn normalize_speech(value: &str) -> Result<String, KernelError> {
    let value = value.trim();
    if value.is_empty() {
        Err(KernelError::EmptySpeech)
    } else {
        Ok(value.to_owned())
    }
}

fn canonical_invocation(value: &DecisionInvocation) -> Result<DecisionInvocation, KernelError> {
    let action = match &value.action {
        DecisionAction::Speak { text } => DecisionAction::Speak {
            text: normalize_speech(text)?,
        },
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

fn validate_handle(value: &DraftSubjectHandle) -> Result<(), KernelError> {
    if value.0.trim().is_empty() || value.0.trim() != value.0 {
        Err(KernelError::EmptySubjectHandle)
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
        WorldEffect::TitleChanged {
            previous_title,
            resulting_title,
        } => {
            if caller != &CallerId::Principal(state.owner.clone())
                || state.phase != WorldPhase::Draft
                || !state.draft_approvals.is_empty()
                || &state.title != previous_title
                || previous_title == resulting_title
                || normalize_title(resulting_title)? != *resulting_title
            {
                return Err(KernelError::Invariant(
                    "title-change effect does not match canonical prior state".into(),
                ));
            }
            state.title = resulting_title.clone();
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
                || canonical_invocation(&event.invocation)? != event.invocation
            {
                return Err(KernelError::Invariant(
                    "decision effect does not match exact opportunity authority".into(),
                ));
            }
            state.events.push(event.clone());
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

    fn owner() -> PrincipalId {
        PrincipalId::new("owner@example.test")
    }

    fn player() -> PrincipalId {
        PrincipalId::new("player@example.test")
    }

    fn auth_principal(principal: PrincipalId) -> AuthenticatedCaller {
        AuthenticatedCaller::fixture(CallerId::Principal(principal))
    }

    fn subject(
        handle: &str,
        label: &str,
        kind: SubjectKind,
        controller: NewController,
    ) -> NewDecisionSubject {
        NewDecisionSubject {
            handle: DraftSubjectHandle::new(handle),
            label: label.into(),
            kind,
            controller,
            affordances: BTreeSet::from([AffordanceKind::Speak]),
        }
    }

    fn creation(id: CommandId, title: &str) -> CreateWorld {
        CreateWorld {
            id,
            owner: owner(),
            title: title.into(),
            subjects: vec![
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
        }
    }

    fn command(
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

    fn submit_owner(
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

    fn activate(kernel: &mut WorldKernel) -> WorldSnapshot {
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
    fn draft_activation_player_and_autonomous_actions_share_one_reducer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, receipt) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "  Delvehold  "),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        assert_eq!(genesis.title, "Delvehold");
        assert_eq!(genesis.phase, WorldPhase::Draft);
        assert_eq!(receipt.subjects.len(), 3);
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
                text: "I open the door.".into()
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
                    CommandBody::SetTitle {
                        title: "Stolen".into()
                    },
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
                    CommandBody::SetTitle {
                        title: "Late".into()
                    },
                ),
                &auth_principal(owner())
            ),
            Err(KernelError::DraftLocked)
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
    fn no_effect_invalid_and_stale_commands_do_not_commit() {
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
            submit_owner(
                &mut kernel,
                &genesis,
                CommandBody::SetTitle {
                    title: " Still ".into()
                }
            ),
            SubmitReceipt::NoEffect(_)
        ));
        assert!(matches!(
            kernel.submit(
                command(
                    &genesis,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::SetTitle { title: " ".into() },
                ),
                &auth_principal(owner())
            ),
            Err(KernelError::EmptyTitle)
        ));
        assert_eq!(kernel.snapshot().unwrap(), genesis);
        assert_eq!(kernel.journal.commit_count(), 1);

        submit_owner(
            &mut kernel,
            &genesis,
            CommandBody::SetTitle {
                title: "Changed".into(),
            },
        );
        let after = kernel.snapshot().unwrap();
        let stale = command(
            &genesis,
            CommandId::new(),
            CallerId::Principal(owner()),
            CommandBody::SetTitle {
                title: "Stale".into(),
            },
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
            CommandBody::SetTitle { title: "B".into() },
        );
        let first_receipt = kernel
            .submit(first.clone(), &auth_principal(owner()))
            .unwrap();
        let second_snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &second_snapshot,
            CommandBody::SetTitle { title: "C".into() },
        );
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
            CommandBody::SetTitle {
                title: "Different".into(),
            },
        );
        assert!(matches!(
            kernel.submit(conflict, &auth_principal(owner())),
            Err(KernelError::CommandIdConflict)
        ));
        assert_eq!(kernel.snapshot().unwrap(), after_second);
    }

    #[test]
    fn invalid_seed_never_allocates_a_world() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let mut invalid = creation(CommandId::new(), "Nope");
        invalid.subjects[1].handle = invalid.subjects[0].handle.clone();
        assert!(matches!(
            WorldKernel::create(&path, invalid, &auth_principal(owner())),
            Err(KernelError::DuplicateSubjectHandle)
        ));
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
            CommandBody::SetTitle {
                title: "Durably changed".into(),
            },
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
        assert_eq!(durable.title, "Durably changed");
        assert!(matches!(
            reopened
                .submit(attempted, &auth_principal(owner()))
                .unwrap(),
            SubmitReceipt::AlreadyApplied(_)
        ));
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
                    CommandBody::SetTitle {
                        title: "Detached".into()
                    },
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
            CommandBody::SetTitle {
                title: "Stolen".into(),
            },
        );
        assert!(matches!(
            kernel.submit(forged, &auth_principal(owner())),
            Err(KernelError::AuthenticationMismatch)
        ));
        assert_eq!(kernel.snapshot().unwrap(), snapshot);
    }
}

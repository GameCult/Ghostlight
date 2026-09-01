//! Sealed replacement world owner under construction.
//!
//! Nothing in this module is a public runtime path yet. The old kernel remains
//! the live crate export until this owner proves persistence and recovery, then
//! the old authority is deleted and this facade takes its name.

mod journal;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
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

        impl $name {
            fn issue() -> Self {
                Self(Uuid::new_v4())
            }

            fn key(self) -> String {
                self.0.to_string()
            }
        }
    };
}

opaque_uuid!(WorldId);
opaque_uuid!(CommandId);

impl CommandId {
    pub(crate) fn new() -> Self {
        Self::issue()
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
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CreateWorld {
    pub(crate) id: CommandId,
    pub(crate) owner: PrincipalId,
    pub(crate) title: String,
}

/// Sealed identity evidence. There is deliberately no production constructor
/// until the Heimdall/app-session owner moves inside the replacement ingress.
pub(crate) struct AuthenticatedPrincipal {
    principal: PrincipalId,
}

impl AuthenticatedPrincipal {
    #[cfg(test)]
    fn fixture(principal: PrincipalId) -> Self {
        Self { principal }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CommandEnvelope {
    pub(crate) id: CommandId,
    pub(crate) world_id: WorldId,
    pub(crate) expected_revision: u64,
    pub(crate) principal: PrincipalId,
    pub(crate) body: CommandBody,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum CommandBody {
    SetTitle { title: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct WorldState {
    schema: String,
    world_id: WorldId,
    revision: u64,
    phase: WorldPhase,
    owner: PrincipalId,
    title: String,
    state_digest: String,
    last_commit_digest: Option<String>,
}

impl WorldState {
    fn genesis(world_id: WorldId, owner: PrincipalId, title: String) -> Result<Self, KernelError> {
        let mut state = Self {
            schema: STATE_SCHEMA.into(),
            world_id,
            revision: 0,
            phase: WorldPhase::Draft,
            owner,
            title,
            state_digest: String::new(),
            last_commit_digest: None,
        };
        state.state_digest = state_digest(&state)?;
        Ok(state)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorldEffect {
    WorldCreated {
        owner: PrincipalId,
        title: String,
    },
    TitleChanged {
        previous_title: String,
        resulting_title: String,
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
    effect: WorldEffect,
    committed_at: DateTime<Utc>,
    digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorldSnapshot {
    pub(crate) world_id: WorldId,
    pub(crate) revision: u64,
    pub(crate) phase: WorldPhase,
    pub(crate) title: String,
    pub(crate) state_digest: String,
    pub(crate) last_commit_digest: Option<String>,
}

impl From<&WorldState> for WorldSnapshot {
    fn from(state: &WorldState) -> Self {
        Self {
            world_id: state.world_id,
            revision: state.revision,
            phase: state.phase,
            title: state.title.clone(),
            state_digest: state.state_digest.clone(),
            last_commit_digest: state.last_commit_digest.clone(),
        }
    }
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

impl From<&WorldCommit> for CreationReceipt {
    fn from(commit: &WorldCommit) -> Self {
        Self {
            command_id: commit.command_id,
            world_id: commit.world_id,
            resulting_state_digest: commit.resulting_state_digest.clone(),
            commit_digest: commit.digest.clone(),
        }
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
    #[error("world owner principal must not be empty")]
    EmptyPrincipal,
    #[error("command targets another world")]
    WorldMismatch,
    #[error("authenticated principal does not match the command")]
    AuthenticationMismatch,
    #[error("principal does not own this world")]
    Unauthorized,
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

struct WorldAggregate {
    state: WorldState,
    journal: journal::WorldJournal,
}

pub(crate) struct WorldKernel {
    aggregate: WorldAggregate,
}

impl WorldKernel {
    pub(crate) fn create(
        path: impl AsRef<Path>,
        input: CreateWorld,
        authenticated: &AuthenticatedPrincipal,
    ) -> Result<(Self, CreationReceipt), KernelError> {
        validate_principal(&input.owner)?;
        if input.owner != authenticated.principal {
            return Err(KernelError::AuthenticationMismatch);
        }
        let title = normalize_title(&input.title)?;
        let creation_digest = digest(&input)?;
        let (journal, state) = match journal::WorldJournal::open_for_creation(
            path.as_ref(),
            input.id,
            &creation_digest,
        )? {
            journal::CreationOpen::Existing { journal, state } => (journal, state),
            journal::CreationOpen::Empty(empty) => {
                let mut state = WorldState::genesis(WorldId::issue(), input.owner, title)?;
                let mut genesis = WorldCommit {
                    schema: COMMIT_SCHEMA.into(),
                    world_id: state.world_id,
                    command_id: input.id,
                    command_digest: creation_digest,
                    previous_revision: None,
                    resulting_revision: 0,
                    previous_state_digest: None,
                    resulting_state_digest: state.state_digest.clone(),
                    previous_commit_digest: None,
                    effect: WorldEffect::WorldCreated {
                        owner: state.owner.clone(),
                        title: state.title.clone(),
                    },
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
            .map(CreationReceipt::from)
            .ok_or_else(|| KernelError::Invariant("world genesis receipt is missing".into()))?;
        Ok((
            Self {
                aggregate: WorldAggregate { state, journal },
            },
            receipt,
        ))
    }

    pub(crate) fn open(
        path: impl AsRef<Path>,
        expected_world_id: WorldId,
    ) -> Result<Self, KernelError> {
        let (journal, state) = journal::WorldJournal::open(path.as_ref(), expected_world_id)?;
        Ok(Self {
            aggregate: WorldAggregate { state, journal },
        })
    }

    pub(crate) fn snapshot(&self) -> Result<WorldSnapshot, KernelError> {
        self.aggregate.journal.ensure_healthy()?;
        Ok(WorldSnapshot::from(&self.aggregate.state))
    }

    pub(crate) fn submit(
        &mut self,
        command: CommandEnvelope,
        authenticated: &AuthenticatedPrincipal,
    ) -> Result<SubmitReceipt, KernelError> {
        self.aggregate.submit(command, authenticated, Utc::now())
    }
}

impl WorldAggregate {
    fn submit(
        &mut self,
        command: CommandEnvelope,
        authenticated: &AuthenticatedPrincipal,
        committed_at: DateTime<Utc>,
    ) -> Result<SubmitReceipt, KernelError> {
        self.journal.ensure_healthy()?;
        if command.world_id != self.state.world_id {
            return Err(KernelError::WorldMismatch);
        }
        if command.principal != authenticated.principal {
            return Err(KernelError::AuthenticationMismatch);
        }
        if command.principal != self.state.owner {
            return Err(KernelError::Unauthorized);
        }
        let command_digest = digest(&command)?;
        if let Some(commit) = self.aggregate_commit(command.id) {
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
        let effect = match &command.body {
            CommandBody::SetTitle { title } => {
                let title = normalize_title(title)?;
                if title == self.state.title {
                    return Ok(SubmitReceipt::NoEffect(WorldSnapshot::from(&self.state)));
                }
                WorldEffect::TitleChanged {
                    previous_title: self.state.title.clone(),
                    resulting_title: title,
                }
            }
        };

        let mut candidate = self.state.clone();
        apply_effect(&mut candidate, &effect)?;
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

    fn aggregate_commit(&self, command_id: CommandId) -> Option<&WorldCommit> {
        self.journal.commit_for(command_id)
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

fn apply_effect(state: &mut WorldState, effect: &WorldEffect) -> Result<(), KernelError> {
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
            if &state.title != previous_title {
                return Err(KernelError::Invariant(
                    "title-change effect does not match prior state".into(),
                ));
            }
            if previous_title == resulting_title
                || normalize_title(resulting_title)? != *resulting_title
            {
                return Err(KernelError::Invariant(
                    "title-change effect is empty or noncanonical".into(),
                ));
            }
            state.title = resulting_title.clone();
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

    fn auth() -> AuthenticatedPrincipal {
        AuthenticatedPrincipal::fixture(owner())
    }

    fn creation(id: CommandId, title: &str) -> CreateWorld {
        CreateWorld {
            id,
            owner: owner(),
            title: title.into(),
        }
    }

    fn command(snapshot: &WorldSnapshot, id: CommandId, title: &str) -> CommandEnvelope {
        CommandEnvelope {
            id,
            world_id: snapshot.world_id,
            expected_revision: snapshot.revision,
            principal: owner(),
            body: CommandBody::SetTitle {
                title: title.into(),
            },
        }
    }

    #[test]
    fn create_submit_restart_and_replay_are_exact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let creation = creation(CommandId::new(), "  First Title  ");
        let (mut kernel, creation_receipt) =
            WorldKernel::create(&path, creation.clone(), &auth()).unwrap();
        let genesis = kernel.snapshot().unwrap();
        assert_eq!(genesis.title, "First Title");
        assert_eq!(creation_receipt.command_id, creation.id);
        assert_eq!(creation_receipt.world_id, genesis.world_id);
        assert_eq!(
            creation_receipt.resulting_state_digest,
            genesis.state_digest
        );
        let command = command(&genesis, CommandId::new(), "Second Title");
        let applied = kernel.submit(command.clone(), &auth()).unwrap();
        let SubmitReceipt::Applied(receipt) = applied else {
            panic!("expected applied receipt")
        };
        let accepted = kernel.snapshot().unwrap();
        assert_eq!(accepted.revision, 1);
        drop(kernel);

        let (mut reopened, retried_creation_receipt) =
            WorldKernel::create(&path, creation, &auth()).unwrap();
        assert_eq!(retried_creation_receipt, creation_receipt);
        assert_eq!(reopened.snapshot().unwrap(), accepted);
        assert_eq!(
            reopened.submit(command, &auth()).unwrap(),
            SubmitReceipt::AlreadyApplied(receipt)
        );
        assert_eq!(reopened.snapshot().unwrap(), accepted);
        drop(reopened);

        let reopened = WorldKernel::open(&path, genesis.world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), accepted);
    }

    #[test]
    fn no_effect_invalid_and_stale_commands_do_not_commit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) =
            WorldKernel::create(&path, creation(CommandId::new(), "Still"), &auth()).unwrap();
        let genesis = kernel.snapshot().unwrap();
        let no_effect = command(&genesis, CommandId::new(), " Still ");
        assert!(matches!(
            kernel.submit(no_effect, &auth()).unwrap(),
            SubmitReceipt::NoEffect(_)
        ));
        let invalid = command(&genesis, CommandId::new(), "   ");
        assert!(matches!(
            kernel.submit(invalid, &auth()),
            Err(KernelError::EmptyTitle)
        ));
        assert_eq!(kernel.snapshot().unwrap(), genesis);
        assert_eq!(kernel.aggregate.journal.commit_count(), 1);
        drop(kernel);

        let mut kernel = WorldKernel::open(&path, genesis.world_id).unwrap();
        assert_eq!(kernel.snapshot().unwrap(), genesis);
        assert_eq!(kernel.aggregate.journal.commit_count(), 1);

        let accepted = command(&genesis, CommandId::new(), "Changed");
        kernel.submit(accepted, &auth()).unwrap();
        let after = kernel.snapshot().unwrap();
        let stale = command(&genesis, CommandId::new(), "Stale");
        assert!(matches!(
            kernel.submit(stale, &auth()),
            Err(KernelError::RevisionMismatch { .. })
        ));
        assert_eq!(kernel.snapshot().unwrap(), after);
        assert_eq!(kernel.aggregate.journal.commit_count(), 2);
        drop(kernel);

        let kernel = WorldKernel::open(&path, genesis.world_id).unwrap();
        assert_eq!(kernel.snapshot().unwrap(), after);
        assert_eq!(kernel.aggregate.journal.commit_count(), 2);
    }

    #[test]
    fn durable_idempotency_precedes_revision_and_detects_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) =
            WorldKernel::create(&path, creation(CommandId::new(), "A"), &auth()).unwrap();
        let genesis = kernel.snapshot().unwrap();
        let id = CommandId::new();
        let first = command(&genesis, id, "B");
        let first_receipt = kernel.submit(first.clone(), &auth()).unwrap();
        let second = command(&kernel.snapshot().unwrap(), CommandId::new(), "C");
        kernel.submit(second, &auth()).unwrap();
        let after_second = kernel.snapshot().unwrap();
        drop(kernel);
        let mut kernel = WorldKernel::open(&path, genesis.world_id).unwrap();
        assert_eq!(kernel.snapshot().unwrap(), after_second);
        assert_eq!(
            kernel.submit(first, &auth()).unwrap(),
            match first_receipt {
                SubmitReceipt::Applied(value) => SubmitReceipt::AlreadyApplied(value),
                _ => unreachable!(),
            }
        );
        let conflict = command(&genesis, id, "Different");
        assert!(matches!(
            kernel.submit(conflict, &auth()),
            Err(KernelError::CommandIdConflict)
        ));
        assert_eq!(kernel.snapshot().unwrap().revision, 2);
    }

    #[test]
    fn a_second_live_owner_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let creation = creation(CommandId::new(), "Held");
        let (kernel, _) = WorldKernel::create(&path, creation.clone(), &auth()).unwrap();
        let world_id = kernel.snapshot().unwrap().world_id;
        assert!(matches!(
            WorldKernel::open(&path, world_id),
            Err(KernelError::Store(_))
        ));
        assert!(matches!(
            WorldKernel::create(&path, creation.clone(), &auth()),
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
    fn creation_id_is_exact_and_conflicts_do_not_rewrite_genesis() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let creation_id = CommandId::new();
        let (kernel, receipt) =
            WorldKernel::create(&path, creation(creation_id, "Original"), &auth()).unwrap();
        let genesis = kernel.snapshot().unwrap();
        drop(kernel);

        assert!(matches!(
            WorldKernel::create(&path, creation(creation_id, "Different"), &auth()),
            Err(KernelError::CreationConflict)
        ));
        let (kernel, retried) =
            WorldKernel::create(&path, creation(creation_id, "Original"), &auth()).unwrap();
        assert_eq!(retried, receipt);
        assert_eq!(kernel.snapshot().unwrap(), genesis);
        drop(kernel);
        assert!(matches!(
            WorldKernel::create(
                &path,
                creation(CommandId::new(), "Unrelated creation"),
                &auth()
            ),
            Err(KernelError::CreationTargetOccupied)
        ));
    }

    #[test]
    fn lost_post_commit_ack_poisons_until_reopen_and_exact_retry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) =
            WorldKernel::create(&path, creation(CommandId::new(), "Certain"), &auth()).unwrap();
        let genesis = kernel.snapshot().unwrap();
        let uncertain = CommandId::new();
        let attempted = command(&genesis, uncertain, "Durably changed");
        kernel
            .aggregate
            .journal
            .fail_after_durable_commit_for_test();

        assert!(matches!(
            kernel.submit(attempted.clone(), &auth()),
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
            reopened.submit(attempted, &auth()).unwrap(),
            SubmitReceipt::AlreadyApplied(_)
        ));
        assert_eq!(reopened.snapshot().unwrap(), durable);
    }

    #[test]
    fn replacing_the_store_path_revokes_the_live_owner() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let displaced = dir.path().join("displaced.cc");
        let (mut kernel, _) =
            WorldKernel::create(&path, creation(CommandId::new(), "Pinned"), &auth()).unwrap();
        let snapshot = kernel.snapshot().unwrap();

        std::fs::rename(&path, &displaced).unwrap();
        std::fs::File::create(&path).unwrap();
        assert!(matches!(kernel.snapshot(), Err(KernelError::OwnershipLost)));
        assert!(matches!(
            kernel.submit(command(&snapshot, CommandId::new(), "Detached"), &auth()),
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
        let (mut kernel, _) =
            WorldKernel::create(&path, creation(CommandId::new(), "Owned"), &auth()).unwrap();
        let snapshot = kernel.snapshot().unwrap();
        let mut forged = command(&snapshot, CommandId::new(), "Stolen");
        forged.principal = PrincipalId::new("attacker@example.test");
        assert!(matches!(
            kernel.submit(forged, &auth()),
            Err(KernelError::AuthenticationMismatch)
        ));
        assert_eq!(kernel.snapshot().unwrap(), snapshot);
    }
}

use super::{
    COMMIT_SCHEMA, CommandId, KernelError, STATE_SCHEMA, WorldCommit, WorldEffect, WorldId,
    WorldState, apply_effect, commit_digest, state_digest, validate_principal,
};
use chrono::Utc;
use cultcache_legacy::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use serde::{Serialize, de::DeserializeOwned};
use std::{cell::Cell, collections::BTreeMap, path::Path};
use thiserror::Error;

const STATE_ROW: &str = "world_state.foundation.v0";
const COMMIT_ROW: &str = "world_commit.foundation.v0";

#[derive(Debug, Error)]
pub(super) enum JournalError {
    #[error("world store is not empty")]
    NotEmpty,
    #[error("opened store contains another world")]
    WorldMismatch,
    #[error("world creation ID was reused with different content")]
    CreationConflict,
    #[error("world ownership is uncertain after command {command_id:?}; drop and reopen")]
    RecoveryRequired { command_id: CommandId },
    #[error("world store path no longer names the owned database")]
    OwnershipLost,
    #[error("world store operation failed: {0}")]
    Store(String),
    #[error("world journal is corrupt: {0}")]
    Corrupt(String),
}

pub(super) enum CreationOpen {
    Empty(EmptyWorldJournal),
    Existing {
        journal: WorldJournal,
        state: WorldState,
    },
}

pub(super) struct EmptyWorldJournal {
    store: OwnedRedbMessagePackBackingStore,
}

#[derive(Clone, Copy)]
enum JournalHealth {
    Healthy,
    RecoveryRequired(CommandId),
    OwnershipLost,
}

pub(super) struct WorldJournal {
    store: OwnedRedbMessagePackBackingStore,
    state_row: CultCacheEnvelope,
    commits: BTreeMap<CommandId, WorldCommit>,
    health: Cell<JournalHealth>,
    #[cfg(test)]
    fail_after_durable_commit: bool,
}

impl WorldJournal {
    pub(super) fn open_for_creation(
        path: &Path,
        creation_id: CommandId,
        creation_digest: &str,
    ) -> Result<CreationOpen, JournalError> {
        let store = OwnedRedbMessagePackBackingStore::new(path)
            .map_err(|error| JournalError::Store(error.to_string()))?;
        store
            .validate_path_identity()
            .map_err(|_| JournalError::OwnershipLost)?;
        let rows = store
            .pull_all()
            .map_err(|error| JournalError::Store(error.to_string()))?;
        store
            .validate_path_identity()
            .map_err(|_| JournalError::OwnershipLost)?;
        if rows.is_empty() {
            return Ok(CreationOpen::Empty(EmptyWorldJournal { store }));
        }
        let (state_row, state, commits) = recover(rows, None)?;
        let Some(genesis) = commits.get(&creation_id) else {
            return Err(JournalError::NotEmpty);
        };
        if genesis.command_digest != creation_digest
            || !matches!(&genesis.effect, WorldEffect::WorldCreated { .. })
        {
            return Err(JournalError::CreationConflict);
        }
        Ok(CreationOpen::Existing {
            journal: Self {
                store,
                state_row,
                commits,
                health: Cell::new(JournalHealth::Healthy),
                #[cfg(test)]
                fail_after_durable_commit: false,
            },
            state,
        })
    }

    pub(super) fn open(
        path: &Path,
        expected_world_id: WorldId,
    ) -> Result<(Self, WorldState), JournalError> {
        let store = OwnedRedbMessagePackBackingStore::new(path)
            .map_err(|error| JournalError::Store(error.to_string()))?;
        store
            .validate_path_identity()
            .map_err(|_| JournalError::OwnershipLost)?;
        let rows = store
            .pull_all()
            .map_err(|error| JournalError::Store(error.to_string()))?;
        store
            .validate_path_identity()
            .map_err(|_| JournalError::OwnershipLost)?;
        let (state_row, state, commits) = recover(rows, Some(expected_world_id))?;
        Ok((
            Self {
                store,
                state_row,
                commits,
                health: Cell::new(JournalHealth::Healthy),
                #[cfg(test)]
                fail_after_durable_commit: false,
            },
            state,
        ))
    }

    pub(super) fn ensure_healthy(&self) -> Result<(), JournalError> {
        match self.health.get() {
            JournalHealth::RecoveryRequired(command_id) => {
                return Err(JournalError::RecoveryRequired { command_id });
            }
            JournalHealth::OwnershipLost => return Err(JournalError::OwnershipLost),
            JournalHealth::Healthy => {}
        }
        if self.store.validate_path_identity().is_err() {
            self.health.set(JournalHealth::OwnershipLost);
            return Err(JournalError::OwnershipLost);
        }
        Ok(())
    }

    pub(super) fn commit(
        &mut self,
        next: &WorldState,
        commit: &WorldCommit,
    ) -> Result<(), JournalError> {
        self.ensure_healthy()?;
        if self.commits.contains_key(&commit.command_id) {
            return Err(JournalError::Corrupt(
                "single-owner append attempted to duplicate a committed command".into(),
            ));
        }
        let current: WorldState = decode(&self.state_row)?;
        verify_append(&current, next, commit)?;
        let next_row = envelope(STATE_ROW, STATE_SCHEMA, next.world_id.key(), next)?;
        let commit_row = envelope(COMMIT_ROW, COMMIT_SCHEMA, commit.command_id.key(), commit)?;
        self.health
            .set(JournalHealth::RecoveryRequired(commit.command_id));
        let swapped = self.store.compare_and_swap_batch(
            std::slice::from_ref(&self.state_row),
            vec![next_row.clone(), commit_row],
        );
        match swapped {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                return Err(JournalError::RecoveryRequired {
                    command_id: commit.command_id,
                });
            }
        }
        #[cfg(test)]
        if self.fail_after_durable_commit {
            return Err(JournalError::RecoveryRequired {
                command_id: commit.command_id,
            });
        }
        if self.store.validate_path_identity().is_err() {
            return Err(JournalError::RecoveryRequired {
                command_id: commit.command_id,
            });
        }
        self.state_row = next_row;
        self.commits.insert(commit.command_id, commit.clone());
        self.health.set(JournalHealth::Healthy);
        Ok(())
    }

    pub(super) fn commit_for(&self, command_id: CommandId) -> Option<&WorldCommit> {
        self.commits.get(&command_id)
    }

    #[cfg(test)]
    pub(super) fn commit_count(&self) -> usize {
        self.commits.len()
    }

    #[cfg(test)]
    pub(super) fn fail_after_durable_commit_for_test(&mut self) {
        self.fail_after_durable_commit = true;
    }
}

impl EmptyWorldJournal {
    pub(super) fn initialize(
        self,
        state: &WorldState,
        genesis: &WorldCommit,
    ) -> Result<WorldJournal, JournalError> {
        self.store
            .validate_path_identity()
            .map_err(|_| JournalError::OwnershipLost)?;
        let mut commits = BTreeMap::new();
        commits.insert(genesis.command_id, genesis.clone());
        verify_history(state, &commits)?;
        let state_row = envelope(STATE_ROW, STATE_SCHEMA, state.world_id.key(), state)?;
        let commit_row = envelope(COMMIT_ROW, COMMIT_SCHEMA, genesis.command_id.key(), genesis)?;
        let inserted = self
            .store
            .append_if_snapshot_unchanged(&[], vec![state_row.clone(), commit_row])
            .map_err(|error| JournalError::Store(error.to_string()))?;
        if !inserted {
            return Err(JournalError::NotEmpty);
        }
        if self.store.validate_path_identity().is_err() {
            return Err(JournalError::RecoveryRequired {
                command_id: genesis.command_id,
            });
        }
        Ok(WorldJournal {
            store: self.store,
            state_row,
            commits,
            health: Cell::new(JournalHealth::Healthy),
            #[cfg(test)]
            fail_after_durable_commit: false,
        })
    }
}

fn recover(
    rows: Vec<CultCacheEnvelope>,
    expected_world_id: Option<WorldId>,
) -> Result<
    (
        CultCacheEnvelope,
        WorldState,
        BTreeMap<CommandId, WorldCommit>,
    ),
    JournalError,
> {
    let mut state_rows = Vec::new();
    let mut commits = BTreeMap::new();
    for row in rows {
        match row.r#type.as_str() {
            STATE_ROW => {
                require_schema(&row, STATE_SCHEMA)?;
                state_rows.push(row);
            }
            COMMIT_ROW => {
                require_schema(&row, COMMIT_SCHEMA)?;
                let commit: WorldCommit = decode(&row)?;
                if row.key != commit.command_id.key() {
                    return Err(JournalError::Corrupt(
                        "commit row key does not match command ID".into(),
                    ));
                }
                if commits.insert(commit.command_id, commit).is_some() {
                    return Err(JournalError::Corrupt("duplicate command commit".into()));
                }
            }
            other => {
                return Err(JournalError::Corrupt(format!(
                    "unadmitted row type {other}"
                )));
            }
        }
    }
    if state_rows.len() != 1 {
        return Err(JournalError::Corrupt(format!(
            "expected one state row, found {}",
            state_rows.len()
        )));
    }
    let state_row = state_rows.pop().expect("length checked");
    let state: WorldState = decode(&state_row)?;
    if state_row.key != state.world_id.key() {
        return Err(JournalError::Corrupt(
            "state row key does not match its world ID".into(),
        ));
    }
    if expected_world_id.is_some_and(|expected| state.world_id != expected) {
        return Err(JournalError::WorldMismatch);
    }
    verify_history(&state, &commits)?;
    Ok((state_row, state, commits))
}

fn verify_append(
    current: &WorldState,
    next: &WorldState,
    commit: &WorldCommit,
) -> Result<(), JournalError> {
    verify_state_shape(current)?;
    verify_state_shape(next)?;
    if current.world_id != next.world_id
        || commit.world_id != current.world_id
        || commit.schema != COMMIT_SCHEMA
        || commit.previous_revision != Some(current.revision)
        || commit.resulting_revision != next.revision
        || next.revision != current.revision.saturating_add(1)
        || commit.previous_state_digest.as_deref() != Some(current.state_digest.as_str())
        || commit.resulting_state_digest != next.state_digest
        || commit.previous_commit_digest != current.last_commit_digest
        || next.last_commit_digest.as_deref() != Some(commit.digest.as_str())
    {
        return Err(JournalError::Corrupt(
            "commit does not bind the exact state transition".into(),
        ));
    }
    if state_digest(current).map_err(kernel_error)? != current.state_digest
        || state_digest(next).map_err(kernel_error)? != next.state_digest
        || commit_digest(commit).map_err(kernel_error)? != commit.digest
    {
        return Err(JournalError::Corrupt("digest mismatch".into()));
    }
    let mut replay = current.clone();
    apply_effect(&mut replay, &commit.effect).map_err(kernel_error)?;
    replay.revision = next.revision;
    replay.state_digest = state_digest(&replay).map_err(kernel_error)?;
    replay.last_commit_digest = Some(commit.digest.clone());
    if &replay != next {
        return Err(JournalError::Corrupt(
            "commit effect does not reproduce next state".into(),
        ));
    }
    Ok(())
}

fn verify_history(
    state: &WorldState,
    commits: &BTreeMap<CommandId, WorldCommit>,
) -> Result<(), JournalError> {
    verify_state_shape(state)?;
    let expected_commits = usize::try_from(state.revision)
        .map_err(|_| JournalError::Corrupt("revision does not fit this runtime".into()))?;
    let expected_commits = expected_commits
        .checked_add(1)
        .ok_or_else(|| JournalError::Corrupt("commit count overflow".into()))?;
    if commits.len() != expected_commits {
        return Err(JournalError::Corrupt(
            "commit count does not equal genesis plus head revision".into(),
        ));
    }
    let mut ordered: Vec<_> = commits.values().collect();
    ordered.sort_by_key(|commit| commit.resulting_revision);
    let genesis = ordered
        .first()
        .ok_or_else(|| JournalError::Corrupt("world has no genesis commit".into()))?;
    let WorldEffect::WorldCreated { owner, title } = &genesis.effect else {
        return Err(JournalError::Corrupt(
            "first commit is not immutable world genesis".into(),
        ));
    };
    if genesis.schema != COMMIT_SCHEMA
        || genesis.world_id != state.world_id
        || genesis.previous_revision.is_some()
        || genesis.resulting_revision != 0
        || genesis.previous_state_digest.is_some()
        || genesis.previous_commit_digest.is_some()
        || commit_digest(genesis).map_err(kernel_error)? != genesis.digest
    {
        return Err(JournalError::Corrupt(
            "genesis commit is not canonical or verifiable".into(),
        ));
    }
    let mut replay =
        WorldState::genesis(state.world_id, owner.clone(), title.clone()).map_err(kernel_error)?;
    verify_state_shape(&replay)?;
    if replay.state_digest != genesis.resulting_state_digest {
        return Err(JournalError::Corrupt(
            "genesis resulting state digest is invalid".into(),
        ));
    }
    replay.last_commit_digest = Some(genesis.digest.clone());

    for commit in ordered.into_iter().skip(1) {
        if commit.schema != COMMIT_SCHEMA
            || commit.world_id != state.world_id
            || commit.previous_revision != Some(replay.revision)
            || commit.resulting_revision != replay.revision.saturating_add(1)
            || commit.previous_state_digest.as_deref() != Some(replay.state_digest.as_str())
            || commit.previous_commit_digest != replay.last_commit_digest
            || commit_digest(commit).map_err(kernel_error)? != commit.digest
        {
            return Err(JournalError::Corrupt(
                "commit chain is not contiguous or verifiable".into(),
            ));
        }
        apply_effect(&mut replay, &commit.effect).map_err(kernel_error)?;
        replay.revision = commit.resulting_revision;
        replay.state_digest = state_digest(&replay).map_err(kernel_error)?;
        if replay.state_digest != commit.resulting_state_digest {
            return Err(JournalError::Corrupt(
                "commit resulting state digest is invalid".into(),
            ));
        }
        replay.last_commit_digest = Some(commit.digest.clone());
    }
    if &replay != state {
        return Err(JournalError::Corrupt(
            "head state does not equal replayed history".into(),
        ));
    }
    Ok(())
}

fn verify_state_shape(state: &WorldState) -> Result<(), JournalError> {
    if state.schema != STATE_SCHEMA {
        return Err(JournalError::Corrupt("invalid state schema".into()));
    }
    if state.title.trim().is_empty() || state.title.trim() != state.title {
        return Err(JournalError::Corrupt(
            "world title is empty or noncanonical".into(),
        ));
    }
    validate_principal(&state.owner).map_err(kernel_error)?;
    Ok(())
}

fn require_schema(row: &CultCacheEnvelope, schema: &str) -> Result<(), JournalError> {
    if row.schema_id.as_deref() != Some(schema) {
        Err(JournalError::Corrupt(format!(
            "row {}/{} has the wrong schema",
            row.r#type, row.key
        )))
    } else {
        Ok(())
    }
}

fn envelope<T: Serialize>(
    row_type: &str,
    schema: &str,
    key: String,
    value: &T,
) -> Result<CultCacheEnvelope, JournalError> {
    Ok(CultCacheEnvelope {
        key,
        r#type: row_type.into(),
        payload: rmp_serde::to_vec_named(value)
            .map_err(|error| JournalError::Store(error.to_string()))?,
        stored_at: Utc::now().to_rfc3339(),
        schema_id: Some(schema.into()),
    })
}

fn decode<T: DeserializeOwned>(row: &CultCacheEnvelope) -> Result<T, JournalError> {
    rmp_serde::from_slice(&row.payload).map_err(|error| {
        JournalError::Corrupt(format!(
            "could not decode row {}/{}: {error}",
            row.r#type, row.key
        ))
    })
}

fn kernel_error(error: KernelError) -> JournalError {
    JournalError::Corrupt(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::PrincipalId;

    #[test]
    fn a_later_effect_cannot_repair_noncanonical_genesis() {
        let world_id = WorldId::issue();
        let owner = PrincipalId::new("owner@example.test");
        let creation_id = CommandId::new();
        let mut genesis_state =
            WorldState::genesis(world_id, owner.clone(), " Bad Genesis ".into()).unwrap();
        let mut genesis = WorldCommit {
            schema: COMMIT_SCHEMA.into(),
            world_id,
            command_id: creation_id,
            command_digest: "sha256:create".into(),
            previous_revision: None,
            resulting_revision: 0,
            previous_state_digest: None,
            resulting_state_digest: genesis_state.state_digest.clone(),
            previous_commit_digest: None,
            effect: WorldEffect::WorldCreated {
                owner,
                title: genesis_state.title.clone(),
            },
            committed_at: Utc::now(),
            digest: String::new(),
        };
        genesis.digest = commit_digest(&genesis).unwrap();
        genesis_state.last_commit_digest = Some(genesis.digest.clone());

        let effect = WorldEffect::TitleChanged {
            previous_title: genesis_state.title.clone(),
            resulting_title: "Canonical".into(),
        };
        let mut head = genesis_state.clone();
        apply_effect(&mut head, &effect).unwrap();
        head.revision = 1;
        head.state_digest = state_digest(&head).unwrap();
        let mut correction = WorldCommit {
            schema: COMMIT_SCHEMA.into(),
            world_id,
            command_id: CommandId::new(),
            command_digest: "sha256:correct".into(),
            previous_revision: Some(0),
            resulting_revision: 1,
            previous_state_digest: Some(genesis_state.state_digest.clone()),
            resulting_state_digest: head.state_digest.clone(),
            previous_commit_digest: Some(genesis.digest.clone()),
            effect,
            committed_at: Utc::now(),
            digest: String::new(),
        };
        correction.digest = commit_digest(&correction).unwrap();
        head.last_commit_digest = Some(correction.digest.clone());

        let mut commits = BTreeMap::new();
        commits.insert(genesis.command_id, genesis);
        commits.insert(correction.command_id, correction);
        assert!(matches!(
            verify_history(&head, &commits),
            Err(JournalError::Corrupt(_))
        ));
    }
}

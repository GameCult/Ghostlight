use super::{
    COMMIT_SCHEMA, CommandId, CommittedCommand, KernelError, STATE_SCHEMA, WorldCommit,
    WorldEffect, WorldId, WorldState, apply_effect, commit_digest, reduce, state_digest,
};
use chrono::Utc;
use cultcache_rs::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use thiserror::Error;

const STATE_ROW: &str = "world_state.foundation.v1";
const COMMIT_ROW: &str = "world_commit.foundation.v2";

#[derive(Debug, Error)]
pub(super) enum JournalError {
    #[error("opened store contains another world")]
    WorldMismatch,
    #[error("world ownership is uncertain after command {command_id:?}; drop and reopen")]
    RecoveryRequired { command_id: CommandId },
    #[error("world store path no longer names the owned database")]
    OwnershipLost,
    #[error("world store operation failed: {0}")]
    Store(String),
    #[error("world journal is corrupt: {0}")]
    Corrupt(String),
}

pub(super) enum JournalOpen {
    Empty(EmptyWorldJournal),
    Live {
        journal: WorldJournal,
        state: WorldState,
    },
}

pub(super) struct EmptyWorldJournal {
    store: OwnedRedbMessagePackBackingStore,
    #[cfg(test)]
    fail_after_durable_initialize: bool,
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
    pub(super) fn open_owner(path: &Path) -> Result<JournalOpen, JournalError> {
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
            return Ok(JournalOpen::Empty(EmptyWorldJournal {
                store,
                #[cfg(test)]
                fail_after_durable_initialize: false,
            }));
        }
        let (state_row, state, commits) = recover(rows, None)?;
        Ok(JournalOpen::Live {
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
        let command_id = commit.command.id();
        if self.commits.contains_key(&command_id) {
            return Err(JournalError::Corrupt(
                "single-owner append attempted to duplicate a committed command".into(),
            ));
        }
        let current: WorldState = decode(&self.state_row)?;
        verify_append(&current, next, commit)?;
        let next_row = envelope(STATE_ROW, STATE_SCHEMA, next.world_id.key(), next)?;
        let commit_row = envelope(COMMIT_ROW, COMMIT_SCHEMA, command_id.key(), commit)?;
        self.health.set(JournalHealth::RecoveryRequired(command_id));
        let swapped = self.store.compare_and_swap_batch(
            std::slice::from_ref(&self.state_row),
            vec![next_row.clone(), commit_row],
        );
        match swapped {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                return Err(JournalError::RecoveryRequired { command_id });
            }
        }
        #[cfg(test)]
        if self.fail_after_durable_commit {
            return Err(JournalError::RecoveryRequired { command_id });
        }
        if self.store.validate_path_identity().is_err() {
            return Err(JournalError::RecoveryRequired { command_id });
        }
        self.state_row = next_row;
        self.commits.insert(command_id, commit.clone());
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
    #[cfg(test)]
    pub(super) fn fail_after_durable_initialize_for_test(&mut self) {
        self.fail_after_durable_initialize = true;
    }

    pub(super) fn initialize(
        self,
        state: &WorldState,
        genesis: &WorldCommit,
    ) -> Result<WorldJournal, JournalError> {
        self.store
            .validate_path_identity()
            .map_err(|_| JournalError::OwnershipLost)?;
        let mut commits = BTreeMap::new();
        let command_id = genesis.command.id();
        commits.insert(command_id, genesis.clone());
        verify_history(state, &commits)?;
        let state_row = envelope(STATE_ROW, STATE_SCHEMA, state.world_id.key(), state)?;
        let commit_row = envelope(COMMIT_ROW, COMMIT_SCHEMA, command_id.key(), genesis)?;
        let inserted = self
            .store
            .append_if_snapshot_unchanged(&[], vec![state_row.clone(), commit_row]);
        if !matches!(inserted, Ok(true)) {
            return Err(JournalError::RecoveryRequired { command_id });
        }
        #[cfg(test)]
        if self.fail_after_durable_initialize {
            return Err(JournalError::RecoveryRequired { command_id });
        }
        if self.store.validate_path_identity().is_err() {
            return Err(JournalError::RecoveryRequired { command_id });
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
                let command_id = commit.command.id();
                if row.key != command_id.key() {
                    return Err(JournalError::Corrupt(
                        "commit row key does not match command ID".into(),
                    ));
                }
                if commits.insert(command_id, commit).is_some() {
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
    apply_committed_command(&mut replay, commit)?;
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
    if commits
        .iter()
        .any(|(command_id, commit)| *command_id != commit.command.id())
    {
        return Err(JournalError::Corrupt(
            "commit index does not derive from the persisted command".into(),
        ));
    }
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
    let CommittedCommand::CreateWorld(genesis_command) = &genesis.command else {
        return Err(JournalError::Corrupt(
            "first commit does not contain the immutable creation command".into(),
        ));
    };
    let WorldEffect::WorldCreated { .. } = &genesis.effect else {
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
    let mut replay = WorldState::genesis(state.world_id, genesis_command, &genesis.effect)
        .map_err(kernel_error)?;
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
        apply_committed_command(&mut replay, commit)?;
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
    super::validate_principal(&state.owner).map_err(kernel_error)?;
    if state.subjects.is_empty()
        || state.controller_assignments.len() != state.subjects.len()
        || state.affordance_grants.is_empty()
    {
        return Err(JournalError::Corrupt(
            "world ontology is empty or has split subject/controller ownership".into(),
        ));
    }
    for entity in state.entities.values() {
        if !super::patch::is_canonical_text(&entity.label) {
            return Err(JournalError::Corrupt(
                "entity label is empty or noncanonical".into(),
            ));
        }
    }
    if state.edges.values().any(|edge| {
        !state.entities.contains_key(&edge.from) || !state.entities.contains_key(&edge.to)
    }) {
        return Err(JournalError::Corrupt(
            "edge endpoint references an unknown entity".into(),
        ));
    }
    let mut controller_ids = BTreeSet::new();
    for (subject_id, subject) in &state.subjects {
        if !super::patch::is_canonical_text(&subject.label) {
            return Err(JournalError::Corrupt(
                "subject label is empty or noncanonical".into(),
            ));
        }
        if let Some(entity_id) = subject.authority_scope
            && state
                .entities
                .get(&entity_id)
                .is_none_or(|entity| entity.kind != super::EntityKind::Place)
        {
            return Err(JournalError::Corrupt(
                "subject authority scope does not name a canonical place".into(),
            ));
        }
        let scope = super::DecisionScope {
            subject_id: *subject_id,
        };
        let assignment = state
            .controller_assignments
            .get(&scope)
            .ok_or_else(|| JournalError::Corrupt("decision subject has no controller".into()))?;
        super::validate_assignment(assignment).map_err(kernel_error)?;
        if !controller_ids.insert(assignment.id()) {
            return Err(JournalError::Corrupt(
                "controller ID owns more than one canonical scope".into(),
            ));
        }
    }
    if state
        .controller_assignments
        .keys()
        .any(|scope| !state.subjects.contains_key(&scope.subject_id))
    {
        return Err(JournalError::Corrupt(
            "controller assignment references an unknown subject".into(),
        ));
    }
    let mut scope_kinds = BTreeSet::new();
    let mut scopes_with_grants = BTreeSet::new();
    for grant in state.affordance_grants.values() {
        if !state.controller_assignments.contains_key(&grant.scope)
            || !scope_kinds.insert((grant.scope, grant.kind))
        {
            return Err(JournalError::Corrupt(
                "affordance grant is duplicated or unscoped".into(),
            ));
        }
        scopes_with_grants.insert(grant.scope);
    }
    if state
        .controller_assignments
        .keys()
        .any(|scope| !scopes_with_grants.contains(scope))
    {
        return Err(JournalError::Corrupt(
            "decision scope has no affordance grant".into(),
        ));
    }
    let required = super::required_approvers(state);
    if !state.draft_approvals.is_subset(&required)
        || (state.phase != super::WorldPhase::Draft && !required.is_subset(&state.draft_approvals))
    {
        return Err(JournalError::Corrupt(
            "draft approvals do not match canonical controller ownership".into(),
        ));
    }
    let mut event_ids = BTreeSet::new();
    let mut previous_event_revision = 0;
    for event in &state.events {
        let assignment = state
            .controller_assignments
            .get(&event.scope)
            .ok_or_else(|| JournalError::Corrupt("event references an unknown scope".into()))?;
        let grant = state
            .affordance_grants
            .get(&event.invocation.affordance_id)
            .ok_or_else(|| JournalError::Corrupt("event uses an unknown affordance".into()))?;
        if !event_ids.insert(event.id)
            || event.revision == 0
            || event.revision > state.revision
            || event.revision <= previous_event_revision
            || event.controller_id != assignment.id()
            || grant.scope != event.scope
            || grant.kind != event.invocation.action.kind()
            || super::validated_invocation(&event.invocation).map_err(kernel_error)?
                != event.invocation
        {
            return Err(JournalError::Corrupt(
                "decision event is noncanonical or violates controller scope".into(),
            ));
        }
        previous_event_revision = event.revision;
    }
    Ok(())
}

fn apply_committed_command(
    state: &mut WorldState,
    commit: &WorldCommit,
) -> Result<(), JournalError> {
    let CommittedCommand::WorldCommand(command) = &commit.command else {
        return Err(JournalError::Corrupt(
            "creation command appears after world genesis".into(),
        ));
    };
    if command.world_id != state.world_id || command.expected_revision != state.revision {
        return Err(JournalError::Corrupt(
            "committed command does not target the exact replay state".into(),
        ));
    }
    let expected_effect = reduce(state, command).map_err(kernel_error)?;
    if expected_effect != commit.effect {
        return Err(JournalError::Corrupt(
            "committed effect is not the deterministic reduction of its command".into(),
        ));
    }
    apply_effect(state, &commit.command.caller(), &expected_effect).map_err(kernel_error)
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

fn decode<T: DeserializeOwned + Serialize>(row: &CultCacheEnvelope) -> Result<T, JournalError> {
    let value: T = rmp_serde::from_slice(&row.payload).map_err(|error| {
        JournalError::Corrupt(format!(
            "could not decode row {}/{}: {error}",
            row.r#type, row.key
        ))
    })?;
    let canonical = rmp_serde::to_vec_named(&value)
        .map_err(|error| JournalError::Corrupt(error.to_string()))?;
    if canonical != row.payload {
        return Err(JournalError::Corrupt(format!(
            "row {}/{} is not canonical",
            row.r#type, row.key
        )));
    }
    Ok(value)
}

fn kernel_error(error: KernelError) -> JournalError {
    JournalError::Corrupt(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{
        AffordanceKind, AuthenticatedCaller, CallerId, CommandBody, CommandEnvelope, CreateWorld,
        Declaration, DraftHandle, EntityDeclaration, EntityId, EntityKind, NewController,
        PrincipalId, Ref, SubjectDeclaration, SubjectKind, WorldKernel, WorldPatch,
    };

    #[derive(serde::Serialize, serde::Deserialize)]
    struct CanonicalFixture {
        value: u8,
    }

    fn owner_patch(subject: SubjectDeclaration) -> WorldPatch {
        WorldPatch {
            declarations: vec![Declaration::Subject(subject)],
            operations: Vec::new(),
            evidence: Vec::new(),
        }
    }

    #[test]
    fn decode_rejects_an_alternate_messagepack_shape() {
        let row = CultCacheEnvelope {
            key: "fixture".into(),
            r#type: "fixture".into(),
            payload: rmp_serde::to_vec(&CanonicalFixture { value: 7 }).unwrap(),
            stored_at: Utc::now().to_rfc3339(),
            schema_id: Some("fixture.v0".into()),
        };
        assert!(decode::<CanonicalFixture>(&row).is_err());
    }

    #[test]
    fn replay_rejects_a_forged_command_or_effect_even_with_valid_hashes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let owner = PrincipalId::new("owner@example.test");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let creation = CreateWorld {
            id: CommandId::new(),
            owner: owner.clone(),
            title: "Before".into(),
            patch: owner_patch(SubjectDeclaration {
                handle: DraftHandle::new("owner"),
                label: "Owner".into(),
                kind: SubjectKind::Person,
                controller: NewController::Human {
                    principal: owner.clone(),
                },
                affordances: BTreeSet::from([AffordanceKind::Speak]),
                authority_scope: None,
            }),
        };
        let (mut kernel, _) = WorldKernel::create(&path, creation, &authenticated).unwrap();
        let snapshot = kernel.snapshot().unwrap();
        let command_id = CommandId::new();
        kernel
            .submit(
                CommandEnvelope {
                    id: command_id,
                    world_id: snapshot.world_id,
                    expected_revision: snapshot.revision,
                    caller: CallerId::Principal(owner),
                    body: CommandBody::ApproveDraft,
                },
                &authenticated,
            )
            .unwrap();

        let mut command_forged_head = kernel.state.clone();
        let mut command_forged_commits = kernel.journal.commits.clone();
        let command_forged = command_forged_commits.get_mut(&command_id).unwrap();
        let CommittedCommand::WorldCommand(command) = &mut command_forged.command else {
            panic!("expected a world command");
        };
        command.body = CommandBody::ActivateWorld;
        command_forged.digest = commit_digest(command_forged).unwrap();
        command_forged_head.last_commit_digest = Some(command_forged.digest.clone());

        assert!(matches!(
            verify_history(&command_forged_head, &command_forged_commits),
            Err(JournalError::Corrupt(_))
        ));

        let mut caller_forged_head = kernel.state.clone();
        let mut caller_forged_commits = kernel.journal.commits.clone();
        let caller_forged = caller_forged_commits.get_mut(&command_id).unwrap();
        let CommittedCommand::WorldCommand(command) = &mut caller_forged.command else {
            panic!("expected a world command");
        };
        command.caller = CallerId::Principal(PrincipalId::new("attacker@example.test"));
        caller_forged.digest = commit_digest(caller_forged).unwrap();
        caller_forged_head.last_commit_digest = Some(caller_forged.digest.clone());

        assert!(matches!(
            verify_history(&caller_forged_head, &caller_forged_commits),
            Err(JournalError::Corrupt(_))
        ));

        let mut effect_forged_head = kernel.state.clone();
        let mut effect_forged_commits = kernel.journal.commits.clone();
        let effect_forged = effect_forged_commits.get_mut(&command_id).unwrap();
        effect_forged.effect = WorldEffect::WorldActivated;
        effect_forged.digest = commit_digest(effect_forged).unwrap();
        effect_forged_head.last_commit_digest = Some(effect_forged.digest.clone());

        assert!(matches!(
            verify_history(&effect_forged_head, &effect_forged_commits),
            Err(JournalError::Corrupt(_))
        ));
    }

    #[test]
    fn a_forged_patch_effect_does_not_apply() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let owner = PrincipalId::new("owner@example.test");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let creation = CreateWorld {
            id: CommandId::new(),
            owner: owner.clone(),
            title: "Kharad".into(),
            patch: owner_patch(SubjectDeclaration {
                handle: DraftHandle::new("owner"),
                label: "Owner".into(),
                kind: SubjectKind::Person,
                controller: NewController::Human {
                    principal: owner.clone(),
                },
                affordances: BTreeSet::from([AffordanceKind::Speak]),
                authority_scope: None,
            }),
        };
        let (mut kernel, _) = WorldKernel::create(&path, creation, &authenticated).unwrap();
        let snapshot = kernel.snapshot().unwrap();
        let command_id = CommandId::new();
        kernel
            .submit(
                CommandEnvelope {
                    id: command_id,
                    world_id: snapshot.world_id,
                    expected_revision: snapshot.revision,
                    caller: CallerId::Principal(owner),
                    body: CommandBody::AdmitPatch {
                        answers: None,
                        patch: WorldPatch {
                            declarations: vec![
                                Declaration::Entity(EntityDeclaration {
                                    handle: DraftHandle::new("rhythm-road"),
                                    label: "The Rhythm Road".into(),
                                    kind: EntityKind::Place,
                                }),
                                Declaration::Subject(SubjectDeclaration {
                                    handle: DraftHandle::new("rhythm-authority"),
                                    label: "The Rhythm Authority".into(),
                                    kind: SubjectKind::Institution,
                                    controller: NewController::OperationalAgent,
                                    affordances: BTreeSet::from([AffordanceKind::Speak]),
                                    authority_scope: Some(Ref::Draft(DraftHandle::new(
                                        "rhythm-road",
                                    ))),
                                }),
                            ],
                            operations: Vec::new(),
                            evidence: Vec::new(),
                        },
                    },
                },
                &authenticated,
            )
            .unwrap();

        let mut forged_head = kernel.state.clone();
        let mut forged_commits = kernel.journal.commits.clone();
        let forged = forged_commits.get_mut(&command_id).unwrap();
        let WorldEffect::PatchAdmitted { resolved } = &mut forged.effect else {
            panic!("expected an admitted patch effect");
        };
        resolved.subjects[0].subject.authority_scope = Some(EntityId::issue());
        forged.digest = commit_digest(forged).unwrap();
        forged_head.last_commit_digest = Some(forged.digest.clone());

        assert!(matches!(
            verify_history(&forged_head, &forged_commits),
            Err(JournalError::Corrupt(_))
        ));
    }

    #[test]
    fn replay_rejects_genesis_that_does_not_derive_from_its_creation_command() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let owner = PrincipalId::new("owner@example.test");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let creation_id = CommandId::new();
        let creation = CreateWorld {
            id: creation_id,
            owner: owner.clone(),
            title: "Admitted".into(),
            patch: owner_patch(SubjectDeclaration {
                handle: DraftHandle::new("owner"),
                label: "Owner".into(),
                kind: SubjectKind::Person,
                controller: NewController::Human { principal: owner },
                affordances: BTreeSet::from([AffordanceKind::Speak]),
                authority_scope: None,
            }),
        };
        let (kernel, _) = WorldKernel::create(&path, creation, &authenticated).unwrap();
        let mut forged_head = kernel.state.clone();
        let mut forged_commits = kernel.journal.commits.clone();
        let forged = forged_commits.get_mut(&creation_id).unwrap();
        let CommittedCommand::CreateWorld(command) = &mut forged.command else {
            panic!("expected genesis creation command");
        };
        command.title = "Different command".into();
        forged.digest = commit_digest(forged).unwrap();
        forged_head.last_commit_digest = Some(forged.digest.clone());

        assert!(matches!(
            verify_history(&forged_head, &forged_commits),
            Err(JournalError::Corrupt(_))
        ));

        let mut binding_forged_head = kernel.state.clone();
        let mut binding_forged_commits = kernel.journal.commits.clone();
        let binding_forged = binding_forged_commits.get_mut(&creation_id).unwrap();
        let WorldEffect::WorldCreated { resolved, .. } = &mut binding_forged.effect else {
            panic!("expected genesis effect");
        };
        resolved.subjects[0].subject.label = "Not the creation subject".into();
        binding_forged.digest = commit_digest(binding_forged).unwrap();
        binding_forged_head.last_commit_digest = Some(binding_forged.digest.clone());

        assert!(matches!(
            verify_history(&binding_forged_head, &binding_forged_commits),
            Err(JournalError::Corrupt(_))
        ));
    }
}

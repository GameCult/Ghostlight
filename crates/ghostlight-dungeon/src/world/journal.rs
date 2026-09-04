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

const STATE_ROW: &str = "world_state.authority.v1";
const COMMIT_ROW: &str = "world_commit.authority.v1";

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
    let is_place = |entity_id: &super::EntityId| {
        state
            .entities
            .get(entity_id)
            .is_some_and(|record| record.kind == super::EntityKind::Place)
    };
    for (entity_id, entity) in &state.entities {
        if !super::patch::is_canonical_text(&entity.label) {
            return Err(JournalError::Corrupt(
                "entity label is empty or noncanonical".into(),
            ));
        }
        if let Some(container) = entity.container
            && (entity.kind != super::EntityKind::Place || !is_place(&container))
        {
            return Err(JournalError::Corrupt(
                "entity container does not name a canonical place".into(),
            ));
        }
        if !super::patch::containment_terminates(*entity_id, &state.entities) {
            return Err(JournalError::Corrupt("place contains itself".into()));
        }
    }
    for edge in state.edges.values() {
        let (from, to) = edge.endpoints();
        if !is_place(&from)
            || !is_place(&to)
            || from == to
            || !super::patch::is_valid_cost(edge.cost())
        {
            return Err(JournalError::Corrupt(
                "route does not join two distinct places at a valid cost".into(),
            ));
        }
    }
    for (subject_id, position) in &state.positions {
        if !state.subjects.contains_key(subject_id) || !is_place(&position.place) {
            return Err(JournalError::Corrupt(
                "position does not name a canonical subject and place".into(),
            ));
        }
    }
    let is_resource = |entity_id: &super::EntityId| {
        state
            .entities
            .get(entity_id)
            .is_some_and(|record| record.kind == super::EntityKind::Resource)
    };
    // Absence is zero and one slot holds one pair, so a store satisfying these
    // clauses cannot carry a zero, an empty holder, a duplicate, or a dangling
    // holding — with no scan for any of them.
    for (subject_id, held) in &state.holdings {
        if !state.subjects.contains_key(subject_id)
            || held.is_empty()
            || held
                .iter()
                .any(|(resource, quantity)| !is_resource(resource) || quantity.0 == 0)
        {
            return Err(JournalError::Corrupt(
                "holding does not name a canonical subject and resource at a nonzero quantity"
                    .into(),
            ));
        }
    }
    for (subject_id, targets) in &state.dependencies {
        let target_is_canonical = |target: &super::DependencyTarget| match target {
            super::DependencyTarget::Resource(entity_id) => is_resource(entity_id),
            super::DependencyTarget::Route(edge_id) => state.edges.contains_key(edge_id),
            super::DependencyTarget::Subject(other) => {
                other != subject_id && state.subjects.contains_key(other)
            }
        };
        if !state.subjects.contains_key(subject_id)
            || targets.is_empty()
            || !targets.iter().all(target_is_canonical)
        {
            return Err(JournalError::Corrupt(
                "dependency does not name a canonical subject and a distinct canonical target"
                    .into(),
            ));
        }
    }
    // The civic subgraph, in the slot the deleted `authority_scope` check
    // occupied: a jurisdiction names live ground under a canonical kind, an
    // office sits on an institution and lends something to a person who holds
    // no other office there, and a forum names a live subject.
    let target_is_canonical = |target: &super::AuthorityTarget| match target {
        super::AuthorityTarget::Subject(subject_id) => state.subjects.contains_key(subject_id),
        super::AuthorityTarget::PlaceSubtree(entity_id) => is_place(entity_id),
    };
    for (subject_id, grants) in &state.authority {
        if !state.subjects.contains_key(subject_id)
            || grants.is_empty()
            || !grants.iter().all(|grant| {
                super::patch::is_civic_name(&grant.kind.0) && target_is_canonical(&grant.over)
            })
        {
            return Err(JournalError::Corrupt(
                "authority does not name a canonical subject and live ground under a canonical kind"
                    .into(),
            ));
        }
    }
    for (institution, offices) in &state.selection {
        let mut incumbents = BTreeSet::new();
        if state.subjects.get(institution).map(|subject| subject.kind)
            != Some(super::SubjectKind::Institution)
            || offices.is_empty()
        {
            return Err(JournalError::Corrupt(
                "an office register does not name a canonical institution".into(),
            ));
        }
        for (name, office) in offices {
            let incumbent_is_person = office.incumbent.is_none_or(|incumbent| {
                state.subjects.get(&incumbent).map(|subject| subject.kind)
                    == Some(super::SubjectKind::Person)
                    && incumbents.insert(incumbent)
            });
            if !super::patch::is_civic_name(&name.0)
                || office.delegated.is_empty()
                || !office
                    .delegated
                    .iter()
                    .all(|kind| super::patch::is_civic_name(&kind.0))
                || !incumbent_is_person
            {
                return Err(JournalError::Corrupt(
                    "an office lends nothing, is misnamed, or is held twice or by a non-person"
                        .into(),
                ));
            }
        }
    }
    for (grievance, forum) in &state.redress {
        if !super::patch::is_civic_name(&grievance.0)
            || !state.subjects.contains_key(&forum.forum)
            || !target_is_canonical(&forum.standing)
        {
            return Err(JournalError::Corrupt(
                "a forum does not name a canonical grievance, subject, and standing".into(),
            ));
        }
    }
    if super::overlapping_holder(state).is_some() {
        return Err(JournalError::Corrupt(
            "one subject holds two overlapping jurisdictions of one kind".into(),
        ));
    }
    let mut controller_ids = BTreeSet::new();
    for (subject_id, subject) in &state.subjects {
        if !super::patch::is_canonical_text(&subject.label) {
            return Err(JournalError::Corrupt(
                "subject label is empty or noncanonical".into(),
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
    for (scope, granted) in &state.affordance_grants {
        if !state.controller_assignments.contains_key(scope)
            || granted.is_empty()
            || granted
                .iter()
                .any(|affordance_id| !state.affordance_catalog.contains_key(affordance_id))
        {
            return Err(JournalError::Corrupt(
                "affordance grant is empty, unscoped, or names no catalog entry".into(),
            ));
        }
    }
    if state
        .controller_assignments
        .keys()
        .any(|scope| !state.affordance_grants.contains_key(scope))
    {
        return Err(JournalError::Corrupt(
            "decision scope has no affordance grant".into(),
        ));
    }
    // Every stored entry still passes the declaration validator, so a forged
    // catalog row cannot install an entry the resolver would have refused.
    for entry in state.affordance_catalog.values() {
        if !super::patch::entry_is_admissible(entry) {
            return Err(JournalError::Corrupt(
                "stored affordance entry is not admissible".into(),
            ));
        }
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
        let entry = state
            .affordance_catalog
            .get(&event.invocation.affordance)
            .ok_or_else(|| JournalError::Corrupt("event uses an unknown affordance".into()))?;
        if !event_ids.insert(event.id)
            || event.revision == 0
            || event.revision > state.revision
            || event.revision <= previous_event_revision
            || event.controller_id != assignment.id()
            || !state
                .affordance_grants
                .get(&event.scope)
                .is_some_and(|granted| granted.contains(&event.invocation.affordance))
            || event.band >= entry.outcome_bands.len()
            || (entry.outcome_bands[event.band].effects.is_empty() && !event.effects.is_empty())
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
    apply_effect(
        state,
        command.id,
        &commit.command.caller(),
        &expected_effect,
    )
    .map_err(kernel_error)
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
    use crate::world::patch::kernel_speak_grant;
    use crate::world::tests::speak_entry;
    use crate::world::{
        AuthenticatedCaller, CallerId, CommandBody, CommandEnvelope, CreateWorld, Declaration,
        DraftHandle, EntityDeclaration, EntityId, EntityKind, NewController, Position, PrincipalId,
        Ref, SubjectDeclaration, SubjectKind, WorldKernel, WorldPatch,
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
                affordances: kernel_speak_grant(),
                position: None,
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
                affordances: kernel_speak_grant(),
                position: None,
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
                                    container: None,
                                }),
                                Declaration::Subject(SubjectDeclaration {
                                    handle: DraftHandle::new("rhythm-authority"),
                                    label: "The Rhythm Authority".into(),
                                    kind: SubjectKind::Institution,
                                    controller: NewController::OperationalAgent,
                                    affordances: BTreeSet::from([speak_entry(&kernel)]),
                                    position: Some(Ref::Draft(DraftHandle::new("rhythm-road"))),
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
        resolved.subjects[0].position = Some(Position {
            place: EntityId::issue(),
        });
        forged.digest = commit_digest(forged).unwrap();
        forged_head.last_commit_digest = Some(forged.digest.clone());

        assert!(matches!(
            verify_history(&forged_head, &forged_commits),
            Err(JournalError::Corrupt(_))
        ));
    }

    /// Replay re-derives the operation half too: a committed relocation rewritten
    /// to move a different subject no longer reduces from its command.
    #[test]
    fn a_forged_relocate_effect_does_not_apply() {
        use crate::world::tests::{activate, admit_topology, auth_principal, command, owner};

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let authenticated = auth_principal(owner());
        let (mut kernel, _) = WorldKernel::create(
            &path,
            crate::world::tests::creation(CommandId::new(), "Kharad"),
            &authenticated,
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let command_id = CommandId::new();
        kernel
            .submit(
                command(
                    &active,
                    command_id,
                    CallerId::Principal(owner()),
                    CommandBody::AdmitPatch {
                        answers: None,
                        patch: WorldPatch {
                            declarations: Vec::new(),
                            operations: vec![crate::world::ComponentOp::Relocate {
                                subject: Ref::Existing(topology.walker),
                                via: Ref::Existing(topology.ramp),
                            }],
                            evidence: Vec::new(),
                        },
                    },
                ),
                &authenticated,
            )
            .unwrap();

        let mut forged_head = kernel.state.clone();
        let mut forged_commits = kernel.journal.commits.clone();
        let forged = forged_commits.get_mut(&command_id).unwrap();
        let WorldEffect::PatchAdmitted { resolved } = &mut forged.effect else {
            panic!("expected an admitted patch effect");
        };
        let crate::world::ResolvedOp::Relocate { subject_id, .. } = &mut resolved.operations[0]
        else {
            panic!("expected a lowered relocation");
        };
        *subject_id = crate::world::SubjectId::issue();
        forged.digest = commit_digest(forged).unwrap();
        forged_head.last_commit_digest = Some(forged.digest.clone());

        assert!(matches!(
            verify_history(&forged_head, &forged_commits),
            Err(JournalError::Corrupt(_))
        ));
    }

    /// Soul: `a_forged_band_or_forged_effect_does_not_apply` proves the live
    /// `apply_effect` arm re-derives. `verify_state_shape` does not re-derive a
    /// band — it only bounds the index and refuses effects under an empty band —
    /// so the layer that kills a persisted forgery is replay effect-equality.
    /// This forges a committed decision inside the journal, re-digests it into a
    /// contiguous chain, and asserts recovery refuses it.
    #[test]
    fn soul_a_forged_band_or_effect_in_the_journal_dies_at_replay() {
        use crate::world::tests::{
            activate, admit_custody, admit_topology, affordance_named, auth_principal, command,
            opportunity_for, owner,
        };
        use crate::world::{
            AuthenticatedCaller, DecisionInvocation, Magnitude, ProposedEffect, Quantity,
            RoleBinding, Target,
        };

        let forge = |kind: &str, mutate: &dyn Fn(&mut crate::world::DecisionEvent)| {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("world.cc");
            let authenticated = auth_principal(owner());
            let (mut kernel, _) = WorldKernel::create(
                &path,
                crate::world::tests::creation(CommandId::new(), "ForgedDecision"),
                &authenticated,
            )
            .unwrap();
            let topology = admit_topology(&mut kernel);
            let custody = admit_custody(&mut kernel, &topology);
            let active = activate(&mut kernel);
            let entry = affordance_named(&active, kind);
            let opportunity = opportunity_for(&active, custody.holder);
            let caller = CallerId::Controller(opportunity.controller_id);
            let command_id = CommandId::new();
            let role = |name: &str, target| RoleBinding {
                role: crate::world::Role(name.into()),
                target,
            };
            kernel
                .submit(
                    command(
                        &active,
                        command_id,
                        caller.clone(),
                        CommandBody::ExerciseDecision {
                            opportunity,
                            invocation: DecisionInvocation {
                                affordance: entry,
                                bindings: vec![
                                    role("from", Target::Subject(custody.holder)),
                                    role("recipient", Target::Subject(custody.counterparty)),
                                    role("place", Target::Entity(topology.yard)),
                                    role("resource", Target::Entity(custody.tithe)),
                                ],
                                proposed: vec![ProposedEffect {
                                    slot: 0,
                                    magnitude: Magnitude::Quantity(Quantity(2)),
                                }],
                                speech: None,
                            },
                        },
                    ),
                    &AuthenticatedCaller::fixture(caller),
                )
                .unwrap();

            // The honest journal recovers.
            assert!(verify_history(&kernel.state, &kernel.journal.commits).is_ok());

            let mut forged_head = kernel.state.clone();
            let mut forged_commits = kernel.journal.commits.clone();
            let forged = forged_commits.get_mut(&command_id).unwrap();
            let WorldEffect::DecisionExercised { event, .. } = &mut forged.effect else {
                panic!("expected an exercised decision effect");
            };
            mutate(event);
            let forged_event = event.clone();
            *forged_head.events.last_mut().unwrap() = forged_event;
            forged.digest = commit_digest(forged).unwrap();
            forged_head.last_commit_digest = Some(forged.digest.clone());
            forged_head.state_digest = String::new();
            forged_head.state_digest = crate::world::state_digest(&forged_head).unwrap();
            (forged_head, forged_commits)
        };

        // A band the seed did not draw, held inside a chain that verifies.
        let (head, commits) = forge("carry_chance", &|event| {
            event.band = (event.band + 1) % 3;
        });
        assert!(matches!(
            verify_history(&head, &commits),
            Err(JournalError::Corrupt(_))
        ));

        // A magnitude the ceiling admitted but the lowering never produced.
        let (head, commits) = forge("carry", &|event| {
            let Some(crate::world::ResolvedOp::Transfer { qty, .. }) = event.effects.first_mut()
            else {
                panic!("expected a lowered transfer");
            };
            *qty = Quantity(1);
        });
        assert!(matches!(
            verify_history(&head, &commits),
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
                affordances: kernel_speak_grant(),
                position: None,
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

    /// Soul falsification: the extended replay shape check is not decoration.
    /// A forged store row carrying a self-loop, a cost out of range, a
    /// non-place endpoint, or a position on a non-place is refused by
    /// `recover`, which is the path `WorldJournal::open` takes.
    #[test]
    fn soul_forged_topology_store_rows_are_refused_on_recover() {
        use crate::world::patch::{AccessKind, Cost, EdgeRecord, MAX_ROUTE_COST};
        use crate::world::tests::{admit_topology, auth_principal, owner};

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            crate::world::tests::creation(CommandId::new(), "Forged Rows"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let commits: Vec<CultCacheEnvelope> = kernel
            .journal
            .commits
            .values()
            .map(|commit| {
                envelope(COMMIT_ROW, COMMIT_SCHEMA, commit.command.id().key(), commit).unwrap()
            })
            .collect();
        let rows_for = |state: &WorldState, row_type: &str| {
            let mut rows = commits.clone();
            rows.push(envelope(row_type, STATE_SCHEMA, state.world_id.key(), state).unwrap());
            rows
        };

        // The honest state recovers.
        assert!(recover(rows_for(&kernel.state, STATE_ROW), None).is_ok());

        let self_loop = {
            let mut state = kernel.state.clone();
            state.edges.insert(
                topology.ramp,
                EdgeRecord::Route {
                    label: "The Yard Ramp".into(),
                    from: topology.yard,
                    to: topology.yard,
                    access: AccessKind::Public,
                    cost: Cost(12),
                    open: true,
                },
            );
            state
        };
        let bad_cost = {
            let mut state = kernel.state.clone();
            state
                .edges
                .get_mut(&topology.ramp)
                .unwrap()
                .set_cost(Cost(MAX_ROUTE_COST + 1));
            state
        };
        let non_place_endpoint = {
            let mut state = kernel.state.clone();
            state.entities.get_mut(&topology.road).unwrap().kind =
                crate::world::EntityKind::Resource;
            state
        };
        let stray_position = {
            let mut state = kernel.state.clone();
            state.positions.insert(
                topology.walker,
                Position {
                    place: EntityId::issue(),
                },
            );
            state
        };
        for forged in [self_loop, bad_cost, non_place_endpoint, stray_position] {
            let error = recover(rows_for(&forged, STATE_ROW), None).unwrap_err();
            assert!(
                matches!(error, JournalError::Corrupt(_)),
                "a forged topology row recovered: {error:?}"
            );
        }
    }

    /// Soul falsification: a store written before the topology partitions is
    /// refused outright, with no migration adapter in the path.
    #[test]
    fn soul_a_pre_topology_store_row_is_refused() {
        use crate::world::tests::{admit_topology, auth_principal, owner};

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            crate::world::tests::creation(CommandId::new(), "Old Store"),
            &auth_principal(owner()),
        )
        .unwrap();
        admit_topology(&mut kernel);
        let mut rows: Vec<CultCacheEnvelope> = kernel
            .journal
            .commits
            .values()
            .map(|commit| {
                envelope(
                    "world_commit.foundation.v2",
                    COMMIT_SCHEMA,
                    commit.command.id().key(),
                    commit,
                )
                .unwrap()
            })
            .collect();
        rows.push(
            envelope(
                "world_state.foundation.v1",
                STATE_SCHEMA,
                kernel.state.world_id.key(),
                &kernel.state,
            )
            .unwrap(),
        );
        let error = recover(rows, None).unwrap_err();
        let JournalError::Corrupt(message) = error else {
            panic!("expected a corrupt store, got {error:?}");
        };
        assert!(
            message.contains("unadmitted row type"),
            "unexpected refusal: {message}"
        );
    }

    /// A committed levy survives a restart exactly: the snapshot, the band, the
    /// lowered effects, and the re-derived authority verdict all come back, and
    /// the same envelope is already applied.
    #[test]
    fn restart_replay_after_a_levy_is_exact() {
        use crate::world::tests::{affordance_named, civic_world, command, opportunity_for, owner};
        use crate::world::{
            DecisionInvocation, Magnitude, ProposedEffect, Quantity, Role, RoleBinding,
            SubmitReceipt, Target,
        };

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            crate::world::tests::creation(CommandId::new(), "Replayed Levy"),
            &crate::world::tests::auth_principal(owner()),
        )
        .unwrap();
        let (_, civic, active) = civic_world(&mut kernel);
        let opportunity = opportunity_for(&active, civic.treasury);
        let caller = CallerId::Controller(opportunity.controller_id);
        let envelope = command(
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
        );
        kernel
            .submit(
                envelope.clone(),
                &AuthenticatedCaller::fixture(caller.clone()),
            )
            .unwrap();
        let committed = kernel.snapshot().unwrap();
        let event = kernel.state.events.last().expect("the levy event").clone();
        let world_id = committed.world_id;
        drop(kernel);

        let mut reopened = WorldKernel::open(&path, world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), committed);
        let replayed = reopened.state.events.last().expect("the replayed event");
        assert_eq!(replayed.band, event.band);
        assert_eq!(replayed.effects, event.effects);
        assert!(matches!(
            reopened
                .submit(envelope, &AuthenticatedCaller::fixture(caller))
                .unwrap(),
            SubmitReceipt::AlreadyApplied(_)
        ));
    }

    /// Soul falsification: the civic shape checks are not decoration. A forged
    /// store row that widens a jurisdiction, doubles an incumbency, seats an
    /// institution, or empties a delegation is refused by `recover`.
    #[test]
    fn a_forged_authority_or_incumbency_is_corrupt() {
        use crate::world::tests::{
            BAILIFF_OFFICE, LEVY_KIND, WARDEN_OFFICE, auth_principal, authority_kind, civic_world,
            office, owner,
        };
        use crate::world::{AuthorityGrant, AuthorityTarget, Office};
        use std::collections::BTreeSet;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            crate::world::tests::creation(CommandId::new(), "Forged Civics"),
            &auth_principal(owner()),
        )
        .unwrap();
        let (topology, civic, _) = civic_world(&mut kernel);
        let commits: Vec<CultCacheEnvelope> = kernel
            .journal
            .commits
            .values()
            .map(|commit| {
                envelope(COMMIT_ROW, COMMIT_SCHEMA, commit.command.id().key(), commit).unwrap()
            })
            .collect();
        let rows_for = |state: &WorldState| {
            let mut rows = commits.clone();
            rows.push(envelope(STATE_ROW, STATE_SCHEMA, state.world_id.key(), state).unwrap());
            rows
        };
        assert!(recover(rows_for(&kernel.state), None).is_ok());

        let widened = {
            let mut state = kernel.state.clone();
            state.authority.insert(
                civic.reeve,
                BTreeSet::from([AuthorityGrant {
                    kind: authority_kind(LEVY_KIND),
                    over: AuthorityTarget::PlaceSubtree(civic.chamber),
                }]),
            );
            state
        };
        let dangling = {
            let mut state = kernel.state.clone();
            state.authority.insert(
                civic.outsider,
                BTreeSet::from([AuthorityGrant {
                    kind: authority_kind(LEVY_KIND),
                    over: AuthorityTarget::PlaceSubtree(EntityId::issue()),
                }]),
            );
            state
        };
        let doubled = {
            let mut state = kernel.state.clone();
            state
                .selection
                .get_mut(&civic.treasury)
                .unwrap()
                .get_mut(&office(BAILIFF_OFFICE))
                .unwrap()
                .incumbent = Some(civic.reeve);
            state
        };
        let seated_institution = {
            let mut state = kernel.state.clone();
            state
                .selection
                .get_mut(&civic.treasury)
                .unwrap()
                .get_mut(&office(WARDEN_OFFICE))
                .unwrap()
                .incumbent = Some(civic.treasury);
            state
        };
        let inert_office = {
            let mut state = kernel.state.clone();
            state.selection.get_mut(&civic.treasury).unwrap().insert(
                office(WARDEN_OFFICE),
                Office {
                    incumbent: Some(civic.reeve),
                    delegated: BTreeSet::new(),
                },
            );
            state
        };
        let office_on_person = {
            let mut state = kernel.state.clone();
            state
                .selection
                .insert(topology.walker, state.selection[&civic.treasury].clone());
            state
        };
        for forged in [
            widened,
            dangling,
            doubled,
            seated_institution,
            inert_office,
            office_on_person,
        ] {
            let error = recover(rows_for(&forged), None).unwrap_err();
            assert!(
                matches!(error, JournalError::Corrupt(_)),
                "a forged civic row recovered: {error:?}"
            );
        }
    }

    /// Soul falsification: a store written before the civic partitions is
    /// refused outright, with no migration adapter in the path.
    #[test]
    fn a_store_from_the_previous_schema_is_refused() {
        use crate::world::tests::{auth_principal, civic_world, owner};

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            crate::world::tests::creation(CommandId::new(), "Pre-Authority"),
            &auth_principal(owner()),
        )
        .unwrap();
        civic_world(&mut kernel);
        let mut rows: Vec<CultCacheEnvelope> = kernel
            .journal
            .commits
            .values()
            .map(|commit| {
                envelope(
                    "world_commit.affordance.v1",
                    COMMIT_SCHEMA,
                    commit.command.id().key(),
                    commit,
                )
                .unwrap()
            })
            .collect();
        rows.push(
            envelope(
                "world_state.affordance.v1",
                STATE_SCHEMA,
                kernel.state.world_id.key(),
                &kernel.state,
            )
            .unwrap(),
        );
        let error = recover(rows, None).unwrap_err();
        let JournalError::Corrupt(message) = error else {
            panic!("expected a corrupt store, got {error:?}");
        };
        assert!(
            message.contains("unadmitted row type"),
            "unexpected refusal: {message}"
        );
    }
}

#[cfg(test)]
mod custody_tests {
    use super::*;
    use crate::world::{
        CallerId, CommandBody, CommandId, ComponentOp, DependencyRef, EntityKind, Quantity, Ref,
        SubmitReceipt, WorldKernel, WorldPatch,
        tests::{admit_custody, admit_topology, auth_principal, command, creation, owner},
    };

    /// Replay re-derives the custody partitions from the commit log alone: a
    /// world reopened after a transfer is byte-identical, and the transfer's own
    /// envelope is idempotent.
    #[test]
    fn restart_replay_after_a_transfer_is_exact() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let authenticated = auth_principal(owner());
        let (mut kernel, _) =
            WorldKernel::create(&path, creation(CommandId::new(), "Replay"), &authenticated)
                .unwrap();
        let topology = admit_topology(&mut kernel);
        let custody = admit_custody(&mut kernel, &topology);
        let active = crate::world::tests::activate(&mut kernel);

        let envelope = command(
            &active,
            CommandId::new(),
            CallerId::Principal(owner()),
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::Transfer {
                        from: Ref::Existing(custody.holder),
                        to: Ref::Existing(custody.counterparty),
                        resource: Ref::Existing(custody.tithe),
                        qty: Quantity(3),
                    }],
                    evidence: Vec::new(),
                },
            },
        );
        let SubmitReceipt::Applied(receipt) =
            kernel.submit(envelope.clone(), &authenticated).unwrap()
        else {
            panic!("expected an applied transfer");
        };
        let accepted = kernel.snapshot().unwrap();
        let world_id = accepted.world_id;
        drop(kernel);

        let mut reopened = WorldKernel::open(&path, world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), accepted);
        assert_eq!(
            reopened.submit(envelope, &authenticated).unwrap(),
            SubmitReceipt::AlreadyApplied(receipt)
        );
    }

    /// Replay recomputes `reduce`, so the band and the lowered operations are
    /// re-derived rather than trusted: a stored event whose band or effects the
    /// seed does not reproduce fails effect equality on the way back in.
    #[test]
    fn restart_replay_after_an_action_is_exact() {
        use crate::world::tests::affordance_named;
        use crate::world::{AuthenticatedCaller, Magnitude, ProposedEffect, RoleBinding, Target};

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let authenticated = auth_principal(owner());
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "ActionReplay"),
            &authenticated,
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let custody = admit_custody(&mut kernel, &topology);
        let active = crate::world::tests::activate(&mut kernel);
        let carry = affordance_named(&active, "carry");
        let opportunity = crate::world::tests::opportunity_for(&active, custody.holder);
        let caller = CallerId::Controller(opportunity.controller_id);

        let role = |name: &str, target| RoleBinding {
            role: crate::world::Role(name.into()),
            target,
        };
        let envelope = command(
            &active,
            CommandId::new(),
            caller.clone(),
            CommandBody::ExerciseDecision {
                opportunity: opportunity.clone(),
                invocation: crate::world::DecisionInvocation {
                    affordance: carry,
                    bindings: vec![
                        role("from", Target::Subject(custody.holder)),
                        role("recipient", Target::Subject(custody.counterparty)),
                        role("place", Target::Entity(topology.yard)),
                        role("resource", Target::Entity(custody.tithe)),
                    ],
                    proposed: vec![ProposedEffect {
                        slot: 0,
                        magnitude: Magnitude::Quantity(Quantity(2)),
                    }],
                    speech: None,
                },
            },
        );
        let authenticated_controller = AuthenticatedCaller::fixture(caller);
        let SubmitReceipt::Applied(receipt) = kernel
            .submit(envelope.clone(), &authenticated_controller)
            .unwrap()
        else {
            panic!("expected an applied invocation");
        };
        let accepted = kernel.snapshot().unwrap();
        let committed = accepted.events.last().expect("the committed event").clone();
        assert!(!committed.effects.is_empty());
        let world_id = accepted.world_id;
        drop(kernel);

        let mut reopened = WorldKernel::open(&path, world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), accepted);
        assert_eq!(
            reopened
                .snapshot()
                .unwrap()
                .events
                .last()
                .expect("the replayed event"),
            &committed
        );
        assert_eq!(
            reopened
                .submit(envelope, &authenticated_controller)
                .unwrap(),
            SubmitReceipt::AlreadyApplied(receipt)
        );
    }

    /// A store written at the pass-2 schema is refused outright. There is no
    /// migration adapter, and the refusal is the behaviour.
    #[test]
    fn a_store_from_the_previous_schema_is_refused() {
        let row = CultCacheEnvelope {
            key: "state".into(),
            r#type: "world_state.topology.v1".into(),
            payload: Vec::new(),
            stored_at: Utc::now().to_rfc3339(),
            schema_id: Some("ghostlight.world_state.topology.v1".into()),
        };
        let error = recover(vec![row], None).unwrap_err();
        let JournalError::Corrupt(message) = error else {
            panic!("expected a corrupt store");
        };
        assert!(
            message.contains("unadmitted row type"),
            "unexpected refusal: {message}"
        );
    }

    /// Absence is zero and one slot holds one pair, so a stored zero, an empty
    /// holder, and a holding on something that is not a resource are each
    /// corrupt at replay.
    ///
    /// Deviation from the spec's wording, decided here: the assertion is against
    /// `verify_state_shape`, which owns these clauses. Routing through
    /// `verify_history` would also fail on replay-equality, so it would pass
    /// even if the shape clauses were deleted — which is the opposite of a
    /// falsification.
    #[test]
    fn a_zero_or_dangling_holding_is_corrupt() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let authenticated = auth_principal(owner());
        let (mut kernel, _) =
            WorldKernel::create(&path, creation(CommandId::new(), "Shape"), &authenticated)
                .unwrap();
        let topology = admit_topology(&mut kernel);
        let custody = admit_custody(&mut kernel, &topology);
        crate::world::tests::activate(&mut kernel);
        verify_state_shape(&kernel.state).expect("the admitted world has a canonical shape");

        let mut zero = kernel.state.clone();
        zero.holdings
            .get_mut(&custody.holder)
            .unwrap()
            .insert(custody.tithe, Quantity(0));
        assert!(matches!(
            verify_state_shape(&zero),
            Err(JournalError::Corrupt(_))
        ));

        let mut emptied = kernel.state.clone();
        emptied.holdings.get_mut(&custody.holder).unwrap().clear();
        assert!(matches!(
            verify_state_shape(&emptied),
            Err(JournalError::Corrupt(_))
        ));

        let mut not_a_resource = kernel.state.clone();
        assert_eq!(
            not_a_resource.entities[&topology.yard].kind,
            EntityKind::Place
        );
        not_a_resource
            .holdings
            .get_mut(&custody.holder)
            .unwrap()
            .insert(topology.yard, Quantity(1));
        assert!(matches!(
            verify_state_shape(&not_a_resource),
            Err(JournalError::Corrupt(_))
        ));

        let mut dangling = kernel.state.clone();
        dangling.dependencies.insert(
            custody.holder,
            BTreeSet::from([crate::world::DependencyTarget::Subject(custody.holder)]),
        );
        assert!(matches!(
            verify_state_shape(&dangling),
            Err(JournalError::Corrupt(_))
        ));
    }

    /// Soul: replay is exact after *every* custody and dependency operation, not
    /// only after a transfer. The reopened world is byte-identical, the last
    /// envelope is idempotent, and the whole chain still verifies.
    #[test]
    fn soul_replay_is_exact_after_every_custody_operation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let authenticated = auth_principal(owner());
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "EveryOperation"),
            &authenticated,
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        // `Admit` and its evidence are Draft-only, so the opening balance is
        // already on the log before activation.
        let custody = admit_custody(&mut kernel, &topology);
        let active = crate::world::tests::activate(&mut kernel);

        let envelope = command(
            &active,
            CommandId::new(),
            CallerId::Principal(owner()),
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![
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
                        ComponentOp::Bind {
                            subject: Ref::Existing(custody.holder),
                            target: DependencyRef::Route(Ref::Existing(topology.shutter)),
                        },
                        ComponentOp::Bind {
                            subject: Ref::Existing(custody.holder),
                            target: DependencyRef::Subject(Ref::Existing(custody.counterparty)),
                        },
                    ],
                    evidence: Vec::new(),
                },
            },
        );
        let SubmitReceipt::Applied(receipt) =
            kernel.submit(envelope.clone(), &authenticated).unwrap()
        else {
            panic!("expected an applied custody patch");
        };
        let accepted = kernel.snapshot().unwrap();
        let holdings = kernel.state.holdings.clone();
        let dependencies = kernel.state.dependencies.clone();
        let digest = kernel.state.state_digest.clone();
        let world_id = accepted.world_id;
        drop(kernel);

        let mut reopened = WorldKernel::open(&path, world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), accepted);
        assert_eq!(reopened.state.holdings, holdings);
        assert_eq!(reopened.state.dependencies, dependencies);
        assert_eq!(reopened.state.state_digest, digest);
        assert_eq!(
            reopened.submit(envelope, &authenticated).unwrap(),
            SubmitReceipt::AlreadyApplied(receipt)
        );
    }

    /// Soul: a dependency naming a target of the wrong kind, or no canonical
    /// target at all, is refused at replay. The shape clauses carry the
    /// referential integrity the operation arm enforces at admission.
    #[test]
    fn soul_a_dependency_on_an_absent_or_wrong_kind_target_is_corrupt() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let authenticated = auth_principal(owner());
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "DependencyShape"),
            &authenticated,
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let custody = admit_custody(&mut kernel, &topology);
        crate::world::tests::activate(&mut kernel);
        verify_state_shape(&kernel.state).expect("the admitted world has a canonical shape");

        // A place is not a resource, so a resource dependency naming one dangles.
        let mut wrong_kind = kernel.state.clone();
        wrong_kind.dependencies.insert(
            custody.holder,
            BTreeSet::from([crate::world::DependencyTarget::Resource(topology.yard)]),
        );
        assert!(matches!(
            verify_state_shape(&wrong_kind),
            Err(JournalError::Corrupt(_))
        ));

        // An empty target set is the second representation of nothing, and there
        // is only supposed to be one.
        let mut emptied = kernel.state.clone();
        emptied.dependencies.insert(custody.holder, BTreeSet::new());
        assert!(matches!(
            verify_state_shape(&emptied),
            Err(JournalError::Corrupt(_))
        ));

        // A holder key that names no canonical subject.
        let mut orphan = kernel.state.clone();
        let absent = *orphan.subjects.keys().next().unwrap();
        orphan.dependencies.insert(
            absent,
            BTreeSet::from([crate::world::DependencyTarget::Resource(custody.tithe)]),
        );
        orphan.subjects.remove(&absent);
        assert!(matches!(
            verify_state_shape(&orphan),
            Err(JournalError::Corrupt(_))
        ));
    }
}

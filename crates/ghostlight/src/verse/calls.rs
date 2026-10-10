//! Provider call records, the run's spend, and replay.
//!
//! `RecordingInferencePort` is the one owner of a run's spend. Before it
//! forwards a call it decides, from the request alone, whether the call could
//! pass the run's token cap; after the reply it charges what the call really
//! cost. Every reply is stored as a `ProviderCallRecord` before the output
//! returns to the lane, so `ReplayInferencePort` can serve the same run again
//! from the store with no provider behind it.
//!
//! Lanes and runners never count tokens. A request is held against the cap by
//! its bound: the serialized provider request's bytes (a token is at least one
//! byte, so bytes bound the prompt) plus the request's own output allowance.
//!
//! The run's spend is one persisted ledger row per run, owned by the store and
//! written where it is spent. A call reserves its bound in that row before the
//! provider is called; a reply settles the reservation to what it reported
//! (prompt plus completion, cached prompt tokens being part of the prompt; none
//! reported is charged the bound); a fault leaves the reservation charged. A
//! restart reads the row, so no restart returns budget. A reply that reports
//! more than the call's bound is charged in full and faults the call, so the
//! run's spend never passes the cap unnoticed. One port at a time owns a run.

use crate::controllers::{
    InferenceEvent, InferenceFault, InferenceOutput, InferencePort, InferencePurpose,
    InferenceRequest, PreparedInference, REQUEST_EXPIRY, TokenUsage, ToolResultOracle, unix_ms,
};
use async_trait::async_trait;
use chrono::Utc;
use codex_connector::CodexProviderRequest;
use cultcache_rs::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::Path,
    sync::{Arc, Mutex},
};
use thiserror::Error;

const CALL_RECORD_ROW: &str = "verse_provider_call.v1";
const RUN_LEDGER_ROW: &str = "verse_run_ledger.v1";
/// The run ledger document's schema.
const RUN_LEDGER_SCHEMA: &str = "ghostlight.verse_run_ledger.v1";
/// The call record document's schema. A store holding any other row type or
/// schema is refused at open: this store is not migrated from, or shared with,
/// another document.
pub const CALL_RECORD_SCHEMA: &str = "ghostlight.verse_provider_call.v1";

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CallRecordError {
    #[error("a run id is 1 to 64 characters of letters, digits, '-' and '_'")]
    InvalidRunId,
    #[error("the call record store could not be read or written")]
    Store,
    #[error("the call record store holds a row that is not a canonical call record")]
    Corrupt,
    #[error("a call record's sequence is not the next in its run")]
    OutOfSequence,
    #[error("replay needs one run's records in dense sequence from 1")]
    NotReplayable,
    #[error("another recording port already owns this run")]
    RunInUse,
    #[error("the call could pass the run's token cap")]
    OverCap,
    #[error("the run was recorded with a different {0}")]
    RunMismatch(RunField),
}

/// What a run is bound to when it begins, named by the field a mismatch refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunField {
    /// The token cap the run was opened with.
    Cap,
    /// The identity of the world the run started from.
    WorldId,
    /// The state digest of the world the run started from.
    StateDigest,
}

impl std::fmt::Display for RunField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Cap => "token cap",
            Self::WorldId => "world id",
            Self::StateDigest => "world state digest",
        })
    }
}

/// The world a run began from: its identity and state digest. Replay is of a
/// restored copy of this world, so it takes the same two values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStart {
    world_id: String,
    state_digest: String,
}

impl RunStart {
    pub fn of(world: &crate::WorldSnapshot) -> Self {
        Self {
            world_id: world.world_id.text(),
            state_digest: world.state_digest.clone(),
        }
    }
}

/// A run's identity as a key segment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RunId(String);

impl RunId {
    pub fn new(id: &str) -> Result<Self, CallRecordError> {
        let valid = !id.is_empty()
            && id.len() <= 64
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
        if valid {
            Ok(Self(id.to_owned()))
        } else {
            Err(CallRecordError::InvalidRunId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One provider call of one run: the request's content digest (never its text, so no
/// prompt or credential is stored), the reply exactly as the lane received it,
/// and what the call was charged. Keyed (run id, sequence).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCallRecord {
    run_id: String,
    sequence: u64,
    purpose: InferencePurpose,
    model: String,
    content_sha256: String,
    events: Vec<InferenceEvent>,
    receipt_digest: String,
    usage: Option<TokenUsage>,
    charged: u64,
}

impl ProviderCallRecord {
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn purpose(&self) -> InferencePurpose {
        self.purpose
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// Lowercase hex of the request's content digest (see `content_digest`).
    pub fn content_sha256(&self) -> &str {
        &self.content_sha256
    }

    pub fn usage(&self) -> Option<TokenUsage> {
        self.usage
    }

    /// Tokens this call was charged against the run's cap.
    pub fn charged(&self) -> u64 {
        self.charged
    }

    /// The reply as the lane received it.
    pub fn output(&self) -> InferenceOutput {
        let output = InferenceOutput::new(self.events.clone(), self.receipt_digest.clone());
        match self.usage {
            Some(usage) => output.with_usage(usage),
            None => output,
        }
    }
}

fn row_key(run: &str, sequence: u64) -> String {
    format!("{run}/{sequence:020}")
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// What a request asks of the provider, without the ids that name it. The
/// request and conversation ids are derived from the command id, and a command
/// id from the world it ran in, so keying on them would tie a recorded run to
/// one world's random identity. Everything else the provider is sent
/// (model, instructions, input, tools, shape) is in the digest.
fn content_digest(request: &CodexProviderRequest) -> Result<String, InferenceFault> {
    // Exhaustive on purpose: a field added upstream must be placed in the key
    // or left out here by name before this compiles.
    let CodexProviderRequest {
        schema_id,
        request_id: _request_id,           // names the call, not what it asks
        conversation_id: _conversation_id, // names the call, not what it asks
        model,
        instructions,
        input,
        reasoning_effort,
        reasoning_summary,
        service_tier,
        output_format_name,
        previous_response_id,
        tools,
        tool_choice,
        parallel_tool_calls,
        output_schema_json,
        max_output_tokens,
        prompt_cache_key,
    } = request;
    let bytes = rmp_serde::to_vec(&(
        schema_id,
        model,
        instructions,
        input,
        reasoning_effort,
        reasoning_summary,
        service_tier,
        output_format_name,
        previous_response_id,
        tools,
        tool_choice,
        parallel_tool_calls,
        output_schema_json,
        max_output_tokens,
        prompt_cache_key,
    ))
    .map_err(|_| InferenceFault::new("a provider request could not be measured"))?;
    Ok(hex(&Sha256::digest(bytes)))
}

/// A run's persisted ledger, the one owner of what the run began as: the cap
/// it was opened under, the world it started from, and the tokens reserved or
/// charged so far.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct RunLedger {
    run_id: String,
    cap: u64,
    start: RunStart,
    spent: u64,
}

enum DecodedRow {
    Call(ProviderCallRecord),
    Ledger(RunLedger),
}

fn canonical<T: Serialize + for<'de> Deserialize<'de>>(
    row: &CultCacheEnvelope,
) -> Result<T, CallRecordError> {
    let value: T = rmp_serde::from_slice(&row.payload).map_err(|_| CallRecordError::Corrupt)?;
    let again = rmp_serde::to_vec_named(&value).map_err(|_| CallRecordError::Corrupt)?;
    if again == row.payload {
        Ok(value)
    } else {
        Err(CallRecordError::Corrupt)
    }
}

fn decode_row(row: &CultCacheEnvelope) -> Result<DecodedRow, CallRecordError> {
    match (row.r#type.as_str(), row.schema_id.as_deref()) {
        (CALL_RECORD_ROW, Some(CALL_RECORD_SCHEMA)) => {
            let record: ProviderCallRecord = canonical(row)?;
            // A run id that is no whole key segment would key into another
            // run's records, and sequences count from 1.
            if RunId::new(&record.run_id).is_err()
                || record.sequence == 0
                || row.key != row_key(&record.run_id, record.sequence)
            {
                return Err(CallRecordError::Corrupt);
            }
            Ok(DecodedRow::Call(record))
        }
        (RUN_LEDGER_ROW, Some(RUN_LEDGER_SCHEMA)) => {
            let ledger: RunLedger = canonical(row)?;
            if RunId::new(&ledger.run_id).is_err() || row.key != ledger.run_id {
                return Err(CallRecordError::Corrupt);
            }
            Ok(DecodedRow::Ledger(ledger))
        }
        _ => Err(CallRecordError::Corrupt),
    }
}

#[derive(Default)]
struct RunTally {
    last_sequence: u64,
    /// The persisted ledger, read at open and written where it is spent.
    ledger: Option<RunLedger>,
}

struct StoreState {
    store: OwnedRedbMessagePackBackingStore,
    runs: BTreeMap<String, RunTally>,
    /// Runs with a recording port alive; one ledger has one owner.
    owned: BTreeSet<String>,
}

/// The call records and spend ledgers of every run in one CultCache file.
/// Records are appended and never changed; a run's sequence is dense from 1.
pub struct CallRecordStore {
    state: Mutex<StoreState>,
}

impl CallRecordStore {
    /// Opens the file, creating it empty when absent. Every row must be a
    /// canonical call record or run ledger of this schema with its key, a
    /// dense sequence, and a ledger that covers its records' charges; anything
    /// else, an earlier or later schema version included, is refused and
    /// nothing is migrated.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CallRecordError> {
        let store = OwnedRedbMessagePackBackingStore::new(path.as_ref())
            .map_err(|_| CallRecordError::Store)?;
        store
            .validate_path_identity()
            .map_err(|_| CallRecordError::Store)?;
        let rows = store.pull_all().map_err(|_| CallRecordError::Store)?;
        let mut runs: BTreeMap<String, RunTally> = BTreeMap::new();
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        let mut charged: BTreeMap<String, u64> = BTreeMap::new();
        for row in &rows {
            match decode_row(row)? {
                DecodedRow::Call(record) => {
                    *counts.entry(record.run_id.clone()).or_default() += 1;
                    let total = charged.entry(record.run_id.clone()).or_default();
                    *total = total.saturating_add(record.charged);
                    let tally = runs.entry(record.run_id).or_default();
                    tally.last_sequence = tally.last_sequence.max(record.sequence);
                }
                DecodedRow::Ledger(ledger) => {
                    let run = ledger.run_id.clone();
                    runs.entry(run).or_default().ledger = Some(ledger);
                }
            }
        }
        // Sequences start at 1 and keys are unique, so a run is dense exactly
        // when its rows number its highest sequence. Every record was reserved
        // in a ledger before it was made, so a run with records has a ledger
        // that covers what they charged.
        for (run, tally) in &runs {
            let rows = counts.get(run).copied().unwrap_or(0);
            let recorded = charged.get(run).copied().unwrap_or(0);
            let covered = tally
                .ledger
                .as_ref()
                .is_some_and(|ledger| ledger.spent >= recorded);
            if rows != tally.last_sequence || !covered {
                return Err(CallRecordError::Corrupt);
            }
        }
        Ok(Self {
            state: Mutex::new(StoreState {
                store,
                runs,
                owned: BTreeSet::new(),
            }),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, StoreState>, CallRecordError> {
        self.state.lock().map_err(|_| CallRecordError::Store)
    }

    /// The sequence the next record of this run must carry.
    pub fn next_sequence(&self, run: &RunId) -> Result<u64, CallRecordError> {
        Ok(self
            .lock()?
            .runs
            .get(run.as_str())
            .map_or(0, |tally| tally.last_sequence)
            + 1)
    }

    /// Tokens this run has reserved or been charged: its persisted ledger.
    pub fn spent(&self, run: &RunId) -> Result<u64, CallRecordError> {
        Ok(self
            .lock()?
            .runs
            .get(run.as_str())
            .and_then(|tally| tally.ledger.as_ref())
            .map_or(0, |ledger| ledger.spent))
    }

    /// Takes the run's ledger for one port, or refuses when another holds it.
    /// The first claim of a run writes its ledger: the cap it is opened under
    /// and the world it starts from. A later claim must name the same cap; the
    /// start is not compared, because a resumed world has moved on.
    fn claim(&self, run: &RunId, cap: u64, start: &RunStart) -> Result<(), CallRecordError> {
        let mut state = self.lock()?;
        if !state.owned.insert(run.as_str().to_owned()) {
            return Err(CallRecordError::RunInUse);
        }
        let begun = match state.runs.get(run.as_str()).and_then(|t| t.ledger.as_ref()) {
            Some(ledger) if ledger.cap == cap => Ok(()),
            Some(_) => Err(CallRecordError::RunMismatch(RunField::Cap)),
            None => Self::write_ledger(
                &mut state,
                RunLedger {
                    run_id: run.as_str().to_owned(),
                    cap,
                    start: start.clone(),
                    spent: 0,
                },
            ),
        };
        if begun.is_err() {
            state.owned.remove(run.as_str());
        }
        begun
    }

    /// The world a run began from, when it has begun.
    fn start_of(&self, run: &RunId) -> Result<Option<RunStart>, CallRecordError> {
        Ok(self
            .lock()?
            .runs
            .get(run.as_str())
            .and_then(|tally| tally.ledger.as_ref())
            .map(|ledger| ledger.start.clone()))
    }

    fn disown(&self, run: &RunId) {
        if let Ok(mut state) = self.state.lock() {
            state.owned.remove(run.as_str());
        }
    }

    /// Persists the ledger, then adopts it in memory. A write that fails
    /// leaves the previous ledger.
    fn write_ledger(state: &mut StoreState, ledger: RunLedger) -> Result<(), CallRecordError> {
        let row = CultCacheEnvelope {
            key: ledger.run_id.clone(),
            r#type: RUN_LEDGER_ROW.into(),
            payload: rmp_serde::to_vec_named(&ledger).map_err(|_| CallRecordError::Store)?,
            stored_at: Utc::now().to_rfc3339(),
            schema_id: Some(RUN_LEDGER_SCHEMA.into()),
        };
        state.store.push(&row).map_err(|_| CallRecordError::Store)?;
        let run = ledger.run_id.clone();
        state.runs.entry(run).or_default().ledger = Some(ledger);
        Ok(())
    }

    /// Persists `spent` as the run's spend; the run must have been claimed.
    fn write_spent(state: &mut StoreState, run: &str, spent: u64) -> Result<(), CallRecordError> {
        let mut ledger = state
            .runs
            .get(run)
            .and_then(|tally| tally.ledger.clone())
            .ok_or(CallRecordError::Store)?;
        ledger.spent = spent;
        Self::write_ledger(state, ledger)
    }

    fn held(state: &StoreState, run: &RunId) -> u64 {
        state
            .runs
            .get(run.as_str())
            .and_then(|tally| tally.ledger.as_ref())
            .map_or(0, |ledger| ledger.spent)
    }

    /// Charges `bound` to the run's persisted ledger, or refuses with
    /// `OverCap` when the ledger plus the bound would pass `cap`. The check and
    /// the write hold one lock, so calls on the wire hold their bounds.
    fn reserve(&self, run: &RunId, bound: u64, cap: u64) -> Result<(), CallRecordError> {
        let mut state = self.lock()?;
        let after = Self::held(&state, run).saturating_add(bound);
        if after > cap {
            return Err(CallRecordError::OverCap);
        }
        Self::write_spent(&mut state, run.as_str(), after)
    }

    /// Replaces a reservation of `bound` with what the call was charged. With
    /// a record, the record is stored first, so a crash between the two writes
    /// leaves the larger reservation charged.
    fn settle(
        &self,
        run: &RunId,
        bound: u64,
        charged: u64,
        record: Option<&ProviderCallRecord>,
    ) -> Result<(), CallRecordError> {
        let mut state = self.lock()?;
        if let Some(record) = record {
            Self::push_record(&mut state, record)?;
        }
        let settled = Self::held(&state, run)
            .saturating_sub(bound)
            .saturating_add(charged);
        Self::write_spent(&mut state, run.as_str(), settled)
    }

    fn append(&self, record: &ProviderCallRecord) -> Result<(), CallRecordError> {
        let mut state = self.lock()?;
        Self::push_record(&mut state, record)
    }

    fn push_record(
        state: &mut StoreState,
        record: &ProviderCallRecord,
    ) -> Result<(), CallRecordError> {
        let last = state
            .runs
            .get(&record.run_id)
            .map_or(0, |tally| tally.last_sequence);
        if record.sequence != last + 1 {
            return Err(CallRecordError::OutOfSequence);
        }
        let row = CultCacheEnvelope {
            key: row_key(&record.run_id, record.sequence),
            r#type: CALL_RECORD_ROW.into(),
            payload: rmp_serde::to_vec_named(record).map_err(|_| CallRecordError::Store)?,
            stored_at: Utc::now().to_rfc3339(),
            schema_id: Some(CALL_RECORD_SCHEMA.into()),
        };
        state.store.push(&row).map_err(|_| CallRecordError::Store)?;
        state
            .runs
            .entry(record.run_id.clone())
            .or_default()
            .last_sequence = record.sequence;
        Ok(())
    }

    /// The run's records in sequence.
    pub fn records(&self, run: &RunId) -> Result<Vec<ProviderCallRecord>, CallRecordError> {
        let prefix = format!("{}/", run.as_str());
        let rows = self
            .lock()?
            .store
            .pull_all()
            .map_err(|_| CallRecordError::Store)?;
        let mut records = Vec::new();
        for row in rows.iter().filter(|row| row.key.starts_with(&prefix)) {
            if let DecodedRow::Call(record) = decode_row(row)? {
                records.push(record);
            }
        }
        records.sort_by_key(|record| record.sequence);
        Ok(records)
    }
}

/// Decorates a port for one run: reserves each call's bound in the run's
/// ledger before the provider is called, settles it to the real cost, and
/// stores every reply before returning it.
pub struct RecordingInferencePort {
    inner: Arc<dyn InferencePort>,
    store: Arc<CallRecordStore>,
    run: RunId,
    cap: u64,
}

impl RecordingInferencePort {
    /// Takes ownership of the run's ledger, which resumes from the store. The
    /// first open of a run records its cap and the world it starts from; a
    /// reopen with another cap is refused with `RunMismatch(Cap)`. A second
    /// port on a run whose first is alive is refused with `RunInUse`.
    pub fn open(
        inner: Arc<dyn InferencePort>,
        store: Arc<CallRecordStore>,
        run: RunId,
        cap_tokens: u64,
        start: &RunStart,
    ) -> Result<Self, CallRecordError> {
        store.claim(&run, cap_tokens, start)?;
        Ok(Self {
            inner,
            store,
            run,
            cap: cap_tokens,
        })
    }

    /// Tokens reserved or charged to the run so far.
    pub fn spent(&self) -> u64 {
        self.store.spent(&self.run).unwrap_or(u64::MAX)
    }

    /// The most this request can cost: its serialized bytes plus its output
    /// allowance. A request with no output allowance has no bound, so it
    /// cannot be held under a cap.
    fn bound(request: &PreparedInference) -> Result<u64, InferenceFault> {
        let allowance = request
            .invocation
            .request
            .max_output_tokens
            .ok_or_else(InferenceFault::budget_exhausted)?;
        let payload = serde_json::to_vec(&request.invocation.request)
            .map_err(|_| InferenceFault::new("a provider request could not be measured"))?;
        Ok((payload.len() as u64).saturating_add(u64::from(allowance)))
    }

    fn settle(
        &self,
        bound: u64,
        identity: (InferencePurpose, String, String),
        result: Result<InferenceOutput, InferenceFault>,
    ) -> Result<InferenceOutput, InferenceFault> {
        // A fault leaves its reservation charged: the request may have run.
        let output = result?;
        let charged = match output.usage {
            Some(usage) => usage.prompt.saturating_add(usage.completion),
            None => bound,
        };
        if charged > bound {
            self.store
                .settle(&self.run, bound, charged, None)
                .map_err(unrecorded)?;
            return Err(InferenceFault::budget_exhausted());
        }
        let (purpose, model, content_sha256) = identity;
        let record = ProviderCallRecord {
            run_id: self.run.as_str().to_owned(),
            sequence: self.store.next_sequence(&self.run).map_err(unrecorded)?,
            purpose,
            model,
            content_sha256,
            events: output.events.clone(),
            receipt_digest: output.receipt_digest.clone(),
            usage: output.usage,
            charged,
        };
        self.store
            .settle(&self.run, bound, charged, Some(&record))
            .map_err(unrecorded)?;
        Ok(output)
    }
}

impl Drop for RecordingInferencePort {
    fn drop(&mut self) {
        self.store.disown(&self.run);
    }
}

fn unrecorded(_: CallRecordError) -> InferenceFault {
    InferenceFault::integrity_violation("a provider call could not be recorded")
}

#[async_trait]
impl InferencePort for RecordingInferencePort {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        self.inner.prepare(request)
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        let bound = Self::bound(&request)?;
        let identity = (
            request.purpose,
            request.invocation.request.model.clone(),
            content_digest(&request.invocation.request)?,
        );
        self.store
            .reserve(&self.run, bound, self.cap)
            .map_err(|error| match error {
                CallRecordError::OverCap => InferenceFault::budget_exhausted(),
                _ => unrecorded(error),
            })?;
        let result = self.inner.infer(request).await;
        self.settle(bound, identity, result)
    }

    fn lend_tool_results(&self, prepared: &PreparedInference, oracle: Box<dyn ToolResultOracle>) {
        self.inner.lend_tool_results(prepared, oracle);
    }
}

/// Serves a run's stored replies, in sequence, to the same requests, matched
/// by what they ask of the provider and not by the ids that name them. It holds
/// no provider and makes no call: a request that is not the next recorded one
/// is a fault.
pub struct ReplayInferencePort {
    caller_runtime_id: String,
    pending: Mutex<VecDeque<ProviderCallRecord>>,
}

impl ReplayInferencePort {
    /// Replays `run` from the store against `world`, a restored copy of the
    /// world the run started from. Any other world is refused here, before a
    /// call, with `RunMismatch` naming the field; a run the store never began
    /// is `NotReplayable`.
    pub fn new(
        caller_runtime_id: impl Into<String>,
        store: &CallRecordStore,
        run: &RunId,
        world: &RunStart,
    ) -> Result<Self, CallRecordError> {
        let began = store.start_of(run)?.ok_or(CallRecordError::NotReplayable)?;
        if began.world_id != world.world_id {
            return Err(CallRecordError::RunMismatch(RunField::WorldId));
        }
        if began.state_digest != world.state_digest {
            return Err(CallRecordError::RunMismatch(RunField::StateDigest));
        }
        Ok(Self {
            caller_runtime_id: caller_runtime_id.into(),
            pending: Mutex::new(store.records(run)?.into()),
        })
    }
}

#[async_trait]
impl InferencePort for ReplayInferencePort {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        PreparedInference::prepare(
            &self.caller_runtime_id,
            unix_ms()?.saturating_add(REQUEST_EXPIRY.as_millis() as u64),
            request,
        )
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| InferenceFault::integrity_violation("the replay queue was poisoned"))?;
        let digest = content_digest(&request.invocation.request)?;
        let Some(next) = pending.front() else {
            return Err(InferenceFault::integrity_violation(
                "replay has no recorded call left",
            ));
        };
        if next.content_sha256 != digest {
            return Err(InferenceFault::integrity_violation(
                "the request is not the next recorded call",
            ));
        }
        let output = next.output();
        pending.pop_front();
        Ok(output)
    }
}

#[cfg(test)]
mod tests;

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
//! The bound is exact for a port that makes one provider round per call (the
//! hosted and local lanes). A port that loops tools inside one call can spend
//! more than one round's bound; its settled usage is still charged, so the next
//! call is refused, but this decorator cannot stop the overshoot of that call.
//!
//! Charging rule, one path: a reply that reports usage is charged prompt plus
//! completion (cached prompt tokens are part of the prompt); a reply that
//! reports none is charged its bound; a fault is charged its bound, except a
//! connect fault, which never reached the provider and costs nothing.

use crate::controllers::{
    InferenceEvent, InferenceFault, InferenceFaultClass, InferenceOutput, InferencePort,
    InferencePurpose, InferenceRequest, PreparedInference, REQUEST_EXPIRY, TokenUsage,
    ToolResultOracle, unix_ms,
};
use async_trait::async_trait;
use chrono::Utc;
use cultcache_rs::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    path::Path,
    sync::{Arc, Mutex},
};
use thiserror::Error;

const CALL_RECORD_ROW: &str = "verse_provider_call.v1";
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

/// One provider call of one run: the request's digest (never its text, so no
/// prompt or credential is stored), the reply exactly as the lane received it,
/// and what the call was charged. Keyed (run id, sequence).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCallRecord {
    run_id: String,
    sequence: u64,
    purpose: InferencePurpose,
    model: String,
    request_sha256: String,
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

    /// Lowercase hex of the provider request's digest.
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
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

fn hex(digest: &[u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_row(row: &CultCacheEnvelope) -> Result<ProviderCallRecord, CallRecordError> {
    if row.r#type != CALL_RECORD_ROW || row.schema_id.as_deref() != Some(CALL_RECORD_SCHEMA) {
        return Err(CallRecordError::Corrupt);
    }
    let record: ProviderCallRecord =
        rmp_serde::from_slice(&row.payload).map_err(|_| CallRecordError::Corrupt)?;
    let canonical = rmp_serde::to_vec_named(&record).map_err(|_| CallRecordError::Corrupt)?;
    if canonical != row.payload
        || RunId::new(&record.run_id).is_err()
        || record.sequence == 0
        || row.key != row_key(&record.run_id, record.sequence)
    {
        return Err(CallRecordError::Corrupt);
    }
    Ok(record)
}

#[derive(Default)]
struct RunTally {
    last_sequence: u64,
    count: u64,
    charged: u64,
}

struct StoreState {
    store: OwnedRedbMessagePackBackingStore,
    runs: BTreeMap<String, RunTally>,
}

/// The call records of every run in one CultCache file. Records are appended
/// and never changed; a run's sequence is dense from 1.
pub struct CallRecordStore {
    state: Mutex<StoreState>,
}

impl CallRecordStore {
    /// Opens the file, creating it empty when absent. Every row must be a
    /// canonical call record of this schema with its key and a dense sequence;
    /// anything else, an earlier or later schema version included, is refused
    /// and nothing is migrated.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CallRecordError> {
        let store = OwnedRedbMessagePackBackingStore::new(path.as_ref())
            .map_err(|_| CallRecordError::Store)?;
        store
            .validate_path_identity()
            .map_err(|_| CallRecordError::Store)?;
        let rows = store.pull_all().map_err(|_| CallRecordError::Store)?;
        let mut runs: BTreeMap<String, RunTally> = BTreeMap::new();
        for row in &rows {
            let record = decode_row(row)?;
            let tally = runs.entry(record.run_id).or_default();
            tally.last_sequence = tally.last_sequence.max(record.sequence);
            tally.count += 1;
            tally.charged = tally.charged.saturating_add(record.charged);
        }
        if runs.values().any(|tally| tally.count != tally.last_sequence) {
            return Err(CallRecordError::Corrupt);
        }
        Ok(Self {
            state: Mutex::new(StoreState { store, runs }),
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

    /// Tokens this run's stored calls were charged.
    pub fn charged(&self, run: &RunId) -> Result<u64, CallRecordError> {
        Ok(self
            .lock()?
            .runs
            .get(run.as_str())
            .map_or(0, |tally| tally.charged))
    }

    fn append(&self, record: &ProviderCallRecord) -> Result<(), CallRecordError> {
        let mut state = self.lock()?;
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
        let tally = state.runs.entry(record.run_id.clone()).or_default();
        tally.last_sequence = record.sequence;
        tally.count += 1;
        tally.charged = tally.charged.saturating_add(record.charged);
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
            records.push(decode_row(row)?);
        }
        records.sort_by_key(|record| record.sequence);
        Ok(records)
    }
}

struct Ledger {
    /// Tokens charged by finished calls, restored from the store at open.
    settled: u64,
    /// Bounds held by calls still on the wire.
    in_flight: u64,
}

/// Decorates a port for one run: refuses a call that could pass the cap,
/// charges the real cost, and stores every reply before returning it.
pub struct RecordingInferencePort {
    inner: Arc<dyn InferencePort>,
    store: Arc<CallRecordStore>,
    run: RunId,
    cap: u64,
    ledger: Mutex<Ledger>,
}

impl RecordingInferencePort {
    /// The run's spend resumes from its stored records.
    pub fn open(
        inner: Arc<dyn InferencePort>,
        store: Arc<CallRecordStore>,
        run: RunId,
        cap_tokens: u64,
    ) -> Result<Self, CallRecordError> {
        let settled = store.charged(&run)?;
        Ok(Self {
            inner,
            store,
            run,
            cap: cap_tokens,
            ledger: Mutex::new(Ledger {
                settled,
                in_flight: 0,
            }),
        })
    }

    /// Tokens charged to the run so far.
    pub fn spent(&self) -> u64 {
        self.ledger.lock().map_or(u64::MAX, |ledger| ledger.settled)
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

    fn reserve(&self, bound: u64) -> Result<(), InferenceFault> {
        let mut ledger = self
            .ledger
            .lock()
            .map_err(|_| InferenceFault::integrity_violation("the run's ledger was poisoned"))?;
        let held = ledger.settled.saturating_add(ledger.in_flight);
        if held.saturating_add(bound) > self.cap {
            return Err(InferenceFault::budget_exhausted());
        }
        ledger.in_flight += bound;
        Ok(())
    }

    fn settle(
        &self,
        bound: u64,
        identity: (InferencePurpose, String, String),
        result: Result<InferenceOutput, InferenceFault>,
    ) -> Result<InferenceOutput, InferenceFault> {
        let mut ledger = self
            .ledger
            .lock()
            .map_err(|_| InferenceFault::integrity_violation("the run's ledger was poisoned"))?;
        ledger.in_flight = ledger.in_flight.saturating_sub(bound);
        let output = match result {
            Ok(output) => output,
            Err(fault) => {
                if fault.class() != InferenceFaultClass::Connect {
                    ledger.settled = ledger.settled.saturating_add(bound);
                }
                return Err(fault);
            }
        };
        let charged = match output.usage {
            Some(usage) => usage.prompt.saturating_add(usage.completion),
            None => bound,
        };
        ledger.settled = ledger.settled.saturating_add(charged);
        let (purpose, model, request_sha256) = identity;
        let sequence = self.store.next_sequence(&self.run).map_err(unrecorded)?;
        let record = ProviderCallRecord {
            run_id: self.run.as_str().to_owned(),
            sequence,
            purpose,
            model,
            request_sha256,
            events: output.events.clone(),
            receipt_digest: output.receipt_digest.clone(),
            usage: output.usage,
            charged,
        };
        self.store.append(&record).map_err(unrecorded)?;
        Ok(output)
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
        self.reserve(bound)?;
        let identity = (
            request.purpose,
            request.invocation.request.model.clone(),
            hex(&request.invocation.provider_request_sha256),
        );
        let result = self.inner.infer(request).await;
        self.settle(bound, identity, result)
    }

    fn lend_tool_results(&self, prepared: &PreparedInference, oracle: Box<dyn ToolResultOracle>) {
        self.inner.lend_tool_results(prepared, oracle);
    }
}

/// Serves a run's stored replies, in sequence, to the same requests. It holds
/// no provider and makes no call: a request that is not the next recorded one
/// is a fault.
pub struct ReplayInferencePort {
    caller_runtime_id: String,
    pending: Mutex<VecDeque<ProviderCallRecord>>,
}

impl ReplayInferencePort {
    /// `records` are one run's, as `CallRecordStore::records` returns them.
    pub fn new(
        caller_runtime_id: impl Into<String>,
        records: Vec<ProviderCallRecord>,
    ) -> Result<Self, CallRecordError> {
        let dense = records.iter().enumerate().all(|(index, record)| {
            record.sequence == index as u64 + 1 && record.run_id == records[0].run_id
        });
        if !dense {
            return Err(CallRecordError::NotReplayable);
        }
        Ok(Self {
            caller_runtime_id: caller_runtime_id.into(),
            pending: Mutex::new(records.into()),
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
        let digest = hex(&request.invocation.provider_request_sha256);
        let Some(next) = pending.front() else {
            return Err(InferenceFault::integrity_violation(
                "replay has no recorded call left",
            ));
        };
        if next.request_sha256 != digest {
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

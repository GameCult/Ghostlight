//! The Aetheria verse run's own organs. The verse binary sits above this
//! module; here sit the parts of a run that belong to the library: the record
//! of every provider call and the spend it implies, the genesis of the
//! canonical verse world, and the manual run that advances it with its record.

mod calls;
mod genesis;
mod run;

pub use genesis::{CreateVerseWorldIntent, VerseGenesisError};
pub use calls::{
    CALL_RECORD_SCHEMA, CallRecordError, CallRecordStore, ProviderCallRecord,
    RecordingInferencePort, ReplayInferencePort, RunField, RunId, RunStart,
};
pub use run::{
    RUN_RECORD_SCHEMA, RecordedCoverBudget, Refusal, RefusalReason, RunClaim, RunOutcome, RunStatus,
    TickCheckpoint, TickReport, VerseRunConfig, VerseRunError, VerseRunRecord, VerseRunStore,
    call_run_id, run_manual,
};

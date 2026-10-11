//! The Aetheria verse run's own organs. The verse binary and its run record
//! live above this module; here sit the parts of a run that belong to the
//! library: the record of every provider call and the spend it implies, and the
//! genesis of the canonical verse world.

mod calls;
mod genesis;

pub use genesis::{CreateVerseWorldIntent, VerseGenesisError};
pub use calls::{
    CALL_RECORD_SCHEMA, CallRecordError, CallRecordStore, ProviderCallRecord,
    RecordingInferencePort, ReplayInferencePort, RunField, RunId, RunStart,
};

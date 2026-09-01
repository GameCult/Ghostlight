#![recursion_limit = "256"]

pub mod agency_corpus;
pub mod agent;
pub mod assessor;
pub mod clock;
pub mod compiler;
pub mod consumer;
pub mod d20;
pub mod domain;
pub mod elaboration;
pub mod follow_through;
pub mod gestalt;
pub mod idunn_health;
pub mod initiative;
pub mod kernel;
pub mod legacy_transition;
pub mod mesh;
pub mod model;
pub mod model_connector;
pub mod model_runtime;
pub mod newspaper;
pub mod outcome;
pub mod persistence;
pub mod persona;
pub mod registry;
pub mod resolution;
pub mod scheduler;
pub mod session_zero;
pub mod surface;
pub mod transition;
pub mod turn;
pub mod vault;
#[cfg(windows)]
#[cfg(windows)]
pub mod windows_secret;
mod world;

pub use kernel::{KernelError, WorldKernel};

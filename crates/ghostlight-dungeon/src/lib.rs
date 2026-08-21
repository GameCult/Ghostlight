#![recursion_limit = "256"]

pub mod assessor;
pub mod compiler;
pub mod d20;
pub mod domain;
pub mod gestalt;
pub mod idunn_health;
pub mod initiative;
pub mod kernel;
pub mod mesh;
pub mod model;
pub mod narrator;
pub mod outcome;
pub mod persistence;
pub mod persona;
pub mod registry;
pub mod resolution;
pub mod scheduler;
pub mod session_zero;
pub mod surface;
pub mod turn;
pub mod vault;
#[cfg(windows)]
pub mod windows_secret;

pub use kernel::{KernelError, WorldKernel};

//! Deterministic simulation primitives without window, GPU, filesystem or wall-clock access.

#![forbid(unsafe_code)]

pub mod docking;
mod flight;
pub mod freight;

pub use flight::{FIXED_DELTA_SECONDS, FlightInput, Snapshot, StepError, World};

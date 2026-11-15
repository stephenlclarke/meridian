//! Deterministic simulation primitives without window, GPU, filesystem or wall-clock access.

#![forbid(unsafe_code)]

pub mod docking;
mod flight;
pub mod freight;
pub mod navigation;
pub mod station;

pub use flight::{FIXED_DELTA_SECONDS, FlightInput, Snapshot, StepError, World};
pub use station::{FlightMode, FlightStart, MotionRefusal, OriginalStationData, StationError};

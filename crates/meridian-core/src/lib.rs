//! Deterministic simulation primitives without window, GPU, filesystem or wall-clock access.

#![forbid(unsafe_code)]

pub mod docking;
mod flight;
pub mod freight;
pub mod navigation;
pub mod station;
pub mod station_services;

pub use flight::{
    FIXED_DELTA_SECONDS, FlightInput, MissionError, MissionSnapshot, ServiceSnapshot, Snapshot,
    StationNpc, StepError, World,
};
pub use station::{FlightMode, FlightStart, MotionRefusal, OriginalStationData, StationError};

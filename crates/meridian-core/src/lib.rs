//! Deterministic simulation primitives without window, GPU, filesystem or wall-clock access.

#![forbid(unsafe_code)]

pub mod docking;
pub mod equipment;
mod flight;
pub mod freight;
pub mod navigation;
pub mod save;
pub mod station;
pub mod station_services;

pub use flight::{
    FIXED_DELTA_SECONDS, FireError, FlightInput, MissionError, MissionSnapshot,
    PULSE_COOLDOWN_TICKS, PULSE_RANGE, PulseContact, PulseLaserSnapshot, PulseShot,
    ServiceSnapshot, Snapshot, StationNpc, StepError, World,
};
pub use station::{FlightMode, FlightStart, MotionRefusal, OriginalStationData, StationError};

pub use flight::{
    PRACTICE_HALF_SIDE, PracticeBeaconId, PracticeBeaconSnapshot, PracticeError, PracticeReview,
    PracticeSnapshot, PracticeStatus, PreparedPracticeConfirmation,
};

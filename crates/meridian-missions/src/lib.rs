//! Deterministic, explicit-state Lua proposals. Only existing World transactions grant gameplay.
#![forbid(unsafe_code)]

mod adapter;
mod controller;
mod evaluator;
mod pack;
mod tagged;
mod value;

pub use adapter::{
    MAX_FLIGHT_PUBLICATION_BYTES, PreparedFlightAcceptance, PreparedFlightTransition,
    prepare_flight_acceptance, prepare_flight_transition, restore_flight_publication,
    serialize_flight_publication,
};
pub use controller::{Action, Event, Instance, PendingTransition, Status};
pub use evaluator::{Evaluation, Limits, evaluate, validate};
pub use pack::{Identity, Manifest, Pack};
pub use tagged::decode_tagged;
pub use value::{Value, decode, encode};

/// Provenance assigned only at the real Lua compiler or protected function-call boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuaFailureStage {
    Compilation,
    Execution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Limit(&'static str),
    Codec(&'static str),
    Pack(&'static str),
    Lua(String),
    UnsupportedOperation(String),
    Identity,
    Replay,
    Suspended,
    Stale,
    World(String),
    Publish(String),
    LuaFailure {
        stage: LuaFailureStage,
        cause: Box<Error>,
    },
    /// Invocation diagnostics also cover compiler/conversion failures; these metrics alone do
    /// not identify a Lua execution failure. Use `lua_failure_stage` for typed provenance.
    Execution {
        cause: Box<Error>,
        instructions: u64,
        peak_heap_bytes: usize,
        allocation_bytes: usize,
    },
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Limit(text) | Self::Codec(text) | Self::Pack(text) => f.write_str(text),
            Self::Lua(text) => f.write_str(text),
            Self::LuaFailure { cause, .. } | Self::Execution { cause, .. } => write!(f, "{cause}"),
            other => write!(f, "mission controller: {other:?}"),
        }
    }
}
impl std::error::Error for Error {}

impl Error {
    /// Preflight, VM setup, host validation and value-conversion errors have no Lua failure stage.
    pub fn lua_failure_stage(&self) -> Option<LuaFailureStage> {
        match self {
            Self::LuaFailure { stage, .. } => Some(*stage),
            Self::Execution { cause, .. } => cause.lua_failure_stage(),
            _ => None,
        }
    }

    pub fn instruction_count(&self) -> Option<u64> {
        match self {
            Self::Execution { instructions, .. } => Some(*instructions),
            _ => None,
        }
    }
}

impl From<mlua::Error> for Error {
    fn from(value: mlua::Error) -> Self {
        if let Some(error) = value.downcast_ref::<Self>() {
            return error.clone();
        }
        let text = value.to_string();
        let mut end = text.len().min(240);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        Self::Lua(text[..end].to_owned())
    }
}

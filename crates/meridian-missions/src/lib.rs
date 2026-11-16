//! Deterministic, explicit-state Lua proposals. Only existing World transactions grant gameplay.
#![forbid(unsafe_code)]

mod adapter;
mod controller;
mod evaluator;
mod pack;
mod tagged;
mod value;

pub use adapter::{
    PreparedFlightAcceptance, prepare_flight_acceptance, restore_flight_publication,
};
pub use controller::{Action, Event, Instance, PendingTransition, Status};
pub use evaluator::{Evaluation, Limits, evaluate, validate};
pub use pack::{Identity, Manifest, Pack};
pub use tagged::decode_tagged;
pub use value::{Value, decode, encode};

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
            Self::Execution { cause, .. } => write!(f, "{cause}"),
            other => write!(f, "mission controller: {other:?}"),
        }
    }
}
impl std::error::Error for Error {}

impl Error {
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

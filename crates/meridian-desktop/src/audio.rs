//! Presentation-only generated audio. The host supplies actual accepted events.
//! No World, save data, window, or gameplay authority is retained here.
mod output;
mod presenter;
mod synth;

pub use output::NativeOutput;
pub use presenter::{
    AudioFrame, AudioOutput, AudioPresenter, Discharge, DischargeJournal, DischargeKind,
    OutputStatus,
};
pub use synth::{SAMPLE_RATE, SoundEffect};

#[cfg(test)]
mod tests;

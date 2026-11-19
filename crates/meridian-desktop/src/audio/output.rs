use std::num::{NonZeroU16, NonZeroU32};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, buffer::SamplesBuffer};

use super::{AudioOutput, SAMPLE_RATE, SoundEffect};

/// Keeps the device alive on the desktop owner. The callback only sets a flag;
/// it cannot access simulation state or resurrect playback after shutdown.
pub struct NativeOutput {
    device: Option<MixerDeviceSink>,
    players: Vec<Player>,
    tool: Option<Player>,
    failed: Arc<AtomicBool>,
    unavailable_reason: Option<String>,
    samples: [SamplesBuffer; 6],
}

impl NativeOutput {
    /// Failure is a quiet presentation condition, not a gameplay error.
    /// This is the only point which opens a device; no implicit reconnect.
    pub fn open() -> Self {
        let failed = Arc::new(AtomicBool::new(false));
        let callback_flag = Arc::clone(&failed);
        let opened = DeviceSinkBuilder::from_default_device().and_then(|builder| {
            builder
                .with_error_callback(move |_| {
                    callback_flag.store(true, Ordering::Release);
                })
                .open_sink_or_fallback()
        });
        let (device, unavailable_reason) = match opened {
            Ok(mut device) => {
                device.log_on_drop(false);
                (Some(device), None)
            }
            Err(error) => (None, Some(error.to_string())),
        };
        Self {
            device,
            players: Vec::new(),
            tool: None,
            failed,
            unavailable_reason,
            samples: [
                SoundEffect::Laser,
                SoundEffect::Launch,
                SoundEffect::Dock,
                SoundEffect::Explosion,
                SoundEffect::Ping,
                SoundEffect::Ecm,
            ]
            .map(|effect| {
                // SAMPLE_RATE is a positive compile-time constant.
                let rate = match NonZeroU32::new(SAMPLE_RATE) {
                    Some(rate) => rate,
                    None => unreachable!("the fixed audio sample rate is positive"),
                };
                let samples: Vec<f32> = effect
                    .pcm()
                    .into_iter()
                    .map(|sample| f32::from(sample) / 32768.0)
                    .collect();
                SamplesBuffer::new(NonZeroU16::MIN, rate, samples)
            }),
        }
    }

    pub fn unavailable_reason(&self) -> Option<&str> {
        if self.failed.load(Ordering::Acquire) {
            Some("audio output device became unavailable")
        } else {
            self.unavailable_reason.as_deref()
        }
    }

    fn player(&self, effect: SoundEffect) -> Option<Player> {
        if !self.available() {
            return None;
        }
        let device = self.device.as_ref()?;
        let player = Player::connect_new(device.mixer());
        player.set_volume(0.4);
        let index = match effect {
            SoundEffect::Laser => 0,
            SoundEffect::Launch => 1,
            SoundEffect::Dock => 2,
            SoundEffect::Explosion => 3,
            SoundEffect::Ping => 4,
            SoundEffect::Ecm => 5,
        };
        player.append(self.samples[index].clone());
        Some(player)
    }
}

impl AudioOutput for NativeOutput {
    fn available(&self) -> bool {
        self.device.is_some() && !self.failed.load(Ordering::Acquire)
    }

    fn maintain(&mut self) {
        if self.failed.load(Ordering::Acquire) {
            self.stop_all();
            self.device = None;
        } else {
            self.players.retain(|player| !player.empty());
            if self.tool.as_ref().is_some_and(Player::empty) {
                self.tool = None;
            }
        }
    }

    fn play(&mut self, effect: SoundEffect) {
        if let Some(player) = self.player(effect) {
            self.players.push(player);
        }
    }

    fn continuous_tool_active(&self) -> bool {
        self.tool.as_ref().is_some_and(|player| !player.empty())
    }

    fn play_continuous_tool(&mut self) {
        if !self.continuous_tool_active() {
            self.tool = self.player(SoundEffect::Laser);
        }
    }

    fn stop_all(&mut self) {
        for player in self.players.drain(..) {
            player.stop();
        }
        if let Some(player) = self.tool.take() {
            player.stop();
        }
    }

    fn shutdown(&mut self) {
        self.stop_all();
        self.device = None;
    }
}

impl Drop for NativeOutput {
    fn drop(&mut self) {
        self.shutdown();
    }
}

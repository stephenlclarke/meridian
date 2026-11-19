use super::SoundEffect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStatus {
    Ready,
    Muted,
    Unavailable,
    Shutdown,
}

/// A small playback boundary, also used by deterministic lifecycle tests.
pub trait AudioOutput {
    fn available(&self) -> bool;
    fn maintain(&mut self);
    fn play(&mut self, effect: SoundEffect);
    fn continuous_tool_active(&self) -> bool;
    fn play_continuous_tool(&mut self);
    fn stop_all(&mut self);
    fn shutdown(&mut self) {
        self.stop_all();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DischargeKind {
    OneShot,
    /// Original cutter semantics: one short laser voice at a time, not a loop.
    ContinuousTool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Discharge {
    pub id: u64,
    pub kind: DischargeKind,
}

/// Host-owned presentation ring. Record only an accepted real discharge.
/// On exhaustion the host must begin a new epoch; IDs never wrap or alias.
#[derive(Default)]
pub struct DischargeJournal {
    events: Vec<Discharge>,
    next_id: u64,
}

impl DischargeJournal {
    pub fn record(&mut self, kind: DischargeKind) -> bool {
        let Some(id) = self.next_id.checked_add(1) else {
            return false;
        };
        self.next_id = id;
        if self.events.len() == 32 {
            self.events.remove(0);
        }
        self.events.push(Discharge { id, kind });
        true
    }

    pub fn events(&self) -> &[Discharge] {
        &self.events
    }
}

/// Host presentation epoch increases only after successful New or Continue.
/// Pass the current snapshot to begin_session before accepting live events.
/// IDs come from accepted discharges, never held input or visible beam TTL.
pub struct AudioFrame<'a> {
    pub epoch: u64,
    /// Host revision increases whenever presentation facts change.
    pub revision: u64,
    pub docked: bool,
    pub discharges: &'a [Discharge],
    pub message: &'a str,
    pub message_live: bool,
}

pub struct AudioPresenter<O: AudioOutput> {
    output: O,
    epoch: Option<u64>,
    previous_docked: bool,
    revision: u64,
    discharge_id: u64,
    previous_message: String,
    muted: bool,
    shutdown: bool,
}

impl<O: AudioOutput> AudioPresenter<O> {
    pub fn new(output: O) -> Self {
        Self {
            output,
            epoch: None,
            previous_docked: true,
            revision: 0,
            discharge_id: 0,
            previous_message: String::new(),
            muted: false,
            shutdown: false,
        }
    }

    /// Priming consumes restored events without making a cold-load sound.
    /// Old frames cannot switch the current session back to a prior epoch.
    pub fn begin_session(&mut self, frame: &AudioFrame<'_>) {
        if self.shutdown || self.epoch.is_some_and(|epoch| frame.epoch <= epoch) {
            return;
        }
        self.output.stop_all();
        self.epoch = Some(frame.epoch);
        self.discharge_id = 0;
        self.consume(frame);
    }

    pub fn status(&self) -> OutputStatus {
        if self.shutdown {
            OutputStatus::Shutdown
        } else if !self.output.available() {
            OutputStatus::Unavailable
        } else if self.muted {
            OutputStatus::Muted
        } else {
            OutputStatus::Ready
        }
    }

    pub fn set_muted(&mut self, muted: bool) {
        if self.shutdown {
            return;
        }
        self.muted = muted;
        if muted {
            self.output.stop_all();
        }
    }

    pub fn update(&mut self, frame: &AudioFrame<'_>) {
        if self.shutdown || self.epoch != Some(frame.epoch) {
            return;
        }
        self.output.maintain();
        if frame.revision <= self.revision {
            return;
        }
        if self.status() == OutputStatus::Ready {
            self.play_changes(frame);
        }
        // Muted or unavailable output still consumes events, without catch-up.
        self.consume(frame);
    }

    fn play_changes(&mut self, frame: &AudioFrame<'_>) {
        for event in frame.discharges {
            if event.id <= self.discharge_id {
                continue;
            }
            // Advance inside the loop so even duplicate IDs in one frame are quiet.
            self.discharge_id = event.id;
            match event.kind {
                DischargeKind::OneShot => self.output.play(SoundEffect::Laser),
                DischargeKind::ContinuousTool => {
                    if !self.output.continuous_tool_active() {
                        self.output.play_continuous_tool();
                    }
                }
            }
        }
        if self.previous_docked != frame.docked {
            self.output.play(if frame.docked {
                SoundEffect::Dock
            } else {
                SoundEffect::Launch
            });
        }
        if self.previous_message != frame.message && frame.message_live {
            self.output.play(if frame.message == "E.C.M." {
                SoundEffect::Ecm
            } else {
                SoundEffect::Ping
            });
        }
    }

    fn consume(&mut self, frame: &AudioFrame<'_>) {
        if let Some(last_id) = frame.discharges.iter().map(|event| event.id).max() {
            self.discharge_id = self.discharge_id.max(last_id);
        }
        self.revision = frame.revision;
        self.previous_docked = frame.docked;
        if self.previous_message != frame.message {
            self.previous_message.clear();
            self.previous_message.push_str(frame.message);
        }
    }

    /// Terminal and idempotent, including updates resumed after window close.
    pub fn shutdown(&mut self) {
        if !self.shutdown {
            self.shutdown = true;
            self.output.shutdown();
        }
    }
}

impl<O: AudioOutput> Drop for AudioPresenter<O> {
    fn drop(&mut self) {
        self.shutdown();
    }
}

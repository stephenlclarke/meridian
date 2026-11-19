//! Desktop-owned presentation epochs and accepted-event cursors.
use super::{
    AudioFrame, AudioOutput, AudioPresenter, DischargeJournal, DischargeKind, OutputStatus,
};

impl AudioOutput for Box<dyn AudioOutput> {
    fn available(&self) -> bool {
        (**self).available()
    }
    fn maintain(&mut self) {
        (**self).maintain();
    }
    fn play(&mut self, effect: super::SoundEffect) {
        (**self).play(effect);
    }
    fn continuous_tool_active(&self) -> bool {
        (**self).continuous_tool_active()
    }
    fn play_continuous_tool(&mut self) {
        (**self).play_continuous_tool();
    }
    fn stop_all(&mut self) {
        (**self).stop_all();
    }
    fn shutdown(&mut self) {
        (**self).shutdown();
    }
}

pub(crate) struct DesktopAudio {
    presenter: AudioPresenter<Box<dyn AudioOutput>>,
    epoch: u64,
    revision: u64,
    docked: bool,
    missile_notice: bool,
    journal: DischargeJournal,
    muted: bool,
    suspended: bool,
}

impl DesktopAudio {
    pub(crate) fn new(output: Box<dyn AudioOutput>) -> Self {
        Self {
            presenter: AudioPresenter::new(output),
            epoch: 0,
            revision: 0,
            docked: false,
            missile_notice: false,
            journal: DischargeJournal::default(),
            muted: false,
            suspended: false,
        }
    }

    pub(crate) fn status(&self) -> OutputStatus {
        self.presenter.status()
    }

    pub(crate) fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.presenter.set_muted(self.muted);
    }

    pub(crate) fn shutdown(&mut self) {
        self.suspended = false;
        self.presenter.shutdown();
    }

    pub(crate) fn suspend(&mut self) {
        if self.status() != OutputStatus::Shutdown {
            self.suspended = true;
            self.presenter.shutdown();
        }
    }

    pub(crate) fn resume(
        &mut self,
        output: impl FnOnce() -> Box<dyn AudioOutput>,
        current: Option<(bool, bool)>,
    ) {
        if !self.suspended {
            return;
        }
        self.suspended = false;
        self.presenter = AudioPresenter::new(output());
        self.presenter.set_muted(self.muted);
        if let Some((docked, missile_notice)) = current {
            self.prime(docked, missile_notice);
        }
    }

    pub(crate) fn begin(&mut self, docked: bool) {
        self.prime(docked, false);
    }

    fn prime(&mut self, docked: bool, missile_notice: bool) {
        if self.status() == OutputStatus::Shutdown {
            return;
        }
        let Some(epoch) = self.epoch.checked_add(1) else {
            self.shutdown();
            return;
        };
        self.epoch = epoch;
        self.revision = 0;
        self.docked = docked;
        self.missile_notice = missile_notice;
        self.journal = DischargeJournal::default();
        self.presenter.begin_session(&AudioFrame {
            epoch,
            revision: 0,
            docked,
            discharges: &[],
            message: if missile_notice { "MISSILE AWAY" } else { "" },
            message_live: missile_notice,
        });
    }

    pub(crate) fn observe(&mut self, docked: bool, missile_notice: bool, fired: bool) {
        if self.epoch == 0 || self.status() == OutputStatus::Shutdown {
            return;
        }
        if self.revision == u64::MAX || (fired && !self.journal.record(DischargeKind::OneShot)) {
            // Retain the preceding facts when rotating so the current accepted edge is not lost.
            self.prime(self.docked, self.missile_notice);
            if self.status() == OutputStatus::Shutdown {
                return;
            }
            if fired {
                let recorded = self.journal.record(DischargeKind::OneShot);
                debug_assert!(recorded);
            }
        }
        if self.docked != docked || self.missile_notice != missile_notice || fired {
            self.revision += 1;
            self.docked = docked;
            self.missile_notice = missile_notice;
        }
        self.presenter.update(&AudioFrame {
            epoch: self.epoch,
            revision: self.revision,
            docked,
            discharges: self.journal.events(),
            message: if missile_notice { "MISSILE AWAY" } else { "" },
            message_live: missile_notice,
        });
    }
}

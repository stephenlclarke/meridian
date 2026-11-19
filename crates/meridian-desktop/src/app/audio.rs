use super::{App, FlightMode, KeyCode};
use crate::audio::host::DesktopAudio;
use crate::audio::{AudioOutput, NativeOutput, OutputStatus};
use std::time::Instant;

impl App {
    pub(super) fn open_audio(&mut self) {
        if self.game.is_internal_validation() {
            return;
        }
        self.audio = Some(DesktopAudio::new(native_output()));
        self.begin_audio_session();
    }

    pub(super) fn begin_audio_session(&mut self) {
        self.notice.missile = false;
        if self.game.playing()
            && let Some(audio) = &mut self.audio
        {
            audio.begin(self.game.world.snapshot().mode == FlightMode::Docked);
        }
    }

    pub(super) fn publish_audio(&mut self, fired: bool) {
        let missile = self.live_missile_notice();
        if let Some(audio) = &mut self.audio {
            audio.observe(
                self.game.world.snapshot().mode == FlightMode::Docked,
                missile,
                fired,
            );
        }
        if self.help.is_some() {
            let line = self.audio_help_line();
            if let Some(help) = &mut self.help
                && let Some(last) = help.lines.last_mut()
                && last.starts_with("Ctrl+Shift+M")
                && *last != line
            {
                *last = line;
                self.request_redraw();
            }
        }
    }

    fn live_missile_notice(&self) -> bool {
        self.notice.missile
            && self
                .notice
                .expires
                .is_some_and(|deadline| Instant::now() < deadline)
    }

    pub(super) fn route_audio_toggle(&mut self, code: KeyCode, repeat: bool) -> bool {
        if code != KeyCode::KeyM || !self.modifiers.control_key() || !self.modifiers.shift_key() {
            return false;
        }
        if !repeat {
            self.publish_audio(false);
            if let Some(audio) = &mut self.audio {
                audio.toggle_mute();
            }
            if self.help.is_some() {
                self.help = Some(self.help_panel());
            }
            self.request_redraw();
        }
        true
    }

    pub(super) fn audio_help_line(&self) -> String {
        let status = self
            .audio
            .as_ref()
            .map_or(OutputStatus::Unavailable, DesktopAudio::status);
        let state = match status {
            OutputStatus::Ready => "On",
            OutputStatus::Muted => "Muted",
            OutputStatus::Unavailable => "Unavailable",
            OutputStatus::Shutdown => "Stopped",
        };
        format!("Ctrl+Shift+M Toggle sound | Audio: {state}")
    }

    pub(super) fn shutdown_audio(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.shutdown();
        }
    }

    pub(super) fn suspend_audio(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.suspend();
        }
    }

    pub(super) fn resume_audio(&mut self) {
        self.resume_audio_with(native_output);
    }

    pub(super) fn resume_audio_with(&mut self, output: impl FnOnce() -> Box<dyn AudioOutput>) {
        let current = self.game.playing().then(|| {
            (
                self.game.world.snapshot().mode == FlightMode::Docked,
                self.live_missile_notice(),
            )
        });
        if let Some(audio) = &mut self.audio {
            audio.resume(output, current);
        }
        self.publish_audio(false);
    }
}

fn native_output() -> Box<dyn AudioOutput> {
    let output = NativeOutput::open();
    if let Some(reason) = output.unavailable_reason() {
        eprintln!("Meridian audio unavailable: {reason}");
    }
    Box::new(output)
}

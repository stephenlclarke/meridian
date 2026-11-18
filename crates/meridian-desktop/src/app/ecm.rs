//! One discrete manual key submits the actual durable ECM command; it is never auto-fire.
use super::{Action, App, KeyCode};
impl App {
    pub(super) fn activate_ecm(&mut self) {
        let before = self
            .game
            .world
            .ecm_snapshot()
            .map(|snapshot| snapshot.activations);
        self.notice = Some(match self.game.command(Action::ActivateEcm) {
            Ok(message) | Err(message) => message,
        });
        let after = self
            .game
            .world
            .ecm_snapshot()
            .map(|snapshot| snapshot.activations);
        if after > before {
            // A committed ECM cancels EMP warmup. Do not retain the host's held U input.
            self.keys.remove(&KeyCode::KeyU);
        }
    }
}

//! Occupied cutter contact requires a separate, single-use World review and saved confirmation.
use super::{App, KeyCode, player_destroyed};
use crate::help::Panel;
use meridian_core::{TargetIdentity, World, equipment::WorkTargetId, traffic::NpcId};

const READY: &str = "Occupied: confirm cut";

/// Only the current World can establish a selected disabled actor at the accepted work hold.
pub(super) fn ready_target(world: &World) -> Option<NpcId> {
    if world.automatic_work_status() == Some(READY) {
        world.selected_disabled_salvage_target()
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Choice {
    Confirm,
    Cancel,
    Consumed,
}

fn choice(code: KeyCode, repeat: bool) -> Choice {
    if repeat {
        return Choice::Consumed;
    }
    match code {
        KeyCode::Enter | KeyCode::NumpadEnter => Choice::Confirm,
        KeyCode::Escape => Choice::Cancel,
        _ => Choice::Consumed,
    }
}

impl App {
    pub(super) fn occupied_cut_panel(&self) -> Option<Panel> {
        self.occupied_cut.as_ref().map(|review| {
            let mut panel = Panel::occupied_cut(review);
            panel.lines.insert(
                0,
                format!(
                    "Target #{}: disabled occupied ship",
                    review.target().serial()
                ),
            );
            panel
        })
    }

    /// Opening the warning does not pause or advance World. The host freezes accepted ticks
    /// so the review retains its exact contact identity, time and ownership.
    pub(super) fn maybe_open_occupied_cut(&mut self) -> bool {
        if !self.simulation_active() || self.game.is_internal_validation() {
            return false;
        }
        let Some(target) = ready_target(&self.game.world) else {
            return false;
        };
        self.clear_held_input();
        let tick = self.game.world.snapshot().tick;
        match self.game.world.review_occupied_cut(target, tick) {
            Ok(review) => self.occupied_cut = Some(review),
            Err(error) => {
                self.game.world.cancel_targeting();
                self.notice.set(error.to_string());
            }
        }
        self.request_redraw();
        true
    }

    fn release_occupied_target(&mut self, target: NpcId) {
        // Do not release a different target if the original review lost its authority.
        let selected = self
            .game
            .world
            .targeting_snapshot()
            .map(|snapshot| snapshot.identity);
        let same_or_lost = match selected {
            None => true,
            Some(TargetIdentity::Ship(id) | TargetIdentity::Work(WorkTargetId::Wreck(id))) => {
                id == target
            }
            _ => false,
        };
        if same_or_lost && !player_destroyed(&self.game.world) {
            self.game.world.cancel_targeting();
        }
        self.clear_held_input();
        self.request_redraw();
    }

    pub(super) fn cancel_occupied_cut(&mut self, message: &str) -> bool {
        let Some(review) = self.occupied_cut.take() else {
            return false;
        };
        let target = review.target();
        review.cancel();
        self.release_occupied_target(target);
        self.notice.set(message.into());
        true
    }

    pub(super) fn close_stale_occupied_cut(&mut self) -> bool {
        let Some(review) = self.occupied_cut.as_ref() else {
            return false;
        };
        if ready_target(&self.game.world) == Some(review.target()) {
            return false;
        }
        self.cancel_occupied_cut("Occupied contact cancelled: target or hold lost")
    }

    pub(super) fn route_occupied_cut(&mut self, code: KeyCode, repeat: bool) -> bool {
        if self.occupied_cut.is_none() {
            return false;
        }
        if self.close_stale_occupied_cut() {
            return true;
        }
        match choice(code, repeat) {
            Choice::Confirm => {
                if let Some(review) = self.occupied_cut.take() {
                    let target = review.target();
                    let message = match self.game.confirm_occupied_cut(review) {
                        Ok(message) | Err(message) => message,
                    };
                    // One explicit confirmation means one saved contact, never a repeated
                    // warning, automatic cut or transfer on the next accepted tick.
                    self.release_occupied_target(target);
                    self.notice.set(message);
                }
            }
            Choice::Cancel => {
                self.cancel_occupied_cut("Occupied contact cancelled; target released");
            }
            Choice::Consumed => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::tests::{TestDirectory, open};
    use meridian_core::FlightInput;

    #[test]
    fn confirmation_key_policy_consumes_ordinary_controls_and_repeats() {
        for key in [KeyCode::Enter, KeyCode::NumpadEnter] {
            assert_eq!(choice(key, false), Choice::Confirm);
            assert_eq!(choice(key, true), Choice::Consumed);
        }
        assert_eq!(choice(KeyCode::Escape, false), Choice::Cancel);
        assert_eq!(choice(KeyCode::Escape, true), Choice::Consumed);
        for key in [
            KeyCode::KeyW,
            KeyCode::KeyA,
            KeyCode::KeyZ,
            KeyCode::KeyU,
            KeyCode::KeyJ,
            KeyCode::KeyP,
            KeyCode::KeyH,
            KeyCode::Tab,
            KeyCode::Delete,
            KeyCode::F5,
            KeyCode::F9,
        ] {
            assert_eq!(choice(key, false), Choice::Consumed);
            assert_eq!(choice(key, true), Choice::Consumed);
        }
    }

    #[test]
    fn ordinary_unselected_flight_does_not_create_an_occupied_review_or_contact() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(
            profile
                .new_commander(98106)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        let mut app = App::with_profile(profile);
        app.game
            .world
            .request_launch(0)
            .unwrap_or_else(|e| panic!("{e}"));
        app.game
            .world
            .step(FlightInput::default())
            .unwrap_or_else(|e| panic!("{e}"));
        let before = app
            .game
            .world
            .save_bytes()
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(ready_target(&app.game.world).is_none());
        assert!(!app.maybe_open_occupied_cut());
        assert!(!app.modal_open());
        assert!(app.occupied_cut_panel().is_none());
        assert!(!app.route_occupied_cut(KeyCode::Enter, false));
        assert!(!app.cancel_occupied_cut("cancel"));
        assert!(!app.close_stale_occupied_cut());
        assert_eq!(app.game.world.occupied_cut_contacts().count(), 0);
        assert_eq!(
            app.game
                .world
                .save_bytes()
                .unwrap_or_else(|e| panic!("{e}")),
            before
        );
    }
}

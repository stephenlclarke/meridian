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
    use crate::app::FrameAdvance;
    use crate::profile::tests::{TestDirectory, open};
    use meridian_core::{FlightInput, TargetFilter, TargetingViewport};
    use std::fs;
    use winit::{event::ElementState, keyboard::PhysicalKey};

    // Produced by the existing core disabled_native_target_with_fit fixture at its first
    // successful actual review. The retained save has no occupied-cut contact or selected lock.
    const HELD_OCCUPIED: &[u8] =
        include_bytes!("../../../meridian-core/tests/fixtures/occupied-cut-held-v17.bin");

    fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
        value.unwrap_or_else(|error| panic!("{error:?}"))
    }

    fn some<T>(value: Option<T>) -> T {
        value.unwrap_or_else(|| panic!("required actual occupied state"))
    }

    fn press(app: &mut App, code: KeyCode, repeat: bool) -> Option<KeyCode> {
        app.route_keyboard(PhysicalKey::Code(code), ElementState::Pressed, repeat, None)
    }

    fn ready_app(directory: &TestDirectory, filter: TargetFilter) -> (App, NpcId) {
        assert_eq!(HELD_OCCUPIED.len(), 39_168);
        ok(fs::write(
            directory.0.join("commander.meridian"),
            HELD_OCCUPIED,
        ));
        let mut profile = open(&directory.0);
        ok(profile.continue_saved());
        let mut app = App::with_profile(profile);
        assert_eq!(app.game.world.snapshot().tick, 34_515);
        let target = some(
            some(app.game.world.traffic_snapshot())
                .actors
                .into_iter()
                .find(|actor| actor.electrical_disabled && actor.emp_crew_aboard == Some(1)),
        )
        .id;
        assert_eq!(target.profile(), 16_101);
        assert_eq!(target.system(), meridian_core::navigation::SystemId::RENAS);
        assert_eq!(target.serial(), 2);
        assert_eq!(some(app.game.world.fitted_salvage_cutter()).id, 116_114);
        assert_eq!(app.game.world.occupied_cut_contacts().count(), 0);
        assert_eq!(app.game.world.target_filter(), TargetFilter::Ships);
        if matches!(filter, TargetFilter::Salvage | TargetFilter::All) {
            assert_eq!(
                ok(app.game.world.cycle_target_filter()),
                TargetFilter::Mining
            );
            assert_eq!(
                ok(app.game.world.cycle_target_filter()),
                TargetFilter::Salvage
            );
            if filter == TargetFilter::All {
                assert_eq!(ok(app.game.world.cycle_target_filter()), TargetFilter::All);
            }
        } else {
            assert_eq!(filter, TargetFilter::Ships);
        }
        // Supply only the actual actor ID to the public viewport contract.
        // A headless host route does not qualify native clipping or keyboard delivery.
        let viewport = TargetingViewport {
            size: [1280, 720],
            front: vec![target],
        };
        let hint = [WorkTargetId::Wreck(target)];
        for _ in 0..4 {
            assert!(ok(app.game.world.cycle_targeting_with_work(
                app.game.world.snapshot().tick,
                viewport.clone(),
                &hint
            )));
            if app.game.world.selected_disabled_salvage_target() == Some(target) {
                break;
            }
        }
        assert_eq!(
            app.game.world.selected_disabled_salvage_target(),
            Some(target)
        );
        let before = app.game.world.snapshot().tick;
        // A cold load has no previous accepted local motion witness. The
        // following real step supplies it before the game opens the review.
        assert!(app.game.world.review_occupied_cut(target, before).is_err());
        // Existing physical hold means the next accepted tick is sufficient; a catch-up
        // frame must stop there, before any extra simulation time or held weapon input.
        assert!(matches!(
            app.advance_frame(true, 8),
            FrameAdvance::Continue(_)
        ));
        assert!(app.modal_open(), "occupied hold did not open confirmation");
        assert!(app.game.world.snapshot().tick <= before + 1);
        assert_eq!(some(app.occupied_cut.as_ref()).target(), target);
        let warning = some(app.occupied_cut_panel());
        assert!(warning.title.contains("Occupied salvage"));
        assert!(
            warning
                .lines
                .iter()
                .any(|line| line.contains(&format!("Target #{}", target.serial())))
        );
        assert!(
            warning
                .lines
                .iter()
                .any(|line| line.contains("No verified"))
        );
        assert!(warning.lines.len() + 2 <= crate::help::MAX_LINES);
        assert!(
            warning
                .lines
                .iter()
                .all(|line| line.len() <= crate::help::MAX_COLUMNS)
        );
        (app, target)
    }

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

    #[test]
    fn actual_occupied_ships_warning_freezes_and_cancels_without_any_contact_or_repeat() {
        let directory = TestDirectory::new();
        let (mut app, target) = ready_app(&directory, TargetFilter::Ships);
        let slot = directory.0.join("commander.meridian");
        let opening = ok(app.game.world.save_bytes());
        let tick = app.game.world.snapshot().tick;
        for key in [
            KeyCode::KeyW,
            KeyCode::KeyA,
            KeyCode::KeyZ,
            KeyCode::KeyU,
            KeyCode::KeyJ,
            KeyCode::KeyP,
            KeyCode::KeyH,
            KeyCode::Tab,
            KeyCode::F5,
            KeyCode::F9,
        ] {
            assert_eq!(press(&mut app, key, false), None);
            assert!(app.keys.is_empty());
        }
        assert_eq!(press(&mut app, KeyCode::Enter, true), None);
        assert!(matches!(
            app.advance_frame(true, 16),
            FrameAdvance::Continue(_)
        ));
        assert_eq!(app.game.world.snapshot().tick, tick);
        assert_eq!(ok(app.game.world.save_bytes()), opening);
        assert_eq!(ok(fs::read(&slot)), HELD_OCCUPIED);
        assert_eq!(app.game.world.occupied_cut_contacts().count(), 0);
        assert_eq!(some(app.occupied_cut.as_ref()).target(), target);

        assert_eq!(press(&mut app, KeyCode::Escape, false), None);
        assert!(!app.modal_open());
        assert!(app.game.world.targeting_snapshot().is_none());
        assert_eq!(ok(app.game.world.save_bytes()), opening);
        assert_eq!(ok(fs::read(&slot)), HELD_OCCUPIED);
        assert!(matches!(
            app.advance_frame(true, 8),
            FrameAdvance::Continue(_)
        ));
        assert!(!app.modal_open());
        assert!(app.game.world.snapshot().tick > tick);
        assert_eq!(app.game.world.occupied_cut_contacts().count(), 0);
    }

    #[test]
    fn actual_occupied_warning_abandons_stale_target_and_focus_loss_without_contact() {
        let stale = TestDirectory::new();
        let (mut app, _) = ready_app(&stale, TargetFilter::Salvage);
        let tick = app.game.world.snapshot().tick;
        app.game.world.cancel_targeting();
        assert!(matches!(
            app.advance_frame(true, 8),
            FrameAdvance::Continue(_)
        ));
        assert_eq!(app.game.world.snapshot().tick, tick);
        assert!(!app.modal_open());
        assert_eq!(app.game.world.occupied_cut_contacts().count(), 0);
        assert_eq!(
            ok(fs::read(stale.0.join("commander.meridian"))),
            HELD_OCCUPIED
        );

        let focus = TestDirectory::new();
        let (mut app, _) = ready_app(&focus, TargetFilter::All);
        app.focus_changed(false);
        assert!(!app.modal_open());
        assert!(app.game.world.targeting_snapshot().is_none());
        assert_eq!(app.game.world.occupied_cut_contacts().count(), 0);
        assert_eq!(
            ok(fs::read(focus.0.join("commander.meridian"))),
            HELD_OCCUPIED
        );
    }

    #[test]
    fn actual_occupied_salvage_confirmation_publishes_one_contact_and_survives_cold_continue() {
        let directory = TestDirectory::new();
        let (mut app, target) = ready_app(&directory, TargetFilter::Salvage);
        let opening = ok(app.game.world.save_bytes());
        let tick = app.game.world.snapshot().tick;
        let cutter = some(app.game.world.fitted_salvage_cutter()).id;
        let unlawful = some(app.occupied_cut.as_ref()).is_unlawful();
        let actor = some(app.game.world.traffic_snapshot()).actors;
        let hold = app.game.world.service_snapshot();
        let cash = app.game.world.credits_tenths();
        assert_eq!(press(&mut app, KeyCode::Enter, false), None);
        assert!(!app.modal_open());
        assert!(app.game.world.targeting_snapshot().is_none());
        let contacts: Vec<_> = app.game.world.occupied_cut_contacts().collect();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].target, target);
        assert_eq!(contacts[0].cutter_item, cutter);
        assert_eq!(contacts[0].world_tick, tick);
        assert_eq!(contacts[0].unlawful, unlawful);
        assert_eq!(some(app.game.world.traffic_snapshot()).actors, actor);
        assert_eq!(app.game.world.service_snapshot(), hold);
        assert_eq!(app.game.world.credits_tenths(), cash);
        let durable = ok(fs::read(directory.0.join("commander.meridian")));
        assert_ne!(durable, opening);
        let decoded = ok(World::from_save_bytes(&durable));
        assert_eq!(
            decoded.occupied_cut_contacts().collect::<Vec<_>>(),
            contacts
        );
        assert_eq!(decoded.service_snapshot(), hold);
        assert_eq!(decoded.credits_tenths(), cash);
        assert_eq!(press(&mut app, KeyCode::Enter, true), None);
        assert!(matches!(
            app.advance_frame(true, 8),
            FrameAdvance::Continue(_)
        ));
        assert!(!app.modal_open());
        assert_eq!(
            app.game.world.occupied_cut_contacts().collect::<Vec<_>>(),
            contacts
        );
        drop(app);
        let mut profile = open(&directory.0);
        ok(profile.continue_saved());
        assert_eq!(
            profile.world.occupied_cut_contacts().collect::<Vec<_>>(),
            contacts
        );
        assert_eq!(profile.world.service_snapshot(), hold);
        assert_eq!(profile.world.credits_tenths(), cash);
        assert!(profile.world.targeting_snapshot().is_none());
    }

    #[test]
    fn actual_occupied_confirmation_failed_writer_publishes_no_contact_or_repeats() {
        let directory = TestDirectory::new();
        let (mut app, _) = ready_app(&directory, TargetFilter::Salvage);
        let opening = ok(app.game.world.save_bytes());
        let slot = directory.0.join("commander.meridian");
        let retained = directory.0.join("retained-occupied-commander");
        ok(fs::rename(&slot, &retained));
        ok(fs::create_dir(&slot));
        assert_eq!(press(&mut app, KeyCode::Enter, false), None);
        assert!(!app.modal_open());
        assert!(app.game.world.targeting_snapshot().is_none());
        assert_eq!(app.game.world.occupied_cut_contacts().count(), 0);
        assert_eq!(ok(app.game.world.save_bytes()), opening);
        assert_eq!(ok(fs::read(&retained)), HELD_OCCUPIED);
        assert!(some(app.notice.text.as_ref()).contains("could not be saved"));
        ok(fs::remove_dir(&slot));
        ok(fs::rename(&retained, &slot));
        assert!(matches!(
            app.advance_frame(true, 8),
            FrameAdvance::Continue(_)
        ));
        assert!(!app.modal_open());
        assert_eq!(app.game.world.occupied_cut_contacts().count(), 0);
    }
}

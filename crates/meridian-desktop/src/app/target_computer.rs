//! Host adapter for flight targeting. Keyboard ownership remains in App::game_key.
use super::{App, DEFAULT_FLIGHT_WINDOW};
use meridian_core::{TargetingSnapshot, World};

fn with_work_status(
    mut target: TargetingSnapshot,
    status: Option<&'static str>,
) -> TargetingSnapshot {
    if let Some(status) = status {
        target.guidance = status;
    }
    target
}

impl App {
    pub(super) fn target_readout(world: &World) -> Option<TargetingSnapshot> {
        world
            .targeting_snapshot()
            .map(|target| with_work_status(target, world.automatic_work_status()))
    }

    pub(super) fn cycle_computer_target(&mut self) {
        // Scanner membership comes from World, not camera clipping or fitted equipment.
        let size = self
            .renderer
            .as_ref()
            .map_or(DEFAULT_FLIGHT_WINDOW, |renderer| {
                renderer.scanner_viewport()
            });
        let tick = self.game.world.snapshot().tick;
        let message = match self.game.world.cycle_scanner_target(tick, size) {
            Ok(true) => self.game.world.targeting_snapshot().map_or_else(
                || "NO TARGETS".into(),
                |target| {
                    format!(
                        "{} selected{}",
                        target.identity.label(),
                        if self.game.world.manual_scanner_target() {
                            " (manual)"
                        } else {
                            ""
                        }
                    )
                },
            ),
            Ok(false) => "NO SCANNER TARGETS".into(),
            Err(error) => error.to_string(),
        };
        self.notice.set(message);
        self.request_redraw();
    }

    pub(super) fn release_computer_target(&mut self) {
        let working = self.game.world.automatic_work_status().is_some();
        self.game.world.cancel_targeting();
        self.notice.set(if working {
            "Automatic work cancelled".into()
        } else {
            "Target released".into()
        });
        self.request_redraw();
    }

    pub(super) fn toggle_computer_assistance(&mut self) {
        let result = self.game.world.toggle_scanner_assistance();
        self.notice.set(match result {
            Ok(true) if self.game.world.targeting_tier().is_none() => {
                "Manual targeting: fit a core for assistance".into()
            }
            Ok(true) => "Target assistance enabled".into(),
            Ok(false) => "Manual target designation; assistance off".into(),
            Err(error) => error.to_string(),
        });
        self.request_redraw();
    }

    pub(super) fn lock_computer_target(&mut self) {
        let tick = self.game.world.snapshot().tick;
        let result = self.game.world.request_selected_target_missile_lock(tick);
        self.notice.set(match result {
            Ok(id) => format!("Missile locked on ship {}", id.serial()),
            Err(error) => error.to_string(),
        });
        self.request_redraw();
    }

    pub(super) fn toggle_target_details(&mut self) {
        let expanded = self.target_details.unwrap_or_else(|| {
            self.renderer
                .as_ref()
                .is_some_and(|renderer| renderer.logical_viewport()[0] >= 1100.0)
        });
        self.target_details = Some(!expanded);
        self.notice.set(
            if expanded {
                "Target details collapsed"
            } else {
                "Target details expanded"
            }
            .into(),
        );
        self.request_redraw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::tests::{TestDirectory, open};
    use meridian_core::{FlightInput, FlightView, TargetFilter, XyzzyAction};
    use winit::keyboard::KeyCode;

    #[test]
    fn work_status_updates_existing_guidance_without_changing_target_fields() {
        use meridian_core::docking::{Basis3, RigidPose};
        use meridian_core::equipment::{TargetingTier, WorkTargetId, WorkTargetRole};
        use meridian_core::{TargetIdentity, TargetMode};

        let mut world = World::new_commander(98105).unwrap_or_else(|e| panic!("{e}"));
        world.request_launch(0).unwrap_or_else(|e| panic!("{e}"));
        for _ in 0..120 {
            world
                .step(FlightInput::default())
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let belt = world
            .mining_belt_snapshot()
            .unwrap_or_else(|| panic!("belt"));
        let rock = belt.bodies[0];
        let original = TargetingSnapshot {
            tracked_fragment: None,
            identity: TargetIdentity::Work(WorkTargetId::Asteroid(rock.id)),
            tier: Some(TargetingTier::Docking),
            filter: TargetFilter::All,
            mode: TargetMode::Work(WorkTargetRole::RockHold),
            guidance: "Rock guidance",
            pose: RigidPose {
                position: rock.position,
                orientation: Basis3::IDENTITY,
            },
            model: Some("Asteroid"),
            status: None,
            fields: vec![("Range", "1 unit".into())],
        };
        assert_eq!(with_work_status(original.clone(), None), original);
        // An independent presentation projection; only the World can publish a work status.
        let mut expected = original.clone();
        expected.guidance = "Approaching work target";
        assert_eq!(
            with_work_status(original, Some("Approaching work target")),
            expected
        );
    }

    #[test]
    fn flight_t_requires_selected_ship_and_details_remain_host_local() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(
            profile
                .new_commander(98104)
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
        let tick = app.game.world.snapshot().tick;
        app.game
            .world
            .request_flight_view(FlightView::Rear, tick)
            .unwrap_or_else(|e| panic!("{e}"));
        let missile = app.game.world.missile_snapshot();
        let before = app
            .game
            .world
            .save_bytes()
            .unwrap_or_else(|e| panic!("{e}"));
        app.game_key(KeyCode::KeyT);
        assert_eq!(
            app.notice.text.as_deref(),
            Some("Lock an actual live target")
        );
        assert_eq!(app.game.world.missile_snapshot(), missile);
        assert!(app.game.world.targeting_tier().is_none());
        let expected = app.game.world.scanner_targets();
        assert!(expected.len() > 1);
        app.game_key(KeyCode::Tab);
        let first = app
            .game
            .world
            .targeting_snapshot()
            .unwrap_or_else(|| panic!("manual target"));
        assert_eq!(first.identity, expected[0].0);
        assert_eq!(first.tier, None);
        assert!(app.game.world.manual_scanner_target());
        assert_eq!(app.game.world.flight_view(), FlightView::Rear);
        app.game_key(KeyCode::KeyF);
        assert!(app.game.world.manual_scanner_target());
        assert_eq!(
            app.game
                .world
                .targeting_snapshot()
                .map(|snapshot| snapshot.identity),
            Some(first.identity)
        );
        app.game_key(KeyCode::Tab);
        assert_eq!(
            app.game
                .world
                .targeting_snapshot()
                .map(|snapshot| snapshot.identity),
            Some(expected[1].0)
        );
        assert_eq!(app.game.world.flight_view(), FlightView::Rear);
        app.game_key(KeyCode::Delete);
        assert!(app.game.world.targeting_snapshot().is_none());
        app.toggle_target_details();
        assert_eq!(app.target_details, Some(true));
        app.toggle_target_details();
        assert_eq!(app.target_details, Some(false));
        assert_eq!(
            app.game
                .world
                .save_bytes()
                .unwrap_or_else(|e| panic!("{e}")),
            before
        );
        assert!(
            crate::help::Panel::flight(None, None)
                .lines
                .iter()
                .any(|line| line.contains("T Lock selected ship"))
        );
    }

    #[test]
    fn scanner_assistance_toggle_preserves_the_actual_contact_and_tab_uses_the_next_contact() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(
            profile
                .new_commander(98107)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        profile
            .xyzzy(XyzzyAction::Enter)
            .unwrap_or_else(|e| panic!("{e}"));
        let mut app = App::with_profile(profile);
        app.game
            .world
            .request_launch(0)
            .unwrap_or_else(|e| panic!("{e}"));
        app.game
            .world
            .step(FlightInput::default())
            .unwrap_or_else(|e| panic!("{e}"));
        let expected = app.game.world.scanner_targets();
        assert!(expected.len() > 1);
        app.game_key(KeyCode::Tab);
        let first = app
            .game
            .world
            .targeting_snapshot()
            .unwrap_or_else(|| panic!("assisted target"));
        assert_eq!(first.identity, expected[0].0);
        assert!(first.tier.is_some());
        assert!(!app.game.world.manual_scanner_target());
        app.game_key(KeyCode::KeyF);
        assert!(app.game.world.manual_scanner_target());
        assert_eq!(
            app.game
                .world
                .targeting_snapshot()
                .map(|snapshot| snapshot.identity),
            Some(first.identity)
        );
        app.game_key(KeyCode::KeyF);
        assert!(!app.game.world.manual_scanner_target());
        assert_eq!(
            app.game
                .world
                .targeting_snapshot()
                .map(|snapshot| snapshot.identity),
            Some(first.identity)
        );
        app.game_key(KeyCode::Tab);
        assert_eq!(
            app.game
                .world
                .targeting_snapshot()
                .map(|snapshot| snapshot.identity),
            Some(expected[1].0)
        );
    }
}

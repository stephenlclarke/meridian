//! Host adapter for flight targeting. Keyboard ownership remains in App::game_key.
use super::{App, StationScene};
use meridian_core::{TargetFilter, TargetingSnapshot, World};

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
        let result = (|| {
            if self.game.world.targeting_tier().is_none() {
                return Err("Fit a targeting computer to select targets".to_owned());
            }
            let snapshot = self.game.world.snapshot();
            let renderer = self
                .renderer
                .as_ref()
                .ok_or("Targeting requires an active flight viewport")?;
            let station = StationScene {
                pose: snapshot.station.ok_or("No local station frame")?,
                port: self
                    .game
                    .world
                    .station_port()
                    .ok_or("No local station geometry")?,
                docked: false,
            };
            let traffic = self.game.world.traffic_snapshot();
            let mule = self.game.world.mule_actor_snapshot();
            let filter = self.game.world.target_filter();
            let belt = (filter != TargetFilter::Ships)
                .then(|| self.game.world.mining_belt_snapshot())
                .flatten();
            let (viewport, work_visible) = renderer.targeting_viewport(
                &snapshot,
                station,
                traffic.as_ref(),
                mule.as_ref(),
                belt.as_ref(),
            )?;
            if !self
                .game
                .world
                .cycle_targeting_with_work(snapshot.tick, viewport, &work_visible)
                .map_err(|error| error.to_string())?
            {
                let fit = self.game.world.work_targeting_fit();
                let message = match filter {
                    TargetFilter::Mining if !fit.mining => "Fit a Mining Module",
                    TargetFilter::Salvage if !fit.salvage => "Fit a Salvage Module",
                    TargetFilter::Mining => "No eligible rocks or ore",
                    TargetFilter::Salvage => "No eligible disabled ships or recoverable wrecks",
                    _ => "NO TARGETS",
                };
                return Ok(message.to_owned());
            }
            Ok(self.game.world.targeting_snapshot().map_or_else(
                || "NO TARGETS".into(),
                |target| format!("{} selected", target.identity.label()),
            ))
        })();
        self.notice.set(match result {
            Ok(message) | Err(message) => message,
        });
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

    pub(super) fn cycle_computer_filter(&mut self) {
        let result = self.game.world.cycle_target_filter();
        self.notice.set(match result {
            Ok(filter) => format!("TARGET FILTER: {}", filter.label()),
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
    use meridian_core::FlightInput;
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
            identity: TargetIdentity::Work(WorkTargetId::Asteroid(rock.id)),
            tier: TargetingTier::Docking,
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
        for (filter, label) in [
            (TargetFilter::Mining, "Mining"),
            (TargetFilter::Salvage, "Salvage"),
            (TargetFilter::All, "All"),
            (TargetFilter::Ships, "Ships"),
        ] {
            app.game_key(KeyCode::KeyF);
            assert_eq!(app.game.world.target_filter(), filter);
            assert_eq!(
                app.notice.text.as_deref(),
                Some(format!("TARGET FILTER: {label}").as_str())
            );
        }
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
}

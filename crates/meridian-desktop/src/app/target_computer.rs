//! Host adapter for flight targeting. Keyboard ownership remains in App::game_key.
use super::{App, StationScene};

impl App {
    pub(super) fn cycle_computer_target(&mut self) {
        let result = (|| {
            if self.game.world.targeting_tier().is_none() {
                return Err("Fit a targeting computer to select ships".to_owned());
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
            let viewport =
                renderer.targeting_viewport(&snapshot, station, traffic.as_ref(), mule.as_ref())?;
            if !self
                .game
                .world
                .cycle_targeting(snapshot.tick, viewport)
                .map_err(|error| error.to_string())?
            {
                return Ok("NO TARGETS".to_owned());
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
        self.game.world.cancel_targeting();
        self.notice.set("Target released".into());
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

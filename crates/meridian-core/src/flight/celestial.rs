use super::World;
use crate::celestial::{self, Snapshot};
impl World {
    /// Derived from saved authoritative navigation/tick and the actual station frame.
    /// No new save state, random global, wall clock or renderer coordinates are required.
    pub fn celestial_snapshot(&self) -> Option<Snapshot> {
        let navigation = self.snapshot().navigation?;
        let system = self
            .navigation_catalogue
            .as_ref()?
            .system(navigation.system)
            .ok()?;
        let anchor = self.state.station?.pose.position;
        Some(celestial::scene(
            system,
            navigation.visit,
            self.state.tick,
            anchor,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FlightInput, FlightMode, MotionRefusal, StepError};
    fn world() -> World {
        World::new_commander(92011).unwrap_or_else(|e| panic!("{e}"))
    }
    #[test]
    fn actual_navigation_tick_pause_and_cold_save_bind_the_same_scene() {
        let mut world = world();
        let before = world
            .celestial_snapshot()
            .unwrap_or_else(|| panic!("scene"));
        assert_eq!(
            before.planet.id.system,
            world
                .snapshot()
                .navigation
                .unwrap_or_else(|| panic!("nav"))
                .system
        );
        world
            .step(FlightInput::default())
            .unwrap_or_else(|e| panic!("{e}"));
        let after = world
            .celestial_snapshot()
            .unwrap_or_else(|| panic!("scene"));
        assert_eq!(after.tick, before.tick + 1);
        assert_ne!(before.angle, after.angle);
        assert_eq!(before.recipe, after.recipe);
        assert_eq!(before.planet, after.planet);
        world
            .request_pause(true, world.snapshot().tick)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(world.step(FlightInput::default()), Err(StepError::Paused));
        assert_eq!(world.celestial_snapshot(), Some(after));
        let saved = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        let cold = World::from_save_bytes(&saved).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(cold.celestial_snapshot(), Some(after));
        // Continue deliberately invalidates UI review epochs; compare physical and celestial state.
        assert_eq!(cold.snapshot(), world.snapshot());
        assert_eq!(cold.mission_snapshot().map(|m| m.paused), Some(true));
    }
    #[test]
    fn invalid_inside_surface_pose_is_refused_by_shared_save_validation() {
        let mut world = world();
        let body = world
            .celestial_snapshot()
            .unwrap_or_else(|| panic!("scene"))
            .planet;
        world.state.position = [
            body.position.x as f32,
            body.position.y as f32,
            body.position.z as f32,
        ];
        assert!(matches!(
            world.save_bytes(),
            Err(crate::save::SaveError::InvalidState(
                "player intersects physical celestial surface"
            ))
        ));
    }
    #[test]
    fn actual_flight_sweep_stops_the_hull_before_a_planet_and_refusal_survives_cold_load() {
        let mut world = world();
        world.request_launch(0).unwrap_or_else(|e| panic!("{e}"));
        for _ in 0..600 {
            world
                .step(FlightInput::default())
                .unwrap_or_else(|e| panic!("{e}"));
            if world.snapshot().mode == FlightMode::Flight {
                break;
            }
        }
        let body = world
            .celestial_snapshot()
            .unwrap_or_else(|| panic!("scene"))
            .planet;
        let hull = world
            .station_data
            .as_ref()
            .unwrap_or_else(|| panic!("station"))
            .player_radius();
        // Unit fixture near the actual derived planet; no native journey acceptance is implied.
        world.state.position = [
            body.position.x as f32,
            body.position.y as f32,
            (body.position.z - body.radius - hull * 1.00001 - 0.01) as f32,
        ];
        world.state.orientation = [0.0, 0.0, 0.0, 1.0];
        world.state.speed = 34.3;
        let before = world.snapshot();
        world
            .step(FlightInput::default())
            .unwrap_or_else(|e| panic!("{e}"));
        let after = world.snapshot();
        assert_eq!(after.position, before.position);
        assert_eq!(after.speed, 0.0);
        assert_eq!(after.tick, before.tick + 1);
        assert_eq!(after.motion_refusal, Some(MotionRefusal::CelestialContact));
        let bytes = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        let cold = World::from_save_bytes(&bytes).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(cold.snapshot().motion_refusal, after.motion_refusal);
    }
}

#[cfg(test)]
mod travel_tests {
    use super::*;
    use crate::{
        FlightInput,
        navigation::{BASIC_SPOOL_TICKS, SystemId},
    };
    #[test]
    fn actual_jump_arrival_rebinds_body_ids_surface_recipe_and_cold_scene() {
        let mut world = World::new_commander(92012).unwrap_or_else(|e| panic!("{e}"));
        let origin = world
            .celestial_snapshot()
            .unwrap_or_else(|| panic!("origin"));
        world.request_launch(0).unwrap_or_else(|e| panic!("{e}"));
        world
            .step(FlightInput::default())
            .unwrap_or_else(|e| panic!("{e}"));
        world
            .request_jump(SystemId::RENAS, world.snapshot().tick)
            .unwrap_or_else(|e| panic!("{e}"));
        for _ in 0..BASIC_SPOOL_TICKS {
            world
                .step(FlightInput::default())
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let arrived = world
            .celestial_snapshot()
            .unwrap_or_else(|| panic!("arrival"));
        assert_eq!(arrived.planet.id.system, SystemId::RENAS);
        assert!(arrived.planet.id.visit > origin.planet.id.visit);
        assert_ne!(arrived.recipe, origin.recipe);
        let bytes = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        let cold = World::from_save_bytes(&bytes).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(cold.celestial_snapshot(), Some(arrived));
    }
}

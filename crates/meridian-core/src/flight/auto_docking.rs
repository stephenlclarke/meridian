//! Fictional station assistance using only the same held controls as manual flight.
use super::{
    ACCELERATION, FIXED_DELTA_SECONDS, FlightInput, FlightMode, PITCH_RATE, ROLL_RATE, World,
    YAW_RATE,
};
use crate::docking::Vec3;
use crate::navigation::NavigationGrade;
use crate::station::DISTANCE_PER_SPEED_UNIT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Side,
    Front,
    Approach,
    Final,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Phase {
    stage: Stage,
    staging: Vec3,
    side: Vec3,
    system: crate::navigation::SystemId,
    visit: u64,
    ready: bool,
}

impl World {
    pub(super) fn docking_computer_available(&self) -> bool {
        self.commander_account.as_ref().is_some_and(|account| {
            self.modular
                .ledger(self, account)
                .is_ok_and(|ledger| ledger.navigation_grade() != NavigationGrade::Empty)
        })
    }

    /// Toggle transient docking assistance. The usual World entry and collision gates still apply.
    pub fn toggle_auto_docking(&mut self) -> Result<&'static str, &'static str> {
        if self.auto_docking.take().is_some() {
            self.cancel_targeting();
            return Ok("Automatic docking cancelled");
        }
        self.engage_auto_docking(false)
    }

    fn engage_auto_docking(&mut self, station_tab: bool) -> Result<&'static str, &'static str> {
        let nav = self
            .snapshot()
            .navigation
            .ok_or("No local navigation context")?;
        if self.is_paused()
            || self.state.mode != FlightMode::Flight
            || nav.travel.is_some()
            || self
                .combat_snapshot()
                .is_some_and(|combat| combat.player.destroyed)
        {
            return Err("Automatic docking requires ordinary local flight");
        }
        if self.emp.electrical_disabled() || !self.docking_computer_available() {
            return Err("Fit an available navigational computer");
        }
        let data = self.station_data.as_ref().ok_or("No local station")?;
        let start = data
            .launch_start()
            .map_err(|_| "Station approach unavailable")?
            .position;
        let staging = Vec3::new(
            f64::from(start[0]),
            f64::from(start[1]),
            f64::from(start[2]),
        );
        let pose = super::flight_pose(self.state.position, self.state.orientation)
            .ok_or("Invalid flight pose")?;
        let stage = if pose.position.z < data.port().center().z + 80.0 {
            Stage::Side
        } else {
            Stage::Approach
        };
        let side = Vec3::new(
            if pose.position.x < 0.0 {
                -staging.z
            } else {
                staging.z
            },
            pose.position.y,
            pose.position.z,
        );
        if !station_tab {
            self.cancel_targeting();
        }
        self.auto_docking = Some(Phase {
            stage,
            staging,
            side,
            system: nav.system,
            visit: nav.visit,
            ready: false,
        });
        Ok("Automatic docking engaged")
    }

    pub fn auto_docking_active(&self) -> bool {
        self.auto_docking.is_some()
    }

    pub fn cancel_auto_docking(&mut self) {
        if self.auto_docking.take().is_some() {
            self.cancel_targeting();
        }
    }

    /// Live phase only; "Ready" means the current fictional approach is aligned,
    /// not that the native entry sweep has admitted the ship.
    pub fn auto_docking_status(&self) -> Option<&'static str> {
        self.auto_docking
            .map(|phase| match (phase.stage, phase.ready) {
                (Stage::Final, true) => "Ready",
                (Stage::Final, false) => "Final approach",
                _ => "Approach",
            })
    }

    pub(super) fn flight_computer_input(&mut self, raw: FlightInput) -> FlightInput {
        if let Some(phase) = self.auto_docking {
            let local = self.snapshot().navigation.is_some_and(|nav| {
                nav.travel.is_none() && nav.system == phase.system && nav.visit == phase.visit
            });
            if raw != FlightInput::default() || self.emp.electrical_disabled() || !local {
                self.auto_docking = None;
                if local && !self.emp.electrical_disabled() {
                    self.take_manual_control();
                } else {
                    self.cancel_targeting();
                }
                return raw;
            }
            if let Some(input) = self.docking_input() {
                return input;
            }
            self.cancel_auto_docking();
            return FlightInput {
                brake: true,
                ..FlightInput::default()
            };
        }
        if self.emp.electrical_disabled() {
            self.cancel_targeting();
            return raw;
        }
        if raw == FlightInput::default() && self.station_docking_assistance() {
            if self.engage_auto_docking(true).is_ok()
                && let Some(input) = self.docking_input()
            {
                return input;
            }
            self.auto_docking = None;
            self.cancel_targeting();
        }
        self.targeting_flight_input(raw)
    }

    fn docking_input(&mut self) -> Option<FlightInput> {
        let data = self.station_data.as_ref()?;
        let pose = super::flight_pose(self.state.position, self.state.orientation)?;
        let frame = data.station_pose().entrance_frame(data.port()).ok()?;
        let mut phase = self.auto_docking?;
        let distance = |point: Vec3| {
            let delta = difference(point, pose.position);
            delta.dot(delta).sqrt()
        };
        let target = |stage| match stage {
            Stage::Side => phase.side,
            Stage::Front => Vec3::new(phase.side.x, phase.side.y, phase.staging.z),
            Stage::Approach => phase.staging,
            Stage::Final => {
                let entrance = frame.position();
                let outward = frame.outward();
                Vec3::new(
                    entrance.x - outward.x * 12.0,
                    entrance.y - outward.y * 12.0,
                    entrance.z - outward.z * 12.0,
                )
            }
        };
        if distance(target(phase.stage)) < 8.0 && self.state.speed <= 0.2 {
            phase.stage = match phase.stage {
                Stage::Side => Stage::Front,
                Stage::Front => Stage::Approach,
                Stage::Approach | Stage::Final => Stage::Final,
            };
            self.auto_docking = Some(phase);
        }
        let final_approach = phase.stage == Stage::Final;
        let point = target(phase.stage);
        let local = pose
            .orientation
            .inverse_coordinates(difference(point, pose.position));
        let up = pose.orientation.inverse_coordinates(frame.up());
        let bearing = local.x.atan2(local.z);
        let elevation = local.y.atan2(local.x.hypot(local.z));
        let bank = -up.x.atan2(up.y);
        let control = |angle: f64, rate: f32| {
            (angle / f64::from(rate * FIXED_DELTA_SECONDS)).clamp(-1.0, 1.0) as f32
        };
        let aligned = bearing.abs() < 0.025
            && elevation.abs() < 0.025
            && (!final_approach || bank.abs() < 0.025);
        phase.ready = final_approach && aligned && distance(frame.position()) < 25.0;
        self.auto_docking = Some(phase);
        let max_speed = if final_approach { 4.0 } else { 8.0 };
        let remaining = distance(point);
        let stopping =
            (2.0 * f64::from(ACCELERATION) * remaining / f64::from(DISTANCE_PER_SPEED_UNIT)).sqrt();
        let desired = if aligned && (final_approach || remaining >= 5.0) {
            stopping.min(max_speed) as f32
        } else {
            0.0
        };
        let throttle =
            ((desired - self.state.speed) / (ACCELERATION * FIXED_DELTA_SECONDS)).clamp(-1.0, 1.0);
        Some(FlightInput {
            yaw: control(bearing, YAW_RATE),
            pitch: control(elevation, PITCH_RATE),
            roll: if final_approach {
                control(bank, ROLL_RATE)
            } else {
                0.0
            },
            throttle,
            brake: false,
        })
    }
}

fn difference(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn launched() -> World {
        let mut world = World::new_commander(8031).unwrap();
        world.request_launch(world.snapshot().tick).unwrap();
        world.step(FlightInput::default()).unwrap();
        assert_eq!(world.snapshot().mode, FlightMode::Flight);
        world
    }

    #[test]
    fn no_station_or_docked_activation_and_manual_override() {
        assert!(World::new().toggle_auto_docking().is_err());
        let mut world = World::new_commander(8032).unwrap();
        assert!(world.toggle_auto_docking().is_err());
        let mut world = launched();
        assert!(world.toggle_auto_docking().is_ok());
        assert!(world.auto_docking_active());
        assert!(world.toggle_auto_docking().is_ok());
        assert!(!world.auto_docking_active());
        world.toggle_auto_docking().unwrap();
        assert!(
            world
                .step(FlightInput {
                    yaw: f32::NAN,
                    ..FlightInput::default()
                })
                .is_err()
        );
        assert!(world.auto_docking_active());
        world
            .step(FlightInput {
                throttle: -1.0,
                ..FlightInput::default()
            })
            .unwrap();
        assert!(!world.auto_docking_active());
    }

    #[test]
    fn ordinary_launch_returns_through_the_original_station_entry() {
        let mut world = launched();
        let start = world.snapshot();
        world.toggle_auto_docking().unwrap();
        for _ in 0..5000 {
            let prior = world.snapshot();
            world.step(FlightInput::default()).unwrap();
            let next = world.snapshot();
            assert_eq!(next.tick, prior.tick + 1);
            assert!(
                next.motion_refusal.is_none(),
                "refused {:?} at {:?}",
                next.motion_refusal,
                next.position
            );
            if next.mode == FlightMode::Docked {
                assert!(!world.auto_docking_active());
                assert!(next.tick > start.tick + 60);
                return;
            }
        }
        panic!("automatic docking did not complete: {:?}", world.snapshot());
    }

    #[test]
    fn fitted_navigation_is_required_and_invalid_pose_refuses() {
        let mut world = World::new_commander(8033).unwrap();
        let navigation = world
            .modular_equipment_snapshot()
            .unwrap()
            .items
            .into_iter()
            .find(|item| item.slot == Some(super::super::ModuleSlot::Navigation))
            .unwrap();
        let tick = world.snapshot().tick;
        let review = world
            .review_module(
                super::super::ModuleAction::Store {
                    item: navigation.id,
                },
                tick,
            )
            .unwrap();
        world.prepare_module(review, tick).unwrap().commit();
        world.request_launch(tick).unwrap();
        world.step(FlightInput::default()).unwrap();
        assert_eq!(
            world.toggle_auto_docking(),
            Err("Fit an available navigational computer")
        );
        let mut world = launched();
        world.state.orientation = [0.0; 4];
        assert_eq!(world.toggle_auto_docking(), Err("Invalid flight pose"));
        assert!(!world.auto_docking_active());
    }

    #[test]
    fn pause_and_cold_restore_never_advance_or_restore_automatic_control() {
        let mut world = launched();
        world.toggle_auto_docking().unwrap();
        let tick = world.snapshot().tick;
        let active = world.snapshot();
        assert!(world.request_pause(true, tick + 1).is_err());
        assert_eq!(world.snapshot(), active);
        assert!(world.auto_docking_active());
        world.request_pause(true, tick).unwrap();
        let paused = world.snapshot();
        assert_eq!(
            world.step(FlightInput::default()),
            Err(super::super::StepError::Paused)
        );
        assert_eq!(world.snapshot(), paused);
        assert!(!world.auto_docking_active());
        world.request_pause(false, tick).unwrap();
        assert!(!world.auto_docking_active());
        world.toggle_auto_docking().unwrap();
        let bytes = world.save_bytes().unwrap();
        let restored = World::decode_saved_world(&bytes).unwrap();
        assert!(!restored.auto_docking_active());
        world.cancel_auto_docking();
        assert!(!world.auto_docking_active());
    }

    #[test]
    fn station_tab_handoff_retains_selection_until_raw_manual_input() {
        let mut world = World::new_commander(8034).unwrap();
        let tick = world.snapshot().tick;
        let review = world
            .review_xyzzy(super::super::XyzzyAction::Enter, tick)
            .unwrap();
        world.prepare_xyzzy(review, tick).unwrap().commit();
        world.request_launch(tick).unwrap();
        world.step(FlightInput::default()).unwrap();
        for _ in 0..100 {
            world
                .cycle_targeting(
                    world.snapshot().tick,
                    super::super::TargetingViewport {
                        size: [640, 360],
                        front: Vec::new(),
                    },
                )
                .unwrap();
            if world.targeting_station_selected() {
                break;
            }
        }
        assert!(world.targeting_station_selected());
        world.step(FlightInput::default()).unwrap();
        assert!(world.auto_docking_active());
        assert!(world.targeting_station_selected());
        assert_eq!(world.auto_docking_status(), Some("Approach"));
        let tick = world.snapshot().tick;
        assert!(world.request_pause(true, tick + 1).is_err());
        assert!(world.auto_docking_active());
        assert!(world.targeting_station_selected());
        world.request_pause(true, tick).unwrap();
        assert!(!world.auto_docking_active());
        assert!(!world.targeting_station_selected());
        world.request_pause(false, tick).unwrap();
        for _ in 0..100 {
            world
                .cycle_targeting(
                    tick,
                    super::super::TargetingViewport {
                        size: [640, 360],
                        front: Vec::new(),
                    },
                )
                .unwrap();
            if world.targeting_station_selected() {
                break;
            }
        }
        assert!(world.targeting_station_selected());
        world.step(FlightInput::default()).unwrap();
        assert!(world.auto_docking_active());
        world
            .step(FlightInput {
                brake: true,
                ..FlightInput::default()
            })
            .unwrap();
        assert!(!world.auto_docking_active());
        assert!(!world.targeting_station_selected());
    }

    #[test]
    fn ordinary_offset_approach_uses_only_validated_flight_ticks() {
        let mut world = launched();
        for _ in 0..100 {
            world
                .step(FlightInput {
                    yaw: 1.0,
                    ..FlightInput::default()
                })
                .unwrap();
        }
        assert!(world.snapshot().position[0].abs() > 50.0);
        world.toggle_auto_docking().unwrap();
        for _ in 0..6000 {
            let before = world.snapshot();
            world.step(FlightInput::default()).unwrap();
            let next = world.snapshot();
            assert_eq!(next.tick, before.tick + 1);
            assert!(
                next.motion_refusal.is_none(),
                "refused {:?} at {:?}",
                next.motion_refusal,
                next.position
            );
            if next.mode == FlightMode::Docked {
                assert!(!world.auto_docking_active());
                return;
            }
        }
        panic!("offset docking did not complete: {:?}", world.snapshot());
    }

    #[test]
    fn hyperspace_request_invalidates_local_docking_visit() {
        let mut world = launched();
        world.toggle_auto_docking().unwrap();
        world
            .request_jump(crate::navigation::SystemId::RENAS, world.snapshot().tick)
            .unwrap();
        world.step(FlightInput::default()).unwrap();
        assert!(!world.auto_docking_active());
        assert!(world.toggle_auto_docking().is_err());
    }
}

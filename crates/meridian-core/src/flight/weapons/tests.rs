#![allow(clippy::unwrap_used)]
use super::*;
use crate::FlightInput;
use crate::navigation::{BASIC_SPOOL_TICKS, SystemId};

fn step(world: &mut World, count: u8) {
    for _ in 0..count {
        world.step(FlightInput::default()).unwrap();
    }
}

fn launch(world: &mut World) {
    world.request_launch(world.snapshot().tick).unwrap();
    step(world, 1);
}

fn fire(world: &mut World) -> Result<PulseShot, FireError> {
    world.request_fire_front_pulse(world.snapshot().tick)
}

fn refusal(world: &mut World, expected: FireError) {
    let before = world.save_bytes().unwrap();
    assert_eq!(fire(world), Err(expected));
    assert_eq!(world.save_bytes().unwrap(), before);
}

#[test]
fn public_fitted_discharge_checks_flight_pause_tick_and_exact_recharge_cadence() {
    let mut prototype = World::new();
    assert_eq!(prototype.pulse_laser_snapshot(), None);
    assert_eq!(fire(&mut prototype), Err(FireError::NoCommander));

    let mut world = World::new_commander(501).unwrap();
    refusal(&mut world, FireError::NotInFlight);
    world.request_launch(0).unwrap();
    refusal(&mut world, FireError::NotInFlight);
    step(&mut world, 1);
    let before = world.save_bytes().unwrap();
    assert_eq!(
        world.request_fire_front_pulse(0),
        Err(FireError::StaleCommand)
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    world.request_pause(true, world.snapshot().tick).unwrap();
    refusal(&mut world, FireError::Paused);
    world.request_pause(false, world.snapshot().tick).unwrap();

    let position = world.snapshot().position;
    let flight = world.snapshot();
    let equipment = world.equipment_snapshot();
    let shot = fire(&mut world).unwrap();
    assert_eq!(shot.tick, 1);
    assert_eq!(shot.fitted_item_id, 508);
    assert_eq!(shot.origin, Vec3::new(0.0, 0.0, f64::from(position[2])));
    assert_eq!(shot.direction, Vec3::new(0.0, 0.0, 1.0));
    assert_eq!(shot.end.z, shot.origin.z + 6000.0);
    assert_eq!(shot.contact, PulseContact::Clear);
    assert_eq!(world.snapshot(), flight);
    assert_eq!(world.equipment_snapshot(), equipment);
    assert_eq!(world.credits_tenths(), Some(7200));
    assert_eq!(
        world.pulse_laser_snapshot(),
        Some(PulseLaserSnapshot {
            fitted_item_id: 508,
            energy_hundredths: 37900,
            heat_fifths: 50,
            cooldown_ticks: 18,
        })
    );
    refusal(&mut world, FireError::CoolingDown);
    step(&mut world, 1);
    assert_eq!(
        world.pulse_laser_snapshot().unwrap().energy_hundredths,
        37907
    );
    step(&mut world, 16);
    let state = world.pulse_laser_snapshot().unwrap();
    assert_eq!(
        (
            state.energy_hundredths,
            state.heat_fifths,
            state.cooldown_ticks
        ),
        (38000, 33, 1)
    );
    refusal(&mut world, FireError::CoolingDown);
    step(&mut world, 1);
    fire(&mut world).unwrap();
    assert_eq!(world.pulse_laser_snapshot().unwrap().heat_fifths, 82);
}

#[test]
fn public_cadence_reaches_overheat_then_resumes_only_after_sufficient_flight_cooling() {
    let mut world = World::new_commander(601).unwrap();
    launch(&mut world);
    for i in 0..38 {
        if i != 0 {
            step(&mut world, 18);
        }
        fire(&mut world).unwrap();
    }
    assert_eq!(world.pulse_laser_snapshot().unwrap().heat_fifths, 1234);
    step(&mut world, 18);
    refusal(&mut world, FireError::Overheated);
    step(&mut world, 16);
    assert_eq!(world.pulse_laser_snapshot().unwrap().heat_fifths, 1200);
    refusal(&mut world, FireError::Overheated);
    step(&mut world, 1);
    fire(&mut world).unwrap();
    assert_eq!(world.pulse_laser_snapshot().unwrap().heat_fifths, 1249);
}

#[test]
fn authentic_jump_and_public_aim_distinguish_station_aperture_miss_and_native_wall_block() {
    let mut world = World::new_commander(701).unwrap();
    launch(&mut world);
    world
        .request_jump(SystemId::RENAS, world.snapshot().tick)
        .unwrap();
    for _ in 0..BASIC_SPOOL_TICKS {
        step(&mut world, 1);
    }
    let out_of_range = fire(&mut world).unwrap();
    assert_eq!(out_of_range.direction, Vec3::new(0.0, 0.0, -1.0));
    assert_eq!(out_of_range.end, Vec3::new(0.0, 0.0, 10000.0));
    assert_eq!(out_of_range.contact, PulseContact::Clear);
    for _ in 0..3000 {
        if world.snapshot().position[2] <= 5500.0 {
            break;
        }
        world
            .step(FlightInput {
                throttle: 1.0,
                ..FlightInput::default()
            })
            .unwrap();
    }
    assert!(world.snapshot().position[2] <= 5500.0);
    for _ in 0..300 {
        if world.snapshot().speed == 0.0 {
            break;
        }
        world
            .step(FlightInput {
                brake: true,
                ..FlightInput::default()
            })
            .unwrap();
    }
    assert_eq!(world.snapshot().speed, 0.0);
    let center = fire(&mut world).unwrap();
    assert!(center.origin.z > 4000.0 && center.origin.z < 5500.0);
    assert_eq!(center.contact, PulseContact::Clear);
    assert!(center.end.z < 0.0); // This ray traverses the real station aperture and axial void.
    step(&mut world, 18);
    // Aim at the pinned wall 70 units right of the center, at the near station plane.
    let yaw = -(70.0 / (center.origin.z - 136.32)).atan() as f32 / (0.65 / 60.0) / 2.0;
    for _ in 0..2 {
        world
            .step(FlightInput {
                yaw,
                ..FlightInput::default()
            })
            .unwrap();
    }
    let before = world.snapshot();
    let station_blocked = fire(&mut world).unwrap();
    let PulseContact::StationBlocked { distance } = station_blocked.contact else {
        panic!("actual native station wall did not stop the rotated front beam");
    };
    assert!(station_blocked.direction.x > 0.0);
    assert!((station_blocked.end.x - 70.0).abs() < 0.05);
    assert!((station_blocked.end.z - 136.32).abs() < 1e-6);
    assert!(distance < PULSE_RANGE);
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.credits_tenths(), Some(7200));
    assert_eq!(world.pulse_laser_snapshot().unwrap().cooldown_ticks, 18);
}

#[test]
fn hot_in_flight_continue_retains_resources_and_refuses_resealed_corruption_and_v3() {
    let mut world = World::new_commander(801).unwrap();
    launch(&mut world);
    fire(&mut world).unwrap();
    step(&mut world, 5);
    let resources = world.pulse_laser_snapshot();
    let bytes = world.save_bytes().unwrap();
    let mut continued = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(continued.snapshot(), world.snapshot());
    assert_eq!(continued.pulse_laser_snapshot(), resources);
    assert_eq!(continued.equipment_snapshot(), world.equipment_snapshot());
    refusal(&mut continued, FireError::CoolingDown);
    let before = continued.save_bytes().unwrap();
    let end = bytes.len() - 8;
    for (offset, value) in [(8, 3), (end - 5, 255), (end - 3, 255), (end - 1, 19)] {
        let mut bad = bytes.clone();
        bad[offset] = value;
        let digest = crate::save::checksum(&bad[..end]);
        bad[end..].copy_from_slice(&digest.to_le_bytes());
        assert!(continued.restore_bytes(&bad).is_err());
        assert_eq!(continued.save_bytes().unwrap(), before);
    }
    step(&mut continued, 13);
    fire(&mut continued).unwrap();
}

#[test]
fn physical_dock_restores_heat_and_energy_and_docked_steps_hold_cooldown() {
    let mut world = World::new_commander(901).unwrap();
    launch(&mut world);
    world
        .request_jump(SystemId::RENAS, world.snapshot().tick)
        .unwrap();
    for _ in 0..BASIC_SPOOL_TICKS {
        step(&mut world, 1);
    }
    for _ in 0..9000 {
        let before = world.snapshot();
        if before.mode == FlightMode::Docked {
            break;
        }
        let target_speed = if before.position[2] > 2500.0 {
            34.3
        } else {
            4.0
        };
        world
            .step(FlightInput {
                throttle: ((target_speed - before.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..FlightInput::default()
            })
            .unwrap();
        assert_eq!(world.snapshot().motion_refusal, None);
        if world.snapshot().mode == FlightMode::Flight && world.snapshot().position[2] < 145.0 {
            let accepted = fire(&mut world);
            assert!(accepted.is_ok() || accepted == Err(FireError::CoolingDown));
        }
    }
    let dock = world.current_dock_context().unwrap();
    assert_eq!(dock.origin(), crate::navigation::DockOrigin::PhysicalEntry);
    let reset = world.pulse_laser_snapshot().unwrap();
    assert_eq!((reset.energy_hundredths, reset.heat_fifths), (38000, 0));
    assert!(reset.cooldown_ticks > 0);
    step(&mut world, 18);
    assert_eq!(world.pulse_laser_snapshot(), Some(reset));
    refusal(&mut world, FireError::NotInFlight);
    world.request_launch(world.snapshot().tick).unwrap();
    assert_eq!(world.pulse_laser_snapshot(), Some(reset));
    step(&mut world, 1);
    assert_eq!(
        world.pulse_laser_snapshot().unwrap().cooldown_ticks,
        reset.cooldown_ticks - 1
    );
}

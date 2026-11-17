#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::practice::tests::{aim_at, consent, launch, shoot, step};
use crate::{FlightInput, PracticeStatus, PulseContact};

pub(in crate::flight) fn stop(world: &mut World) {
    for _ in 0..300 {
        if world.snapshot().speed == 0.0 {
            return;
        }
        world
            .step(FlightInput {
                brake: true,
                ..FlightInput::default()
            })
            .unwrap();
    }
    panic!("public brake did not stop");
}
fn practice(seed: u64) -> World {
    let mut world = World::new_commander(seed).unwrap();
    consent(&mut world);
    launch(&mut world);
    world
}
pub(in crate::flight) fn fire(world: &mut World, target: PracticeBeaconId) -> MissileOrigin {
    let tick = world.snapshot().tick;
    world.request_missile_lock(Some(target), tick).unwrap();
    world.request_missile_arm(true, tick).unwrap();
    world.request_fire_missile(tick).unwrap()
}
fn projectile(world: &World, id: ProjectileId) -> ProjectileSnapshot {
    world
        .missile_snapshot()
        .unwrap()
        .projectiles
        .into_iter()
        .flatten()
        .find(|p| p.origin.projectile == id)
        .unwrap()
}
pub(in crate::flight) fn finish(world: &mut World, id: ProjectileId) -> ProjectileSnapshot {
    for _ in 0..500 {
        let p = projectile(world, id);
        if p.ended.is_some() {
            return p;
        }
        step(world, 1);
    }
    panic!("finite projectile did not end");
}
pub(in crate::flight) fn fly_axis(world: &mut World, axis: usize, value: f64, speed: f32) {
    stop(world);
    let opening = world.snapshot().position;
    let sign = (value - f64::from(opening[axis])).signum();
    let mut point = opening.map(f64::from);
    point[axis] = value;
    aim_at(world, Vec3::new(point[0], point[1], point[2]));
    for _ in 0..30000 {
        if sign * (f64::from(world.snapshot().position[axis]) - value) >= 0.0 {
            stop(world);
            return;
        }
        let current = world.snapshot().speed;
        world
            .step(FlightInput {
                throttle: ((speed - current) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..FlightInput::default()
            })
            .unwrap();
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    panic!("public axis flight did not reach waypoint");
}
pub(in crate::flight) fn dock(world: &mut World) {
    stop(world);
    aim_at(world, Vec3::new(0.0, 0.0, 0.0));
    for _ in 0..9000 {
        let s = world.snapshot();
        if s.mode == FlightMode::Docked {
            return;
        }
        let speed = if s.position[2] > 2500.0 { 34.3 } else { 4.0 };
        world
            .step(FlightInput {
                throttle: ((speed - s.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..FlightInput::default()
            })
            .unwrap();
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    panic!("real physical entry did not dock");
}

#[test]
fn actual_three_pylon_items_require_lock_arm_front_and_exact_finite_consumption() {
    let mut world = practice(1601);
    stop(&mut world);
    let ids = world
        .practice_snapshot()
        .unwrap()
        .beacons
        .unwrap()
        .map(|p| p.id);
    let stock = world.missile_snapshot().unwrap();
    assert_eq!(stock.rounds.map(|p| p.unwrap().item), [1609, 1610, 1611]);
    assert_eq!(
        world.request_fire_missile(0),
        Err(MissileError::StaleCommand)
    );
    assert_eq!(
        world.request_fire_missile(world.snapshot().tick),
        Err(MissileError::NotArmed)
    );
    world
        .request_missile_arm(true, world.snapshot().tick)
        .unwrap();
    let before = world.missile_snapshot();
    assert_eq!(
        world.request_fire_missile(world.snapshot().tick),
        Err(MissileError::NoLiveLock)
    );
    assert_eq!(world.missile_snapshot(), before);
    let other = practice(1602).practice_snapshot().unwrap().beacons.unwrap()[1].id;
    assert_eq!(
        world.request_missile_lock(Some(other), world.snapshot().tick),
        Err(MissileError::NoLiveLock)
    );
    assert_eq!(world.missile_snapshot(), before);
    world
        .request_missile_lock(Some(ids[1]), world.snapshot().tick)
        .unwrap();
    aim_at(&mut world, Vec3::new(70.0, 0.0, 136.32));
    let before = world.missile_snapshot();
    assert_eq!(
        world.request_fire_missile(world.snapshot().tick),
        Err(MissileError::TargetBehind)
    );
    assert_eq!(world.missile_snapshot(), before);
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1]
        .pose
        .position;
    aim_at(&mut world, target);
    let credits = world.credits_tenths();
    let equipment = world.equipment_snapshot();
    let pulse = world.pulse_laser_snapshot();
    let first = world.request_fire_missile(world.snapshot().tick).unwrap();
    assert_eq!(first.item, 1611);
    assert_eq!(first.legal_at_launch, 0);
    assert_eq!(first.projectile.entity(), 5);
    assert_eq!(world.pulse_laser_snapshot(), pulse);
    assert_eq!(world.credits_tenths(), credits);
    assert_eq!(world.equipment_snapshot(), equipment);
    assert_eq!(
        world
            .missile_snapshot()
            .unwrap()
            .rounds
            .iter()
            .flatten()
            .count(),
        2
    );
    world
        .request_missile_arm(true, world.snapshot().tick)
        .unwrap();
    let before = world.missile_snapshot();
    assert_eq!(
        world.request_fire_missile(world.snapshot().tick),
        Err(MissileError::CoolingDown)
    );
    assert_eq!(world.missile_snapshot(), before);
    step(&mut world, 59);
    assert_eq!(
        world.request_fire_missile(world.snapshot().tick),
        Err(MissileError::CoolingDown)
    );
    step(&mut world, 1);
    let second = world.request_fire_missile(world.snapshot().tick).unwrap();
    assert_eq!(second.item, 1610);
    step(&mut world, 60);
    let third = fire(&mut world, ids[1]);
    assert_eq!(third.item, 1609);
    assert!(first.emission < second.emission && second.emission < third.emission);
    assert_ne!(second.projectile, third.projectile);
    assert_eq!(world.missile_snapshot().unwrap().rounds, [None; 3]);
    let before = world.missile_snapshot();
    assert_eq!(
        world.request_missile_arm(true, world.snapshot().tick),
        Err(MissileError::NoAmmunition)
    );
    assert_eq!(world.missile_snapshot(), before);
}

#[test]
fn actual_swept_beacon_impact_has_no_inherited_velocity_and_keeps_first_origin() {
    let mut world = practice(1621);
    let beacon = world.practice_snapshot().unwrap().beacons.unwrap()[1];
    assert!(world.snapshot().speed > 0.0);
    let origin = fire(&mut world, beacon.id);
    let birth = projectile(&world, origin.projectile);
    assert_eq!(birth.position, origin.muzzle);
    assert_eq!(birth.age_ticks, 0);
    assert_eq!(birth.propulsion_travel, 0.0);
    assert!((origin.muzzle.z - origin.launch_position.z - 24.0).abs() < 1e-9);
    step(&mut world, 1);
    let moved = projectile(&world, origin.projectile);
    assert!((length(sub(moved.position, birth.position)) - 13.2).abs() < 1e-9);
    assert!((moved.propulsion_travel - 13.2).abs() < 1e-9);
    assert!(!moved.muzzle_cleared);
    stop(&mut world);
    let ended = finish(&mut world, origin.projectile);
    assert_eq!(ended.ended, Some(MissileEnd::Impact));
    assert_eq!(
        ended.contact,
        Some(MissileContact::Practice {
            id: beacon.id,
            first_hit: true
        })
    );
    assert!((ended.position.z - (beacon.pose.position.z - 3.0)).abs() < 1e-8);
    assert!((ended.propulsion_travel - 1773.0).abs() < 1e-8);
    assert_eq!(
        world.practice_snapshot().unwrap().beacons.unwrap()[1].missile_hit,
        Some(origin)
    );
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 1);
    assert_eq!(world.legal_value(), Some(0));
    assert!(
        world
            .combat_snapshot()
            .unwrap()
            .last_ordinary_impact
            .is_none()
    );
    let second = fire(&mut world, beacon.id);
    let repeated = finish(&mut world, second.projectile);
    assert_eq!(
        repeated.contact,
        Some(MissileContact::Practice {
            id: beacon.id,
            first_hit: false
        })
    );
    assert_eq!(
        world.practice_snapshot().unwrap().beacons.unwrap()[1].missile_hit,
        Some(origin)
    );
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 1);
}

#[test]
fn actual_pending_missile_blocks_quiet_credit_and_end_starts_fresh_interval() {
    let mut world = practice(1631);
    let beacons = world.practice_snapshot().unwrap().beacons.unwrap();
    assert!(matches!(
        shoot(&mut world).contact,
        PulseContact::PracticeBeacon {
            first_hit: true,
            ..
        }
    ));
    stop(&mut world);
    for ordinal in [0, 2] {
        aim_at(&mut world, beacons[ordinal].pose.position);
        shoot(&mut world);
        step(&mut world, 18);
    }
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 3);
    fly_axis(&mut world, 0, 5000.0, 4.0);
    assert!(world.practice_reception_snapshot().unwrap().ready);
    aim_at(&mut world, beacons[1].pose.position);
    let origin = fire(&mut world, beacons[1].id);
    assert_eq!(world.practice_reception_snapshot().unwrap().quiet_ticks, 0);
    let tick = world.snapshot().tick;
    let moving = world.missile_snapshot();
    let condition = world.combat_snapshot();
    assert_eq!(
        world.step(FlightInput {
            yaw: f32::NAN,
            ..FlightInput::default()
        }),
        Err(StepError::NonFiniteInput)
    );
    assert_eq!(world.missile_snapshot(), moving);
    assert_eq!(world.combat_snapshot(), condition);
    world.request_pause(true, tick).unwrap();
    let before = world.missile_snapshot();
    assert_eq!(world.step(FlightInput::default()), Err(StepError::Paused));
    assert_eq!(
        world.request_missile_arm(false, tick),
        Err(MissileError::Paused)
    );
    assert_eq!(world.missile_snapshot(), before);
    world.request_pause(false, tick).unwrap();
    step(&mut world, 180);
    assert!(projectile(&world, origin.projectile).ended.is_none());
    assert_eq!(world.practice_reception_snapshot().unwrap().quiet_ticks, 0);
    assert!(!world.practice_reception_snapshot().unwrap().ready);
    let ended = finish(&mut world, origin.projectile);
    assert_eq!(ended.ended, Some(MissileEnd::Impact));
    assert_eq!(world.practice_reception_snapshot().unwrap().quiet_ticks, 0);
    step(&mut world, 179);
    assert!(!world.practice_reception_snapshot().unwrap().ready);
    step(&mut world, 1);
    assert!(world.practice_reception_snapshot().unwrap().ready);
}

#[test]
fn actual_pulse_intercepts_owned_programmatic_missile_body_without_refunding_round() {
    let mut world = practice(1641);
    stop(&mut world);
    let id = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let origin = fire(&mut world, id);
    let shot = shoot(&mut world);
    assert!(matches!(shot.contact,PulseContact::Projectile{id,..} if id==origin.projectile));
    let ended = projectile(&world, origin.projectile);
    assert_eq!(ended.ended, Some(MissileEnd::Intercepted));
    assert!(ended.intercepting_emission.unwrap() > origin.emission);
    assert_eq!(
        world
            .missile_snapshot()
            .unwrap()
            .rounds
            .iter()
            .flatten()
            .count(),
        2
    );
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 0);
    assert_eq!(world.legal_value(), Some(0));
    assert!(
        world
            .combat_snapshot()
            .unwrap()
            .last_ordinary_impact
            .is_none()
    );
}

#[test]
fn actual_ordinary_pulse_failure_ends_a_live_guided_target_without_ammo_repair() {
    let mut world = practice(1651);
    stop(&mut world);
    fly_axis(&mut world, 0, 3000.0, 4.0);
    let id = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    aim_at(&mut world, Vec3::new(70.0, 0.0, 136.32));
    let origin = fire(&mut world, id);
    step(&mut world, 10);
    let guided = projectile(&world, origin.projectile);
    assert!(guided.ended.is_none());
    assert!(length(sub(guided.direction, guided.origin.orientation.z)) > 0.1);
    // The body retains native launch orientation while propulsion snaps to target.
    assert_eq!(guided.origin.orientation, origin.orientation);
    assert!(matches!(
        shoot(&mut world).contact,
        PulseContact::StationBlocked { .. }
    ));
    assert_eq!(
        world.practice_snapshot().unwrap().status,
        PracticeStatus::Failed
    );
    step(&mut world, 1);
    assert_eq!(
        projectile(&world, origin.projectile).ended,
        Some(MissileEnd::TargetLost)
    );
    assert_eq!(
        world
            .missile_snapshot()
            .unwrap()
            .rounds
            .iter()
            .flatten()
            .count(),
        2
    );
    assert_eq!(world.practice_reception_snapshot().unwrap().quiet_ticks, 0);
}

#[test]
fn actual_out_of_range_lock_expires_at_six_thousand_before_the_lifetime_bound() {
    let mut world = practice(1661);
    stop(&mut world);
    fly_axis(&mut world, 0, 7000.0, 4.0);
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1];
    aim_at(&mut world, target.pose.position);
    let origin = fire(&mut world, target.id);
    assert!(length(sub(target.pose.position, origin.muzzle)) > 6000.0);
    let ended = finish(&mut world, origin.projectile);
    assert_eq!(ended.ended, Some(MissileEnd::RangeExpired));
    assert_eq!(ended.propulsion_travel, 6000.0);
    assert_eq!(ended.age_ticks, 455);
    assert!(ended.age_ticks < MISSILE_LIFETIME_TICKS);
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 0);
    assert_eq!(world.legal_value(), Some(0));
    let before = projectile(&world, origin.projectile);
    step(&mut world, 60);
    assert_eq!(projectile(&world, origin.projectile), before);
}

#[test]
fn actual_missile_hits_nearest_native_station_and_records_ordinary_failure() {
    let mut world = practice(1671);
    stop(&mut world);
    fly_axis(&mut world, 0, 3000.0, 4.0);
    fly_axis(&mut world, 2, -3000.0, 4.0);
    fly_axis(&mut world, 0, 120.0, 1.0);
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[2];
    aim_at(&mut world, target.pose.position);
    let origin = fire(&mut world, target.id);
    let ended = finish(&mut world, origin.projectile);
    assert_eq!(ended.ended, Some(MissileEnd::Impact));
    assert_eq!(ended.contact, Some(MissileContact::Station));
    let hit = world
        .combat_snapshot()
        .unwrap()
        .last_ordinary_impact
        .unwrap();
    assert_eq!(hit.weapon, super::super::OrdinaryWeapon::StandardMissile);
    assert_eq!(hit.emission, origin.emission);
    assert_eq!(hit.item, origin.item);
    assert_eq!(hit.target, super::super::OrdinaryTarget::MainStation);
    assert_eq!(hit.victim_entity, 1);
    assert_eq!(hit.point, ended.position);
    assert_eq!(world.legal_value(), Some(64));
    assert_eq!(
        world.practice_snapshot().unwrap().status,
        PracticeStatus::Failed
    );
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 0);
    assert_eq!(
        world.combat_snapshot().unwrap().player.energy_hundredths,
        38_000
    );
}

#[test]
fn actual_spent_custody_continues_and_preserves_funded_paid_gear() {
    let mut world = crate::flight::equipment::tests::earned_world(1681);
    // Earned funds finish at Renas. Return through charged public travel before consent.
    if world.snapshot().navigation.unwrap().fuel_units < 68 {
        world.request_refuel(world.snapshot().tick).unwrap();
    }
    crate::flight::equipment::tests::journey(&mut world, crate::navigation::SystemId::AVENOR);
    assert_eq!(world.snapshot().mode, crate::FlightMode::Docked);
    assert_eq!(
        world.snapshot().navigation.unwrap().system,
        crate::navigation::SystemId::AVENOR
    );
    world
        .request_equipment(
            crate::equipment::Command::PurchaseCargoBay,
            world.snapshot().tick,
        )
        .unwrap();
    let paid = world.equipment_snapshot().unwrap().owned.unwrap();
    let cash = world.credits_tenths();
    consent(&mut world);
    launch(&mut world);
    stop(&mut world);
    let saved = world.save_bytes().unwrap();
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    world
        .request_missile_lock(Some(target), world.snapshot().tick)
        .unwrap();
    assert!(world.save_bytes().is_ok());
    world
        .request_missile_lock(None, world.snapshot().tick)
        .unwrap();
    assert!(world.save_bytes().is_ok());
    let origin = fire(&mut world, target);
    let before = world.missile_snapshot();
    let active = world.save_bytes().unwrap();
    let mut corrupt = active.clone();
    corrupt[0] ^= 1;
    assert!(world.restore_bytes(&corrupt).is_err());
    assert_eq!(world.missile_snapshot(), before);
    world = World::from_save_bytes(&active).unwrap();
    assert_eq!(world.missile_snapshot(), before);
    assert_eq!(
        World::from_save_bytes(&saved)
            .unwrap()
            .missile_snapshot()
            .unwrap()
            .rounds
            .iter()
            .flatten()
            .count(),
        3
    );
    finish(&mut world, origin.projectile);
    dock(&mut world);
    assert_eq!(world.equipment_snapshot().unwrap().owned, Some(paid));
    assert_eq!(world.credits_tenths(), cash);
    world
        .request_equipment(
            crate::equipment::Command::UnfitCargoBay(paid.id),
            world.snapshot().tick,
        )
        .unwrap();
    let stored = world.equipment_snapshot().unwrap().owned.unwrap();
    assert_eq!(stored.id, paid.id);
    assert!(!stored.fitted);
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(
        world
            .missile_snapshot()
            .unwrap()
            .rounds
            .iter()
            .flatten()
            .count(),
        2
    );
    let docked = world.save_bytes().unwrap();
    world = World::from_save_bytes(&docked).unwrap();
    assert_eq!(world.equipment_snapshot().unwrap().owned, Some(stored));
    assert_eq!(world.credits_tenths(), cash);
    launch(&mut world);
    assert_eq!(
        world
            .missile_snapshot()
            .unwrap()
            .rounds
            .iter()
            .flatten()
            .count(),
        2
    );
    assert_eq!(world.missile_snapshot().unwrap().cooldown_ticks, 0);
}

#[test]
fn pure_source_geometry_retains_missile_faces_and_player_surface_without_granting_an_impact() {
    assert_eq!(MISSILE_VERTICES.len(), 7);
    assert_eq!(MISSILE_FACES.len(), 8);
    assert_eq!(
        MISSILE_FACES
            .iter()
            .map(|face| face.len() - 2)
            .sum::<usize>(),
        10
    );
    let shape = pose(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0)).unwrap();
    let ray = pose(Vec3::new(0.0, 0.0, -100.0), Vec3::new(0.0, 0.0, 1.0)).unwrap();
    assert!(mesh_distance(shape, ray, 200.0).is_some());
    let data = crate::OriginalStationData::bundled().unwrap();
    // The original hull has a centre opening; an offset ray meets its actual rear surface.
    assert_eq!(
        data.player_beam_distance(shape, ray.position, ray.orientation.z, 200.0)
            .unwrap(),
        None
    );
    let local_origin = Vec3::new(0.0, 5.0, -100.0);
    let expected = 52.30711098924325;
    let distance = data
        .player_beam_distance(shape, local_origin, ray.orientation.z, 200.0)
        .unwrap()
        .unwrap();
    assert!((distance - expected).abs() < 1e-9);
    assert_eq!(
        data.player_beam_distance(shape, local_origin, ray.orientation.z, 52.0)
            .unwrap(),
        None
    );
    let moved = pose(Vec3::new(40.0, -30.0, 70.0), Vec3::new(1.0, 0.0, 0.0)).unwrap();
    let distance = data
        .player_beam_distance(
            moved,
            add(moved.position, moved.orientation.apply(local_origin)),
            moved.orientation.z,
            200.0,
        )
        .unwrap()
        .unwrap();
    assert!((distance - expected).abs() < 1e-9);
    // A pure native-hull ray is not a live player-hit acceptance or mission receipt.
}

#[test]
fn untouched_current_controller_keeps_owned_rounds_and_refuses_v4_through_v7() {
    let world = World::new_commander(1691).unwrap();
    let expected = world.missile_snapshot().unwrap();
    let saves = [
        crate::flight::persistence::v4_test_fixture(&world).unwrap(),
        crate::flight::persistence::v5_test_fixture(&world).unwrap(),
        crate::flight::persistence::v6_test_fixture(&world).unwrap(),
        crate::flight::persistence::v7_test_fixture(&world).unwrap(),
    ];
    for bytes in saves {
        assert_eq!(
            World::from_save_bytes(&bytes).unwrap_err(),
            crate::save::SaveError::UnsupportedVersion
        );
    }
    let loaded = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(loaded.missile_snapshot(), Some(expected));
    assert_eq!(loaded.credits_tenths(), world.credits_tenths());
    assert_eq!(loaded.equipment_snapshot(), world.equipment_snapshot());
    assert_eq!(
        loaded
            .missile_snapshot()
            .unwrap()
            .rounds
            .map(|r| r.unwrap().item),
        [1699, 1700, 1701]
    );
}

mod durable;

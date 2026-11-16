#![allow(clippy::unwrap_used)]

use super::*;
use crate::docking::Basis3;
use crate::navigation::BASIC_SPOOL_TICKS;
use crate::{FlightInput, PulseContact};

fn consent(world: &mut World) {
    let tick = world.snapshot().tick;
    let review = world.review_practice(tick).unwrap();
    let prepared = world.prepare_practice_confirmation(review, tick).unwrap();
    prepared.commit();
}

fn step(world: &mut World, count: u8) {
    for _ in 0..count {
        world.step(FlightInput::default()).unwrap();
    }
}

fn launch(world: &mut World) {
    world.request_launch(world.snapshot().tick).unwrap();
    step(world, 1);
}

fn shoot(world: &mut World) -> crate::PulseShot {
    world
        .request_fire_front_pulse(world.snapshot().tick)
        .unwrap()
}

#[test]
fn actual_avenor_review_is_bound_to_profile_tick_epoch_and_publication() {
    let mut world = World::new_commander(561).unwrap();
    let mut other = World::new_commander(562).unwrap();
    let review = world.review_practice(0).unwrap();
    assert_eq!(
        World::new().review_practice(0),
        Err(PracticeError::NoCommander)
    );
    assert!(matches!(
        other.prepare_practice_confirmation(review, 0),
        Err(PracticeError::StaleReview)
    ));
    let before = world.save_bytes().unwrap();
    let prepared = world.prepare_practice_confirmation(review, 0).unwrap();
    let durable = prepared.bytes().to_vec();
    let loaded = World::from_save_bytes(&durable).unwrap();
    assert_eq!(
        loaded.practice_snapshot().unwrap().status,
        PracticeStatus::Consented
    );
    assert!(loaded.practice_snapshot().unwrap().beacons.is_none());
    drop(prepared);
    assert_eq!(world.save_bytes().unwrap(), before);
    assert_eq!(
        world.practice_snapshot().unwrap().status,
        PracticeStatus::Unaccepted
    );
    world.restore_bytes(&before).unwrap();
    assert!(matches!(
        world.prepare_practice_confirmation(review, 0),
        Err(PracticeError::StaleReview)
    ));
    let refreshed = world.review_practice(0).unwrap();
    world.request_pause(true, 0).unwrap();
    assert!(matches!(
        world.prepare_practice_confirmation(refreshed, 0),
        Err(PracticeError::Paused)
    ));
    world.request_pause(false, 0).unwrap();
    let refreshed = world.review_practice(0).unwrap();
    step(&mut world, 1);
    assert!(matches!(
        world.prepare_practice_confirmation(refreshed, 0),
        Err(PracticeError::StaleReview)
    ));
    let prepared = world.prepare_practice_confirmation(refreshed, 1).unwrap();
    let durable = prepared.bytes().to_vec();
    prepared.commit();
    assert_eq!(world.save_bytes().unwrap(), durable);
    assert_eq!(
        world.review_practice(1),
        Err(PracticeError::AlreadyConsented)
    );
    assert_eq!(world.credits_tenths(), Some(7200));
}

fn aim_at(world: &mut World, target: Vec3) {
    let snapshot = world.snapshot();
    assert_eq!(snapshot.speed, 0.0);
    let pose = flight_pose(snapshot.position, snapshot.orientation).unwrap();
    let aim = (target.x - pose.position.x).atan2(target.z - pose.position.z);
    let yaw = (aim - pose.orientation.z.x.atan2(pose.orientation.z.z)) as f32;
    let ticks = (yaw.abs() / (0.65 / 60.0)).ceil() as u8;
    for _ in 0..ticks {
        world
            .step(FlightInput {
                yaw: yaw / (0.65 / 60.0) / f32::from(ticks),
                ..FlightInput::default()
            })
            .unwrap();
    }
}

#[test]
fn launch_freezes_three_real_targets_and_first_hits_survive_continue_without_repeat_credit() {
    let mut world = World::new_commander(571).unwrap();
    consent(&mut world);
    world.request_launch(0).unwrap();
    assert!(world.practice_snapshot().unwrap().beacons.is_none());
    step(&mut world, 1);
    let opening = world.snapshot();
    let beacons = world.practice_snapshot().unwrap().beacons.unwrap();
    assert_eq!(beacons.map(|beacon| beacon.id.ordinal()), [0, 1, 2]);
    assert_eq!(
        beacons.map(|beacon| beacon.pose.position.x),
        [-120.0, 0.0, 120.0]
    );
    assert_eq!(
        beacons.map(|beacon| beacon.pose.position.z - f64::from(opening.position[2])),
        [1500.0, 1800.0, 2100.0]
    );
    let first = shoot(&mut world);
    assert_eq!(
        first.contact,
        PulseContact::PracticeBeacon {
            id: beacons[1].id,
            distance: 1797.0,
            first_hit: true
        }
    );
    assert!(world.practice_snapshot().unwrap().beacons.unwrap()[1].flashing);
    let tick = world.snapshot().tick;
    world.request_pause(true, tick).unwrap();
    assert_eq!(
        world.step(FlightInput::default()),
        Err(crate::StepError::Paused)
    );
    assert!(world.practice_snapshot().unwrap().beacons.unwrap()[1].flashing);
    world.request_pause(false, tick).unwrap();
    step(&mut world, 9);
    assert!(world.practice_snapshot().unwrap().beacons.unwrap()[1].flashing);
    step(&mut world, 1);
    assert!(!world.practice_snapshot().unwrap().beacons.unwrap()[1].flashing);
    let mut world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let restored = world.practice_snapshot().unwrap().beacons.unwrap();
    assert_eq!(
        restored.map(|beacon| beacon.id),
        beacons.map(|beacon| beacon.id)
    );
    step(&mut world, 8);
    let repeat = shoot(&mut world);
    assert!(
        matches!(repeat.contact,PulseContact::PracticeBeacon{id,first_hit:false,..} if id==beacons[1].id)
    );
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 1);
    for _ in 0..60 {
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
    for ordinal in [0, 2] {
        aim_at(&mut world, beacons[ordinal].pose.position);
        let hit = shoot(&mut world);
        assert!(
            matches!(hit.contact,PulseContact::PracticeBeacon{id,first_hit:true,..} if id==beacons[ordinal].id)
        );
        step(&mut world, crate::PULSE_COOLDOWN_TICKS);
    }
    let snapshot = world.practice_snapshot().unwrap();
    assert_eq!(snapshot.first_hits, 3);
    assert_eq!(
        World::from_save_bytes(&world.save_bytes().unwrap())
            .unwrap()
            .practice_snapshot(),
        Some(snapshot)
    );
    assert_eq!(world.credits_tenths(), Some(7200));
    assert_eq!(world.mission_snapshot().unwrap().freight.completed_jobs, 0);
}

#[test]
fn arrival_retires_the_original_range_and_away_berth_cannot_offer_it() {
    let mut world = World::new_commander(581).unwrap();
    consent(&mut world);
    launch(&mut world);
    shoot(&mut world);
    world
        .request_jump(SystemId::RENAS, world.snapshot().tick)
        .unwrap();
    for _ in 0..BASIC_SPOOL_TICKS {
        step(&mut world, 1);
    }
    let retired = world.practice_snapshot().unwrap();
    assert_eq!(
        retired,
        PracticeSnapshot {
            status: PracticeStatus::Retired,
            beacons: None,
            first_hits: 1
        }
    );
    let mut world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(world.practice_snapshot(), Some(retired));
    assert_eq!(shoot(&mut world).contact, PulseContact::Clear);
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
    }
    assert_eq!(
        world.current_dock_context().unwrap().system(),
        SystemId::RENAS
    );
    assert_eq!(
        world.review_practice(world.snapshot().tick),
        Err(PracticeError::AvenorBerthRequired)
    );
}

#[test]
fn source_cube_order_is_joint_with_the_real_native_station_triangle() {
    let station = OriginalStationData::bundled().unwrap();
    let ray = flight_pose([70.0, 0.0, 500.0], [0.0, 1.0, 0.0, 0.0]).unwrap();
    let distance = station
        .beam_block_distance(
            station.station_pose(),
            ray.position,
            ray.orientation.z,
            crate::PULSE_RANGE,
        )
        .unwrap();
    assert!((distance.unwrap() - 363.68).abs() < 1e-9);
    // Test-only frames bracket the native first hit and never enter World.
    let behind = RigidPose {
        position: Vec3::new(70.0, 0.0, -2000.0),
        orientation: Basis3::IDENTITY,
    };
    assert_eq!(geometry::nearest(behind, ray, None), Some((1, 697.0)));
    assert_eq!(geometry::nearest(behind, ray, distance), None);
    let before = RigidPose {
        position: Vec3::new(70.0, 0.0, -1500.0),
        orientation: Basis3::IDENTITY,
    };
    assert_eq!(geometry::nearest(before, ray, distance), Some((1, 197.0)));
    assert_eq!(geometry::nearest(before, ray, Some(197.0)), None);

    let unsafe_station = RigidPose {
        position: Vec3::new(0.0, 0.0, -1800.0),
        orientation: Basis3::IDENTITY,
    };
    let unsafe_planet = RigidPose {
        position: Vec3::new(2600.0, -700.0, 5700.0),
        orientation: Basis3::IDENTITY,
    };
    let unsafe_sun = RigidPose {
        position: Vec3::new(-5000.0, 1500.0, 10700.0),
        orientation: Basis3::IDENTITY,
    };
    for pose in [unsafe_station, unsafe_planet, unsafe_sun] {
        assert!(!geometry::range_clear(
            &station,
            station.station_pose(),
            pose
        ));
    }
}

#[test]
fn resealed_range_authority_and_hit_pose_changes_refuse_without_live_replacement() {
    use crate::save::{Writer, checksum};
    let mut world = World::new_commander(591).unwrap();
    consent(&mut world);
    launch(&mut world);
    shoot(&mut world);
    let live = world.save_bytes().unwrap();
    let practice = world.practice.unwrap();
    let mut encoded = Writer::new();
    practice.encode(&mut encoded);
    let start = live.len() - 8 - 5 - encoded.bytes.len();
    assert_eq!(&live[start..start + encoded.bytes.len()], encoded.bytes);
    let mut changed_opening = practice;
    changed_opening.range.as_mut().unwrap().opening.position = [2600.0, -700.0, 5700.0];
    let mut changed_hit = practice;
    changed_hit.range.as_mut().unwrap().hits[1]
        .as_mut()
        .unwrap()
        .firing_pose
        .orientation = [0.0, 1.0, 0.0, 0.0];
    let mut changed_tick = practice;
    changed_tick.range.as_mut().unwrap().hits[1]
        .as_mut()
        .unwrap()
        .tick += 1;
    let mut foreign = practice;
    foreign.consent = World::new_commander(592)
        .unwrap()
        .current_dock_context()
        .unwrap();
    for candidate in [changed_opening, changed_hit, changed_tick, foreign] {
        let mut encoded = Writer::new();
        candidate.encode(&mut encoded);
        let mut dirty = live.clone();
        dirty[start..start + encoded.bytes.len()].copy_from_slice(&encoded.bytes);
        let end = dirty.len() - 8;
        let digest = checksum(&dirty[..end]);
        dirty[end..].copy_from_slice(&digest.to_le_bytes());
        assert!(world.restore_bytes(&dirty).is_err());
        assert_eq!(world.save_bytes().unwrap(), live);
    }
}

fn assert_resealed_refusal(world: &mut World, live: &[u8], mut changed: Vec<u8>) {
    let end = changed.len() - 8;
    let digest = crate::save::checksum(&changed[..end]);
    changed[end..].copy_from_slice(&digest.to_le_bytes());
    assert_eq!(
        world.restore_bytes(&changed),
        Err(crate::save::SaveError::InvalidState(
            "T02 practice consent, visit or receipts"
        ))
    );
    assert_eq!(world.save_bytes().unwrap().as_slice(), live);
}

fn clear_pulse_tail(bytes: &mut [u8]) {
    let start = bytes.len() - 13;
    bytes[start..start + 2].copy_from_slice(&38_000_u16.to_le_bytes());
    bytes[start + 2..start + 5].fill(0);
}

#[test]
fn practice_receipt_requires_committed_pulse_and_exact_same_tick_flight_pose() {
    use crate::save::Writer;
    let mut world = World::new_commander(601).unwrap();
    consent(&mut world);
    launch(&mut world);
    shoot(&mut world);
    let live = world.save_bytes().unwrap();
    let mut tail_only = live.clone();
    clear_pulse_tail(&mut tail_only);
    assert_resealed_refusal(&mut world, &live, tail_only);

    let practice = world.practice.unwrap();
    let mut mutated_opening = practice;
    mutated_opening.range.as_mut().unwrap().opening.position[1] += 1.0;
    let mut mutated_hit = practice;
    mutated_hit.range.as_mut().unwrap().hits[1]
        .as_mut()
        .unwrap()
        .firing_pose
        .position[1] += 1.0;
    // Both one-metre shifts still let the same ray hit the central 6m cube.
    for mutated in [mutated_opening, mutated_hit] {
        let mut bytes = Writer::new();
        mutated.encode(&mut bytes);
        let end = live.len() - 13;
        let mut inconsistent = live.clone();
        inconsistent[end - bytes.bytes.len()..end].copy_from_slice(&bytes.bytes);
        assert_resealed_refusal(&mut world, &live, inconsistent);
    }
    let continued = World::from_save_bytes(&live).unwrap();
    assert_eq!(continued.practice_snapshot(), world.practice_snapshot());
    assert_eq!(
        continued.pulse_laser_snapshot(),
        world.pulse_laser_snapshot()
    );
}

#[test]
fn practice_hit_keeps_cooldown_floor_after_actual_hyperspace_retirement() {
    let mut world = World::new_commander(611).unwrap();
    consent(&mut world);
    launch(&mut world);
    for _ in 0..60 {
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
    world
        .request_jump(SystemId::RENAS, world.snapshot().tick)
        .unwrap();
    for _ in 1..BASIC_SPOOL_TICKS {
        step(&mut world, 1);
    }
    assert!(matches!(
        shoot(&mut world).contact,
        PulseContact::PracticeBeacon {
            first_hit: true,
            ..
        }
    ));
    step(&mut world, 1);
    assert_eq!(
        world.practice_snapshot().unwrap().status,
        PracticeStatus::Retired
    );
    assert_eq!(world.pulse_laser_snapshot().unwrap().cooldown_ticks, 17);
    let live = world.save_bytes().unwrap();
    let mut contradiction = live.clone();
    clear_pulse_tail(&mut contradiction);
    assert_resealed_refusal(&mut world, &live, contradiction);
    let mut continued = World::from_save_bytes(&live).unwrap();
    assert_eq!(continued.practice_snapshot(), world.practice_snapshot());
    step(&mut continued, 17);
    assert_eq!(continued.pulse_laser_snapshot().unwrap().cooldown_ticks, 0);
    assert!(continued.save_bytes().is_ok());
}

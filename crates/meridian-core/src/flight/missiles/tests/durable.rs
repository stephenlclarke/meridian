//! Public-command positive continuations; private mutations are negative decoder fixtures only.
use super::*;
use crate::save::{SAVE_VERSION, Writer, checksum};

fn same(a: &World, b: &World) {
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.missile_snapshot(), b.missile_snapshot());
    assert_eq!(a.combat_snapshot(), b.combat_snapshot());
    assert_eq!(a.pulse_laser_snapshot(), b.pulse_laser_snapshot());
    assert_eq!(a.practice_snapshot(), b.practice_snapshot());
    assert_eq!(
        a.practice_reception_snapshot(),
        b.practice_reception_snapshot()
    );
    assert_eq!(a.equipment_snapshot(), b.equipment_snapshot());
    assert_eq!(a.credits_tenths(), b.credits_tenths());
    assert_eq!(a.legal_value(), b.legal_value());
    assert_eq!(a.is_paused(), b.is_paused());
}
fn cold(world: &World) -> World {
    let bytes = world.save_bytes().unwrap();
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), SAVE_VERSION);
    let loaded = World::from_save_bytes(&bytes).unwrap();
    same(world, &loaded);
    loaded
}
fn together(a: &mut World, b: &mut World, count: u16) {
    for i in 0..count {
        let input = FlightInput {
            throttle: if i < 10 { 0.1 } else { 0.0 },
            ..FlightInput::default()
        };
        a.step(input).unwrap();
        b.step(input).unwrap();
        same(a, b);
    }
}
fn active(seed: u64) -> (World, MissileOrigin) {
    let mut world = practice(seed);
    stop(&mut world);
    let id = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let origin = fire(&mut world, id);
    (world, origin)
}

#[test]
fn public_lock_arm_launch_and_first_impact_continue_without_reemission() {
    let mut world = practice(2601);
    stop(&mut world);
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    world
        .request_missile_lock(Some(target), world.snapshot().tick)
        .unwrap();
    world
        .request_missile_arm(true, world.snapshot().tick)
        .unwrap();
    let mut loaded = cold(&world);
    let a = world.request_fire_missile(world.snapshot().tick).unwrap();
    let b = loaded.request_fire_missile(loaded.snapshot().tick).unwrap();
    assert_eq!(a, b);
    same(&world, &loaded);
    loaded = cold(&loaded); // actual age-zero live actor and consumed item
    together(&mut world, &mut loaded, 40);
    loaded = cold(&loaded); // active guided motion, cooldown and clearance
    together(&mut world, &mut loaded, 120);
    assert_eq!(
        projectile(&world, a.projectile).ended,
        Some(MissileEnd::Impact)
    );
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 1);
    let first = world.practice_snapshot().unwrap().beacons.unwrap()[1].missile_hit;
    loaded = cold(&loaded);
    together(&mut world, &mut loaded, 200);
    assert_eq!(loaded.practice_snapshot().unwrap().first_hits, 1);
    assert_eq!(
        loaded.practice_snapshot().unwrap().beacons.unwrap()[1].missile_hit,
        first
    );
    assert_eq!(
        loaded
            .missile_snapshot()
            .unwrap()
            .rounds
            .iter()
            .flatten()
            .count(),
        2
    );
}

#[test]
fn pending_pause_and_true_end_resume_the_same_quiet_interval() {
    let mut world = practice(2611);
    let beacons = world.practice_snapshot().unwrap().beacons.unwrap();
    shoot(&mut world);
    stop(&mut world);
    for i in [0, 2] {
        aim_at(&mut world, beacons[i].pose.position);
        shoot(&mut world);
        step(&mut world, 18);
    }
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 3);
    fly_axis(&mut world, 0, 5000.0, 4.0);
    aim_at(&mut world, beacons[1].pose.position);
    let origin = fire(&mut world, beacons[1].id);
    step(&mut world, 180);
    assert!(projectile(&world, origin.projectile).ended.is_none());
    world.request_pause(true, world.snapshot().tick).unwrap();
    let mut loaded = cold(&world);
    let before = loaded.save_bytes().unwrap();
    assert_eq!(loaded.step(FlightInput::default()), Err(StepError::Paused));
    assert_eq!(loaded.save_bytes().unwrap(), before);
    assert_eq!(loaded.practice_reception_snapshot().unwrap().quiet_ticks, 0);
    world.request_pause(false, world.snapshot().tick).unwrap();
    loaded.request_pause(false, loaded.snapshot().tick).unwrap();
    while projectile(&world, origin.projectile).ended.is_none() {
        step(&mut world, 1);
        step(&mut loaded, 1);
        same(&world, &loaded);
    }
    loaded = cold(&loaded);
    assert_eq!(loaded.practice_reception_snapshot().unwrap().quiet_ticks, 0);
    step(&mut loaded, 179);
    assert!(!loaded.practice_reception_snapshot().unwrap().ready);
    loaded = cold(&loaded);
    step(&mut loaded, 1);
    assert!(loaded.practice_reception_snapshot().unwrap().ready);
}

#[test]
fn all_three_consumed_rounds_stay_absent_after_cold_dock_and_relaunch() {
    let mut world = practice(2621);
    stop(&mut world);
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let mut items = [0; 3];
    for (i, item) in items.iter_mut().enumerate() {
        let origin = fire(&mut world, target);
        *item = origin.item;
        world = cold(&world);
        finish(&mut world, origin.projectile);
        world = cold(&world);
        assert_eq!(
            world
                .missile_snapshot()
                .unwrap()
                .rounds
                .iter()
                .flatten()
                .count(),
            2 - i
        );
    }
    assert_eq!(items, [2631, 2630, 2629]);
    dock(&mut world);
    world = cold(&world);
    launch(&mut world);
    world = cold(&world);
    assert_eq!(world.missile_snapshot().unwrap().rounds, [None; 3]);
    assert_eq!(
        world.request_missile_arm(true, world.snapshot().tick),
        Err(MissileError::NoAmmunition)
    );
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 1);
}

#[test]
fn native_station_missile_receipt_and_once_only_legal_survive_cold_continue() {
    let mut world = practice(2631);
    stop(&mut world);
    fly_axis(&mut world, 0, 3000.0, 4.0);
    fly_axis(&mut world, 2, -3000.0, 4.0);
    fly_axis(&mut world, 0, 120.0, 1.0);
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[2];
    aim_at(&mut world, target.pose.position);
    let origin = fire(&mut world, target.id);
    step(&mut world, 10);
    let mut loaded = cold(&world);
    while projectile(&world, origin.projectile).ended.is_none() {
        step(&mut world, 1);
        step(&mut loaded, 1);
        same(&world, &loaded);
    }
    assert_eq!(world.legal_value(), Some(64));
    corrupted(&mut loaded, |w| {
        w.missiles.impacts[0].as_mut().unwrap().point.x += 1.0
    });
    corrupted(&mut loaded, |w| {
        w.missiles.impacts[0].as_mut().unwrap().incident += 1
    });
    corrupted(&mut loaded, |w| {
        w.practice.as_mut().unwrap().ordinary_hit = None
    });
    let receipt = world
        .combat_snapshot()
        .unwrap()
        .last_ordinary_impact
        .unwrap();
    assert_eq!(receipt.weapon, crate::OrdinaryWeapon::StandardMissile);
    assert_eq!(receipt.item, origin.item);
    assert_eq!(receipt.emission, origin.emission);
    loaded = cold(&loaded);
    together(&mut world, &mut loaded, 180);
    assert_eq!(loaded.legal_value(), Some(64));
    assert_eq!(
        loaded.combat_snapshot().unwrap().last_ordinary_impact,
        Some(receipt)
    );
    assert_eq!(
        loaded.practice_snapshot().unwrap().status,
        PracticeStatus::Failed
    );
}

#[test]
fn actual_beam_interception_and_target_loss_are_durable_without_refund() {
    let (mut world, origin) = active(2641);
    assert!(matches!(
        shoot(&mut world).contact,
        PulseContact::Projectile { .. }
    ));
    corrupted(&mut world, |w| w.missiles.beams[0] = None);
    corrupted(&mut world, |w| {
        w.missiles.beams[0].as_mut().unwrap().emission += 1
    });
    corrupted(&mut world, |w| {
        w.missiles.beams[0].as_mut().unwrap().shot.end.x += 1.0
    });
    let loaded = cold(&world);
    assert_eq!(
        projectile(&loaded, origin.projectile).ended,
        Some(MissileEnd::Intercepted)
    );
    let mut lost = practice(2651);
    stop(&mut lost);
    fly_axis(&mut lost, 0, 3000.0, 4.0);
    let target = lost.practice_snapshot().unwrap().beacons.unwrap()[1];
    aim_at(&mut lost, Vec3::new(70.0, 0.0, 136.32));
    let origin = fire(&mut lost, target.id);
    step(&mut lost, 10);
    shoot(&mut lost);
    step(&mut lost, 1);
    assert_eq!(
        projectile(&lost, origin.projectile).ended,
        Some(MissileEnd::TargetLost)
    );
    let mut loaded = cold(&lost);
    step(&mut loaded, 60);
    assert_eq!(
        loaded
            .missile_snapshot()
            .unwrap()
            .rounds
            .iter()
            .flatten()
            .count(),
        2
    );
    assert_eq!(
        loaded.practice_snapshot().unwrap().status,
        PracticeStatus::Failed
    );
}

fn refused_unchanged(world: &mut World, bytes: &[u8]) {
    let before = world.save_bytes().unwrap();
    assert!(World::from_save_bytes(bytes).is_err());
    assert!(world.restore_bytes(bytes).is_err());
    assert_eq!(world.save_bytes().unwrap(), before); // host can keep publishing the prior valid bytes
}
fn corrupted(world: &mut World, mutate: impl FnOnce(&mut World)) {
    let mut candidate = cold(world);
    mutate(&mut candidate);
    let bytes = candidate.unchecked_test_save().unwrap();
    refused_unchanged(world, &bytes);
}
#[test]
fn contradictory_custody_origin_target_motion_and_pending_payloads_are_atomic_refusals() {
    let (mut world, origin) = active(2661);
    step(&mut world, 3);
    corrupted(&mut world, |w| w.missiles.projectiles[0] = None);
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().origin.item += 1
    });
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0]
            .as_mut()
            .unwrap()
            .origin
            .projectile
            .ordinal = 1
    });
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0]
            .as_mut()
            .unwrap()
            .origin
            .owner
            .owner += 1
    });
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0]
            .as_mut()
            .unwrap()
            .origin
            .owner
            .visit += 1
    });
    let foreign = practice(9991).practice_snapshot().unwrap().beacons.unwrap()[1].id;
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().target = foreign.into()
    });
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().position.x = f64::NAN
    });
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0]
            .as_mut()
            .unwrap()
            .propulsion_travel += 1.0
    });
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().age_ticks += 1
    });
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0]
            .as_mut()
            .unwrap()
            .origin
            .orientation
            .x
            .x += 0.1
    });
    corrupted(&mut world, |w| {
        w.missiles.motions[0].as_mut().unwrap().position.x += 1.0
    });
    corrupted(&mut world, |w| w.missiles.cooldown = 0);
    corrupted(&mut world, |w| {
        w.practice.as_mut().unwrap().pending_missiles = 0
    });
    step(&mut world, 57);
    let target = origin_target(&world, origin);
    fire(&mut world, target);
    corrupted(&mut world, |w| {
        w.missiles.projectiles[1].as_mut().unwrap().origin.emission =
            w.missiles.projectiles[0].unwrap().origin.emission
    });
    corrupted(&mut world, |w| {
        w.missiles.projectiles[1].as_mut().unwrap().origin.item =
            w.missiles.projectiles[0].unwrap().origin.item
    });
}
fn origin_target(world: &World, origin: MissileOrigin) -> PracticeBeaconId {
    projectile(world, origin.projectile)
        .target
        .practice()
        .unwrap()
}

#[test]
fn first_missile_receipt_and_collision_point_cannot_be_rewritten_with_a_fresh_checksum() {
    let (mut world, origin) = active(2671);
    finish(&mut world, origin.projectile);
    let live = world.save_bytes().unwrap();
    let mut origin_bytes = Writer::new();
    origin.encode(&mut origin_bytes);
    let start = live
        .windows(origin_bytes.bytes.len())
        .rposition(|s| s == origin_bytes.bytes)
        .unwrap();
    let mut bad = live.clone();
    bad[start] ^= 1;
    let end = bad.len() - 8;
    let digest = checksum(&bad[..end]);
    bad[end..].copy_from_slice(&digest.to_le_bytes());
    refused_unchanged(&mut world, &bad);
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().contact = Some(MissileContact::Station)
    });
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().position.z += 1.0
    });
    corrupted(&mut world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().last_tick += 1
    });
}

#[test]
fn actual_range_expiry_and_hyperspace_cull_keep_endpoints_and_cadence() {
    let mut world = practice(2681);
    stop(&mut world);
    fly_axis(&mut world, 0, 7000.0, 4.0);
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1];
    aim_at(&mut world, target.pose.position);
    let origin = fire(&mut world, target.id);
    finish(&mut world, origin.projectile);
    assert_eq!(
        projectile(&world, origin.projectile).ended,
        Some(MissileEnd::RangeExpired)
    );
    let _loaded = cold(&world);
    let mut world = practice(2691);
    stop(&mut world);
    world
        .request_jump(crate::navigation::SystemId::RENAS, world.snapshot().tick)
        .unwrap();
    for _ in 1..crate::navigation::BASIC_SPOOL_TICKS {
        step(&mut world, 1);
    }
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let origin = fire(&mut world, target);
    step(&mut world, 1);
    assert_eq!(
        projectile(&world, origin.projectile).ended,
        Some(MissileEnd::Culled)
    );
    assert_eq!(world.missile_snapshot().unwrap().cooldown_ticks, 59);
    let mut loaded = cold(&world);
    together(&mut world, &mut loaded, 60);
    assert_eq!(loaded.missile_snapshot().unwrap().cooldown_ticks, 0);
}

#[test]
fn current_reader_keeps_owned_stock_and_refuses_older_payloads() {
    let untouched = World::new_commander(2701).unwrap();
    for bytes in [
        crate::flight::persistence::v4_test_fixture(&untouched).unwrap(),
        crate::flight::persistence::v5_test_fixture(&untouched).unwrap(),
        crate::flight::persistence::v6_test_fixture(&untouched).unwrap(),
        crate::flight::persistence::v7_test_fixture(&untouched).unwrap(),
    ] {
        assert_eq!(
            World::from_save_bytes(&bytes).unwrap_err(),
            crate::save::SaveError::UnsupportedVersion
        );
    }
    {
        let loaded = World::from_save_bytes(&untouched.save_bytes().unwrap()).unwrap();
        same(&untouched, &loaded);
        assert_eq!(
            loaded
                .missile_snapshot()
                .unwrap()
                .rounds
                .iter()
                .flatten()
                .count(),
            3
        );
        let upgraded = cold(&loaded);
        same(&loaded, &upgraded);
    }
    let (mut world, _) = active(2711);
    assert_eq!(
        crate::flight::persistence::v7_test_fixture(&world),
        Err(crate::save::SaveError::UnsupportedMissiles)
    );
    let mut fake = world.save_bytes().unwrap();
    fake[8..10].copy_from_slice(&7_u16.to_le_bytes());
    let end = fake.len() - 8;
    let digest = checksum(&fake[..end]);
    fake[end..].copy_from_slice(&digest.to_le_bytes());
    refused_unchanged(&mut world, &fake);
}

#[test]
fn same_tick_coordinated_launch_pose_clock_and_behind_target_are_atomic_refusals() {
    let (mut world, origin) = active(2721);
    let before = world.snapshot();
    corrupted(&mut world, |w| {
        let p = w.missiles.projectiles[0].as_mut().unwrap();
        p.origin.launch_position.x += 1.0;
        p.origin.muzzle.x += 1.0;
        p.position.x += 1.0;
    });
    corrupted(&mut world, |w| {
        let p = w.missiles.projectiles[0].as_mut().unwrap();
        let direction = add(p.origin.orientation.z, scale(p.origin.orientation.x, 0.05));
        p.origin.orientation = pose(p.origin.launch_position, direction)
            .unwrap()
            .orientation;
        p.origin.muzzle = add(
            p.origin.launch_position,
            scale(p.origin.orientation.z, MISSILE_MUZZLE_DISTANCE),
        );
        p.position = p.origin.muzzle;
        p.direction = p.origin.orientation.z;
    });
    corrupted(&mut world, |w| {
        let p = w.missiles.projectiles[0].as_mut().unwrap();
        p.origin.physical_tick = p.origin.physical_tick.checked_sub(1).unwrap();
        w.missiles.cooldown = MISSILE_COOLDOWN_TICKS - 1;
    });
    assert_eq!(world.snapshot(), before);
    assert_eq!(projectile(&world, origin.projectile).origin, origin);
    let _valid = cold(&world);

    // Both IDs belong to the live range, but only the forward one could be launched at.
    let mut front = practice(2731);
    let beacons = front.practice_snapshot().unwrap().beacons.unwrap();
    fly_axis(
        &mut front,
        2,
        (beacons[0].pose.position.z + beacons[2].pose.position.z) / 2.0,
        1.0,
    );
    aim_at(&mut front, beacons[2].pose.position);
    let origin = fire(&mut front, beacons[2].id);
    assert!(sub(beacons[0].pose.position, origin.launch_position).dot(origin.orientation.z) < 0.0);
    corrupted(&mut front, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().target = beacons[0].id.into()
    });
    let _valid = cold(&front);
}

#[test]
fn current_launch_binding_preserves_later_same_tick_pulse_and_historical_origins() {
    let (mut world, origin) = active(2741);
    let ship = crate::flight::flight_pose(world.snapshot().position, world.snapshot().orientation)
        .unwrap();
    assert_eq!(origin.launch_position, ship.position);
    assert_eq!(origin.orientation, ship.orientation);
    assert_eq!(origin.physical_tick, world.combat.physical_tick());
    let _launched = cold(&world);
    assert!(matches!(
        shoot(&mut world).contact,
        PulseContact::Projectile { .. }
    ));
    let intercepted = projectile(&world, origin.projectile);
    assert_eq!(world.snapshot().tick, origin.launched_tick);
    assert_eq!(world.combat.physical_tick(), origin.physical_tick);
    assert!(intercepted.intercepting_emission.unwrap() > origin.emission);
    let mut loaded = cold(&world);
    together(&mut world, &mut loaded, 20);
    assert_ne!(
        crate::flight::flight_pose(world.snapshot().position, world.snapshot().orientation)
            .unwrap()
            .position,
        origin.launch_position
    );
    assert_eq!(projectile(&loaded, origin.projectile).origin, origin);
    assert_eq!(
        projectile(&loaded, origin.projectile).ended,
        Some(MissileEnd::Intercepted)
    );
    let _historical_ended = cold(&loaded);

    let (mut live, live_origin) = active(2751);
    let mut continued = cold(&live);
    together(&mut live, &mut continued, 20);
    assert!(projectile(&live, live_origin.projectile).ended.is_none());
    assert_ne!(
        crate::flight::flight_pose(live.snapshot().position, live.snapshot().orientation)
            .unwrap()
            .position,
        live_origin.launch_position
    );
    assert_eq!(
        projectile(&continued, live_origin.projectile).origin,
        live_origin
    );
    let _historical_live = cold(&continued);
}

#[test]
fn actual_two_owned_missiles_require_every_live_peer_in_the_intercepting_beam_scene() {
    let (mut world, first) = active(2761);
    step(&mut world, 60);
    assert_eq!(projectile(&world, first.projectile).ended, None);
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let second = fire(&mut world, target);
    assert!(
        matches!(shoot(&mut world).contact, PulseContact::Projectile { id, .. } if id == second.projectile)
    );
    let slot = usize::from(second.projectile.ordinal());
    assert!(world.missiles.beams[slot].as_ref().unwrap().bodies[0].is_some());
    let _valid = cold(&world);
    // An omitted tail must not remove an earlier still-live physical projectile.
    corrupted(&mut world, |bad| {
        bad.missiles.beams[slot].as_mut().unwrap().bodies.clear();
    });
    // Keeping the vector length while removing the peer has the same refusal requirement.
    corrupted(&mut world, |bad| {
        bad.missiles.beams[slot].as_mut().unwrap().bodies[0] = None;
    });
    let _still_valid = cold(&world);
}

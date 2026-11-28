#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::practice::tests::{aim_at, consent, launch, shoot, step};
use crate::{FlightInput, PracticeStatus};

fn stop(world: &mut World) {
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
    panic!("public braking never stopped the actual craft");
}
fn aim_station(world: &mut World) {
    stop(world);
    aim_at(world, Vec3::new(70.0, 0.0, 136.32));
}
fn station_shot(world: &World) -> PulseShot {
    let pose = flight_pose(world.state.position, world.state.orientation).unwrap();
    let distance = world
        .station_data
        .as_ref()
        .unwrap()
        .beam_block_distance(
            world.state.station.unwrap(),
            pose.position,
            pose.orientation.z,
            super::super::PULSE_RANGE,
        )
        .unwrap()
        .unwrap();
    PulseShot {
        tick: world.state.tick,
        fitted_item_id: world.fitted_front_pulse().unwrap(),
        origin: pose.position,
        direction: pose.orientation.z,
        end: add_scaled(pose.position, pose.orientation.z, distance),
        contact: PulseContact::StationBlocked { distance },
    }
}
fn new_flight(seed: u64) -> World {
    let mut world = World::new_commander(seed).unwrap();
    launch(&mut world);
    aim_station(&mut world);
    world
}
fn three_hits(seed: u64) -> World {
    let mut world = World::new_commander(seed).unwrap();
    consent(&mut world);
    launch(&mut world);
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
        assert!(matches!(
            shoot(&mut world).contact,
            PulseContact::PracticeBeacon {
                first_hit: true,
                ..
            }
        ));
        step(&mut world, 18);
    }
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 3);
    world
}

#[test]
fn actual_station_hit_permanently_fails_three_hit_practice_and_survives_continue() {
    let mut world = three_hits(1401);
    step(&mut world, 180);
    assert!(world.practice_reception_snapshot().unwrap().ready);
    let equipment = world.equipment_snapshot();
    let credits = world.credits_tenths();
    aim_station(&mut world);
    let shot = shoot(&mut world);
    assert!(matches!(shot.contact, PulseContact::StationBlocked { .. }));
    let hit = world
        .combat_snapshot()
        .unwrap()
        .last_ordinary_impact
        .unwrap();
    assert_eq!(hit.point, shot.end);
    assert_eq!(hit.item, 1408);
    assert_eq!(hit.victim_entity, 1);
    assert_eq!(hit.attacker.owner, 1401);
    assert_eq!(hit.attacker.system, SystemId::AVENOR);
    assert_eq!(world.legal_value(), Some(64));
    assert_eq!(
        world.practice_snapshot().unwrap().status,
        PracticeStatus::Failed
    );
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 3);
    assert!(world.practice_snapshot().unwrap().beacons.is_none());
    let mut continued = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    for _ in 0..20 {
        step(&mut continued, 180);
    }
    let reception = continued.practice_reception_snapshot().unwrap();
    assert_eq!(reception.ordinary_hit, Some(hit));
    assert_eq!(reception.quiet_ticks, 0);
    assert!(!reception.ready);
    assert!(reception.trace.is_none());
    assert_eq!(continued.equipment_snapshot(), equipment);
    assert_eq!(continued.credits_tenths(), credits);
}

#[test]
fn station_attack_window_is_inclusive_at_ten_seconds_and_pause_earns_no_time() {
    let mut world = new_flight(1411);
    shoot(&mut world);
    assert_eq!(world.legal_value(), Some(64));
    let now = world.snapshot().tick;
    world.request_pause(true, now).unwrap();
    let before = world.save_bytes().unwrap();
    assert!(world.step(FlightInput::default()).is_err());
    assert_eq!(world.request_fire_front_pulse(now), Err(FireError::Paused));
    assert_eq!(world.save_bytes().unwrap(), before);
    world.request_pause(false, now).unwrap();
    for _ in 0..3 {
        step(&mut world, 200);
    }
    shoot(&mut world);
    assert_eq!(world.legal_value(), Some(64));
    for _ in 0..3 {
        step(&mut world, 200);
    }
    step(&mut world, 1);
    shoot(&mut world);
    assert_eq!(world.legal_value(), Some(128));
    let restored = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(restored.legal_value(), Some(128));
    assert_eq!(restored.combat_snapshot(), world.combat_snapshot());
}

#[test]
fn private_prepared_contact_rejects_duplicate_foreign_stale_and_restored_worlds_atomically() {
    let mut world = new_flight(1421);
    let shot = station_shot(&world);
    let mut forged = shot;
    forged.contact = PulseContact::Clear;
    let before_forgery = world.save_bytes().unwrap();
    assert!(matches!(
        world.prepare_pulse_impact(forged),
        Err(FireError::InvalidGeometry)
    ));
    assert_eq!(world.save_bytes().unwrap(), before_forgery);
    let duplicate = world.prepare_pulse_impact(shot).unwrap();
    let first = world.prepare_pulse_impact(shot).unwrap();
    first.commit(&mut world).unwrap();
    let committed = world.save_bytes().unwrap();
    assert_eq!(duplicate.commit(&mut world), Err(FireError::StaleCommand));
    assert_eq!(world.save_bytes().unwrap(), committed);
    assert!(matches!(
        world.prepare_pulse_impact(shot),
        Err(FireError::CoolingDown)
    ));

    let source = new_flight(1422);
    let mut foreign = new_flight(1423);
    let plan = source.prepare_pulse_impact(station_shot(&source)).unwrap();
    let before = foreign.save_bytes().unwrap();
    assert_eq!(plan.commit(&mut foreign), Err(FireError::StaleCommand));
    assert_eq!(foreign.save_bytes().unwrap(), before);

    let mut source = source;
    let plan = source.prepare_pulse_impact(station_shot(&source)).unwrap();
    step(&mut source, 1);
    let before = source.save_bytes().unwrap();
    assert_eq!(plan.commit(&mut source), Err(FireError::StaleCommand));
    assert_eq!(source.save_bytes().unwrap(), before);
    let plan = source.prepare_pulse_impact(station_shot(&source)).unwrap();
    source.restore_bytes(&before).unwrap();
    let restored = source.save_bytes().unwrap();
    assert_eq!(plan.commit(&mut source), Err(FireError::StaleCommand));
    assert_eq!(source.save_bytes().unwrap(), restored);
}

#[test]
fn actual_reserved_beacon_hit_has_no_ordinary_damage_law_or_failure() {
    let mut world = World::new_commander(1431).unwrap();
    consent(&mut world);
    launch(&mut world);
    assert!(matches!(
        shoot(&mut world).contact,
        PulseContact::PracticeBeacon {
            first_hit: true,
            ..
        }
    ));
    let snapshot = world.combat_snapshot().unwrap();
    assert!(snapshot.last_ordinary_impact.is_none());
    assert_eq!(snapshot.player.front_shield_hundredths, 30_000);
    assert_eq!(snapshot.player.rear_shield_hundredths, 30_000);
    assert_eq!(snapshot.player.energy_hundredths, 37_900);
    assert_eq!(world.legal_value(), Some(0));
    assert!(
        world
            .practice_reception_snapshot()
            .unwrap()
            .ordinary_hit
            .is_none()
    );
    step(&mut world, 18);
    assert!(matches!(
        shoot(&mut world).contact,
        PulseContact::PracticeBeacon {
            first_hit: false,
            ..
        }
    ));
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 1);
    assert!(
        world
            .combat_snapshot()
            .unwrap()
            .last_ordinary_impact
            .is_none()
    );
}

#[test]
fn player_damage_reducer_uses_shield_side_shared_energy_recharge_and_fatal_refusal() {
    // Internal physical-contact reducer qualification. No missile launch or
    // ammunition consumption is asserted: that producer is the next slice.
    let mut world = new_flight(1441);
    world.combat.resolve_impact(
        &mut world.pulse,
        OrdinaryTarget::Player { front: false },
        OrdinaryWeapon::FrontPulseLaser,
    );
    assert_eq!(
        world
            .combat_snapshot()
            .unwrap()
            .player
            .rear_shield_hundredths,
        28_500
    );
    step(&mut world, 250);
    step(&mut world, 250);
    assert_eq!(
        world
            .combat_snapshot()
            .unwrap()
            .player
            .rear_shield_hundredths,
        30_000
    );
    assert!(matches!(
        world.save_bytes(),
        Err(SaveError::InvalidState(
            "unsupported ordnance damage boundary"
        ))
    ));
    // Reset through the exact stock birth rather than introducing another pool factory.
    let mut world = new_flight(1441);
    world.combat.resolve_impact(
        &mut world.pulse,
        OrdinaryTarget::Player { front: true },
        OrdinaryWeapon::StandardMissile,
    );
    let condition = world.combat_snapshot().unwrap().player;
    assert_eq!(condition.front_shield_hundredths, 14_000);
    assert_eq!(condition.rear_shield_hundredths, 30_000);
    assert_eq!(condition.energy_hundredths, 38_000);
    step(&mut world, 1);
    assert_eq!(
        world
            .combat_snapshot()
            .unwrap()
            .player
            .front_shield_hundredths,
        14_003
    );
    world.combat.resolve_impact(
        &mut world.pulse,
        OrdinaryTarget::Player { front: true },
        OrdinaryWeapon::StandardMissile,
    );
    assert_eq!(
        world.combat_snapshot().unwrap().player.energy_hundredths,
        36_003
    );
    assert!(matches!(
        world.save_bytes(),
        Err(SaveError::InvalidState(
            "unsupported ordnance damage boundary"
        ))
    ));
    world.combat.resolve_impact(
        &mut world.pulse,
        OrdinaryTarget::MainStation,
        OrdinaryWeapon::StandardMissile,
    );
    assert_eq!(
        world.combat_snapshot().unwrap().player.energy_hundredths,
        36_003
    );
    for _ in 0..3 {
        world.combat.resolve_impact(
            &mut world.pulse,
            OrdinaryTarget::Player { front: true },
            OrdinaryWeapon::StandardMissile,
        );
    }
    assert!(world.combat_snapshot().unwrap().player.destroyed);
    let before = world.combat_snapshot();
    assert_eq!(
        world.step(FlightInput::default()),
        Err(crate::StepError::InvalidMode)
    );
    assert_eq!(world.combat_snapshot(), before);
    assert_eq!(
        world
            .combat_snapshot()
            .unwrap()
            .player
            .rear_shield_hundredths,
        30_000
    );
}

#[test]
fn save_validation_refuses_removed_failure_forged_contact_and_unearned_legal_bits() {
    let mut world = three_hits(1451);
    aim_station(&mut world);
    shoot(&mut world);
    let original = world.practice.unwrap().ordinary_hit.unwrap();
    world.practice.as_mut().unwrap().ordinary_hit = None;
    assert!(world.save_bytes().is_err());
    world.practice.as_mut().unwrap().ordinary_hit = Some(original);
    world
        .practice
        .as_mut()
        .unwrap()
        .ordinary_hit
        .as_mut()
        .unwrap()
        .attacker
        .owner += 1;
    assert!(world.save_bytes().is_err());
    world.practice.as_mut().unwrap().ordinary_hit = Some(original);
    world.combat.last.as_mut().unwrap().point.x += 1.0;
    assert!(world.save_bytes().is_err());
    world.combat.last = Some(original);
    assert!(world.save_bytes().is_ok());
    // Exercise the loader with a newly checksummed omission, preserving the
    // actual station incident, original range hits and weapon bytes.
    let live = world.save_bytes().unwrap();
    let mut state = Writer::new();
    world.combat.encode(&mut state);
    let mut failure = Writer::new();
    failure.option(world.practice.unwrap().ordinary_hit.as_ref(), |w, hit| {
        hit.encode(w)
    });
    let failure_start =
        crate::flight::persistence::pulse_save_start(&world, &live) - failure.bytes.len();
    let mut removed = live[..live.len() - 8].to_vec();
    removed.splice(failure_start..failure_start + failure.bytes.len(), [0]);
    let digest = crate::save::checksum(&removed);
    removed.extend(digest.to_le_bytes());
    assert!(world.restore_bytes(&removed).is_err());
    assert_eq!(world.save_bytes().unwrap(), live);
    let combat_start = failure_start - state.bytes.len();
    let mut unearned = live.clone();
    unearned[combat_start + 28] = 0; // No first-window penalty despite an actual incident.
    let end = unearned.len() - 8;
    let digest = crate::save::checksum(&unearned[..end]);
    unearned[end..].copy_from_slice(&digest.to_le_bytes());
    assert!(world.restore_bytes(&unearned).is_err());
    assert_eq!(world.save_bytes().unwrap(), live);
    assert!(legal_reachable(64, 0, 1));
    assert!(legal_reachable(128, 0, 2));
    assert!(!legal_reachable(65, 0, 1));
    assert!(legal_reachable(65, 1, 1));
    assert!(legal_reachable(255, 0, 4));
}

#[test]
fn current_hot_practice_keeps_hits_and_refuses_v6_and_v7() {
    let world = three_hits(1461);
    let legacy = crate::flight::persistence::unsupported_version_test_input(&world, 6).unwrap();
    assert_eq!(u16::from_le_bytes([legacy[8], legacy[9]]), 6);
    assert_eq!(
        World::from_save_bytes(&legacy).unwrap_err(),
        crate::save::SaveError::UnsupportedVersion
    );
    let mut continued = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(continued.practice_snapshot(), world.practice_snapshot());
    assert_eq!(
        continued.practice_reception_snapshot(),
        world.practice_reception_snapshot()
    );
    assert_eq!(
        continued.pulse_laser_snapshot(),
        world.pulse_laser_snapshot()
    );
    assert_eq!(continued.equipment_snapshot(), world.equipment_snapshot());
    assert!(
        continued
            .combat_snapshot()
            .unwrap()
            .last_ordinary_impact
            .is_none()
    );
    aim_station(&mut continued);
    shoot(&mut continued);
    assert_eq!(continued.legal_value(), Some(64));
    assert_eq!(
        continued.practice_snapshot().unwrap().status,
        PracticeStatus::Failed
    );
    let v7 = crate::flight::persistence::unsupported_version_test_input(&continued, 7).unwrap();
    assert_eq!(u16::from_le_bytes([v7[8], v7[9]]), 7);
    assert_eq!(
        World::from_save_bytes(&v7).unwrap_err(),
        crate::save::SaveError::UnsupportedVersion
    );
    assert_eq!(
        World::from_save_bytes(&continued.save_bytes().unwrap())
            .unwrap()
            .practice_reception_snapshot(),
        continued.practice_reception_snapshot()
    );
}

fn reseal(bytes: &mut [u8]) {
    let end = bytes.len() - 8;
    let digest = crate::save::checksum(&bytes[..end]);
    bytes[end..].copy_from_slice(&digest.to_le_bytes());
}

fn assert_atomic_saved_refusal(world: &mut World, live: &[u8], mut changed: Vec<u8>) {
    reseal(&mut changed);
    assert!(world.restore_bytes(&changed).is_err());
    assert_eq!(world.save_bytes().unwrap().as_slice(), live);
}

#[test]
fn resealed_station_receipt_requires_owned_pulse_resources_without_practice_anchor() {
    let mut world = World::new_commander(1471).unwrap();
    // Genuine docked ticks leave room for a one-field physical-clock forgery
    // that still passes the old physical_tick <= global_tick bound.
    step(&mut world, 10);
    launch(&mut world);
    aim_station(&mut world);
    shoot(&mut world);
    assert!(world.practice.is_none());
    let hit = world.combat.last.unwrap();
    assert_eq!(hit.tick, world.state.tick);
    assert_eq!(hit.physical_tick, world.combat.physical_tick);
    let laser = world.pulse_laser_snapshot().unwrap();
    assert_eq!(laser.cooldown_ticks, 18);
    let live = world.save_bytes().unwrap();
    let pulse_start = crate::flight::persistence::pulse_save_start(&world, &live);
    let mut reset = live.clone();
    reset[pulse_start..pulse_start + 2].copy_from_slice(&38_000_u16.to_le_bytes());
    reset[pulse_start + 2..pulse_start + 5].fill(0);
    assert_atomic_saved_refusal(&mut world, &live, reset);

    let mut heat_only = live.clone();
    heat_only[pulse_start + 2..pulse_start + 4].copy_from_slice(&0_u16.to_le_bytes());
    assert_atomic_saved_refusal(&mut world, &live, heat_only);
    let mut energy_only = live.clone();
    energy_only[pulse_start..pulse_start + 2].copy_from_slice(&38_000_u16.to_le_bytes());
    assert_atomic_saved_refusal(&mut world, &live, energy_only);

    let combat_start = pulse_start - crate::flight::persistence::combat_save_len(&world);
    let mut clock_only = live.clone();
    let later = world.combat.physical_tick + 1;
    assert!(later <= world.state.tick);
    clock_only[combat_start + 16..combat_start + 24].copy_from_slice(&later.to_le_bytes());
    assert_atomic_saved_refusal(&mut world, &live, clock_only);

    let mut continued = World::from_save_bytes(&live).unwrap();
    assert_eq!(continued.pulse_laser_snapshot(), Some(laser));
    assert_eq!(
        continued.request_fire_front_pulse(hit.tick),
        Err(FireError::CoolingDown)
    );
    // A later clear shot can legitimately replace laser resource evidence
    // without refreshing the last ordinary incident or its station window.
    stop(&mut continued);
    let z = f64::from(continued.snapshot().position[2]);
    aim_at(&mut continued, Vec3::new(0.0, 0.0, z + 6000.0));
    assert_eq!(shoot(&mut continued).contact, PulseContact::Clear);
    assert_eq!(continued.combat.last, Some(hit));
    assert!(World::from_save_bytes(&continued.save_bytes().unwrap()).is_ok());
    aim_at(&mut continued, Vec3::new(0.0, 0.0, 0.0));
    for _ in 0..9000 {
        let snapshot = continued.snapshot();
        if snapshot.mode == FlightMode::Docked {
            break;
        }
        let target_speed = if snapshot.position[2] > 2500.0 {
            34.3
        } else {
            4.0
        };
        continued
            .step(crate::FlightInput {
                throttle: ((target_speed - snapshot.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..crate::FlightInput::default()
            })
            .unwrap();
        assert_eq!(continued.snapshot().motion_refusal, None);
    }
    assert_eq!(continued.snapshot().mode, FlightMode::Docked);
    assert_eq!(
        continued.pulse_laser_snapshot().unwrap().energy_hundredths,
        38_000
    );
    assert_eq!(continued.pulse_laser_snapshot().unwrap().heat_fifths, 0);
    assert_eq!(continued.combat.last, Some(hit));
    assert!(World::from_save_bytes(&continued.save_bytes().unwrap()).is_ok());
}

#[test]
fn resealed_last_station_window_cannot_be_refreshed_without_a_new_incident() {
    let mut world = new_flight(1481);
    shoot(&mut world);
    let hit = world.combat.last.unwrap();
    for _ in 0..3 {
        step(&mut world, 200);
    }
    step(&mut world, 1);
    assert_eq!(world.combat.physical_tick - hit.physical_tick, 601);
    let live = world.save_bytes().unwrap();
    let combat_start = crate::flight::persistence::pulse_save_start(&world, &live)
        - crate::flight::persistence::combat_save_len(&world);
    let window = world.combat.windows[0].unwrap();
    assert_eq!(window.physical_tick, hit.physical_tick);
    assert_eq!(window.visit, hit.attacker.visit);
    // Derive the one-field window location from the original encoded window
    // prefix, not a replacement state or a recomputed whole combat block.
    let mut prefix = Writer::new();
    prefix.u64(world.combat.emissions);
    prefix.u64(world.combat.incidents);
    prefix.u64(world.combat.physical_tick);
    world.combat.shields.encode(&mut prefix);
    prefix.u8(world.combat.penalties);
    prefix.bool(world.combat.unbound_player_impact);
    prefix.bool(true);
    prefix.u64(window.visit);
    let window_time = combat_start + prefix.bytes.len();
    assert_eq!(
        &live[window_time..window_time + 8],
        &hit.physical_tick.to_le_bytes()
    );
    let mut refreshed = live.clone();
    refreshed[window_time..window_time + 8]
        .copy_from_slice(&world.combat.physical_tick.to_le_bytes());
    assert_atomic_saved_refusal(&mut world, &live, refreshed);
    assert_eq!(world.legal_value(), Some(64));
    assert_eq!(world.combat.last, Some(hit));
    let mut continued = World::from_save_bytes(&live).unwrap();
    shoot(&mut continued);
    assert_eq!(continued.legal_value(), Some(128));
    assert!(continued.combat.last.unwrap().incident > hit.incident);
    assert!(World::from_save_bytes(&continued.save_bytes().unwrap()).is_ok());

    // A different real visit may retain an earlier station's exact window.
    // Equality binds to that incident's visit, not the current navigation visit.
    continued
        .request_jump(SystemId::RENAS, continued.snapshot().tick)
        .unwrap();
    for _ in 0..crate::navigation::BASIC_SPOOL_TICKS {
        step(&mut continued, 1);
    }
    let previous = continued.combat.last.unwrap();
    assert!(previous.attacker.visit < continued.snapshot().navigation.unwrap().visit);
    assert!(World::from_save_bytes(&continued.save_bytes().unwrap()).is_ok());
}

#[test]
fn unsupported_damage_boundary_precedes_missile_graph_on_save_and_atomic_restore() {
    let mut world = new_flight(2761);
    let supported = world.save_bytes().unwrap();
    // Negative fixture only: this private reducer has no retained ordnance origin.
    world.combat.resolve_impact(
        &mut world.pulse,
        OrdinaryTarget::Player { front: true },
        OrdinaryWeapon::StandardMissile,
    );
    let expected = SaveError::InvalidState("unsupported ordnance damage boundary");
    let condition = world.combat_snapshot();
    let ordnance = world.missile_snapshot();
    let flight = world.snapshot();
    assert_eq!(world.save_bytes(), Err(expected));
    assert_eq!(world.combat_snapshot(), condition);
    assert_eq!(world.missile_snapshot(), ordnance);
    assert_eq!(world.snapshot(), flight);
    let unsupported = world.unchecked_test_save().unwrap();
    assert_eq!(World::from_save_bytes(&unsupported).err(), Some(expected));
    let mut live = World::from_save_bytes(&supported).unwrap();
    let before = live.save_bytes().unwrap();
    assert_eq!(live.restore_bytes(&unsupported), Err(expected));
    assert_eq!(live.save_bytes().unwrap(), before);
}

fn funded_lareria_station_flight(seed: u64) -> World {
    use crate::flight::equipment::tests::{earned_world, journey};
    let mut world = earned_world(seed);
    world.request_refuel(world.snapshot().tick).unwrap();
    journey(&mut world, SystemId::AVENOR);
    world.request_refuel(world.snapshot().tick).unwrap();
    let tick = world.snapshot().tick;
    let review = world
        .review_module(
            crate::ModuleAction::Purchase {
                kind: crate::ModuleKind::IntermediateNavigation,
            },
            tick,
        )
        .unwrap();
    world.prepare_module(review, tick).unwrap().commit();
    journey(&mut world, SystemId::LARERIA);
    launch(&mut world);
    aim_station(&mut world);
    world
}

#[test]
fn real_lareria_station_pulse_uses_exact_pools_crime_window_and_cold_receipt() {
    let mut world = funded_lareria_station_flight(15941);
    assert_eq!(
        world.snapshot().navigation.unwrap().system,
        SystemId::LARERIA
    );
    assert_eq!(world.legal_value(), Some(0));
    assert_eq!(world.combat.windows, [None; 3]);
    let pools = world.flight_resources_snapshot().unwrap();
    assert_eq!(
        (
            pools.energy_hundredths,
            pools.heat_fifths,
            pools.cooldown_ticks
        ),
        (38_000, 0, 0)
    );
    let credits = world.credits_tenths();
    let ammo = world.ammunition_snapshot();
    let shot = world
        .request_fire_front_pulse(world.snapshot().tick)
        .unwrap();
    assert!(matches!(shot.contact, PulseContact::StationBlocked { .. }));
    let after = world.flight_resources_snapshot().unwrap();
    assert_eq!(
        (
            after.energy_hundredths,
            after.heat_fifths,
            after.cooldown_ticks
        ),
        (37_900, 50, 18)
    );
    assert_eq!(world.credits_tenths(), credits);
    assert_eq!(world.ammunition_snapshot(), ammo);
    assert_eq!(world.legal_value(), Some(64));
    let hit = world
        .combat_snapshot()
        .unwrap()
        .last_ordinary_impact
        .unwrap();
    assert_eq!(hit.attacker.system, SystemId::LARERIA);
    assert_eq!(hit.target, OrdinaryTarget::MainStation);
    assert_eq!(hit.victim_entity, 1);
    assert_eq!(hit.weapon, OrdinaryWeapon::FrontPulseLaser);
    assert_eq!(hit.item, 15948);
    assert_eq!(hit.point, shot.end);
    assert_eq!(&world.combat.windows[..2], &[None; 2]);
    assert_eq!(
        world.combat.windows[2],
        Some(AttackWindow {
            visit: hit.attacker.visit,
            physical_tick: hit.physical_tick,
        })
    );
    let bytes = world.save_bytes().unwrap();
    let mut cold = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(cold.combat, world.combat);
    assert_eq!(
        cold.flight_resources_snapshot(),
        world.flight_resources_snapshot()
    );
    assert_eq!(cold.legal_value(), Some(64));

    // Negative save fixture must not redirect Lareria's receipt to another station's window.
    let window = cold.combat.windows[2].take();
    cold.combat.windows[0] = window;
    let forged = cold.unchecked_test_save().unwrap();
    assert!(world.restore_bytes(&forged).is_err());
    assert_eq!(world.save_bytes().unwrap(), bytes);
    let mut cold = World::from_save_bytes(&bytes).unwrap();
    for _ in 0..ATTACK_WINDOW {
        cold.step(FlightInput::default()).unwrap();
    }
    let inclusive = cold.request_fire_front_pulse(cold.snapshot().tick).unwrap();
    assert!(matches!(
        inclusive.contact,
        PulseContact::StationBlocked { .. }
    ));
    assert_eq!(cold.legal_value(), Some(64));
    let mut cold = World::from_save_bytes(&cold.save_bytes().unwrap()).unwrap();
    for _ in 0..=ATTACK_WINDOW {
        cold.step(FlightInput::default()).unwrap();
    }
    let later = cold.request_fire_front_pulse(cold.snapshot().tick).unwrap();
    assert!(matches!(later.contact, PulseContact::StationBlocked { .. }));
    assert_eq!(cold.legal_value(), Some(128));
    let loaded = World::from_save_bytes(&cold.save_bytes().unwrap()).unwrap();
    assert_eq!(loaded.combat, cold.combat);
    assert_eq!(
        loaded.flight_resources_snapshot(),
        cold.flight_resources_snapshot()
    );
    assert_eq!(loaded.legal_value(), Some(128));
}

#[test]
fn missile_station_reducer_and_codec_keep_three_independent_attack_windows() {
    // Component mapping/codec proof. Successful World contact is exercised by the
    // real laser above and the existing public missile station journeys.
    let mut source = World::new_commander(15942).unwrap();
    consent(&mut source);
    launch(&mut source);
    let target = source.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let tick = source.snapshot().tick;
    source.request_missile_lock(Some(target), tick).unwrap();
    source.request_missile_arm(true, tick).unwrap();
    let origin = source.request_fire_missile(tick).unwrap();
    let mut combat = State::default();
    let mut pulse = super::super::weapons::PulseResources::default();
    for (index, system) in [SystemId::AVENOR, SystemId::RENAS, SystemId::LARERIA]
        .into_iter()
        .enumerate()
    {
        assert_eq!(station_index(system), Some(index));
        let (reserved, emission) = combat.reserve_missile_emission().unwrap();
        combat = reserved;
        let mut fixture = origin;
        fixture.owner.system = system;
        fixture.owner.visit = 2 + index as u64;
        fixture.emission = emission;
        let (impact, penalty) = combat
            .record_missile_impact(
                &mut pulse,
                fixture,
                source.snapshot(),
                origin.muzzle,
                OrdinaryTarget::MainStation,
            )
            .unwrap();
        assert_eq!(penalty, 64);
        assert_eq!(impact.attacker.system, system);
        assert_eq!(impact.weapon, OrdinaryWeapon::StandardMissile);
        assert_eq!(impact.victim_entity, 1);
        assert_eq!(
            combat.windows[index],
            Some(AttackWindow {
                visit: fixture.owner.visit,
                physical_tick: 0,
            })
        );
    }
    assert_eq!(combat.penalties, 3);
    let mut writer = crate::save::Writer::new();
    combat.encode(&mut writer);
    let mut reader = crate::save::Reader::new(&writer.bytes);
    let decoded = State::decode(&mut reader, crate::save::SAVE_VERSION).unwrap();
    assert!(reader.finished());
    assert_eq!(decoded, combat);
    assert_eq!(
        decoded.windows.map(|window| window.unwrap().visit),
        [2, 3, 4]
    );
}

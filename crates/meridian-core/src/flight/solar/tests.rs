#![allow(clippy::unwrap_used)]
use super::*;
use crate::docking::Vec3;
use crate::flight::equipment::tests::{earned_world, journey};
use crate::flight::missiles::tests::stop;
use crate::flight::practice::tests::launch;
use crate::{ModuleAction, ModuleKind, ModuleProvenance, XyzzyAction};

fn advance_policy(speed: f64, ticks: u16) -> (SolarSnapshot, u16) {
    let mut state = SolarSnapshot::default();
    let mut fuel = 0;
    for _ in 0..ticks {
        let (next, gain) = accepted_exposure(state, 2.7, 1.0, speed, true, fuel, 130);
        state = next;
        fuel += gain;
    }
    (state, fuel)
}

#[test]
fn original_corona_threshold_fraction_rate_nose_and_independent_thermal_policy() {
    let (fast, fuel) = advance_policy(8.0, 60);
    assert_eq!(fuel, 7);
    assert_eq!(fast.status, SolarScoopStatus::Collecting);
    assert!(fast.fuel_remainder.abs() < 1e-9);
    let (slow, fuel) = advance_policy(4.0, 60);
    assert_eq!(fuel, 3);
    assert!((slow.fuel_remainder - 0.5).abs() < 1e-9);
    let (next, gain) = accepted_exposure(slow, 3.0, 0.79, 8.0, true, 3, 130);
    assert_eq!(
        (next.status, next.fuel_remainder, gain),
        (SolarScoopStatus::TurnTowardSun, 0.0, 0)
    );
    let (next, gain) = accepted_exposure(slow, 3.01, 1.0, 8.0, true, 3, 130);
    assert_eq!((next.status, gain), (SolarScoopStatus::Inactive, 0));
    let (next, gain) = accepted_exposure(slow, 3.0, 1.0, 8.0, false, 3, 130);
    assert_eq!((next.status, gain), (SolarScoopStatus::NoScoops, 0));
    let (next, gain) = accepted_exposure(
        SolarSnapshot {
            fuel_remainder: 0.99,
            ..slow
        },
        2.7,
        1.0,
        8.0,
        true,
        199,
        200,
    );
    assert_eq!(
        (next.status, next.fuel_remainder, gain),
        (SolarScoopStatus::TankFull, 0.0, 1)
    );
    let (next, gain) = accepted_exposure(slow, 1.1, 1.0, 8.0, true, 3, 130);
    assert!(next.cabin_temperature > slow.cabin_temperature);
    assert_eq!(gain, 0);
    let mut temp = SolarSnapshot::default();
    for _ in 0..300 {
        temp = accepted_exposure(temp, 1.35, -1.0, 0.0, false, 130, 130).0;
    }
    assert_eq!(temp.cabin_temperature, 1.0);
    assert!(cool(0.9, 300) < 0.5);
}

fn aim_3d(world: &mut World, target: Vec3) {
    stop(world);
    let local = |world: &World| {
        let state = world.snapshot();
        let pose = crate::station::flight_pose(state.position, state.orientation).unwrap();
        pose.orientation.inverse_coordinates(Vec3::new(
            target.x - pose.position.x,
            target.y - pose.position.y,
            target.z - pose.position.z,
        ))
    };
    let initial = local(world);
    let yaw = initial.x.atan2(initial.z) as f32;
    let yaw_ticks =
        (yaw.abs() / (super::super::YAW_RATE * super::super::FIXED_DELTA_SECONDS)).ceil() as u16;
    for _ in 0..yaw_ticks {
        world
            .step(FlightInput {
                yaw: yaw
                    / (super::super::YAW_RATE * super::super::FIXED_DELTA_SECONDS)
                    / f32::from(yaw_ticks),
                ..FlightInput::default()
            })
            .unwrap();
    }
    let aimed = local(world);
    let pitch = aimed.y.atan2(aimed.x.hypot(aimed.z)) as f32;
    let pitch_ticks = (pitch.abs() / (super::super::PITCH_RATE * super::super::FIXED_DELTA_SECONDS))
        .ceil() as u16;
    for _ in 0..pitch_ticks {
        world
            .step(FlightInput {
                pitch: pitch
                    / (super::super::PITCH_RATE * super::super::FIXED_DELTA_SECONDS)
                    / f32::from(pitch_ticks),
                ..FlightInput::default()
            })
            .unwrap();
    }
}

fn ratio(world: &World) -> f64 {
    MotionFrame::from(world.snapshot())
        .ratio_and_nose(world.celestial_snapshot().unwrap().sun)
        .0
}
fn cruise(world: &mut World, speed: f32) {
    let current = world.snapshot().speed;
    world
        .step(FlightInput {
            throttle: ((speed - current)
                / (super::super::ACCELERATION * super::super::FIXED_DELTA_SECONDS))
                .clamp(-1.0, 1.0),
            ..FlightInput::default()
        })
        .unwrap();
}
fn solar_approach(world: &mut World) {
    launch(world);
    let sun = world.celestial_snapshot().unwrap().sun;
    aim_3d(world, sun.position);
    assert!(MotionFrame::from(world.snapshot()).ratio_and_nose(sun).1 > 0.999);
    for _ in 0..6000 {
        if ratio(world) <= 4.0 {
            return;
        }
        cruise(world, 34.3);
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    panic!("actual flight did not reach local solar exposure");
}
fn fuel(world: &World) -> u16 {
    world.snapshot().navigation.unwrap().fuel_units
}
fn refill_solar(world: &mut World, added: u16) {
    let opening = fuel(world);
    for _ in 0..1000 {
        cruise(world, 8.0);
        assert!(!world.combat_snapshot().unwrap().player.destroyed);
        if fuel(world) >= opening + added && world.solar_snapshot().unwrap().fuel_remainder > 0.0001
        {
            return;
        }
    }
    panic!("actual fitted scoop did not collect");
}
fn module(world: &mut World, kind: ModuleKind) -> u64 {
    let tick = world.snapshot().tick;
    let review = world
        .review_module(ModuleAction::Purchase { kind }, tick)
        .unwrap();
    world.prepare_module(review, tick).unwrap().commit()
}
fn secret(world: &mut World, action: XyzzyAction) {
    let tick = world.snapshot().tick;
    let review = world.review_xyzzy(action, tick).unwrap();
    world.prepare_xyzzy(review, tick).unwrap().commit();
}

#[test]
fn paid_solar_cabin_and_ecm_share_the_exact_combined_save() {
    let mut world = earned_world(18294);
    let tank = module(&mut world, ModuleKind::FuelStorage);
    let scoop = module(&mut world, ModuleKind::FuelScoops);
    let ecm = module(&mut world, ModuleKind::Ecm);
    let cabin = module(
        &mut world,
        ModuleKind::Cabin(crate::CabinGrade::PackedStasis),
    );
    let fit = world.modular_equipment_snapshot().unwrap();
    assert_eq!((fit.utility_mounts_used, fit.equipment_units_used), (3, 3));
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 5);
    for id in [tank, scoop, ecm, cabin] {
        assert!(
            fit.items
                .iter()
                .any(|item| item.id == id && item.slot.is_some())
        );
    }
    world.request_refuel(world.snapshot().tick).unwrap();
    journey(&mut world, SystemId::AVENOR);
    solar_approach(&mut world);
    refill_solar(&mut world, 8);
    let solar = world.solar_snapshot().unwrap();
    let fuel = fuel(&world);
    let cabins = world.passenger_cabins();
    let cash = world.credits_tenths();
    let activation = world.request_ecm(world.snapshot().tick).unwrap();
    assert_eq!(world.solar_snapshot(), Some(solar));
    let bytes = world.save_bytes().unwrap();
    assert!(bytes.windows(8).any(|tag| tag == b"MSOL0003"));
    assert!(bytes.windows(8).any(|tag| tag == b"MECM0003"));
    let restored = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(restored.solar_snapshot(), Some(solar));
    assert_eq!(restored.snapshot().navigation.unwrap().fuel_units, fuel);
    assert_eq!(restored.credits_tenths(), cash);
    assert_eq!(restored.passenger_cabins(), cabins);
    assert_eq!(restored.fitted_ecm().unwrap().id, ecm);
    assert_eq!(
        restored.ecm_snapshot().unwrap().last_activation,
        Some(activation)
    );
    assert_eq!(restored.ecm_snapshot().unwrap().active, None);
    assert_eq!(restored.ecm_snapshot().unwrap().remaining_ticks, 32);
}

#[test]
fn real_earned_paid_scoops_add_unbought_fuel_to_the_extended_tank_and_resume_mid_fraction() {
    let mut world = earned_world(18291);
    module(&mut world, ModuleKind::FuelStorage);
    let item = module(&mut world, ModuleKind::FuelScoops);
    assert!(
        world
            .fuel_scoops_snapshot()
            .unwrap()
            .owned
            .is_some_and(|owned| owned.id == item
                && matches!(
                    owned.provenance,
                    ModuleProvenance::Paid {
                        paid_tenths: 19_000,
                        ..
                    }
                ))
    );
    world.request_refuel(world.snapshot().tick).unwrap();
    journey(&mut world, SystemId::AVENOR);
    assert_eq!(fuel(&world), 166);
    let cash = world.credits_tenths();
    solar_approach(&mut world);
    refill_solar(&mut world, 8);
    assert_eq!(
        world.solar_snapshot().unwrap().status,
        SolarScoopStatus::Collecting
    );
    assert!(fuel(&world) > 166);
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(
        world.snapshot().navigation.unwrap().fuel_capacity_units,
        200
    );
    assert!(world.solar.runs.len() < 50);
    let mut encoded = Writer::new();
    world.solar.encode(&mut encoded).unwrap();
    assert_eq!(world.solar.encoded_len(), encoded.bytes.len() + 12);
    let previous = world.solar_snapshot().unwrap();
    world.request_pause(true, world.snapshot().tick).unwrap();
    assert!(world.step(FlightInput::default()).is_err());
    assert_eq!(world.solar_snapshot(), Some(previous));
    world.request_pause(false, world.snapshot().tick).unwrap();
    let bytes = world.save_bytes().unwrap();
    let mut cold = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(cold.solar_snapshot(), Some(previous));
    for _ in 0..30 {
        cruise(&mut world, 8.0);
        cruise(&mut cold, 8.0);
    }
    assert_eq!(world.solar_snapshot(), cold.solar_snapshot());
    assert_eq!(fuel(&world), fuel(&cold));
    let before = world.save_bytes().unwrap();
    let mut forged = World::from_save_bytes(&before).unwrap();
    forged.solar.receipts.get_mut(0).unwrap().added += 1;
    assert!(forged.save_bytes().is_err());
    assert!(
        world
            .restore_bytes(&forged.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(world.save_bytes().unwrap(), before);
}

#[test]
fn explicit_fitted_test_scoops_collect_without_a_purchase_or_unlimited_fuel() {
    let mut world = World::new_commander(18292).unwrap();
    secret(&mut world, XyzzyAction::Enter);
    secret(&mut world, XyzzyAction::ToggleUnlimitedFuel);
    journey(&mut world, SystemId::RENAS);
    assert_eq!(fuel(&world), 96);
    let item = world.fuel_scoops_snapshot().unwrap().owned.unwrap();
    assert!(matches!(item.provenance, ModuleProvenance::XYZzy { .. }));
    let cash = world.credits_tenths();
    solar_approach(&mut world);
    refill_solar(&mut world, 4);
    assert!(fuel(&world) > 96);
    assert_eq!(world.credits_tenths(), cash);
    let cold = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(cold.solar_snapshot(), world.solar_snapshot());
    assert_eq!(cold.fuel_scoops_snapshot().unwrap().owned, Some(item));
}

#[test]
fn dedicated_scoop_test_refits_same_item_without_refill_and_refuses_full_ordinary_mounts() {
    let mut world = World::new_commander(18294).unwrap();
    assert_eq!(
        world.review_xyzzy(XyzzyAction::FitFuelScoops, 0),
        Err(crate::XyzzyError::Inactive)
    );
    secret(&mut world, XyzzyAction::Enter);
    let item = world.fuel_scoops_snapshot().unwrap().owned.unwrap();
    let store = |world: &mut World| {
        let tick = world.snapshot().tick;
        let review = world
            .review_module(ModuleAction::Store { item: item.id }, tick)
            .unwrap();
        world.prepare_module(review, tick).unwrap().commit();
    };
    store(&mut world);
    let fuel_before = fuel(&world);
    let cash = world.credits_tenths();
    secret(&mut world, XyzzyAction::FitFuelScoops);
    assert_eq!(world.fuel_scoops_snapshot().unwrap().owned, Some(item));
    assert_eq!(fuel(&world), fuel_before);
    assert_eq!(world.credits_tenths(), cash);
    secret(&mut world, XyzzyAction::FitFuelScoops);
    store(&mut world);
    module(&mut world, ModuleKind::FuelStorage);
    let before = world.save_bytes().unwrap();
    assert_eq!(
        world.review_xyzzy(XyzzyAction::FitFuelScoops, world.snapshot().tick),
        Err(crate::XyzzyError::UtilityFitConflict)
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    let cold = World::from_save_bytes(&before).unwrap();
    assert_eq!(cold.fuel_scoops_snapshot(), world.fuel_scoops_snapshot());
}

#[test]
fn real_unfitted_approach_overheats_and_destroys_without_granting_fuel() {
    let mut world = World::new_commander(18293).unwrap();
    let before = fuel(&world);
    solar_approach(&mut world);
    world
        .request_missile_arm(true, world.snapshot().tick)
        .unwrap();
    assert!(world.missile_snapshot().unwrap().armed);
    overheat_unfitted_ship(&mut world);
    assert_eq!(fuel(&world), before);
    assert_eq!(world.solar_snapshot().unwrap().cabin_temperature, 1.0);
    assert!(world.combat_snapshot().unwrap().player.destroyed);
    assert!(!world.missile_snapshot().unwrap().armed);
    assert_eq!(
        world.solar.fatal_clock(),
        Some(world.combat.physical_tick())
    );
    assert!(world.step(FlightInput::default()).is_err());
    let cold = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(cold.solar_snapshot(), world.solar_snapshot());
    assert_eq!(cold.combat_snapshot(), world.combat_snapshot());
}

fn overheat_unfitted_ship(world: &mut World) {
    let mut seen = false;
    for _ in 0..1500 {
        if world.combat_snapshot().unwrap().player.destroyed {
            break;
        }
        if ratio(world) > 1.95 {
            cruise(world, 34.3);
        } else {
            cruise(world, 0.0);
        }
        seen |= world.solar_snapshot().unwrap().status == SolarScoopStatus::NoScoops;
    }
    assert!(seen);
}

#[test]
fn sealed_dispatch_fatal_solar_loss_survives_cold_without_payment_or_clue() {
    use crate::freight::DispatchAction;
    let mut world = World::new_commander(18293).unwrap();
    let cash = world.credits_tenths();
    let tick = world.snapshot().tick;
    let review = world
        .review_module(ModuleAction::Dispatch(DispatchAction::Collect(1)), tick)
        .unwrap();
    world.prepare_module(review, tick).unwrap().commit();
    let record = world.dispatch_records().pop().unwrap();
    solar_approach(&mut world);
    overheat_unfitted_ship(&mut world);
    let cold = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert!(cold.combat_snapshot().unwrap().player.destroyed);
    let lost = cold.dispatch_records().pop().unwrap();
    assert_eq!(lost.issue_revision, record.issue_revision);
    assert!(lost.lost_with_carrier);
    assert!(lost.terminal.is_none());
    assert_eq!(cold.credits_tenths(), cash);
    assert!(
        cold.unwritten_route_clue_progress()
            .unwrap()
            .earned
            .is_empty()
    );
    assert!(
        cold.review_module(
            ModuleAction::Dispatch(DispatchAction::Receive),
            cold.snapshot().tick
        )
        .is_err()
    );
}

fn outward_target(world: &World) -> Vec3 {
    let p = world.snapshot().position;
    let sun = world.celestial_snapshot().unwrap().sun.position;
    Vec3::new(
        2.0 * f64::from(p[0]) - sun.x,
        2.0 * f64::from(p[1]) - sun.y,
        2.0 * f64::from(p[2]) - sun.z,
    )
}

#[test]
fn current_heat_only_trailer_removal_refuses_but_exact_legacy_airborne_continues() {
    let mut world = World::new_commander(18301).unwrap();
    solar_approach(&mut world);
    assert!(world.solar_snapshot().unwrap().cabin_temperature > 0.0);
    assert!(world.solar.receipts.is_empty());
    let before = world.save_bytes().unwrap();
    let removed =
        super::super::persistence::solar_trailer_removed_test_input(&world, false).unwrap();
    assert!(World::from_save_bytes(&removed).is_err());
    assert!(world.restore_bytes(&removed).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
    let genuine_legacy =
        super::super::persistence::solar_trailer_removed_test_input(&world, true).unwrap();
    let mut legacy = World::from_save_bytes(&genuine_legacy).unwrap();
    assert_eq!(legacy.snapshot(), world.snapshot());
    assert_eq!(legacy.solar_snapshot(), Some(SolarSnapshot::default()));
    assert!(legacy.solar.legacy_origin.is_some());
    legacy.step(FlightInput::default()).unwrap();
    assert!(legacy.solar_snapshot().unwrap().cabin_temperature > 0.0);
    assert!(World::from_save_bytes(&legacy.save_bytes().unwrap()).is_ok());
}

#[test]
fn removing_even_exterior_steps_from_a_same_visit_trace_refuses() {
    let mut world = World::new_commander(18302).unwrap();
    solar_approach(&mut world);
    let outward = outward_target(&world);
    aim_3d(&mut world, outward);
    for _ in 0..1200 {
        cruise(&mut world, 34.3);
        if ratio(&world) > 4.05 {
            break;
        }
    }
    assert!(ratio(&world) > 4.05);
    for yaw in [0.2, -0.2, 0.3, -0.3] {
        world
            .step(FlightInput {
                yaw,
                ..FlightInput::default()
            })
            .unwrap();
    }
    let bytes = world.save_bytes().unwrap();
    let mut forged = World::from_save_bytes(&bytes).unwrap();
    let sun = world.celestial_snapshot().unwrap().sun;
    let remove = forged
        .solar
        .runs
        .iter()
        .enumerate()
        .find(|(i, run)| {
            *i > 0
                && *i + 1 < forged.solar.runs.len()
                && run.opening.frame.ratio_and_nose(sun).0 > THERMAL_LIMIT
        })
        .map(|(i, _)| i)
        .unwrap();
    forged.solar.runs.remove(remove);
    assert!(forged.save_bytes().is_err());
    assert!(
        world
            .restore_bytes(&forged.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(world.save_bytes().unwrap(), bytes);
}

fn leave_corona(world: &mut World) {
    let outward = outward_target(world);
    aim_3d(world, outward);
    for _ in 0..1500 {
        cruise(world, 34.3);
        if ratio(world) > 4.05 {
            break;
        }
    }
    assert!(ratio(world) > 4.05);
    stop(world);
}

fn return_to_corona(world: &mut World) {
    let sun = world.celestial_snapshot().unwrap().sun.position;
    aim_3d(world, sun);
    for _ in 0..1500 {
        cruise(world, 34.3);
        if ratio(world) <= 4.0 {
            return;
        }
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    panic!("same commander could not return to the solar region");
}

#[test]
fn chunked_history_crosses_old_limit_and_repeated_same_commander_scooping_remains_live() {
    let mut world = World::new_commander(18303).unwrap();
    secret(&mut world, XyzzyAction::Enter);
    secret(&mut world, XyzzyAction::ToggleUnlimitedFuel);
    journey(&mut world, SystemId::RENAS);
    let identity = world.commander_id();
    let initial_fuel = fuel(&world);
    let initial_cash = world.credits_tenths();
    solar_approach(&mut world);
    let mut first_disk = Vec::new();
    let mut forced_limit = 0;
    for cycle in 0..3 {
        let opening = fuel(&world);
        refill_solar(&mut world, 4);
        assert!(fuel(&world) >= opening + 4);
        leave_corona(&mut world);
        // Distinct accepted controls deliberately cross many independently bounded chunks,
        // including the former whole-file cap, without forcing flight or dropping evidence.
        for index in 0..720 {
            let yaw = if index % 2 == 0 { 0.1 } else { -0.1 };
            world
                .step(FlightInput {
                    yaw,
                    ..FlightInput::default()
                })
                .unwrap();
        }
        let current = world.save_bytes().unwrap();
        assert!(current.len() > crate::save::MAX_SAVE_BYTES);
        assert!(world.solar.runs.completed.len() > 1);
        assert!(
            world
                .solar
                .runs
                .chunks()
                .all(|chunk| chunk.len() <= RUNS_PER_CHUNK)
        );
        assert_eq!(world.commander_id(), identity);
        let cold = World::from_save_bytes(&current).unwrap();
        assert_eq!(cold.snapshot(), world.snapshot());
        assert_eq!(cold.solar_snapshot(), world.solar_snapshot());
        assert_eq!(cold.solar, world.solar);
        if cycle == 0 {
            forced_limit = world.solar.encoded_len();
            first_disk = current;
        } else {
            world.solar.file_limit = Some(forced_limit);
            assert!(world.solar_save_limit_reached(forced_limit));
            assert_eq!(world.save_bytes(), Err(SaveError::TooLarge));
            let previous = World::from_save_bytes(&first_disk).unwrap();
            assert_eq!(previous.commander_id(), identity);
            assert!(fuel(&previous) < fuel(&world));
        }
        world = cold;
        if cycle < 2 {
            return_to_corona(&mut world);
        }
    }
    assert!(fuel(&world) >= initial_fuel + 12);
    assert_eq!(world.credits_tenths(), initial_cash);
    assert!(!world.solar_save_limit_reached(crate::save::MAX_SOLAR_HISTORY_BYTES));
    world.solar.file_limit = Some(forced_limit);
    assert!(world.solar_save_limit_reached(forced_limit));
    assert_eq!(world.save_bytes(), Err(SaveError::TooLarge));
    return_to_corona(&mut world);
    // Even after the forced outer refusal, the exact same pilot can still steer and scoop.
    refill_solar(&mut world, 2);
    assert!(world.solar_save_limit_reached(forced_limit));
    assert_eq!(world.save_bytes(), Err(SaveError::TooLarge));
    assert!(fuel(&world) >= initial_fuel + 14);
}

#[test]
fn malformed_chunk_size_and_truncated_history_refuse_without_live_replacement() {
    let mut world = World::new_commander(18306).unwrap();
    solar_approach(&mut world);
    stop(&mut world);
    for i in 0..(RUNS_PER_CHUNK + 4) {
        world
            .step(FlightInput {
                yaw: if i % 2 == 0 { 0.1 } else { -0.1 },
                ..FlightInput::default()
            })
            .unwrap();
    }
    let before = world.save_bytes().unwrap();
    let start = before.len() - 8 - world.solar.encoded_len();
    let count_at = start + 8 + 1;
    let length_at = count_at + 4;
    let reseal = |bytes: &mut [u8]| {
        let end = bytes.len() - 8;
        let digest = crate::save::checksum(&bytes[..end]);
        bytes[end..].copy_from_slice(&digest.to_le_bytes());
    };
    let mut oversized = before.clone();
    oversized[count_at..count_at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    reseal(&mut oversized);
    let mut wrong_chunk = before.clone();
    wrong_chunk[length_at..length_at + 2].copy_from_slice(&1_u16.to_le_bytes());
    reseal(&mut wrong_chunk);
    let mut truncated = before.clone();
    truncated.remove(length_at + 2);
    reseal(&mut truncated);
    for bad in [oversized, wrong_chunk, truncated] {
        assert!(World::from_save_bytes(&bad).is_err());
        assert!(world.restore_bytes(&bad).is_err());
        assert_eq!(world.save_bytes().unwrap(), before);
    }
}

#[test]
fn genuine_planet_refusal_replays_attempted_input_and_rejects_forged_clear_stop_or_crossing() {
    let mut world = World::new_commander(18307).unwrap();
    let initial_fuel = fuel(&world);
    solar_approach(&mut world);
    leave_corona(&mut world);
    let planet = world.celestial_snapshot().unwrap().planet;
    aim_3d(&mut world, planet.position);
    for _ in 0..6000 {
        cruise(&mut world, 34.3);
        if world.snapshot().motion_refusal == Some(crate::MotionRefusal::CelestialContact) {
            break;
        }
    }
    assert_eq!(
        world.snapshot().motion_refusal,
        Some(crate::MotionRefusal::CelestialContact)
    );
    assert_eq!(world.snapshot().speed, 0.0);
    let last = world.solar.runs.last().unwrap();
    let Motion::Stopped {
        kind: StopKind::Celestial,
        input,
    } = last.motion
    else {
        panic!("actual stop lost its attempted control");
    };
    let scene = State::scene(
        &world,
        last.opening.system,
        last.opening.visit,
        last.opening.tick,
    )
    .unwrap();
    let hull = world.station_data.as_ref().unwrap().player_radius();
    let attempted = last
        .opening
        .frame
        .advance(Motion::Active(input), crate::PlayerHullKind::Peregrine)
        .unwrap();
    assert!(
        scene
            .planet
            .blocks_motion(last.opening.frame.position, attempted.position, hull)
    );
    assert!(
        !scene
            .sun
            .blocks_motion(last.opening.frame.position, attempted.position, hull)
    );
    assert!(
        super::history::replay_motion(
            &world,
            last.opening,
            Motion::Active(input),
            &scene,
            None,
            &mut std::iter::empty()
        )
        .is_err()
    );
    let bytes = world.save_bytes().unwrap();
    let cold = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(cold.snapshot(), world.snapshot());
    assert_eq!(cold.solar_snapshot(), world.solar_snapshot());
    assert_eq!(fuel(&cold), initial_fuel);
    let mut crossing = World::from_save_bytes(&bytes).unwrap();
    let index = crossing.solar.runs.len() - 1;
    crossing.solar.runs.get_mut(index).unwrap().motion = Motion::Active(input);
    assert!(crossing.save_bytes().is_err());
    assert!(
        world
            .restore_bytes(&crossing.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(world.save_bytes().unwrap(), bytes);

    // A stationary turn near the genuine obstacle is legal, and must not be replaced
    // by a made-up stop merely because a stop occurred on the preceding tick.
    let orientation = world.snapshot().orientation;
    world
        .step(FlightInput {
            yaw: 0.25,
            ..FlightInput::default()
        })
        .unwrap();
    assert_eq!(world.snapshot().motion_refusal, None);
    assert_ne!(world.snapshot().orientation, orientation);
    let genuine = world.save_bytes().unwrap();
    let mut forged = World::from_save_bytes(&genuine).unwrap();
    let index = forged.solar.runs.len() - 1;
    let run = forged.solar.runs.last().unwrap();
    let Motion::Active(input) = run.motion else {
        panic!("expected clear movement");
    };
    let clear_scene = State::scene(
        &world,
        run.opening.system,
        run.opening.visit,
        run.opening.tick,
    )
    .unwrap();
    let attempted = run
        .opening
        .frame
        .advance(Motion::Active(input), crate::PlayerHullKind::Peregrine)
        .unwrap();
    assert!(!clear_scene.blocks_motion(run.opening.frame.position, attempted.position, hull));
    let fake = Motion::Stopped {
        input,
        kind: StopKind::Celestial,
    };
    assert!(
        super::history::replay_motion(
            &world,
            run.opening,
            fake,
            &clear_scene,
            None,
            &mut std::iter::empty()
        )
        .is_err()
    );
    forged.solar.runs.get_mut(index).unwrap().motion = fake;
    assert!(forged.save_bytes().is_err());
    assert!(
        world
            .restore_bytes(&forged.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(world.save_bytes().unwrap(), genuine);
    let recovered = World::from_save_bytes(&genuine).unwrap();
    assert_eq!(recovered.solar_snapshot(), world.solar_snapshot());
    assert_eq!(fuel(&recovered), initial_fuel);
}

#[test]
fn actual_npc_scene_proves_only_the_bound_attempted_sweep_and_witness_codec() {
    let mut world = World::new_commander(18308).unwrap();
    launch(&mut world);
    let mut retained = None;
    for _ in 0..1800 {
        cruise(&mut world, 34.3);
        let identity = world.combat_identity().unwrap();
        let scene = world.traffic.missile_scene(
            identity.system,
            world.state.tick,
            crate::traffic::SceneStage::Command(world.combat.emissions),
        );
        if !scene.bodies.is_empty() {
            retained = Some(scene);
            break;
        }
    }
    let identity = world.combat_identity().unwrap();
    let scene = retained.unwrap_or_else(|| panic!("actual traffic scene"));
    let actor = scene
        .bodies
        .first()
        .unwrap_or_else(|| panic!("actual traffic scene"));
    let radius = world
        .station_data
        .as_ref()
        .unwrap()
        .player_clearance_radius()
        * 2.0
        + 1.0;
    // This is an isolated geometry fixture using an actual retained scene, not a claimed
    // public collision journey. The attempted player pose is deliberately placed just outside.
    let frame = MotionFrame {
        position: [
            actor.pose.position.x as f32,
            actor.pose.position.y as f32,
            (actor.pose.position.z - radius - 0.1) as f32,
        ],
        orientation: [0.0, 0.0, 0.0, 1.0],
        speed: 34.3,
    };
    let opening = Anchor {
        system: identity.system,
        visit: identity.visit,
        tick: world.state.tick,
        clock: world.combat.physical_tick(),
        frame,
    };
    let witness = NpcRefusal {
        clock: opening.clock,
        scene,
    };
    let mut writer = Writer::new();
    witness.encode(&mut writer).unwrap();
    let mut reader = Reader::new(&writer.bytes);
    assert_eq!(NpcRefusal::decode(&mut reader).unwrap(), witness);
    assert!(reader.finished());
    let motion = Motion::Stopped {
        input: FlightInput::default(),
        kind: StopKind::Traffic,
    };
    let scene = State::scene(&world, identity.system, identity.visit, opening.tick).unwrap();
    let stopped = super::history::replay_motion(
        &world,
        opening,
        motion,
        &scene,
        None,
        &mut std::iter::once(&witness),
    )
    .unwrap();
    assert_eq!(stopped.position, frame.position);
    assert_eq!(stopped.speed, 0.0);
    assert!(
        super::history::replay_motion(
            &world,
            opening,
            motion,
            &scene,
            None,
            &mut std::iter::empty()
        )
        .is_err()
    );
    let mut wrong = witness;
    wrong.clock += 1;
    assert!(
        super::history::replay_motion(
            &world,
            opening,
            motion,
            &scene,
            None,
            &mut std::iter::once(&wrong)
        )
        .is_err()
    );
}

#[test]
fn historic_mule_active_crossing_refuses_but_genuine_legacy_overlap_escape_remains_legal() {
    let bytes = include_bytes!("../../../tests/fixtures/renas-mule-pre-pr-overlap-v17.bin");
    let mut world = World::from_save_bytes(bytes).unwrap();
    let identity = world.combat_identity().unwrap();
    assert!(world.mule_actor_snapshot().is_none());
    let collider = world
        .historic_renas_mule_collider(identity.system, identity.visit)
        .unwrap()
        .unwrap();
    let opening = State::anchor(
        world.state,
        identity.system,
        identity.visit,
        world.combat.physical_tick(),
    );
    let scene = State::scene(&world, identity.system, identity.visit, opening.tick).unwrap();
    let clear_escape = super::history::replay_motion(
        &world,
        opening,
        Motion::Active(FlightInput::default()),
        &scene,
        Some(&collider),
        &mut std::iter::empty(),
    )
    .unwrap();
    world.step(FlightInput::default()).unwrap();
    assert_eq!(MotionFrame::from(world.snapshot()), clear_escape);
    assert_eq!(world.snapshot().motion_refusal, None);

    let data = world.station_data.as_ref().unwrap();
    let source = crate::mule::MuleHullSource::bundled().unwrap();
    let berth = data.renas_mule_berth(&source).unwrap();
    let launch = data.launch_start().unwrap().position;
    let dx = berth.position.x - f64::from(launch[0]);
    let dz = berth.position.z - f64::from(launch[2]);
    let distance = dx.hypot(dz);
    let direction = [dx / distance, 0.0, dz / distance];
    let yaw = dx.atan2(dz) as f32;
    let orientation = super::super::axis_rotation(1, yaw);
    let span = f64::from(
        super::super::MAX_SPEED
            * super::super::FIXED_DELTA_SECONDS
            * super::super::DISTANCE_PER_SPEED_UNIT,
    );
    let input = FlightInput::default();
    let mut crossing = None;
    for step in 0..=((distance / span).ceil() as usize + 2) {
        let frame = MotionFrame {
            position: core::array::from_fn(|axis| {
                launch[axis] + (direction[axis] * step as f64 * span) as f32
            }),
            orientation,
            speed: super::super::MAX_SPEED,
        };
        let attempted = frame
            .advance(Motion::Active(input), crate::PlayerHullKind::Peregrine)
            .unwrap();
        let before = crate::station::flight_pose(frame.position, frame.orientation).unwrap();
        let after = crate::station::flight_pose(attempted.position, attempted.orientation).unwrap();
        if !data.crosses_aperture(before, after)
            && data.motion_clear(before, after, 0.0)
            && world
                .historic_renas_mule_motion_blocked(Some(&collider), before, after, 0.0)
                .unwrap()
        {
            crossing = Some(frame);
            break;
        }
    }
    let frame = crossing.unwrap_or_else(|| {
        panic!("actual fixed Mule collider did not block one accepted-size sweep")
    });
    let mut anchor = State::anchor(
        world.state,
        identity.system,
        identity.visit,
        world.combat.physical_tick(),
    );
    anchor.frame = frame;
    let scene = State::scene(&world, identity.system, identity.visit, anchor.tick).unwrap();
    assert!(
        super::history::replay_motion(
            &world,
            anchor,
            Motion::Active(input),
            &scene,
            Some(&collider),
            &mut std::iter::empty()
        )
        .is_err()
    );
    let stopped = super::history::replay_motion(
        &world,
        anchor,
        Motion::Stopped {
            input,
            kind: StopKind::Mule,
        },
        &scene,
        Some(&collider),
        &mut std::iter::empty(),
    )
    .unwrap();
    assert_eq!(stopped.position, frame.position);
    assert_eq!(stopped.speed, 0.0);
    anchor.frame.speed = 0.0;
    assert!(
        super::history::replay_motion(
            &world,
            anchor,
            Motion::Active(input),
            &scene,
            Some(&collider),
            &mut std::iter::empty()
        )
        .is_ok()
    );
}
#[test]
fn fitted_partially_charged_emp_keeps_actual_final_transfer_when_solar_heat_kills() {
    use crate::{EmpAxis, EmpEquipmentAction};
    let mut world = earned_world(18304);
    let fit = |world: &mut World, command| {
        let tick = world.snapshot().tick;
        let review = world.review_emp_equipment(command, tick).unwrap();
        world.prepare_emp_equipment(review, tick).unwrap().commit();
    };
    fit(&mut world, EmpEquipmentAction::Purchase);
    let item = world.owned_emp().unwrap().item;
    for _ in 0..2 {
        fit(
            &mut world,
            EmpEquipmentAction::Upgrade {
                item,
                axis: EmpAxis::Capacity,
            },
        );
    }
    solar_approach(&mut world);
    let mut final_before = 0.0;
    for _ in 0..1500 {
        if world.combat_snapshot().unwrap().player.destroyed {
            break;
        }
        final_before = world.emp_snapshot().unwrap().bank_energy;
        if ratio(&world) > 1.95 {
            cruise(&mut world, 34.3);
        } else {
            cruise(&mut world, 0.0);
        }
    }
    assert_eq!(world.solar_snapshot().unwrap().cabin_temperature, 1.0);
    let emp = world.emp_snapshot().unwrap();
    assert!(emp.bank_energy > final_before && emp.bank_energy < emp.bank_capacity);
    let cold = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(cold.emp_snapshot(), Some(emp));
    assert_eq!(cold.combat_snapshot(), world.combat_snapshot());
}

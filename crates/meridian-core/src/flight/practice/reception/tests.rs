#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::practice::tests::{aim_at, consent, launch, shoot, step};
use crate::navigation::BASIC_SPOOL_TICKS;
use crate::{FlightInput, PULSE_COOLDOWN_TICKS, PulseContact};

fn tick(world: &World) -> u64 {
    world.snapshot().tick
}

fn three_hits(seed: u64) -> World {
    let mut world = World::new_commander(seed).unwrap();
    consent(&mut world);
    launch(&mut world);
    let beacons = world.practice_snapshot().unwrap().beacons.unwrap();
    assert!(
        matches!(shoot(&mut world).contact,PulseContact::PracticeBeacon { id, first_hit:true,.. } if id == beacons[1].id)
    );
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
        assert!(
            matches!(shoot(&mut world).contact,PulseContact::PracticeBeacon { id,first_hit:true,.. } if id == beacons[ordinal].id)
        );
        if ordinal == 0 {
            step(&mut world, PULSE_COOLDOWN_TICKS);
        }
    }
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 3);
    assert_eq!(world.practice_reception_snapshot().unwrap().quiet_ticks, 0);
    world
}

fn approach(world: &mut World, near: bool) {
    for _ in 0..9000 {
        let before = world.snapshot();
        if before.mode == FlightMode::Docked || (near && before.position[2] < 145.0) {
            return;
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
    }
    panic!("actual approach did not reach the intended physical boundary");
}

fn jump_and_dock(world: &mut World, destination: SystemId) {
    world.request_jump(destination, tick(world)).unwrap();
    for _ in 0..BASIC_SPOOL_TICKS {
        step(world, 1);
    }
    approach(world, false);
    assert_eq!(world.current_dock_context().unwrap().system(), destination);
}

#[test]
fn quiet_clock_pause_and_every_actual_discharge_reset_before_paid_state_is_untouched() {
    let mut world = three_hits(1201);
    step(&mut world, 179);
    assert_eq!(
        world.practice_reception_snapshot().unwrap().quiet_ticks,
        179
    );
    assert!(!world.practice_reception_snapshot().unwrap().ready);
    world.request_pause(true, tick(&world)).unwrap();
    assert_eq!(
        world.step(FlightInput::default()),
        Err(crate::StepError::Paused)
    );
    assert_eq!(
        world.practice_reception_snapshot().unwrap().quiet_ticks,
        179
    );
    world.request_pause(false, tick(&world)).unwrap();
    step(&mut world, 1);
    assert!(world.practice_reception_snapshot().unwrap().ready);
    let z = f64::from(world.snapshot().position[2]);
    aim_at(&mut world, Vec3::new(1000.0, 0.0, z + 2000.0));
    assert_eq!(shoot(&mut world).contact, PulseContact::Clear);
    assert_eq!(world.practice_reception_snapshot().unwrap().quiet_ticks, 0);
    step(&mut world, 180);
    assert!(world.practice_reception_snapshot().unwrap().ready);
    aim_at(&mut world, Vec3::new(70.0, 0.0, 136.32));
    assert!(matches!(
        shoot(&mut world).contact,
        PulseContact::StationBlocked { .. }
    ));
    assert_eq!(world.practice_reception_snapshot().unwrap().quiet_ticks, 0);
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 3);
    assert_eq!(world.credits_tenths(), Some(7200));
    let restored = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(
        restored.practice_reception_snapshot(),
        world.practice_reception_snapshot()
    );
}

#[test]
fn later_actual_avenor_reception_publishes_once_and_refuses_ready_renas() {
    let mut world = three_hits(1211);
    step(&mut world, 180);
    assert!(world.practice_reception_snapshot().unwrap().ready);
    assert_eq!(
        world.review_practice_reception(tick(&world)),
        Err(PracticeError::AvenorBerthRequired)
    );
    jump_and_dock(&mut world, SystemId::RENAS);
    assert!(world.practice_reception_snapshot().unwrap().ready);
    assert_eq!(
        world.review_practice_reception(tick(&world)),
        Err(PracticeError::AvenorBerthRequired)
    );
    let mut reset = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    launch(&mut reset);
    assert_eq!(shoot(&mut reset).contact, PulseContact::Clear);
    assert_eq!(reset.practice_reception_snapshot().unwrap().quiet_ticks, 0);
    jump_and_dock(&mut reset, SystemId::AVENOR);
    assert_eq!(
        reset.review_practice_reception(tick(&reset)),
        Err(PracticeError::ReceptionNotReady)
    );
    launch(&mut world);
    jump_and_dock(&mut world, SystemId::AVENOR);
    let review = world.review_practice_reception(tick(&world)).unwrap();
    let before = world.save_bytes().unwrap();
    let now = tick(&world);
    drop(world.prepare_practice_reception(review, now).unwrap());
    assert_eq!(world.save_bytes().unwrap(), before);
    let equipment = world.equipment_snapshot();
    let wallet = world.credits_tenths();
    step(&mut world, 1);
    let now = tick(&world);
    let prepared = world.prepare_practice_reception(review, now).unwrap();
    let bytes = prepared.bytes().to_vec();
    assert!(
        World::from_save_bytes(&bytes)
            .unwrap()
            .practice_reception_snapshot()
            .unwrap()
            .trace
            .is_some()
    );
    prepared.commit();
    assert_eq!(world.save_bytes().unwrap(), bytes);
    assert_resealed_wrong_terminal_refused(&mut world, &bytes);
    let received = world.practice_reception_snapshot().unwrap();
    let trace = received.trace.unwrap();
    assert!(!received.ready);
    assert_eq!(trace.source_id, "l337.training/safe-gunnery");
    assert_eq!(trace.clue_id, "safe-gunnery-family-trace");
    assert_eq!(
        trace.receipt_id,
        "l337.unwritten-route/clue/safe-gunnery-family-trace"
    );
    assert_eq!(trace.owner_seed, 1211);
    assert_eq!(trace.carrier_ordinal, 1212);
    assert_eq!(
        trace.dock_sequence,
        world.current_dock_context().unwrap().sequence()
    );
    assert_eq!(trace.confirmed_tick, now);
    assert_eq!(world.credits_tenths(), wallet);
    assert_eq!(world.equipment_snapshot(), equipment);
    assert!(matches!(
        world.prepare_practice_reception(review, now),
        Err(PracticeError::AlreadyReceived)
    ));
    let mut continued = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(continued.practice_reception_snapshot(), Some(received));
    launch(&mut continued);
    shoot(&mut continued);
    assert_eq!(continued.practice_reception_snapshot(), Some(received));
    assert!(continued.save_bytes().is_ok());
}

#[test]
fn premature_actual_avenor_entry_freezes_quiet_progress_and_refuses_reception() {
    let mut world = three_hits(1221);
    // Turn back toward the actual port, then reset the attained interval just
    // outside the entry plane so the following real approach docks too soon.
    aim_at(&mut world, Vec3::new(0.0, 0.0, 0.0));
    approach(&mut world, true);
    assert_eq!(world.snapshot().mode, FlightMode::Flight);
    shoot(&mut world);
    approach(&mut world, false);
    assert_eq!(
        world.current_dock_context().unwrap().system(),
        SystemId::AVENOR
    );
    assert!(world.practice_reception_snapshot().unwrap().quiet_ticks < 20);
    assert_eq!(
        world.review_practice_reception(tick(&world)),
        Err(PracticeError::ReceptionNotReady)
    );
    let before = world.practice_reception_snapshot();
    step(&mut world, 180);
    assert_eq!(world.practice_reception_snapshot(), before);
    assert_eq!(
        World::from_save_bytes(&world.save_bytes().unwrap())
            .unwrap()
            .practice_reception_snapshot(),
        before
    );
}

#[test]
fn v5_active_and_retired_hits_do_not_mint_an_unobserved_interval_or_trace() {
    let mut world = three_hits(1231);
    step(&mut world, 180);
    assert!(world.practice_reception_snapshot().unwrap().ready);
    let bytes = crate::flight::persistence::v5_test_fixture(&world).unwrap();
    let mut active = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(active.practice_snapshot(), world.practice_snapshot());
    assert_eq!(active.pulse_laser_snapshot(), world.pulse_laser_snapshot());
    assert_eq!(
        active.practice_reception_snapshot().unwrap(),
        PracticeReceptionSnapshot {
            quiet_ticks: 0,
            ready: false,
            trace: None
        }
    );
    step(&mut active, 180);
    assert!(!active.practice_reception_snapshot().unwrap().ready);
    step(&mut active, 1);
    assert!(active.practice_reception_snapshot().unwrap().ready);
    jump_and_dock(&mut world, SystemId::RENAS);
    let old_retired = crate::flight::persistence::v5_test_fixture(&world).unwrap();
    let mut retired = World::from_save_bytes(&old_retired).unwrap();
    assert_eq!(retired.practice_snapshot(), world.practice_snapshot());
    assert_eq!(
        retired.practice_reception_snapshot().unwrap(),
        PracticeReceptionSnapshot {
            quiet_ticks: 0,
            ready: false,
            trace: None
        }
    );
    step(&mut retired, 180);
    assert!(!retired.practice_reception_snapshot().unwrap().ready);
}

fn assert_resealed_wrong_terminal_refused(world: &mut World, live: &[u8]) {
    use crate::save::{Writer, checksum};
    let practice = world.practice.unwrap();
    let mut original = Writer::new();
    practice.encode(&mut original);
    let end = live.len() - 13 - 1 - original.bytes.len();
    let mut original_reception = Writer::new();
    practice.encode_reception(&mut original_reception);
    let start = end - original_reception.bytes.len();
    assert_eq!(&live[start..end], original_reception.bytes.as_slice());
    let mut too_early = practice;
    too_early.reception.anchor = Some(0);
    let mut foreign = practice;
    foreign.reception.completion.as_mut().unwrap().dock = World::new_commander(2999)
        .unwrap()
        .current_dock_context()
        .unwrap();
    for bad in [too_early, foreign] {
        let mut w = Writer::new();
        bad.encode_reception(&mut w);
        let mut dirty = live.to_vec();
        dirty[start..end].copy_from_slice(&w.bytes);
        let end = dirty.len() - 8;
        let digest = checksum(&dirty[..end]);
        dirty[end..].copy_from_slice(&digest.to_le_bytes());
        assert_eq!(
            world.restore_bytes(&dirty),
            Err(SaveError::InvalidState("T02 ceasefire or reception"))
        );
        assert_eq!(world.save_bytes().unwrap().as_slice(), live);
    }
}

#![allow(clippy::unwrap_used)]
use super::*;
use crate::equipment::Command as Equipment;
use crate::flight::missiles::tests::{dock, finish, fire, fly_axis, stop};
use crate::flight::practice::tests::{aim_at, consent, launch, shoot, step};
use crate::freight::g02::{Choice, Command, Event};
use crate::save::{SAVE_VERSION, checksum};

fn cold(world: &World) -> World {
    let saved = world.save_bytes().unwrap();
    assert_eq!(u16::from_le_bytes([saved[8], saved[9]]), SAVE_VERSION);
    let loaded = World::from_save_bytes(&saved).unwrap();
    assert_eq!(loaded.snapshot(), world.snapshot());
    assert_eq!(
        loaded.practice_ledger_snapshot(),
        world.practice_ledger_snapshot()
    );
    assert_eq!(loaded.missile_snapshot(), world.missile_snapshot());
    assert_eq!(loaded.combat_snapshot(), world.combat_snapshot());
    assert_eq!(loaded.pulse_laser_snapshot(), world.pulse_laser_snapshot());
    assert_eq!(loaded.equipment_snapshot(), world.equipment_snapshot());
    assert_eq!(loaded.credits_tenths(), world.credits_tenths());
    loaded
}
fn close(world: &mut World) -> PracticeOutcome {
    let before = world.practice_ledger_snapshot().unwrap();
    let current = before.attempts.last().unwrap();
    assert_eq!(before.active, Some(current.id));
    assert_eq!(current.outcome, None);
    let review = world
        .review_practice_closure(world.snapshot().tick)
        .unwrap();
    assert_eq!(review.attempt(), current.id);
    let outcome = review.outcome();
    world
        .prepare_practice_closure(review, world.snapshot().tick)
        .unwrap()
        .commit();
    let after = world.practice_ledger_snapshot().unwrap();
    assert_eq!(after.active, None);
    assert_eq!(after.attempts.len(), before.attempts.len());
    assert_eq!(after.attempts.last().unwrap().id, current.id);
    assert_eq!(after.attempts.last().unwrap().outcome, Some(outcome));
    let archived = before.attempts.len() - 1;
    assert_eq!(&after.attempts[..archived], &before.attempts[..archived]);
    outcome
}
fn receive(world: &mut World) {
    let review = world
        .review_practice_reception(world.snapshot().tick)
        .unwrap();
    world
        .prepare_practice_reception(review, world.snapshot().tick)
        .unwrap()
        .commit();
}
fn three_hits(world: &mut World) {
    launch(world);
    let beacons = world.practice_snapshot().unwrap().beacons.unwrap();
    assert!(matches!(
        shoot(world).contact,
        crate::PulseContact::PracticeBeacon {
            first_hit: true,
            ..
        }
    ));
    stop(world);
    for ordinal in [0, 2] {
        aim_at(world, beacons[ordinal].pose.position);
        shoot(world);
        step(world, 18);
    }
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 3);
    step(world, 180);
    dock(world);
    assert!(world.practice_reception_snapshot().unwrap().ready);
}
fn funded(seed: u64) -> World {
    let mut world = crate::flight::equipment::tests::earned_world(seed);
    if world.snapshot().navigation.unwrap().fuel_units < 68 {
        world.request_refuel(world.snapshot().tick).unwrap();
    }
    crate::flight::equipment::tests::journey(&mut world, SystemId::AVENOR);
    world
}
fn refused(world: &mut World, bytes: &[u8]) {
    let prior = world.save_bytes().unwrap();
    assert!(World::from_save_bytes(bytes).is_err());
    assert!(world.restore_bytes(bytes).is_err());
    assert_eq!(world.save_bytes().unwrap(), prior);
}
fn corrupt(world: &mut World, change: impl FnOnce(&mut World)) {
    let mut changed = cold(world);
    change(&mut changed);
    let bytes = changed.unchecked_test_save().unwrap();
    refused(world, &bytes);
}

#[test]
fn failed_funded_missile_attempt_closes_retries_and_keeps_all_old_custody_and_law() {
    let mut world = funded(3001);
    world
        .request_equipment(Equipment::PurchaseCargoBay, world.snapshot().tick)
        .unwrap();
    let paid = world.equipment_snapshot().unwrap().owned;
    let cash = world.credits_tenths();
    consent(&mut world);
    launch(&mut world);
    stop(&mut world);
    let old_targets = world
        .practice_snapshot()
        .unwrap()
        .beacons
        .unwrap()
        .map(|b| b.id);
    fly_axis(&mut world, 0, 3000.0, 4.0);
    fly_axis(&mut world, 2, -3000.0, 4.0);
    fly_axis(&mut world, 0, 120.0, 1.0);
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[2];
    aim_at(&mut world, target.pose.position);
    let origin = fire(&mut world, target.id);
    assert_eq!(origin.item, 3011);
    finish(&mut world, origin.projectile);
    assert_eq!(world.legal_value(), Some(64));
    let incident = world.combat_snapshot().unwrap().last_ordinary_impact;
    fly_axis(&mut world, 0, 3000.0, 4.0);
    fly_axis(&mut world, 2, 3000.0, 4.0);
    fly_axis(&mut world, 0, 0.0, 1.0);
    dock(&mut world);
    let stock = world.missile_snapshot().unwrap();
    let before = world.save_bytes().unwrap();
    let review = world
        .review_practice_closure(world.snapshot().tick)
        .unwrap();
    assert_eq!(review.outcome(), PracticeOutcome::Failed);
    let old_id = review.attempt();
    {
        let prepared = world
            .prepare_practice_closure(review, world.snapshot().tick)
            .unwrap();
        let staged = World::from_save_bytes(prepared.bytes()).unwrap();
        assert_eq!(
            staged.practice_ledger_snapshot().unwrap().attempts[0].outcome,
            Some(PracticeOutcome::Failed)
        );
    }
    assert_eq!(world.save_bytes().unwrap(), before);
    assert_eq!(close(&mut world), PracticeOutcome::Failed);
    world = cold(&world);
    assert_eq!(world.missile_snapshot().unwrap(), stock);
    assert_eq!(world.practice_reception_snapshot().unwrap().trace, None);
    consent(&mut world);
    world = cold(&world);
    let ledger = world.practice_ledger_snapshot().unwrap();
    assert_eq!(ledger.attempts.len(), 2);
    assert_eq!(ledger.attempts[0].id, old_id);
    assert_eq!(ledger.attempts[0].ordinary_hit, incident);
    assert_eq!(ledger.attempts[0].targets, Some(old_targets));
    assert_ne!(ledger.active, Some(old_id));
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(world.equipment_snapshot().unwrap().owned, paid);
    assert_eq!(world.legal_value(), Some(64));
    launch(&mut world);
    stop(&mut world);
    let new = world.practice_snapshot().unwrap().beacons.unwrap();
    assert_ne!(new.map(|b| b.id), old_targets);
    let next = fire(&mut world, new[1].id);
    assert_ne!(next.projectile, origin.projectile);
    assert_ne!(next.item, origin.item);
    assert_eq!(next.item, 3010);
    world = cold(&world);
    finish(&mut world, next.projectile);
    world = cold(&world);
    let missiles = world.missile_snapshot().unwrap();
    assert_eq!(missiles.projectiles[0].unwrap().origin, origin);
    assert_eq!(missiles.rounds.iter().flatten().count(), 1);
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 1);
    assert_eq!(
        world.practice_snapshot().unwrap().status,
        super::super::PracticeStatus::Active
    );
    assert_eq!(
        world.practice_ledger_snapshot().unwrap().attempts[0].outcome,
        Some(PracticeOutcome::Failed)
    );
    assert_eq!(world.legal_value(), Some(64));
    assert_eq!(
        world.combat_snapshot().unwrap().last_ordinary_impact,
        incident
    );
    corrupt(&mut world, |w| {
        w.practice_history[0].ordinary_hit = None;
        w.practice_history[0].settlement = Some(PracticeOutcome::Abandoned);
        w.practice_history[0].cancel.as_mut().unwrap().outcome = PracticeOutcome::Abandoned;
    });
}

#[test]
fn explicit_incomplete_closure_has_no_reward_and_old_quotes_cannot_confirm_a_retry() {
    let mut world = World::new_commander(3101).unwrap();
    let first = world.review_practice(0).unwrap();
    let initial = world.save_bytes().unwrap();
    {
        let prepared = world.prepare_practice_confirmation(first, 0).unwrap();
        let staged = World::from_save_bytes(prepared.bytes()).unwrap();
        assert_eq!(staged.practice_ledger_snapshot().unwrap().attempts.len(), 1);
        // The host did not publish these valid bytes, so it drops the preparation.
        drop(prepared);
    }
    assert_eq!(world.save_bytes().unwrap(), initial);
    consent(&mut world);
    let old = world.review_practice_closure(0).unwrap();
    assert_eq!(old.outcome(), PracticeOutcome::Abandoned);
    let accepted = world.save_bytes().unwrap();
    {
        let prepared = world.prepare_practice_closure(old, 0).unwrap();
        drop(prepared);
    }
    assert_eq!(world.save_bytes().unwrap(), accepted);
    close(&mut world);
    let retry = world.review_practice(0).unwrap();
    assert!(world.prepare_practice_confirmation(first, 0).is_err());
    world
        .prepare_practice_confirmation(retry, 0)
        .unwrap()
        .commit();
    assert!(world.prepare_practice_closure(old, 0).is_err());
    assert_eq!(
        world.practice_ledger_snapshot().unwrap().attempts[0].outcome,
        Some(PracticeOutcome::Abandoned)
    );
    assert_eq!(world.credits_tenths(), Some(7200));
    assert_eq!(world.practice_reception_snapshot().unwrap().trace, None);
    assert_eq!(
        world
            .missile_snapshot()
            .unwrap()
            .rounds
            .iter()
            .flatten()
            .count(),
        3
    );
    world = cold(&world);
    assert_eq!(world.practice_ledger_snapshot().unwrap().attempts.len(), 2);
    world.request_pause(true, 0).unwrap();
    assert_eq!(world.review_practice_closure(0), Err(PracticeError::Paused));
    assert_eq!(world.review_practice(0), Err(PracticeError::Paused));
    world.request_pause(false, 0).unwrap();
    let quoted = world.review_practice_closure(0).unwrap();
    let saved = world.save_bytes().unwrap();
    world.restore_bytes(&saved).unwrap();
    assert!(world.prepare_practice_closure(quoted, 0).is_err());
}

#[test]
fn two_completed_attempts_keep_the_first_trusted_trace_once_through_cold_continue() {
    let mut world = World::new_commander(3201).unwrap();
    consent(&mut world);
    three_hits(&mut world);
    receive(&mut world);
    let trace = world.practice_reception_snapshot().unwrap().trace.unwrap();
    let paid = world.credits_tenths();
    let initial_id = world.practice_ledger_snapshot().unwrap().attempts[0].id;
    assert_eq!(
        world.review_practice_closure(world.snapshot().tick),
        Err(PracticeError::NoActiveAttempt)
    );
    world = cold(&world);
    consent(&mut world);
    assert_eq!(
        world.practice_reception_snapshot().unwrap().trace,
        Some(trace)
    );
    three_hits(&mut world);
    receive(&mut world);
    world = cold(&world);
    let ledger = world.practice_ledger_snapshot().unwrap();
    assert_eq!(ledger.attempts.len(), 2);
    assert!(
        ledger
            .attempts
            .iter()
            .all(|a| a.outcome == Some(PracticeOutcome::Complete))
    );
    assert_eq!(
        ledger
            .attempts
            .iter()
            .filter(|a| a.awarded_first_trace)
            .count(),
        1
    );
    assert_eq!(ledger.attempts[0].id, initial_id);
    assert_eq!(ledger.trace, Some(trace));
    assert_eq!(
        world.practice_reception_snapshot().unwrap().trace,
        Some(trace)
    );
    assert_eq!(world.credits_tenths(), paid);
    corrupt(&mut world, |w| w.practice_trace = None);
    corrupt(&mut world, |w| {
        w.practice_trace = Some(w.practice.unwrap().attempt_id())
    });
}

#[test]
fn sixteen_real_reviewed_offers_exhaust_without_grants_and_shared_occupancy_is_derived() {
    let mut world = World::new_commander(3301).unwrap();
    for i in 0..PRACTICE_MAX_OFFERS {
        consent(&mut world);
        assert_eq!(
            world
                .practice_ledger_snapshot()
                .unwrap()
                .active
                .unwrap()
                .ordinal(),
            i as u8
        );
        assert_eq!(close(&mut world), PracticeOutcome::Abandoned);
    }
    let ledger = world.practice_ledger_snapshot().unwrap();
    assert_eq!(ledger.attempts.len(), 16);
    assert_eq!(ledger.offers_remaining, 0);
    assert_eq!(ledger.retained_mission_records, 16);
    assert_eq!(ledger.active_services, 0);
    assert_eq!(world.review_practice(0), Err(PracticeError::OfferCapacity));
    let _continued = cold(&world);
    assert_eq!(world.credits_tenths(), Some(7200));
    assert_eq!(
        world
            .missile_snapshot()
            .unwrap()
            .rounds
            .iter()
            .flatten()
            .count(),
        3
    );
    let mut world = World::new_commander(3302).unwrap();
    let review = world.review_practice(0).unwrap();
    let actor = world.station_contacts()[0];
    world.request_contact(actor, 0).unwrap();
    let Event::Reviewed(offer) = world
        .request_freight(
            Command::ReviewOffer {
                actor: actor.actor(),
            },
            0,
        )
        .unwrap()
    else {
        panic!("actual offer required")
    };
    world
        .request_freight(
            Command::Confirm {
                review: offer,
                choice: Choice::Accept,
            },
            0,
        )
        .unwrap();
    assert!(world.prepare_practice_confirmation(review, 0).is_err());
    consent(&mut world);
    let ledger = world.practice_ledger_snapshot().unwrap();
    assert_eq!(ledger.active_services, 2);
    assert_eq!(ledger.retained_mission_records, 2);
    assert_eq!(world.mission_snapshot().unwrap().freight.held_tonnes, 8);
    close(&mut world);
    assert_eq!(world.practice_ledger_snapshot().unwrap().active_services, 1);
    assert_eq!(world.mission_snapshot().unwrap().freight.held_tonnes, 8);
    let _continued = cold(&world);
}

#[test]
fn wrong_reception_and_foreign_or_stale_confirmations_preserve_the_current_attempt() {
    let mut world = World::new_commander(3401).unwrap();
    consent(&mut world);
    let review = world.review_practice_closure(0).unwrap();
    let mut foreign = World::new_commander(3402).unwrap();
    consent(&mut foreign);
    let before = foreign.save_bytes().unwrap();
    assert!(foreign.prepare_practice_closure(review, 0).is_err());
    assert_eq!(foreign.save_bytes().unwrap(), before);
    crate::flight::equipment::tests::journey(&mut world, SystemId::RENAS);
    assert_eq!(
        world.review_practice_closure(world.snapshot().tick),
        Err(PracticeError::AvenorBerthRequired)
    );
    let before = world.save_bytes().unwrap();
    assert!(
        world
            .prepare_practice_closure(review, world.snapshot().tick)
            .is_err()
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    if world.snapshot().navigation.unwrap().fuel_units < 68 {
        world.request_refuel(world.snapshot().tick).unwrap();
    }
    crate::flight::equipment::tests::journey(&mut world, SystemId::AVENOR);
    assert_eq!(close(&mut world), PracticeOutcome::Abandoned);
    let _continued = cold(&world);
}

#[test]
fn forged_history_identity_outcome_order_capacity_and_future_receipts_are_atomic_refusals() {
    let mut world = World::new_commander(3501).unwrap();
    consent(&mut world);
    close(&mut world);
    consent(&mut world);
    close(&mut world);
    consent(&mut world);
    corrupt(&mut world, |w| {
        w.practice_history.remove(0);
    });
    corrupt(&mut world, |w| w.practice_history[1].attempt = 0);
    corrupt(&mut world, |w| {
        w.practice_history[0].cancel.as_mut().unwrap().outcome = PracticeOutcome::Failed
    });
    corrupt(&mut world, |w| w.practice_history[0].cancel = None);
    corrupt(&mut world, |w| w.practice_history[0].settlement = None);
    corrupt(&mut world, |w| {
        w.practice_history[0].cancel.as_mut().unwrap().tick = u64::MAX
    });
    corrupt(&mut world, |w| {
        w.practice.as_mut().unwrap().attempt = u8::MAX
    });
    corrupt(&mut world, |w| {
        let old = w.practice_history[0];
        w.practice_history.resize(16, old);
    });
    corrupt(&mut world, |w| {
        w.practice_history[0].cancel.as_mut().unwrap().dock = World::new_commander(3502)
            .unwrap()
            .current_dock_context()
            .unwrap()
    });
}

#[test]
fn epoch_overflow_during_continue_is_an_atomic_refusal_after_valid_history_decode() {
    let mut world = World::new_commander(3601).unwrap();
    consent(&mut world);
    close(&mut world);
    consent(&mut world);
    let valid = world.save_bytes().unwrap();
    let mut encoded = Writer::new();
    world
        .freight_session
        .as_ref()
        .unwrap()
        .encode_save(&mut encoded);
    let mut state = Writer::new();
    world
        .freight_session
        .as_ref()
        .unwrap()
        .state
        .encode_save(&mut state);
    assert_eq!(
        valid
            .windows(encoded.bytes.len())
            .filter(|b| *b == encoded.bytes)
            .count(),
        1
    );
    let start = valid
        .windows(encoded.bytes.len())
        .position(|b| b == encoded.bytes)
        .unwrap();
    let mut bad = valid.clone();
    let offset = start + state.bytes.len();
    bad[offset..offset + 8].copy_from_slice(&u64::MAX.to_le_bytes());
    let end = bad.len() - 8;
    let digest = checksum(&bad[..end]);
    bad[end..].copy_from_slice(&digest.to_le_bytes());
    assert_eq!(
        World::from_save_bytes(&bad).err(),
        Some(SaveError::SequenceExhausted)
    );
    assert_eq!(world.restore_bytes(&bad), Err(SaveError::SequenceExhausted));
    assert_eq!(world.save_bytes().unwrap(), valid);
}

#[test]
fn exact_v8_live_and_completed_states_upgrade_without_restock_or_invented_closure() {
    let mut world = World::new_commander(3701).unwrap();
    consent(&mut world);
    launch(&mut world);
    stop(&mut world);
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let origin = fire(&mut world, target);
    let old = crate::flight::persistence::v8_test_fixture(&world).unwrap();
    let loaded = World::from_save_bytes(&old).unwrap();
    assert_eq!(loaded.missile_snapshot(), world.missile_snapshot());
    assert_eq!(
        loaded.practice_ledger_snapshot().unwrap().attempts[0].outcome,
        None
    );
    let _upgraded = cold(&loaded);
    assert_eq!(origin.item, 3711);
    let mut world = World::new_commander(3702).unwrap();
    consent(&mut world);
    three_hits(&mut world);
    receive(&mut world);
    let trace = world.practice_reception_snapshot().unwrap().trace;
    let old = crate::flight::persistence::v8_test_fixture(&world).unwrap();
    let mut loaded = World::from_save_bytes(&old).unwrap();
    assert_eq!(loaded.practice_reception_snapshot().unwrap().trace, trace);
    assert_eq!(
        loaded.practice_ledger_snapshot().unwrap().attempts[0].outcome,
        Some(PracticeOutcome::Complete)
    );
    consent(&mut loaded);
    let _upgraded = cold(&loaded);
    assert_eq!(loaded.practice_reception_snapshot().unwrap().trace, trace);
}

#[test]
fn closed_prelaunch_attempt_never_reopens_a_provider_without_new_consent() {
    let mut world = World::new_commander(3801).unwrap();
    consent(&mut world);
    close(&mut world);
    launch(&mut world);
    stop(&mut world);
    assert_eq!(
        world.practice_snapshot().unwrap().status,
        super::super::PracticeStatus::Retired
    );
    assert!(world.practice_snapshot().unwrap().beacons.is_none());
    assert_eq!(world.practice_ledger_snapshot().unwrap().active, None);
    assert_eq!(shoot(&mut world).contact, crate::PulseContact::Clear);
    let _loaded = cold(&world);
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 0);
    assert_eq!(world.practice_ledger_snapshot().unwrap().attempts.len(), 1);
}

// The terminal berth differs from the retired range visit. Later physical docks
// make the current-door check insufficient to validate the old terminal receipt.
fn terminal_history_after_later_docks(seed: u64, completed: bool) -> World {
    let mut world = World::new_commander(seed).unwrap();
    consent(&mut world);
    launch(&mut world);
    if completed {
        let beacons = world.practice_snapshot().unwrap().beacons.unwrap();
        assert!(matches!(
            shoot(&mut world).contact,
            crate::PulseContact::PracticeBeacon {
                first_hit: true,
                ..
            }
        ));
        stop(&mut world);
        for i in [0, 2] {
            aim_at(&mut world, beacons[i].pose.position);
            shoot(&mut world);
            step(&mut world, 18);
        }
        assert_eq!(world.practice_snapshot().unwrap().first_hits, 3);
        step(&mut world, 180);
    } else {
        stop(&mut world);
    }
    for system in [SystemId::RENAS, SystemId::AVENOR] {
        world.request_jump(system, world.snapshot().tick).unwrap();
        for _ in 0..crate::navigation::BASIC_SPOOL_TICKS {
            step(&mut world, 1);
        }
    }
    dock(&mut world);
    step(&mut world, 5);
    if completed {
        assert!(world.practice_reception_snapshot().unwrap().ready);
        receive(&mut world);
    } else {
        assert_eq!(close(&mut world), PracticeOutcome::Abandoned);
    }
    world = cold(&world);
    let terminal = world.practice.unwrap().terminal_dock().unwrap();
    consent(&mut world);
    assert_eq!(world.practice.unwrap().consent, terminal);
    world = cold(&world);
    for _ in 0..2 {
        launch(&mut world);
        stop(&mut world);
        dock(&mut world);
        world = cold(&world);
    }
    let old = world.practice_history[0];
    assert_eq!(old.terminal_dock(), Some(terminal));
    assert!(world.current_dock_context().unwrap().sequence() >= terminal.sequence() + 2);
    assert!(world.current_dock_context().unwrap().visit() > terminal.visit());
    assert!(terminal.sequence() + 1 <= terminal.visit());
    assert!(terminal.tick() + 1 <= old.terminal_tick().unwrap());
    assert!(old.retired.unwrap().tick < terminal.tick() - 1);
    world
}

fn reject_single_terminal_dock_field_changes(world: &mut World) {
    let old = world.practice_history[0];
    let terminal = old.terminal_dock().unwrap();
    let valid = world.save_bytes().unwrap();
    let mut record = Writer::new();
    old.encode_full(&mut record);
    let starts: Vec<_> = valid
        .windows(record.bytes.len())
        .enumerate()
        .filter_map(|(i, bytes)| (bytes == record.bytes).then_some(i))
        .collect();
    assert_eq!(starts.len(), 1);
    let mut dock_bytes = Writer::new();
    terminal.encode_save(&mut dock_bytes);
    assert_eq!(dock_bytes.bytes.len(), 52);
    let offsets: Vec<_> = record
        .bytes
        .windows(dock_bytes.bytes.len())
        .enumerate()
        .filter_map(|(i, bytes)| (bytes == dock_bytes.bytes).then_some(i))
        .collect();
    assert_eq!(offsets.len(), 1);
    for (offset, expected) in [
        (27, terminal.visit()),
        (35, terminal.sequence()),
        (43, terminal.tick()),
    ] {
        assert_eq!(
            u64::from_le_bytes(dock_bytes.bytes[offset..offset + 8].try_into().unwrap()),
            expected
        );
    }
    for (field, replacement) in [
        (35, terminal.sequence() + 1),
        (27, terminal.visit() + 1),
        (43, terminal.tick() + 1),
        (43, terminal.tick() - 1),
    ] {
        let mut bad = valid.clone();
        let offset = starts[0] + offsets[0] + field;
        bad[offset..offset + 8].copy_from_slice(&replacement.to_le_bytes());
        let end = bad.len() - 8;
        assert_eq!(&bad[..offset], &valid[..offset]);
        assert_eq!(&bad[offset + 8..end], &valid[offset + 8..end]);
        let digest = checksum(&bad[..end]);
        bad[end..].copy_from_slice(&digest.to_le_bytes());
        refused(world, &bad);
    }
}

#[test]
fn cancelled_terminal_berth_precedes_retry_consent_despite_later_physical_docks() {
    let mut world = terminal_history_after_later_docks(3901, false);
    let before = world.practice_ledger_snapshot().unwrap();
    assert_eq!(before.attempts[0].outcome, Some(PracticeOutcome::Abandoned));
    assert_eq!(before.trace, None);
    reject_single_terminal_dock_field_changes(&mut world);
    assert_eq!(world.practice_ledger_snapshot().unwrap(), before);
    let _continued = cold(&world);
    // A genuinely later berth also permits a fresh consent after closing the retry.
    assert_eq!(close(&mut world), PracticeOutcome::Abandoned);
    let prior_dock = world.practice.unwrap().terminal_dock().unwrap();
    launch(&mut world);
    stop(&mut world);
    dock(&mut world);
    assert!(world.current_dock_context().unwrap().sequence() > prior_dock.sequence());
    assert!(
        world.current_dock_context().unwrap().tick()
            > world.practice.unwrap().terminal_tick().unwrap()
    );
    consent(&mut world);
    let continued = cold(&world);
    assert_eq!(
        continued.practice_ledger_snapshot().unwrap().attempts.len(),
        3
    );
}

#[test]
fn completed_terminal_berth_precedes_retry_consent_and_preserves_first_trace() {
    let mut world = terminal_history_after_later_docks(3902, true);
    let before = world.practice_ledger_snapshot().unwrap();
    assert_eq!(before.attempts[0].outcome, Some(PracticeOutcome::Complete));
    assert!(before.trace.is_some());
    assert_eq!(
        before
            .attempts
            .iter()
            .filter(|p| p.awarded_first_trace)
            .count(),
        1
    );
    reject_single_terminal_dock_field_changes(&mut world);
    assert_eq!(world.practice_ledger_snapshot().unwrap(), before);
    let _continued = cold(&world);
}

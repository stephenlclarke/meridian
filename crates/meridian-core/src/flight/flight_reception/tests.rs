//! Public journeys create authority. Internal mutations below are negative codec/geometry fixtures only.
use super::{
    FlightReceptionAction as Action, FlightReceptionError as Error,
    FlightReceptionOutcome as Outcome, FlightReceptionSnapshot, FlightView, Reading, World,
};
use crate::docking::Vec3;
use crate::flight::missiles::tests::{dock, stop};
use crate::flight::practice::tests::{aim_at, consent, launch};
use crate::freight::g02::{Choice, Command, Event};
use crate::navigation::{DockOrigin, SystemId};
use crate::{FlightInput, FlightMode, StepError};

fn ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|e| panic!("unexpected refusal: {e:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("missing actual state"))
}
fn snapshot(world: &World) -> FlightReceptionSnapshot {
    some(world.flight_reception_snapshot())
}
fn commander(seed: u64) -> World {
    ok(World::new_commander(seed))
}
fn confirm(world: &mut World, action: Action) {
    let tick = world.snapshot().tick;
    let review = ok(world.review_flight_reception(action, tick));
    let prepared = ok(world.prepare_flight_reception(review, tick));
    let bytes = prepared.bytes().to_vec();
    let staged = ok(World::from_save_bytes(&bytes));
    let expected = snapshot(&staged);
    prepared.commit();
    assert_eq!(snapshot(world), expected);
    assert_eq!(ok(world.save_bytes()), bytes);
}
fn cold(world: &World) -> World {
    let bytes = ok(world.save_bytes());
    assert_eq!(
        u16::from_le_bytes([bytes[8], bytes[9]]),
        crate::save::SAVE_VERSION
    );
    let loaded = ok(World::from_save_bytes(&bytes));
    assert_eq!(loaded.snapshot(), world.snapshot());
    assert_eq!(snapshot(&loaded), snapshot(world));
    assert_eq!(loaded.missile_snapshot(), world.missile_snapshot());
    assert_eq!(loaded.pulse_laser_snapshot(), world.pulse_laser_snapshot());
    assert_eq!(loaded.combat_snapshot(), world.combat_snapshot());
    assert_eq!(
        loaded.practice_ledger_snapshot(),
        world.practice_ledger_snapshot()
    );
    assert_eq!(loaded.credits_tenths(), world.credits_tenths());
    loaded
}
fn steps(world: &mut World, count: usize, input: FlightInput) {
    for _ in 0..count {
        ok(world.step(input));
    }
}
fn stationary_facing_station(world: &mut World) {
    launch(world);
    stop(world);
    // Actual source launch and braking stop outside the station, within the lesson's band.
    let p = world.snapshot().position;
    let range = f64::from(p[0])
        .hypot(f64::from(p[1]))
        .hypot(f64::from(p[2]));
    assert!((1000.0..=1500.0).contains(&range));
    aim_at(world, Vec3::new(0.0, 0.0, 0.0));
}
fn measured(world: &World) -> bool {
    snapshot(world)
        .attempts
        .last()
        .is_some_and(|a| a.measured_ticks.is_some())
}
fn approach(world: &mut World) {
    stationary_facing_station(world);
    steps(
        world,
        2,
        FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        },
    );
    assert!(measured(world));
}
fn reject_mutation(world: &mut World, mutate: impl FnOnce(&mut World)) {
    let before = ok(world.save_bytes());
    let before_snapshot = snapshot(world);
    let mut changed = cold(world);
    mutate(&mut changed);
    let bytes = ok(changed.unchecked_test_save());
    assert!(World::from_save_bytes(&bytes).is_err());
    assert!(world.restore_bytes(&bytes).is_err());
    assert_eq!(ok(world.save_bytes()), before);
    assert_eq!(snapshot(world), before_snapshot);
}

#[test]
fn public_launch_closing_swept_return_separate_reception_zero_cash_and_once_trace() {
    let mut world = commander(4101);
    let cash = world.credits_tenths();
    let missiles = world.missile_snapshot();
    let owned = some(world.equipment_snapshot()).owned;
    let first_dock = some(world.current_dock_context());
    confirm(&mut world, Action::Accept);
    assert_eq!(
        world.review_flight_reception(Action::Receive, 0),
        Err(Error::NotReady)
    );
    approach(&mut world);
    let reading = snapshot(&world).attempts[0];
    let [first, last] = some(reading.measured_ticks);
    assert_eq!(last - first, 1);
    let ranges = some(reading.measured_ranges);
    assert!((1000.0..=1500.0).contains(&ranges[0]) && ranges[1] < ranges[0]);
    assert!(some(reading.measured_visit) > first_dock.visit());
    assert_eq!(world.current_dock_context(), None);
    assert!(
        world
            .review_flight_reception(Action::Receive, world.snapshot().tick)
            .is_err()
    );
    assert_eq!(snapshot(&world).trace, None);
    world = cold(&world);
    dock(&mut world);
    let arrival = some(world.current_dock_context());
    assert_eq!(arrival.origin(), DockOrigin::PhysicalEntry);
    assert!(arrival.sequence() > first_dock.sequence() && arrival.tick() > last);
    assert!(snapshot(&world).ready);
    assert_eq!(snapshot(&world).trace, None);
    confirm(&mut world, Action::Receive);
    let trace = some(snapshot(&world).trace);
    assert_eq!(trace.source_id, "l337.training/flight-reception");
    assert_eq!(
        trace.receipt_id,
        "l337.unwritten-route/clue/flight-reception-family-trace"
    );
    assert_eq!(trace.clue_id, "flight-reception-family-trace");
    assert_eq!(trace.dock_sequence, arrival.sequence());
    assert_eq!(
        snapshot(&world).attempts[0].outcome,
        Some(Outcome::Complete)
    );
    assert_eq!(
        world.review_flight_reception(Action::Receive, world.snapshot().tick),
        Err(Error::NoActiveAttempt)
    );
    world = cold(&world);
    confirm(&mut world, Action::Accept);
    approach(&mut world);
    dock(&mut world);
    confirm(&mut world, Action::Receive);
    assert_eq!(snapshot(&world).trace, Some(trace));
    assert_eq!(snapshot(&world).attempts.len(), 2);
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(some(world.equipment_snapshot()).owned, owned);
    assert_eq!(some(world.missile_snapshot()).rounds, some(missiles).rounds);
    assert!(
        some(world.missile_snapshot())
            .projectiles
            .iter()
            .all(Option::is_none)
    );
    let _continued = cold(&world);
}

#[test]
fn dropped_accept_receive_and_cancel_preparations_leave_live_bytes_and_receipts_unchanged() {
    let mut world = commander(4201);
    for action in [Action::Accept, Action::Cancel, Action::Receive] {
        if action == Action::Receive {
            approach(&mut world);
            dock(&mut world);
        }
        let tick = world.snapshot().tick;
        let before = ok(world.save_bytes());
        let before_snapshot = snapshot(&world);
        let review = ok(world.review_flight_reception(action, tick));
        let prepared = ok(world.prepare_flight_reception(review, tick));
        let staged = ok(World::from_save_bytes(prepared.bytes()));
        assert_ne!(snapshot(&staged), before_snapshot);
        // Simulated host writer failure: the complete valid proposal is dropped, never committed.
        drop(prepared);
        assert_eq!(ok(world.save_bytes()), before);
        assert_eq!(snapshot(&world), before_snapshot);
        if action == Action::Accept {
            confirm(&mut world, Action::Accept);
        }
    }
    assert_eq!(snapshot(&world).trace, None);
    confirm(&mut world, Action::Receive);
    assert!(snapshot(&world).trace.is_some());
}

#[test]
fn wrong_early_foreign_stale_paused_and_post_continue_reviews_refuse_atomically() {
    let mut world = commander(4301);
    let mut foreign = commander(4302);
    assert_eq!(
        World::new().review_flight_reception(Action::Accept, 0),
        Err(Error::NoCommander)
    );
    let review = ok(world.review_flight_reception(Action::Accept, 0));
    let before = ok(foreign.save_bytes());
    assert!(foreign.prepare_flight_reception(review, 0).is_err());
    assert_eq!(ok(foreign.save_bytes()), before);
    steps(&mut world, 1, FlightInput::default());
    assert!(world.prepare_flight_reception(review, 0).is_err());
    // A current-tick command may confirm the unchanged review after a normal docked step.
    drop(ok(world.prepare_flight_reception(review, 1)));
    let review = ok(world.review_flight_reception(Action::Accept, 1));
    let bytes = ok(world.save_bytes());
    ok(world.restore_bytes(&bytes));
    assert!(world.prepare_flight_reception(review, 1).is_err());
    ok(world.request_pause(true, 1));
    assert_eq!(
        world.review_flight_reception(Action::Accept, 1),
        Err(Error::Paused)
    );
    assert_eq!(world.step(FlightInput::default()), Err(StepError::Paused));
    ok(world.request_pause(false, 1));
    confirm(&mut world, Action::Accept);
    assert_eq!(
        world.review_flight_reception(Action::Receive, 1),
        Err(Error::NotReady)
    );
    crate::flight::equipment::tests::journey(&mut world, SystemId::RENAS);
    assert_eq!(
        world.review_flight_reception(Action::Receive, world.snapshot().tick),
        Err(Error::BerthRequired)
    );
    assert_eq!(
        world.review_flight_reception(Action::Cancel, world.snapshot().tick),
        Err(Error::BerthRequired)
    );
}

#[test]
fn stationary_front_alignment_view_and_paused_clock_cannot_manufacture_a_reading() {
    let mut world = commander(4401);
    confirm(&mut world, Action::Accept);
    launch(&mut world); // outward motion and wrong aim
    steps(&mut world, 1, FlightInput::default());
    assert!(!measured(&world));
    stop(&mut world);
    aim_at(&mut world, Vec3::new(0.0, 0.0, 0.0));
    steps(&mut world, 10, FlightInput::default());
    assert!(!measured(&world));
    for view in [FlightView::Rear, FlightView::Left, FlightView::Right] {
        let tick = world.snapshot().tick;
        ok(world.request_flight_view(view, tick));
        steps(
            &mut world,
            2,
            FlightInput {
                throttle: 1.0,
                ..FlightInput::default()
            },
        );
        assert!(!measured(&world));
    }
    let tick = world.snapshot().tick;
    ok(world.request_pause(true, tick));
    let before = ok(world.save_bytes());
    assert_eq!(world.step(FlightInput::default()), Err(StepError::Paused));
    assert_eq!(
        world.request_flight_view(FlightView::Front, tick),
        Err(Error::Paused)
    );
    assert_eq!(ok(world.save_bytes()), before);
    ok(world.request_pause(false, tick));
    ok(world.request_flight_view(FlightView::Front, tick));
    steps(&mut world, 1, FlightInput::default());
    assert!(measured(&world));
    let _continued = cold(&world);
}

#[test]
fn reserved_fit_revision_blocks_measurement_without_refund_reset_or_losing_old_items() {
    let mut world = crate::flight::equipment::tests::earned_world(4501);
    ok(world.request_equipment(
        crate::equipment::Command::PurchaseCargoBay,
        world.snapshot().tick,
    ));
    let item = some(some(world.equipment_snapshot()).owned).id;
    confirm(&mut world, Action::Accept);
    let cash = world.credits_tenths();
    let missiles = world.missile_snapshot();
    ok(world.request_equipment(
        crate::equipment::Command::UnfitCargoBay(item),
        world.snapshot().tick,
    ));
    stationary_facing_station(&mut world);
    steps(
        &mut world,
        2,
        FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        },
    );
    assert!(!measured(&world));
    dock(&mut world);
    assert_eq!(
        world.review_flight_reception(Action::Receive, world.snapshot().tick),
        Err(Error::NotReady)
    );
    confirm(&mut world, Action::Cancel);
    assert_eq!(
        snapshot(&world).attempts[0].outcome,
        Some(Outcome::Abandoned)
    );
    assert_eq!(snapshot(&world).trace, None);
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(some(world.missile_snapshot()).rounds, some(missiles).rounds);
    assert_eq!(some(some(world.equipment_snapshot()).owned).id, item);
    confirm(&mut world, Action::Accept);
    approach(&mut world);
    dock(&mut world);
    let reading = snapshot(&world).attempts[1].measured_ticks;
    ok(world.request_equipment(
        crate::equipment::Command::FitCargoBay(item),
        world.snapshot().tick,
    ));
    assert_eq!(snapshot(&world).attempts[1].measured_ticks, reading);
    assert!(snapshot(&world).ready);
    confirm(&mut world, Action::Receive);
    assert!(snapshot(&world).trace.is_some());
    let _continued = cold(&world);
}

#[test]
fn cancellation_retry_sixteen_retained_berth_slots_and_three_real_shared_owners() {
    let mut world = commander(4601);
    for i in 0..16 {
        confirm(&mut world, Action::Accept);
        assert_eq!(some(snapshot(&world).active).ordinal(), i);
        confirm(&mut world, Action::Cancel);
    }
    assert_eq!(
        world.review_flight_reception(Action::Accept, 0),
        Err(Error::BerthCapacity)
    );
    assert_eq!(snapshot(&world).attempts.len(), 16);
    assert_eq!(snapshot(&world).retained_mission_records, 16);
    assert_eq!(snapshot(&world).active_services, 0);
    assert_eq!(snapshot(&world).trace, None);
    assert_eq!(world.credits_tenths(), Some(7200));
    let _continued = cold(&world);
    crate::flight::equipment::tests::journey(&mut world, SystemId::RENAS);
    confirm(&mut world, Action::Accept);
    assert_eq!(snapshot(&world).attempts.len(), 17);
    assert_eq!(
        some(snapshot(&world).attempts.last()).issuing_dock.system(),
        SystemId::RENAS
    );
    let _continued = cold(&world);
    let mut world = commander(4602);
    consent(&mut world);
    let npc = world.station_contacts()[0];
    ok(world.request_contact(npc, 0));
    let Event::Reviewed(review) =
        ok(world.request_freight(Command::ReviewOffer { actor: npc.actor() }, 0))
    else {
        panic!("actual freight review required")
    };
    ok(world.request_freight(
        Command::Confirm {
            review,
            choice: Choice::Accept,
        },
        0,
    ));
    confirm(&mut world, Action::Accept);
    assert_eq!(snapshot(&world).retained_mission_records, 3);
    assert_eq!(snapshot(&world).active_services, 3);
    assert_eq!(some(world.practice_ledger_snapshot()).active_services, 3);
    assert_eq!(
        some(world.practice_ledger_snapshot()).retained_mission_records,
        3
    );
    assert_eq!(some(world.mission_snapshot()).freight.held_tonnes, 8);
    let _continued = cold(&world);
    confirm(&mut world, Action::Cancel);
    assert_eq!(snapshot(&world).active_services, 2);
    assert_eq!(some(world.practice_ledger_snapshot()).active_services, 2);
    assert_eq!(some(world.mission_snapshot()).freight.held_tonnes, 8);
}

#[test]
fn checked_reading_and_terminal_chronology_tamper_never_replace_live_save() {
    let mut world = commander(4701);
    confirm(&mut world, Action::Accept);
    approach(&mut world);
    dock(&mut world);
    confirm(&mut world, Action::Receive);
    reject_mutation(&mut world, |w| w.flight_reception.trace = None);
    reject_mutation(&mut world, |w| w.flight_reception.trace = Some(127));
    reject_mutation(&mut world, |w| {
        w.flight_reception.attempts[0].fit.inventory += 1
    });
    reject_mutation(&mut world, |w| {
        w.flight_reception.attempts[0].fit.items[0] = None
    });
    reject_mutation(&mut world, |w| {
        w.flight_reception.attempts[0].fit.equipment_revision = u64::MAX
    });
    reject_mutation(&mut world, |w| {
        w.flight_reception.attempts[0].accepted_tick = u64::MAX
    });
    reject_mutation(&mut world, |w| {
        w.flight_reception.attempts[0].reading = None
    });
    reject_mutation(&mut world, |w| {
        some(w.flight_reception.attempts[0].reading.as_mut())
            .last
            .physical_tick += 1
    });
    reject_mutation(&mut world, |w| {
        some(w.flight_reception.attempts[0].reading.as_mut())
            .last
            .flight
            .tick += 6
    });
    reject_mutation(&mut world, |w| {
        some(w.flight_reception.attempts[0].reading.as_mut())
            .last
            .visit += 1
    });
    reject_mutation(&mut world, |w| {
        some(w.flight_reception.attempts[0].reading.as_mut())
            .first
            .flight
            .speed = 0.0
    });
    reject_mutation(&mut world, |w| {
        some(w.flight_reception.attempts[0].terminal.as_mut()).dock = None
    });
    reject_mutation(&mut world, |w| {
        some(w.flight_reception.attempts[0].terminal.as_mut()).tick = u64::MAX
    });
    reject_mutation(&mut world, |w| {
        some(w.flight_reception.attempts[0].terminal.as_mut()).dock =
            Some(w.flight_reception.attempts[0].dock)
    });
    confirm(&mut world, Action::Accept);
    steps(&mut world, 10, FlightInput::default());
    reject_mutation(&mut world, |w| {
        let too_late = w.flight_reception.attempts[1].accepted_tick + 1;
        some(w.flight_reception.attempts[0].terminal.as_mut()).tick = too_late;
    });
    reject_mutation(&mut world, |w| {
        w.flight_reception.attempts[1].accepted_tick = 0
    });
    reject_mutation(&mut world, |w| {
        let last = some(w.flight_reception.attempts.last()).to_owned();
        w.flight_reception.attempts.resize(129, last);
    });
}

#[test]
fn current_continue_keeps_cancelled_gunnery_history_and_refuses_v9() {
    let mut world = commander(4801);
    consent(&mut world);
    let review = ok(world.review_practice_closure(0));
    ok(world.prepare_practice_closure(review, 0)).commit();
    consent(&mut world);
    let before = some(world.practice_ledger_snapshot());
    let bytes = ok(crate::flight::persistence::v9_test_fixture(&world));
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 9);
    assert_eq!(
        World::from_save_bytes(&bytes).unwrap_err(),
        crate::save::SaveError::UnsupportedVersion
    );
    let loaded = ok(World::from_save_bytes(&ok(world.save_bytes())));
    assert_eq!(some(loaded.practice_ledger_snapshot()), before);
    assert!(snapshot(&loaded).attempts.is_empty());
    assert_eq!(snapshot(&loaded).trace, None);
    assert_eq!(loaded.missile_snapshot(), world.missile_snapshot());
    let _continued = cold(&loaded);
    assert!(
        crate::flight::persistence::v9_test_fixture(&{
            confirm(&mut world, Action::Accept);
            world
        })
        .is_err()
    );
}

#[test]
fn fixed_step_geometry_bounds_reject_stationary_wrong_aim_range_and_discontinuous_clocks() {
    let mut world = commander(4901);
    confirm(&mut world, Action::Accept);
    approach(&mut world);
    let reading = some(world.flight_reception.attempts[0].reading);
    assert!(Reading::qualifies(reading.first, reading.last));
    assert!(
        f64::from(crate::FIXED_DELTA_SECONDS) > 0.0 && f64::from(crate::FIXED_DELTA_SECONDS) <= 0.1
    );
    for axis in [0, 1, 2] {
        let mut changed = reading.last;
        changed.flight.position[axis] += 5.0;
        assert!(!Reading::qualifies(reading.first, changed));
    }
    for range in [999.9, 1500.1] {
        let mut sample = reading.first;
        sample.flight.position = [0.0, 0.0, range];
        assert!(!sample.qualifies());
    }
    for (degrees, qualifies) in [(1.99_f64, true), (2.01_f64, false)] {
        let half = (core::f64::consts::PI + degrees.to_radians()) / 2.0;
        let mut angled = reading.first;
        angled.flight.orientation = [0.0, half.sin() as f32, 0.0, half.cos() as f32];
        assert_eq!(angled.qualifies(), qualifies);
    }
    let mut sample = reading.first;
    sample.flight.orientation = [0.0, 0.0, 0.0, 1.0];
    assert!(!sample.qualifies());
    sample.flight = reading.first.flight;
    sample.flight.speed = 0.0;
    assert!(!sample.qualifies());
    let mut last = reading.last;
    last.flight = reading.first.flight;
    assert!(!Reading::qualifies(reading.first, last));
}

#[test]
fn actual_missile_mesh_obstructs_sight_and_open_station_doorway_stays_clear() {
    let mut world = commander(5001);
    consent(&mut world);
    launch(&mut world);
    stop(&mut world);
    let target = some(some(world.practice_snapshot()).beacons)[1].id;
    let origin = crate::flight::missiles::tests::fire(&mut world, target);
    let p = some(some(world.missile_snapshot()).projectiles[0]);
    // Narrow-phase fixture only: the body is the actual publicly emitted missile. Move only
    // the test ray5 units off its open centre to intersect its authored native faces.
    let mut sample = some(world.flight_reception_sample(origin.owner.visit));
    assert!(world.flight_sample_bodies_valid(sample, false));
    let mut missing = sample;
    missing.bodies.fill(None);
    assert!(!world.flight_sample_bodies_valid(missing, false));
    sample.flight.position = [0.0, 5.0, (p.position.z - 100.0) as f32];
    sample.flight.orientation = [0.0, 0.0, 0.0, 1.0];
    assert!(!world.flight_sample_sight_clear(sample));
    let mut clear = sample;
    clear.bodies.fill(None);
    assert!(world.flight_sample_sight_clear(clear));
    // A real public approach through the native station opening requires no fabricated wall.
    let mut world = commander(5002);
    confirm(&mut world, Action::Accept);
    stationary_facing_station(&mut world);
    steps(
        &mut world,
        2,
        FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        },
    );
    assert!(measured(&world));
}

#[test]
fn failure_hook_only_closes_active_attempt_for_actual_hull_or_fatal_condition() {
    let mut world = commander(5101);
    confirm(&mut world, Action::Accept);
    let attempt = world.flight_reception.attempts[0];
    assert!(!attempt.must_fail(world.player_ship_id(), false));
    assert!(attempt.must_fail(commander(5102).player_ship_id(), false));
    assert!(attempt.must_fail(world.player_ship_id(), true));
    world.observe_flight_reception(None);
    assert_eq!(snapshot(&world).attempts[0].outcome, None);
    // Negative retained-state fixture: a forged failed tag without a fatal/hull cause refuses.
    reject_mutation(&mut world, |w| {
        w.flight_reception.attempts[0].terminal = Some(super::Terminal {
            outcome: Outcome::Failed,
            tick: 0,
            dock: None,
        })
    });
    confirm(&mut world, Action::Cancel);
    world.observe_flight_reception(None);
    assert_eq!(
        snapshot(&world).attempts[0].outcome,
        Some(Outcome::Abandoned)
    );
    assert_eq!(world.snapshot().mode, FlightMode::Docked);
}

#[test]
fn fatal_reducer_boundary_closes_without_clue_or_financial_settlement() {
    let mut world = commander(5201);
    confirm(&mut world, Action::Accept);
    launch(&mut world);
    let cash = world.credits_tenths();
    let rounds = some(world.missile_snapshot()).rounds;
    // Internal reducer boundary only. This does not claim a witnessed native/player hit
    // or create accepted damage-save authority; existing ordnance validators still refuse it.
    world.pulse.damage_energy(u16::MAX);
    world.observe_flight_reception(None);
    assert_eq!(snapshot(&world).attempts[0].outcome, Some(Outcome::Failed));
    assert_eq!(snapshot(&world).attempts[0].terminal_dock, None);
    assert_eq!(snapshot(&world).trace, None);
    assert_eq!(snapshot(&world).active, None);
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(some(world.missile_snapshot()).rounds, rounds);
    assert_eq!(
        world.step(FlightInput::default()),
        Err(StepError::InvalidMode)
    );
    assert!(world.save_bytes().is_err());
}

#[test]
fn original_gunnery_clue_and_owned_resources_survive_independent_flight_trace() {
    let mut world = commander(5301);
    consent(&mut world);
    launch(&mut world);
    let beacons = some(some(world.practice_snapshot()).beacons);
    crate::flight::practice::tests::shoot(&mut world);
    stop(&mut world);
    for i in [0, 2] {
        aim_at(&mut world, beacons[i].pose.position);
        crate::flight::practice::tests::shoot(&mut world);
        steps(&mut world, 18, FlightInput::default());
    }
    assert_eq!(some(world.practice_snapshot()).first_hits, 3);
    steps(&mut world, 180, FlightInput::default());
    dock(&mut world);
    let tick = world.snapshot().tick;
    let reception = ok(world.review_practice_reception(tick));
    ok(world.prepare_practice_reception(reception, tick)).commit();
    let original = some(some(world.practice_reception_snapshot()).trace);
    let ledger = some(world.practice_ledger_snapshot());
    let cash = world.credits_tenths();
    let rounds = some(world.missile_snapshot()).rounds;
    confirm(&mut world, Action::Accept);
    approach(&mut world);
    dock(&mut world);
    confirm(&mut world, Action::Receive);
    assert_eq!(
        some(world.practice_reception_snapshot()).trace,
        Some(original)
    );
    assert_eq!(
        some(world.practice_ledger_snapshot()).attempts,
        ledger.attempts
    );
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(some(world.missile_snapshot()).rounds, rounds);
    assert_ne!(some(snapshot(&world).trace).receipt_id, original.receipt_id);
    let _continued = cold(&world);
}

#[test]
fn accept_receive_and_cancel_reviews_survive_ordinary_docked_ticks_and_stamp_actual_commit() {
    for (i, action) in [Action::Accept, Action::Receive, Action::Cancel]
        .into_iter()
        .enumerate()
    {
        let mut world = commander(5401 + i as u64);
        if action != Action::Accept {
            confirm(&mut world, Action::Accept);
        }
        if action == Action::Receive {
            approach(&mut world);
            dock(&mut world);
        }
        let reviewed_tick = world.snapshot().tick;
        let review = ok(world.review_flight_reception(action, reviewed_tick));
        let before = snapshot(&world);
        let original_bytes = ok(world.save_bytes());
        steps(&mut world, 7, FlightInput::default());
        let current_tick = world.snapshot().tick;
        assert_eq!(current_tick, reviewed_tick + 7);
        assert_eq!(snapshot(&world), before);
        assert_ne!(ok(world.save_bytes()), original_bytes);
        let delayed_bytes = ok(world.save_bytes());
        assert!(
            world
                .prepare_flight_reception(review, reviewed_tick)
                .is_err()
        );
        // Private review-field corruption is a negative fixture, never public consent.
        let future = super::FlightReceptionReview {
            tick: current_tick + 1,
            ..review
        };
        assert!(
            world
                .prepare_flight_reception(future, current_tick)
                .is_err()
        );
        assert_eq!(ok(world.save_bytes()), delayed_bytes);
        let prepared = ok(world.prepare_flight_reception(review, current_tick));
        let staged = ok(World::from_save_bytes(prepared.bytes()));
        let after = snapshot(&staged);
        prepared.commit();
        assert_eq!(snapshot(&world), after);
        let attempt = some(after.attempts.last());
        if action == Action::Accept {
            assert_eq!(attempt.accepted_tick, current_tick);
        } else {
            assert_eq!(attempt.terminal_tick, Some(current_tick));
        }
        let _continued = cold(&world);
    }
}

fn delayed_material_refusal(world: &mut World, review: super::FlightReceptionReview) {
    steps(world, 7, FlightInput::default());
    let before = ok(world.save_bytes());
    let snapshot_before = snapshot(world);
    let tick = world.snapshot().tick;
    assert!(matches!(
        world.prepare_flight_reception(review, tick),
        Err(Error::StaleReview)
    ));
    assert_eq!(ok(world.save_bytes()), before);
    assert_eq!(snapshot(world), snapshot_before);
}

#[test]
fn delayed_reviews_still_bind_shared_records_money_markets_view_pause_and_actual_dock() {
    let mut world = commander(5501);
    let review = ok(world.review_flight_reception(Action::Accept, 0));
    consent(&mut world); // A real T02 reservation changes shared occupancy without changing tick.
    delayed_material_refusal(&mut world, review);

    let mut world = commander(5502);
    let review = ok(world.review_flight_reception(Action::Accept, 0));
    ok(world.request_station_service(
        crate::station_services::Command::BuyCommodity(
            crate::station_services::CommodityId::Minerals,
        ),
        0,
    ));
    delayed_material_refusal(&mut world, review);

    let mut world = commander(5503);
    let review = ok(world.review_flight_reception(Action::Accept, 0));
    ok(world.request_flight_view(FlightView::Rear, 0));
    delayed_material_refusal(&mut world, review);

    let mut world = commander(5504);
    let review = ok(world.review_flight_reception(Action::Accept, 0));
    ok(world.request_pause(true, 0));
    ok(world.request_pause(false, 0));
    delayed_material_refusal(&mut world, review);

    let mut world = commander(5505);
    let review = ok(world.review_flight_reception(Action::Accept, 0));
    let dock_before = some(world.current_dock_context());
    stationary_facing_station(&mut world);
    dock(&mut world);
    assert!(some(world.current_dock_context()).sequence() > dock_before.sequence());
    delayed_material_refusal(&mut world, review);
}

#[test]
fn paid_fit_change_invalidates_delayed_review_even_after_actual_fit_is_restored() {
    let mut world = crate::flight::equipment::tests::earned_world(5601);
    let tick = world.snapshot().tick;
    ok(world.request_equipment(crate::equipment::Command::PurchaseCargoBay, tick));
    let item = some(some(world.equipment_snapshot()).owned).id;
    let review = ok(world.review_flight_reception(Action::Accept, tick));
    ok(world.request_equipment(crate::equipment::Command::UnfitCargoBay(item), tick));
    ok(world.request_equipment(crate::equipment::Command::FitCargoBay(item), tick));
    assert!(some(some(world.equipment_snapshot()).owned).fitted);
    delayed_material_refusal(&mut world, review);
    confirm(&mut world, Action::Accept);
}

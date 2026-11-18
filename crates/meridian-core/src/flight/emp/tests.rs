//! Real commerce/flight producers and separate numerical-kernel fixtures. Owner execution pending.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::equipment::tests::earned_world;
use crate::flight::practice::tests::launch;
use crate::navigation::SystemId;
use crate::{FlightInput, XyzzyAction};
fn equipment(w: &mut World, action: EmpEquipmentAction) {
    let tick = w.snapshot().tick;
    let review = w.review_emp_equipment(action, tick).unwrap();
    let stage = w.prepare_emp_equipment(review, tick).unwrap();
    let candidate = World::from_save_bytes(stage.bytes()).unwrap();
    stage.commit();
    assert_eq!(w.emp_snapshot(), candidate.emp_snapshot());
    assert_eq!(w.credits_tenths(), candidate.credits_tenths());
}
fn paid() -> World {
    let mut w = earned_world(16101);
    equipment(&mut w, EmpEquipmentAction::Purchase);
    w
}
fn secret(w: &mut World) {
    let tick = w.snapshot().tick;
    let review = w.review_xyzzy(XyzzyAction::Enter, tick).unwrap();
    w.prepare_xyzzy(review, tick).unwrap().commit();
}
fn steps(w: &mut World, count: usize) {
    for _ in 0..count {
        w.step(FlightInput::default()).unwrap();
    }
}
fn cold(w: &World) -> World {
    let bytes = w.save_bytes().unwrap();
    let loaded = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(loaded.emp_snapshot(), w.emp_snapshot());
    assert_eq!(loaded.traffic_snapshot(), w.traffic_snapshot());
    assert_eq!(
        loaded.flight_resources_snapshot(),
        w.flight_resources_snapshot()
    );
    loaded
}
fn poison_refuses(w: &World, live: &mut World) {
    let before = live.save_bytes().unwrap();
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
}
#[test]
fn paid_emp_is_one_empty_owned_item_and_failed_writer_preserves_all_state() {
    let mut w = earned_world(16101);
    let before = w.save_bytes().unwrap();
    let offer = w
        .emp_equipment_offers()
        .into_iter()
        .find(|o| o.action == EmpEquipmentAction::Purchase)
        .unwrap();
    assert_eq!(
        (offer.price_tenths, offer.required_technology, offer.refusal),
        (30000, 9, None)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let cash = w.credits_tenths().unwrap();
    let tick = w.snapshot().tick;
    let review = w
        .review_emp_equipment(EmpEquipmentAction::Purchase, tick)
        .unwrap();
    let stage = w.prepare_emp_equipment(review, tick).unwrap();
    assert_eq!(
        stage.publish_with(|_| Err::<(), _>("disk failure")),
        Err("disk failure")
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    assert!(w.emp_snapshot().is_none());
    equipment(&mut w, EmpEquipmentAction::Purchase);
    let item = w.owned_emp().unwrap();
    assert_eq!(item.provenance, EmpProvenance::Paid);
    assert_eq!(item.paid_tenths, 30000);
    assert_eq!(w.credits_tenths(), Some(cash - 30000));
    assert_eq!(w.emp_snapshot().unwrap().bank_energy, 0.0);
    assert_eq!(
        w.review_emp_equipment(EmpEquipmentAction::Purchase, w.snapshot().tick),
        Err(EmpError::AlreadyOwned)
    );
    cold(&w);
}
#[test]
fn docked_reads_pause_and_stored_flight_never_fill_a_bank() {
    let mut w = paid();
    let item = w.owned_emp().unwrap().item;
    steps(&mut w, 500);
    for _ in 0..100 {
        assert_eq!(w.emp_snapshot().unwrap().bank_energy, 0.0);
    }
    assert_eq!(w.emp.meter.generated, 0);
    w.request_pause(true, w.snapshot().tick).unwrap();
    let frozen = w.emp_snapshot();
    assert_eq!(
        w.step(FlightInput::default()),
        Err(crate::StepError::Paused)
    );
    assert_eq!(w.emp_snapshot(), frozen);
    w.request_pause(false, w.snapshot().tick).unwrap();
    equipment(&mut w, EmpEquipmentAction::Store { item });
    launch(&mut w);
    steps(&mut w, 1300);
    assert_eq!(w.emp.meter, meter::Meter::default());
    assert!(!w.emp.main_present);
    cold(&w);
}
#[test]
fn stored_clock_gap_requires_a_real_refit_interval_and_capacity_upgrade_does_not_fill() {
    let mut w = paid();
    let item = w.owned_emp().unwrap().item;
    launch(&mut w);
    steps(&mut w, 1300);
    assert_eq!(w.emp_snapshot().unwrap().bank_energy, 8.0);
    crate::flight::missiles::tests::dock(&mut w);
    let retained = w.emp.meter.bank;
    equipment(
        &mut w,
        EmpEquipmentAction::Upgrade {
            item,
            axis: EmpAxis::Capacity,
        },
    );
    assert_eq!(w.emp.meter.bank, retained);
    equipment(&mut w, EmpEquipmentAction::Store { item });
    launch(&mut w);
    steps(&mut w, 120);
    assert_eq!(w.emp.meter.bank, retained);
    crate::flight::missiles::tests::dock(&mut w);
    equipment(&mut w, EmpEquipmentAction::Fit { item });
    let start = w.emp.meter.bank;
    let stale = w.emp.bank_clock;
    w.request_launch(w.snapshot().tick).unwrap();
    assert!(stale < w.emp.meter_clock);
    steps(&mut w, 1);
    assert_eq!(w.emp.meter.bank, start);
    steps(&mut w, 1);
    assert_eq!(w.emp.meter.bank, start + 20);
    cold(&w);
}
#[test]
fn secret_grant_reuses_paid_emp_and_secret_only_base_cannot_buy_grades() {
    let mut w = paid();
    let old = w.owned_emp().unwrap();
    secret(&mut w);
    assert_eq!(w.owned_emp(), Some(old));
    let mut granted = World::new_commander(16102).unwrap();
    secret(&mut granted);
    let item = granted.owned_emp().unwrap();
    assert_eq!(item.provenance, EmpProvenance::Xyzzy);
    assert_eq!(item.paid_tenths, 0);
    assert_eq!(granted.emp.meter.bank, 0);
    assert_eq!(
        granted.review_emp_equipment(
            EmpEquipmentAction::Upgrade {
                item: item.item,
                axis: EmpAxis::Power
            },
            granted.snapshot().tick
        ),
        Err(EmpError::PaidBaseRequired)
    );
    cold(&granted);
}
#[test]
fn altered_bank_and_coordinated_transfer_totals_refuse_without_replacing_live_world() {
    let mut w = paid();
    launch(&mut w);
    steps(&mut w, 100);
    let good = w.save_bytes().unwrap();
    let mut bad = World::from_save_bytes(&good).unwrap();
    bad.emp.meter.bank += 4;
    bad.emp.meter.bank_gained += 4;
    bad.emp.meter.main_debited += 5;
    bad.emp.meter.generated += 5;
    assert!(
        bad.emp.meter.validate().is_ok(),
        "aggregate conservation alone must be insufficient"
    );
    poison_refuses(&bad, &mut w);
}
#[test]
fn altered_paid_grade_receipt_and_foreign_hardware_refuse_current_save() {
    let mut w = paid();
    let good = w.save_bytes().unwrap();
    let mut bad = World::from_save_bytes(&good).unwrap();
    Arc::make_mut(&mut bad.emp.equipment)[0].secret = true;
    poison_refuses(&bad, &mut w);
    let mut bad = World::from_save_bytes(&good).unwrap();
    bad.emp.meter_clock += 1;
    poison_refuses(&bad, &mut w);
}
#[test]
fn initial_unfunded_and_stale_emp_commands_are_atomic() {
    let mut w = World::new_commander(16103).unwrap();
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.review_emp_equipment(EmpEquipmentAction::Purchase, 0),
        Err(EmpError::InsufficientFunds)
    );
    assert_eq!(w.request_emp_trigger(true, 1), Err(EmpError::StaleReview));
    assert_eq!(
        w.request_emp_trigger(true, 0),
        Err(EmpError::FlightRequired)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
}
#[test]
fn folded_meter_matches_independent_scalar_posting_with_clipping_and_disable_recovery() {
    // Numerical fixtures are deliberately separate from actual property/World acceptance.
    for recharge_grade in 0..=3 {
        for energy_stage in 0..=3 {
            let grades = EmpGrades {
                power: 0,
                capacity: 3,
                recharge: recharge_grade,
            };
            let rate = 210 * (1 + energy_stage);
            for ticks in [1, 2, 1199, 1200, 1201, 1800, 1801, 5000] {
                let mut scalar = meter::Meter::default();
                let mut folded = scalar;
                for clock in 1..=ticks {
                    scalar.post_main(clock, rate).unwrap();
                    scalar.transfer(grades).unwrap();
                }
                folded.advance(0, ticks, rate, Some(grades)).unwrap();
                assert_eq!(folded, scalar);
                let drained = scalar.main;
                scalar.drain(drained, ticks).unwrap();
                folded.drain(drained, ticks).unwrap();
                for clock in ticks + 1..=ticks + 1801 {
                    scalar.post_main(clock, rate).unwrap();
                    scalar.transfer(grades).unwrap();
                }
                folded
                    .advance(ticks, ticks + 1801, rate, Some(grades))
                    .unwrap();
                assert_eq!(folded, scalar);
            }
        }
    }
}
#[test]
fn source_grade_prices_profile_and_strict_thirty_second_recovery_are_distinct_from_ecm() {
    for axis in [EmpAxis::Power, EmpAxis::Capacity, EmpAxis::Recharge] {
        let mut grades = EmpGrades::default();
        for level in 0..3 {
            let unit = if axis == EmpAxis::Power { 30000 } else { 15000 };
            assert_eq!(
                grades.upgrade_offer(axis).unwrap(),
                (unit << level, [10, 12, 14][level])
            );
            grades = grades.advancing(axis).unwrap();
        }
        assert_eq!(grades.advancing(axis), Err(EmpError::MaximumGrade));
    }
    let profile = EmpGrades {
        power: 3,
        capacity: 0,
        recharge: 0,
    }
    .profile(3)
    .unwrap();
    assert_eq!(profile.power, 640 * 3000);
    assert_eq!(profile.range, 7500.0);
    let mut pool = meter::Meter::default();
    pool.post_main(1, 210).unwrap();
    pool.drain(210, 1).unwrap();
    pool.post_main(1801, 210).unwrap();
    assert_eq!(pool.main, 0);
    assert_eq!(pool.disabled_at, Some(1));
    pool.post_main(1802, 210).unwrap();
    assert_eq!(pool.main, 210);
    assert_eq!(pool.disabled_at, None);
}

fn aim_input(w: &World, target: crate::traffic::NpcId) -> FlightInput {
    let p = w.snapshot();
    let body = w
        .traffic_snapshot()
        .unwrap()
        .actors
        .into_iter()
        .find(|a| a.id == target)
        .unwrap();
    let pose = super::super::flight_pose(p.position, p.orientation).unwrap();
    let desired = (body.pose.position.x - f64::from(p.position[0]))
        .atan2(body.pose.position.z - f64::from(p.position[2]));
    let yaw = pose.orientation.z.x.atan2(pose.orientation.z.z);
    let error = (desired - yaw).sin().atan2((desired - yaw).cos());
    FlightInput {
        yaw: (error / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32,
        ..FlightInput::default()
    }
}
fn defender(mut w: World) -> (World, crate::traffic::NpcId) {
    w.request_station_service(crate::station_services::Command::BuyFood, w.snapshot().tick)
        .unwrap();
    launch(&mut w);
    crate::flight::missiles::tests::stop(&mut w);
    crate::flight::missiles::tests::fly_axis(&mut w, 0, -600.0, 10.0);
    for _ in 0..24000 {
        w.step(FlightInput::default()).unwrap();
        if let Some(hit) = w.traffic_snapshot().unwrap().last_player_hit
            && hit.visit == w.combat_identity().unwrap().visit
            && hit.world_tick == w.snapshot().tick
        {
            return (w, hit.attacker);
        }
    }
    panic!("no actual recent native attacker; no substitute aggression receipt permitted");
}
fn start_charge(w: &mut World, target: crate::traffic::NpcId) {
    for _ in 0..1200 {
        match w.request_emp_trigger(true, w.snapshot().tick) {
            Ok(()) => {
                assert_eq!(w.emp.charge.as_ref().unwrap().target, target);
                return;
            }
            Err(
                EmpError::NoTarget
                | EmpError::NoRecentAggressor
                | EmpError::InvalidContact
                | EmpError::Resources,
            ) => {}
            Err(error) => panic!("unexpected start refusal {error:?}"),
        }
        w.step(aim_input(w, target)).unwrap();
    }
    panic!("real pilot failed to aim the EMP at its actual recent attacker");
}
fn hold(w: &mut World, target: crate::traffic::NpcId, ticks: usize) {
    for _ in 0..ticks {
        w.request_emp_trigger(true, w.snapshot().tick).unwrap();
        w.step(aim_input(w, target)).unwrap();
    }
}
#[test]
fn actual_secret_base_release_spends_once_drains_shield_and_preserves_hull_cargo_and_cash_cold() {
    let mut w = World::new_commander(12001).unwrap();
    secret(&mut w);
    let (mut w, target) = defender(w);
    start_charge(&mut w, target);
    let start = w.emp.charge.as_ref().unwrap().start.physical_tick;
    hold(&mut w, target, 599);
    assert!(w.emp.pulses.is_empty());
    let before = w.traffic.emp_resources(target).unwrap();
    let cash = w.credits_tenths();
    let cargo = w.commodity_quantity(crate::station_services::CommodityId::Food);
    let emissions = w.combat.emissions;
    let spent = w.emp.meter.bank_spent;
    hold(&mut w, target, 1);
    assert_eq!(w.emp.pulses.len(), 1);
    assert_eq!(w.combat.physical_tick() - start, 600);
    assert_eq!(w.emp.meter.bank_spent - spent, 8 * 3000);
    assert_eq!(w.combat.emissions, emissions);
    assert!(
        w.emp.pulses[0]
            .contacts
            .iter()
            .any(|c| c.victim == scene::Victim::Player),
        "actual target-centred splash must include the nearby firing hull"
    );
    let after = w.traffic.emp_resources(target).unwrap();
    assert_eq!(after.0, before.0, "EMP must not be fatal hull damage");
    assert!(after.1 < before.1 || after.2 < before.2);
    assert!(after.3.is_some());
    assert_eq!(w.credits_tenths(), cash);
    assert_eq!(
        w.commodity_quantity(crate::station_services::CommodityId::Food),
        cargo
    );
    hold(&mut w, target, 100);
    assert_eq!(w.emp.pulses.len(), 1, "held input must not rearm");
    cold(&w);
    let mut bad = cold(&w);
    Arc::make_mut(&mut bad.emp.pulses)[0].origin.item += 1;
    poison_refuses(&bad, &mut w);
    let mut bad = cold(&w);
    Arc::make_mut(&mut bad.emp.pulses)[0].contacts[0].power_hundredths += 1;
    poison_refuses(&bad, &mut w);
}
#[test]
fn cold_continue_does_not_restore_held_input_and_early_release_has_no_discharge() {
    let mut w = World::new_commander(12001).unwrap();
    secret(&mut w);
    let (mut w, target) = defender(w);
    start_charge(&mut w, target);
    hold(&mut w, target, 120);
    let mut loaded = cold(&w);
    loaded.step(aim_input(&loaded, target)).unwrap();
    assert_eq!(
        loaded.emp.charge.as_ref().unwrap().phase,
        EmpChargePhase::Cancelled
    );
    assert_eq!(loaded.emp.meter.bank_spent, 0);
    assert!(loaded.emp.pulses.is_empty());
    cold(&loaded);
    w.request_emp_trigger(false, w.snapshot().tick).unwrap();
    w.step(aim_input(&w, target)).unwrap();
    assert_eq!(
        w.emp.charge.as_ref().unwrap().phase,
        EmpChargePhase::Cancelled
    );
    assert_eq!(w.emp.meter.bank_spent, 0);
}
#[test]
fn actual_paid_upgraded_emp_disables_native_attacker_then_recovers_without_loot_or_hull_replacement()
 {
    let mut w = paid();
    // The base is bought with earned market cash. Explicit XYZZY supplies later grade funds and
    // Elite energy; this is not a claim of a wholly ordinary high-grade acquisition journey.
    secret(&mut w);
    let item = w.owned_emp().unwrap().item;
    for axis in [EmpAxis::Power, EmpAxis::Capacity, EmpAxis::Recharge] {
        for _ in 0..2 {
            equipment(&mut w, EmpEquipmentAction::Upgrade { item, axis });
        }
    }
    let (mut w, target) = defender(w);
    let cargo = w.commodity_quantity(crate::station_services::CommodityId::Food);
    let cash = w.credits_tenths();
    for _ in 0..4 {
        start_charge(&mut w, target);
        hold(&mut w, target, 600);
        assert_eq!(
            w.emp.charge.as_ref().unwrap().phase,
            EmpChargePhase::Released
        );
        assert!(
            w.emp.charge.as_ref().unwrap().pulse.is_some(),
            "actual aimed release refused"
        );
        if w.traffic.emp_resources(target).unwrap().3.unwrap().disabled {
            break;
        }
        w.request_emp_trigger(false, w.snapshot().tick).unwrap();
        w.step(aim_input(&w, target)).unwrap();
    }
    assert!(w.traffic.emp_resources(target).unwrap().3.unwrap().disabled);
    let victim = w
        .traffic_snapshot()
        .unwrap()
        .actors
        .into_iter()
        .find(|a| a.id == target)
        .unwrap();
    assert!(victim.energy_hundredths > 0);
    assert_eq!(victim.emp_crew_aboard, Some(1));
    let hull = victim.energy_hundredths;
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.request_jump(SystemId::AVENOR, w.snapshot().tick),
        Err(crate::navigation::NavigationError::WrongMode)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let episode = *w.emp.episodes.last().unwrap();
    let shots = w.traffic.beam_receipts().len();
    let mut loaded = cold(&w);
    for _ in 0..1800 {
        w.request_emp_trigger(false, w.snapshot().tick).unwrap();
        loaded
            .request_emp_trigger(false, loaded.snapshot().tick)
            .unwrap();
        steps(&mut w, 1);
        steps(&mut loaded, 1);
    }
    assert_eq!(w.combat.physical_tick(), episode.disabled_at + 1800);
    assert!(w.traffic.emp_resources(target).unwrap().3.unwrap().disabled);
    assert_eq!(w.traffic.beam_receipts().len(), shots);
    steps(&mut w, 1);
    steps(&mut loaded, 1);
    assert!(!w.traffic.emp_resources(target).unwrap().3.unwrap().disabled);
    assert_eq!(w.traffic.emp_resources(target).unwrap().0, hull);
    assert_eq!(
        w.traffic
            .emp_resources(target)
            .unwrap()
            .3
            .unwrap()
            .crew_aboard,
        1
    );
    assert_eq!(
        w.emp.episodes.last().unwrap().recovered_at.unwrap().0,
        episode.disabled_at + 1801
    );
    assert_eq!(w.emp_snapshot(), loaded.emp_snapshot());
    assert_eq!(w.traffic_snapshot(), loaded.traffic_snapshot());
    assert_eq!(w.credits_tenths(), cash);
    assert_eq!(
        w.commodity_quantity(crate::station_services::CommodityId::Food),
        cargo
    );
    cold(&w);
}

#[test]
fn real_arrival_posts_main_but_leaves_bank_gap_until_a_following_qualified_interval() {
    let mut w = paid();
    let physical = w.combat.physical_tick();
    w.request_launch(w.snapshot().tick).unwrap();
    steps(&mut w, 1);
    w.request_jump(SystemId::AVENOR, w.snapshot().tick).unwrap();
    steps(&mut w, usize::from(crate::navigation::BASIC_SPOOL_TICKS));
    assert_eq!(w.combat.physical_tick(), physical + 601);
    assert_eq!(w.emp.meter.bank, 600 * 20);
    assert_eq!(w.emp.bank_clock, physical + 600);
    cold(&w);
    let bank = w.emp.meter.bank;
    steps(&mut w, 1);
    assert_eq!(w.emp.meter.bank, bank);
    steps(&mut w, 1);
    assert_eq!(w.emp.meter.bank, bank + 20);
    cold(&w);
}

#[test]
fn failed_terminal_geometry_is_consumed_and_held_input_cannot_retry_or_spend() {
    let mut w = World::new_commander(12001).unwrap();
    secret(&mut w);
    let (mut w, target) = defender(w);
    start_charge(&mut w, target);
    assert_eq!(
        w.request_fire_front_laser(w.snapshot().tick),
        Err(crate::FireError::InsufficientEnergy)
    );
    assert_eq!(
        w.request_jump(SystemId::RENAS, w.snapshot().tick),
        Err(crate::navigation::NavigationError::WrongMode)
    );
    // Ordinary flight rotation is allowed while warming. Turn away through the public controls.
    for _ in 0..600 {
        w.request_emp_trigger(true, w.snapshot().tick).unwrap();
        w.step(FlightInput {
            pitch: 1.0,
            ..FlightInput::default()
        })
        .unwrap();
    }
    let charge = w.emp.charge.as_ref().unwrap();
    assert_eq!(charge.phase, EmpChargePhase::Released);
    assert_eq!(charge.pulse, None);
    assert_eq!(w.emp.meter.bank_spent, 0);
    assert!(w.emp.pulses.is_empty());
    for _ in 0..100 {
        w.request_emp_trigger(true, w.snapshot().tick).unwrap();
        w.step(FlightInput::default()).unwrap();
    }
    assert!(w.emp.pulses.is_empty());
    assert_eq!(w.emp.meter.bank_spent, 0);
    cold(&w);
}

#[test]
fn actual_non_aggressor_hull_cannot_authorize_emp_or_lose_resources() {
    let mut w = World::new_commander(12005).unwrap();
    secret(&mut w);
    launch(&mut w);
    crate::flight::missiles::tests::stop(&mut w);
    crate::flight::missiles::tests::fly_axis(&mut w, 0, -600.0, 10.0);
    steps(&mut w, 1300);
    let target = w
        .traffic_snapshot()
        .unwrap()
        .actors
        .into_iter()
        .find(|a| a.population == crate::traffic::Population::Trader)
        .unwrap()
        .id;
    let mut refused = false;
    for _ in 0..1200 {
        let before = w.emp.meter.bank_spent;
        match w.request_emp_trigger(true, w.snapshot().tick) {
            Err(EmpError::NoRecentAggressor) => {
                refused = true;
                break;
            }
            Err(EmpError::NoTarget | EmpError::InvalidContact) => {}
            other => panic!("unqualified native hull got unexpected admission {other:?}"),
        }
        assert_eq!(w.emp.meter.bank_spent, before);
        w.step(aim_input(&w, target)).unwrap();
    }
    assert!(refused);
    assert!(w.emp.charge.is_none());
    assert!(w.emp.pulses.is_empty());
    assert!(w.traffic.emp_events().is_empty());
    cold(&w);
}

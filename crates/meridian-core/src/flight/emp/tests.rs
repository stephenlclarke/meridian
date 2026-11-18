//! Real commerce/flight producers and separate numerical-kernel fixtures. Owner execution pending.
#![allow(clippy::unwrap_used)]
use super::{
    Arc, EmpAxis, EmpChargePhase, EmpEquipmentAction, EmpError, EmpGrades, EmpProvenance, World,
    meter, scene,
};
use crate::flight::equipment::tests::{earned_from, earned_world};
use crate::flight::practice::tests::launch;
use crate::freight::g02;
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
    let mut w = World::new_commander(16101).unwrap();
    let issuer = w.station_contacts()[0];
    w.request_contact(issuer, w.snapshot().tick).unwrap();
    let offer = w
        .request_freight(
            g02::Command::ReviewOffer {
                actor: issuer.actor(),
            },
            w.snapshot().tick,
        )
        .unwrap();
    let g02::Event::Reviewed(review) = offer else {
        panic!("actual freight offer missing");
    };
    let accepted = w
        .request_freight(
            g02::Command::Confirm {
                review,
                choice: g02::Choice::Accept,
            },
            w.snapshot().tick,
        )
        .unwrap();
    let g02::Event::Accepted { job, units } = accepted else {
        panic!("actual freight not accepted");
    };
    assert_eq!(units.len(), 8);
    assert_eq!(w.mission_snapshot().unwrap().freight.held_tonnes, 8);
    // The real protected contract freezes local traffic during these real paid market routes.
    // Keep every existing independent earned_from cash checkpoint and capacity refusal intact.
    let mut w = earned_from(w);
    assert_eq!(w.traffic.emp_clock(SystemId::AVENOR), Some(0));
    assert_eq!(w.traffic.emp_clock(SystemId::RENAS), Some(0));
    assert!(w.traffic.beam_receipts().is_empty());
    assert!(w.traffic.player_hits().is_empty());
    let receiver = w.station_contacts()[0];
    w.request_contact(receiver, w.snapshot().tick).unwrap();
    let receiving = w
        .request_freight(
            g02::Command::ReviewReceiving {
                actor: receiver.actor(),
                job,
            },
            w.snapshot().tick,
        )
        .unwrap();
    let g02::Event::Reviewed(review) = receiving else {
        panic!("actual receiving review missing");
    };
    let cash = w.credits_tenths().unwrap();
    let received = w
        .request_freight(
            g02::Command::Confirm {
                review,
                choice: g02::Choice::ReceiveAll,
            },
            w.snapshot().tick,
        )
        .unwrap();
    assert!(matches!(
        received,
        g02::Event::Received {
            remaining: 0,
            completion: Some(_),
            clue_awarded: true,
            ..
        }
    ));
    assert_eq!(w.credits_tenths(), Some(cash + 7300));
    assert_eq!(w.mission_snapshot().unwrap().freight.held_tonnes, 0);
    assert!(!w.npc_missile_protected());
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
    let point = w
        .station_data
        .as_ref()
        .unwrap()
        .player_planar_aimpoint(body.pose, pose.position)
        .unwrap()
        .unwrap_or_else(|| panic!("real target has no native planar surface"));
    let desired = (point.x - f64::from(p.position[0])).atan2(point.z - f64::from(p.position[2]));
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
    // Lure the real pirate away from the ordinary trader/station. Collateral civilians
    // remain protected; no role, position, contact, resource or receipt is injected.
    crate::flight::missiles::tests::fly_axis(&mut w, 0, -4500.0, 34.3);
    assert!(w.snapshot().position[0] <= -4500.0);
    for _ in 0..24000 {
        w.step(FlightInput::default()).unwrap();
        if let Some(hit) = w.traffic_snapshot().unwrap().last_player_hit
            && hit.visit == w.combat_identity().unwrap().visit
            && hit.world_tick == w.snapshot().tick
        {
            return (w, hit.attacker);
        }
    }
    panic!(
        "no actual recent native attacker; no substitute aggression receipt permitted; owner {:?}; ordinary proof {}; actual traffic {:?}",
        w.combat_identity(),
        w.traffic.receipt_count(),
        w.traffic_snapshot()
    );
}
fn admitted_emp_scene(
    w: &World,
    target: crate::traffic::NpcId,
) -> Result<Vec<scene::Contact>, EmpError> {
    let item = w.owned_emp().ok_or(EmpError::MissingItem)?;
    let owner = w.combat_identity().ok_or(EmpError::NoCommander)?;
    let p = w.snapshot();
    let player =
        super::super::flight_pose(p.position, p.orientation).ok_or(EmpError::InvalidContact)?;
    let profile = item.grades.profile(super::State::energy_stage(w)?)?;
    let native = scene::Scene::capture(
        &w.traffic,
        &w.missiles,
        w.combat.emissions,
        owner,
        player,
        p.tick,
        w.combat.physical_tick(),
    )?;
    let contacts = native.contacts(w, profile, target)?;
    if contacts.iter().any(|c| {
        matches!(c.victim, scene::Victim::Npc(id)
        if !super::State::recent_attack(&w.traffic, owner, id, p.tick))
    }) {
        return Err(EmpError::ProtectedTarget);
    }
    Ok(contacts)
}
fn target_settled(
    w: &World,
    target: crate::traffic::NpcId,
    previous: &mut Option<crate::docking::RigidPose>,
    quiet: &mut usize,
) -> bool {
    let body = w
        .traffic_snapshot()
        .unwrap()
        .actors
        .into_iter()
        .find(|a| a.id == target)
        .unwrap();
    let steady = body.speed_units == 0.0
        && previous.is_some_and(|old| {
            let turn = crate::docking::Vec3::new(
                body.pose.orientation.z.x - old.orientation.z.x,
                body.pose.orientation.z.y - old.orientation.z.y,
                body.pose.orientation.z.z - old.orientation.z.z,
            );
            body.pose.position == old.position && turn.dot(turn) < 1e-12
        });
    *quiet = if steady { *quiet + 1 } else { 0 };
    *previous = Some(body.pose);
    *quiet >= 30
}
fn traffic_start_failure_evidence(w: &World) -> String {
    w.traffic.validation_failure_evidence(
        w.combat_identity().unwrap().owner,
        w.snapshot().tick,
        w.combat.emissions,
        w.station_data.as_ref().unwrap(),
        w.station_markets.as_ref().unwrap(),
    )
}
fn charge_pilot_evidence(w: &World, target: crate::traffic::NpcId) -> String {
    let owner = w.combat_identity().unwrap();
    format!(
        "world={:?}; physical_tick={}; target={target:?}; recent_attacker={}; emp={:?}; meter={:?}; clocks(meter,bank,held,posted)={:?}; player_resources={:?}; combat={:?}; target_resources={:?}; traffic={:?}; previous_pulse={:#?}",
        w.snapshot(),
        w.combat.physical_tick(),
        super::State::recent_attack(&w.traffic, owner, target, w.snapshot().tick),
        w.emp_snapshot(),
        w.emp.meter,
        (
            w.emp.meter_clock,
            w.emp.bank_clock,
            w.emp.held_tick,
            w.emp.posted_tick
        ),
        w.flight_resources_snapshot(),
        w.combat_snapshot(),
        w.traffic.emp_resources(target),
        w.traffic_snapshot(),
        w.emp.pulses.last(),
    )
}
fn start_charge(w: &mut World, target: crate::traffic::NpcId) {
    let initial = charge_pilot_evidence(w, target);
    let mut attempts = Vec::new();
    let mut previous = None;
    let mut quiet = 0;
    for attempt in 0..1200 {
        // The exact-600-tick positive case needs an actually settled native target;
        // a start admission does not promise that a later moving surface stays on the ray.
        if !target_settled(w, target, &mut previous, &mut quiet) {
            attempts.push((attempt, "target_unsettled".to_owned()));
            w.step(aim_input(w, target)).unwrap();
            continue;
        }
        if let Err(error) = admitted_emp_scene(w, target) {
            attempts.push((attempt, format!("scene_admission {error:?}")));
            w.step(aim_input(w, target)).unwrap();
            continue;
        }
        match w.request_emp_trigger(true, w.snapshot().tick) {
            Ok(()) => {
                assert_eq!(w.emp.charge.as_ref().unwrap().target, target);
                return;
            }
            Err(
                error @ (EmpError::NoTarget
                | EmpError::NoRecentAggressor
                | EmpError::InvalidContact
                | EmpError::Resources),
            ) => {
                attempts.push((attempt, format!("trigger_refusal {error:?}")));
            }
            Err(error) => panic!(
                "unexpected start refusal {error:?}; {}",
                traffic_start_failure_evidence(w)
            ),
        }
        w.step(aim_input(w, target)).unwrap();
    }
    panic!(
        "real pilot failed to aim the EMP at its actual recent attacker; initial={initial}; attempts={attempts:?}; terminal={}",
        charge_pilot_evidence(w, target)
    );
}
fn hold(w: &mut World, target: crate::traffic::NpcId, ticks: usize) {
    for _ in 0..ticks {
        w.request_emp_trigger(true, w.snapshot().tick).unwrap();
        w.step(aim_input(w, target)).unwrap();
    }
}
fn self_splash_failure_evidence(w: &World, target: crate::traffic::NpcId) -> String {
    let pulse = &w.emp.pulses[0];
    let release = &pulse.release;
    let profile = w
        .owned_emp()
        .unwrap()
        .grades
        .profile(super::State::energy_stage(w).unwrap())
        .unwrap();
    let primary = pulse.contacts[0].point;
    let delta = crate::docking::Vec3::new(
        release.player.position.x - primary.x,
        release.player.position.y - primary.y,
        release.player.position.z - primary.z,
    );
    let length = delta.dot(delta).sqrt();
    let surface = (length.is_finite() && length > 1e-9).then(|| {
        let scale = 1.0 / length;
        let direction =
            crate::docking::Vec3::new(delta.x * scale, delta.y * scale, delta.z * scale);
        w.station_data.as_ref().unwrap().player_beam_distance(
            release.player,
            primary,
            direction,
            profile.range,
        )
    });
    let attenuation = match &surface {
        Some(Ok(Some(distance))) => Some(profile.splash(*distance)),
        _ => None,
    };
    format!(
        "release_profile={profile:?}; splash_range={}; primary_to_player_centre={length}; native_player_surface_within_full_range={surface:?}; native_surface_splash_power={attenuation:?}; replayed_contacts={:?}; retained_pulse={pulse:#?}; player_resources={:?}",
        profile.range / 3.0,
        release.contacts(w, profile, target),
        w.flight_resources_snapshot(),
    )
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
    // Compare the same accepted native step without an EMP release. Ordinary NPC
    // firing/recharge can change hull energy on this frame; only EMP damage must be zero.
    let bytes = w.save_bytes().unwrap();
    // Use the validated decoder, not Continue: Continue invalidates freight review state.
    // Decoding clears held_tick/posted_tick and contact, and creates a mining scope.
    // Fork the live step fields explicitly, including EMP transients and scope identity;
    // only immutable bundled data and fully encoded non-Clone owners come from decode.
    let decoded = World::decode_saved_world(&bytes).unwrap();
    assert_eq!(w.mission_snapshot().unwrap().contact, None);
    let mut native_control = World {
        state: w.state,
        traffic: w.traffic.clone(),
        mining: w.mining.clone(),
        mining_scope: Arc::clone(&w.mining_scope),
        damage: w.damage.clone(),
        station_data: decoded.station_data,
        navigation_catalogue: decoded.navigation_catalogue,
        navigation: w.navigation,
        jump_journal: decoded.jump_journal,
        route_plan: w.route_plan,
        commander_account: decoded.commander_account,
        freight_session: decoded.freight_session,
        station_markets: decoded.station_markets,
        pulse: w.pulse,
        combat: w.combat,
        missiles: w.missiles.clone(),
        ammunition: w.ammunition.clone(),
        modular: w.modular.clone(),
        xyzzy: w.xyzzy.clone(),
        emp: w.emp.clone(),
        navigation_trade: w.navigation_trade.clone(),
        flight_reception: w.flight_reception.clone(),
        practice: w.practice,
        practice_history: w.practice_history.clone(),
        practice_trace: w.practice_trace,
    };
    assert_eq!(native_control.save_bytes().unwrap(), bytes);
    assert_eq!(native_control.mission_snapshot(), w.mission_snapshot());
    assert_eq!(native_control.emp, w.emp);
    assert_eq!(native_control.state, w.state);
    assert!(Arc::ptr_eq(&native_control.mining_scope, &w.mining_scope));
    native_control
        .request_emp_trigger(false, native_control.snapshot().tick)
        .unwrap();
    native_control.step(aim_input(&w, target)).unwrap();
    let before = native_control.traffic.emp_resources(target).unwrap();
    let cash = w.credits_tenths();
    let cargo = w.commodity_quantity(crate::station_services::CommodityId::Food);
    let emissions = w.combat.emissions;
    let spent = w.emp.meter.bank_spent;
    hold(&mut w, target, 1);
    assert_eq!(
        w.emp.pulses.len(),
        1,
        "terminal {:?}; current native contact admission {:?}; actual traffic {:?}",
        w.emp_snapshot(),
        admitted_emp_scene(&w, target),
        w.traffic_snapshot()
    );
    assert_eq!(w.combat.physical_tick() - start, 600);
    assert_eq!(w.emp.meter.bank_spent - spent, 8 * 3000);
    assert_eq!(w.combat.emissions, emissions);
    assert!(
        w.emp.pulses[0]
            .contacts
            .iter()
            .any(|c| c.victim == scene::Victim::Player),
        "actual target-centred splash must include the nearby firing hull; {}",
        self_splash_failure_evidence(&w, target)
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
    // Real facing-shield preparation keeps the finite EMP sequence viable under its
    // own splash. It does not mint an attack receipt or refill the player's electricity.
    let mut shield_shots = 0;
    for _ in 0..300 {
        if native_facing_budget(&w, target).is_some()
            && w.flight_resources_snapshot().unwrap().cooldown_ticks == 0
        {
            let shot = w.request_fire_front_laser(w.snapshot().tick).unwrap();
            assert!(matches!(shot.contact, crate::PulseContact::Npc { id, .. } if id == target));
            assert!(w.traffic.emp_resources(target).unwrap().0 > 9500);
            shield_shots += 1;
            if shield_shots == 5 {
                break;
            }
        }
        w.step(aim_input(&w, target)).unwrap();
    }
    assert_eq!(
        shield_shots, 5,
        "actual facing-shield preparation must complete without killing/fleeing"
    );
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
    assert!(w.emp.pulses.iter().any(|pulse| {
        pulse
            .contacts
            .iter()
            .any(|contact| contact.victim == scene::Victim::Player && contact.power_hundredths > 0)
    }));
    let player_disabled_at = w.emp.meter.disabled_at;
    if let Some(at) = player_disabled_at {
        assert_eq!(at, episode.disabled_at);
    }
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
    if player_disabled_at.is_some() {
        assert!(w.emp.electrical_disabled());
        assert_eq!(w.emp.meter.main, 0);
    }
    steps(&mut w, 1);
    steps(&mut loaded, 1);
    assert!(!w.traffic.emp_resources(target).unwrap().3.unwrap().disabled);
    if player_disabled_at.is_some() {
        assert!(!w.emp.electrical_disabled());
        assert!(w.emp.meter.main > 0);
    }
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
    assert!(!w.xyzzy_snapshot().unwrap().unlimited_fuel);
    let cash = w.credits_tenths().unwrap();
    let fuel_paid = w.request_refuel(w.snapshot().tick).unwrap();
    assert!(fuel_paid > 0);
    assert_eq!(w.credits_tenths(), Some(cash - fuel_paid));
    assert_eq!(w.snapshot().navigation.unwrap().fuel_units, 130);
    let physical = w.combat.physical_tick();
    w.request_launch(w.snapshot().tick).unwrap();
    steps(&mut w, 1);
    w.request_jump(SystemId::AVENOR, w.snapshot().tick).unwrap();
    steps(
        &mut w,
        usize::try_from(crate::navigation::BASIC_SPOOL_TICKS).unwrap(),
    );
    assert_eq!(w.combat.physical_tick(), physical + 601);
    assert_eq!(w.snapshot().navigation.unwrap().fuel_units, 96);
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

type LaserCommand = fn(&mut World, u64) -> Result<crate::PulseShot, crate::FireError>;
fn assert_laser_commands_refuse_atomically(w: &mut World, expected: crate::FireError, tick: u64) {
    let commands: [LaserCommand; 2] = [
        World::request_fire_front_pulse,
        World::request_fire_front_laser,
    ];
    let before = w.save_bytes().unwrap();
    let resources = w.flight_resources_snapshot();
    let emp = w.emp_snapshot();
    let fit = w.front_laser_snapshot().unwrap();
    let (item, weapon) = w.fitted_front_laser().unwrap();
    assert_eq!((fit.fitted_item_id, fit.weapon), (item, weapon));
    for command in commands {
        assert_eq!(command(w, tick), Err(expected));
        assert_eq!(w.save_bytes().unwrap(), before);
        assert_eq!(w.flight_resources_snapshot(), resources);
        assert_eq!(w.emp_snapshot(), emp);
        assert_eq!(w.front_laser_snapshot(), Some(fit));
    }
}
fn assert_emp_blocks_both_lasers(w: &mut World) {
    assert!(w.emp.input_blocked());
    let tick = w.snapshot().tick;
    assert_laser_commands_refuse_atomically(w, crate::FireError::InsufficientEnergy, tick);
    assert_laser_commands_refuse_atomically(w, crate::FireError::StaleCommand, tick + 1);
    w.request_pause(true, tick).unwrap();
    assert_laser_commands_refuse_atomically(w, crate::FireError::Paused, tick);
    w.request_pause(false, tick).unwrap();
    assert_laser_commands_refuse_atomically(w, crate::FireError::InsufficientEnergy, tick);
}
#[test]
fn both_laser_commands_keep_normal_discharge_and_fitted_property_without_emp() {
    let mut w = World::new_commander(16104).unwrap();
    launch(&mut w);
    let fit = w.front_laser_snapshot().unwrap();
    assert_eq!(fit.weapon, crate::OrdinaryWeapon::FrontPulseLaser);
    let commands: [LaserCommand; 2] = [
        World::request_fire_front_pulse,
        World::request_fire_front_laser,
    ];
    for command in commands {
        let resources = w.flight_resources_snapshot().unwrap();
        let tick = w.snapshot().tick;
        let shot = command(&mut w, tick).unwrap();
        assert_eq!(shot.fitted_item_id, fit.fitted_item_id);
        let after = w.flight_resources_snapshot().unwrap();
        assert_eq!(after.energy_hundredths, resources.energy_hundredths - 100);
        assert_eq!(after.heat_fifths, resources.heat_fifths + 50);
        assert_eq!(after.cooldown_ticks, 18);
        assert_eq!(w.front_laser_snapshot(), Some(fit));
        cold(&w);
        steps(&mut w, 18);
    }
    assert_eq!(w.combat.emissions, 2);
    assert!(w.emp_snapshot().is_none());
}
#[test]
fn real_charge_blocks_both_laser_commands_but_preserves_fitted_snapshot_and_cancelled_firing() {
    let mut w = World::new_commander(12001).unwrap();
    secret(&mut w);
    let (mut w, target) = defender(w);
    let fit = w.front_laser_snapshot().unwrap();
    start_charge(&mut w, target);
    assert_eq!(
        w.emp_snapshot().unwrap().charge_phase,
        Some(EmpChargePhase::Charging)
    );
    assert_eq!(w.front_laser_snapshot(), Some(fit));
    assert_emp_blocks_both_lasers(&mut w);
    w.request_emp_trigger(false, w.snapshot().tick).unwrap();
    w.step(aim_input(&w, target)).unwrap();
    assert_eq!(
        w.emp_snapshot().unwrap().charge_phase,
        Some(EmpChargePhase::Cancelled)
    );
    assert_eq!(w.emp.meter.bank_spent, 0);
    let resources = w.flight_resources_snapshot().unwrap();
    let tick = w.snapshot().tick;
    let shot = w.request_fire_front_laser(tick).unwrap();
    assert_eq!(shot.fitted_item_id, fit.fitted_item_id);
    assert_eq!(
        w.flight_resources_snapshot().unwrap().energy_hundredths,
        resources.energy_hundredths - 100
    );
    assert_eq!(w.front_laser_snapshot(), Some(fit));
    cold(&w);
}
#[test]
fn isolated_electrical_disable_blocks_both_lasers_with_positive_fatal_energy_and_truthful_fit() {
    let mut w = World::new_commander(16105).unwrap();
    secret(&mut w);
    launch(&mut w);
    assert!(w.emp.charge.is_none());
    assert!(w.flight_resources_snapshot().unwrap().energy_hundredths > 0);
    // Isolated admission fixture: exercise the real finite drain kernel without inventing an
    // encounter receipt. Its absent pulse provenance MUST refuse ordinary save validation.
    // This is neither a produced World encounter nor a cold-Continue acceptance claim.
    let main = w.emp.meter.main;
    let clock = w.combat.physical_tick();
    assert!(main > 0);
    w.emp.meter.drain(main, clock).unwrap();
    assert!(w.emp.meter.validate().is_ok());
    assert!(w.emp.electrical_disabled());
    assert!(w.save_bytes().is_err());
    let before = w.unchecked_test_save().unwrap();
    let resources = w.flight_resources_snapshot();
    let emp = w.emp_snapshot();
    let fit = w.front_laser_snapshot().unwrap();
    let commands: [LaserCommand; 2] = [
        World::request_fire_front_pulse,
        World::request_fire_front_laser,
    ];
    for command in commands {
        let tick = w.snapshot().tick;
        assert_eq!(
            command(&mut w, tick),
            Err(crate::FireError::InsufficientEnergy)
        );
        assert_eq!(w.unchecked_test_save().unwrap(), before);
        assert_eq!(w.flight_resources_snapshot(), resources);
        assert_eq!(w.emp_snapshot(), emp);
        assert_eq!(w.front_laser_snapshot(), Some(fit));
        assert!(w.save_bytes().is_err());
    }
}

fn steer_to_real_point(w: &World, target: crate::docking::Vec3) -> FlightInput {
    let p = w.snapshot();
    let pose = super::super::flight_pose(p.position, p.orientation).unwrap();
    let delta = crate::docking::Vec3::new(
        target.x - pose.position.x,
        target.y - pose.position.y,
        target.z - pose.position.z,
    );
    let x = delta.dot(pose.orientation.x);
    let y = delta.dot(pose.orientation.y);
    let z = delta.dot(pose.orientation.z);
    FlightInput {
        yaw: (x.atan2(z) / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32,
        pitch: (y.atan2(x.hypot(z)) / (0.7 / 60.0)).clamp(-1.0, 1.0) as f32,
        ..FlightInput::default()
    }
}
fn track_real_target(w: &World, target: crate::traffic::NpcId) -> FlightInput {
    let body = w
        .traffic_snapshot()
        .unwrap()
        .actors
        .into_iter()
        .find(|a| a.id == target)
        .unwrap();
    let p = w.snapshot();
    let player = super::super::flight_pose(p.position, p.orientation).unwrap();
    // This existing selector is geometry-only; it issues no missile or EMP authority.
    let point = w
        .station_data
        .as_ref()
        .unwrap()
        .player_missile_aimpoint(body.pose, player.position)
        .unwrap();
    steer_to_real_point(w, point)
}
fn leave_native_npc_beam_plane(w: &mut World) {
    // Real NPC rays are planar. Normal pitch/throttle raises the actual player clear of
    // those rays, conserving finite receipt capacity for the later public fatal shot.
    for _ in 0..600 {
        let p = w.snapshot();
        if p.position[1] >= 300.0 && p.speed == 0.0 {
            return;
        }
        let mut input = steer_to_real_point(
            w,
            crate::docking::Vec3::new(
                f64::from(p.position[0]),
                f64::from(p.position[1]) + 10000.0,
                f64::from(p.position[2]),
            ),
        );
        input.brake = p.position[1] >= 300.0;
        if !input.brake {
            input.throttle = ((10.0 - p.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0);
        }
        w.step(input).unwrap();
        assert_eq!(w.snapshot().motion_refusal, None);
    }
    panic!("ordinary pilot failed to leave the native NPC ray plane");
}
fn start_real_3d_charge(w: &mut World, target: crate::traffic::NpcId) {
    for _ in 0..300 {
        if admitted_emp_scene(w, target).is_err() {
            w.step(track_real_target(w, target)).unwrap();
            continue;
        }
        match w.request_emp_trigger(true, w.snapshot().tick) {
            Ok(()) => {
                assert_eq!(w.emp.charge.as_ref().unwrap().target, target);
                return;
            }
            Err(EmpError::NoTarget | EmpError::InvalidContact | EmpError::Resources) => {}
            other => panic!(
                "real recovery fixture lost recent-attacker admission {other:?}; {}",
                traffic_start_failure_evidence(w)
            ),
        }
        w.step(track_real_target(w, target)).unwrap();
    }
    panic!("actual three-dimensional EMP aim never admitted its native attacker");
}
fn disabled_native_target() -> (World, crate::traffic::NpcId) {
    let mut w = paid();
    // As in the retained World case, the base is paid with earned cash; later funds/Elite
    // energy are an explicit XYZZY test setup, not an ordinary progression acceptance.
    secret(&mut w);
    let item = w.owned_emp().unwrap().item;
    for axis in [EmpAxis::Power, EmpAxis::Capacity, EmpAxis::Recharge] {
        for _ in 0..2 {
            equipment(&mut w, EmpEquipmentAction::Upgrade { item, axis });
        }
    }
    let (mut w, target) = defender(w);
    leave_native_npc_beam_plane(&mut w);
    let mut shield_shots = 0;
    for _ in 0..300 {
        if native_facing_budget(&w, target).is_some()
            && w.flight_resources_snapshot().unwrap().cooldown_ticks == 0
        {
            let tick = w.snapshot().tick;
            let shot = w.request_fire_front_laser(tick).unwrap();
            assert!(matches!(shot.contact, crate::PulseContact::Npc { id, .. } if id == target));
            assert!(w.traffic.emp_resources(target).unwrap().0 > 9500);
            shield_shots += 1;
            if shield_shots == 5 {
                break;
            }
        }
        w.step(track_real_target(&w, target)).unwrap();
    }
    assert_eq!(
        shield_shots, 5,
        "real paid laser must strip a facing shield without killing/fleeing"
    );
    for _ in 0..2 {
        start_real_3d_charge(&mut w, target);
        for _ in 0..600 {
            w.request_emp_trigger(true, w.snapshot().tick).unwrap();
            w.step(track_real_target(&w, target)).unwrap();
        }
        assert!(w.emp.charge.as_ref().unwrap().pulse.is_some());
        if w.traffic.emp_resources(target).unwrap().3.unwrap().disabled {
            assert!(
                !w.emp.electrical_disabled(),
                "recovery-order fixture requires a powered player"
            );
            let victim = w
                .traffic_snapshot()
                .unwrap()
                .actors
                .into_iter()
                .find(|a| a.id == target)
                .unwrap();
            assert_eq!(
                victim.speed_units, 0.0,
                "retain the real stopped target within the native laser range"
            );
            assert_eq!(w.snapshot().speed, 0.0);
            assert!(native_facing_budget(&w, target).is_some());
            assert!(
                w.traffic.receipt_count() < crate::traffic::MAX_TRAFFIC_SHOTS - 7,
                "retain real ordinary proof capacity for preparation and fatal command"
            );
            return (w, target);
        }
        w.request_emp_trigger(false, w.snapshot().tick).unwrap();
        w.step(track_real_target(&w, target)).unwrap();
    }
    panic!("actual aimed paid EMP releases did not disable the real recent attacker");
}
fn native_facing_budget(w: &World, target: crate::traffic::NpcId) -> Option<u32> {
    let p = w.snapshot();
    let pose = super::super::flight_pose(p.position, p.orientation)?;
    let owner = w.combat_identity()?;
    let contact = w.traffic.beam(
        w.station_data.as_ref()?,
        owner.system,
        pose.position,
        pose.orientation.z,
        6000.0,
    )?;
    if contact.id != target {
        return None;
    }
    let victim = w
        .traffic_snapshot()?
        .actors
        .into_iter()
        .find(|a| a.id == target)?
        .pose;
    let facing_front = victim.orientation.z.dot(crate::docking::Vec3::new(
        pose.position.x - victim.position.x,
        pose.position.y - victim.position.y,
        pose.position.z - victim.position.z,
    )) >= 0.0;
    let (energy, front, rear, _) = w.traffic.emp_resources(target)?;
    Some(u32::from(energy) + u32::from(if facing_front { front } else { rear }))
}
fn prepare_one_fatal_laser_at_recovery(
    w: &mut World,
    target: crate::traffic::NpcId,
    deadline: u64,
) {
    let damage = u32::from(w.front_laser_snapshot().unwrap().damage_hundredths);
    while w.combat.physical_tick() < deadline - 1 {
        let remaining = deadline - w.combat.physical_tick();
        if let Some(budget) = native_facing_budget(w, target) {
            // Native SH025 earns at most seven hull and three facing-shield hundredths per
            // tick. This upper bound admits only a genuinely fatal next-boundary public shot.
            let ready = budget > 0 && u64::from(budget) + remaining * 10 <= u64::from(damage);
            let cooled = w.flight_resources_snapshot().unwrap().cooldown_ticks == 0;
            if ready && cooled {
                return;
            }
            if budget > damage && cooled {
                let tick = w.snapshot().tick;
                let shot = w.request_fire_front_laser(tick).unwrap();
                assert!(
                    matches!(shot.contact, crate::PulseContact::Npc { id, .. } if id == target)
                );
                assert!(w.traffic.emp_resources(target).unwrap().0 > 0);
            }
        }
        w.step(track_real_target(w, target)).unwrap();
    }
    panic!("public laser preparation did not leave the live disabled hull within one fatal shot");
}
fn prepared_recovery_boundary() -> (World, crate::traffic::NpcId, super::runtime::Episode, u64) {
    let (mut w, target) = disabled_native_target();
    let episode = *w.emp.episodes.last().unwrap();
    assert_eq!(episode.victim, target);
    let deadline = episode.disabled_at + 1801;
    while w.combat.physical_tick() < deadline - 60 {
        w.step(track_real_target(&w, target)).unwrap();
    }
    prepare_one_fatal_laser_at_recovery(&mut w, target, deadline);
    while w.combat.physical_tick() < deadline - 1 {
        w.step(track_real_target(&w, target)).unwrap();
    }
    assert!(w.traffic.emp_resources(target).unwrap().3.unwrap().disabled);
    assert!(w.emp.episodes.last().unwrap().recovered_at.is_none());
    assert!(native_facing_budget(&w, target).unwrap() > 0);
    cold(&w);
    (w, target, episode, deadline)
}
#[test]
fn native_recovery_then_same_tick_fatal_public_laser_saves_and_continues_without_resurrection() {
    let (mut w, target, episode, deadline) = prepared_recovery_boundary();
    w.step(track_real_target(&w, target)).unwrap();
    assert_eq!(w.combat.physical_tick(), deadline);
    let recovered = w.emp.episodes.last().unwrap().recovered_at.unwrap();
    assert_eq!(recovered, (deadline, w.snapshot().tick));
    assert!(!w.traffic.emp_resources(target).unwrap().3.unwrap().disabled);
    assert!(w.traffic.emp_resources(target).unwrap().0 > 0);
    cold(&w);
    let tick = w.snapshot().tick;
    let shot = w.request_fire_front_laser(tick).unwrap();
    assert!(matches!(shot.contact, crate::PulseContact::Npc { id, .. } if id == target));
    assert_eq!(w.traffic.emp_resources(target).unwrap().0, 0);
    let local = w.traffic.emp_clock(target.system()).unwrap();
    let before_recovery = w
        .traffic
        .emp_before_recovery(target, local, tick, episode.origin)
        .unwrap();
    assert!(
        before_recovery.0 > 0,
        "later same-tick fatal command leaked into the step boundary"
    );
    assert!(before_recovery.3.unwrap().disabled);
    let mut loaded = cold(&w);
    assert_eq!(
        loaded.emp.episodes.last().unwrap().recovered_at,
        Some(recovered)
    );
    steps(&mut loaded, 1);
    assert_eq!(loaded.traffic.emp_resources(target).unwrap().0, 0);
    assert!(
        loaded
            .traffic_snapshot()
            .unwrap()
            .actors
            .iter()
            .any(|a| a.id == target && a.phase == crate::traffic::NpcPhase::Destroyed)
    );
    cold(&loaded);
}
#[test]
fn native_death_before_recovery_stays_dead_and_refuses_recovery_without_resource_or_save_change() {
    let (mut w, target, episode, deadline) = prepared_recovery_boundary();
    let tick = w.snapshot().tick;
    let shot = w.request_fire_front_laser(tick).unwrap();
    assert!(matches!(shot.contact, crate::PulseContact::Npc { id, .. } if id == target));
    assert_eq!(w.traffic.emp_resources(target).unwrap().0, 0);
    w.step(FlightInput::default()).unwrap();
    assert_eq!(w.combat.physical_tick(), deadline);
    assert!(w.emp.episodes.last().unwrap().recovered_at.is_none());
    assert_eq!(w.traffic.emp_resources(target).unwrap().0, 0);
    let local = w.traffic.emp_clock(target.system()).unwrap();
    let before_recovery = w
        .traffic
        .emp_before_recovery(target, local, w.snapshot().tick, episode.origin)
        .unwrap();
    assert_eq!(
        before_recovery.0, 0,
        "prior-tick fatal command must remain in the boundary query"
    );
    let before = w.save_bytes().unwrap();
    let tick = w.snapshot().tick;
    assert_eq!(
        w.traffic.emp_event(
            episode.origin,
            target,
            tick,
            deadline,
            crate::traffic::EmpResourceAction::Recover
        ),
        Err(crate::traffic::TrafficError::InvalidState)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let mut loaded = cold(&w);
    steps(&mut loaded, 1);
    assert_eq!(loaded.traffic.emp_resources(target).unwrap().0, 0);
    assert!(loaded.emp.episodes.last().unwrap().recovered_at.is_none());
    cold(&loaded);
}

//! Real World command tests; compilation/execution remain assigned to the existing owner lane.
use super::*;
use crate::FlightInput;
use crate::docking::Vec3;
use crate::traffic::{BeamSource, NpcPhase, Population};
fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    match v {
        Ok(v) => v,
        Err(e) => panic!("unexpected refusal: {e:?}"),
    }
}
fn some<T>(v: Option<T>) -> T {
    match v {
        Some(v) => v,
        None => panic!("missing actual traffic projection"),
    }
}
fn steps(w: &mut World, n: usize) {
    for _ in 0..n {
        ok(w.step(FlightInput::default()));
    }
}
#[test]
fn original_public_market_route_keeps_native_clearance_with_live_off_axis_traffic() {
    let mut w = ok(World::new_commander(12001));
    ok(w.request_station_service(crate::station_services::Command::BuyFood, 0));
    // Reuse the baseline public journey unchanged, including every no-contact assertion.
    crate::flight::equipment::tests::journey(&mut w, crate::navigation::SystemId::RENAS);
    assert_eq!(w.snapshot().mode, FlightMode::Docked);
    if some(w.snapshot().navigation).fuel_units < 68 {
        ok(w.request_refuel(w.snapshot().tick));
    }
    crate::flight::equipment::tests::journey(&mut w, crate::navigation::SystemId::AVENOR);
    assert_eq!(w.snapshot().mode, FlightMode::Docked);
    assert_eq!(w.snapshot().motion_refusal, None);
    assert!(w.traffic.enabled());
    assert!(
        some(w.traffic_snapshot())
            .actors
            .iter()
            .any(|a| a.population == Population::Pirate)
    );
    assert_eq!(
        w.commodity_quantity(crate::station_services::CommodityId::Food),
        Some(1)
    );
    let _continued = cold(&w);
}
fn clear_lane(seed: u64) -> World {
    let mut w = ok(World::new_commander(seed));
    crate::flight::practice::tests::launch(&mut w);
    crate::flight::missiles::tests::stop(&mut w);
    crate::flight::missiles::tests::fly_axis(&mut w, 0, -600.0, 10.0);
    w
}
fn cold(w: &World) -> World {
    let bytes = ok(w.save_bytes());
    let loaded = ok(World::from_save_bytes(&bytes));
    assert_eq!(loaded.traffic_snapshot(), w.traffic_snapshot());
    assert_eq!(loaded.combat_snapshot(), w.combat_snapshot());
    assert_eq!(
        loaded.flight_resources_snapshot(),
        w.flight_resources_snapshot()
    );
    assert_eq!(loaded.credits_tenths(), w.credits_tenths());
    assert_eq!(
        loaded.mission_snapshot().map(|m| m.freight),
        w.mission_snapshot().map(|m| m.freight)
    );
    loaded
}
fn cargo_pilot() -> World {
    let mut w = ok(World::new_commander(12001));
    ok(w.request_station_service(crate::station_services::Command::BuyFood, 0));
    crate::flight::practice::tests::launch(&mut w);
    crate::flight::missiles::tests::stop(&mut w);
    crate::flight::missiles::tests::fly_axis(&mut w, 0, -600.0, 10.0);
    w
}
fn first_player_contact(w: &mut World) -> crate::traffic::NpcPlayerHit {
    for _ in 0..24000 {
        ok(w.step(FlightInput::default()));
        if let Some(hit) = some(w.traffic_snapshot()).last_player_hit {
            return hit;
        }
    }
    panic!("actual cargo pilot never received an admitted native pirate Pulse contact");
}
#[test]
fn actual_cargo_pilot_receives_native_npc_damage_without_player_incident_or_invented_fine() {
    let mut w = cargo_pilot();
    let credits = w.credits_tenths();
    let hit = first_player_contact(&mut w);
    assert_eq!(hit.attacker.serial(), 2);
    assert_eq!(hit.attacker.profile(), 12001);
    assert_eq!(hit.player_hull, 12002);
    assert_eq!(hit.world_tick, w.snapshot().tick);
    assert_eq!(
        hit.player_pose,
        some(flight_pose(w.snapshot().position, w.snapshot().orientation))
    );
    let condition = some(w.combat_snapshot());
    let shields = [
        condition.player.front_shield_hundredths,
        condition.player.rear_shield_hundredths,
    ];
    assert_eq!(shields[usize::from(!hit.front)], 28500);
    assert_eq!(shields[usize::from(hit.front)], 30000);
    assert_eq!(condition.last_ordinary_impact, None);
    assert_eq!(w.combat.emissions, 0);
    assert_eq!(w.combat.incidents, 0);
    assert_eq!(w.legal_value(), Some(0));
    assert_eq!(w.credits_tenths(), credits);
    assert_eq!(
        w.commodity_quantity(crate::station_services::CommodityId::Food),
        Some(1)
    );
    assert!(
        some(w.traffic_snapshot())
            .actors
            .iter()
            .all(|a| a.population != Population::Police)
    );
    let mut loaded = cold(&w);
    steps(&mut w, 36);
    steps(&mut loaded, 36);
    assert_eq!(w.traffic_snapshot(), loaded.traffic_snapshot());
    assert_eq!(w.combat_snapshot(), loaded.combat_snapshot());
}
#[test]
fn sustained_actual_npc_pulses_reach_real_hull_energy_and_exact_cold_replay_rejects_extra_damage() {
    let mut w = cargo_pilot();
    first_player_contact(&mut w);
    let mut hull_hit = false;
    for _ in 0..6000 {
        ok(w.step(FlightInput::default()));
        if some(w.flight_resources_snapshot()).energy_hundredths < 38000 {
            hull_hit = true;
            break;
        }
    }
    assert!(
        hull_hit,
        "real pirate did not deplete a facing shield before the retained proof bound"
    );
    let good = ok(w.save_bytes());
    let mut loaded = cold(&w);
    steps(&mut w, 18);
    steps(&mut loaded, 18);
    assert_eq!(w.combat_snapshot(), loaded.combat_snapshot());
    let mut bad = ok(World::from_save_bytes(&good));
    bad.pulse.damage_energy(1);
    let corrupt = ok(bad.unchecked_test_save());
    let live_before = ok(w.save_bytes());
    assert!(w.restore_bytes(&corrupt).is_err());
    assert_eq!(ok(w.save_bytes()), live_before);
}
#[test]
fn active_t02_retains_player_identity_incident_refusal_and_freezes_incoming_npc_state() {
    let mut source = clear_lane(12005);
    steps(&mut source, 500);
    let id = some(some(source.traffic_snapshot()).actors.first()).id;
    let mut w = ok(World::new_commander(12005));
    crate::flight::practice::tests::consent(&mut w);
    crate::flight::practice::tests::launch(&mut w);
    steps(&mut w, 2000);
    assert!(some(w.traffic_snapshot()).actors.is_empty());
    let before = ok(w.save_bytes());
    // Descriptive copied NPC identity confers no new incident or damage authority.
    let shot = PulseShot {
        tick: w.snapshot().tick,
        fitted_item_id: 1,
        origin: Vec3::new(0.0, 0.0, 0.0),
        direction: Vec3::new(0.0, 0.0, 1.0),
        end: Vec3::new(0.0, 0.0, 1.0),
        contact: PulseContact::Npc { id, distance: 1.0 },
    };
    assert_eq!(
        w.prepare_traffic_pulse(shot, 1),
        Err(FireError::ProtectedTraffic)
    );
    assert_eq!(ok(w.save_bytes()), before);
    assert_eq!(w.combat.incidents, 0);
    assert_eq!(w.legal_value(), Some(0));
    let _continued = cold(&w);
}
fn market_quote(
    w: &World,
    id: crate::station_services::CommodityId,
) -> crate::station_services::CommodityQuote {
    let system = some(w.snapshot().navigation).system;
    let site = match system {
        crate::navigation::SystemId::AVENOR => crate::station_services::MarketSite::Avenor,
        crate::navigation::SystemId::RENAS => crate::station_services::MarketSite::Renas,
        _ => panic!("unsupported local traffic market"),
    };
    some(w.station_markets.as_ref()).commodity_quote(site, id)
}
#[test]
fn paused_stale_and_invalid_input_do_not_advance_physical_birth_stock_or_resources() {
    let mut w = clear_lane(12001);
    steps(&mut w, 600);
    let tick = w.snapshot().tick;
    ok(w.request_pause(true, tick));
    let before = ok(w.save_bytes());
    assert_eq!(w.step(FlightInput::default()), Err(StepError::Paused));
    assert_eq!(ok(w.save_bytes()), before);
    ok(w.request_pause(false, tick));
    let before = ok(w.save_bytes());
    assert_eq!(
        w.request_fire_front_pulse(tick - 1),
        Err(FireError::StaleCommand)
    );
    assert_eq!(ok(w.save_bytes()), before);
    assert_eq!(
        w.step(FlightInput {
            throttle: f32::NAN,
            ..FlightInput::default()
        }),
        Err(StepError::NonFiniteInput)
    );
    assert_eq!(ok(w.save_bytes()), before);
    let _continued = cold(&w);
}
#[test]
fn real_public_flight_birth_and_cold_continue_preserve_paid_npc_manifest_without_player_grants() {
    let mut w = clear_lane(12005);
    steps(&mut w, 500);
    let scene = some(w.traffic_snapshot());
    let trader = some(
        scene
            .actors
            .iter()
            .find(|a| a.population == Population::Trader),
    );
    assert_eq!(trader.hull_id, "SH025");
    assert_eq!(
        trader.cargo,
        Some((crate::station_services::CommodityId::Food, 9))
    );
    assert_eq!(trader.cash_tenths, 117966);
    assert_eq!(
        market_quote(&w, crate::station_services::CommodityId::Food).stock,
        119
    );
    assert_eq!(
        w.commodity_quantity(crate::station_services::CommodityId::Food),
        Some(0)
    );
    assert_eq!(w.credits_tenths(), Some(7200));
    let mut loaded = cold(&w);
    steps(&mut w, 120);
    steps(&mut loaded, 120);
    assert_eq!(w.traffic_snapshot(), loaded.traffic_snapshot());
    assert_eq!(w.service_snapshot(), loaded.service_snapshot());
}
#[test]
fn trader_completes_real_native_entry_then_sells_once_and_never_respawns_after_continue() {
    let mut w = clear_lane(12005);
    let mut complete = false;
    for _ in 0..24000 {
        ok(w.step(FlightInput::default()));
        if some(w.traffic_snapshot())
            .actors
            .iter()
            .any(|a| a.population == Population::Trader && a.phase == NpcPhase::Complete)
        {
            complete = true;
            break;
        }
    }
    assert!(
        complete,
        "native traffic pilot did not complete the physical route/entry"
    );
    let scene = some(w.traffic_snapshot());
    let trader = some(
        scene
            .actors
            .iter()
            .find(|a| a.population == Population::Trader),
    );
    assert_eq!(trader.cash_tenths, 120000 - 9 * (226 - 208));
    assert_eq!(trader.cargo, None);
    assert_eq!(
        market_quote(&w, crate::station_services::CommodityId::Food).stock,
        128
    );
    assert_eq!(w.credits_tenths(), Some(7200));
    let mut loaded = cold(&w);
    steps(&mut loaded, 2000);
    assert_eq!(loaded.traffic_snapshot(), w.traffic_snapshot());
    assert_eq!(
        market_quote(&loaded, crate::station_services::CommodityId::Food).stock,
        128
    );
}
#[test]
fn genuine_pirate_contact_damages_actual_trader_and_queues_only_unavailable_police_source() {
    let mut w = clear_lane(12001);
    let mut contacted = false;
    for _ in 0..12000 {
        ok(w.step(FlightInput::default()));
        let scene = some(w.traffic_snapshot());
        if let Some(beam) = scene.last_beam
            && matches!(beam.source, BeamSource::Npc(_))
        {
            assert_eq!(beam.victim.serial(), 1);
            assert!(scene.verified_crimes > 0);
            assert_eq!(scene.police_required_hull, Some("SH031"));
            let trader = some(scene.actors.iter().find(|a| a.id == beam.victim));
            assert!(
                trader.front_shield_hundredths < 30000
                    || trader.rear_shield_hundredths < 30000
                    || trader.energy_hundredths < 38000
            );
            contacted = true;
            break;
        }
    }
    assert!(
        contacted,
        "admitted pirate never made an actual unoccluded native hull contact"
    );
    let scene = some(w.traffic_snapshot());
    assert!(scene.actors.iter().all(|a| matches!(
        a.population,
        Population::Trader | Population::Pirate
    ) && a.hull_id == "SH025"));
    assert_eq!(w.credits_tenths(), Some(7200));
    let _continued = cold(&w);
}
#[test]
fn actual_player_pulse_uses_first_npc_mesh_and_bound_weapon_resources_and_retains_hit_cold() {
    let mut w = clear_lane(12005);
    let mut hit = None;
    for _ in 0..1500 {
        if let Some(scene) = w.traffic_snapshot()
            && let Some(target) = scene
                .actors
                .iter()
                .find(|a| a.population == Population::Trader && a.phase != NpcPhase::Complete)
        {
            let p = w.snapshot();
            let delta = Vec3::new(
                target.pose.position.x - f64::from(p.position[0]),
                0.0,
                target.pose.position.z - f64::from(p.position[2]),
            );
            let desired = delta.x.atan2(delta.z);
            let pose = some(flight_pose(p.position, p.orientation));
            let yaw = pose.orientation.z.x.atan2(pose.orientation.z.z);
            let error = (desired - yaw).sin().atan2((desired - yaw).cos());
            let turn = (error / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32;
            if matches!(w.pulse_scene(pose), Ok((_, PulseContact::Npc { .. }, _))) {
                let before = some(w.flight_resources_snapshot());
                let shot = ok(w.request_fire_front_pulse(p.tick));
                let PulseContact::Npc { id, .. } = shot.contact else {
                    panic!("lost admitted NPC contact");
                };
                let after = some(w.flight_resources_snapshot());
                assert_eq!(after.energy_hundredths, before.energy_hundredths - 100);
                let scene = some(w.traffic_snapshot());
                let beam = some(scene.last_beam);
                assert_eq!(beam.victim, id);
                assert!(
                    matches!(beam.source,BeamSource::Player{owner:12005,item,..} if item==shot.fitted_item_id)
                );
                hit = Some(id);
                break;
            }
            ok(w.step(FlightInput {
                yaw: turn,
                ..FlightInput::default()
            }));
            continue;
        }
        ok(w.step(FlightInput::default()));
    }
    assert!(
        hit.is_some(),
        "public aiming never admitted the real moving hull"
    );
    assert_eq!(w.credits_tenths(), Some(7200));
    let _continued = cold(&w);
}
#[test]
fn old_v13_and_corrupt_current_traffic_refuse_without_replacing_live_bytes() {
    let initial = ok(World::new_commander(12005));
    let old = ok(crate::flight::persistence::v13_test_fixture(&initial));
    assert_eq!(u16::from_le_bytes([old[8], old[9]]), 13);
    assert_eq!(
        World::from_save_bytes(&old).unwrap_err(),
        crate::save::SaveError::UnsupportedVersion
    );
    let mut live = clear_lane(12005);
    steps(&mut live, 500);
    let valid = ok(live.save_bytes());
    let mut bad = ok(World::from_save_bytes(&valid));
    bad.traffic = crate::traffic::State::default();
    let invalid = ok(bad.unchecked_test_save());
    assert!(live.restore_bytes(&invalid).is_err());
    assert_eq!(ok(live.save_bytes()), valid);
    let mut bad = ok(World::from_save_bytes(&valid));
    let actor = some(
        some(bad.traffic_snapshot())
            .actors
            .into_iter()
            .find(|a| a.population == Population::Trader),
    );
    bad.state.position = [
        actor.pose.position.x as f32,
        actor.pose.position.y as f32,
        actor.pose.position.z as f32,
    ];
    let invalid = ok(bad.unchecked_test_save());
    assert!(live.restore_bytes(&invalid).is_err());
    assert_eq!(ok(live.save_bytes()), valid);
}
#[test]
fn later_real_npc_market_purchase_preserves_original_completed_t04_stock_receipts() {
    use crate::NavigationTradeAction as Action;
    use crate::navigation::SystemId;
    use crate::station_services::{Command, CommodityId};
    fn lesson(w: &mut World, action: Action) {
        let tick = w.snapshot().tick;
        let review = ok(w.review_navigation_trade(action, tick));
        ok(w.prepare_navigation_trade(review, tick)).commit();
    }
    fn service(w: &mut World, command: Command) {
        let tick = w.snapshot().tick;
        ok(w.request_station_service(command, tick));
    }
    let mut w = ok(World::new_commander(12005));
    lesson(&mut w, Action::Accept);
    lesson(
        &mut w,
        Action::Plan {
            destination: SystemId::RENAS,
            commodity: CommodityId::Food,
        },
    );
    service(&mut w, Command::BuyFood);
    crate::flight::equipment::tests::journey(&mut w, SystemId::RENAS);
    service(&mut w, Command::SellFood);
    service(&mut w, Command::RefillFuel);
    crate::flight::equipment::tests::journey(&mut w, SystemId::AVENOR);
    lesson(&mut w, Action::Receive);
    let receipt = some(w.navigation_trade_snapshot());
    assert!(receipt.trace.is_some());
    assert!(some(w.traffic_snapshot()).actors.is_empty());
    let cash = w.credits_tenths();
    crate::flight::practice::tests::launch(&mut w);
    crate::flight::missiles::tests::stop(&mut w);
    crate::flight::missiles::tests::fly_axis(&mut w, 0, -600.0, 10.0);
    steps(&mut w, 600);
    let trader = some(
        some(w.traffic_snapshot())
            .actors
            .into_iter()
            .find(|a| a.population == Population::Trader),
    );
    assert_eq!(trader.cargo, Some((CommodityId::Food, 9)));
    assert_eq!(market_quote(&w, CommodityId::Food).stock, 118);
    assert_eq!(w.navigation_trade_snapshot(), Some(receipt.clone()));
    assert_eq!(w.credits_tenths(), cash);
    let loaded = cold(&w);
    assert_eq!(loaded.navigation_trade_snapshot(), Some(receipt));
}

#[test]
fn actual_celestial_geometry_blocks_npc_beam_segments_before_damage() {
    // Component geometry, not a claimed public World flight to these coordinates.
    let w = ok(World::new_commander(12001));
    let catalogue = some(w.navigation_catalogue.as_ref());
    let system = ok(catalogue.system(crate::navigation::SystemId::RENAS));
    let scene = crate::celestial::scene(system, 0, 1, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(scene.planet.position, Vec3::new(6000.0, -1800.0, 9500.0));
    assert_eq!(scene.planet.radius, 2200.0);
    let origin = Vec3::new(8000.0, 0.0, 9500.0);
    let target = Vec3::new(2400.0, 0.0, 9000.0);
    // Independent closest-point scalar reference for the review's concrete obstruction.
    let delta = Vec3::new(-5600.0, 0.0, -500.0);
    let offset = Vec3::new(-2000.0, -1800.0, 0.0);
    let t = offset.dot(delta) / delta.dot(delta);
    let nearest = Vec3::new(origin.x + t * delta.x, 0.0, origin.z + t * delta.z);
    let from_center = Vec3::new(nearest.x - 6000.0, 1800.0, nearest.z - 9500.0);
    assert!(delta.dot(delta).sqrt() < super::super::weapons::PULSE_RANGE);
    assert!(from_center.dot(from_center).sqrt() < scene.planet.radius);
    assert!(!w.traffic_beam_celestial_clear(system.id, 1, origin, target));
    for body in [scene.planet, scene.sun] {
        let left = Vec3::new(
            body.position.x - body.radius - 100.0,
            body.position.y,
            body.position.z,
        );
        let right = Vec3::new(
            body.position.x + body.radius + 100.0,
            body.position.y,
            body.position.z,
        );
        assert!(!w.traffic_beam_celestial_clear(system.id, 1, left, right));
        let outside = Vec3::new(left.x, left.y + body.radius * 2.0, left.z);
        let nearby = Vec3::new(outside.x - 1.0, outside.y, outside.z);
        assert!(w.traffic_beam_celestial_clear(system.id, 1, outside, nearby));
    }
}

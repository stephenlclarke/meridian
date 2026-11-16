//! Real World command tests; compilation/execution remain assigned to the existing owner lane.
use super::*;
use crate::docking::Vec3;
use crate::traffic::{BeamSource, NpcPhase, Population};
use crate::{FlightInput, MotionRefusal};
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
    assert_eq!(loaded.credits_tenths(), w.credits_tenths());
    assert_eq!(
        loaded.mission_snapshot().map(|m| m.freight),
        w.mission_snapshot().map(|m| m.freight)
    );
    loaded
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
        if let Some(beam) = scene.last_beam {
            if matches!(beam.source, BeamSource::Npc(_)) {
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
        if let Some(scene) = w.traffic_snapshot() {
            if let Some(target) = scene
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
fn original_v13_continue_has_absent_traffic_and_corrupt_current_restore_preserves_live_bytes() {
    let initial = ok(World::new_commander(12005));
    let old = ok(crate::flight::persistence::v13_test_fixture(&initial));
    assert_eq!(u16::from_le_bytes([old[8], old[9]]), 13);
    let mut older = ok(World::from_save_bytes(&old));
    assert!(older.traffic_snapshot().is_none());
    crate::flight::practice::tests::launch(&mut older);
    steps(&mut older, 1500);
    assert!(older.traffic_snapshot().is_none());
    assert_eq!(older.credits_tenths(), Some(7200));
    let loaded = cold(&older);
    assert!(loaded.traffic_snapshot().is_none());
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

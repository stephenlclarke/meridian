//! Actual public inputs for positive previews; pure copied summaries are arithmetic fixtures only.
use super::*;
use crate::station_services::Command;
use crate::{FlightInput, ModuleAction, ModuleSlot, NavigationTradeAction, XyzzyAction};
fn ok<T, E: fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}
fn some<T>(v: Option<T>) -> T {
    v.unwrap_or_else(|| panic!("missing actual state"))
}
fn preview(w: &World) -> NavigationTradePreview {
    ok(w.navigation_trade_preview(SystemId::LARERIA, w.snapshot().tick))
}
fn trade(w: &mut World, id: CommodityId) {
    ok(w.request_station_service(Command::BuyCommodity(id), w.snapshot().tick));
}
fn lesson(w: &mut World, action: NavigationTradeAction) {
    let tick = w.snapshot().tick;
    let review = ok(w.review_navigation_trade(action, tick));
    ok(w.prepare_navigation_trade(review, tick)).commit();
}
#[test]
fn original_cash_limited_scan_keeps_kg_and_g_native_and_stock_two_leg_opportunities_are_not_transactions()
 {
    let w = ok(World::new_commander(29101));
    let before = ok(w.save_bytes());
    let p = preview(&w);
    assert_eq!(p.direct_profits.len(), 1);
    let d = &p.direct_profits[0];
    assert_eq!(d.destination, SystemId::RENAS);
    assert_eq!(d.estimated_delta_tenths, 2618);
    assert_eq!(d.carried_delta_tenths, 0);
    assert_eq!(
        d.exports,
        vec![
            TradeExportEstimate {
                commodity: CommodityId::Gold,
                quantity: 1,
                ask_tenths: 4345,
                benchmark_bid_tenths: 5658,
                gross_tenths: 1313
            },
            TradeExportEstimate {
                commodity: CommodityId::GemStones,
                quantity: 3,
                ask_tenths: 894,
                benchmark_bid_tenths: 1329,
                gross_tenths: 1305
            }
        ]
    );
    assert_eq!(CommodityId::Gold.unit().symbol(), "kg");
    assert_eq!(CommodityId::GemStones.unit().symbol(), "g");
    assert_eq!(p.paths.len(), 1);
    assert_eq!(
        p.paths[0].systems,
        [SystemId::AVENOR, SystemId::RENAS, SystemId::LARERIA]
    );
    assert_eq!(p.paths[0].independent_leg_gross_tenths, 62656);
    let leg = some(p.paths[0].legs[0].opportunity.as_ref());
    assert_eq!(
        (
            leg.commodity,
            leg.quantity,
            leg.ask_tenths,
            leg.benchmark_bid_tenths,
            leg.gross_tenths
        ),
        (CommodityId::Gold, 47, 4345, 5658, 61711)
    );
    let leg = some(p.paths[0].legs[1].opportunity.as_ref());
    assert_eq!(
        (
            leg.commodity,
            leg.quantity,
            leg.ask_tenths,
            leg.benchmark_bid_tenths,
            leg.gross_tenths
        ),
        (CommodityId::Alloys, 63, 593, 608, 945)
    );
    assert!(p.paths[0].labels.contains(&"Trade opportunities"));
    assert!(p.assumptions().contains("not guaranteed"));
    assert_eq!(w.navigation_route_snapshot(), None);
    assert_eq!(ok(w.save_bytes()), before);
    let cold = ok(World::from_save_bytes(&before));
    assert_eq!(preview(&cold), p);
}
#[test]
fn genuine_purchase_refreshes_cash_stock_and_carried_opportunity_without_changing_benchmark_or_installing_a_route()
 {
    let mut w = ok(World::new_commander(29111));
    let before = preview(&w);
    trade(&mut w, CommodityId::GemStones);
    let bytes = ok(w.save_bytes());
    let after = preview(&w);
    assert_ne!(after, before);
    assert_eq!(after.cash_tenths, 6306);
    assert_eq!(after.ordinary_quantities[15], 1);
    assert_eq!(after.used_tonnes, 0);
    assert_eq!(after.quotes[0][15].stock, 105);
    assert_eq!(after.benchmark_bids, before.benchmark_bids);
    assert_eq!(after.direct_profits[0].carried_delta_tenths, 505);
    assert_eq!(after.direct_profits[0].estimated_delta_tenths, 2688);
    assert_eq!(ok(w.save_bytes()), bytes);
    assert_eq!(w.commodity_quantity(CommodityId::GemStones), Some(1));
    assert!(w.navigation_route_snapshot().is_none());
}
#[test]
fn actual_lesson_reserved_food_is_excluded_from_carried_sale_and_capacity_is_still_reserved() {
    let mut w = ok(World::new_commander(29121));
    lesson(&mut w, NavigationTradeAction::Accept);
    lesson(
        &mut w,
        NavigationTradeAction::Plan {
            destination: SystemId::RENAS,
            commodity: CommodityId::Food,
        },
    );
    let empty = preview(&w);
    assert_eq!(empty.used_tonnes, 1);
    assert_eq!(empty.ordinary_quantities[0], 0);
    trade(&mut w, CommodityId::Food);
    let before = ok(w.save_bytes());
    let p = preview(&w);
    assert_eq!(p.ordinary_quantities[0], 1);
    assert_eq!(p.reserved_quantities[0], 1);
    assert_eq!(p.used_tonnes, 1);
    assert_eq!(p.direct_profits[0].carried_delta_tenths, 0);
    assert_eq!(p.paths[0].best_carried_sale_tenths, 0);
    assert_eq!(ok(w.save_bytes()), before);
}
#[test]
fn original_trade_score_can_choose_genuine_extra_stop_but_preview_never_supplies_fuel_or_fitting() {
    let mut w = ok(World::new_commander(29131));
    let review = ok(w.review_xyzzy(XyzzyAction::Enter, 0));
    ok(w.prepare_xyzzy(review, 0)).commit();
    let review = ok(w.review_xyzzy(XyzzyAction::ToggleUnlimitedFuel, 0));
    ok(w.prepare_xyzzy(review, 0)).commit();
    crate::flight::equipment::tests::journey(&mut w, SystemId::RENAS);
    let bytes = ok(w.save_bytes());
    let p = preview(&w);
    assert_eq!(p.direct_profits.len(), 2);
    let trade = some(
        p.paths
            .iter()
            .find(|p| p.labels.contains(&"Trade opportunities")),
    );
    assert_eq!(
        trade.systems,
        [SystemId::RENAS, SystemId::AVENOR, SystemId::LARERIA]
    );
    let fewest = some(
        p.paths
            .iter()
            .find(|p| p.labels.contains(&"Fewest (original distance tie)")),
    );
    assert_eq!(fewest.systems, [SystemId::RENAS, SystemId::LARERIA]);
    assert_eq!(ok(w.save_bytes()), bytes);
    assert_eq!(w.snapshot().navigation.map(|n| n.fuel_units), Some(96));
    assert!(w.navigation_route_snapshot().is_none());
}
#[test]
fn unchanged_navigation_admission_refuses_unknown_hidden_other_galaxy_stale_paused_unfitted_and_flight_previews()
 {
    let mut w = ok(World::new_commander(29141));
    for id in [
        SystemId {
            galaxy: 0,
            system: 3,
        },
        SystemId {
            galaxy: 0,
            system: 256,
        },
        SystemId {
            galaxy: 1,
            system: 7,
        },
        SystemId::AVENOR,
    ] {
        let before = ok(w.save_bytes());
        assert!(w.navigation_trade_preview(id, 0).is_err());
        assert_eq!(ok(w.save_bytes()), before);
    }
    assert!(matches!(
        w.navigation_trade_preview(SystemId::RENAS, 1),
        Err(TradePlanningError::Navigation(
            NavigationRouteError::StaleReview
        ))
    ));
    ok(w.request_pause(true, 0));
    assert!(w.navigation_trade_preview(SystemId::RENAS, 0).is_err());
    ok(w.request_pause(false, 0));
    let item = some(
        some(w.modular_equipment_snapshot())
            .items
            .into_iter()
            .find(|i| i.slot == Some(ModuleSlot::Navigation)),
    )
    .id;
    let review = ok(w.review_module(ModuleAction::Store { item }, 0));
    ok(w.prepare_module(review, 0)).commit();
    assert!(matches!(
        w.navigation_trade_preview(SystemId::RENAS, 0),
        Err(TradePlanningError::Navigation(
            NavigationRouteError::NoComputer
        ))
    ));
    let mut w = ok(World::new_commander(29142));
    ok(w.request_launch(0));
    ok(w.step(FlightInput::default()));
    let bytes = ok(w.save_bytes());
    assert!(
        w.navigation_trade_preview(SystemId::RENAS, w.snapshot().tick)
            .is_err()
    );
    assert_eq!(ok(w.save_bytes()), bytes);
}
#[test]
fn exact_signed_carried_cancellation_and_native_63_storage_cap_are_not_rounded_or_treated_as_tonnes()
 {
    // Pure estimate input only; this modified descriptive copy is never a positive World/profile.
    let w = ok(World::new_commander(29151));
    let mut p = preview(&w);
    p.cash_tenths = 0;
    p.ordinary_quantities[13] = u64::MAX / 17;
    p.ordinary_quantities[14] = u64::MAX / 17;
    p.benchmark_bids[2][13] = p.quotes[0][13].bid_tenths + 1;
    p.benchmark_bids[2][14] = p.quotes[0][14].bid_tenths - 1;
    let d = ok(p.direct_profit(2, 3.4));
    assert_eq!(d.carried_delta_tenths, 0);
    assert_eq!(d.estimated_delta_tenths, 0);
    p.ordinary_quantities = [0; 17];
    p.ordinary_quantities[13] = 63;
    p.cash_tenths = 10_000_000;
    let d = ok(p.direct_profit(2, 3.4));
    assert!(!d.exports.iter().any(|e| e.commodity == CommodityId::Gold));
    assert_eq!(preview(&w).ordinary_quantities, [0; 17]);
}

//! Real public commands and genuine finite commerce. No positive cash/cargo/pose grants.
use super::*;
use crate::freight::g02;
use crate::navigation::SystemId;
use crate::station_services::{Command, CommodityId as Id, TradeSide};
use crate::{FlightInput, ModuleAction, NavigationTradeAction};
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("unexpected refusal: {error:?}"),
    }
}
fn some<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => panic!("missing actual World state"),
    }
}
fn trade(side: TradeSide, id: Id, quantity: u64) -> CommodityTrade {
    CommodityTrade {
        side,
        commodity: id,
        quantity,
    }
}
fn bulk(w: &mut World, side: TradeSide, id: Id, quantity: u64) -> CommodityTradeEvent {
    let tick = w.snapshot().tick;
    ok(w.request_commodity_trade(trade(side, id, quantity), tick))
}
fn refuses(w: &mut World, t: CommodityTrade, error: MissionError) {
    let before = ok(w.save_bytes());
    let tick = w.snapshot().tick;
    assert_eq!(w.request_commodity_trade(t, tick), Err(error));
    assert_eq!(ok(w.save_bytes()), before);
}
fn journey(w: &mut World, to: SystemId) {
    crate::flight::equipment::tests::journey(w, to);
}
fn refuel(w: &mut World) {
    let tick = w.snapshot().tick;
    ok(w.request_refuel(tick));
}
fn lesson(w: &mut World, action: NavigationTradeAction) {
    let tick = w.snapshot().tick;
    let review = ok(w.review_navigation_trade(action, tick));
    ok(w.prepare_navigation_trade(review, tick)).commit();
}
fn cold(w: &World) -> World {
    let bytes = ok(w.save_bytes());
    let actual = ok(World::from_save_bytes(&bytes));
    assert_eq!(actual.credits_tenths(), w.credits_tenths());
    assert_eq!(actual.service_snapshot(), w.service_snapshot());
    assert_eq!(
        actual.mission_snapshot().map(|m| m.freight),
        w.mission_snapshot().map(|m| m.freight)
    );
    assert_eq!(
        actual.navigation_trade_snapshot(),
        w.navigation_trade_snapshot()
    );
    for id in Id::ALL {
        assert_eq!(actual.commodity_quantity(id), w.commodity_quantity(id));
        assert_eq!(actual.commodity_quote(id), w.commodity_quote(id));
    }
    actual
}
fn earned_bulk(seed: u64) -> World {
    let mut w = ok(World::new_commander(seed));
    for (quantity, expected) in [(29, 11434), (47, 18296), (75, 29042)] {
        if some(w.snapshot().navigation).system != SystemId::AVENOR {
            if some(w.snapshot().navigation).fuel_units < 68 {
                refuel(&mut w);
            }
            journey(&mut w, SystemId::AVENOR);
        }
        bulk(&mut w, TradeSide::Buy, Id::Minerals, quantity);
        journey(&mut w, SystemId::RENAS);
        bulk(&mut w, TradeSide::Sell, Id::Minerals, quantity);
        assert_eq!(w.credits_tenths(), Some(expected));
    }
    bulk(&mut w, TradeSide::Buy, Id::Alloys, 48);
    journey(&mut w, SystemId::AVENOR);
    bulk(&mut w, TradeSide::Sell, Id::Alloys, 48);
    assert_eq!(w.credits_tenths(), Some(33650));
    refuel(&mut w);
    bulk(&mut w, TradeSide::Buy, Id::GemStones, 37);
    journey(&mut w, SystemId::RENAS);
    bulk(&mut w, TradeSide::Sell, Id::GemStones, 37);
    assert_eq!(w.credits_tenths(), Some(49541));
    bulk(&mut w, TradeSide::Buy, Id::Alloys, 83);
    journey(&mut w, SystemId::AVENOR);
    bulk(&mut w, TradeSide::Sell, Id::Alloys, 83);
    assert_eq!(w.credits_tenths(), Some(57509));
    bulk(&mut w, TradeSide::Buy, Id::GemStones, 64);
    journey(&mut w, SystemId::RENAS);
    bulk(&mut w, TradeSide::Sell, Id::GemStones, 64);
    assert_eq!(w.credits_tenths(), Some(85349));
    w
}
#[test]
fn actual_bulk_candidate_cancels_failed_publication_and_cold_commit_without_partial_live_changes() {
    let mut w = ok(World::new_commander(15101));
    let before = ok(w.save_bytes());
    let prepared = ok(w.prepare_commodity_trade(trade(TradeSide::Buy, Id::Minerals, 29), 0));
    assert_eq!(
        (
            prepared.event().quantity,
            prepared.event().unit_price_tenths,
            prepared.event().gross_tenths,
            prepared.event().paid_tenths
        ),
        (29, 241, 6989, 6989)
    );
    let staged = ok(World::from_save_bytes(prepared.bytes()));
    assert_eq!(staged.commodity_quantity(Id::Minerals), Some(29));
    assert_eq!(staged.credits_tenths(), Some(211));
    drop(prepared);
    assert_eq!(ok(w.save_bytes()), before);
    let prepared = ok(w.prepare_commodity_trade(trade(TradeSide::Buy, Id::Minerals, 29), 0));
    assert_eq!(
        prepared.publish_with(|_| Err::<(), _>("publication failed")),
        Err("publication failed")
    );
    assert_eq!(ok(w.save_bytes()), before);
    let mut published = Vec::new();
    let prepared = ok(w.prepare_commodity_trade(trade(TradeSide::Buy, Id::Minerals, 29), 0));
    let event = ok(prepared.publish_with(|bytes| {
        published.extend_from_slice(bytes);
        Ok::<(), ()>(())
    }));
    assert_eq!(event.paid_tenths, 6989);
    assert_eq!(ok(w.save_bytes()), published);
    let mut singles = ok(World::new_commander(15101));
    for _ in 0..29 {
        ok(singles.request_station_service(Command::BuyCommodity(Id::Minerals), 0));
    }
    assert_eq!(ok(w.save_bytes()), ok(singles.save_bytes()));
    let _continued = cold(&w);
}
#[test]
fn real_bulk_profit_funds_genuine_paid_tool_and_native_export_law_without_extra_rewards() {
    let mut w = earned_bulk(15201);
    let singles = crate::flight::equipment::tests::earned_world(15201);
    assert_eq!(ok(w.save_bytes()), ok(singles.save_bytes()));
    if some(w.snapshot().navigation).fuel_units < 34 {
        refuel(&mut w);
    }
    journey(&mut w, SystemId::AVENOR);
    let cash = some(w.credits_tenths());
    let tick = w.snapshot().tick;
    let review = ok(w.review_module(ModuleAction::PurchaseMiningLaser, tick));
    let item = ok(w.prepare_module(review, tick)).commit();
    assert_eq!(some(w.credits_tenths()), cash - 33_000);
    assert_eq!(some(w.fitted_mining_tool()).item, item);
    for (id, quantity) in [(Id::Slaves, 3), (Id::Narcotics, 2), (Id::Weapons, 2)] {
        bulk(&mut w, TradeSide::Buy, id, quantity);
    }
    assert_eq!(w.legal_value(), Some(0));
    let tick = w.snapshot().tick;
    assert!(w.request_launch(tick - 1).is_err());
    assert_eq!(w.legal_value(), Some(0));
    ok(w.request_pause(true, tick));
    assert!(w.request_launch(tick).is_err());
    ok(w.request_pause(false, tick));
    ok(w.request_launch(tick));
    assert_eq!(w.legal_value(), Some(9));
    let loaded = cold(&w);
    assert_eq!(loaded.legal_value(), Some(9));
}
#[test]
fn current_tick_pause_and_real_airborne_berth_admission_guard_every_bulk_native_row() {
    let mut w = ok(World::new_commander(15301));
    ok(w.step(FlightInput::default()));
    let before = ok(w.save_bytes());
    for id in Id::ALL {
        assert_eq!(
            w.request_commodity_trade(trade(TradeSide::Buy, id, 2), 0),
            Err(MissionError::StaleCommand)
        );
        assert_eq!(ok(w.save_bytes()), before);
    }
    ok(w.request_pause(true, 1));
    let before = ok(w.save_bytes());
    for id in Id::ALL {
        assert_eq!(
            w.request_commodity_trade(trade(TradeSide::Buy, id, 2), 1),
            Err(MissionError::Paused)
        );
        assert_eq!(ok(w.save_bytes()), before);
    }
    ok(w.request_pause(false, 1));
    ok(w.request_launch(1));
    let before = ok(w.save_bytes());
    for id in Id::ALL {
        assert_eq!(
            w.request_commodity_trade(trade(TradeSide::Sell, id, 2), 1),
            Err(MissionError::NotDocked)
        );
        assert_eq!(ok(w.save_bytes()), before);
    }
}
#[test]
fn actual_zero_stock_funds_and_unsellable_batch_refusals_keep_identical_complete_save_bytes() {
    let mut w = ok(World::new_commander(15401));
    for (t, error) in [
        (trade(TradeSide::Buy, Id::Minerals, 0), Error::ZeroQuantity),
        (
            trade(TradeSide::Buy, Id::Gold, u64::MAX),
            Error::QuantityLimit,
        ),
        (trade(TradeSide::Buy, Id::Gold, 2), Error::InsufficientFunds),
        (
            trade(
                TradeSide::Sell,
                Id::Gold,
                crate::station_services::MAX_MARKET_QUANTITY,
            ),
            Error::PriceOverflow,
        ),
        (trade(TradeSide::Buy, Id::AlienItems, 2), Error::OutOfStock),
        (trade(TradeSide::Buy, Id::Minerals, 218), Error::OutOfStock),
        (trade(TradeSide::Sell, Id::Food, 2), Error::NoSellableCargo),
    ] {
        refuses(&mut w, t, MissionError::Service(error));
    }
    bulk(&mut w, TradeSide::Buy, Id::Minerals, 2);
    refuses(
        &mut w,
        trade(TradeSide::Sell, Id::Minerals, 3),
        MissionError::Service(Error::NoSellableCargo),
    );
    assert_eq!(w.commodity_quantity(Id::Minerals), Some(2));
    let _continued = cold(&w);
}
#[test]
fn t04_keeps_single_identified_trade_receipts_and_protects_held_tonne_from_bulk_sales() {
    let mut w = ok(World::new_commander(15501));
    lesson(&mut w, NavigationTradeAction::Accept);
    lesson(
        &mut w,
        NavigationTradeAction::Plan {
            destination: SystemId::RENAS,
            commodity: Id::Minerals,
        },
    );
    refuses(
        &mut w,
        trade(TradeSide::Buy, Id::Minerals, 2),
        MissionError::NavigationTrade(NavigationTradeError::SingleUnitRequired),
    );
    bulk(&mut w, TradeSide::Buy, Id::Minerals, 1);
    let buy = some(some(w.navigation_trade_snapshot()).attempts[0].buy);
    assert_eq!((buy.cargo_before, buy.cargo_after), (0, 1));
    assert_eq!(w.reserved_commodity_quantity(Id::Minerals), Some(1));
    bulk(&mut w, TradeSide::Buy, Id::Minerals, 2);
    refuses(
        &mut w,
        trade(TradeSide::Sell, Id::Minerals, 3),
        MissionError::Service(Error::NoSellableCargo),
    );
    bulk(&mut w, TradeSide::Sell, Id::Minerals, 2);
    assert_eq!(w.commodity_quantity(Id::Minerals), Some(1));
    assert_eq!(
        some(w.navigation_trade_snapshot()).attempts[0].buy,
        Some(buy)
    );
    w = cold(&w);
    journey(&mut w, SystemId::RENAS);
    refuses(
        &mut w,
        trade(TradeSide::Sell, Id::Minerals, 2),
        MissionError::NavigationTrade(NavigationTradeError::SingleUnitRequired),
    );
    let cash = some(w.credits_tenths());
    let sale = bulk(&mut w, TradeSide::Sell, Id::Minerals, 1);
    assert_eq!(some(w.credits_tenths()), cash + sale.credited_tenths);
    let actual = some(some(w.navigation_trade_snapshot()).attempts[0].sale);
    assert_eq!(actual.tonne, buy.tonne);
    assert_eq!(w.reserved_commodity_quantity(Id::Minerals), Some(0));
    let _continued = cold(&w);
}
#[test]
fn actual_reserved_capacity_and_full_hold_preserve_tonne_vs_kg_and_gram_rules() {
    let mut w = earned_bulk(15601);
    if some(w.snapshot().navigation).fuel_units < 34 {
        refuel(&mut w);
    }
    journey(&mut w, SystemId::AVENOR);
    lesson(&mut w, NavigationTradeAction::Accept);
    lesson(
        &mut w,
        NavigationTradeAction::Plan {
            destination: SystemId::RENAS,
            commodity: Id::Minerals,
        },
    );
    refuses(
        &mut w,
        trade(TradeSide::Buy, Id::Food, 92),
        MissionError::Service(Error::HoldFull),
    );
    bulk(&mut w, TradeSide::Buy, Id::Food, 91);
    assert_eq!(some(w.service_snapshot()).used_tonnes, 92);
    for id in [Id::Gold, Id::Platinum, Id::GemStones] {
        bulk(&mut w, TradeSide::Buy, id, 2);
        assert_eq!(some(w.service_snapshot()).used_tonnes, 92);
    }
    assert_eq!(
        some(w.navigation_trade_snapshot()).reserved_capacity_tonnes,
        1
    );
    let _continued = cold(&w);
    refuses(
        &mut w,
        trade(TradeSide::Buy, Id::Alloys, 2),
        MissionError::Service(Error::HoldFull),
    );
}
#[test]
fn ordinary_bulk_machinery_never_replaces_g02_ids_or_changes_its_eight_receipts_and_once_reward() {
    let mut w = ok(World::new_commander(15701));
    let npc = w.station_contacts()[0];
    ok(w.request_contact(npc, 0));
    let offer = g02::Command::ReviewOffer { actor: npc.actor() };
    let review = match ok(w.request_freight(offer, 0)) {
        g02::Event::Reviewed(review) => review,
        event => panic!("expected review, got {event:?}"),
    };
    bulk(&mut w, TradeSide::Buy, Id::Minerals, 2);
    let before = ok(w.save_bytes());
    assert_eq!(
        w.request_freight(
            g02::Command::Confirm {
                review,
                choice: g02::Choice::Accept
            },
            0
        ),
        Err(MissionError::Freight(g02::Error::StaleReview))
    );
    assert_eq!(ok(w.save_bytes()), before);
    let review = match ok(w.request_freight(offer, 0)) {
        g02::Event::Reviewed(review) => review,
        event => panic!("expected review, got {event:?}"),
    };
    let (job, units) = match ok(w.request_freight(
        g02::Command::Confirm {
            review,
            choice: g02::Choice::Accept,
        },
        0,
    )) {
        g02::Event::Accepted { job, units } => (job, units),
        event => panic!("expected actual allocation, got {event:?}"),
    };
    let freight = some(w.mission_snapshot()).freight;
    bulk(&mut w, TradeSide::Buy, Id::Machinery, 2);
    refuses(
        &mut w,
        trade(TradeSide::Sell, Id::Machinery, 3),
        MissionError::Service(Error::NoSellableCargo),
    );
    assert_eq!(some(w.mission_snapshot()).freight, freight);
    w = cold(&w);
    journey(&mut w, SystemId::RENAS);
    let npc = w.station_contacts()[0];
    let tick = w.snapshot().tick;
    ok(w.request_contact(npc, tick));
    for (i, unit) in units.into_iter().enumerate() {
        let review = match ok(w.request_freight(
            g02::Command::ReviewReceiving {
                actor: npc.actor(),
                job,
            },
            tick,
        )) {
            g02::Event::Reviewed(review) => review,
            event => panic!("expected receiving review, got {event:?}"),
        };
        assert_eq!(review.next_unit(), Some(unit));
        let cash = some(w.credits_tenths());
        ok(w.request_freight(
            g02::Command::Confirm {
                review,
                choice: g02::Choice::ReceiveOne(unit),
            },
            tick,
        ));
        assert_eq!(
            some(w.credits_tenths()),
            cash + if i == 7 { 7300 } else { 0 }
        );
    }
    assert_eq!(some(w.mission_snapshot()).freight.held_tonnes, 0);
    assert_eq!(w.commodity_quantity(Id::Machinery), Some(2));
    bulk(&mut w, TradeSide::Sell, Id::Machinery, 2);
    refuses(
        &mut w,
        trade(TradeSide::Sell, Id::Machinery, 1),
        MissionError::Service(Error::NoSellableCargo),
    );
    let loaded = cold(&w);
    assert_eq!(some(loaded.mission_snapshot()).freight.completed_jobs, 1);
}

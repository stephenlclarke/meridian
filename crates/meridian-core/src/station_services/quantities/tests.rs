//! Arithmetic/transaction component fixtures only; no World ownership or gameplay grants.
use super::*;
use crate::station_services::{Change, Command};
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("unexpected refusal: {error:?}"),
    }
}
fn account() -> AccountProjection {
    AccountProjection {
        owner: 101,
        hull: 102,
        revision: 0,
        credits_tenths: u32::MAX / 2,
        fuel_tenths: 130,
        food_tonnes: 0,
        reserved_food_tonnes: 0,
        used_tonnes: 0,
        hold_capacity_tonnes: 92,
        other_commodities: [0; 16],
        reserved_commodities: [0; 16],
    }
}
fn facts() -> ServiceFacts {
    ServiceFacts {
        owner: 101,
        hull: 102,
        site: MarketSite::Avenor,
        station_entity: 1,
        visit: 1,
        dock_sequence: 1,
        epoch: 1,
        paused: false,
    }
}
fn trade(side: TradeSide, commodity: CommodityId, quantity: u64) -> CommodityTrade {
    CommodityTrade {
        side,
        commodity,
        quantity,
    }
}
fn change(
    market: &StationMarkets,
    account: AccountProjection,
    facts: ServiceFacts,
    trade: CommodityTrade,
) -> Change<CommodityTradeEvent> {
    ok(
        ok(market.prepare_commodity_trade(account, Some(facts), trade)).checked_change(
            market,
            account,
            Some(facts),
        ),
    )
}
fn refuses(
    market: &StationMarkets,
    account: AccountProjection,
    trade: CommodityTrade,
    error: Error,
) {
    let before = market.data;
    assert!(
        matches!(market.prepare_commodity_trade(account,Some(facts()),trade),Err(actual) if actual==error)
    );
    assert_eq!(market.data, before);
}

#[test]
fn every_available_native_row_matches_the_complete_state_of_repeated_single_trades() {
    for site in [MarketSite::Avenor, MarketSite::Renas] {
        for id in CommodityId::ALL {
            let mut berth = facts();
            berth.site = site;
            let market = ok(StationMarkets::new_commander(101));
            let before = account();
            let quote = market.commodity_quote(site, id);
            if quote.stock == 0 {
                refuses(
                    &market,
                    before,
                    trade(TradeSide::Buy, id, 2),
                    Error::OutOfStock,
                );
                continue;
            }
            let quantity = quote.stock.min(3);
            let bulk = change(&market, before, berth, trade(TradeSide::Buy, id, quantity));
            let mut singles = ok(StationMarkets::new_commander(101));
            let mut p = before;
            for _ in 0..quantity {
                let next =
                    ok(
                        ok(singles.prepare(p, Some(berth), Command::BuyCommodity(id)))
                            .checked_change(&singles, p, Some(berth)),
                    );
                p = next.account_after;
                singles = next.market;
            }
            assert_eq!(bulk.account_after, p);
            assert_eq!(bulk.market.data, singles.data);
            assert_eq!(
                bulk.event.gross_tenths,
                quantity * u64::from(quote.ask_tenths)
            );
            assert_eq!(u64::from(bulk.event.paid_tenths), bulk.event.gross_tenths);
            let sold = change(
                &bulk.market,
                bulk.account_after,
                berth,
                trade(TradeSide::Sell, id, quantity),
            );
            for _ in 0..quantity {
                let next =
                    ok(
                        ok(singles.prepare(p, Some(berth), Command::SellCommodity(id)))
                            .checked_change(&singles, p, Some(berth)),
                    );
                p = next.account_after;
                singles = next.market;
            }
            assert_eq!(sold.account_after, p);
            assert_eq!(sold.market.data, singles.data);
            assert_eq!(
                sold.event.gross_tenths,
                quantity * u64::from(quote.bid_tenths)
            );
            assert_eq!(sold.market.commodity_quote(site, id), quote);
            assert!(sold.market.validate_save(101, 92, 92).is_ok());
        }
    }
}
#[test]
fn quantity_one_retains_all_original_food_and_other_events_and_ledger_bytes() {
    let berth = facts();
    for id in CommodityId::ALL
        .into_iter()
        .filter(|id| *id != CommodityId::AlienItems)
    {
        let market = ok(StationMarkets::new_commander(101));
        let p = account();
        let single = ok(
            ok(market.prepare(p, Some(berth), Command::BuyCommodity(id))).checked_change(
                &market,
                p,
                Some(berth),
            ),
        );
        let batch = change(&market, p, berth, trade(TradeSide::Buy, id, 1));
        assert_eq!(ok(batch.event.single_event()), single.event);
        assert_eq!(batch.account_after, single.account_after);
        assert_eq!(batch.market.data, single.market.data);
        let p = batch.account_after;
        let market = batch.market;
        let single = ok(
            ok(market.prepare(p, Some(berth), Command::SellCommodity(id))).checked_change(
                &market,
                p,
                Some(berth),
            ),
        );
        let batch = change(&market, p, berth, trade(TradeSide::Sell, id, 1));
        assert_eq!(ok(batch.event.single_event()), single.event);
        assert_eq!(batch.account_after, single.account_after);
        assert_eq!(batch.market.data, single.market.data);
    }
}
#[test]
fn exact_funds_stock_and_hold_limits_are_all_or_nothing_and_not_a_partial_purchase() {
    let market = ok(StationMarkets::new_commander(101));
    let id = CommodityId::Minerals;
    let ask = market.commodity_quote(MarketSite::Avenor, id).ask_tenths;
    let mut p = account();
    p.credits_tenths = ask * 2 - 1;
    refuses(
        &market,
        p,
        trade(TradeSide::Buy, id, 2),
        Error::InsufficientFunds,
    );
    p.credits_tenths = ask * 2;
    let bought = change(&market, p, facts(), trade(TradeSide::Buy, id, 2));
    assert_eq!(bought.account_after.credits_tenths, 0);
    let mut full = account();
    full.food_tonnes = 91;
    full.used_tonnes = 91;
    refuses(&market, full, trade(TradeSide::Buy, id, 2), Error::HoldFull);
    let stock = market.commodity_quote(MarketSite::Avenor, id).stock;
    refuses(
        &market,
        account(),
        trade(TradeSide::Buy, id, stock + 1),
        Error::OutOfStock,
    );
    let mut p = account();
    p.food_tonnes = 92;
    p.used_tonnes = 92;
    for id in [
        CommodityId::Gold,
        CommodityId::Platinum,
        CommodityId::GemStones,
    ] {
        let next = change(&market, p, facts(), trade(TradeSide::Buy, id, 3));
        assert_eq!(next.account_after.used_tonnes, 92);
        assert_eq!(next.account_after.other_commodities[id.index() - 1], 3);
    }
}
#[test]
fn protected_native_units_are_never_sold_with_unreserved_units_or_as_a_partial_batch() {
    let market = ok(StationMarkets::new_commander(101));
    for id in CommodityId::ALL {
        let mut p = account();
        if id == CommodityId::Food {
            p.food_tonnes = 5;
            p.reserved_food_tonnes = 2;
        } else {
            p.other_commodities[id.index() - 1] = 5;
            p.reserved_commodities[id.index() - 1] = 2;
        }
        p.used_tonnes = if id.unit() == CargoUnit::Tonne { 5 } else { 0 };
        refuses(
            &market,
            p,
            trade(TradeSide::Sell, id, 4),
            Error::NoSellableCargo,
        );
        let sold = change(&market, p, facts(), trade(TradeSide::Sell, id, 3));
        assert_eq!(sold.event.quantity, 3);
        refuses(
            &sold.market,
            sold.account_after,
            trade(TradeSide::Sell, id, 1),
            Error::NoSellableCargo,
        );
    }
}
#[test]
fn zero_quantity_native_bounds_price_counter_and_stock_overflow_refuse_without_mutation() {
    let mut market = ok(StationMarkets::new_commander(101));
    let id = CommodityId::Gold;
    refuses(
        &market,
        account(),
        trade(TradeSide::Buy, id, 0),
        Error::ZeroQuantity,
    );
    refuses(
        &market,
        account(),
        trade(TradeSide::Buy, id, u64::MAX),
        Error::QuantityLimit,
    );
    refuses(
        &market,
        account(),
        trade(TradeSide::Sell, id, MAX_MARKET_QUANTITY),
        Error::PriceOverflow,
    );
    let mut p = account();
    p.other_commodities[id.index() - 1] = MAX_MARKET_QUANTITY;
    refuses(
        &market,
        p,
        trade(TradeSide::Buy, id, 2),
        Error::QuantityLimit,
    );
    let mut p = account();
    p.revision = u64::MAX - 1;
    refuses(
        &market,
        p,
        trade(TradeSide::Buy, id, 2),
        Error::SequenceExhausted,
    );
    market.data.revision = u64::MAX - 1;
    refuses(
        &market,
        account(),
        trade(TradeSide::Buy, id, 2),
        Error::SequenceExhausted,
    );
    market.data.revision = 0;
    market.data.paid_tenths = u64::MAX;
    refuses(
        &market,
        account(),
        trade(TradeSide::Buy, id, 2),
        Error::SequenceExhausted,
    );
    market.data.paid_tenths = 0;
    market.data.other_bought[0][id.index() - 1] = u64::MAX;
    refuses(
        &market,
        account(),
        trade(TradeSide::Buy, id, 2),
        Error::SequenceExhausted,
    );
    market.data.other_bought[0][id.index() - 1] = 0;
    p = account();
    p.other_commodities[id.index() - 1] = 2;
    market.data.credited_tenths = u64::MAX;
    refuses(
        &market,
        p,
        trade(TradeSide::Sell, id, 2),
        Error::SequenceExhausted,
    );
    market.data.credited_tenths = 0;
    market.data.other_sold[0][id.index() - 1] = u64::MAX;
    refuses(
        &market,
        p,
        trade(TradeSide::Sell, id, 2),
        Error::SequenceExhausted,
    );
    market.data.other_sold[0][id.index() - 1] = 0;
    p = account();
    p.other_commodities[id.index() - 1] = 2;
    market.data.other_quotes[0][id.index() - 1].stock = MAX_MARKET_QUANTITY - 1;
    refuses(
        &market,
        p,
        trade(TradeSide::Sell, id, 2),
        Error::QuantityLimit,
    );
}
#[test]
fn ordinary_sale_preserves_cash_saturation_and_reports_exact_gross_and_actual_credit() {
    let market = ok(StationMarkets::new_commander(101));
    let id = CommodityId::Gold;
    let mut p = account();
    p.credits_tenths = u32::MAX - 100;
    p.other_commodities[id.index() - 1] = 3;
    let sold = change(&market, p, facts(), trade(TradeSide::Sell, id, 3));
    assert_eq!(sold.event.unit_price_tenths, 4009);
    assert_eq!(sold.event.gross_tenths, 12027);
    assert_eq!(sold.event.credited_tenths, 100);
    assert_eq!(sold.account_after.credits_tenths, u32::MAX);
    assert_eq!(sold.market.data.credited_tenths, 100);
    let mut huge = account();
    huge.other_commodities[id.index() - 1] = 1_000_000_000_000;
    let sold = change(
        &market,
        huge,
        facts(),
        trade(TradeSide::Sell, id, 1_000_000_000_000),
    );
    assert_eq!(sold.event.quantity, 1_000_000_000_000);
    assert_eq!(sold.account_after.other_commodities[id.index() - 1], 0);
    assert_eq!(sold.market.data.revision, 1_000_000_000_000);
}
#[test]
fn prospective_quantity_change_rechecks_complete_account_market_and_actual_berth() {
    let market = ok(StationMarkets::new_commander(101));
    let p = account();
    let berth = facts();
    let t = trade(TradeSide::Buy, CommodityId::Minerals, 3);
    let plan = ok(market.prepare_commodity_trade(p, Some(berth), t));
    let mut changed = p;
    changed.credits_tenths -= 1;
    assert!(matches!(
        plan.checked_change(&market, changed, Some(berth)),
        Err(Error::StalePublication)
    ));
    let plan = ok(market.prepare_commodity_trade(p, Some(berth), t));
    let mut changed = berth;
    changed.epoch += 1;
    assert!(matches!(
        plan.checked_change(&market, p, Some(changed)),
        Err(Error::StalePublication)
    ));
    let plan = ok(market.prepare_commodity_trade(p, Some(berth), t));
    let changed = change(&market, p, berth, t);
    assert!(matches!(
        plan.checked_change(&changed.market, p, Some(berth)),
        Err(Error::StalePublication)
    ));
    assert_eq!(
        market
            .commodity_quote(MarketSite::Avenor, CommodityId::Minerals)
            .stock,
        217
    );
}
#[test]
fn quantity_preparation_uses_existing_owner_and_paused_or_missing_berth_guards() {
    let market = ok(StationMarkets::new_commander(101));
    let p = account();
    let t = trade(TradeSide::Buy, CommodityId::Minerals, 3);
    assert!(matches!(
        market.prepare_commodity_trade(p, None, t),
        Err(Error::NotDocked)
    ));
    let mut berth = facts();
    berth.paused = true;
    assert!(matches!(
        market.prepare_commodity_trade(p, Some(berth), t),
        Err(Error::Paused)
    ));
    let mut berth = facts();
    berth.owner = 201;
    assert!(matches!(
        market.prepare_commodity_trade(p, Some(berth), t),
        Err(Error::WrongOwner)
    ));
}

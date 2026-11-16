#![allow(clippy::unwrap_used)]
use super::{
    AccountProjection, CargoUnit, Command, CommodityId, Error, Event, MAX_MARKET_QUANTITY,
    MarketSite, ServiceFacts, StationMarkets,
};
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
fn facts(site: MarketSite) -> ServiceFacts {
    ServiceFacts {
        owner: 101,
        hull: 102,
        site,
        station_entity: 1,
        visit: 1,
        dock_sequence: 1,
        epoch: 0,
        paused: false,
    }
}
#[test]
fn original_seventeen_quotes_and_each_native_purchase_sale_reconcile() {
    let expected = [
        [
            (226, 208, 128),
            (347, 319, 128),
            (5142, 4746, 12),
            (3811, 3517, 4),
            (708, 652, 72),
            (9145, 8441, 14),
            (5661, 5225, 9),
            (3613, 3335, 23),
            (1315, 1213, 54),
            (747, 689, 92),
            (4414, 4074, 18),
            (1193, 1101, 35),
            (241, 221, 217),
            (4345, 4009, 47),
            (7605, 7019, 21),
            (894, 824, 106),
            (41338, 38158, 0),
        ],
        [
            (216, 196, 112),
            (346, 316, 163),
            (5418, 4950, 15),
            (3866, 3532, 5),
            (787, 719, 64),
            (7879, 7199, 12),
            (5072, 4634, 12),
            (3438, 3140, 26),
            (1375, 1255, 58),
            (593, 541, 135),
            (4269, 3901, 19),
            (1102, 1006, 38),
            (425, 387, 47),
            (6192, 5658, 14),
            (9032, 8254, 8),
            (1455, 1329, 32),
            (37041, 33849, 0),
        ],
    ];
    for site in [MarketSite::Avenor, MarketSite::Renas] {
        for id in CommodityId::ALL {
            let mut market = StationMarkets::new_commander(101).unwrap();
            let quote = market.commodity_quote(site, id);
            assert_eq!(
                (quote.ask_tenths, quote.bid_tenths, quote.stock),
                expected[site.index()][id.index()]
            );
            let definition = id.definition().unwrap();
            assert_eq!((definition.id, definition.unit), (id, id.unit()));
            let before = account();
            if id == CommodityId::AlienItems {
                assert_eq!(definition.target_stock, 4);
                assert_eq!(quote.target_stock, 0);
                assert!(matches!(
                    market.prepare(before, Some(facts(site)), Command::BuyCommodity(id)),
                    Err(Error::OutOfStock)
                ));
                continue;
            }
            let bought = market
                .prepare(before, Some(facts(site)), Command::BuyCommodity(id))
                .unwrap()
                .checked_change(&market, before, Some(facts(site)))
                .unwrap();
            let after = bought.account_after;
            assert_eq!(
                after.credits_tenths,
                before.credits_tenths - quote.ask_tenths
            );
            assert_eq!(after.used_tonnes, u16::from(id.unit() == CargoUnit::Tonne));
            market = bought.market;
            assert_eq!(market.commodity_quote(site, id).stock, quote.stock - 1);
            let sold = market
                .prepare(after, Some(facts(site)), Command::SellCommodity(id))
                .unwrap()
                .checked_change(&market, after, Some(facts(site)))
                .unwrap();
            assert_eq!(
                sold.account_after.credits_tenths,
                before.credits_tenths - quote.ask_tenths + quote.bid_tenths
            );
            assert_eq!(sold.account_after.used_tonnes, 0);
            assert_eq!(sold.market.commodity_quote(site, id), quote);
            let summary = sold.market.validate_save(101, 92, 92).unwrap();
            assert_eq!(summary.other_commodities, [0; 16]);
            assert_eq!(summary.food, 0);
            assert_eq!(summary.paid, u64::from(quote.ask_tenths));
            assert_eq!(summary.credited, u64::from(quote.bid_tenths));
        }
    }
    assert_eq!(CommodityId::from_index(17), None);
    assert_eq!(CommodityId::Weapons.definition().unwrap().name, "Weapons");
    assert_eq!(
        CommodityId::GemStones.definition().unwrap().name,
        "Gem-Stones"
    );
    assert_eq!(
        CommodityId::display_order()
            .unwrap()
            .map(CommodityId::index),
        [9, 8, 10, 2, 7, 12, 13, 15, 14, 1, 4, 5, 0, 11, 6, 3, 16]
    );
}
#[test]
fn full_tonne_hold_still_accepts_kg_and_grams_without_conversion() {
    let mut market = StationMarkets::new_commander(101).unwrap();
    let mut account = account();
    account.food_tonnes = 92;
    account.used_tonnes = 92;
    let berth = Some(facts(MarketSite::Avenor));
    let before = market.data;
    assert!(matches!(
        market.prepare(
            account,
            berth,
            Command::BuyCommodity(CommodityId::Machinery)
        ),
        Err(Error::HoldFull)
    ));
    assert_eq!(market.data, before);
    for id in [
        CommodityId::Gold,
        CommodityId::Platinum,
        CommodityId::GemStones,
    ] {
        let change = market
            .prepare(account, berth, Command::BuyCommodity(id))
            .unwrap()
            .checked_change(&market, account, berth)
            .unwrap();
        account = change.account_after;
        market = change.market;
        assert_eq!(account.used_tonnes, 92);
        assert_eq!(account.other_commodities[id.index() - 1], 1);
    }
    account.other_commodities[CommodityId::Gold.index() - 1] = MAX_MARKET_QUANTITY;
    assert!(matches!(
        market.prepare(account, berth, Command::BuyCommodity(CommodityId::Gold)),
        Err(Error::QuantityLimit)
    ));
}
#[test]
fn protected_ordinary_units_and_failed_checks_leave_every_market_row_unchanged() {
    let mut market = StationMarkets::new_commander(101).unwrap();
    let berth = Some(facts(MarketSite::Avenor));
    for id in CommodityId::ALL.into_iter().skip(1) {
        let mut account = account();
        account.other_commodities[id.index() - 1] = 1;
        account.reserved_commodities[id.index() - 1] = 1;
        account.used_tonnes = u16::from(id.unit() == CargoUnit::Tonne);
        let before = market.data;
        assert!(matches!(
            market.prepare(account, berth, Command::SellCommodity(id)),
            Err(Error::NoSellableCargo)
        ));
        assert_eq!(market.data, before);
    }
    let mut poor = account();
    poor.credits_tenths = 0;
    let before = market.data;
    assert!(matches!(
        market.prepare(poor, berth, Command::BuyCommodity(CommodityId::Machinery)),
        Err(Error::InsufficientFunds)
    ));
    assert_eq!(market.data, before);
    let mut loaded = account();
    loaded.other_commodities[7] = 1;
    loaded.used_tonnes = 1;
    market.data.other_quotes[0][7].stock = MAX_MARKET_QUANTITY;
    let before = market.data;
    assert!(matches!(
        market.prepare(
            loaded,
            berth,
            Command::SellCommodity(CommodityId::Machinery)
        ),
        Err(Error::QuantityLimit)
    ));
    assert_eq!(market.data, before);
    market.data.other_bought[0][7] = u64::MAX;
    let before = market.data;
    assert!(matches!(
        market.prepare(
            account(),
            berth,
            Command::BuyCommodity(CommodityId::Machinery)
        ),
        Err(Error::SequenceExhausted)
    ));
    assert_eq!(market.data, before);
}
#[test]
fn complete_native_account_binding_revokes_prepared_trade_and_sale_saturates() {
    let market = StationMarkets::new_commander(101).unwrap();
    let berth = Some(facts(MarketSite::Avenor));
    let mut account = account();
    let prepared = market
        .prepare(account, berth, Command::BuyCommodity(CommodityId::Gold))
        .unwrap();
    account.other_commodities[14] = 1;
    assert!(matches!(
        prepared.checked_change(&market, account, berth),
        Err(Error::StalePublication)
    ));
    account.other_commodities[12] = 1;
    account.credits_tenths = u32::MAX - 3;
    let sold = market
        .prepare(account, berth, Command::SellCommodity(CommodityId::Gold))
        .unwrap()
        .checked_change(&market, account, berth)
        .unwrap();
    assert_eq!(sold.account_after.credits_tenths, u32::MAX);
    assert!(matches!(
        sold.event,
        Event::SoldCommodity {
            credited_tenths: 3,
            ..
        }
    ));
}

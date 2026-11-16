//! Account/market component conservation. These fixtures grant no World mining-tool capability.
use super::*;
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("unexpected refusal: {error:?}"),
    }
}
fn born() -> (CommanderAccount, State) {
    (
        ok(CommanderAccount::born_sh025(FreshCommanderIds {
            owner: 101,
            hull: 102,
            inventory: 103,
            items: core::array::from_fn(|i| 104 + i as u64),
        })),
        State::new_commander(101),
    )
}
#[test]
fn mined_native_units_join_only_existing_cargo_and_revision_with_no_credit_fuel_or_fit_grant() {
    let (account, freight) = born();
    let before = account.data;
    for id in [
        services::CommodityId::Minerals,
        services::CommodityId::Alloys,
        services::CommodityId::Gold,
        services::CommodityId::Platinum,
    ] {
        let candidate =
            ok(account.prepare_mining_yield(&freight, id, services::Reservations::default()));
        let mut expected = before;
        expected.other_commodities[id.index() - 1] += 1;
        expected.revision += 1;
        assert_eq!(candidate.data, expected);
        assert_eq!(account.data, before);
        let projection = ok(candidate.service_projection(&freight));
        assert_eq!(
            projection.used_tonnes,
            u16::from(id.unit() == services::CargoUnit::Tonne)
        );
    }
}
#[test]
fn actual_hold_and_tutorial_reservations_refuse_before_yield_without_mutating_the_account() {
    let (mut account, freight) = born();
    account.data.food_tonnes = 92;
    let before = account.data;
    assert_eq!(
        account.prepare_mining_yield(
            &freight,
            services::CommodityId::Minerals,
            services::Reservations::default()
        ),
        Err(services::Error::HoldFull)
    );
    assert_eq!(account.data, before);
    account.data.food_tonnes = 91;
    let before = account.data;
    assert_eq!(
        account.prepare_mining_yield(
            &freight,
            services::CommodityId::Minerals,
            services::Reservations {
                capacity: 1,
                commodity: None
            }
        ),
        Err(services::Error::HoldFull)
    );
    assert!(
        account
            .prepare_mining_yield(
                &freight,
                services::CommodityId::Minerals,
                services::Reservations::default()
            )
            .is_ok()
    );
    assert_eq!(account.data, before);
    assert_eq!(
        account.prepare_mining_yield(
            &freight,
            services::CommodityId::Food,
            services::Reservations::default()
        ),
        Err(services::Error::InvalidAccount)
    );
}
#[test]
fn selling_a_mined_native_unit_requires_matching_irreversible_yield_in_market_conservation() {
    let (account, freight) = born();
    let id = services::CommodityId::Gold;
    let mined = ok(account.prepare_mining_yield(&freight, id, services::Reservations::default()));
    let markets = ok(services::StationMarkets::new_commander(101));
    let facts = services::ServiceFacts {
        owner: 101,
        hull: 102,
        site: services::MarketSite::Avenor,
        station_entity: 1,
        visit: 1,
        dock_sequence: 1,
        epoch: 1,
        paused: false,
    };
    let (sold, market, event) = ok(mined.prepare_station_service(
        &freight,
        &markets,
        facts,
        services::Command::SellCommodity(id),
        services::Reservations::default(),
    ));
    assert!(matches!(event,services::Event::SoldCommodity{commodity,..} if commodity==id));
    assert_eq!(sold.data.other_commodities[id.index() - 1], 0);
    assert!(market.validate_save(101, 92, 92).is_err());
    let mut quantities = [0; 16];
    quantities[id.index() - 1] = 1;
    let summary = ok(market.validate_save_with_mining(101, 92, 92, quantities));
    assert_eq!(summary.other_commodities[id.index() - 1], 0);
    assert!(sold.data.credits > account.data.credits);
    assert_eq!(
        sold.data.credits,
        account.data.credits
            + u32::try_from(summary.credited)
                .unwrap_or_else(|_| panic!("fixture proceeds overflow"))
    );
    assert_eq!(sold.data.fit, account.data.fit);
    assert_eq!(sold.data.fuel_tenths, account.data.fuel_tenths);
}

#![allow(clippy::unwrap_used)]
use super::*;
use crate::FlightInput;
use crate::flight::equipment::tests::journey;
use crate::navigation::{BASIC_SPOOL_TICKS, NavigationError, OriginalNavigation, RoutePreference};

fn first_plan(world: &World, destination: SystemId) -> RoutePlan {
    world.route_alternatives(destination).unwrap()[0]
        .plan
        .clone()
}

#[test]
fn route_trade_details_share_current_score_and_do_not_install_or_save_a_route() {
    let world = World::new_commander(11109).unwrap();
    let before = world.save_bytes().unwrap();
    let routes = world.route_alternatives(SystemId::LARERIA).unwrap();
    let markets = world.station_markets.as_ref().unwrap();
    for route in &routes {
        assert_eq!(route.trade_legs.len(), route.plan.systems().len() - 1);
        assert_eq!(
            route.trade_tenths,
            route
                .trade_legs
                .iter()
                .map(|leg| leg.profit_tenths)
                .sum::<u64>()
        );
        for (leg, hop) in route.trade_legs.iter().zip(route.plan.systems().windows(2)) {
            assert_eq!([leg.from, leg.to], [hop[0], hop[1]]);
            for row in &leg.exports {
                let origin = super::super::services::service_site(leg.from).unwrap();
                let target = super::super::services::service_site(leg.to).unwrap();
                assert_eq!(
                    markets.commodity_quote(origin, row.commodity).ask_tenths,
                    row.ask_tenths
                );
                assert_eq!(
                    markets.commodity_quote(target, row.commodity).bid_tenths,
                    row.bid_tenths
                );
                assert!(row.quantity > 0);
            }
        }
    }
    assert!(
        routes
            .iter()
            .any(|route| route.trade_legs.iter().any(|leg| !leg.exports.is_empty()))
    );
    assert!(world.active_route_progress().is_none());
    assert_eq!(world.save_bytes().unwrap(), before);
    assert_eq!(
        World::from_save_bytes(&before)
            .unwrap()
            .route_alternatives(SystemId::LARERIA)
            .unwrap(),
        routes
    );
}

fn cold(world: &World) -> World {
    let restored = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(
        restored.active_route_progress(),
        world.active_route_progress()
    );
    assert_eq!(restored.snapshot(), world.snapshot());
    restored
}

#[test]
fn exact_supported_course_distinguishes_three_grades_current_fuel_and_preferences() {
    let world = World::new_commander(11101).unwrap();
    let catalogue = OriginalNavigation::bundled().unwrap();
    let basic = first_plan(&world, SystemId::LARERIA);
    assert_eq!(basic.grade, NavigationGrade::Basic);
    assert_eq!(basic.grade.planning_radius_ly(), Some(20.0));
    assert_eq!(
        basic.systems(),
        &[SystemId::AVENOR, SystemId::RENAS, SystemId::LARERIA]
    );

    let mut economy = RouteEconomy::default();
    economy.trade[0][2] = 50_000;
    economy.trade[2][1] = 50_000;
    economy.sale[2] = 500;
    economy.sale[1] = 100;
    for (grade, radius) in [
        (NavigationGrade::Intermediate, Some(27.0)),
        (NavigationGrade::Advanced, None),
    ] {
        let mut template = basic.clone();
        template.grade = grade;
        assert_eq!(grade.planning_radius_ly(), radius);
        let alternatives = route_alternatives(
            &catalogue,
            &template,
            SystemId::AVENOR,
            SystemId::LARERIA,
            &economy,
        )
        .unwrap();
        assert_eq!(
            alternatives[0].plan.systems(),
            &[SystemId::AVENOR, SystemId::LARERIA]
        );
        assert_eq!(
            alternatives[0].preferences,
            vec![RoutePreference::FewestJumps, RoutePreference::Safest]
        );
        assert_eq!(alternatives[0].fuel_units, 99);
        assert_eq!(alternatives[1].plan.systems(), basic.systems());
        assert_eq!(
            alternatives[1].preferences,
            vec![
                RoutePreference::TradeOpportunities,
                RoutePreference::BestCargoSale
            ]
        );
        assert_eq!(alternatives[1].fuel_units, 107);
        assert_eq!(alternatives[1].cargo_sale_tenths, 500);
        assert_eq!(alternatives[1].trade_tenths, 100_000);
        template.origin_fuel = 34;
        let narrowed = route_alternatives(
            &catalogue,
            &template,
            SystemId::AVENOR,
            SystemId::LARERIA,
            &economy,
        )
        .unwrap();
        assert_eq!(narrowed.len(), 1);
        assert_eq!(narrowed[0].plan.systems(), basic.systems());
        template.origin_fuel = 33;
        assert_eq!(
            route_alternatives(
                &catalogue,
                &template,
                SystemId::AVENOR,
                SystemId::LARERIA,
                &economy
            ),
            Err(RouteError::NoFeasibleRoute),
        );
    }
    assert_eq!(
        world.route_alternatives(SystemId {
            galaxy: 1,
            system: 7
        }),
        Err(RouteError::UnsupportedSystem)
    );
    assert_eq!(
        world.route_alternatives(SystemId {
            galaxy: 0,
            system: 8
        }),
        Err(RouteError::UnsupportedSystem)
    );
    assert_eq!(
        world.route_alternatives(SystemId::AVENOR),
        Err(RouteError::AlreadyThere)
    );
}

#[test]
fn route_preparation_preserves_live_state_until_commit_and_refuses_foreign_or_forged_plans() {
    let mut world = World::new_commander(11102).unwrap();
    let foreign = World::new_commander(11103).unwrap();
    let tick = world.snapshot().tick;
    let before = world.save_bytes().unwrap();
    let foreign_plan = first_plan(&foreign, SystemId::LARERIA);
    assert_eq!(
        world.set_navigation_route(&foreign_plan, tick),
        Err(RouteError::StalePlan)
    );
    let plan = first_plan(&world, SystemId::LARERIA);
    assert_eq!(
        world.set_navigation_route(&plan, tick + 1),
        Err(RouteError::StalePlan)
    );
    let mut forged = plan.clone();
    forged.systems = vec![SystemId::AVENOR, SystemId::LARERIA];
    assert_eq!(
        world.set_navigation_route(&forged, tick),
        Err(RouteError::StalePlan)
    );
    let preview = world.prepare_navigation_route(Some(&plan), tick).unwrap();
    let restored = World::from_save_bytes(preview.bytes()).unwrap();
    assert_eq!(restored.active_route_progress().unwrap().plan, plan);
    drop(preview);
    assert_eq!(world.save_bytes().unwrap(), before);
    assert!(world.active_route_progress().is_none());
    world.set_navigation_route(&plan, tick).unwrap();
    assert_eq!(cold(&world).active_route_progress().unwrap().plan, plan);
    world.clear_navigation_route(tick).unwrap();
    assert_eq!(world.save_bytes().unwrap(), before);
}

#[test]
fn actual_jump_arrival_waits_for_physical_dock_then_requires_paid_refuelling() {
    let mut world = World::new_commander(11104).unwrap();
    journey(&mut world, SystemId::RENAS);
    journey(&mut world, SystemId::AVENOR);
    assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 62);
    let plan = first_plan(&world, SystemId::LARERIA);
    world
        .set_navigation_route(&plan, world.snapshot().tick)
        .unwrap();
    assert_eq!(
        world.active_route_progress().unwrap().next_system,
        Some(SystemId::RENAS)
    );
    world.request_launch(world.snapshot().tick).unwrap();
    world.step(FlightInput::default()).unwrap();
    assert_eq!(
        world.request_jump(SystemId::LARERIA, world.snapshot().tick),
        Err(NavigationError::OutOfRange)
    );
    assert_eq!(world.active_route_progress().unwrap().completed_legs, 0);
    world
        .request_jump(SystemId::RENAS, world.snapshot().tick)
        .unwrap();
    for _ in 0..BASIC_SPOOL_TICKS {
        world.step(FlightInput::default()).unwrap();
    }
    assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 28);
    let mut world = cold(&world);
    let arrival = world.active_route_progress().unwrap();
    assert_eq!(arrival.status, RouteStatus::AwaitingDock);
    assert_eq!(arrival.completed_legs, 0);
    assert!(arrival.next_system.is_none());
    for _ in 0..9000 {
        let snapshot = world.snapshot();
        if snapshot.mode == FlightMode::Docked {
            break;
        }
        let speed = if snapshot.position[2] > 2500.0 {
            34.3
        } else {
            4.0
        };
        world
            .step(FlightInput {
                throttle: ((speed - snapshot.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..FlightInput::default()
            })
            .unwrap();
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    assert_eq!(
        world.current_dock_context().unwrap().system(),
        SystemId::RENAS
    );
    let shortage = cold(&world).active_route_progress().unwrap();
    assert_eq!(shortage.status, RouteStatus::RefuelRequired);
    assert_eq!(shortage.completed_legs, 1);
    assert_eq!(shortage.next_fuel_units, Some(74));
    assert!(shortage.next_system.is_none());
    assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 28);
    assert!(world.request_refuel(world.snapshot().tick).unwrap() > 0);
    assert_eq!(
        world.active_route_progress().unwrap().next_system,
        Some(SystemId::LARERIA)
    );
    journey(&mut world, SystemId::LARERIA);
    let completed = cold(&world).active_route_progress().unwrap();
    assert_eq!(completed.completed_legs, 2);
    assert_eq!(completed.status, RouteStatus::Complete);
}

#[test]
fn route_estimates_share_cash_and_limit_market_capacity_and_non_tonne_room() {
    use crate::station_services::{MAX_MARKET_QUANTITY, MarketSite, StationMarkets};

    let world = World::new_commander(11105).unwrap();
    let mut cargo = world.reserved_service_projection().unwrap();
    cargo.hold_capacity_tonnes = 8;
    cargo.used_tonnes = 4;
    cargo.food_tonnes = 4;
    cargo.reserved_food_tonnes = 2;
    let gold = CommodityId::Gold.index();
    let textiles = CommodityId::Textiles.index();
    cargo.other_commodities[gold - 1] = 62;
    cargo.reserved_commodities[gold - 1] = 61;
    let markets = StationMarkets::new_commander(11105).unwrap();
    let empty_market = |site| {
        CommodityId::ALL.map(|id| CommodityQuote {
            bid_tenths: 0,
            ask_tenths: 0,
            stock: 0,
            ..markets.commodity_quote(site, id)
        })
    };
    let mut origin = empty_market(MarketSite::Avenor);
    origin[gold].ask_tenths = 5;
    origin[gold].stock = 10;
    origin[textiles].ask_tenths = 20;
    origin[textiles].stock = 1;
    let mut destination = empty_market(MarketSite::Lareria);
    destination[0].bid_tenths = 25;
    destination[0].stock = MAX_MARKET_QUANTITY - 1;
    destination[gold].bid_tenths = 15;
    destination[textiles].bid_tenths = 40;

    cargo.credits_tenths = 0;
    let empty_wallet = estimate_route_economy(cargo, &[Some(origin), Some(destination), None]);
    assert_eq!(empty_wallet.trade[0][1], 0);
    assert_eq!(empty_wallet.sale, [0, 40, 0]);

    cargo.credits_tenths = 25;
    let limited = estimate_route_economy(cargo, &[Some(origin), Some(destination), None]);
    assert_eq!(limited.trade[0][1], 30);
    assert_eq!(limited.sale[1], 40);
    assert_eq!(limited.trade[1][1], 0);

    cargo.credits_tenths = 100;
    let non_tonne_room = estimate_route_economy(cargo, &[Some(origin), Some(destination), None]);
    assert_eq!(non_tonne_room.trade[0][1], 30);

    destination[gold].stock = MAX_MARKET_QUANTITY - 1;
    let full_after_own_sale =
        estimate_route_economy(cargo, &[Some(origin), Some(destination), None]);
    assert_eq!(full_after_own_sale.trade[0][1], 20);
    assert_eq!(full_after_own_sale.sale[1], 40);

    cargo.credits_tenths = u32::MAX - 5;
    let saturated = estimate_route_economy(cargo, &[Some(origin), Some(destination), None]);
    assert_eq!(saturated.trade[0][1], 0);
    assert_eq!(saturated.sale[1], 5);
}

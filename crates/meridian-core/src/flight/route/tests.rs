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

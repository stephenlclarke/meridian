//! Real current-hull route/berth/fuel producers; no caller supplied path or arrival.
use super::*;
use crate::freight::g02;
use crate::navigation::BASIC_SPOOL_TICKS;
use crate::{FlightInput, ModuleAction, XyzzyAction};
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("expected route/state"))
}
fn idle(w: &mut World, n: usize) {
    for _ in 0..n {
        ok(w.step(FlightInput::default()));
    }
}
fn install(w: &mut World, target: SystemId, kind: RouteKind) {
    let tick = w.snapshot().tick;
    let review = ok(w.review_navigation_route(target, kind, tick));
    let before = ok(w.save_bytes());
    let resources = w.flight_resources_snapshot();
    let cash = w.credits_tenths();
    let nav = w.snapshot().navigation;
    let prepared = ok(w.prepare_navigation_route(review, tick));
    let candidate = ok(World::from_save_bytes(prepared.bytes()));
    prepared.commit();
    assert_ne!(ok(w.save_bytes()), before);
    assert_eq!(
        w.navigation_route_snapshot(),
        candidate.navigation_route_snapshot()
    );
    assert_eq!(w.flight_resources_snapshot(), resources);
    assert_eq!(w.credits_tenths(), cash);
    assert_eq!(w.snapshot().navigation, nav);
}
fn cold(w: &World) -> World {
    let loaded = ok(World::from_save_bytes(&ok(w.save_bytes())));
    assert_eq!(
        loaded.navigation_route_snapshot(),
        w.navigation_route_snapshot()
    );
    assert_eq!(loaded.snapshot().navigation, w.snapshot().navigation);
    assert_eq!(loaded.credits_tenths(), w.credits_tenths());
    loaded
}
fn protected() -> World {
    let mut w = ok(World::new_commander(28101));
    // Real existing G02 consent provides a protected public pilot, without editing traffic.
    let issuer = w.station_contacts()[0];
    ok(w.request_contact(issuer, w.snapshot().tick));
    let offer = ok(w.request_freight(
        g02::Command::ReviewOffer {
            actor: issuer.actor(),
        },
        w.snapshot().tick,
    ));
    let g02::Event::Reviewed(review) = offer else {
        panic!("offer")
    };
    let accepted = ok(w.request_freight(
        g02::Command::Confirm {
            review,
            choice: g02::Choice::Accept,
        },
        w.snapshot().tick,
    ));
    assert!(matches!(accepted, g02::Event::Accepted { .. }));
    w
}
fn leg(w: &mut World, expected_fuel: u16) {
    ok(w.request_launch(w.snapshot().tick));
    idle(w, 1);
    ok(w.request_navigation_route_jump(w.snapshot().tick));
    idle(w, BASIC_SPOOL_TICKS as usize);
    assert_eq!(some(w.snapshot().navigation).fuel_units, expected_fuel);
    assert_eq!(
        some(w.navigation_route_snapshot()).phase,
        NavigationRoutePhase::AwaitingDock
    );
    let before = ok(w.save_bytes());
    assert_eq!(
        w.request_navigation_route_jump(w.snapshot().tick),
        Err(NavigationRouteError::AwaitingDock)
    );
    assert_eq!(ok(w.save_bytes()), before);
    cold(w);
    crate::flight::missiles::tests::dock(w);
    assert_eq!(some(w.snapshot().navigation).fuel_units, expected_fuel);
    cold(w);
}
#[test]
fn ordinary_chart_visibility_is_source_known_and_hidden_port_is_neither_discovered_nor_admitted() {
    let w = ok(World::new_commander(28111));
    let before = ok(w.save_bytes());
    let known = some(w.known_navigation_systems());
    assert_eq!(known.len(), 256);
    assert_eq!(known[7], SystemId::AVENOR);
    assert!(!known.contains(&SystemId {
        galaxy: 0,
        system: 256
    }));
    assert!(!known.contains(&SystemId {
        galaxy: 1,
        system: 7
    }));
    for target in [
        SystemId {
            galaxy: 0,
            system: 256,
        },
        SystemId {
            galaxy: 1,
            system: 7,
        },
        SystemId {
            galaxy: 0,
            system: 3,
        },
    ] {
        assert!(matches!(
            w.review_navigation_route(target, RouteKind::FewestJumps, 0),
            Err(NavigationRouteError::UnsupportedDestination)
        ));
    }
    assert_eq!(ok(w.save_bytes()), before);
}
#[test]
fn actual_stock_route_advances_only_after_physical_berths_and_refuelling_is_an_actual_paid_service()
{
    let mut w = protected();
    let cash = w.credits_tenths();
    install(&mut w, SystemId::LARERIA, RouteKind::FewestJumps);
    assert_eq!(
        some(w.navigation_route_snapshot()).systems,
        [SystemId::AVENOR, SystemId::RENAS, SystemId::LARERIA]
    );
    leg(&mut w, 96);
    assert_eq!(
        some(w.navigation_route_snapshot()).next,
        Some(SystemId::LARERIA)
    );
    leg(&mut w, 23);
    assert_eq!(
        some(w.navigation_route_snapshot()).phase,
        NavigationRoutePhase::Complete
    );
    assert_eq!(w.credits_tenths(), cash);
    assert_eq!(some(w.mission_snapshot()).freight.held_tonnes, 8);
    let before = ok(w.save_bytes());
    assert!(matches!(
        w.review_navigation_route(SystemId::AVENOR, RouteKind::FewestJumps, w.snapshot().tick),
        Err(NavigationRouteError::NoRoute)
    ));
    assert_eq!(ok(w.save_bytes()), before);
    assert_eq!(ok(w.request_refuel(w.snapshot().tick)), 214);
    assert_eq!(w.credits_tenths(), Some(some(cash) - 214));
    assert_eq!(some(w.snapshot().navigation).fuel_units, 130);
    install(&mut w, SystemId::AVENOR, RouteKind::Safer);
    cold(&w);
}
#[test]
fn review_is_scope_bound_and_fresh_fuel_fit_revision_and_cancel_are_revalidated_before_publication()
{
    let mut w = ok(World::new_commander(28121));
    let tick = w.snapshot().tick;
    let review = ok(w.review_navigation_route(SystemId::RENAS, RouteKind::FewestJumps, tick));
    let mut foreign = ok(World::new_commander(28121));
    assert!(matches!(
        foreign.prepare_navigation_route(review.clone(), tick),
        Err(NavigationRouteError::StaleReview)
    ));
    let mut restored = cold(&w);
    assert!(matches!(
        restored.prepare_navigation_route(review.clone(), tick),
        Err(NavigationRouteError::StaleReview)
    ));
    let before = ok(w.save_bytes());
    let prepared = ok(w.prepare_navigation_route(review.clone(), tick));
    let failure: Result<(), &str> = prepared.publish_with(|_| Err("simulated publication refusal"));
    assert_eq!(failure, Err("simulated publication refusal"));
    assert_eq!(ok(w.save_bytes()), before);
    ok(w.prepare_clear_navigation_route(tick)).commit();
    assert!(matches!(
        w.prepare_navigation_route(review, tick),
        Err(NavigationRouteError::StaleReview)
    ));
    let navitem = some(w.modular_equipment_snapshot())
        .items
        .into_iter()
        .find(|item| item.slot == Some(ModuleSlot::Navigation))
        .map(|item| item.id);
    let review = ok(w.review_module(
        ModuleAction::Store {
            item: some(navitem),
        },
        tick,
    ));
    ok(w.prepare_module(review, tick)).commit();
    assert!(matches!(
        w.review_navigation_route(SystemId::RENAS, RouteKind::FewestJumps, tick),
        Err(NavigationRouteError::NoComputer)
    ));
}
#[test]
fn actual_manual_off_route_jump_cancels_bookmark_and_busy_clear_cannot_cancel_or_refund_a_spool() {
    let mut w = ok(World::new_commander(28131));
    // Explicit genuine testing-menu equipment setup, separate from the normal stock journey.
    let review = ok(w.review_xyzzy(XyzzyAction::Enter, 0));
    ok(w.prepare_xyzzy(review, 0)).commit();
    let review = ok(w.review_xyzzy(XyzzyAction::ToggleUnlimitedFuel, 0));
    ok(w.prepare_xyzzy(review, 0)).commit();
    install(&mut w, SystemId::RENAS, RouteKind::FewestJumps);
    ok(w.request_launch(w.snapshot().tick));
    idle(&mut w, 1);
    ok(w.request_jump(SystemId::LARERIA, w.snapshot().tick));
    assert_eq!(
        some(w.navigation_route_snapshot()).phase,
        NavigationRoutePhase::Cancelled
    );
    let before = ok(w.save_bytes());
    assert!(matches!(
        w.prepare_clear_navigation_route(w.snapshot().tick),
        Err(NavigationRouteError::Busy)
    ));
    assert_eq!(ok(w.save_bytes()), before);
    idle(&mut w, BASIC_SPOOL_TICKS as usize);
    assert_eq!(some(w.snapshot().navigation).fuel_units, 31);
    assert_eq!(
        w.request_navigation_route_jump(w.snapshot().tick),
        Err(NavigationRouteError::Cancelled)
    );
    cold(&w);
}
#[test]
fn coherent_route_path_item_owner_and_length_corruption_refuse_without_replacing_live_world() {
    let mut w = ok(World::new_commander(28141));
    install(&mut w, SystemId::RENAS, RouteKind::FewestJumps);
    let before = ok(w.save_bytes());
    for mutation in 0..4 {
        let mut bad = cold(&w);
        let plan = some(bad.route_plan.plan.as_mut());
        match mutation {
            0 => {
                plan.path[1] = SystemId {
                    galaxy: 0,
                    system: 3,
                }
            }
            1 => plan.item += 999,
            2 => plan.dock = some(ok(World::new_commander(99999)).current_dock_context()),
            _ => plan.len = 4,
        }
        assert!(bad.save_bytes().is_err());
        assert!(w.restore_bytes(&ok(bad.unchecked_test_save())).is_err());
        assert_eq!(ok(w.save_bytes()), before);
    }
}

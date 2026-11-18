//! Real App J/N binding after fresh Profile continuation; no native execution claim.
use super::*;
use crate::profile::tests::{TestDirectory, open};
use meridian_core::navigation::RouteKind;
fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
#[test]
fn continued_installed_route_owns_j_and_n_clears_world_and_any_old_menu_destination_without_fuel() {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(28211)));
    let review = ok(profile.world.review_navigation_route(
        meridian_core::navigation::SystemId::RENAS,
        RouteKind::FewestJumps,
        0,
    ));
    ok(profile.install_navigation_route(review));
    drop(profile);
    let mut cold = open(&directory.0);
    ok(cold.continue_saved());
    let mut a = App::with_profile(cold);
    let mut menu = ok(Docked::new(&a.game.world));
    menu.route = Some(meridian_core::navigation::SystemId::LARERIA);
    a.docked = Some(menu);
    a.game_key(KeyCode::KeyL);
    assert_eq!(a.advance_frame(true, 1), FrameAdvance::Continue(None));
    a.game_key(KeyCode::KeyJ);
    assert_eq!(
        a.game
            .world
            .snapshot()
            .navigation
            .and_then(|nav| nav.travel)
            .map(|travel| travel.destination),
        Some(meridian_core::navigation::SystemId::RENAS)
    );
    assert_eq!(
        a.game.world.snapshot().navigation.map(|nav| nav.fuel_units),
        Some(130)
    );
    // The in-flight World was not saved; release its lease before continuing the disk route.
    drop(a);
    let mut cold = open(&directory.0);
    ok(cold.continue_saved());
    let mut b = App::with_profile(cold);
    let mut menu = ok(Docked::new(&b.game.world));
    menu.route = Some(meridian_core::navigation::SystemId::LARERIA);
    b.docked = Some(menu);
    let before = b.game.world.snapshot();
    b.game_key(KeyCode::KeyN);
    assert!(b.game.world.navigation_route_snapshot().is_none());
    assert!(b.docked.as_ref().is_some_and(|menu| menu.route.is_none()));
    assert_eq!(b.game.world.snapshot(), before);
    assert_eq!(b.game.world.credits_tenths(), Some(7200));
    drop(b);
    let mut continued = open(&directory.0);
    ok(continued.continue_saved());
    assert!(continued.world.navigation_route_snapshot().is_none());
    let help = crate::help::Panel::flight(None, None);
    assert!(
        help.lines
            .iter()
            .any(|line| line.contains("N Clear installed route"))
    );
}

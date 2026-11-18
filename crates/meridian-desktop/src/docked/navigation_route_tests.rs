//! Actual search/planning keys, presented route confirmation and atomic profile files.
use super::*;
use crate::profile::tests::{TestDirectory, open};
use meridian_core::navigation::RouteKind;
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("expected route review/frame"))
}
fn start() -> (TestDirectory, Profile, Docked) {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(28201)));
    ok(profile.save());
    let mut menu = ok(Docked::new(&profile.world));
    ok(menu.open(&profile.world, ui::Screen::Galaxy));
    (directory, profile, menu)
}
fn paint(menu: &mut Docked, profile: &Profile) {
    let frame = ok(menu.build_frame(&profile.world, some(ui::Viewport::new(1280.0, 720.0)), 0.0));
    menu.presented(frame);
}
fn input(menu: &mut Docked, profile: &mut Profile, value: ui::Input) {
    ok(menu.input(profile, value));
    paint(menu, profile);
}
fn lareria(menu: &mut Docked, profile: &mut Profile) {
    paint(menu, profile);
    input(menu, profile, ui::Input::Key(ui::Key::Search));
    input(menu, profile, ui::Input::Text("Lareria".into()));
    input(menu, profile, ui::Input::Key(ui::Key::Confirm));
    input(menu, profile, ui::Input::Key(ui::Key::Options));
    assert_eq!(menu.planning, Some(RouteKind::FewestJumps));
    input(menu, profile, ui::Input::Key(ui::Key::Route));
}
#[test]
fn actual_chart_search_and_planning_review_publish_the_stock_two_leg_route_and_continue_exactly() {
    let (directory, mut profile, mut menu) = start();
    let before = ok(profile.world.save_bytes());
    let snapshot = profile.world.snapshot();
    lareria(&mut menu, &mut profile);
    let details = &some(menu.pending.as_ref()).view.details;
    assert!(
        details
            .iter()
            .any(|line| line.contains("G01-S007 -> G01-S185 -> G01-S091"))
    );
    assert!(details.iter().any(|line| line.contains("2 hops")));
    assert!(
        details
            .iter()
            .any(|line| line.contains("First leg fuel 3.4"))
    );
    assert!(details.iter().any(|line| line.contains("not guaranteed")));
    assert_eq!(ok(profile.world.save_bytes()), before);
    assert_eq!(
        ok(std::fs::read(directory.0.join("commander.meridian"))),
        before
    );
    input(&mut menu, &mut profile, ui::Input::Key(ui::Key::Confirm));
    assert_eq!(profile.world.snapshot(), snapshot);
    assert_eq!(profile.world.credits_tenths(), Some(7200));
    let route = some(profile.world.navigation_route_snapshot());
    assert_eq!(route.next, Some(SystemId::RENAS));
    let disk = ok(std::fs::read(directory.0.join("commander.meridian")));
    assert_eq!(disk, ok(profile.world.save_bytes()));
    drop(profile); // Release the actual exclusive Store lease before Continue.
    let mut profile = open(&directory.0);
    ok(profile.continue_saved());
    assert_eq!(profile.world.navigation_route_snapshot(), Some(route));
    assert_eq!(menu.route, Some(SystemId::RENAS));
    paint(&mut menu, &profile);
    input(&mut menu, &mut profile, ui::Input::Key(ui::Key::Compare));
    assert!(profile.world.navigation_route_snapshot().is_none());
    assert!(menu.route.is_none());
    assert_eq!(profile.world.snapshot(), snapshot);
    assert_eq!(profile.world.credits_tenths(), Some(7200));
}
#[test]
fn actual_route_writer_failure_preserves_live_plan_fuel_wallet_and_the_prior_file() {
    let (directory, mut profile, mut menu) = start();
    lareria(&mut menu, &mut profile);
    let before = ok(profile.world.save_bytes());
    let snapshot = profile.world.snapshot();
    let slot = directory.0.join("commander.meridian");
    let retained = directory.0.join("prior-navigation-save");
    ok(std::fs::rename(&slot, &retained));
    ok(std::fs::create_dir(&slot));
    assert!(
        menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm))
            .is_err()
    );
    assert_eq!(ok(profile.world.save_bytes()), before);
    assert_eq!(profile.world.snapshot(), snapshot);
    assert_eq!(profile.world.credits_tenths(), Some(7200));
    assert!(profile.world.navigation_route_snapshot().is_none());
    assert_eq!(ok(std::fs::read(&retained)), before);
    assert!(slot.is_dir());
}
#[test]
fn help_matches_real_route_mode_and_clear_bindings_and_a_review_cannot_be_changed_by_those_keys() {
    let (_directory, mut profile, mut menu) = start();
    lareria(&mut menu, &mut profile);
    let before = ok(profile.world.save_bytes());
    let token = some(menu.pending.as_ref()).view.token;
    for key in [ui::Key::Options, ui::Key::Compare] {
        input(&mut menu, &mut profile, ui::Input::Key(key));
        assert_eq!(some(menu.pending.as_ref()).view.token, token);
        assert_eq!(ok(profile.world.save_bytes()), before);
    }
    for screen in [ui::Screen::Galaxy, ui::Screen::System] {
        let help = crate::help::Panel::station(screen, Some("Select destination"), true, true);
        assert!(
            help.lines
                .iter()
                .any(|line| line.contains("O Direct / Fewest / Safer"))
        );
        assert!(
            help.lines
                .iter()
                .any(|line| line.contains("C Clear installed route"))
        );
        assert!(help.lines.len() + 2 <= crate::help::MAX_LINES);
        assert!(
            help.lines
                .iter()
                .all(|line| line.len() <= crate::help::MAX_COLUMNS)
        );
    }
}

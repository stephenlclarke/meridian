use super::{App, mule_validation::Launch};
use crate::{
    authored_hull::fixture_path,
    docked_screens::{self as ui, Input, Key},
    profile::tests::{TestDirectory, open},
};
use std::ffi::OsString;
use winit::keyboard::KeyCode;

fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}

#[test]
fn normal_game_entry_searches_authored_package_and_never_changes_owned_ship_or_save() {
    let directory = TestDirectory::new();
    let profile = directory.0.join("profile");
    let args = [
        OsString::from("--save-dir"),
        profile.into_os_string(),
        OsString::from("--hull-package"),
        fixture_path(false).into_os_string(),
    ];
    let mut app = ok(App::from_launch(ok(Launch::parse(args.into_iter()))));
    assert!(app.commander_open && app.game.playing());
    let before = ok(app.game.world.save_bytes());
    // Ordinary navigation out of Commander Details invalidates freight review authority
    // via request_pause, even when the pause value is unchanged. Match that actual transition.
    let ordinary_directory = TestDirectory::new();
    let mut ordinary_profile = open(&ordinary_directory.0);
    let seed = app
        .game
        .world
        .commander_id()
        .unwrap_or_else(|| panic!("commander"))
        .seed();
    assert!(ok(ordinary_profile.new_commander(seed)));
    let mut ordinary = App::with_profile(ordinary_profile);
    ordinary.commander_open = true;
    assert_eq!(ok(ordinary.game.world.save_bytes()), before);
    assert!(ordinary.commander_key(KeyCode::Digit3));
    assert!(app.commander_key(KeyCode::Digit3));
    assert!(app.menu_active());
    let after_open = ok(app.game.world.save_bytes());
    assert_eq!(after_open, ok(ordinary.game.world.save_bytes()));
    assert_ne!(
        after_open, before,
        "ordinary navigation must invalidate review authority"
    );
    let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
    let menu = app
        .docked
        .as_mut()
        .unwrap_or_else(|| panic!("actual docked menu"));
    for action in [
        Input::Key(Key::Search),
        Input::Text("TEST901".into()),
        Input::Key(Key::Confirm),
    ] {
        let frame = ok(menu.build_frame(&app.game.world, viewport, 0.0));
        menu.presented(frame);
        ok(menu.input(&mut app.game, action));
    }
    let comparison = ok(menu.build_frame(&app.game.world, viewport, 0.0));
    let [baseline, selected] = comparison.models.as_slice() else {
        panic!("expected actual Compare panes");
    };
    assert_eq!(baseline.object, "SH025");
    assert_eq!(selected.object, "TEST901");
    assert!(app.authored_hulls.visual_for(selected).is_some());
    assert!(
        comparison
            .labels
            .iter()
            .any(|label| label.text.contains("Exchange unavailable"))
    );
    menu.presented(comparison);
    ok(menu.input(&mut app.game, Input::Key(Key::Compare)));
    let solo = ok(menu.build_frame(&app.game.world, viewport, 0.0));
    assert_eq!(solo.models.len(), 1);
    assert_eq!(solo.models[0].object, "TEST901");
    menu.presented(solo);
    ok(menu.input(&mut app.game, Input::Key(Key::Exchange)));
    let frame = ok(menu.build_frame(&app.game.world, viewport, 0.0));
    menu.presented(frame);
    ok(menu.input(&mut app.game, Input::Key(Key::Details)));
    let details = ok(menu.build_frame(&app.game.world, viewport, 0.0));
    assert!(
        details
            .labels
            .iter()
            .any(|label| label.text.contains("Core sweep Side pass: CLEAR"))
    );
    assert_eq!(ok(app.game.world.save_bytes()), after_open);
}

#[test]
fn malformed_explicit_package_refuses_before_profile_is_opened() {
    let directory = TestDirectory::new();
    let profile = directory.0.join("unopened-profile");
    let missing = directory.0.join("missing.json");
    let args = [
        OsString::from("--inspect-hull-package"),
        missing.into_os_string(),
        OsString::from("--save-dir"),
        profile.clone().into_os_string(),
    ];
    assert!(App::from_launch(ok(Launch::parse(args.into_iter()))).is_err());
    assert!(!profile.exists());
}

#[test]
fn one_click_authored_showroom_requires_exactly_one_isolated_package() {
    let parse = |args: &[&str]| Launch::parse(args.iter().map(|arg| OsString::from(*arg)));
    for args in [
        vec!["--open-authored-showroom"],
        vec!["--hull-package", "fresh.json", "--open-authored-showroom"],
        vec![
            "--open-authored-showroom",
            "--inspect-hull-package",
            "one.json",
            "--inspect-hull-package",
            "two.json",
        ],
        vec![
            "--open-authored-showroom",
            "--inspect-hull-package",
            "one.json",
            "--open-authored-showroom",
        ],
        vec![
            "--internal-mule-validation",
            "front",
            "--open-authored-showroom",
            "--inspect-hull-package",
            "one.json",
        ],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
    let Launch::Authored {
        packages,
        open_showroom,
        ..
    } = ok(parse(&[
        "--open-authored-showroom",
        "--save-dir",
        "slot",
        "--hull-package",
        "fresh.json",
        "--inspect-hull-package",
        "one.json",
    ]))
    else {
        panic!("expected authored launch");
    };
    assert!(open_showroom);
    assert_eq!(packages.len(), 2);
    assert_eq!(packages.iter().filter(|p| p.is_isolated()).count(), 1);
}

#[test]
fn one_click_authored_showroom_opens_actual_local_model_once_without_world_mutation() {
    let directory = TestDirectory::new();
    let profile = directory.0.join("profile");
    let path = directory.0.join("original-id.json");
    let input = ok(std::fs::read_to_string(fixture_path(false)))
        .replace("TEST901", "SH023")
        .replace("Fixture Skiff", "Peregrine Mk III");
    ok(std::fs::write(&path, input));
    let args = [
        OsString::from("--inspect-hull-package"),
        path.into_os_string(),
        OsString::from("--open-authored-showroom"),
        OsString::from("--save-dir"),
        profile.clone().into_os_string(),
    ];
    let mut app = ok(App::from_launch(ok(Launch::parse(args.into_iter()))));
    assert!(!app.commander_open && app.menu_active());
    let seed = app
        .game
        .world
        .commander_id()
        .unwrap_or_else(|| panic!("commander"))
        .seed();
    let control = ok(meridian_core::World::new_commander(seed));
    let initial = ok(control.save_bytes());
    assert_eq!(ok(app.game.world.save_bytes()), initial);
    assert!(!profile.join("commander.meridian").exists());
    let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
    let menu = app.docked.as_mut().unwrap_or_else(|| panic!("menu"));
    assert_eq!(menu.screen(), ui::Screen::Showroom);
    let first = ok(menu.build_frame(&app.game.world, viewport, 0.0));
    let [baseline, selected] = first.models.as_slice() else {
        panic!("expected Compare panes");
    };
    assert_eq!(baseline.object, "SH025");
    assert_eq!(selected.object, "LOCAL:SH023");
    assert!(app.authored_hulls.visual_for(selected).is_some());
    assert!(crate::mule_render::showroom::hull(selected).is_none());
    for action in [
        Input::Key(Key::Search),
        Input::Text("SH023".into()),
        Input::Key(Key::Confirm),
    ] {
        let frame = ok(menu.build_frame(&app.game.world, viewport, 0.0));
        menu.presented(frame);
        ok(menu.input(&mut app.game, action));
    }
    assert_eq!(ok(app.game.world.save_bytes()), initial);
    app.show_station_commander();
    assert!(app.commander_key(KeyCode::Digit3));
    let menu = app
        .docked
        .as_mut()
        .unwrap_or_else(|| panic!("reopened menu"));
    let reopened = ok(menu.build_frame(&app.game.world, viewport, 0.0));
    let [_, selected] = reopened.models.as_slice() else {
        panic!("expected Compare panes");
    };
    assert_eq!(selected.object, "SH023");
    assert_eq!(
        crate::mule_render::showroom::hull(selected),
        Some(crate::mule_render::showroom::Hull::PeregrineMkIII)
    );
    assert!(app.authored_hulls.visual_for(selected).is_none());
}

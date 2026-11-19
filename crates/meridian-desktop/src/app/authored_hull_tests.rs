use super::{App, mule_validation::Launch};
use crate::{
    authored_hull::fixture_path,
    docked_screens::{self as ui, Input, Key},
    profile::tests::TestDirectory,
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
    assert!(app.commander_key(KeyCode::Digit3));
    assert!(app.menu_active());
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
    assert_eq!(ok(app.game.world.save_bytes()), before);
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

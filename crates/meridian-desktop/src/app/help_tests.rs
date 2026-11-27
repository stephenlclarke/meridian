//! Actual host routing and core snapshots, without a synthetic gameplay input loop.
use super::{
    Action, App, Docked, ElementState, FlightInput, FrameAdvance, KeyCode, PhysicalKey, ui,
};
use crate::profile::tests::{TestDirectory, open};

fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn app(directory: &TestDirectory) -> App {
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(98103)));
    App::with_profile(profile)
}
fn press(app: &mut App, code: KeyCode, text: Option<&str>) -> Option<KeyCode> {
    app.route_keyboard(PhysicalKey::Code(code), ElementState::Pressed, false, text)
}
fn paint(app: &mut App) -> ui::Frame {
    let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
    let menu = app.docked.as_mut().unwrap_or_else(|| panic!("menu"));
    let frame = ok(menu.build_frame(&app.game.world, viewport, 0.0));
    menu.presented(frame.clone());
    frame
}
fn station(app: &mut App, screen: ui::Screen) {
    let mut menu = ok(Docked::new(&app.game.world));
    ok(menu.open(&app.game.world, screen));
    app.docked = Some(menu);
    paint(app);
}

#[test]
fn p_toggles_chart_profit_scan_without_pausing_or_mutating_world() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    station(&mut app, ui::Screen::Galaxy);
    let before = ok(app.game.world.save_bytes());
    let paused = app.paused;
    assert_eq!(press(&mut app, KeyCode::KeyP, Some("p")), None);
    let shown = paint(&mut app);
    assert!(
        shown
            .labels
            .iter()
            .any(|label| label.text == "P: Profit scan ON")
    );
    assert!(
        shown
            .labels
            .iter()
            .any(|label| label.text.contains("Renas:"))
    );
    assert_eq!(app.paused, paused);
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(press(&mut app, KeyCode::KeyP, Some("p")), None);
    assert!(
        paint(&mut app)
            .labels
            .iter()
            .any(|label| label.text == "P: Profit scan OFF")
    );
    assert_eq!(press(&mut app, KeyCode::KeyH, Some("h")), None);
    assert!(app.help.as_ref().is_some_and(|help| {
        help.lines
            .iter()
            .any(|line| line.contains("P Toggle best-profit"))
    }));
    assert_eq!(press(&mut app, KeyCode::Escape, None), None);
    station(&mut app, ui::Screen::System);
    assert_eq!(
        press(&mut app, KeyCode::KeyP, Some("p")),
        Some(KeyCode::KeyP)
    );
    app.docked = None;
    assert_eq!(
        press(&mut app, KeyCode::KeyP, Some("p")),
        Some(KeyCode::KeyP)
    );
    assert_eq!(ok(app.game.world.save_bytes()), before);
}

#[test]
fn every_station_help_retains_its_screen_selection_and_all_navigation_bindings() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    for screen in ui::Screen::ALL {
        station(&mut app, screen);
        app.menu_input(ui::Input::Key(ui::Key::Next));
        let before = paint(&mut app);
        let save = ok(app.game.world.save_bytes());
        assert_eq!(press(&mut app, KeyCode::KeyH, Some("h")), None);
        let panel = app.help.as_ref().unwrap_or_else(|| panic!("help"));
        let text = panel.lines.join("\n");
        for required in [
            "1 Galaxy",
            "2 System",
            "3 Showroom",
            "4 Market",
            "5 Equipment",
            "Tab / Shift+Tab",
            "Escape Back",
        ] {
            assert!(text.contains(required), "{screen:?} omitted {required}");
        }
        for code in [
            KeyCode::Enter,
            KeyCode::KeyG,
            KeyCode::KeyX,
            KeyCode::Digit1,
            KeyCode::Tab,
            KeyCode::KeyS,
            KeyCode::KeyR,
            KeyCode::KeyP,
            KeyCode::KeyL,
        ] {
            assert_eq!(press(&mut app, code, None), None);
        }
        app.menu_input(ui::Input::Key(ui::Key::Next));
        app.scroll(2.0);
        app.pointer([200.0, 180.0]);
        assert_eq!(press(&mut app, KeyCode::Escape, None), None);
        assert!(app.help.is_none());
        let after = paint(&mut app);
        assert_eq!(after.screen, before.screen);
        assert_eq!(after.labels, before.labels);
        assert_eq!(ok(app.game.world.save_bytes()), save);
    }
}

#[test]
fn help_above_a_real_purchase_review_cannot_confirm_cancel_save_or_spend() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    station(&mut app, ui::Screen::Trading);
    app.menu_input(ui::Input::Key(ui::Key::Buy));
    let review = paint(&mut app);
    let panel = app
        .docked
        .as_ref()
        .unwrap_or_else(|| panic!("menu"))
        .help_panel();
    assert!(panel.title.to_ascii_lowercase().contains("buy"));
    assert!(
        panel
            .lines
            .iter()
            .any(|line| line == "After help: Escape / B cancels the review")
    );
    assert_eq!(
        crate::docked::key(KeyCode::Escape, false, ui::Screen::Trading),
        Some(ui::Key::Back)
    );
    let world = ok(app.game.world.save_bytes());
    let disk = std::fs::read(directory.0.join("commander.meridian")).ok();
    press(&mut app, KeyCode::KeyH, Some("h"));
    assert_eq!(
        app.help.as_ref().map(|help| help.title.as_str()),
        Some(panel.title.as_str())
    );
    assert_eq!(press(&mut app, KeyCode::Enter, None), None);
    app.menu_input(ui::Input::Key(ui::Key::Confirm));
    assert_eq!(app.advance_frame(true, 60), FrameAdvance::Continue(None));
    assert_eq!(ok(app.game.world.save_bytes()), world);
    assert_eq!(
        std::fs::read(directory.0.join("commander.meridian")).ok(),
        disk
    );
    press(&mut app, KeyCode::Escape, None);
    assert_eq!(paint(&mut app).labels, review.labels);
    // The original review remains actionable after help is gone.
    app.menu_input(ui::Input::Key(ui::Key::Confirm));
    assert_ne!(ok(app.game.world.save_bytes()), world);
}

#[test]
fn question_help_preserves_literal_h_search_through_focus_and_occlusion() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    station(&mut app, ui::Screen::Galaxy);
    app.menu_input(ui::Input::Key(ui::Key::Search));
    // Text routing follows current controller state even before its next paint.
    assert_eq!(press(&mut app, KeyCode::KeyH, Some("h")), None);
    // A composing keyboard can report a physical key before it supplies printable text.
    app.modifiers = winit::keyboard::ModifiersState::SHIFT;
    assert_eq!(press(&mut app, KeyCode::Digit3, None), None);
    assert_eq!(press(&mut app, KeyCode::Digit3, Some("#")), None);
    assert!(app.game.session.help_review().is_none());
    app.modifiers = winit::keyboard::ModifiersState::empty();
    let search = paint(&mut app);
    assert!(search.search_active);
    assert!(app.help.is_none());
    let save = ok(app.game.world.save_bytes());
    assert_eq!(press(&mut app, KeyCode::Slash, Some("?")), None);
    assert!(app.help.is_some());
    app.focus_changed(false);
    app.occlusion_changed(true);
    app.focus_changed(true);
    app.occlusion_changed(false);
    press(&mut app, KeyCode::KeyH, Some("h"));
    assert!(app.help.is_none());
    let restored = paint(&mut app);
    assert!(restored.search_active);
    assert_eq!(restored.labels, search.labels);
    assert_eq!(ok(app.game.world.save_bytes()), save);
}

#[test]
fn help_freezes_actual_flight_without_changing_manual_pause_or_replaying_held_fire() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    ok(app.game.world.request_launch(0));
    ok(app.game.world.step(FlightInput::default()));
    for manually_paused in [false, true] {
        app.paused = manually_paused;
        ok(app
            .game
            .world
            .request_pause(manually_paused, app.game.world.snapshot().tick));
        app.keys.extend([KeyCode::KeyW, KeyCode::KeyZ]);
        let saved = ok(app.game.world.save_bytes());
        press(&mut app, KeyCode::KeyH, Some("h"));
        assert!(app.keys.is_empty());
        assert!(!app.simulation_active());
        app.focus_changed(false);
        app.focus_changed(true);
        assert_eq!(app.advance_frame(true, 600), FrameAdvance::Continue(None));
        assert_eq!(ok(app.game.world.save_bytes()), saved);
        assert_eq!(press(&mut app, KeyCode::KeyP, None), None);
        assert_eq!(app.paused, manually_paused);
        press(&mut app, KeyCode::Slash, Some("?"));
        assert_eq!(app.simulation_active(), !manually_paused);
        assert!(app.keys.is_empty());
        assert_eq!(ok(app.game.world.save_bytes()), saved);
        if !manually_paused {
            assert_eq!(app.advance_frame(true, 1), FrameAdvance::Continue(None));
            assert!(app.pulse.is_none());
            assert!(app.mining.is_none());
        }
    }
}

#[test]
fn flight_help_lists_the_computer_keys_and_consumes_them_while_open() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    ok(app.game.command(Action::Launch));
    for _ in 0..120 {
        ok(app.game.world.step(FlightInput::default()));
    }
    let before = ok(app.game.world.save_bytes());
    press(&mut app, KeyCode::KeyH, Some("h"));
    let help = app.help.as_ref().unwrap_or_else(|| panic!("flight help"));
    let text = help.lines.join("\n");
    for binding in [
        "N Start or cancel auto-docking",
        "Tab Next scanner target (no core: mark)",
        "F Manual/assist",
        "Basic aligns | Advanced approaches | Docking+nav docks | Specialists auto-work",
        "scoop ore | cut wreck | occupied review",
        "T Lock selected ship for missiles",
        "Shift+I Target details",
    ] {
        assert!(text.contains(binding), "Missing {binding}");
    }
    for key in [
        KeyCode::KeyN,
        KeyCode::KeyT,
        KeyCode::KeyF,
        KeyCode::Tab,
        KeyCode::Backspace,
    ] {
        assert_eq!(press(&mut app, key, None), None);
    }
    assert!(app.help.is_some());
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert!(!app.game.world.auto_docking_active());
    press(&mut app, KeyCode::Escape, None);
    app.game_key(KeyCode::KeyN);
    assert!(app.game.world.auto_docking_active());
    assert!(app.game.lines(true, None).is_empty());
    let saved = ok(app.game.world.save_bytes());
    press(&mut app, KeyCode::KeyH, Some("h"));
    press(&mut app, KeyCode::KeyI, Some("i"));
    let panel = app
        .help
        .as_ref()
        .unwrap_or_else(|| panic!("missing flight status"));
    assert_eq!(panel.title, "Flight status");
    assert!(panel.lines.iter().any(|line| line.contains("AUTO DOCK")));
    press(&mut app, KeyCode::Escape, None);
    assert_eq!(ok(app.game.world.save_bytes()), saved);
    app.game_key(KeyCode::KeyN);
    assert!(!app.game.world.auto_docking_active());
    app.game_key(KeyCode::KeyN);
    press(&mut app, KeyCode::KeyH, Some("h"));
    assert!(app.game.world.auto_docking_active());
    app.focus_changed(false);
    assert!(!app.game.world.auto_docking_active());
    assert!(app.help.is_some());
}

#[test]
fn title_help_intercepts_new_continue_quit_and_repeated_toggle_without_a_profile_change() {
    let directory = TestDirectory::new();
    let profile = open(&directory.0);
    let mut app = App::with_profile(profile);
    assert!(!app.game.playing());
    press(&mut app, KeyCode::KeyH, Some("h"));
    let title = app.help.clone();
    for code in [KeyCode::KeyN, KeyCode::KeyC, KeyCode::Enter, KeyCode::KeyS] {
        assert_eq!(press(&mut app, code, None), None);
    }
    assert_eq!(
        app.route_keyboard(
            PhysicalKey::Code(KeyCode::KeyH),
            ElementState::Pressed,
            true,
            Some("h")
        ),
        None
    );
    assert_eq!(app.help, title);
    assert!(!app.game.playing());
    press(&mut app, KeyCode::Escape, None);
    assert!(app.help.is_none());
    assert!(!app.game.playing());
    assert_eq!(
        press(&mut app, KeyCode::KeyN, Some("n")),
        Some(KeyCode::KeyN)
    );
}

#[test]
fn session_review_help_uses_b_cancel_and_escape_first_only_dismisses_help() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    assert!(!app.menu_active());
    ok(app.game.command(Action::ReviewTradeLesson));
    let pending = app
        .game
        .session
        .pending_trade
        .unwrap_or_else(|| panic!("real trade review"));
    let before = ok(app.game.world.save_bytes());
    press(&mut app, KeyCode::KeyH, Some("h"));
    let panel = app.help.as_ref().unwrap_or_else(|| panic!("help"));
    assert!(panel.title.contains("Trade lesson review"));
    assert!(
        panel
            .lines
            .iter()
            .any(|line| line == "After help: B cancels; Escape saves and quits")
    );
    assert!(
        !panel
            .lines
            .iter()
            .any(|line| line.contains("Escape / B cancels"))
    );
    assert_eq!(press(&mut app, KeyCode::Enter, None), None);
    assert_eq!(press(&mut app, KeyCode::Escape, None), None);
    assert!(app.help.is_none());
    assert_eq!(app.game.session.pending_trade, Some(pending));
    assert_eq!(ok(app.game.world.save_bytes()), before);
    // The real non-menu App handler maps B to Session cancellation after help closes.
    assert_eq!(
        press(&mut app, KeyCode::KeyB, Some("b")),
        Some(KeyCode::KeyB)
    );
    app.game_key(KeyCode::KeyB);
    assert!(app.game.session.pending_trade.is_none());
    assert_eq!(ok(app.game.world.save_bytes()), before);
}

#[test]
fn training_shortcuts_open_the_same_review_from_commander_and_station() {
    use winit::keyboard::ModifiersState;
    for (digit, action) in [
        (KeyCode::Digit1, Action::ReviewFlightLesson),
        (KeyCode::Digit2, Action::CancelFlightLesson),
        (KeyCode::Digit3, Action::ReviewPractice),
        (KeyCode::Digit4, Action::ReviewPracticeReception),
        (KeyCode::Digit5, Action::ReviewPracticeClosure),
    ] {
        let expected_dir = TestDirectory::new();
        let mut expected = app(&expected_dir);
        let message = expected.game.command(action).unwrap_or_else(|error| error);
        for from_commander in [true, false] {
            let dir = TestDirectory::new();
            let mut app = app(&dir);
            station(&mut app, ui::Screen::System);
            if from_commander {
                app.show_station_commander();
                assert!(app.commander_open);
            }
            app.modifiers = ModifiersState::SHIFT;
            assert_eq!(press(&mut app, digit, None), None);
            assert!(!app.commander_open && !app.menu_active());
            assert_eq!(app.notice.text.as_deref(), Some(message.as_str()));
            assert_eq!(
                app.game.session.help_review(),
                expected.game.session.help_review()
            );
            assert!(!dir.0.join("commander.meridian").exists());
        }
    }
}

#[test]
fn control_shortcuts_preserve_modal_enter_and_use_the_correct_save_slot() {
    use winit::keyboard::ModifiersState;
    let dir = TestDirectory::new();
    let mut ordinary = app(&dir);
    let fleet = dir.0.join("commander.meridian");
    press(&mut ordinary, KeyCode::KeyH, None);
    ordinary.modifiers = ModifiersState::CONTROL;
    for code in [KeyCode::KeyS, KeyCode::KeyR, KeyCode::Enter] {
        assert_eq!(press(&mut ordinary, code, None), None);
        assert!(ordinary.help.is_some());
    }
    assert!(!fleet.exists());
    ordinary.modifiers = ModifiersState::empty();
    press(&mut ordinary, KeyCode::Escape, None);
    assert!(ordinary.help.is_none());
    ordinary.modifiers = ModifiersState::CONTROL;
    assert_eq!(press(&mut ordinary, KeyCode::KeyS, Some("s")), None);
    assert!(!ordinary.keys.contains(&KeyCode::KeyS));
    let saved = ok(std::fs::read(&fleet));
    let opening = ordinary.game.world.snapshot();
    ordinary.game_key(KeyCode::KeyL);
    ordinary
        .game
        .world
        .step(FlightInput::default())
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_ne!(ordinary.game.world.snapshot(), opening);
    assert_eq!(press(&mut ordinary, KeyCode::KeyR, Some("r")), None);
    assert_eq!(ordinary.game.world.snapshot(), opening);
    let mut preview = App::with_profile(ok(crate::profile::Profile::open_medusa_preview(
        dir.0.clone(),
    )));
    preview.modifiers = ModifiersState::CONTROL;
    assert_eq!(press(&mut preview, KeyCode::KeyS, None), None);
    assert!(!preview.keys.contains(&KeyCode::KeyS));
    let prior = preview.game.world.snapshot();
    assert!(dir.0.join("medusa-preview.meridian").is_file());
    ok(preview.game.world.request_launch(prior.tick));
    ok(preview.game.world.step(FlightInput::default()));
    assert_eq!(press(&mut preview, KeyCode::KeyR, None), None);
    assert_eq!(preview.game.world.snapshot(), prior);
    assert_eq!(ok(std::fs::read(fleet)), saved);
}

#[test]
fn flight_status_cycles_notice_positions_and_keeps_the_last_launch_notice() {
    use crate::hud::cockpit::notice::Placement;
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    let launch = ok(app.game.command(Action::Launch));
    app.notice.set(launch.clone());
    ok(app.game.world.step(FlightInput::default()));
    let before = ok(app.game.world.save_bytes());
    let prepared = ok(app.prepare_presentation(true, None, 0., |_| None));
    assert_eq!(prepared.hud_lines, [launch.clone()]);
    let deadline = app
        .notice
        .expires
        .unwrap_or_else(|| panic!("timed launch feedback"));
    assert!(app.expire_hud_notice(deadline));
    press(&mut app, KeyCode::KeyH, Some("h"));
    press(&mut app, KeyCode::KeyI, Some("i"));
    for expected in [
        Placement::Instruments,
        Placement::UpperLeft,
        Placement::UpperRight,
        Placement::StatusOnly,
        Placement::Instruments,
    ] {
        assert_eq!(app.notice_placement, expected);
        let panel = app.help.as_ref().unwrap_or_else(|| panic!("status card"));
        assert!(
            panel
                .lines
                .iter()
                .any(|line| line == &format!("Notice position: {}", expected.name()))
        );
        assert!(panel.lines.iter().any(|line| line.contains(&launch)));
        assert!(panel.footer.contains("N Change notice position"));
        press(&mut app, KeyCode::KeyN, Some("n"));
    }
    press(&mut app, KeyCode::Escape, None);
    assert_eq!(ok(app.game.world.save_bytes()), before);
}

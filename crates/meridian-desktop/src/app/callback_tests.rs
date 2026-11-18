//! Callback decisions exercised without an event loop, window, GPU or private player save.
use super::{App, FrameAdvance, step_frame};
use crate::{
    docked_screens as ui,
    profile::tests::{TestDirectory, open},
};
use meridian_core::{
    FIXED_DELTA_SECONDS, FlightInput, FlightMode, FlightStart, MotionRefusal, StepError, World,
};
use winit::{
    event::ElementState,
    keyboard::{KeyCode, NativeKeyCode, PhysicalKey},
};

fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn app(directory: &TestDirectory) -> App {
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(98011)));
    App::with_profile(profile)
}
fn launched(app: &mut App) {
    ok(app
        .game
        .world
        .request_launch(app.game.world.snapshot().tick));
    for _ in 0..600 {
        if app.game.world.snapshot().mode == FlightMode::Flight {
            break;
        }
        assert_eq!(app.advance_frame(true, 1), FrameAdvance::Continue(None));
    }
    assert_eq!(app.game.world.snapshot().mode, FlightMode::Flight);
}
fn key(
    app: &mut App,
    code: KeyCode,
    state: ElementState,
    repeat: bool,
    text: Option<&str>,
) -> Option<KeyCode> {
    app.route_keyboard(PhysicalKey::Code(code), state, repeat, text)
}
fn paint(app: &mut App) -> Vec<String> {
    let menu = app.docked.as_mut().unwrap_or_else(|| panic!("menu"));
    let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
    let frame = ok(menu.build_frame(&app.game.world, viewport, 0.0));
    let labels = frame
        .labels
        .iter()
        .map(|label| label.text.clone())
        .collect();
    menu.presented(frame);
    labels
}

#[test]
fn redraw_preparation_routes_actual_hud_menu_and_inactive_frames_without_committing() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    ok(app.game.save());
    let before = ok(app.game.world.save_bytes());
    let disk = ok(std::fs::read(directory.0.join("commander.meridian")));
    app.notice.set("Fallback notice".into());
    let mut menu = ok(crate::docked::Docked::new(&app.game.world));
    let mut reference = ok(crate::docked::Docked::new(&app.game.world));
    ok(menu.open(&app.game.world, ui::Screen::System));
    ok(reference.open(&app.game.world, ui::Screen::System));
    app.docked = Some(menu);
    let expected_hud = app.game.lines(true, Some("Pulse fired"));
    let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
    let expected_menu = ok(reference.build_frame(&app.game.world, viewport, 1.25));
    let result = app.prepare_presentation(true, Some("Pulse fired"), 1.25, |renderer| {
        assert!(renderer.is_none());
        Some([1280.0, 720.0])
    });
    let prepared = App::route_presentation(result, |error| panic!("unexpected failure: {error}"))
        .unwrap_or_else(|| panic!("ready frame discarded"));
    assert_eq!(prepared.hud_lines, expected_hud);
    assert_eq!(prepared.docked_frame, Some(expected_menu));
    assert_eq!(app.notice.text.as_deref(), Some("Fallback notice"));
    app.docked
        .as_mut()
        .unwrap_or_else(|| panic!("menu"))
        .close();
    let expected_hud = app.game.lines(false, Some("Fallback notice"));
    let result =
        app.prepare_presentation(false, None, 0.5, |_| panic!("inactive viewport queried"));
    let prepared =
        App::route_presentation(result, |error| panic!("inactive frame failed: {error}"))
            .unwrap_or_else(|| panic!("inactive frame treated as fatal"));
    assert_eq!(prepared.hud_lines, expected_hud);
    assert!(prepared.docked_frame.is_none());
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(
        ok(std::fs::read(directory.0.join("commander.meridian"))),
        disk
    );
}

#[test]
fn redraw_error_router_reports_actual_menu_failure_once_without_forwarding_a_frame() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    ok(app.game.save());
    let before = ok(app.game.world.save_bytes());
    let disk = ok(std::fs::read(directory.0.join("commander.meridian")));
    let mut menu = ok(crate::docked::Docked::new(&app.game.world));
    ok(menu.open(&app.game.world, ui::Screen::Trading));
    let failure = App::build_docked_frame(
        true,
        || Some([640.0, 360.0]),
        Some(&mut menu),
        &World::new(),
        0.0,
    );
    let prepared = failure.map(|docked_frame| super::PreparedPresentation {
        hud_lines: Vec::new(),
        docked_frame,
    });
    let mut reported = Vec::new();
    let ready = App::route_presentation(prepared, |error| reported.push(error));
    assert!(ready.is_none());
    assert_eq!(reported, vec![String::from("No equipment account")]);
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(
        ok(std::fs::read(directory.0.join("commander.meridian"))),
        disk
    );
}

#[test]
fn inactive_and_unusable_menu_viewports_do_not_advance_a_frame_or_the_world() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    let before = ok(app.game.world.save_bytes());
    assert!(ok(app.docked_frame(0.5, |_| panic!("inactive viewport queried"))).is_none());
    let mut menu = ok(crate::docked::Docked::new(&app.game.world));
    ok(menu.open(&app.game.world, ui::Screen::Trading));
    app.docked = Some(menu);
    for viewport in [
        None,
        Some([319.0, 240.0]),
        Some([640.0, 239.0]),
        Some([f64::NAN, 720.0]),
        Some([640.0, f64::INFINITY]),
    ] {
        assert!(
            ok(app.docked_frame(0.5, |renderer| {
                assert!(renderer.is_none());
                viewport
            }))
            .is_none()
        );
    }
    let frame = ok(app.docked_frame(0.0, |_| Some([640.0, 360.0])))
        .unwrap_or_else(|| panic!("valid frame absent"));
    assert_eq!(frame.stamp.revision, 1);
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert!(
        ok(App::build_docked_frame(
            true,
            || Some([640.0, 360.0]),
            None,
            &app.game.world,
            0.0
        ))
        .is_none()
    );
}

#[test]
fn app_menu_preparation_builds_the_same_actual_frame_without_committing_a_command() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    ok(app.game.save());
    let before = ok(app.game.world.save_bytes());
    let disk = ok(std::fs::read(directory.0.join("commander.meridian")));
    let mut menu = ok(crate::docked::Docked::new(&app.game.world));
    let mut reference = ok(crate::docked::Docked::new(&app.game.world));
    ok(menu.open(&app.game.world, ui::Screen::System));
    ok(reference.open(&app.game.world, ui::Screen::System));
    app.docked = Some(menu);
    let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
    let expected = ok(reference.build_frame(&app.game.world, viewport, 1.25));
    let actual = ok(app.docked_frame(1.25, |_| Some([1280.0, 720.0])))
        .unwrap_or_else(|| panic!("active frame absent"));
    assert_eq!(actual, expected);
    assert!(!actual.labels.is_empty());
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(
        ok(std::fs::read(directory.0.join("commander.meridian"))),
        disk
    );
}

#[test]
fn actual_menu_build_failure_is_returned_without_a_frame_or_world_mutation() {
    let directory = TestDirectory::new();
    let app = app(&directory);
    let mut menu = ok(crate::docked::Docked::new(&app.game.world));
    ok(menu.open(&app.game.world, ui::Screen::Equipment));
    let unowned = World::new();
    let before = unowned.snapshot();
    let failure = App::build_docked_frame(
        true,
        || Some([1280.0, 720.0]),
        Some(&mut menu),
        &unowned,
        0.25,
    );
    assert_eq!(failure, Err("No equipment account".into()));
    assert_eq!(unowned.snapshot(), before);
    assert!(unowned.equipment_snapshot().is_none());
    let recovered = ok(App::build_docked_frame(
        true,
        || Some([1280.0, 720.0]),
        Some(&mut menu),
        &app.game.world,
        0.0,
    ))
    .unwrap_or_else(|| panic!("valid frame absent"));
    assert_eq!(recovered.stamp.revision, 2);
}

#[test]
fn active_callback_uses_held_axes_and_fire_on_actual_advanced_ticks() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    launched(&mut app);
    let mut reference = ok(World::from_save_bytes(&ok(app.game.world.save_bytes())));
    for code in [
        KeyCode::ArrowUp,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyW,
        KeyCode::KeyZ,
    ] {
        assert_eq!(
            key(&mut app, code, ElementState::Pressed, false, None),
            Some(code)
        );
    }
    let before = app.game.world.snapshot();
    let mut expected_pulse = None;
    let input = FlightInput {
        pitch: 1.0,
        yaw: 1.0,
        roll: 1.0,
        throttle: 1.0,
        brake: false,
    };
    let expected_refusal = ok(step_frame(
        &mut reference,
        input,
        2,
        true,
        &mut expected_pulse,
        &mut None,
    ));
    assert_eq!(
        app.advance_frame(true, 2),
        FrameAdvance::Continue(expected_refusal)
    );
    assert_eq!(app.game.world.snapshot(), reference.snapshot());
    assert_eq!(app.pulse, expected_pulse);
    assert_eq!(app.pulse.map(|shot| shot.tick), Some(before.tick + 1));
    assert!(app.game.world.snapshot().speed > before.speed);
    assert_ne!(app.game.world.snapshot().orientation, before.orientation);
    assert_eq!(
        app.game.world.flight_resources_snapshot(),
        reference.flight_resources_snapshot()
    );
}

#[test]
fn opposite_controls_cancel_and_brake_overrides_the_held_throttle() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    launched(&mut app);
    let before = app.game.world.snapshot();
    for code in [
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::KeyA,
        KeyCode::KeyD,
        KeyCode::KeyQ,
        KeyCode::KeyE,
        KeyCode::KeyW,
        KeyCode::Space,
    ] {
        assert_eq!(
            key(&mut app, code, ElementState::Pressed, false, None),
            Some(code)
        );
    }
    assert_eq!(app.advance_frame(true, 2), FrameAdvance::Continue(None));
    let after = app.game.world.snapshot();
    assert_eq!(after.orientation, before.orientation);
    assert!(after.speed < before.speed);
    assert!(app.pulse.is_none());
}

#[test]
fn inactive_and_zero_tick_callbacks_do_not_step_or_repeat_a_discharge() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    launched(&mut app);
    assert_eq!(
        key(&mut app, KeyCode::KeyZ, ElementState::Pressed, false, None),
        Some(KeyCode::KeyZ)
    );
    assert_eq!(app.advance_frame(true, 1), FrameAdvance::Continue(None));
    let pulse = app.pulse;
    let before = ok(app.game.world.save_bytes());
    app.notice.set("keep existing notice".into());
    assert_eq!(app.advance_frame(false, 600), FrameAdvance::Continue(None));
    assert_eq!(app.advance_frame(true, 0), FrameAdvance::Continue(None));
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(app.pulse, pulse);
    assert_eq!(app.notice.text.as_deref(), Some("keep existing notice"));
}

#[test]
fn actual_paused_world_error_is_fatal_without_stepping_or_exiting_a_fake_loop() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    launched(&mut app);
    ok(app
        .game
        .world
        .request_pause(true, app.game.world.snapshot().tick));
    let before = ok(app.game.world.save_bytes());
    assert_eq!(
        app.advance_frame(true, 1),
        FrameAdvance::Fatal(StepError::Paused)
    );
    assert_eq!(ok(app.game.world.save_bytes()), before);
    // The real redraw edge logs/records this actual error and exits before snapshot/render work.
    assert!(app.error.is_none());
}

#[test]
fn frame_feedback_preserves_contact_precedence_and_error_classification() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    for (contact, message) in [
        (
            MotionRefusal::CelestialContact,
            "Celestial contact: stopped before the physical surface",
        ),
        (
            MotionRefusal::StationContact,
            "Station contact: stopped at last clear position",
        ),
        (
            MotionRefusal::EntryConditions,
            "Entry refused: align with entrance and slow down",
        ),
    ] {
        assert_eq!(
            app.frame_feedback(Ok(Some(contact))),
            FrameAdvance::Continue(Some(contact))
        );
        assert_eq!(app.notice.text.as_deref(), Some(message));
    }
    // Typed step-result fixtures exercise the shared callback classification, not a fabricated World.
    for error in [StepError::Collision, StepError::UnsafeEntry] {
        assert_eq!(app.frame_feedback(Err(error)), FrameAdvance::Continue(None));
        assert_eq!(app.notice.text, Some(error.to_string()));
    }
    let previous = app.notice.clone();
    for error in [
        StepError::NonFiniteInput,
        StepError::InputOutOfRange,
        StepError::TickOverflow,
        StepError::NonFiniteState,
        StepError::StaleCommand,
        StepError::InvalidMode,
        StepError::UnsafeLaunch,
        StepError::Paused,
    ] {
        assert_eq!(app.frame_feedback(Err(error)), FrameAdvance::Fatal(error));
        assert_eq!(app.notice, previous);
    }
}

#[test]
fn app_callback_retains_real_entry_contact_when_the_last_catchup_tick_is_clear() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    // Public native-station flight constructor; no mutation of private simulation fields.
    app.game.world = ok(World::in_flight_at_original_station(FlightStart {
        position: [0.0, 0.0, 260.0],
        orientation: [0.0, 1.0, 0.0, 0.0],
        speed: 9.0,
    }));
    for _ in 0..100 {
        let state = app.game.world.snapshot();
        if state.position[2] - state.speed * 18.0 * FIXED_DELTA_SECONDS <= 136.32 {
            break;
        }
        assert_eq!(app.advance_frame(true, 1), FrameAdvance::Continue(None));
    }
    let before = app.game.world.snapshot();
    assert_eq!(
        app.advance_frame(true, 2),
        FrameAdvance::Continue(Some(MotionRefusal::EntryConditions))
    );
    let after = app.game.world.snapshot();
    assert_eq!(after.motion_refusal, None);
    assert_eq!(after.position, before.position);
    assert_eq!(after.speed, 0.0);
    assert_eq!(after.tick, before.tick + 2);
    assert_eq!(
        app.notice.text.as_deref(),
        Some("Entry refused: align with entrance and slow down")
    );
}

#[test]
fn paused_contact_warning_expires_while_world_retains_the_refusal() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    app.game.world = ok(World::in_flight_at_original_station(FlightStart {
        position: [0.0, 0.0, 260.0],
        orientation: [0.0, 1.0, 0.0, 0.0],
        speed: 9.0,
    }));
    for _ in 0..100 {
        if app.advance_frame(true, 1)
            == FrameAdvance::Continue(Some(MotionRefusal::EntryConditions))
        {
            break;
        }
    }
    let stopped = app.game.world.snapshot();
    assert_eq!(stopped.motion_refusal, Some(MotionRefusal::EntryConditions));
    app.paused = true;
    let warning = "Entry refused: align with entrance and slow down";
    let frame = ok(app.prepare_presentation(false, None, 0.0, |_| None));
    assert_eq!(frame.hud_lines, [warning]);
    let deadline = app
        .notice
        .expires
        .unwrap_or_else(|| panic!("missing notice deadline"));
    assert!(app.expire_hud_notice(deadline));
    assert_eq!(app.advance_frame(false, 0), FrameAdvance::Continue(None));
    let frame = ok(app.prepare_presentation(false, None, 0.0, |_| None));
    assert!(frame.hud_lines.is_empty());
    assert_eq!(app.game.world.snapshot(), stopped);
}

#[test]
fn press_repeat_release_and_unidentified_keys_preserve_held_input_rules() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    assert_eq!(
        key(&mut app, KeyCode::KeyW, ElementState::Pressed, false, None),
        Some(KeyCode::KeyW)
    );
    assert!(app.keys.contains(&KeyCode::KeyW));
    assert_eq!(
        key(&mut app, KeyCode::KeyW, ElementState::Pressed, true, None),
        None
    );
    assert!(app.keys.contains(&KeyCode::KeyW));
    assert_eq!(
        app.route_keyboard(
            PhysicalKey::Unidentified(NativeKeyCode::Unidentified),
            ElementState::Pressed,
            false,
            Some("w")
        ),
        None
    );
    assert!(app.keys.contains(&KeyCode::KeyW));
    app.focused = false;
    app.occluded = true;
    assert_eq!(
        key(&mut app, KeyCode::KeyW, ElementState::Released, false, None),
        None
    );
    assert!(app.keys.is_empty());
    assert_eq!(
        key(&mut app, KeyCode::KeyW, ElementState::Pressed, false, None),
        None
    );
    app.focused = true;
    assert_eq!(
        key(&mut app, KeyCode::KeyW, ElementState::Pressed, false, None),
        None
    );
    assert!(app.keys.is_empty());
}

#[test]
fn docked_repeats_allow_navigation_only_and_never_latch_flight_keys() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    assert!(app.open_menu_key(KeyCode::Digit3));
    paint(&mut app);
    let before = ok(app.game.world.save_bytes());
    for code in [
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Backspace,
    ] {
        assert_eq!(
            key(&mut app, code, ElementState::Pressed, true, None),
            Some(code)
        );
    }
    for code in [KeyCode::KeyG, KeyCode::Enter, KeyCode::F5, KeyCode::KeyZ] {
        assert_eq!(key(&mut app, code, ElementState::Pressed, true, None), None);
    }
    assert_eq!(
        key(&mut app, KeyCode::KeyG, ElementState::Pressed, false, None),
        Some(KeyCode::KeyG)
    );
    assert!(app.keys.is_empty());
    assert_eq!(ok(app.game.world.save_bytes()), before);
}

#[test]
fn presented_search_consumes_printable_repeated_text_before_key_dispatch() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    assert!(app.open_menu_key(KeyCode::Digit3));
    paint(&mut app);
    app.menu_input(ui::Input::Key(ui::Key::Search));
    paint(&mut app);
    assert!(app.docked.as_ref().is_some_and(|menu| menu.search_active()));
    let before = ok(app.game.world.save_bytes());
    assert_eq!(
        key(
            &mut app,
            KeyCode::KeyP,
            ElementState::Pressed,
            false,
            Some("P")
        ),
        None
    );
    let labels = paint(&mut app);
    assert!(labels.iter().any(|s| s == "> P"));
    assert_eq!(
        key(
            &mut app,
            KeyCode::KeyE,
            ElementState::Pressed,
            true,
            Some("e")
        ),
        None
    );
    let labels = paint(&mut app);
    assert!(labels.iter().any(|s| s == "> Pe"));
    assert_eq!(
        key(
            &mut app,
            KeyCode::Enter,
            ElementState::Pressed,
            false,
            Some("\r")
        ),
        Some(KeyCode::Enter)
    );
    assert!(app.keys.is_empty());
    assert_eq!(ok(app.game.world.save_bytes()), before);
}

#[test]
fn focus_and_occlusion_resets_clear_held_actions_and_preserve_pause_and_clock_rules() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    launched(&mut app);
    assert_eq!(
        key(&mut app, KeyCode::KeyZ, ElementState::Pressed, false, None),
        Some(KeyCode::KeyZ)
    );
    assert_eq!(app.advance_frame(true, 1), FrameAdvance::Continue(None));
    assert!(app.pulse.is_some());
    app.dragging = true;
    assert_eq!(app.clock.ticks(1.0 / 120.0, true), 0);
    let tick = app.game.world.snapshot().tick;
    app.focus_changed(false);
    assert!(app.keys.is_empty());
    assert!(!app.dragging);
    assert!(app.pulse.is_none());
    assert!(
        app.game
            .world
            .mission_snapshot()
            .is_some_and(|mission| mission.paused)
    );
    assert_eq!(app.game.world.snapshot().tick, tick);
    assert_eq!(app.clock.ticks(600.0, true), 0);
    assert_eq!(app.clock.ticks(1.0 / 60.0, true), 1);
    app.focus_changed(true);
    assert!(
        !app.game
            .world
            .mission_snapshot()
            .is_some_and(|mission| mission.paused)
    );
    app.occlusion_changed(true);
    assert!(
        app.game
            .world
            .mission_snapshot()
            .is_some_and(|mission| mission.paused)
    );
    app.paused = true;
    app.occlusion_changed(false);
    assert!(
        app.game
            .world
            .mission_snapshot()
            .is_some_and(|mission| mission.paused)
    );
    app.paused = false;
    app.focus_changed(true);
    assert!(
        !app.game
            .world
            .mission_snapshot()
            .is_some_and(|mission| mission.paused)
    );
}

#[test]
fn focus_loss_discards_presented_search_authority_without_changing_wallet_or_fit() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    assert!(app.open_menu_key(KeyCode::Digit3));
    paint(&mut app);
    app.menu_input(ui::Input::Key(ui::Key::Search));
    paint(&mut app);
    assert!(app.docked.as_ref().is_some_and(|menu| menu.search_active()));
    let credits = app.game.world.credits_tenths();
    let modules = app.game.world.modular_equipment_snapshot();
    app.focus_changed(false);
    assert!(!app.docked.as_ref().is_some_and(|menu| menu.search_active()));
    assert_eq!(
        key(
            &mut app,
            KeyCode::KeyG,
            ElementState::Pressed,
            false,
            Some("g")
        ),
        None
    );
    assert_eq!(app.game.world.credits_tenths(), credits);
    assert_eq!(app.game.world.modular_equipment_snapshot(), modules);
}

#[test]
fn flight_feedback_expires_once_at_its_deadline_and_repeated_input_renews_it() {
    let mut notice = super::Notice::default();
    notice.set("Entry refused".into());
    let first = notice.expires.unwrap_or_else(|| panic!("notice deadline"));
    assert!(!notice.expire(first - std::time::Duration::from_millis(1)));
    notice.set("Entry refused".into());
    let renewed = notice.expires.unwrap_or_else(|| panic!("renewed deadline"));
    assert!(renewed >= first);
    assert_eq!(notice.text.as_deref(), Some("Entry refused"));
    assert!(notice.expire(renewed));
    assert!(notice.text.is_none());
    assert!(!notice.expire(renewed));
}

fn startup(directory: &TestDirectory) -> App {
    ok(App::from_launch(super::mule_validation::Launch::Profile(
        directory.0.clone(),
    )))
}

fn commander_labels(app: &mut App) -> Vec<String> {
    ok(
        app.prepare_presentation(app.simulation_active(), None, 0.0, |_| {
            Some([1280.0, 720.0])
        }),
    )
    .docked_frame
    .unwrap_or_else(|| panic!("missing Commander frame"))
    .labels
    .into_iter()
    .map(|label| label.text)
    .collect()
}

// Keep the real paused berth and all other save fields. Set the review epoch one
// below its limit after startup so a resume alone fits but resume plus launch refuses.
fn near_exhausted_paused_dock_save() -> Vec<u8> {
    let mut world = ok(World::new_commander(98013));
    ok(world.request_pause(true, world.snapshot().tick));
    let before = ok(world.save_bytes());
    ok(world.request_pause(true, world.snapshot().tick));
    let mut bytes = ok(world.save_bytes());
    let body_len = bytes.len() - size_of::<u64>();
    let mut changed = before[..body_len]
        .iter()
        .zip(&bytes[..body_len])
        .enumerate()
        .filter_map(|(index, (old, new))| (old != new).then_some(index));
    let epoch = changed
        .next()
        .unwrap_or_else(|| panic!("missing pause epoch"));
    assert_eq!(changed.next(), None);
    assert_eq!(&bytes[epoch..epoch + 8], &3_u64.to_le_bytes());
    bytes[epoch..epoch + 8].copy_from_slice(&(u64::MAX - 2).to_le_bytes());
    let checksum = bytes[..body_len]
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
        });
    bytes[body_len..].copy_from_slice(&checksum.to_le_bytes());
    bytes
}

#[test]
fn refused_commander_launch_keeps_real_docked_pause_through_immediate_save() {
    let directory = TestDirectory::new();
    let path = directory.0.join("commander.meridian");
    let opening_save = near_exhausted_paused_dock_save();
    ok(std::fs::write(&path, &opening_save));
    let mut app = startup(&directory);
    assert!(app.commander_open && app.paused && app.has_berth());
    let berth = app.game.world.current_dock_context();
    let snapshot = app.game.world.snapshot();
    let paused_before_launch = ok(app.game.world.save_bytes());

    // The genuine World check reaches launch preparation and refuses the exhausted sequence.
    assert!(app.commander_key(KeyCode::KeyL));
    let refusal = StepError::TickOverflow.to_string();
    assert_eq!(app.notice.text.as_deref(), Some(refusal.as_str()));
    assert!(app.commander_open && app.paused && !app.simulation_active());
    assert_eq!(app.game.world.current_dock_context(), berth);
    assert_eq!(app.game.world.snapshot(), snapshot);
    assert_eq!(ok(app.game.world.save_bytes()), paused_before_launch);
    assert_eq!(ok(std::fs::read(&path)), opening_save);

    assert!(!app.commander_key(KeyCode::F5));
    app.save_profile();
    let saved = ok(std::fs::read(&path));
    assert_eq!(saved, paused_before_launch);
    let continued = ok(World::from_save_bytes(&saved));
    assert!(
        continued
            .mission_snapshot()
            .is_some_and(|mission| mission.paused)
    );
    assert_eq!(continued.snapshot(), snapshot);
    assert_eq!(continued.current_dock_context(), berth);
}

#[test]
fn accepted_commander_launch_resumes_paused_world_and_saves_the_departure() {
    let directory = TestDirectory::new();
    let path = directory.0.join("commander.meridian");
    let mut world = ok(World::new_commander(98014));
    ok(world.request_pause(true, world.snapshot().tick));
    ok(std::fs::write(&path, ok(world.save_bytes())));
    let mut app = startup(&directory);
    assert!(app.commander_open && app.paused && app.has_berth());
    let tick = app.game.world.snapshot().tick;
    assert_eq!(app.game.world.request_launch(tick), Err(StepError::Paused));

    assert!(app.commander_key(KeyCode::KeyL));
    assert!(!app.commander_open && !app.paused && app.simulation_active());
    let snapshot = app.game.world.snapshot();
    assert_eq!(snapshot.mode, FlightMode::Launching);
    assert_eq!(snapshot.tick, tick);
    assert_eq!(app.game.world.current_dock_context(), None);
    assert!(
        app.game
            .world
            .mission_snapshot()
            .is_some_and(|mission| !mission.paused)
    );

    app.save_profile();
    let continued = ok(World::from_save_bytes(&ok(std::fs::read(&path))));
    assert_eq!(continued.snapshot(), snapshot);
    assert!(
        continued
            .mission_snapshot()
            .is_some_and(|mission| !mission.paused)
    );
}

fn docked_exterior(directory: &TestDirectory) -> App {
    let mut app = startup(directory);
    assert!(app.commander_key(KeyCode::Digit2));
    paint(&mut app);
    app.menu_input(ui::Input::Key(ui::Key::Back));
    assert!(app.has_berth() && !app.commander_open && !app.menu_active());
    app
}

#[test]
fn ordinary_docked_exterior_clears_timed_feedback_above_the_instruments() {
    let directory = TestDirectory::new();
    let mut app = docked_exterior(&directory);
    let idle = ok(app.prepare_presentation(true, None, 0.0, |_| None));
    assert!(idle.hud_lines.is_empty());
    assert!(idle.docked_frame.is_none());
    assert!(app.timed_hud_notices());

    app.game_key(KeyCode::KeyT);
    let deadline = app
        .notice
        .expires
        .unwrap_or_else(|| panic!("notice deadline"));
    let feedback = ok(app.prepare_presentation(true, None, 0.0, |_| None));
    assert_eq!(feedback.hud_lines.len(), 1);
    assert!(feedback.hud_lines[0].starts_with("Speaking with"));
    assert!(feedback.hud_lines[0].contains("R reviews freight terms"));
    assert!(!app.expire_hud_notice(deadline - std::time::Duration::from_millis(1)));

    app.paused = true;
    app.reset_input();
    assert!(!app.simulation_active());
    let before_expiry = ok(app.game.world.save_bytes());
    assert!(app.expire_hud_notice(deadline));
    let expired = ok(app.prepare_presentation(false, None, 0.0, |_| None));
    assert!(expired.hud_lines.is_empty());
    assert!(expired.docked_frame.is_none());
    assert_eq!(ok(app.game.world.save_bytes()), before_expiry);

    let cockpit = ok(crate::hud::cockpit::Cockpit::capture(
        &app.game.world,
        app.game.world.snapshot(),
    ));
    let draw = |lines: &[String]| {
        let mut vertices = Vec::new();
        crate::hud::vertices_with_cockpit(lines, 1280, 720, Some(&cockpit), &mut vertices);
        vertices.len()
    };
    assert!(draw(&expired.hud_lines) > 0);
    assert!(draw(&feedback.hud_lines) > draw(&expired.hud_lines));
}

#[test]
fn docked_freight_review_stays_accessible_after_feedback_expires_and_help_closes() {
    let directory = TestDirectory::new();
    let mut app = docked_exterior(&directory);
    app.game_key(KeyCode::KeyT);
    app.game_key(KeyCode::KeyR);
    let deadline = app
        .notice
        .expires
        .unwrap_or_else(|| panic!("notice deadline"));
    assert!(app.expire_hud_notice(deadline));
    let review = ok(app.prepare_presentation(true, None, 0.0, |_| None));
    let content = review.hud_lines.join(" ");
    assert!(content.contains("730 CR ON COMPLETION | ENTER ACCEPTS | B CANCELS"));
    assert!(content.contains(meridian_core::freight::g02::presentation::OFFER_BRIEFING));
    assert_eq!(
        app.game
            .world
            .mission_snapshot()
            .map(|m| m.freight.held_tonnes),
        Some(0)
    );

    assert_eq!(
        key(&mut app, KeyCode::KeyH, ElementState::Pressed, false, None),
        None
    );
    let help = app
        .help
        .as_ref()
        .unwrap_or_else(|| panic!("missing review help"));
    let controls = help.lines.join(" ");
    assert!(help.title.contains("Freight review"));
    for control in [
        "0 Commander",
        "1 Galaxy",
        "4 Market",
        "5 Equipment",
        "B cancels",
    ] {
        assert!(controls.contains(control), "{control}: {controls}");
    }
    assert_eq!(
        key(&mut app, KeyCode::Enter, ElementState::Pressed, false, None),
        None
    );
    assert_eq!(
        app.game
            .world
            .mission_snapshot()
            .map(|m| m.freight.held_tonnes),
        Some(0)
    );
    assert_eq!(
        key(
            &mut app,
            KeyCode::Escape,
            ElementState::Pressed,
            false,
            None
        ),
        None
    );
    assert!(app.help.is_none());
    assert_eq!(
        ok(app.prepare_presentation(true, None, 0.0, |_| None)).hud_lines,
        review.hud_lines
    );
    assert_eq!(
        key(&mut app, KeyCode::Enter, ElementState::Pressed, false, None),
        Some(KeyCode::Enter)
    );
    app.game_key(KeyCode::Enter);
    assert_eq!(
        app.game
            .world
            .mission_snapshot()
            .map(|m| m.freight.held_tonnes),
        Some(8)
    );
    assert!(app.game.session.help_review().is_none());
}

#[test]
fn ordinary_start_opens_fresh_commander_with_live_station_bindings_and_without_a_save() {
    let directory = TestDirectory::new();
    let path = directory.0.join("commander.meridian");
    let mut app = startup(&directory);
    assert!(app.commander_open && app.has_berth() && !app.simulation_active());
    let labels = commander_labels(&mut app);
    assert_eq!(labels[0], "COMMANDER DETAILS");
    for value in [
        "Commander ID",
        "SH025",
        "Avenor",
        "720.0 CR",
        "5 Equipment",
        "L Launch",
    ] {
        assert!(
            labels.iter().any(|label| label.contains(value)),
            "{value}: {labels:?}"
        );
    }
    assert!(!path.exists());

    assert_eq!(
        key(&mut app, KeyCode::KeyH, ElementState::Pressed, false, None),
        None
    );
    assert_eq!(
        app.help.as_ref().map(|panel| panel.title.as_str()),
        Some("Commander details")
    );
    assert_eq!(
        key(&mut app, KeyCode::KeyL, ElementState::Pressed, false, None),
        None
    );
    assert_eq!(
        key(
            &mut app,
            KeyCode::Escape,
            ElementState::Pressed,
            false,
            None
        ),
        None
    );
    assert!(app.commander_open && app.has_berth());

    for (code, screen) in [
        (KeyCode::Digit1, ui::Screen::Galaxy),
        (KeyCode::Digit2, ui::Screen::System),
        (KeyCode::Digit3, ui::Screen::Showroom),
        (KeyCode::Digit4, ui::Screen::Trading),
        (KeyCode::Digit5, ui::Screen::Equipment),
    ] {
        assert!(app.commander_key(code));
        assert!(app.menu_active());
        assert_eq!(app.docked.as_ref().map(|menu| menu.screen()), Some(screen));
        app.show_station_commander();
        assert!(app.commander_open && !app.menu_active() && !app.simulation_active());
    }
    assert!(!path.exists());
    assert!(app.commander_key(KeyCode::KeyL));
    assert!(!app.commander_open);
    assert_eq!(app.game.world.snapshot().mode, FlightMode::Launching);
    assert!(app.simulation_active());
    assert!(!path.exists());
}

#[test]
fn ordinary_cold_airborne_save_is_held_at_exact_snapshot_until_explicit_resume() {
    let directory = TestDirectory::new();
    let path = directory.0.join("commander.meridian");
    let expected = {
        let mut app = app(&directory);
        launched(&mut app);
        let snapshot = app.game.world.snapshot();
        ok(app.game.world.request_pause(true, snapshot.tick));
        ok(app.game.save());
        snapshot
    };
    let bytes = ok(std::fs::read(&path));
    let mut app = startup(&directory);
    assert!(app.commander_open && app.game.playing() && !app.has_berth());
    assert_eq!(app.game.world.snapshot(), expected);
    let labels = commander_labels(&mut app);
    assert!(
        labels
            .iter()
            .any(|label| label.contains("Enter Resume flight"))
    );
    assert!(!labels.iter().any(|label| label.contains("Market")));
    app.focus_changed(false);
    app.focus_changed(true);
    app.occlusion_changed(true);
    app.occlusion_changed(false);
    for blocked in [
        KeyCode::Digit2,
        KeyCode::Digit4,
        KeyCode::KeyL,
        KeyCode::KeyW,
    ] {
        assert!(app.commander_key(blocked));
    }
    assert_eq!(
        key(&mut app, KeyCode::Enter, ElementState::Pressed, true, None),
        None
    );
    assert!(!app.simulation_active());
    assert!(
        app.game
            .world
            .mission_snapshot()
            .is_some_and(|mission| mission.paused)
    );
    assert_eq!(app.game.world.snapshot(), expected);
    assert_eq!(ok(std::fs::read(&path)), bytes);

    assert!(app.commander_key(KeyCode::Enter));
    assert!(!app.commander_open && app.simulation_active());
    assert_eq!(app.game.world.snapshot(), expected);
    assert_eq!(
        app.advance_frame(app.simulation_active(), 1),
        FrameAdvance::Continue(None)
    );
    assert_eq!(app.game.world.snapshot().tick, expected.tick + 1);
    assert_eq!(ok(std::fs::read(&path)), bytes);
}

#[test]
fn corrupt_and_unsupported_startup_keep_the_slot_until_confirmed_replacement_is_saved() {
    let samples: [&[u8]; 2] = [
        b"damaged commander",
        include_bytes!("../../../meridian-core/tests/fixtures/pr44-v16-m02-purchases.bin"),
    ];
    for bytes in samples {
        let directory = TestDirectory::new();
        let path = directory.0.join("commander.meridian");
        ok(std::fs::write(&path, bytes));
        let mut app = startup(&directory);
        assert!(app.commander_open && !app.game.playing() && !app.has_berth());
        assert!(
            commander_labels(&mut app)
                .iter()
                .any(|line| line.contains("startup refused"))
        );
        assert!(app.commander_key(KeyCode::Digit2));
        assert!(!app.commander_key(KeyCode::KeyC));
        app.load_profile();
        assert!(
            app.notice
                .text
                .as_deref()
                .is_some_and(|notice| notice.contains("Continue refused"))
        );
        ok(app.game.prepare_quit());
        assert_eq!(ok(std::fs::read(&path)), bytes);

        assert!(!ok(app.game.new_commander(98012)));
        assert_eq!(
            key(&mut app, KeyCode::KeyN, ElementState::Pressed, true, None),
            None
        );
        assert!(!app.game.playing());
        assert!(ok(app.game.new_commander(98012)));
        app.sync_profile_controls();
        assert!(app.commander_open && app.has_berth());
        assert_eq!(ok(std::fs::read(&path)), bytes);
        ok(app.game.save());
        assert_ne!(ok(std::fs::read(&path)), bytes);
    }
}

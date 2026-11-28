//! Authored, UNRUN. Pure host/core lifecycle with isolated test directories, no GPU/window.
use super::*;
use crate::app::{App, FrameAdvance};
use crate::profile::tests::TestDirectory;
use meridian_core::{FlightInput, XyzzyAction};
use winit::{
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};
fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn args(values: &[&str]) -> impl Iterator<Item = OsString> {
    values.iter().map(|value| OsString::from(*value))
}
fn profile(view: View) -> Profile {
    ok(Launch::Mule(view).open())
}

#[test]
fn opt_in_parser_accepts_only_six_exact_internal_forms_and_keeps_normal_save_slot() {
    for (name, view) in [
        ("front", View::Front),
        ("rear", View::Rear),
        ("far", View::Far),
        ("side", View::Side),
        ("dock", View::Dock),
        ("dock-obstructed", View::DockObstructed),
    ] {
        assert_eq!(
            ok(Launch::parse(args(&["--internal-mule-validation", name]))),
            Launch::Mule(view)
        );
    }
    assert_eq!(
        ok(Launch::parse(args(&["--save-dir", "fixture-slot"]))),
        Launch::Profile(PathBuf::from("fixture-slot"))
    );
    for values in [
        vec!["--internal-mule-validation"],
        vec!["--internal-mule-validation", ""],
        vec!["--internal-mule-validation", "FRONT"],
        vec!["--internal-mule-validation", "unknown"],
        vec!["--internal-mule-validation", "front", "--save-dir", "slot"],
        vec!["--save-dir", "slot", "--internal-mule-validation", "front"],
        vec!["--internal-mule-validation", "front", "rear"],
        vec!["--internal-mule-validation", "dock", "--save-dir", "slot"],
        vec![
            "--internal-mule-validation",
            "front",
            "--internal-mule-validation",
            "front",
        ],
    ] {
        assert!(Launch::parse(args(&values)).is_err(), "{values:?}");
    }
}

#[test]
fn medusa_preview_launch_option_refuses_mixed_or_missing_profile_arguments() {
    assert_eq!(
        ok(Launch::parse(args(&[
            "--medusa-preview-dir",
            "preview-slot"
        ]))),
        Launch::MedusaPreview(PathBuf::from("preview-slot"))
    );
    for values in [
        vec!["--medusa-preview-dir"],
        vec!["--medusa-preview-dir", ""],
        vec!["--medusa-preview-dir", "--save-dir"],
        vec!["--medusa-preview-dir", "slot", "--save-dir", "other"],
        vec!["--save-dir", "slot", "--medusa-preview-dir", "other"],
        vec![
            "--medusa-preview-dir",
            "slot",
            "--internal-mule-validation",
            "front",
        ],
    ] {
        assert!(Launch::parse(args(&values)).is_err(), "{values:?}");
    }
}

#[test]
fn all_source_derived_views_publish_actual_fresh_scoped_world_and_stationary_pilot() {
    let q = profile(View::Far);
    for view in [View::Front, View::Rear, View::Far, View::Side] {
        let p = profile(view);
        let frame = p
            .world
            .mule_actor_snapshot()
            .unwrap_or_else(|| panic!("actual frame"));
        assert!(p.is_internal_validation());
        assert!(p.playing());
        assert!(p.world.commander_id().is_none());
        assert!(p.world.player_ship_id().is_none());
        assert!(p.world.traffic_snapshot().is_none());
        assert!(p.world.save_bytes().is_err());
        let observer = p
            .world
            .mule_observer_pose(&frame)
            .unwrap_or_else(|| panic!("scoped observer"));
        let s = p.world.snapshot();
        assert_eq!(s.speed, 0.0);
        assert_eq!(
            [
                observer.position.x as f32,
                observer.position.y as f32,
                observer.position.z as f32
            ],
            s.position
        );
        assert_eq!(frame.actor().hull_id, "SH001");
        assert_eq!(frame.world_tick(), s.tick);
        assert_eq!(
            frame.source_identity(),
            (
                meridian_core::mule::MULE_RESOURCE_SHA256,
                meridian_core::mule::MULE_CONTRACT_SHA256
            )
        );
        let actual = crate::mule_render::Scene {
            actor: frame.actor(),
            envelope: frame.presentation().as_ref(),
            observer,
        };
        assert_eq!(actual.actor.id, frame.actor().id);
        assert!(q.world.mule_observer_pose(&frame).is_none());
        let delta = meridian_core::docking::Vec3::new(
            observer.position.x - actual.actor.pose.position.x,
            observer.position.y - actual.actor.pose.position.y,
            observer.position.z - actual.actor.pose.position.z,
        );
        let source = ok(MuleHullSource::bundled());
        assert!(
            delta.dot(delta).sqrt()
                > source.maximum_origin_radius() + ok(observer_source()).0 * 1.001
        );
        if view == View::Far {
            assert!(delta.dot(delta).sqrt() - source.maximum_origin_radius() > 128.0);
        }
    }
}

#[test]
fn no_profile_storage_or_command_can_change_the_scenario_or_existing_save() {
    let directory = TestDirectory::new();
    let mut normal = ok(Profile::open(directory.0.clone()));
    assert!(ok(normal.new_commander(917)));
    ok(normal.save());
    let save = directory.0.join("commander.meridian");
    let before = ok(std::fs::read(&save));
    let mut internal = profile(View::Front);
    let state = internal.world.snapshot();
    let frame = internal
        .world
        .mule_actor_snapshot()
        .unwrap_or_else(|| panic!("frame"));
    assert!(internal.save().is_err());
    assert!(internal.continue_saved().is_err());
    assert!(internal.new_commander(918).is_err());
    assert!(internal.xyzzy(XyzzyAction::Enter).is_err());
    for action in [
        crate::session::Action::Launch,
        crate::session::Action::Emp(meridian_core::EmpEquipmentAction::Purchase),
        crate::session::Action::FireMissile,
        crate::session::Action::SetFlightView(meridian_core::FlightView::Rear),
    ] {
        assert!(internal.command(action).is_err());
    }
    ok(internal.prepare_quit());
    assert_eq!(internal.world.snapshot(), state);
    assert!(internal.world.mule_snapshot_is_current(&frame));
    assert_eq!(ok(std::fs::read(&save)), before);
    let bad = vec![
        OsString::from("--internal-mule-validation"),
        OsString::from("front"),
        OsString::from("--save-dir"),
        directory.0.join("must-not-exist").into_os_string(),
    ];
    assert!(Launch::parse(bad.into_iter()).is_err());
    assert!(!directory.0.join("must-not-exist").exists());
    drop(normal);
    let mut cold = ok(Profile::open(directory.0.clone()));
    ok(cold.continue_saved());
    assert!(!cold.is_internal_validation());
    assert!(cold.world.mule_actor_snapshot().is_none());
    assert_eq!(ok(std::fs::read(&save)), before);
}

#[test]
fn paused_step_and_running_path_advance_only_the_real_mule_with_frozen_observer() {
    let mut app = App::with_profile(profile(View::Front));
    assert!(app.paused);
    assert!(!app.simulation_active());
    let state = app.game.world.snapshot();
    let first = app
        .game
        .world
        .mule_actor_snapshot()
        .unwrap_or_else(|| panic!("frame"));
    assert!(matches!(
        app.advance_frame(false, 6),
        FrameAdvance::Continue(_)
    ));
    assert_eq!(app.game.world.snapshot(), state);
    assert_eq!(
        ok(app.validation_control(KeyCode::Period)),
        Control::Updated
    );
    let next = app
        .game
        .world
        .mule_actor_snapshot()
        .unwrap_or_else(|| panic!("frame"));
    assert_eq!(next.world_tick(), first.world_tick() + 1);
    assert_eq!(next.generation(), first.generation() + 1);
    assert_eq!(next.actor().id, first.actor().id);
    assert!(!app.game.world.mule_snapshot_is_current(&first));
    assert_eq!(app.game.world.snapshot().position, state.position);
    assert_eq!(app.game.world.snapshot().orientation, state.orientation);
    assert!(app.paused);
    assert_eq!(ok(app.validation_control(KeyCode::KeyP)), Control::Updated);
    assert!(app.simulation_active());
    for code in [KeyCode::KeyW, KeyCode::KeyU, KeyCode::KeyZ, KeyCode::Space] {
        app.keys.insert(code);
    }
    assert!(matches!(
        app.advance_frame(true, 1),
        FrameAdvance::Continue(_)
    ));
    assert_eq!(app.game.world.snapshot().tick, state.tick + 2);
    assert_eq!(app.game.world.snapshot().position, state.position);
    assert!(app.pulse.is_none());
    assert!(app.mining.is_none());
    assert_eq!(
        ok(app.validation_control(KeyCode::Period)),
        Control::Ignored
    );
    assert_eq!(app.game.world.snapshot().tick, state.tick + 2);
    assert_eq!(ok(app.validation_control(KeyCode::KeyP)), Control::Updated);
    assert!(app.paused);
    assert_eq!(ok(app.validation_control(KeyCode::Escape)), Control::Quit);
    ok(app.game.prepare_quit());
}

#[test]
fn internal_keyboard_focus_and_repeat_cannot_move_observer_or_open_profile_actions() {
    let mut app = App::with_profile(profile(View::Rear));
    let state = app.game.world.snapshot();
    for code in [
        KeyCode::KeyN,
        KeyCode::KeyC,
        KeyCode::KeyS,
        KeyCode::KeyR,
        KeyCode::KeyW,
        KeyCode::KeyU,
        KeyCode::KeyZ,
        KeyCode::KeyH,
        KeyCode::KeyX,
    ] {
        assert_eq!(
            app.route_keyboard(PhysicalKey::Code(code), ElementState::Pressed, false, None),
            None
        );
        assert!(app.keys.is_empty());
    }
    assert_eq!(
        app.route_keyboard(
            PhysicalKey::Code(KeyCode::Period),
            ElementState::Pressed,
            true,
            None
        ),
        None
    );
    assert_eq!(
        app.route_keyboard(
            PhysicalKey::Code(KeyCode::Period),
            ElementState::Pressed,
            false,
            None
        ),
        Some(KeyCode::Period)
    );
    app.focus_changed(false);
    assert!(!app.simulation_active());
    assert_eq!(
        app.route_keyboard(
            PhysicalKey::Code(KeyCode::KeyP),
            ElementState::Pressed,
            false,
            None
        ),
        None
    );
    app.focus_changed(true);
    app.occlusion_changed(true);
    assert!(!app.simulation_active());
    app.occlusion_changed(false);
    assert_eq!(app.game.world.snapshot(), state);
    assert!(app.help.is_none());
    assert!(app.xyzzy.is_none());
    assert!(app.docked.is_none());
    assert!(
        app.game
            .lines(false, None)
            .iter()
            .any(|l| l.contains("NO COMMANDER / NO SAVE"))
    );
}

#[test]
fn ordinary_world_cannot_be_relabelled_as_a_validation_profile() {
    assert!(Profile::mule_validation(ok(World::new_commander(917))).is_err());
    let mut world = ok(World::new_commander(918));
    let before = ok(world.save_bytes());
    ok(world.request_launch(world.snapshot().tick));
    ok(world.step(FlightInput::default()));
    assert!(world.mule_actor_snapshot().is_none());
    assert!(!before.is_empty());
}

#[test]
fn side_entry_uses_real_pilot_with_read_only_left_view_and_keeps_world_view_unchanged() {
    let app = ok(App::from_launch(Launch::Mule(View::Side)));
    assert!(app.paused);
    assert_eq!(app.validation_view, meridian_core::FlightView::Left);
    assert_eq!(
        app.game.world.flight_view(),
        meridian_core::FlightView::Front
    );
    let frame = app
        .game
        .world
        .mule_actor_snapshot()
        .unwrap_or_else(|| panic!("actual frame"));
    let observer = app
        .game
        .world
        .mule_observer_pose(&frame)
        .unwrap_or_else(|| panic!("actual pilot"));
    assert!(observer.position.x > frame.actor().pose.position.x);
    assert_eq!(
        observer.orientation,
        meridian_core::docking::Basis3::IDENTITY
    );
    let source = ok(MuleHullSource::bundled());
    let (_, left) = ok(observer_source());
    let right = source
        .pieces()
        .iter()
        .map(|p| ok(p.support_bound(meridian_core::docking::Vec3::new(1.0, 0.0, 0.0))))
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        ((observer.position.x - frame.actor().pose.position.x) + left - right + 0.5).abs() < 0.001
    );
    assert!(app.game.world.save_bytes().is_err());
}

#[test]
fn docking_modes_hold_the_real_terminal_frame_and_display_its_outcome() {
    use meridian_core::traffic::{MuleValidationRefusal, NpcPhase};

    for (name, x_offset, phase, refusal, notice) in [
        ("dock", 0.0, NpcPhase::Complete, None, "CROSSING ACCEPTED"),
        (
            "dock-obstructed",
            69.0,
            NpcPhase::Approach,
            Some(MuleValidationRefusal::StationContact),
            "MULE STOPPED",
        ),
    ] {
        let mut app = ok(App::from_launch(ok(Launch::parse(args(&[
            "--internal-mule-validation",
            name,
        ])))));
        let opening = app
            .game
            .world
            .mule_actor_snapshot()
            .unwrap_or_else(|| panic!("opening frame"));
        let observer = app
            .game
            .world
            .mule_observer_pose(&opening)
            .unwrap_or_else(|| panic!("actual observer"));
        let [width, height] = crate::app::DEFAULT_FLIGHT_WINDOW;
        let projection = crate::renderer::flight_projection(width, height);
        let flight = app.validation_view;
        let check_framing = |frame: &meridian_core::traffic::MuleActorSnapshot| {
            assert!(
                ok(crate::mule_render::detailed_hull_fits_view(
                    crate::mule_render::Scene {
                        actor: frame.actor(),
                        envelope: frame.presentation().as_ref(),
                        observer,
                    },
                    crate::mule_render::View {
                        camera: observer,
                        flight,
                        projection,
                        tick: frame.world_tick(),
                    },
                )),
                "{name} detailed hull clipped at tick {}",
                frame.world_tick()
            );
        };
        let port_center = app
            .game
            .world
            .station_port()
            .unwrap_or_else(|| panic!("actual port"))
            .center();
        assert_eq!(opening.actor().pose.position.x, port_center.x + x_offset);
        check_framing(&opening);
        assert_eq!(opening.actor().phase, NpcPhase::Approach);
        assert!(app.paused && app.game.is_internal_validation());
        for _ in 0..DOCKING_TICK_LIMIT.div_ceil(6) {
            assert!(matches!(
                app.advance_frame(true, 6),
                FrameAdvance::Continue(None)
            ));
            let frame = app
                .game
                .world
                .mule_actor_snapshot()
                .unwrap_or_else(|| panic!("approach frame"));
            check_framing(&frame);
            if app.notice.text.is_some() {
                break;
            }
        }
        let terminal = app
            .game
            .world
            .mule_actor_snapshot()
            .unwrap_or_else(|| panic!("terminal frame"));
        assert_eq!(terminal.actor().phase, phase);
        assert_eq!(terminal.motion_refusal(), refusal);
        assert_eq!(terminal.actor().speed_units, 0.0);
        assert_eq!(terminal.actor().id, opening.actor().id);
        assert_eq!(app.game.world.mule_observer_pose(&terminal), Some(observer));
        assert_eq!(
            terminal.actor().pose.position.z <= port_center.z,
            refusal.is_none()
        );
        assert!((1..=DOCKING_TICK_LIMIT).contains(&terminal.world_tick()));
        assert!(app.game.world.current_dock_context().is_none());
        assert!(app.game.world.save_bytes().is_err());
        assert_eq!(app.notice.text.as_deref(), Some(notice));
        let lines = app.game.lines(false, app.notice.text.as_deref());
        assert!(lines.iter().any(|line| line == notice));
        assert!(
            lines
                .iter()
                .any(|line| line == &format!("MULE PHASE {phase:?} | SPEED 0.00"))
        );
        if let Some(refusal) = refusal {
            assert!(
                lines
                    .iter()
                    .any(|line| line == &format!("WORLD MOTION REFUSED: {refusal:?}"))
            );
        }
        assert_eq!(
            ok(app.validation_control(KeyCode::Period)),
            Control::Ignored
        );
        assert_eq!(ok(app.validation_control(KeyCode::KeyP)), Control::Ignored);
        assert!(app.game.world.mule_snapshot_is_current(&terminal));
        assert_eq!(ok(app.validation_control(KeyCode::Escape)), Control::Quit);
    }
}

#[test]
fn presented_draw_timing_aggregates_empty_single_and_multiple_samples() {
    use crate::renderer::Presentation;
    let mut timing = FrameTiming::default();
    timing.observe(&Ok(Presentation::Skipped), Duration::from_nanos(900));
    timing.observe(&Err("draw failed".into()), Duration::from_nanos(900));
    assert_eq!(
        timing.report(),
        r#"MULE_PRESENTED_DRAW_WALL {"presentedFrames":0,"totalNanoseconds":0,"minNanoseconds":null,"maxNanoseconds":null}"#
    );
    timing.observe(&Ok(Presentation::Presented), Duration::from_nanos(20));
    assert_eq!(
        timing.report(),
        r#"MULE_PRESENTED_DRAW_WALL {"presentedFrames":1,"totalNanoseconds":20,"minNanoseconds":20,"maxNanoseconds":20}"#
    );
    for ns in [12, 34] {
        timing.observe(&Ok(Presentation::Presented), Duration::from_nanos(ns));
    }
    assert_eq!(
        timing.report(),
        r#"MULE_PRESENTED_DRAW_WALL {"presentedFrames":3,"totalNanoseconds":66,"minNanoseconds":12,"maxNanoseconds":34}"#
    );
    let directory = TestDirectory::new();
    let ordinary = App::with_profile(crate::profile::tests::open(&directory.0));
    assert!(ordinary.validation_timing.is_none());
}

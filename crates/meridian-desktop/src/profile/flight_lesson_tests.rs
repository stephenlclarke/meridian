//! Actual Profile/Store actions and public flight. No fabricated measurement or docking receipt.
use super::Profile;
use crate::session::Action;
use meridian_core::{FlightInput, FlightMode, FlightReceptionOutcome, FlightView, World};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

fn ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|e| panic!("{e:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("missing retained state"))
}
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let nonce = ok(SystemTime::now().duration_since(UNIX_EPOCH)).as_nanos();
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "meridian-flight-lesson-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path)
            .unwrap_or_else(|error| panic!("Cannot create {}: {error}", path.display()));
        Self(path)
    }
    fn profile(&self) -> Profile {
        let mut p = ok(Profile::open(self.0.clone()));
        assert!(ok(p.new_commander(11001)));
        p
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn action(p: &mut Profile, a: Action) {
    ok(p.command(a));
}
fn step(world: &mut World, input: FlightInput) {
    ok(world.step(input));
}
fn stop(world: &mut World) {
    for _ in 0..300 {
        if world.snapshot().speed == 0.0 {
            return;
        }
        step(
            world,
            FlightInput {
                brake: true,
                ..FlightInput::default()
            },
        );
    }
    panic!("public braking failed");
}
fn aim(world: &mut World) {
    stop(world);
    let s = world.snapshot();
    let [x, y, z, w] = s.orientation.map(f64::from);
    let current = (2.0 * (x * z + w * y)).atan2(1.0 - 2.0 * (x * x + y * y));
    let target = (-f64::from(s.position[0])).atan2(-f64::from(s.position[2]));
    let yaw = (target - current) as f32;
    let count = (yaw.abs() / (0.65 / 60.0)).ceil() as u16;
    for _ in 0..count {
        step(
            world,
            FlightInput {
                yaw: yaw / (0.65 / 60.0) / f32::from(count),
                ..FlightInput::default()
            },
        );
    }
}
fn measured_return(p: &mut Profile) {
    action(p, Action::Launch);
    for _ in 0..600 {
        if p.world.snapshot().mode == FlightMode::Flight {
            break;
        }
        step(&mut p.world, FlightInput::default());
    }
    assert_eq!(p.world.snapshot().mode, FlightMode::Flight);
    aim(&mut p.world);
    for _ in 0..2 {
        step(
            &mut p.world,
            FlightInput {
                throttle: 1.0,
                ..FlightInput::default()
            },
        );
    }
    assert!(
        some(p.world.flight_reception_snapshot()).attempts[0]
            .measured_ticks
            .is_some()
    );
    aim(&mut p.world);
    for _ in 0..9000 {
        let s = p.world.snapshot();
        if s.mode == FlightMode::Docked {
            break;
        }
        step(
            &mut p.world,
            FlightInput {
                throttle: ((4.0 - s.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..FlightInput::default()
            },
        );
        assert_eq!(p.world.snapshot().motion_refusal, None);
    }
    assert_eq!(p.world.snapshot().mode, FlightMode::Docked);
    assert!(some(p.world.flight_reception_snapshot()).ready);
}
#[test]
fn each_lesson_publication_failure_preserves_live_state_and_the_retained_valid_file() {
    for command in [Action::ReviewFlightLesson, Action::CancelFlightLesson] {
        let directory = Directory::new();
        let mut p = directory.profile();
        if matches!(command, Action::CancelFlightLesson) {
            action(&mut p, Action::ReviewFlightLesson);
            action(&mut p, Action::Confirm);
        }
        ok(p.save());
        let before = ok(p.world.save_bytes());
        let target = directory.0.join("commander.meridian");
        let retained = directory.0.join("retained-valid-slot");
        ok(fs::rename(&target, &retained));
        ok(fs::create_dir(&target));
        ok(fs::write(target.join("sentinel"), b"retain"));
        action(&mut p, command);
        assert!(p.command(Action::Confirm).is_err());
        assert_eq!(ok(p.world.save_bytes()), before);
        assert_eq!(ok(fs::read(&retained)), before);
        assert_eq!(ok(fs::read(target.join("sentinel"))), b"retain");
        assert!(p.command(Action::Confirm).is_err());
        ok(fs::remove_dir_all(&target));
        ok(fs::rename(&retained, &target));
        action(&mut p, command);
        action(&mut p, Action::Confirm);
        assert_eq!(ok(fs::read(&target)), ok(p.world.save_bytes()));
    }
}
#[test]
fn genuine_measured_return_writer_failure_back_and_reception_retry_keep_one_trace_and_zero_cash() {
    let directory = Directory::new();
    let mut p = directory.profile();
    let cash = p.world.credits_tenths();
    action(&mut p, Action::ReviewFlightLesson);
    for _ in 0..12 {
        step(&mut p.world, FlightInput::default());
    }
    action(&mut p, Action::Confirm);
    measured_return(&mut p);
    ok(p.save());
    let before = ok(p.world.save_bytes());
    let target = directory.0.join("commander.meridian");
    let retained = directory.0.join("valid-before-receive");
    action(&mut p, Action::ReviewFlightLesson);
    action(&mut p, Action::Back);
    assert!(p.command(Action::Confirm).is_err());
    assert_eq!(ok(p.world.save_bytes()), before);
    ok(fs::rename(&target, &retained));
    ok(fs::create_dir(&target));
    action(&mut p, Action::ReviewFlightLesson);
    assert!(p.command(Action::Confirm).is_err());
    assert_eq!(ok(p.world.save_bytes()), before);
    assert_eq!(ok(fs::read(&retained)), before);
    assert_eq!(some(p.world.flight_reception_snapshot()).trace, None);
    ok(fs::remove_dir(&target));
    ok(fs::rename(&retained, &target));
    action(&mut p, Action::ReviewFlightLesson);
    for _ in 0..9 {
        step(&mut p.world, FlightInput::default());
    }
    action(&mut p, Action::Confirm);
    let settled = some(p.world.flight_reception_snapshot());
    assert!(settled.trace.is_some());
    assert_eq!(
        settled.attempts[0].outcome,
        Some(FlightReceptionOutcome::Complete)
    );
    assert_eq!(p.world.credits_tenths(), cash);
    let saved = ok(fs::read(&target));
    assert_eq!(ok(p.world.save_bytes()), saved);
    assert!(p.command(Action::Confirm).is_err());
    assert_eq!(ok(fs::read(&target)), saved);
    ok(p.continue_saved());
    assert_eq!(some(p.world.flight_reception_snapshot()), settled);
    assert_eq!(p.world.credits_tenths(), cash);
}
#[test]
fn views_details_and_other_commands_invalidate_reviews_without_granting_consent() {
    let directory = Directory::new();
    let mut p = directory.profile();
    let before = ok(p.world.save_bytes());
    action(&mut p, Action::ReviewFlightLesson);
    action(&mut p, Action::Back);
    assert!(p.command(Action::Confirm).is_err());
    assert_eq!(ok(p.world.save_bytes()), before);
    for view in [
        FlightView::Front,
        FlightView::Rear,
        FlightView::Left,
        FlightView::Right,
    ] {
        action(&mut p, Action::ReviewFlightLesson);
        action(&mut p, Action::SetFlightView(view));
        assert!(p.command(Action::Confirm).is_err());
        assert!(
            some(p.world.flight_reception_snapshot())
                .attempts
                .is_empty()
        );
        action(&mut p, Action::FlightLessonDetails);
        let lines = p.lines(true, None);
        assert!(lines.len() <= crate::hud::line_budget(640, 360));
        let text = lines.join("\n");
        assert!(text.contains(crate::flight_lesson::view_name(view)));
        assert!(text.contains("F12 REVIEW CANCELLATION"));
    }
    action(&mut p, Action::ReviewFlightLesson);
    action(&mut p, Action::Confirm);
    action(&mut p, Action::CancelFlightLesson);
    action(&mut p, Action::Confirm);
    action(&mut p, Action::FlightLessonDetails);
    assert!(p.lines(true, None).join("\n").contains("ABANDONED 1"));
}
#[test]
fn cancelled_lesson_and_actual_view_survive_process_exit_before_fresh_consent() {
    let directory = Directory::new();
    let mut p = directory.profile();
    action(&mut p, Action::ReviewFlightLesson);
    action(&mut p, Action::Confirm);
    action(&mut p, Action::CancelFlightLesson);
    action(&mut p, Action::Confirm);
    action(&mut p, Action::SetFlightView(FlightView::Left));
    ok(p.prepare_quit());
    drop(p);
    let status = ok(Command::new(ok(std::env::current_exe()))
        .args([
            "--exact",
            "profile::flight_lesson_tests::flight_lesson_cold_child",
            "--nocapture",
        ])
        .env("MERIDIAN_FLIGHT_LESSON_TEST_SLOT", &directory.0)
        .status());
    assert!(status.success());
    let mut cold = ok(Profile::open(directory.0.clone()));
    ok(cold.continue_saved());
    let state = some(cold.world.flight_reception_snapshot());
    assert_eq!(state.attempts.len(), 2);
    assert_eq!(
        state.attempts[0].outcome,
        Some(FlightReceptionOutcome::Abandoned)
    );
    assert_eq!(state.active, Some(state.attempts[1].id));
    assert_eq!(state.trace, None);
    assert_eq!(cold.world.flight_view(), FlightView::Left);
}
#[test]
fn flight_lesson_cold_child() {
    let Some(path) = std::env::var_os("MERIDIAN_FLIGHT_LESSON_TEST_SLOT") else {
        return;
    };
    let mut p = ok(Profile::open(PathBuf::from(path)));
    ok(p.continue_saved());
    let state = some(p.world.flight_reception_snapshot());
    assert_eq!(state.attempts.len(), 1);
    assert_eq!(
        state.attempts[0].outcome,
        Some(FlightReceptionOutcome::Abandoned)
    );
    assert_eq!(p.world.flight_view(), FlightView::Left);
    assert_eq!(state.trace, None);
    action(&mut p, Action::ReviewFlightLesson);
    action(&mut p, Action::Confirm);
    ok(p.prepare_quit());
}

#[test]
fn normal_docked_launch_and_all_flight_views_keep_critical_rows_visible_at_minimum_size() {
    fn visible(p: &Profile) {
        let budget = crate::hud::line_budget(640, 360);
        for notice in [None, Some("Actual input notice")] {
            let lines = p.lines(true, notice);
            assert!(
                lines.len() <= budget,
                "{} HUD rows exceed {budget}",
                lines.len()
            );
            assert!(
                lines
                    .iter()
                    .all(|line| line.len() <= crate::hud::MAX_COLUMNS)
            );
            let rendered = lines
                .into_iter()
                .take(budget)
                .collect::<Vec<_>>()
                .join("\n");
            for control in [
                "I MISSION DETAILS",
                "C LESSON",
                "F6/F12 REVIEW",
                "F5 SAVE",
                "F9 RELOAD SAVE",
                "ESC SAVE AND QUIT",
            ] {
                assert!(
                    rendered.contains(control),
                    "Clipped critical control {control}"
                );
            }
            assert!(rendered.contains(crate::flight_lesson::view_name(p.world.flight_view())));
        }
    }
    let directory = Directory::new();
    let mut p = directory.profile();
    visible(&p);
    action(&mut p, Action::ReviewFlightLesson);
    action(&mut p, Action::Confirm);
    visible(&p);
    action(&mut p, Action::Launch);
    visible(&p);
    for _ in 0..600 {
        if p.world.snapshot().mode == FlightMode::Flight {
            break;
        }
        step(&mut p.world, FlightInput::default());
    }
    assert_eq!(p.world.snapshot().mode, FlightMode::Flight);
    for view in [
        FlightView::Front,
        FlightView::Rear,
        FlightView::Left,
        FlightView::Right,
    ] {
        action(&mut p, Action::SetFlightView(view));
        visible(&p);
    }
}

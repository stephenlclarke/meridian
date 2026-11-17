//! Real Store/process continuity for the finite traffic World; authored pending qualification.
use super::tests::{TestDirectory, action, open};
use super::{Action, Profile};
use meridian_core::{FlightInput, FlightMode, World, traffic::Population};
use std::{fs, process::Command};

fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}

fn step(world: &mut World, input: FlightInput) {
    ok(world.step(input));
    assert_eq!(world.snapshot().motion_refusal, None);
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
    panic!("public brake failed");
}

fn traffic_profile(directory: &TestDirectory) -> Profile {
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(12005)));
    action(&mut profile, Action::Launch);
    step(&mut profile.world, FlightInput::default());
    assert_eq!(profile.world.snapshot().mode, FlightMode::Flight);
    stop(&mut profile.world);
    // Turn through public yaw commands, then fly clear of the native departure lane.
    let angle = -core::f32::consts::FRAC_PI_2;
    let ticks = (angle.abs() / (0.65 / 60.0)).ceil() as u16;
    for _ in 0..ticks {
        step(
            &mut profile.world,
            FlightInput {
                yaw: angle / (0.65 / 60.0) / f32::from(ticks),
                ..FlightInput::default()
            },
        );
    }
    for _ in 0..3000 {
        if profile.world.snapshot().position[0] <= -600.0 {
            break;
        }
        let speed = profile.world.snapshot().speed;
        step(
            &mut profile.world,
            FlightInput {
                throttle: ((10.0 - speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..FlightInput::default()
            },
        );
    }
    assert!(profile.world.snapshot().position[0] <= -600.0);
    stop(&mut profile.world);
    for _ in 0..500 {
        step(&mut profile.world, FlightInput::default());
    }
    let traffic = profile
        .world
        .traffic_snapshot()
        .unwrap_or_else(|| panic!("traffic"));
    let trader = traffic
        .actors
        .iter()
        .find(|actor| actor.population == Population::Trader)
        .unwrap_or_else(|| panic!("paid trader did not actually depart"));
    assert_eq!(trader.hull_id, "SH025");
    assert_eq!(trader.cash_tenths, 117966);
    assert_eq!(profile.world.credits_tenths(), Some(7200));
    profile
}

#[test]
fn traffic_cold_process_child() {
    let Some(path) = std::env::var_os("MERIDIAN_TRAFFIC_TEST_SLOT") else {
        return;
    };
    let directory = std::path::Path::new(&path);
    let saved = ok(fs::read(directory.join("commander.meridian")));
    let mut expected = ok(World::from_save_bytes(&saved));
    let mut profile = open(directory);
    ok(profile.continue_saved());
    assert_eq!(
        profile.world.traffic_snapshot(),
        expected.traffic_snapshot()
    );
    assert_eq!(
        profile.world.service_snapshot(),
        expected.service_snapshot()
    );
    assert_eq!(
        profile.world.flight_resources_snapshot(),
        expected.flight_resources_snapshot()
    );
    assert_eq!(ok(profile.world.save_bytes()), ok(expected.save_bytes()));
    for _ in 0..120 {
        step(&mut profile.world, FlightInput::default());
        step(&mut expected, FlightInput::default());
    }
    assert_eq!(
        profile.world.traffic_snapshot(),
        expected.traffic_snapshot()
    );
    assert_eq!(
        profile.world.service_snapshot(),
        expected.service_snapshot()
    );
    assert_eq!(profile.world.credits_tenths(), Some(7200));
    assert_eq!(ok(fs::read(directory.join("commander.meridian"))), saved);
    ok(profile.prepare_quit());
    let expected_after = ok(expected.save_bytes());
    assert_eq!(
        ok(fs::read(directory.join("commander.meridian"))),
        expected_after
    );
}

#[test]
fn actual_profile_quit_and_fresh_process_continue_preserve_traffic_and_finite_markets() {
    let directory = TestDirectory::new();
    let mut profile = traffic_profile(&directory);
    let mut expected = ok(World::from_save_bytes(&ok(profile.world.save_bytes())));
    for _ in 0..120 {
        step(&mut expected, FlightInput::default());
    }
    ok(profile.prepare_quit());
    drop(profile);
    let status = ok(Command::new(ok(std::env::current_exe()))
        .args([
            "--exact",
            "profile::traffic_tests::traffic_cold_process_child",
            "--nocapture",
        ])
        .env("MERIDIAN_TRAFFIC_TEST_SLOT", &directory.0)
        .status());
    assert!(status.success());
    let mut continued = open(&directory.0);
    ok(continued.continue_saved());
    assert_eq!(
        continued.world.traffic_snapshot(),
        expected.traffic_snapshot()
    );
    assert_eq!(
        continued.world.service_snapshot(),
        expected.service_snapshot()
    );
    assert_eq!(
        continued.world.flight_resources_snapshot(),
        expected.flight_resources_snapshot()
    );
    assert_eq!(continued.world.credits_tenths(), Some(7200));
}

#[test]
fn failed_quit_preserves_live_traffic_and_previous_owned_slot() {
    let directory = TestDirectory::new();
    let mut profile = traffic_profile(&directory);
    ok(profile.save());
    let destination = directory.0.join("commander.meridian");
    let retained = directory.0.join("retained-traffic-save");
    let previous = ok(fs::read(&destination));
    for _ in 0..60 {
        step(&mut profile.world, FlightInput::default());
    }
    let before = ok(profile.world.save_bytes());
    let traffic = profile.world.traffic_snapshot();
    let markets = profile.world.service_snapshot();
    ok(fs::rename(&destination, &retained));
    ok(fs::create_dir(&destination));
    assert!(profile.prepare_quit().is_err());
    assert!(profile.playing());
    assert_eq!(ok(profile.world.save_bytes()), before);
    assert_eq!(profile.world.traffic_snapshot(), traffic);
    assert_eq!(profile.world.service_snapshot(), markets);
    assert_eq!(ok(fs::read(&retained)), previous);
    ok(fs::remove_dir(&destination));
    ok(fs::rename(&retained, &destination));
    assert_eq!(ok(fs::read(&destination)), previous);
}

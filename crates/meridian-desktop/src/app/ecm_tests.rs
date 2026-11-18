//! Actual desktop input, paid equipment reviews and durable files. Authored, UNRUN.
use super::emp_tests::{earned, pursue_recent_attacker, yaw_to};
use super::{
    Action, App, Docked, ElementState, FlightInput, FrameAdvance, KeyCode, PhysicalKey, ui,
};
use crate::profile::tests::{TestDirectory, open};
use meridian_core::{
    EmpChargePhase, MissileEnd, ModuleAction, ModuleKind, ModuleProvenance, World, XyzzyAction,
};
use std::fs;

fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("missing actual ECM fixture value"))
}
fn app(directory: &TestDirectory, seed: u64) -> App {
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(seed)));
    App::with_profile(profile)
}
fn paint(app: &mut App) -> ui::Frame {
    let viewport = some(ui::Viewport::new(1280.0, 720.0));
    let menu = some(app.docked.as_mut());
    let frame = ok(menu.build_frame(&app.game.world, viewport, 0.0));
    menu.presented(frame.clone());
    frame
}
fn menu_key(app: &mut App, key: ui::Key) {
    app.menu_input(ui::Input::Key(key));
    paint(app);
}
fn labels(frame: &ui::Frame) -> String {
    frame
        .labels
        .iter()
        .map(|label| label.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn shop(app: &mut App) {
    let mut menu = ok(Docked::new(&app.game.world));
    ok(menu.open(&app.game.world, ui::Screen::Equipment));
    app.docked = Some(menu);
    paint(app);
    for _ in 0..32 {
        let selected = labels(&paint(app));
        if selected.contains("> ECM utility:") {
            return;
        }
        menu_key(app, ui::Key::NextSlot);
    }
    panic!("actual ECM equipment slot was not presented");
}
fn route(app: &mut App, key: KeyCode, repeat: bool) -> Option<KeyCode> {
    let result = app.route_keyboard(PhysicalKey::Code(key), ElementState::Pressed, repeat, None);
    if let Some(code) = result {
        app.game_key(code);
    }
    result
}
fn advance(app: &mut App, ticks: usize) {
    match app.advance_frame(app.simulation_active(), ticks) {
        FrameAdvance::Continue(_) => {}
        FrameAdvance::Fatal(error) => panic!("actual frame failed: {error:?}"),
    }
}
fn funded(directory: &TestDirectory) -> App {
    let mut app = app(directory, 19101);
    earned(&mut app);
    ok(app.game.save());
    app
}
fn ready(directory: &TestDirectory) -> App {
    let mut app = funded(directory);
    ok(app.game.command(Action::Module(ModuleAction::Purchase {
        kind: ModuleKind::EcmSystem,
    })));
    ok(app.game.command(Action::ReviewPractice));
    ok(app.game.command(Action::Confirm));
    ok(app.game.command(Action::Launch));
    advance(&mut app, 1);
    for _ in 0..300 {
        if app.game.world.snapshot().speed == 0.0 {
            return app;
        }
        ok(app.game.world.step(FlightInput {
            brake: true,
            ..FlightInput::default()
        }));
    }
    panic!("public brake did not stop the actual practice ship");
}
fn launch_missile(app: &mut App) {
    let target = some(some(app.game.world.practice_snapshot()).beacons)[1].id;
    let tick = app.game.world.snapshot().tick;
    ok(app.game.world.request_missile_lock(Some(target), tick));
    route(app, KeyCode::F2, false);
    route(app, KeyCode::F3, false);
    assert!(some(app.game.world.missile_snapshot()).projectiles[0].is_some());
}
fn disk(directory: &TestDirectory) -> Vec<u8> {
    ok(fs::read(directory.0.join("commander.meridian")))
}
#[test]
fn presented_paid_ecm_purchase_store_refit_and_cold_property_keep_one_original_item() {
    let directory = TestDirectory::new();
    let mut app = funded(&directory);
    shop(&mut app);
    let cash = app.game.world.credits_tenths();
    let resources = app.game.world.flight_resources_snapshot();
    let before = ok(app.game.world.save_bytes());
    let previous_disk = disk(&directory);
    let labels = labels(&paint(&mut app));
    assert!(labels.contains("2100.0") || labels.contains("2100"));
    assert!(labels.contains("22 shared energy"));
    menu_key(&mut app, ui::Key::Buy);
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(disk(&directory), previous_disk);
    menu_key(&mut app, ui::Key::Confirm);
    let item = some(
        some(app.game.world.modular_equipment_snapshot())
            .items
            .into_iter()
            .find(|i| i.kind == ModuleKind::EcmSystem),
    );
    assert!(matches!(
        item.provenance,
        ModuleProvenance::Paid {
            paid_tenths: 21000,
            ..
        }
    ));
    assert_eq!(
        app.game.world.credits_tenths(),
        cash.map(|cash| cash - 21000)
    );
    assert_eq!(
        some(app.game.world.ecm_snapshot()).fitted_item,
        Some(item.id)
    );
    menu_key(&mut app, ui::Key::Store);
    menu_key(&mut app, ui::Key::Confirm);
    assert!(some(app.game.world.ecm_snapshot()).fitted_item.is_none());
    menu_key(&mut app, ui::Key::Fit);
    menu_key(&mut app, ui::Key::Confirm);
    assert_eq!(
        some(app.game.world.ecm_snapshot()).fitted_item,
        Some(item.id)
    );
    assert_eq!(app.game.world.flight_resources_snapshot(), resources);
    let actual = ok(app.game.world.save_bytes());
    assert_eq!(disk(&directory), actual);
    drop(app);
    let mut loaded = open(&directory.0);
    ok(loaded.continue_saved());
    assert_eq!(some(loaded.world.ecm_snapshot()).fitted_item, Some(item.id));
    assert_eq!(loaded.world.flight_resources_snapshot(), resources);
    assert_eq!(loaded.world.credits_tenths(), cash.map(|cash| cash - 21000));
}
#[test]
fn actual_ecm_purchase_writer_failure_retains_the_reviewed_world_and_previous_file() {
    let directory = TestDirectory::new();
    let mut app = funded(&directory);
    shop(&mut app);
    menu_key(&mut app, ui::Key::Buy);
    let before = ok(app.game.world.save_bytes());
    let previous = disk(&directory);
    let slot = directory.0.join("commander.meridian");
    let retained = directory.0.join("retained-valid-commander");
    ok(fs::rename(&slot, &retained));
    ok(fs::create_dir(&slot));
    menu_key(&mut app, ui::Key::Confirm);
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(ok(fs::read(&retained)), previous);
    assert!(some(app.game.world.ecm_snapshot()).fitted_item.is_none());
    ok(fs::remove_dir(&slot));
    ok(fs::rename(&retained, &slot));
}
#[test]
fn unfunded_and_forged_ecm_rows_cannot_grant_property_or_confirmation_authority() {
    let directory = TestDirectory::new();
    let mut app = app(&directory, 19101);
    shop(&mut app);
    let before = ok(app.game.world.save_bytes());
    menu_key(&mut app, ui::Key::Buy);
    menu_key(&mut app, ui::Key::Confirm);
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert!(!directory.0.join("commander.meridian").exists());
    for (node, item, operation) in [
        ("EMP", None, ui::EquipmentOperation::Buy),
        (
            ModuleKind::EcmSystem.node(),
            Some(1),
            ui::EquipmentOperation::Buy,
        ),
        (
            ModuleKind::EcmSystem.node(),
            Some(1),
            ui::EquipmentOperation::Store,
        ),
        (
            ModuleKind::EcmSystem.node(),
            None,
            ui::EquipmentOperation::Configure,
        ),
    ] {
        assert!(ui::ecm::action(&app.game.world, node, item, operation).is_err());
    }
    assert_eq!(ok(app.game.world.save_bytes()), before);
}
#[test]
fn discrete_n_publishes_real_missile_termination_without_impact_bank_or_refund() {
    let directory = TestDirectory::new();
    let mut app = ready(&directory);
    launch_missile(&mut app);
    let missile = some(some(app.game.world.missile_snapshot()).projectiles[0]);
    let rounds = some(app.game.world.missile_snapshot()).rounds;
    let cash = app.game.world.credits_tenths();
    let cargo = app.game.world.service_snapshot();
    let pools = some(app.game.world.flight_resources_snapshot());
    assert_eq!(route(&mut app, KeyCode::KeyN, false), Some(KeyCode::KeyN));
    let ended = some(some(app.game.world.missile_snapshot()).projectiles[0]);
    assert_eq!(ended.origin, missile.origin);
    assert_eq!(ended.ended, Some(MissileEnd::Ecm));
    assert!(ended.contact.is_none() && ended.intercepting_emission.is_none());
    assert_eq!(
        some(app.game.world.flight_resources_snapshot()).energy_hundredths,
        pools.energy_hundredths - 2200
    );
    assert_eq!(some(app.game.world.missile_snapshot()).rounds, rounds);
    assert_eq!(app.game.world.credits_tenths(), cash);
    assert_eq!(app.game.world.service_snapshot(), cargo);
    assert!(app.game.world.emp_snapshot().is_none());
    assert_eq!(disk(&directory), ok(app.game.world.save_bytes()));
    let hud = app.game.lines(true, app.notice.as_deref());
    let row = some(hud.iter().position(|line| line.contains("ECM FIELD")));
    assert!(row < crate::hud::line_budget(640, 360));
    assert!(some(app.notice.as_ref()).contains("22 energy spent"));
    let loaded = ok(World::from_save_bytes(&disk(&directory)));
    assert!(some(loaded.ecm_snapshot()).field.is_none());
    assert_eq!(some(some(loaded.missile_snapshot()).projectiles[0]), ended);
    assert_eq!(some(loaded.ecm_snapshot()).remaining_ticks, 32);
}
#[test]
fn manual_activation_writer_failure_rolls_back_cost_missile_field_and_disk_together() {
    let directory = TestDirectory::new();
    let mut app = ready(&directory);
    launch_missile(&mut app);
    ok(app.game.save());
    let before = ok(app.game.world.save_bytes());
    let slot = directory.0.join("commander.meridian");
    let retained = directory.0.join("retained-valid-commander");
    let previous = disk(&directory);
    ok(fs::rename(&slot, &retained));
    ok(fs::create_dir(&slot));
    route(&mut app, KeyCode::KeyN, false);
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(ok(fs::read(&retained)), previous);
    assert!(some(app.notice.as_ref()).contains("not activated"));
    assert!(
        some(some(app.game.world.missile_snapshot()).projectiles[0])
            .ended
            .is_none()
    );
    ok(fs::remove_dir(&slot));
    ok(fs::rename(&retained, &slot));
}
#[test]
fn cold_continue_retains_timer_but_new_real_missile_needs_a_fresh_charged_n() {
    let directory = TestDirectory::new();
    let mut app = ready(&directory);
    route(&mut app, KeyCode::KeyN, false);
    ok(app.game.prepare_quit());
    drop(app);
    let mut profile = open(&directory.0);
    ok(profile.continue_saved());
    let mut loaded = App::with_profile(profile);
    assert!(loaded.keys.is_empty());
    assert!(some(loaded.game.world.ecm_snapshot()).field.is_none());
    assert_eq!(some(loaded.game.world.ecm_snapshot()).remaining_ticks, 32);
    assert!(
        loaded
            .game
            .lines(true, None)
            .iter()
            .any(|line| line.contains("ECM COOLDOWN"))
    );
    launch_missile(&mut loaded);
    let before = ok(loaded.game.world.save_bytes());
    route(&mut loaded, KeyCode::KeyN, false);
    assert_eq!(ok(loaded.game.world.save_bytes()), before);
    assert!(
        some(some(loaded.game.world.missile_snapshot()).projectiles[0])
            .ended
            .is_none()
    );
    advance(&mut loaded, 32);
    let energy = some(loaded.game.world.flight_resources_snapshot()).energy_hundredths;
    route(&mut loaded, KeyCode::KeyN, false);
    assert_eq!(some(loaded.game.world.ecm_snapshot()).activations, 2);
    assert_eq!(
        some(loaded.game.world.flight_resources_snapshot()).energy_hundredths,
        energy - 2200
    );
    assert_eq!(
        some(some(loaded.game.world.missile_snapshot()).projectiles[0]).ended,
        Some(MissileEnd::Ecm)
    );
    assert_eq!(disk(&directory), ok(loaded.game.world.save_bytes()));
}
#[test]
fn h_question_focus_pause_and_repeat_cannot_issue_or_renew_manual_ecm() {
    let directory = TestDirectory::new();
    let mut app = ready(&directory);
    for opener in [KeyCode::KeyH, KeyCode::Slash] {
        let text = if opener == KeyCode::Slash {
            Some("?")
        } else {
            Some("h")
        };
        assert_eq!(
            app.route_keyboard(
                PhysicalKey::Code(opener),
                ElementState::Pressed,
                false,
                text
            ),
            None
        );
        let panel = some(app.help.as_ref());
        assert!(panel.lines.iter().any(|line| line.contains("N Manual ECM")));
        assert!(panel.lines.len() + 2 <= crate::help::MAX_LINES);
        assert!(
            panel
                .lines
                .iter()
                .all(|line| line.len() <= crate::help::MAX_COLUMNS)
        );
        let before = ok(app.game.world.save_bytes());
        assert_eq!(route(&mut app, KeyCode::KeyN, false), None);
        advance(&mut app, 40);
        assert_eq!(ok(app.game.world.save_bytes()), before);
        route(&mut app, KeyCode::Escape, false);
    }
    app.focus_changed(false);
    assert_eq!(route(&mut app, KeyCode::KeyN, false), None);
    app.focus_changed(true);
    app.occlusion_changed(true);
    assert_eq!(route(&mut app, KeyCode::KeyN, false), None);
    app.occlusion_changed(false);
    app.paused = true;
    app.reset_input();
    let paused = ok(app.game.world.save_bytes());
    route(&mut app, KeyCode::KeyN, false);
    advance(&mut app, 40);
    assert_eq!(ok(app.game.world.save_bytes()), paused);
    app.paused = false;
    app.reset_input();
    route(&mut app, KeyCode::KeyN, false);
    let live = ok(app.game.world.save_bytes());
    assert_eq!(route(&mut app, KeyCode::KeyN, true), None);
    assert_eq!(ok(app.game.world.save_bytes()), live);
    advance(&mut app, 40);
    assert_eq!(some(app.game.world.ecm_snapshot()).activations, 1);
    assert!(some(app.game.world.ecm_snapshot()).field.is_none());
}
#[test]
fn host_manual_ecm_cancels_a_real_emp_charge_only_after_durable_publication() {
    let directory = TestDirectory::new();
    let mut app = app(&directory, 12001);
    // Explicit secret funding/EMP kit. ECM remains paid and takes a real freed utility mount.
    ok(app.game.xyzzy(XyzzyAction::Enter));
    let scoop = some(some(app.game.world.fuel_scoops_snapshot()).owned).id;
    ok(app
        .game
        .command(Action::Module(ModuleAction::Store { item: scoop })));
    ok(app.game.command(Action::Module(ModuleAction::Purchase {
        kind: ModuleKind::EcmSystem,
    })));
    let target = pursue_recent_attacker(&mut app);
    assert_eq!(
        app.route_keyboard(
            PhysicalKey::Code(KeyCode::KeyU),
            ElementState::Pressed,
            false,
            None
        ),
        Some(KeyCode::KeyU)
    );
    let mut charging = false;
    for _ in 0..1200 {
        let actor = some(
            some(app.game.world.traffic_snapshot())
                .actors
                .into_iter()
                .find(|actor| actor.id == target),
        );
        let input = yaw_to(
            &app.game.world,
            actor.pose.position.x,
            actor.pose.position.z,
        );
        ok(app.game.world.step(input));
        advance(&mut app, 1);
        if app
            .game
            .world
            .emp_snapshot()
            .is_some_and(|emp| emp.charge_phase == Some(EmpChargePhase::Charging))
        {
            charging = true;
            break;
        }
    }
    assert!(
        charging,
        "real host charge never admitted; no substitute attack allowed"
    );
    let before = some(app.game.world.emp_snapshot());
    let energy = some(app.game.world.flight_resources_snapshot()).energy_hundredths;
    ok(app.game.save());
    let world_before = ok(app.game.world.save_bytes());
    let slot = directory.0.join("commander.meridian");
    let retained = directory.0.join("retained-valid-commander");
    ok(fs::rename(&slot, &retained));
    ok(fs::create_dir(&slot));
    route(&mut app, KeyCode::KeyN, false);
    assert_eq!(ok(app.game.world.save_bytes()), world_before);
    assert_eq!(app.game.world.emp_snapshot(), Some(before.clone()));
    assert!(app.keys.contains(&KeyCode::KeyU));
    assert_eq!(ok(fs::read(&retained)), world_before);
    ok(fs::remove_dir(&slot));
    ok(fs::rename(&retained, &slot));
    route(&mut app, KeyCode::KeyN, false);
    let after = some(app.game.world.emp_snapshot());
    assert_eq!(after.charge_phase, Some(EmpChargePhase::Cancelled));
    assert_eq!(
        (after.bank_energy, after.releases),
        (before.bank_energy, before.releases)
    );
    assert_eq!(
        some(app.game.world.flight_resources_snapshot()).energy_hundredths,
        energy - 2200
    );
    assert!(!app.keys.contains(&KeyCode::KeyU));
    let loaded = ok(World::from_save_bytes(&disk(&directory)));
    assert_eq!(loaded.emp_snapshot(), app.game.world.emp_snapshot());
    advance(&mut app, 1);
    assert_eq!(
        some(app.game.world.emp_snapshot()).releases,
        before.releases
    );
}

#[test]
fn real_unfitted_flight_and_session_bypass_refuse_without_world_or_file_changes() {
    let directory = TestDirectory::new();
    let mut app = app(&directory, 19101);
    ok(app.game.command(Action::Launch));
    advance(&mut app, 1);
    ok(app.game.save());
    let before = ok(app.game.world.save_bytes());
    let previous = disk(&directory);
    route(&mut app, KeyCode::KeyN, false);
    assert!(some(app.notice.as_ref()).contains("Fit an owned E.C.M. System"));
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(disk(&directory), previous);
    assert!(
        app.game
            .session
            .command(&mut app.game.world, Action::ActivateEcm)
            .is_err()
    );
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(disk(&directory), previous);
}

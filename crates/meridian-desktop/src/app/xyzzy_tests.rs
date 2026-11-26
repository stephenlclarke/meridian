//! Real host input and file publication, with disposable isolated commander slots.
use super::{
    Action, App, Docked, ElementState, FlightInput, FlightMode, FrameAdvance, KeyCode,
    ModifiersState, PhysicalKey, World, ui,
};
use crate::profile::tests::{TestDirectory, open};
use meridian_core::{ModuleKind, ModuleProvenance, XyzzyAction};
use std::fs;

fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn app(directory: &TestDirectory) -> App {
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(99034)));
    App::with_profile(profile)
}
fn press(app: &mut App, code: KeyCode) -> Option<KeyCode> {
    app.route_keyboard(PhysicalKey::Code(code), ElementState::Pressed, false, None)
}
fn menu(app: &mut App) {
    app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
    assert_eq!(press(app, KeyCode::KeyX), None);
    app.modifiers = ModifiersState::empty();
    assert!(app.xyzzy.is_some());
}
fn activate(app: &mut App) {
    menu(app);
    assert_eq!(press(app, KeyCode::Enter), None);
    assert_eq!(press(app, KeyCode::Enter), None);
    assert!(app.xyzzy.as_ref().is_some_and(|menu| !menu.reviewing));
    press(app, KeyCode::Escape);
    assert_eq!(app.game.world.commander_name(), Some("CLARKE"));
}

#[test]
fn secret_menu_requires_explicit_chord_and_confirmation_and_cancel_never_grants() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    let before = ok(app.game.world.save_bytes());
    assert_eq!(press(&mut app, KeyCode::KeyX), Some(KeyCode::KeyX));
    assert!(app.xyzzy.is_none());
    menu(&mut app);
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert!(!directory.0.join("commander.meridian").exists());
    assert_eq!(press(&mut app, KeyCode::Enter), None);
    assert!(app.xyzzy.as_ref().is_some_and(|menu| menu.reviewing));
    assert_eq!(ok(app.game.world.save_bytes()), before);
    press(&mut app, KeyCode::Escape);
    assert!(app.xyzzy.as_ref().is_some_and(|menu| !menu.reviewing));
    press(&mut app, KeyCode::KeyB);
    assert!(app.xyzzy.is_none());
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert!(!directory.0.join("commander.meridian").exists());
}

#[test]
fn actual_secret_menu_publishes_clarke_real_outfit_and_visible_provenance_then_cold_loads() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    activate(&mut app);
    let world = &app.game.world;
    assert_eq!(world.credits_tenths(), Some(10_000_000));
    let modules = world
        .modular_equipment_snapshot()
        .unwrap_or_else(|| panic!("modules"));
    for kind in [
        ModuleKind::NavalLaser,
        ModuleKind::EliteEnergyUnit,
        ModuleKind::AdvancedNavigation,
        ModuleKind::Targeting(meridian_core::equipment::TargetingTier::Military),
        ModuleKind::FuelScoops,
    ] {
        assert!(modules.items.iter().any(|item| item.kind == kind
            && item.slot.is_some()
            && matches!(item.provenance, ModuleProvenance::XYZzy { .. })));
    }
    assert!(
        world
            .equipment_snapshot()
            .and_then(|snapshot| snapshot.owned)
            .is_some_and(|item| item.xyzzy && item.fitted)
    );
    let specialists = world.work_targeting_fit();
    assert!(specialists.mining && specialists.salvage);
    let emp = world.owned_emp().unwrap_or_else(|| panic!("EMP"));
    assert!(emp.fitted);
    assert_eq!(
        (emp.grades.power, emp.grades.capacity, emp.grades.recharge),
        (3, 3, 3)
    );
    let ammo = world
        .ammunition_snapshot()
        .unwrap_or_else(|| panic!("ammo"));
    assert_eq!(
        ammo.items
            .iter()
            .filter(|item| item.pylon.is_some() && !item.consumed)
            .count(),
        9
    );
    assert!(ammo.purchases.is_empty());
    assert!(app.game.lines(true, None).is_empty());
    assert!(
        app.frame_title(true, FlightMode::Docked)
            .starts_with("CLARKE / XYZZY")
    );
    let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
    for screen in ui::Screen::ALL {
        let mut docked = ok(Docked::new(&app.game.world));
        ok(docked.open(&app.game.world, screen));
        let frame = ok(App::build_docked_frame(
            true,
            || Some([viewport.width, viewport.height]),
            Some(&mut docked),
            &app.game.world,
            0.0,
        ))
        .unwrap_or_else(|| panic!("frame"));
        assert!(frame.labels[0].text.contains("CLARKE / XYZZY"));
    }
    let bytes = ok(fs::read(directory.0.join("commander.meridian")));
    let cold_world = ok(World::from_save_bytes(&bytes));
    assert_eq!(cold_world.commander_name(), Some("CLARKE"));
    assert_eq!(
        cold_world.modular_equipment_snapshot(),
        app.game.world.modular_equipment_snapshot()
    );
    assert_eq!(
        cold_world.ammunition_snapshot(),
        app.game.world.ammunition_snapshot()
    );
    drop(app);
    let mut cold = open(&directory.0);
    ok(cold.continue_saved());
    assert_eq!(cold.world.commander_name(), Some("CLARKE"));
    assert_eq!(cold.world.snapshot().mode, FlightMode::Docked);
    assert!(cold.lines(true, None).is_empty());
    assert!(
        cold.world
            .xyzzy_snapshot()
            .is_some_and(|state| state.active && state.unlimited_fuel)
    );
    let mut cold_app = App::with_profile(cold);
    menu(&mut cold_app);
    let panel = cold_app
        .xyzzy
        .as_ref()
        .unwrap_or_else(|| panic!("missing cold XYZZY panel"))
        .panel(&cold_app.game.world, None);
    assert!(
        panel
            .lines
            .iter()
            .any(|line| line.contains("Fuel UNLIMITED"))
    );
}

#[test]
fn disposable_preview_selects_each_stock_shield_without_replacing_maximum_other_equipment() {
    use meridian_core::ShieldGrade;
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    activate(&mut app);
    let cash = app.game.world.credits_tenths();
    let fuel = app.game.world.snapshot().navigation;
    let ammo = app.game.world.ammunition_snapshot();
    let other_modules = |world: &World| {
        world
            .modular_equipment_snapshot()
            .unwrap_or_else(|| panic!("modules"))
            .items
            .into_iter()
            .filter(|item| item.kind.slot() != meridian_core::ModuleSlot::Shield)
            .collect::<Vec<_>>()
    };
    let original = other_modules(&app.game.world);
    menu(&mut app);
    press(&mut app, KeyCode::ArrowUp);
    press(&mut app, KeyCode::ArrowUp);
    for grade in ShieldGrade::ALL {
        assert_eq!(
            app.xyzzy.as_ref().map(crate::xyzzy::Menu::action),
            Some(XyzzyAction::FitShield(grade))
        );
        press(&mut app, KeyCode::Enter);
        // A reviewing row cannot change the grade until cancellation or publication.
        press(&mut app, KeyCode::ArrowRight);
        assert_eq!(
            app.xyzzy.as_ref().map(crate::xyzzy::Menu::action),
            Some(XyzzyAction::FitShield(grade))
        );
        press(&mut app, KeyCode::Enter);
        let player = app
            .game
            .world
            .combat_snapshot()
            .unwrap_or_else(|| panic!("combat"))
            .player;
        assert_eq!(
            (
                player.shield_grade,
                player.front_shield_hundredths,
                player.rear_shield_hundredths
            ),
            (grade, grade.capacity(), grade.capacity())
        );
        assert_eq!(app.game.world.credits_tenths(), cash);
        assert_eq!(app.game.world.snapshot().navigation, fuel);
        assert_eq!(app.game.world.ammunition_snapshot(), ammo);
        assert_eq!(other_modules(&app.game.world), original);
        let saved = ok(fs::read(directory.0.join("commander.meridian")));
        assert_eq!(
            ok(World::from_save_bytes(&saved)).combat_snapshot(),
            app.game.world.combat_snapshot()
        );
        press(&mut app, KeyCode::ArrowRight);
    }
    press(&mut app, KeyCode::ArrowDown); // pass the explicit four-tank test row
    press(&mut app, KeyCode::ArrowDown); // wrap to ordinary activation / maximum
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.game
            .world
            .combat_snapshot()
            .unwrap_or_else(|| panic!("combat"))
            .player
            .shield_grade,
        ShieldGrade::Elite
    );
}

#[test]
fn actual_max_tank_row_is_explicit_and_publishes_four_distinct_items_to_disk() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    menu(&mut app);
    press(&mut app, KeyCode::ArrowUp);
    assert_eq!(
        app.xyzzy.as_ref().map(crate::xyzzy::Menu::action),
        Some(XyzzyAction::MaximumFuelStorage)
    );
    let panel = app
        .xyzzy
        .as_ref()
        .unwrap_or_else(|| panic!("menu"))
        .panel(&app.game.world, None);
    assert!(panel.lines.iter().any(|line| line.contains("TEST MAX")));
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.game
            .world
            .snapshot()
            .navigation
            .map(|n| n.fuel_capacity_units),
        Some(130)
    );
    press(&mut app, KeyCode::Enter);
    let fuel = app
        .game
        .world
        .snapshot()
        .navigation
        .unwrap_or_else(|| panic!("fuel"));
    assert_eq!((fuel.fuel_units, fuel.fuel_capacity_units), (410, 410));
    let snapshot = app
        .game
        .world
        .modular_equipment_snapshot()
        .unwrap_or_else(|| panic!("equipment"));
    let tanks = snapshot
        .items
        .iter()
        .filter(|item| item.kind == ModuleKind::FuelStorage)
        .collect::<Vec<_>>();
    assert_eq!(tanks.len(), 4);
    assert!(tanks.iter().all(
        |item| item.slot.is_some() && matches!(item.provenance, ModuleProvenance::XYZzy { .. })
    ));
    let bytes = ok(fs::read(directory.0.join("commander.meridian")));
    let cold = ok(World::from_save_bytes(&bytes));
    assert_eq!(cold.snapshot().navigation, Some(fuel));
    assert_eq!(cold.modular_equipment_snapshot(), Some(snapshot));
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.game.world.modular_equipment_snapshot(),
        cold.modular_equipment_snapshot()
    );
}

#[test]
fn dedicated_fuel_scoops_row_does_not_change_other_equipment_or_fuel() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    activate(&mut app);
    let before = app.game.world.modular_equipment_snapshot();
    let fuel = app.game.world.snapshot().navigation;
    let cash = app.game.world.credits_tenths();
    menu(&mut app);
    for _ in 0..4 {
        press(&mut app, KeyCode::ArrowDown);
    }
    assert_eq!(
        app.xyzzy.as_ref().map(crate::xyzzy::Menu::action),
        Some(XyzzyAction::FitFuelScoops)
    );
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.game.world.modular_equipment_snapshot(), before);
    assert_eq!(app.game.world.snapshot().navigation, fuel);
    assert_eq!(app.game.world.credits_tenths(), cash);
    let saved = ok(fs::read(directory.0.join("commander.meridian")));
    assert_eq!(
        ok(World::from_save_bytes(&saved)).modular_equipment_snapshot(),
        before
    );
}

#[test]
fn failed_actual_profile_publication_retains_world_saved_bytes_and_secret_review() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    ok(app.game.save());
    let slot = directory.0.join("commander.meridian");
    let retained = directory.0.join("retained-ordinary-commander");
    let before = ok(fs::read(&slot));
    ok(fs::rename(&slot, &retained));
    ok(fs::create_dir(&slot));
    menu(&mut app);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert!(
        app.notice
            .text
            .as_deref()
            .is_some_and(|message| message.contains("could not be saved"))
    );
    assert!(app.xyzzy.as_ref().is_some_and(|menu| menu.reviewing));
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert_eq!(ok(fs::read(&retained)), before);
    assert!(slot.is_dir());
    assert_eq!(app.game.world.commander_name(), Some("Rowan Calder"));
    assert!(
        app.game
            .world
            .xyzzy_snapshot()
            .is_some_and(|state| !state.active)
    );
}

#[test]
fn help_above_secret_confirmation_blocks_publication_and_resumes_the_same_review() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    menu(&mut app);
    press(&mut app, KeyCode::Enter);
    let before = ok(app.game.world.save_bytes());
    press(&mut app, KeyCode::KeyH);
    assert!(
        app.help
            .as_ref()
            .is_some_and(|panel| panel.title == "XYZZY testing help")
    );
    assert_eq!(press(&mut app, KeyCode::Enter), None);
    assert_eq!(press(&mut app, KeyCode::F5), None);
    assert_eq!(app.advance_frame(true, 1200), FrameAdvance::Continue(None));
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert!(!directory.0.join("commander.meridian").exists());
    press(&mut app, KeyCode::Escape);
    assert!(app.help.is_none());
    assert!(app.xyzzy.as_ref().is_some_and(|menu| menu.reviewing));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.game.world.commander_name(), Some("CLARKE"));
}

#[test]
fn activate_refit_and_toggle_fuel_without_leaving_the_menu() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    let mut station = ok(Docked::new(&app.game.world));
    ok(station.open(&app.game.world, ui::Screen::Equipment));
    app.docked = Some(station);
    menu(&mut app);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert!(app.xyzzy.as_ref().is_some_and(|menu| !menu.reviewing));
    let items = app.game.world.ammunition_snapshot();
    let modules = app.game.world.modular_equipment_snapshot();
    for steps in [1, 1, 5] {
        for _ in 0..steps {
            press(&mut app, KeyCode::ArrowDown);
        }
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.game.world.credits_tenths(), Some(10_000_000));
        assert_eq!(app.game.world.ammunition_snapshot(), items);
        assert_eq!(app.game.world.modular_equipment_snapshot(), modules);
    }
    assert!(app.xyzzy.as_ref().is_some_and(|menu| !menu.reviewing));
    // Up wraps through tank, shield and scoop testing rows to the fuel action.
    press(&mut app, KeyCode::ArrowUp);
    press(&mut app, KeyCode::ArrowUp);
    press(&mut app, KeyCode::ArrowUp);
    press(&mut app, KeyCode::ArrowUp);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::ArrowDown);
    assert_eq!(
        app.xyzzy.as_ref().map(crate::xyzzy::Menu::action),
        Some(XyzzyAction::ToggleUnlimitedFuel)
    );
    press(&mut app, KeyCode::Enter);
    assert!(
        app.game
            .world
            .xyzzy_snapshot()
            .is_some_and(|state| !state.unlimited_fuel)
    );
    press(&mut app, KeyCode::Escape);
    assert!(app.xyzzy.is_none());
    assert_eq!(
        app.docked.as_ref().map(Docked::screen),
        Some(ui::Screen::Equipment)
    );
    let cold = ok(World::from_save_bytes(&ok(fs::read(
        directory.0.join("commander.meridian"),
    ))));
    assert!(
        cold.xyzzy_snapshot()
            .is_some_and(|state| !state.unlimited_fuel)
    );
}

#[test]
fn secret_menu_freezes_clock_input_and_focus_without_changing_manual_pause() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    app.keys.extend([KeyCode::KeyW, KeyCode::KeyZ]);
    app.paused = true;
    menu(&mut app);
    let before = ok(app.game.world.save_bytes());
    assert!(app.keys.is_empty());
    app.focus_changed(false);
    app.occlusion_changed(true);
    assert_eq!(app.advance_frame(true, 3000), FrameAdvance::Continue(None));
    assert_eq!(ok(app.game.world.save_bytes()), before);
    app.focus_changed(true);
    app.occlusion_changed(false);
    assert_eq!(press(&mut app, KeyCode::KeyP), None);
    press(&mut app, KeyCode::Escape);
    assert!(app.paused);
    assert!(app.keys.is_empty());
    assert!(!app.simulation_active());
}

#[test]
fn title_and_actual_flight_refuse_secret_menu_without_creating_or_replacing_a_profile() {
    let directory = TestDirectory::new();
    let mut app = App::with_profile(open(&directory.0));
    app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
    assert_eq!(press(&mut app, KeyCode::KeyX), None);
    assert!(app.xyzzy.is_none());
    assert!(!app.game.playing());
    assert!(!directory.0.join("commander.meridian").exists());
    assert!(ok(app.game.new_commander(99035)));
    ok(app.game.command(Action::Launch));
    for _ in 0..120 {
        ok(app.game.world.step(FlightInput::default()));
    }
    assert_eq!(app.game.world.snapshot().mode, FlightMode::Flight);
    let before = ok(app.game.world.save_bytes());
    assert_eq!(press(&mut app, KeyCode::KeyX), None);
    assert!(app.xyzzy.is_none());
    assert_eq!(ok(app.game.world.save_bytes()), before);
    assert!(!directory.0.join("commander.meridian").exists());
}

#[test]
fn paid_mining_fit_help_uses_real_tool_without_advertising_automatic_pickup() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    activate(&mut app);
    ok(app.game.command(Action::Module(
        meridian_core::ModuleAction::PurchaseMiningLaser,
    )));
    let paid = app
        .game
        .world
        .modular_equipment_snapshot()
        .unwrap_or_else(|| panic!("modules"));
    assert!(
        paid.items
            .iter()
            .any(|item| item.kind == ModuleKind::MiningLaser
                && matches!(item.provenance, ModuleProvenance::Paid { .. }))
    );
    ok(app.game.command(Action::Launch));
    for _ in 0..120 {
        ok(app.game.world.step(FlightInput::default()));
    }
    let panel = app.help_panel();
    let text = panel.lines.join("\n");
    assert!(text.contains("Z Mine eligible rock with fitted Mining Laser"));
    assert!(text.contains("no automatic pickup"));
    assert!(!text.contains("Z unavailable"));
    assert!(!text.contains("Fire fitted Pulse"));
}

#[test]
fn testing_menu_h_help_wins_over_hidden_galaxy_and_showroom_search_without_losing_query() {
    let directory = TestDirectory::new();
    let mut app = app(&directory);
    let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
    for screen in [ui::Screen::Galaxy, ui::Screen::Showroom] {
        let mut docked = ok(Docked::new(&app.game.world));
        ok(docked.open(&app.game.world, screen));
        let frame = ok(docked.build_frame(&app.game.world, viewport, 0.0));
        docked.presented(frame);
        app.docked = Some(docked);
        app.menu_input(ui::Input::Key(ui::Key::Search));
        // H is still ordinary query text while search is the top context.
        assert_eq!(
            app.route_keyboard(
                PhysicalKey::Code(KeyCode::KeyH),
                ElementState::Pressed,
                false,
                Some("h")
            ),
            None
        );
        let before = ok(app
            .docked
            .as_mut()
            .unwrap_or_else(|| panic!("menu"))
            .build_frame(&app.game.world, viewport, 0.0));
        let world = ok(app.game.world.save_bytes());
        menu(&mut app);
        press(&mut app, KeyCode::Enter);
        assert!(app.xyzzy.as_ref().is_some_and(|menu| menu.reviewing));
        assert_eq!(
            app.route_keyboard(
                PhysicalKey::Code(KeyCode::KeyH),
                ElementState::Pressed,
                false,
                Some("h")
            ),
            None
        );
        assert!(
            app.help
                .as_ref()
                .is_some_and(|panel| panel.title == "XYZZY testing help")
        );
        press(&mut app, KeyCode::Escape);
        assert!(app.help.is_none());
        assert!(app.xyzzy.as_ref().is_some_and(|menu| menu.reviewing));
        press(&mut app, KeyCode::Escape);
        assert!(app.xyzzy.as_ref().is_some_and(|menu| !menu.reviewing));
        press(&mut app, KeyCode::Escape);
        assert!(app.xyzzy.is_none());
        assert!(app.docked.as_ref().is_some_and(Docked::search_active));
        let after = ok(app
            .docked
            .as_mut()
            .unwrap_or_else(|| panic!("menu"))
            .build_frame(&app.game.world, viewport, 0.0));
        assert_eq!(after.labels, before.labels, "{screen:?}");
        assert_eq!(ok(app.game.world.save_bytes()), world);
        assert!(!directory.0.join("commander.meridian").exists());
    }
}

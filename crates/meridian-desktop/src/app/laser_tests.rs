//! Real App keys/held fire, presented menu review, file publisher and fresh profile reader.
//! Authored host tests are not native rendering/gameplay evidence.
use super::{Action, App, Docked, ElementState, FrameAdvance, KeyCode, PhysicalKey, ui};
use crate::profile::tests::{TestDirectory, open};
use meridian_core::{
    FlightView, LaserFacing, ModuleAction, ModuleKind, ModuleProvenance, XyzzyAction,
};

fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("expected actual item/frame"))
}
fn app(directory: &TestDirectory, seed: u64) -> App {
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(seed)));
    App::with_profile(profile)
}
fn key(a: &mut App, code: KeyCode, state: ElementState) -> Option<KeyCode> {
    a.route_keyboard(PhysicalKey::Code(code), state, false, None)
}
fn game_key(a: &mut App, code: KeyCode) {
    assert_eq!(key(a, code, ElementState::Pressed), Some(code));
    a.game_key(code);
    key(a, code, ElementState::Released);
}
fn paint(a: &mut App) -> ui::Frame {
    let menu = some(a.docked.as_mut());
    let frame = ok(menu.build_frame(&a.game.world, some(ui::Viewport::new(1280.0, 720.0)), 0.0));
    menu.presented(frame.clone());
    frame
}
fn menu_key(a: &mut App, code: ui::Key) -> ui::Frame {
    a.menu_input(ui::Input::Key(code));
    paint(a)
}
fn select_purchase(a: &mut App, name: &str) {
    for _ in 0..16 {
        if paint(a)
            .labels
            .iter()
            .any(|l| l.text == format!("> {name} (purchase)"))
        {
            return;
        }
        menu_key(a, ui::Key::Next);
    }
    panic!("actual equipment list did not expose the requested purchase");
}
fn launch(a: &mut App) {
    game_key(a, KeyCode::KeyL);
    assert_eq!(a.advance_frame(true, 1), FrameAdvance::Continue(None));
}

#[test]
fn actual_digit_view_binding_drives_each_fitted_body_axis_and_fresh_profile_keeps_selection_and_spend()
 {
    for (index, facing, view, code, expected) in [
        (
            0,
            LaserFacing::Front,
            FlightView::Front,
            KeyCode::Digit0,
            [0.0, 0.0, 1.0],
        ),
        (
            1,
            LaserFacing::Rear,
            FlightView::Rear,
            KeyCode::Digit1,
            [0.0, 0.0, -1.0],
        ),
        (
            2,
            LaserFacing::Left,
            FlightView::Left,
            KeyCode::Digit2,
            [-1.0, 0.0, 0.0],
        ),
        (
            3,
            LaserFacing::Right,
            FlightView::Right,
            KeyCode::Digit3,
            [1.0, 0.0, 0.0],
        ),
    ] {
        let directory = TestDirectory::new();
        let mut a = app(&directory, 27201 + index);
        let item = some(a.game.world.front_laser_snapshot()).fitted_item_id;
        if facing != LaserFacing::Front {
            ok(a.game.command(Action::Module(ModuleAction::InstallLaser {
                item,
                facing,
                replacing: None,
            })));
        }
        launch(&mut a);
        game_key(&mut a, code);
        assert_eq!(a.game.world.flight_view(), view);
        let before = some(a.game.world.flight_resources_snapshot());
        key(&mut a, KeyCode::KeyZ, ElementState::Pressed);
        assert_eq!(a.advance_frame(true, 1), FrameAdvance::Continue(None));
        key(&mut a, KeyCode::KeyZ, ElementState::Released);
        let shot = some(a.pulse);
        assert_eq!(
            [shot.direction.x, shot.direction.y, shot.direction.z],
            expected
        );
        assert_eq!(shot.fitted_item_id, item);
        assert!(a.mining.is_none());
        let resources = some(a.game.world.flight_resources_snapshot());
        assert_eq!(resources.energy_hundredths, before.energy_hundredths - 100);
        assert_eq!(resources.heat_fifths, 50);
        assert_eq!(resources.cooldown_ticks, 18);
        assert!(
            a.game
                .lines(true, None)
                .iter()
                .any(|line| line.contains(&format!(
                    "Z FIRE {} PULSE",
                    format!("{facing:?}").to_ascii_uppercase()
                )))
        );
        ok(a.game.prepare_quit());
        let bytes = ok(a.game.world.save_bytes());
        assert_eq!(
            ok(std::fs::read(directory.0.join("commander.meridian"))),
            bytes
        );
        let mut cold = open(&directory.0);
        ok(cold.continue_saved());
        assert_eq!(cold.world.flight_view(), view);
        assert_eq!(
            some(cold.world.selected_laser_snapshot()).fitted_item_id,
            item
        );
        assert_eq!(cold.world.flight_resources_snapshot(), Some(resources));
        assert_eq!(
            cold.world.modular_equipment_snapshot(),
            a.game.world.modular_equipment_snapshot()
        );
    }
}

#[test]
fn selected_empty_mount_does_not_fall_back_to_front_and_help_hud_explain_the_refusal() {
    let directory = TestDirectory::new();
    let mut a = app(&directory, 27211);
    launch(&mut a);
    game_key(&mut a, KeyCode::Digit3);
    let resources = a.game.world.flight_resources_snapshot();
    let combat = a.game.world.combat_snapshot();
    key(&mut a, KeyCode::KeyZ, ElementState::Pressed);
    assert_eq!(a.advance_frame(true, 1), FrameAdvance::Continue(None));
    key(&mut a, KeyCode::KeyZ, ElementState::Released);
    assert!(a.pulse.is_none());
    assert!(a.mining.is_none());
    assert_eq!(a.game.world.flight_resources_snapshot(), resources);
    assert_eq!(a.game.world.combat_snapshot(), combat);
    assert!(
        a.game
            .lines(true, None)
            .iter()
            .any(|line| line.contains("RIGHT UNAVAILABLE: NO FITTED LASER"))
    );
    assert!(
        a.help_panel()
            .lines
            .iter()
            .any(|line| line.contains("no fitted Right laser"))
    );
}

#[test]
fn presented_paid_left_and_rear_purchase_charge_once_then_right_offer_refuses_and_cold_custody_matches()
 {
    let directory = TestDirectory::new();
    let mut a = app(&directory, 27221);
    super::emp_tests::earned(&mut a); // Existing public finite-market/charged-travel pilot.
    assert_eq!(a.game.world.credits_tenths(), Some(33650));
    assert!(!some(a.game.world.xyzzy_snapshot()).active);
    ok(a.game.save());
    let mut menu = ok(Docked::new(&a.game.world));
    ok(menu.open(&a.game.world, ui::Screen::Equipment));
    a.docked = Some(menu);
    paint(&mut a);
    menu_key(&mut a, ui::Key::NextSlot); // Left, following the displayed four-mount order.
    let mut ids = Vec::new();
    for (facing, balance) in [(LaserFacing::Left, 19150), (LaserFacing::Rear, 4650)] {
        select_purchase(&mut a, "Pulse Laser");
        let before = ok(a.game.world.save_bytes());
        let disk = ok(std::fs::read(directory.0.join("commander.meridian")));
        let review = menu_key(&mut a, ui::Key::Buy);
        assert!(
            review
                .labels
                .iter()
                .any(|l| l.text.contains("Offer 1450.0 CR"))
        );
        assert_eq!(ok(a.game.world.save_bytes()), before);
        assert_eq!(
            ok(std::fs::read(directory.0.join("commander.meridian"))),
            disk
        );
        menu_key(&mut a, ui::Key::Confirm);
        assert_eq!(a.game.world.credits_tenths(), Some(balance));
        let item = some(a.game.world.laser_snapshot(facing)).fitted_item_id;
        assert!(
            some(a.game.world.modular_equipment_snapshot())
                .items
                .into_iter()
                .any(|owned| owned.id == item
                    && matches!(owned.provenance, ModuleProvenance::Paid { .. }))
        );
        ids.push(item);
        assert_eq!(
            ok(std::fs::read(directory.0.join("commander.meridian"))),
            ok(a.game.world.save_bytes())
        );
        if facing == LaserFacing::Left {
            menu_key(&mut a, ui::Key::NextSlot); // Right.
            menu_key(&mut a, ui::Key::NextSlot); // Rear.
        }
    }
    assert_ne!(ids[0], ids[1]);
    // Reopen the actual menu at front, then choose Right with the actual slot key.
    let mut menu = ok(Docked::new(&a.game.world));
    ok(menu.open(&a.game.world, ui::Screen::Equipment));
    a.docked = Some(menu);
    paint(&mut a);
    menu_key(&mut a, ui::Key::NextSlot);
    menu_key(&mut a, ui::Key::NextSlot);
    select_purchase(&mut a, "Pulse Laser");
    let before = ok(a.game.world.save_bytes());
    let refused = menu_key(&mut a, ui::Key::Buy);
    assert!(
        refused
            .labels
            .iter()
            .any(|l| l.text.contains("InsufficientCredits"))
    );
    menu_key(&mut a, ui::Key::Confirm);
    assert_eq!(ok(a.game.world.save_bytes()), before);
    assert!(a.game.world.laser_snapshot(LaserFacing::Right).is_none());
    let mut cold = open(&directory.0);
    ok(cold.continue_saved());
    assert_eq!(
        cold.world.modular_equipment_snapshot(),
        a.game.world.modular_equipment_snapshot()
    );
    assert_eq!(cold.world.credits_tenths(), Some(4650));
}

#[test]
fn front_mining_does_not_steal_side_fire_and_a_side_mining_laser_is_an_ordinary_selected_weapon() {
    let directory = TestDirectory::new();
    let mut a = app(&directory, 27231);
    ok(a.game.xyzzy(XyzzyAction::Enter)); // Explicit testing setup, not earned/native evidence.
    let pulse = some(
        some(a.game.world.modular_equipment_snapshot())
            .items
            .into_iter()
            .find(|item| item.kind == ModuleKind::PulseLaser),
    )
    .id;
    ok(a.game
        .command(Action::Module(ModuleAction::PurchaseMiningLaser)));
    let mining = some(a.game.world.front_laser_snapshot()).fitted_item_id;
    ok(a.game.command(Action::Module(ModuleAction::InstallLaser {
        item: pulse,
        facing: LaserFacing::Left,
        replacing: None,
    })));
    launch(&mut a);
    game_key(&mut a, KeyCode::Digit2);
    key(&mut a, KeyCode::KeyZ, ElementState::Pressed);
    assert_eq!(a.advance_frame(true, 1), FrameAdvance::Continue(None));
    key(&mut a, KeyCode::KeyZ, ElementState::Released);
    assert_eq!(some(a.pulse).fitted_item_id, pulse);
    assert!(a.mining.is_none());
    assert!(a.game.world.mining_tool_authority().is_ok()); // Front tool exists during side shot.
    let previous = some(a.pulse);
    game_key(&mut a, KeyCode::Digit0);
    assert_eq!(a.advance_frame(true, 18), FrameAdvance::Continue(None));
    key(&mut a, KeyCode::KeyZ, ElementState::Pressed);
    assert_eq!(a.advance_frame(true, 1), FrameAdvance::Continue(None));
    key(&mut a, KeyCode::KeyZ, ElementState::Released);
    assert_eq!(a.pulse, Some(previous)); // Front work has no eligible rock; no ordinary fire fallback.
    assert!(
        a.help_panel()
            .lines
            .iter()
            .any(|line| line.contains("Z Mine eligible rock"))
    );
    // A separate actual docked transaction moves the same paid mining tool to Rear.
    let other = TestDirectory::new();
    let mut b = app(&other, 27232);
    ok(b.game.xyzzy(XyzzyAction::Enter));
    ok(b.game
        .command(Action::Module(ModuleAction::PurchaseMiningLaser)));
    let paid = some(b.game.world.front_laser_snapshot()).fitted_item_id;
    assert_ne!(paid, mining);
    ok(b.game.command(Action::Module(ModuleAction::InstallLaser {
        item: paid,
        facing: LaserFacing::Rear,
        replacing: None,
    })));
    launch(&mut b);
    game_key(&mut b, KeyCode::Digit1);
    assert!(b.game.world.mining_tool_authority().is_err());
    key(&mut b, KeyCode::KeyZ, ElementState::Pressed);
    assert_eq!(b.advance_frame(true, 1), FrameAdvance::Continue(None));
    assert_eq!(some(b.pulse).fitted_item_id, paid);
    assert_eq!(
        some(b.game.world.selected_laser_snapshot()).damage_hundredths,
        5000
    );
    assert!(b.mining.is_none());
    let help = b.help_panel().lines.join("\n");
    assert!(help.contains("Z Fire Rear Mining Laser"));
    assert!(!help.contains("Z Mine eligible rock"));
}

#[test]
fn actual_selected_help_preserves_all_screen_maps_and_blocks_view_fire_and_publication_while_open()
{
    let directory = TestDirectory::new();
    let mut a = app(&directory, 27241);
    for screen in ui::Screen::ALL {
        let mut menu = ok(Docked::new(&a.game.world));
        ok(menu.open(&a.game.world, screen));
        a.docked = Some(menu);
        paint(&mut a);
        let text = a.help_panel().lines.join("\n");
        for required in [
            "1 Galaxy",
            "2 System",
            "3 Showroom",
            "4 Market",
            "5 Equipment",
            "Tab / Shift+Tab",
        ] {
            assert!(text.contains(required));
        }
        if screen == ui::Screen::Equipment {
            assert!(text.contains("Front / left / right / rear"));
        }
    }
    a.docked = None;
    launch(&mut a);
    for (facing, code) in [
        (LaserFacing::Front, KeyCode::Digit0),
        (LaserFacing::Rear, KeyCode::Digit1),
        (LaserFacing::Left, KeyCode::Digit2),
        (LaserFacing::Right, KeyCode::Digit3),
    ] {
        game_key(&mut a, code);
        let before = ok(a.game.world.save_bytes());
        assert_eq!(key(&mut a, KeyCode::KeyH, ElementState::Pressed), None);
        let panel = some(a.help.as_ref());
        assert!(
            panel
                .lines
                .iter()
                .any(|line| line.contains("0 Front | 1 Rear | 2 Left | 3 Right"))
        );
        assert!(panel.lines.len() + 2 <= crate::help::MAX_LINES);
        assert!(
            panel
                .lines
                .iter()
                .all(|line| line.len() <= crate::help::MAX_COLUMNS)
        );
        if facing != LaserFacing::Front {
            assert!(
                panel
                    .lines
                    .iter()
                    .any(|line| line.contains(&format!("no fitted {facing:?} laser")))
            );
        }
        for blocked in [
            KeyCode::Digit0,
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::KeyZ,
            KeyCode::F5,
        ] {
            assert_eq!(key(&mut a, blocked, ElementState::Pressed), None);
        }
        assert_eq!(a.advance_frame(true, 60), FrameAdvance::Continue(None));
        assert_eq!(ok(a.game.world.save_bytes()), before);
        assert_eq!(a.game.world.selected_laser_facing(), facing);
        assert_eq!(key(&mut a, KeyCode::Escape, ElementState::Pressed), None);
        assert!(a.help.is_none());
        assert!(a.keys.is_empty());
    }
}

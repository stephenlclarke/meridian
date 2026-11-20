//! Real host input, presented station reviews and atomic files; all profiles are disposable.
use super::{
    Action, App, Docked, ElementState, FlightInput, FlightMode, FrameAdvance, KeyCode, PhysicalKey,
    World, ui,
};
use crate::profile::tests::{TestDirectory, open};
use meridian_core::navigation::{BASIC_SPOOL_TICKS, SystemId};
use meridian_core::station_services::{Command, CommodityId};
use meridian_core::{EmpAxis, EmpChargePhase, EmpEquipmentAction, EmpProvenance, XyzzyAction};
use std::fs;
fn ok<T, E: std::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn app(d: &TestDirectory, seed: u64) -> App {
    let mut p = open(&d.0);
    assert!(ok(p.new_commander(seed)));
    App::with_profile(p)
}
fn key(a: &mut App, code: KeyCode, state: ElementState) -> Option<KeyCode> {
    a.route_keyboard(PhysicalKey::Code(code), state, false, None)
}
fn paint(a: &mut App) -> ui::Frame {
    let view = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
    let m = a.docked.as_mut().unwrap_or_else(|| panic!("menu"));
    let frame = ok(m.build_frame(&a.game.world, view, 0.0));
    m.presented(frame.clone());
    frame
}
fn menu_key(a: &mut App, k: ui::Key) {
    a.menu_input(ui::Input::Key(k));
    paint(a);
}
fn shop(a: &mut App) {
    let mut m = ok(Docked::new(&a.game.world));
    ok(m.open(&a.game.world, ui::Screen::Equipment));
    a.docked = Some(m);
    paint(a);
    for _ in 0..5 {
        menu_key(a, ui::Key::NextSlot);
    }
}
// Funding uses the original finite markets and charged public travel, never a wallet fixture.
fn journey(w: &mut World, target: SystemId) {
    ok(w.request_launch(w.snapshot().tick));
    ok(w.step(FlightInput::default()));
    ok(w.request_jump(target, w.snapshot().tick));
    for _ in 0..BASIC_SPOOL_TICKS {
        ok(w.step(FlightInput::default()));
    }
    for _ in 0..9000 {
        let s = w.snapshot();
        if s.mode == FlightMode::Docked {
            return;
        }
        let speed = if s.position[2] > 2500.0 { 34.3 } else { 4.0 };
        ok(w.step(FlightInput {
            throttle: ((speed - s.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
            ..FlightInput::default()
        }));
        assert_eq!(w.snapshot().motion_refusal, None);
    }
    panic!("actual public flight failed to berth");
}
fn trade(w: &mut World, id: CommodityId, n: usize, buy: bool) {
    for _ in 0..n {
        ok(w.request_station_service(
            if buy {
                Command::BuyCommodity(id)
            } else {
                Command::SellCommodity(id)
            },
            w.snapshot().tick,
        ));
    }
}
fn earned(a: &mut App) {
    let w = &mut a.game.world;
    for (quantity, cash) in [(29, 11434), (47, 18296), (75, 29042)] {
        if w.snapshot()
            .navigation
            .unwrap_or_else(|| panic!("nav"))
            .system
            != SystemId::AVENOR
        {
            if w.snapshot()
                .navigation
                .unwrap_or_else(|| panic!("nav"))
                .fuel_units
                < 68
            {
                ok(w.request_refuel(w.snapshot().tick));
            }
            journey(w, SystemId::AVENOR);
        }
        trade(w, CommodityId::Minerals, quantity, true);
        journey(w, SystemId::RENAS);
        trade(w, CommodityId::Minerals, quantity, false);
        assert_eq!(w.credits_tenths(), Some(cash));
    }
    trade(w, CommodityId::Alloys, 48, true);
    journey(w, SystemId::AVENOR);
    trade(w, CommodityId::Alloys, 48, false);
    assert_eq!(w.credits_tenths(), Some(33650));
}
#[test]
fn presented_paid_emp_purchase_store_refit_and_cold_continue_keep_one_empty_owned_item() {
    let d = TestDirectory::new();
    let mut a = app(&d, 19101);
    earned(&mut a);
    shop(&mut a);
    let before = ok(a.game.world.save_bytes());
    menu_key(&mut a, ui::Key::Buy);
    assert_eq!(ok(a.game.world.save_bytes()), before);
    menu_key(&mut a, ui::Key::Confirm);
    let item = a
        .game
        .world
        .owned_emp()
        .unwrap_or_else(|| panic!("paid EMP"));
    assert_eq!(item.provenance, EmpProvenance::Paid);
    assert_eq!(item.paid_tenths, 30000);
    assert_eq!(a.game.world.credits_tenths(), Some(3650));
    assert_eq!(
        a.game.world.emp_snapshot().map(|s| s.bank_energy),
        Some(0.0)
    );
    menu_key(&mut a, ui::Key::Store);
    menu_key(&mut a, ui::Key::Confirm);
    assert!(
        !a.game
            .world
            .owned_emp()
            .unwrap_or_else(|| panic!("stored"))
            .fitted
    );
    menu_key(&mut a, ui::Key::Fit);
    menu_key(&mut a, ui::Key::Confirm);
    assert_eq!(a.game.world.owned_emp(), Some(item));
    let snapshot = a.game.world.emp_snapshot();
    drop(a);
    let mut p = open(&d.0);
    ok(p.continue_saved());
    assert_eq!(p.world.emp_snapshot(), snapshot);
    assert_eq!(p.world.credits_tenths(), Some(3650));
}
#[test]
fn paid_emp_grade_review_refuses_failed_atomic_writer_and_keeps_same_paid_base() {
    let d = TestDirectory::new();
    // Continue a real pre-upgrade save containing a paid base and the old XYZZY grant.
    let bytes = include_bytes!("../../../meridian-core/tests/fixtures/xyzzy-paid-base-v17.bin");
    ok(fs::write(d.0.join("commander.meridian"), bytes));
    let mut profile = open(&d.0);
    ok(profile.continue_saved());
    let mut a = App::with_profile(profile);
    let item = a.game.world.owned_emp().unwrap_or_else(|| panic!("item"));
    assert_eq!(item.provenance, EmpProvenance::Paid);
    shop(&mut a);
    menu_key(&mut a, ui::Key::Next);
    menu_key(&mut a, ui::Key::Buy);
    let slot = d.0.join("commander.meridian");
    let retained = d.0.join("retained-valid-commander");
    let before = ok(fs::read(&slot));
    ok(fs::rename(&slot, &retained));
    ok(fs::create_dir(&slot));
    menu_key(&mut a, ui::Key::Confirm);
    assert_eq!(ok(a.game.world.save_bytes()), before);
    assert_eq!(ok(fs::read(&retained)), before);
    assert_eq!(a.game.world.owned_emp(), Some(item));
    ok(fs::remove_dir(&slot));
    ok(fs::rename(&retained, &slot));
    menu_key(&mut a, ui::Key::Buy);
    menu_key(&mut a, ui::Key::Confirm);
    let after = a
        .game
        .world
        .owned_emp()
        .unwrap_or_else(|| panic!("upgraded"));
    assert_eq!(after.item, item.item);
    assert_eq!(after.provenance, EmpProvenance::Paid);
    assert_eq!(after.grades.power, 1);
    assert_eq!(a.game.world.credits_tenths(), Some(9_970_000));
    assert_eq!(
        a.game.world.emp_snapshot().map(|s| s.bank_energy),
        Some(0.0)
    );
}
#[test]
fn emp_rows_are_current_offers_not_purchase_or_foreign_item_authority() {
    let d = TestDirectory::new();
    let mut a = app(&d, 19103);
    let before = ok(a.game.world.save_bytes());
    let rows = ui::emp::rows(&a.game.world);
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].can_buy);
    assert_eq!(rows[0].price_tenths, Some(30000));
    assert!(
        a.game
            .command(Action::Emp(EmpEquipmentAction::Purchase))
            .is_err()
    );
    assert_eq!(ok(a.game.world.save_bytes()), before);
    ok(a.game.xyzzy(XyzzyAction::Enter));
    let before = ok(a.game.world.save_bytes());
    assert!(
        ui::emp::action(
            &a.game.world,
            "emp-property",
            Some(u64::MAX),
            ui::EquipmentOperation::Store
        )
        .is_err()
    );
    let rows = ui::emp::rows(&a.game.world);
    assert!(
        rows.iter()
            .filter(|r| r.node != "emp-property")
            .all(|r| !r.can_buy && r.refusal.is_some())
    );
    assert_eq!(ok(a.game.world.save_bytes()), before);
}
#[test]
fn emp_input_is_physical_tick_bound_and_help_focus_pause_never_replay_a_held_key() {
    let d = TestDirectory::new();
    let mut a = app(&d, 19104);
    ok(a.game.xyzzy(XyzzyAction::Enter));
    ok(a.game.world.request_launch(a.game.world.snapshot().tick));
    for _ in 0..1800 {
        ok(a.game.world.step(FlightInput::default()));
    }
    let before = ok(a.game.world.save_bytes());
    assert_eq!(
        key(&mut a, KeyCode::KeyU, ElementState::Pressed),
        Some(KeyCode::KeyU)
    );
    assert_eq!(a.advance_frame(true, 0), FrameAdvance::Continue(None));
    assert_eq!(ok(a.game.world.save_bytes()), before);
    assert_eq!(a.advance_frame(true, 1), FrameAdvance::Continue(None));
    assert_eq!(a.game.world.emp_snapshot().map(|s| s.releases), Some(0));
    assert!(
        a.notice
            .text
            .as_deref()
            .is_some_and(|n| n.contains("EMP refused"))
    );
    assert_eq!(key(&mut a, KeyCode::KeyH, ElementState::Pressed), None);
    assert!(a.help.is_some());
    assert!(!a.keys.contains(&KeyCode::KeyU));
    let paused = ok(a.game.world.save_bytes());
    assert_eq!(a.advance_frame(true, 600), FrameAdvance::Continue(None));
    assert_eq!(ok(a.game.world.save_bytes()), paused);
    a.focus_changed(false);
    a.focus_changed(true);
    key(&mut a, KeyCode::Escape, ElementState::Pressed);
    assert!(a.help.is_none());
    assert!(a.keys.is_empty());
    let save = ok(a.game.world.save_bytes());
    let line = ui::emp::status(&a.game.world).unwrap_or_else(|| panic!("status"));
    assert!(line.contains("EMP"));
    assert_eq!(ok(a.game.world.save_bytes()), save);
    assert!(
        crate::help::Panel::flight(None, None)
            .lines
            .iter()
            .any(|l| l.contains("Hold U 10 seconds"))
    );
}
#[test]
fn emp_equipment_help_retains_station_navigation_and_bounded_modal_text() {
    let p =
        crate::help::Panel::station(ui::Screen::Equipment, Some("EMP power upgrade"), true, true);
    assert!(p.lines.iter().any(|l| l.contains("EMP slot")));
    assert!(p.lines.iter().any(|l| l.contains("5 Equipment")));
    assert!(p.lines.len() + 2 <= crate::help::MAX_LINES);
    assert!(p.lines.iter().all(|l| l.len() <= crate::help::MAX_COLUMNS));
    // Forged input cannot turn an unavailable grade into a paid receipt.
    let d = TestDirectory::new();
    let mut a = app(&d, 19105);
    ok(a.game.xyzzy(XyzzyAction::Enter));
    let item = a.game.world.owned_emp().unwrap_or_else(|| panic!("item"));
    let before = ok(a.game.world.save_bytes());
    assert!(
        a.game
            .command(Action::Emp(EmpEquipmentAction::Upgrade {
                item: item.item,
                axis: EmpAxis::Power
            }))
            .is_err()
    );
    assert_eq!(ok(a.game.world.save_bytes()), before);
}

fn yaw_to(w: &World, x: f64, z: f64) -> FlightInput {
    let s = w.snapshot();
    let [qx, qy, qz, qw] = s.orientation.map(f64::from);
    let yaw = (2.0 * (qx * qz + qw * qy)).atan2(1.0 - 2.0 * (qx * qx + qy * qy));
    let desired = (x - f64::from(s.position[0])).atan2(z - f64::from(s.position[2]));
    let delta = (desired - yaw).sin().atan2((desired - yaw).cos());
    FlightInput {
        yaw: (delta / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32,
        ..FlightInput::default()
    }
}
fn stop(w: &mut World) {
    for _ in 0..300 {
        if w.snapshot().speed == 0.0 {
            return;
        }
        ok(w.step(FlightInput {
            brake: true,
            ..FlightInput::default()
        }));
    }
    panic!("brake failed");
}
// Actual recent attack admission uses the same public physical producers as the core
// acceptance case. This is explicitly headless setup, never native piloting evidence.
fn recent_attacker(a: &mut App) -> meridian_core::traffic::NpcId {
    ok(a.game.xyzzy(XyzzyAction::Enter));
    let w = &mut a.game.world;
    ok(w.request_station_service(Command::BuyFood, w.snapshot().tick));
    ok(w.request_launch(w.snapshot().tick));
    ok(w.step(FlightInput::default()));
    stop(w);
    let z = f64::from(w.snapshot().position[2]);
    for _ in 0..180 {
        ok(w.step(yaw_to(w, -600.0, z)));
    }
    let mut reached = false;
    for _ in 0..30000 {
        if w.snapshot().position[0] <= -600.0 {
            reached = true;
            break;
        }
        let speed = w.snapshot().speed;
        ok(w.step(FlightInput {
            throttle: ((10.0 - speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
            ..FlightInput::default()
        }));
        assert_eq!(w.snapshot().motion_refusal, None);
    }
    assert!(reached);
    stop(w);
    for _ in 0..24000 {
        ok(w.step(FlightInput::default()));
        if let Some(hit) = w.traffic_snapshot().and_then(|t| t.last_player_hit)
            && hit.world_tick == w.snapshot().tick
        {
            return hit.attacker;
        }
    }
    panic!("no genuine attacker; never fabricate aggression");
}
#[test]
fn actual_recent_attacker_charge_uses_six_hundred_host_ticks_and_never_retries_while_held() {
    let d = TestDirectory::new();
    let mut a = app(&d, 12001);
    let target = recent_attacker(&mut a);
    assert_eq!(
        key(&mut a, KeyCode::KeyU, ElementState::Pressed),
        Some(KeyCode::KeyU)
    );
    let mut charging = false;
    for _ in 0..1200 {
        let actor = a
            .game
            .world
            .traffic_snapshot()
            .unwrap_or_else(|| panic!("traffic"))
            .actors
            .into_iter()
            .find(|n| n.id == target)
            .unwrap_or_else(|| panic!("target"));
        let input = yaw_to(&a.game.world, actor.pose.position.x, actor.pose.position.z);
        ok(a.game.world.step(input));
        advance(&mut a, 1);
        if a.game
            .world
            .emp_snapshot()
            .is_some_and(|s| s.charge_phase == Some(EmpChargePhase::Charging))
        {
            charging = true;
            break;
        }
    }
    assert!(
        charging,
        "actual adapter never started a real recent-attacker charge"
    );
    assert_eq!(a.game.world.emp_snapshot().map(|s| s.charge_ticks), Some(1));
    advance(&mut a, 598);
    let before = a.game.world.emp_snapshot().unwrap_or_else(|| panic!("EMP"));
    assert_eq!(before.charge_phase, Some(EmpChargePhase::Charging));
    assert_eq!(before.charge_ticks, 599);
    advance(&mut a, 1);
    let terminal = a.game.world.emp_snapshot().unwrap_or_else(|| panic!("EMP"));
    assert_eq!(terminal.charge_phase, Some(EmpChargePhase::Released));
    assert_eq!(terminal.charge_ticks, 600);
    // Geometry is rechecked after real target motion. Either a typed pulse or consumed
    // refusal is honest; the callback must not silently retry a refused terminal frame.
    assert_eq!(
        terminal.releases,
        u16::from(terminal.charge_release.is_some())
    );
    let hud = ui::emp::status(&a.game.world).unwrap_or_else(|| panic!("HUD"));
    assert!(hud.contains(if terminal.charge_release.is_some() {
        "PULSE RELEASED"
    } else {
        "REFUSED; RELEASE U"
    }));
    advance(&mut a, 10);
    assert_eq!(
        a.game.world.emp_snapshot().map(|s| s.releases),
        Some(terminal.releases)
    );
    ok(a.game.save());
    drop(a);
    let mut p = open(&d.0);
    ok(p.continue_saved());
    let mut cold = App::with_profile(p);
    assert!(cold.keys.is_empty());
    advance(&mut cold, 1);
    assert_eq!(
        cold.game.world.emp_snapshot().map(|s| s.releases),
        Some(terminal.releases)
    );
}

fn advance(app: &mut App, ticks: usize) {
    match app.advance_frame(true, ticks) {
        FrameAdvance::Continue(_) => {}
        FrameAdvance::Fatal(error) => panic!("actual frame failed: {error:?}"),
    }
}

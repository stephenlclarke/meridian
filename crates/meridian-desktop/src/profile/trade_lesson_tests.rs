//! Real Profile/Store boundaries. Positive receipts come only from public World commands.
use super::Profile;
use crate::session::Action;
use meridian_core::{
    FlightInput, NavigationTradeOutcome, World,
    navigation::{BASIC_SPOOL_TICKS, DockOrigin},
    station_services::CommodityId,
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|e| panic!("{e:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("missing retained state"))
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let nonce = ok(SystemTime::now().duration_since(UNIX_EPOCH)).as_nanos();
        let path =
            std::env::temp_dir().join(format!("meridian-trade-{}-{nonce}", std::process::id()));
        ok(fs::create_dir(&path));
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
fn bytes(p: &Profile) -> Vec<u8> {
    ok(p.world.save_bytes())
}
fn review(p: &mut Profile, a: Action) {
    action(p, a);
    for _ in 0..7 {
        ok(p.world.step(FlightInput::default()));
    }
    action(p, Action::Confirm);
}
fn planned(p: &mut Profile) {
    review(p, Action::ReviewTradeLesson);
    review(p, Action::PlanTradeLesson);
}
fn journey(p: &mut Profile) {
    action(p, Action::Launch);
    ok(p.world.step(FlightInput::default()));
    action(p, Action::Jump);
    for _ in 0..BASIC_SPOOL_TICKS {
        ok(p.world.step(FlightInput::default()));
    }
    for _ in 0..9000 {
        let state = p.world.snapshot();
        if state.mode == meridian_core::FlightMode::Docked {
            break;
        }
        let speed = if state.position[2] > 2500.0 {
            34.3
        } else {
            4.0
        };
        ok(p.world.step(FlightInput {
            throttle: ((speed - state.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
            ..FlightInput::default()
        }));
        assert_eq!(p.world.snapshot().motion_refusal, None);
    }
    assert_eq!(
        some(p.world.current_dock_context()).origin(),
        DockOrigin::PhysicalEntry
    );
}
fn failed_write(p: &mut Profile, d: &Directory, a: Action) {
    ok(p.save());
    let before = bytes(p);
    let slot = d.0.join("commander.meridian");
    let retained = d.0.join("retained-valid");
    ok(fs::rename(&slot, &retained));
    ok(fs::create_dir(&slot));
    ok(fs::write(slot.join("sentinel"), b"retain"));
    assert!(p.command(a).is_err());
    assert_eq!(bytes(p), before);
    assert_eq!(ok(fs::read(&retained)), before);
    assert_eq!(ok(fs::read(slot.join("sentinel"))), b"retain");
    ok(fs::remove_dir_all(&slot));
    ok(fs::rename(&retained, &slot));
}
fn cold(mut p: Profile, d: &Directory) -> Profile {
    ok(p.save());
    let expected = some(p.world.navigation_trade_snapshot());
    // Release the parent Store lease before the real cold process opens this slot.
    drop(p);
    let status = ok(Command::new(ok(std::env::current_exe()))
        .args([
            "--exact",
            "profile::trade_lesson_tests::trade_cold_child",
            "--nocapture",
        ])
        .env("MERIDIAN_TRADE_CHILD", &d.0)
        .status());
    assert!(status.success());
    let mut p = ok(Profile::open(d.0.clone()));
    ok(p.continue_saved());
    assert_eq!(some(p.world.navigation_trade_snapshot()), expected);
    p
}
#[test]
fn trade_cold_child() {
    let Some(path) = std::env::var_os("MERIDIAN_TRADE_CHILD") else {
        return;
    };
    let path = PathBuf::from(path);
    let original = ok(fs::read(path.join("commander.meridian")));
    assert_eq!(u16::from_le_bytes([original[8], original[9]]), 11);
    let expected = ok(World::from_save_bytes(&original));
    let mut p = ok(Profile::open(path));
    ok(p.continue_saved());
    assert_eq!(
        p.world.navigation_trade_snapshot(),
        expected.navigation_trade_snapshot()
    );
    assert_eq!(p.world.service_snapshot(), expected.service_snapshot());
    assert_eq!(p.world.credits_tenths(), expected.credits_tenths());
    assert_eq!(
        p.world.flight_reception_snapshot(),
        expected.flight_reception_snapshot()
    );
    assert_eq!(
        p.world.practice_ledger_snapshot(),
        expected.practice_ledger_snapshot()
    );
    assert!(p.command(Action::Confirm).is_err());
}
#[test]
fn trade_accept_plan_cancel_back_and_writer_errors_keep_the_original_file_and_live_state() {
    let d = Directory::new();
    let mut p = d.profile();
    for a in [
        Action::ReviewTradeLesson,
        Action::PlanTradeLesson,
        Action::CancelTradeLesson,
    ] {
        let before = bytes(&p);
        action(&mut p, a);
        action(&mut p, Action::Back);
        assert_eq!(bytes(&p), before);
        assert!(p.command(Action::Confirm).is_err());
        action(&mut p, a);
        failed_write(&mut p, &d, Action::Confirm);
        assert!(p.command(Action::Confirm).is_err());
        review(&mut p, a);
    }
    let s = some(p.world.navigation_trade_snapshot());
    assert_eq!(s.attempts.len(), 1);
    assert_eq!(
        s.attempts[0].outcome,
        Some(NavigationTradeOutcome::Abandoned)
    );
    assert_eq!(s.trace, None);
    assert_eq!(s.reserved_capacity_tonnes, 0);
}
#[test]
fn ordinary_buy_sale_and_fuel_errors_drop_prepared_market_account_and_lesson_together() {
    let d = Directory::new();
    let mut p = d.profile();
    planned(&mut p);
    failed_write(&mut p, &d, Action::BuySelected);
    action(&mut p, Action::BuySelected);
    assert_eq!(ok(fs::read(d.0.join("commander.meridian"))), bytes(&p));
    let held = some(p.world.navigation_trade_snapshot()).attempts[0].held_tonne;
    assert!(held.is_some());
    journey(&mut p);
    failed_write(&mut p, &d, Action::Refuel);
    action(&mut p, Action::Refuel);
    assert!(some(p.world.navigation_trade_snapshot()).attempts[0].fuel_paid_tenths > 0);
    failed_write(&mut p, &d, Action::SellSelected);
    action(&mut p, Action::SellSelected);
    let s = some(p.world.navigation_trade_snapshot());
    assert_eq!(some(s.attempts[0].sale).tonne, some(held));
    assert_eq!(ok(fs::read(d.0.join("commander.meridian"))), bytes(&p));
}
#[test]
fn actual_food_trip_cold_held_sold_complete_and_reception_error_keep_one_trace_with_real_loss() {
    let d = Directory::new();
    let mut p = d.profile();
    let initial = some(p.world.credits_tenths());
    planned(&mut p);
    action(&mut p, Action::BuySelected);
    p = cold(p, &d);
    let held = some(p.world.navigation_trade_snapshot()).attempts[0].held_tonne;
    assert!(p.command(Action::ReviewTradeLesson).is_err());
    journey(&mut p);
    action(&mut p, Action::Refuel);
    action(&mut p, Action::SellSelected);
    p = cold(p, &d);
    assert_eq!(
        some(some(p.world.navigation_trade_snapshot()).attempts[0].sale).tonne,
        some(held)
    );
    journey(&mut p);
    let cash = p.world.credits_tenths();
    action(&mut p, Action::ReviewTradeLesson);
    failed_write(&mut p, &d, Action::Confirm);
    assert_eq!(some(p.world.navigation_trade_snapshot()).trace, None);
    review(&mut p, Action::ReviewTradeLesson);
    assert_eq!(p.world.credits_tenths(), cash);
    let complete = some(p.world.navigation_trade_snapshot());
    let trace = some(complete.trace);
    assert_eq!(
        complete.attempts[0].outcome,
        Some(NavigationTradeOutcome::Complete)
    );
    assert_eq!(some(p.world.credits_tenths()) as i64 - initial as i64, -98);
    assert_eq!(complete.attempts[0].cash_difference_tenths, Some(-98));
    p = cold(p, &d);
    planned(&mut p);
    review(&mut p, Action::CancelTradeLesson);
    assert_eq!(some(p.world.navigation_trade_snapshot()).trace, Some(trace));
}
#[test]
fn selected_commodity_plan_and_stale_reviews_are_visible_and_never_silently_repaired() {
    let d = Directory::new();
    let mut p = d.profile();
    review(&mut p, Action::ReviewTradeLesson);
    action(&mut p, Action::NextCommodity);
    action(&mut p, Action::PlanTradeLesson);
    let selected = some(p.session.pending_trade).plan();
    assert_eq!(some(selected).commodity, CommodityId::Furs);
    action(&mut p, Action::Back);
    action(&mut p, Action::PreviousCommodity);
    action(&mut p, Action::PlanTradeLesson);
    let tick = p.world.snapshot().tick;
    ok(p.world
        .request_flight_view(meridian_core::FlightView::Rear, tick));
    let before = bytes(&p);
    assert!(p.command(Action::Confirm).is_err());
    assert_eq!(bytes(&p), before);
    action(&mut p, Action::PlanTradeLesson);
    ok(p.continue_saved());
    let before = bytes(&p);
    let saved = ok(fs::read(d.0.join("commander.meridian")));
    let error = p
        .command(Action::Confirm)
        .err()
        .unwrap_or_else(|| panic!("invalidated plan committed"));
    assert!(error.contains("trade: 4/5/6"));
    assert!(error.contains("freight: R"));
    assert_eq!(bytes(&p), before);
    assert_eq!(ok(fs::read(d.0.join("commander.meridian"))), saved);
}
#[test]
fn trade_reviews_and_details_fit_existing_hud_budget_with_selected_terms_and_costs() {
    let d = Directory::new();
    let mut p = d.profile();
    fn visible(p: &Profile) {
        let lines = p.lines(true, None);
        assert!(lines.len() <= crate::hud::line_budget(640, 360));
        assert!(lines.iter().all(|l| l.len() <= crate::hud::MAX_COLUMNS));
    }
    for a in [Action::TradeLessonDetails, Action::ReviewTradeLesson] {
        action(&mut p, a);
        visible(&p);
    }
    action(&mut p, Action::Confirm);
    action(&mut p, Action::PlanTradeLesson);
    visible(&p);
    let text = p.lines(true, None).join("\n");
    assert!(text.contains("Food"));
    assert!(text.contains("DESTINATION"));
    assert!(text.contains("ENTER CONFIRM AND SAVE"));
    action(&mut p, Action::Confirm);
    action(&mut p, Action::TradeLessonDetails);
    visible(&p);
    assert!(p.lines(true, None).join("\n").contains("RESERVED HOLD 1T"));
    action(&mut p, Action::BuySelected);
    action(&mut p, Action::TradeLessonDetails);
    visible(&p);
    assert!(p.lines(true, None).join("\n").contains("HELD true"));
}

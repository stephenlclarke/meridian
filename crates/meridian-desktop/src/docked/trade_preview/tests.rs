use super::*;
use crate::profile::tests::{TestDirectory, open};
use meridian_core::{navigation::RouteKind, station_services::Command};
fn ok<T, E: core::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}
fn some<T>(v: Option<T>) -> T {
    v.unwrap_or_else(|| panic!("missing information/frame"))
}
fn paint(menu: &mut Docked, p: &Profile, viewport: ui::Viewport) -> ui::Frame {
    let f = ok(menu.build_frame(&p.world, viewport, 0.0));
    menu.presented(f.clone());
    f
}
fn input(menu: &mut Docked, p: &mut Profile, key: ui::Key, v: ui::Viewport) {
    ok(menu.input(p, ui::Input::Key(key)));
    paint(menu, p, v);
}
fn start() -> (TestDirectory, Profile, Docked, ui::Viewport) {
    let d = TestDirectory::new();
    let mut p = open(&d.0);
    assert!(ok(p.new_commander(29201)));
    let r = ok(p
        .world
        .review_navigation_route(SystemId::RENAS, RouteKind::FewestJumps, 0));
    ok(p.install_navigation_route(r));
    let mut menu = ok(Docked::new(&p.world));
    ok(menu.open(&p.world, ui::Screen::Galaxy));
    let v = some(ui::Viewport::new(320.0, 240.0));
    paint(&mut menu, &p, v);
    input(&mut menu, &mut p, ui::Key::Search, v);
    ok(menu.input(&mut p, ui::Input::Text("Lareria".into())));
    paint(&mut menu, &p, v);
    input(&mut menu, &mut p, ui::Key::Confirm, v);
    (d, p, menu, v)
}
#[test]
fn actual_chart_g_information_pages_are_readable_at_minimum_size_and_enter_never_changes_an_installed_route_or_save()
 {
    let (d, mut p, mut menu, v) = start();
    let before = ok(p.world.save_bytes());
    let route = p.world.navigation_route_snapshot();
    input(&mut menu, &mut p, ui::Key::Buy, v);
    let count = match &some(menu.pending.as_ref()).intent {
        Intent::TradeInformation(info) => info.pages.len(),
        _ => panic!("actual G information"),
    };
    assert!(count > 3);
    let mut texts = String::new();
    for page in 0..count {
        let frame = paint(&mut menu, &p, v);
        let review = &some(menu.pending.as_ref()).view;
        for row in &review.details {
            assert!(
                frame.labels.iter().any(|label| label.text == *row),
                "page{page} row{row}"
            );
            texts.push_str(row);
            texts.push(' ');
        }
        assert!(
            frame
                .labels
                .iter()
                .any(|label| label.text.contains("PgUp/PgDn"))
        );
        input(&mut menu, &mut p, ui::Key::PageNext, v);
        assert_eq!(ok(p.world.save_bytes()), before);
    }
    assert!(texts.contains("INFORMATION ONLY"));
    assert!(texts.contains("not guaranteed"));
    assert!(texts.contains("1 kg"));
    assert!(texts.contains("3 g"));
    assert!(texts.contains("261.8"));
    input(&mut menu, &mut p, ui::Key::Confirm, v);
    assert!(menu.pending.is_none());
    assert_eq!(p.world.navigation_route_snapshot(), route);
    assert_eq!(ok(p.world.save_bytes()), before);
    assert_eq!(ok(std::fs::read(d.0.join("commander.meridian"))), before);
}
#[test]
fn actual_market_purchase_invalidates_old_information_and_reopening_uses_current_cash_stock_and_cargo()
 {
    let (_d, mut p, mut menu, v) = start();
    input(&mut menu, &mut p, ui::Key::Buy, v);
    ok(p.command(Action::StationService(Command::BuyCommodity(
        CommodityId::GemStones,
    ))));
    let before = ok(p.world.save_bytes());
    input(&mut menu, &mut p, ui::Key::Confirm, v);
    assert!(menu.pending.is_none());
    assert!(menu.feedback.contains("Estimates changed"));
    assert_eq!(ok(p.world.save_bytes()), before);
    input(&mut menu, &mut p, ui::Key::Buy, v);
    let Intent::TradeInformation(info) = &some(menu.pending.as_ref()).intent else {
        panic!("fresh information")
    };
    assert_eq!(info.preview.cash_tenths, 6306);
    assert_eq!(info.preview.quotes[0][15].stock, 105);
    assert_eq!(info.preview.direct_profits[0].estimated_delta_tenths, 2688);
}
#[test]
fn information_reflows_without_replanning_and_blocks_route_or_purchase_keys_while_help_stays_bounded()
 {
    let (_d, mut p, mut menu, v) = start();
    input(&mut menu, &mut p, ui::Key::Buy, v);
    let before = ok(p.world.save_bytes());
    let snapshot = match &some(menu.pending.as_ref()).intent {
        Intent::TradeInformation(info) => info.preview.clone(),
        _ => panic!("info"),
    };
    let wide = some(ui::Viewport::new(1280.0, 720.0));
    paint(&mut menu, &p, wide);
    for key in [
        ui::Key::Buy,
        ui::Key::Route,
        ui::Key::Options,
        ui::Key::Compare,
    ] {
        input(&mut menu, &mut p, key, wide);
        let Intent::TradeInformation(info) = &some(menu.pending.as_ref()).intent else {
            panic!("info retained")
        };
        assert_eq!(info.preview, snapshot);
        assert_eq!(ok(p.world.save_bytes()), before);
    }
    for screen in [ui::Screen::Galaxy, ui::Screen::System] {
        let help = crate::help::Panel::station(screen, Some("Trade info"), true, true);
        assert!(help.lines.len() + 2 <= crate::help::MAX_LINES);
        assert!(
            help.lines
                .iter()
                .all(|line| line.len() <= crate::help::MAX_COLUMNS)
        );
        assert!(
            help.lines
                .iter()
                .any(|line| line.contains("G Trade/profit"))
        );
    }
    input(&mut menu, &mut p, ui::Key::Back, wide);
    assert!(menu.pending.is_none());
    assert_eq!(ok(p.world.save_bytes()), before);
}

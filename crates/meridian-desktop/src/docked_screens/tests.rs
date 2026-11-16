use super::*;
use meridian_core::{
    World,
    navigation::{OriginalNavigation, SystemId},
    station_services::CommodityId,
};

fn catalogue() -> Catalogue {
    Catalogue::bundled().unwrap_or_else(|e| panic!("{e}"))
}
fn world() -> World {
    World::new_commander(94001).unwrap_or_else(|e| panic!("{e}"))
}
fn context<'a>(world: &'a World, catalogue: &'a Catalogue) -> Context<'a> {
    Context {
        world,
        catalogue,
        stamp: Stamp {
            session: 1,
            revision: 1,
        },
        current_hull: "SH025",
        paused: false,
        current_reach_ly: Some(9.0),
        full_reach_ly: Some(9.0),
        route_targets: &[SystemId::AVENOR, SystemId::RENAS],
        models: &[],
        planet_previews: false,
        navigation_bodies: &[],
        exchanges: &[],
        equipment: None,
        review: None,
        bindings: &[],
    }
}
fn view() -> Viewport {
    Viewport::new(1280.0, 800.0).unwrap_or_else(|| panic!("valid viewport"))
}
fn key(ui: &mut DockedScreens, c: &Context<'_>, key: Key) -> Outcome {
    let frame = ui.frame(c, view());
    ui.handle(Input::Key(key), &frame, c)
}
fn point(frame: &Frame, control: &Control) -> [f64; 2] {
    frame
        .hits
        .iter()
        .find(|h| &h.control == control)
        .map(|h| h.bounds.centre())
        .unwrap_or_else(|| panic!("missing control {control:?}"))
}
fn click(ui: &mut DockedScreens, c: &Context<'_>, control: Control) -> Outcome {
    let frame = ui.frame(c, view());
    let p = point(&frame, &control);
    ui.handle(Input::Click(p), &frame, c)
}
fn equipment() -> EquipmentPanel {
    EquipmentPanel {
        selected_slot: "shield".into(),
        slots: vec![
            EquipmentSlot {
                id: "shield".into(),
                label: "Shield".into(),
                fitted: "Basic".into(),
                available: true,
            },
            EquipmentSlot {
                id: "energy".into(),
                label: "Energy".into(),
                fitted: "Basic".into(),
                available: true,
            },
        ],
        rows: vec![EquipmentRow {
            node: "fixture.shield".into(),
            name: "Shield projection fixture".into(),
            group: "Shield Modules".into(),
            price_tenths: Some(12345),
            quantity: OfferQuantity::NotTracked,
            instance: Some(77),
            owned: 2,
            fitted: 1,
            compatibility: "Fits selected shield slot".into(),
            refusal: None,
            readout: vec!["Offer % Basic: cap F/R 200%/200%; regen F/R 100%/100%".into()],
            options: vec![EquipmentOption {
                id: "fixture.capacity".into(),
                label: "Capacity".into(),
                value: "200%".into(),
                price_tenths: Some(3210),
                refusal: None,
            }],
            can_buy: true,
            can_fit: true,
            can_store: true,
        }],
        equipment_units: [5, 24],
        utility_mounts: [1, 3],
        assigned_crew: Some(1),
        required_crew: Some(1),
        feedback: "Fitted the same owned module".into(),
    }
}
#[test]
fn exact_catalogue_taxonomy_is_162_ships_not_171_ships() {
    let c = catalogue();
    assert_eq!(c.worlds.len(), 2048);
    assert_eq!(c.objects.len(), 171);
    assert_eq!(c.search_ships("").len(), 162);
    assert_eq!(c.object("SH028").map(|o| o.kind), Some("station"));
    assert_eq!(c.object("SH025").map(|o| o.name), Some("Peregrine Mk V"));
    assert!(c.search_ships("SH028").is_empty());
    assert!(
        c.world(SystemId {
            galaxy: 0,
            system: 256
        })
        .is_none()
    );
    assert!(
        c.world(SystemId {
            galaxy: 8,
            system: 0
        })
        .is_none()
    );
}
#[test]
fn every_imported_g01_coordinate_and_name_matches_real_core_content() {
    let c = catalogue();
    let nav = OriginalNavigation::bundled().unwrap_or_else(|e| panic!("{e}"));
    for system in 0..256 {
        let id = SystemId { galaxy: 0, system };
        let actual = nav.system(id).unwrap_or_else(|e| panic!("{e}"));
        let row = c.world(id).unwrap_or_else(|| panic!("missing world"));
        assert_eq!(row.name, actual.name);
        assert_eq!(row.position, [actual.x_ly, actual.y_ly]);
    }
    assert!(
        c.distance(
            SystemId::AVENOR,
            SystemId {
                galaxy: 1,
                system: 7
            }
        )
        .is_none()
    );
}
#[test]
fn malformed_or_partial_catalogues_refuse_instead_of_fabricating_data() {
    assert!(Catalogue::decode("0\t7\tAvenor", "", " ").is_err());
    assert!(
        Catalogue::decode(
            "0\t0\tTest\tNaN\t1\t0\tA\t0\tA\t1\t1\t1\t1\tfalse\tx\ty",
            "",
            ""
        )
        .is_err()
    );
}
#[test]
fn search_orders_prefixes_and_retains_source_identities() {
    let c = catalogue();
    let results = c.search_worlds(0, "ave");
    assert!(!results.is_empty());
    assert!(results[0].name.to_lowercase().starts_with("ave"));
    assert_eq!(c.search_ships("sh025")[0].id, "SH025");
    assert!(c.search_ships("not-a-hull").is_empty());
}
#[test]
fn all_five_tabs_are_navigable_by_pointer_and_semantic_keyboard() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    for screen in Screen::ALL {
        assert_eq!(
            click(&mut ui, &ctx, Control::Open(screen)),
            Outcome::Updated
        );
        assert_eq!(ui.screen(), screen);
    }
    ui.open(Screen::Galaxy);
    for expected in [
        Screen::System,
        Screen::Showroom,
        Screen::Trading,
        Screen::Equipment,
        Screen::Galaxy,
    ] {
        assert_eq!(key(&mut ui, &ctx, Key::NextTab), Outcome::Updated);
        assert_eq!(ui.screen(), expected);
    }
    assert_eq!(
        key(&mut ui, &ctx, Key::Back),
        Outcome::Request(Request::Close)
    );
}
#[test]
fn actual_market_bid_ask_stock_hold_native_units_and_legal_colours_are_projected() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Trading);
    let buy = ui.frame(&ctx, view());
    let q = w
        .commodity_quote(CommodityId::Food)
        .unwrap_or_else(|| panic!("quote"));
    assert!(
        buy.labels
            .iter()
            .any(|l| l.text == super::money(u64::from(q.ask_tenths)))
    );
    assert_eq!(
        click(&mut ui, &ctx, Control::TradeMode(true)),
        Outcome::Updated
    );
    assert_eq!(key(&mut ui, &ctx, Key::BuyMode), Outcome::Updated);
    assert!(!ui.sell);
    assert_eq!(key(&mut ui, &ctx, Key::SellMode), Outcome::Updated);
    let sell = ui.frame(&ctx, view());
    assert!(
        sell.labels
            .iter()
            .any(|l| l.text == super::money(u64::from(q.bid_tenths)))
    );
    assert!(sell.labels.iter().any(|l| l.text == "kg"));
    assert!(sell.labels.iter().any(|l| l.text == "g"));
    assert!(
        sell.labels
            .iter()
            .any(|l| l.text.contains("Slaves") && l.ink == Ink::Orange)
    );
}
#[test]
fn market_keyboard_pointer_equivalence_outputs_review_only_and_preserves_world() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let before = w.save_bytes().unwrap_or_else(|e| panic!("{e}"));
    let mut a = DockedScreens::new(SystemId::AVENOR, "SH025");
    a.open(Screen::Trading);
    let mut b = DockedScreens::new(SystemId::AVENOR, "SH025");
    b.open(Screen::Trading);
    let expected = Outcome::Request(Request::ReviewCommodity {
        commodity: CommodityId::Food,
        buy: true,
        quantity: 1,
    });
    assert_eq!(key(&mut a, &ctx, Key::Confirm), expected);
    assert_eq!(click(&mut b, &ctx, Control::Key(Key::Confirm)), expected);
    assert_eq!(w.save_bytes().unwrap_or_else(|e| panic!("{e}")), before);
}
#[test]
fn selection_changes_do_not_execute_the_selected_commodity() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Trading);
    assert_eq!(
        click(&mut ui, &ctx, Control::SelectCommodity(CommodityId::Gold)),
        Outcome::Updated
    );
    assert_eq!(
        key(&mut ui, &ctx, Key::Sell),
        Outcome::Request(Request::ReviewCommodity {
            commodity: CommodityId::Gold,
            buy: false,
            quantity: 1,
        })
    );
}
#[test]
fn stale_painted_state_and_changed_session_refuse_without_actions() {
    let c = catalogue();
    let w = world();
    let mut ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Trading);
    let frame = ui.frame(&ctx, view());
    ctx.stamp.revision += 1;
    assert!(matches!(
        ui.handle(Input::Key(Key::Confirm), &frame, &ctx),
        Outcome::Refused(_)
    ));
    ctx.stamp = frame.stamp;
    ui.open(Screen::Equipment);
    assert!(matches!(
        ui.handle(
            Input::Click(point(&frame, &Control::Key(Key::Confirm))),
            &frame,
            &ctx
        ),
        Outcome::Refused(_)
    ));
}
#[test]
fn browsing_other_galaxies_never_authorises_travel_or_moves_commander() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let before = w.snapshot();
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    assert_eq!(click(&mut ui, &ctx, Control::Galaxy(1)), Outcome::Updated);
    assert_eq!(ui.selected_system().galaxy, 1);
    assert!(ui.frame(&ctx, view()).range_rings.is_empty());
    assert!(matches!(
        key(&mut ui, &ctx, Key::Route),
        Outcome::Refused(_)
    ));
    assert_eq!(w.snapshot(), before);
}
#[test]
fn chart_pointer_and_keyboard_select_the_same_source_world() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    let f = ui.frame(&ctx, view());
    let target = f
        .markers
        .iter()
        .find(|m| m.system == SystemId::RENAS)
        .copied()
        .unwrap_or_else(|| panic!("Renas visible"));
    assert_eq!(
        ui.handle(Input::Click(target.point), &f, &ctx),
        Outcome::Updated
    );
    assert_eq!(ui.selected_system(), SystemId::RENAS);
    assert_eq!(
        key(&mut ui, &ctx, Key::Route),
        Outcome::Request(Request::ReviewRoute(SystemId::RENAS))
    );
}
#[test]
fn search_consumes_text_and_navigation_keys_without_firing_other_actions() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    assert_eq!(key(&mut ui, &ctx, Key::Search), Outcome::Updated);
    let f = ui.frame(&ctx, view());
    assert_eq!(
        ui.handle(Input::Text("Renas".into()), &f, &ctx),
        Outcome::Updated
    );
    assert_eq!(key(&mut ui, &ctx, Key::Confirm), Outcome::Updated);
    assert_eq!(ui.selected_system(), SystemId::RENAS);
    assert_eq!(key(&mut ui, &ctx, Key::Search), Outcome::Updated);
    assert_eq!(key(&mut ui, &ctx, Key::Buy), Outcome::Ignored);
    assert_eq!(key(&mut ui, &ctx, Key::Back), Outcome::Updated);
    assert!(!ui.frame(&ctx, view()).search_active);
}
#[test]
fn model_previews_require_real_asset_handles_and_share_the_same_orientation() {
    let c = catalogue();
    let w = world();
    let models = [AvailableModel {
        object: "SH025",
        geometry_key: "assets/legacy/sh025.mesh",
    }];
    let mut ctx = context(&w, &c);
    ctx.models = &models;
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Showroom);
    let before = ui.frame(&ctx, view());
    assert_eq!(before.models.len(), 2);
    assert_eq!(before.models[0].orientation, before.models[1].orientation);
    let rect = before.model_bounds.unwrap_or_else(|| panic!("model"));
    let from = rect.centre();
    let to = [from[0] + 10.0, from[1] + 10.0];
    assert_eq!(
        ui.handle(
            Input::Drag {
                from,
                to,
                roll: false
            },
            &before,
            &ctx
        ),
        Outcome::Updated
    );
    let after = ui.frame(&ctx, view());
    assert_eq!(after.models[0].orientation, after.models[1].orientation);
    assert_ne!(after.models[0].orientation, before.models[0].orientation);
    ctx.models = &[];
    assert!(ui.frame(&ctx, view()).models.is_empty());
}
#[test]
fn unsupported_exchange_cannot_be_inferred_from_catalogue_flyable_status() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Showroom);
    assert!(matches!(
        key(&mut ui, &ctx, Key::Exchange),
        Outcome::Refused(_)
    ));
    assert!(c.object("SH025").is_some_and(|o| o.player_flyable));
}
#[test]
fn modular_equipment_displays_authoritative_offer_owned_fit_readout_and_feedback() {
    let c = catalogue();
    let w = world();
    let panel = equipment();
    let mut ctx = context(&w, &c);
    ctx.equipment = Some(&panel);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Equipment);
    let f = ui.frame(
        &ctx,
        Viewport::new(1440.0, 1000.0).unwrap_or_else(|| panic!("viewport")),
    );
    for expected in [
        "1234.5 CR",
        "Owned 2 | Fitted 1",
        "Equipment units 5/24",
        "Fitted the same owned module",
        "Offer % Basic",
    ] {
        assert!(
            f.labels.iter().any(|l| l.text.contains(expected)),
            "missing {expected}"
        );
    }
    assert!(!f.labels.iter().any(|l| l.text.starts_with("Available:")));
}
#[test]
fn modular_buy_fit_store_are_equivalent_review_intents_not_ui_owned_transactions() {
    let c = catalogue();
    let w = world();
    let panel = equipment();
    let mut ctx = context(&w, &c);
    ctx.equipment = Some(&panel);
    let before = w.save_bytes().unwrap_or_else(|e| panic!("{e}"));
    for (keycode, operation) in [
        (Key::Confirm, EquipmentOperation::Buy),
        (Key::Buy, EquipmentOperation::Buy),
        (Key::Fit, EquipmentOperation::Fit),
        (Key::Store, EquipmentOperation::Store),
    ] {
        let mut a = DockedScreens::new(SystemId::AVENOR, "SH025");
        a.open(Screen::Equipment);
        let mut b = DockedScreens::new(SystemId::AVENOR, "SH025");
        b.open(Screen::Equipment);
        let expected = Outcome::Request(Request::ReviewEquipment {
            node: panel.rows[0].node.clone(),
            instance: if operation == EquipmentOperation::Buy {
                None
            } else {
                Some(77)
            },
            slot: "shield".into(),
            operation,
            option: None,
        });
        assert_eq!(key(&mut a, &ctx, keycode), expected);
        assert_eq!(
            click(
                &mut b,
                &ctx,
                Control::Key(if keycode == Key::Buy {
                    Key::Confirm
                } else {
                    keycode
                })
            ),
            expected
        );
    }
    assert_eq!(w.save_bytes().unwrap_or_else(|e| panic!("{e}")), before);
}
#[test]
fn module_options_preserve_instance_and_slot_identity() {
    let c = catalogue();
    let w = world();
    let panel = equipment();
    let mut ctx = context(&w, &c);
    ctx.equipment = Some(&panel);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Equipment);
    key(&mut ui, &ctx, Key::Options);
    assert_eq!(
        key(&mut ui, &ctx, Key::Confirm),
        Outcome::Request(Request::ReviewEquipment {
            node: "fixture.shield".into(),
            instance: Some(77),
            slot: "shield".into(),
            operation: EquipmentOperation::Configure,
            option: Some("fixture.capacity".into())
        })
    );
    assert_eq!(key(&mut ui, &ctx, Key::Back), Outcome::Updated);
    assert!(!ui.options);
}
#[test]
fn incompatible_unowned_and_paused_equipment_actions_refuse() {
    let c = catalogue();
    let w = world();
    let mut panel = equipment();
    panel.rows[0].can_fit = false;
    panel.rows[0].can_store = false;
    panel.rows[0].instance = None;
    let mut ctx = context(&w, &c);
    ctx.equipment = Some(&panel);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Equipment);
    assert!(matches!(key(&mut ui, &ctx, Key::Fit), Outcome::Refused(_)));
    assert!(matches!(
        key(&mut ui, &ctx, Key::Store),
        Outcome::Refused(_)
    ));
    ctx.paused = true;
    assert!(matches!(
        key(&mut ui, &ctx, Key::Confirm),
        Outcome::Refused(_)
    ));
}
#[test]
fn slot_click_and_next_slot_request_real_projection_without_moving_equipment() {
    let c = catalogue();
    let w = world();
    let panel = equipment();
    let mut ctx = context(&w, &c);
    ctx.equipment = Some(&panel);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Equipment);
    let expected = Outcome::Request(Request::InspectEquipmentSlot("energy".into()));
    assert_eq!(key(&mut ui, &ctx, Key::NextSlot), expected);
    assert_eq!(click(&mut ui, &ctx, Control::SelectSlot(1)), expected);
}
#[test]
fn modal_blocks_clickthrough_and_confirms_only_the_supplied_review_token() {
    let c = catalogue();
    let w = world();
    let review = Review {
        token: 655,
        title: "Fit owned module".into(),
        details: vec!["No charge".into()],
        can_confirm: true,
        refusal: None,
    };
    let mut ctx = context(&w, &c);
    ctx.review = Some(&review);
    let mut a = DockedScreens::new(SystemId::AVENOR, "SH025");
    let mut b = DockedScreens::new(SystemId::AVENOR, "SH025");
    assert_eq!(
        key(&mut a, &ctx, Key::Confirm),
        Outcome::Request(Request::ConfirmReview(655))
    );
    assert_eq!(
        click(&mut b, &ctx, Control::ReviewConfirm),
        Outcome::Request(Request::ConfirmReview(655))
    );
    let f = a.frame(&ctx, view());
    assert_eq!(f.hits.len(), 2);
    assert!(f.models.is_empty());
    assert_eq!(key(&mut a, &ctx, Key::NextTab), Outcome::Ignored);
    assert_eq!(
        key(&mut a, &ctx, Key::Back),
        Outcome::Request(Request::CancelReview(655))
    );
    assert_eq!(
        click(&mut b, &ctx, Control::ReviewCancel),
        Outcome::Request(Request::CancelReview(655))
    );
    ctx.paused = true;
    assert!(matches!(
        key(&mut a, &ctx, Key::Confirm),
        Outcome::Refused(_)
    ));
    assert!(matches!(
        click(&mut b, &ctx, Control::ReviewConfirm),
        Outcome::Refused(_)
    ));
}
#[test]
fn review_controls_without_a_current_review_never_issue_requests() {
    let c = catalogue();
    let w = world();
    let mut ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    let frame = ui.frame(&ctx, view());
    assert!(
        frame
            .hits
            .iter()
            .all(|hit| !matches!(hit.control, Control::ReviewConfirm | Control::ReviewCancel))
    );
    let review = Review {
        token: 655,
        title: "Expired review fixture".into(),
        details: vec![],
        can_confirm: true,
        refusal: None,
    };
    ctx.review = Some(&review);
    let painted = ui.frame(&ctx, view());
    // Retain a painted modal after its authority disappears, even without a stamp change.
    ctx.review = None;
    for control in [Control::ReviewConfirm, Control::ReviewCancel] {
        let input = Input::Click(point(&painted, &control));
        assert_eq!(ui.handle(input, &painted, &ctx), Outcome::Ignored);
    }
}
#[test]
fn missing_equipment_provider_is_visible_and_never_a_fake_shop() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Equipment);
    let f = ui.frame(&ctx, view());
    assert!(
        f.labels
            .iter()
            .any(|l| l.text.contains("Equipment service unavailable"))
    );
    assert!(matches!(
        key(&mut ui, &ctx, Key::Confirm),
        Outcome::Refused(_)
    ));
}
#[test]
fn real_body_identity_and_physical_position_flow_to_navigation_without_an_ore_store() {
    let c = catalogue();
    let w = world();
    let bodies = [NavigationBody {
        id: NavigationBodyId {
            system: SystemId::AVENOR,
            ordinal: 41,
        },
        name: "Avenor belt provider fixture".into(),
        position_world: [12.0, 345.0, -98.0],
        distance_world: Some(390.0),
        can_target: true,
        refusal: None,
    }];
    let mut ctx = context(&w, &c);
    ctx.navigation_bodies = &bodies;
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::System);
    assert_eq!(
        ui.frame(&ctx, view()).navigation_cues[0].position_world,
        bodies[0].position_world
    );
    assert_eq!(
        click(&mut ui, &ctx, Control::SelectBody(bodies[0].id)),
        Outcome::Request(Request::ReviewBodyRoute(bodies[0].id))
    );
    assert_eq!(
        key(&mut ui, &ctx, Key::BodyRoute),
        Outcome::Request(Request::ReviewBodyRoute(bodies[0].id))
    );
    ctx.navigation_bodies = &[];
    assert!(ui.frame(&ctx, view()).navigation_cues.is_empty());
    assert!(
        !ui.frame(&ctx, view())
            .hits
            .iter()
            .any(|h| matches!(h.control, Control::SelectBody(_)))
    );
}
#[test]
fn invalid_pointer_and_drag_values_do_not_change_orientation_or_issue_commands() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Showroom);
    let before = ui.orientation;
    let frame = ui.frame(&ctx, view());
    assert_eq!(
        ui.handle(
            Input::Drag {
                from: [f64::NAN, 0.0],
                to: [1.0, 1.0],
                roll: false
            },
            &frame,
            &ctx
        ),
        Outcome::Ignored
    );
    assert_eq!(ui.orientation, before);
    assert_eq!(
        ui.handle(Input::PointerMoved([f64::INFINITY, 0.0]), &frame, &ctx),
        Outcome::Updated
    );
    assert!(ui.hovered.is_none());
    ui.advance(f64::INFINITY);
    assert_eq!(ui.orientation, before);
}
#[test]
fn all_screen_geometry_is_finite_and_controls_stay_inside_supported_viewports() {
    let c = catalogue();
    let w = world();
    let panel = equipment();
    let mut ctx = context(&w, &c);
    ctx.equipment = Some(&panel);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    for (width, height) in [(640.0, 360.0), (1000.0, 700.0), (1440.0, 1000.0)] {
        let viewport = Viewport::new(width, height).unwrap_or_else(|| panic!("viewport"));
        for screen in Screen::ALL {
            ui.open(screen);
            let f = ui.frame(&ctx, viewport);
            for label in &f.labels {
                assert!(label.at.into_iter().all(f64::is_finite));
                assert!(label.clip.contains(label.at));
            }
            for (rect, _) in f.actionable_regions() {
                assert!(
                    rect.x >= 0.0
                        && rect.y >= 0.0
                        && rect.x + rect.width <= width + 0.001
                        && rect.y + rect.height <= height + 0.001,
                    "{screen:?}: {rect:?}"
                );
            }
        }
    }
    assert!(Viewport::new(f64::NAN, 800.0).is_none());
    assert!(Viewport::new(0.0, 0.0).is_none());
}
#[test]
fn chart_markers_carry_plot_clipping_for_both_chart_scales() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    for (width, height) in [(640.0, 360.0), (1000.0, 700.0), (1440.0, 1000.0)] {
        let viewport = Viewport::new(width, height).unwrap_or_else(|| panic!("viewport"));
        for local in [false, true] {
            ui.local = local;
            let nav = w
                .snapshot()
                .navigation
                .unwrap_or_else(|| panic!("navigation"));
            let origin = c.world(nav.system).unwrap_or_else(|| panic!("world"));
            let layout = ChartLayout::new(
                viewport,
                &c,
                ui.galaxy,
                local.then_some((origin, nav.fuel_units)),
            );
            let frame = ui.frame(&ctx, viewport);
            assert!(!frame.markers.is_empty());
            for marker in &frame.markers {
                assert_eq!(marker.clip, layout.plot);
                assert!(marker.clip.contains(marker.point));
            }
        }
    }
}
#[test]
fn visible_shortcuts_are_from_the_supplied_binding_map() {
    let c = catalogue();
    let w = world();
    let hints = [
        BindingHint {
            key: Key::Confirm,
            label: "Enter: review",
        },
        BindingHint {
            key: Key::Back,
            label: "Esc: back",
        },
    ];
    let mut ctx = context(&w, &c);
    ctx.bindings = &hints;
    let ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    let f = ui.frame(&ctx, view());
    assert!(
        f.labels
            .iter()
            .any(|l| l.text.contains("Enter: review | Esc: back"))
    );
}

#[test]
fn focus_loss_stops_text_and_rotation_without_changing_the_commander() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Showroom);
    key(&mut ui, &ctx, Key::Search);
    let before = w.snapshot();
    let frame = ui.frame(&ctx, view());
    assert_eq!(ui.handle(Input::FocusLost, &frame, &ctx), Outcome::Updated);
    assert!(!ui.searching);
    assert!(ui.rotation_paused);
    assert_eq!(w.snapshot(), before);
}
#[test]
fn planet_preview_is_a_provider_request_for_the_selected_real_world_only() {
    let c = catalogue();
    let w = world();
    let mut ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::System);
    assert!(ui.frame(&ctx, view()).planets.is_empty());
    ctx.planet_previews = true;
    let frame = ui.frame(&ctx, view());
    assert_eq!(frame.planets.len(), 1);
    assert_eq!(frame.planets[0].system, SystemId::AVENOR);
}

#[test]
fn quantity_controls_match_keyboard_and_pointer_without_trading() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let before = w.save_bytes().unwrap_or_else(|e| panic!("{e}"));
    let mut keyboard = DockedScreens::new(SystemId::AVENOR, "SH025");
    let mut pointer = DockedScreens::new(SystemId::AVENOR, "SH025");
    keyboard.open(Screen::Trading);
    pointer.open(Screen::Trading);
    for command in [
        Key::QuantityLessTen,
        Key::QuantityMoreTen,
        Key::QuantityMore,
        Key::QuantityLess,
    ] {
        assert_eq!(key(&mut keyboard, &ctx, command), Outcome::Updated);
        assert_eq!(
            click(&mut pointer, &ctx, Control::Key(command)),
            Outcome::Updated
        );
    }
    let expected = Outcome::Request(Request::ReviewCommodity {
        commodity: CommodityId::Food,
        buy: true,
        quantity: 11,
    });
    assert_eq!(key(&mut keyboard, &ctx, Key::Buy), expected);
    assert_eq!(
        click(&mut pointer, &ctx, Control::Key(Key::Confirm)),
        expected
    );
    assert_eq!(w.save_bytes().unwrap_or_else(|e| panic!("{e}")), before);
}

#[test]
fn changed_quantity_invalidates_old_frame_and_commodity_selection_resets_it() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Trading);
    let old = ui.frame(&ctx, view());
    assert_eq!(
        ui.handle(Input::Key(Key::QuantityMoreTen), &old, &ctx),
        Outcome::Updated
    );
    assert!(matches!(
        ui.handle(Input::Key(Key::Buy), &old, &ctx),
        Outcome::Refused(_)
    ));
    assert_eq!(key(&mut ui, &ctx, Key::Next), Outcome::Updated);
    assert_eq!(ui.quantity, 1);
    assert_eq!(key(&mut ui, &ctx, Key::QuantityLess), Outcome::Updated);
    assert_eq!(ui.quantity, 1);
    ui.quantity = u64::MAX;
    assert_eq!(key(&mut ui, &ctx, Key::QuantityMoreTen), Outcome::Updated);
    assert_eq!(ui.quantity, u64::MAX);
    assert_eq!(
        click(&mut ui, &ctx, Control::SelectCommodity(CommodityId::Gold)),
        Outcome::Updated
    );
    assert_eq!(
        key(&mut ui, &ctx, Key::Sell),
        Outcome::Request(Request::ReviewCommodity {
            commodity: CommodityId::Gold,
            buy: false,
            quantity: 1,
        })
    );
}

#[test]
fn quantity_pointer_controls_remain_visible_at_compact_viewport() {
    let c = catalogue();
    let w = world();
    let ctx = context(&w, &c);
    let mut ui = DockedScreens::new(SystemId::AVENOR, "SH025");
    ui.open(Screen::Trading);
    let viewport = Viewport::new(640.0, 360.0).unwrap_or_else(|| panic!("viewport"));
    let frame = ui.frame(&ctx, viewport);
    for command in [
        Key::QuantityLessTen,
        Key::QuantityLess,
        Key::QuantityMore,
        Key::QuantityMoreTen,
    ] {
        let hit = frame
            .hits
            .iter()
            .find(|h| h.control == Control::Key(command))
            .unwrap_or_else(|| panic!("quantity control missing"));
        assert!(hit.enabled);
        assert!(hit.bounds.x >= 0.0 && hit.bounds.x + hit.bounds.width <= viewport.width);
        assert!(hit.bounds.y >= 0.0 && hit.bounds.y + hit.bounds.height <= viewport.height);
    }
    assert!(
        frame
            .labels
            .iter()
            .any(|l| l.text.contains("Quantity: 1 t"))
    );
}

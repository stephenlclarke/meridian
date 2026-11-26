use super::*;
#[test]
fn code_generated_atlas_matches_existing_glyph_rows() {
    let a = atlas();
    assert_eq!(a.len(), 80 * 56);
    for ch in b'A'..=b'Z' {
        for (y, row) in crate::hud::glyph(char::from(ch)).into_iter().enumerate() {
            for x in 0..5 {
                assert_eq!(
                    a[(usize::from(ch) / 16 * 7 + y) * 80 + usize::from(ch) % 16 * 5 + x],
                    if row & (1 << (4 - x)) != 0 { 255 } else { 0 }
                );
            }
        }
    }
}
#[test]
fn text_shader_validates_and_atlas_coordinates_clip_instead_of_stretching() {
    let module = wgpu::naga::front::wgsl::parse_str(include_str!("text.wgsl"))
        .unwrap_or_else(|e| panic!("{e}"));
    wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{e}"));
    let mut out = Vec::new();
    let canvas = Rect::new(0.0, 0.0, 640.0, 360.0);
    let l = crate::docked_screens::Label {
        text: "A".into(),
        at: [8.0, 8.0],
        clip: Rect::new(10.0, 10.0, 2.0, 3.0),
        scale: 1.0,
        ink: Ink::White,
    };
    label(&mut out, &l, canvas, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out.len(), 6);
    assert_eq!(out[0].uv, [(5.0 + 2.0) / 80.0, (28.0 + 2.0) / 56.0]);
    assert_eq!(out[0].position, position([10.0, 10.0], [640.0, 360.0]));
}
#[test]
fn lines_and_model_triangles_clip_all_edges_with_finite_preserved_depth() {
    let c = Rect::new(10.0, 20.0, 30.0, 40.0);
    let clipped = clip_segment([-100.0, 40.0], [100.0, 40.0], c)
        .unwrap_or_else(|| panic!("missing clipped line"));
    for (actual, expected) in clipped
        .0
        .into_iter()
        .chain(clipped.1)
        .zip([10.0, 40.0, 40.0, 40.0])
    {
        assert!((actual - expected).abs() < 1e-10);
    }
    assert!(clip_segment([f64::NAN, 0.0], [0.0; 2], c).is_none());
    let v = |x, y, z| ClipVertex {
        p: [x, y, z],
        c: [1.0; 3],
    };
    let mut out = Vec::new();
    triangle(
        &mut out,
        [v(0.0, 0.0, 0.2), v(100.0, 0.0, 0.4), v(20.0, 100.0, 0.8)],
        c,
        [640.0, 360.0],
    );
    assert!(!out.is_empty());
    for v in out {
        let x = (f64::from(v.position[0]) + 1.0) * 320.0;
        let y = (1.0 - f64::from(v.position[1])) * 180.0;
        assert!((10.0 - 0.0001..=40.0001).contains(&x));
        assert!((20.0 - 0.0001..=60.0001).contains(&y));
        assert!((0.2..=0.8).contains(&v.position[2]));
    }
}
#[test]
fn only_actual_matching_approved_model_renders_and_all_three_rotations_preserve_extent() {
    let m = Model::sh025().unwrap_or_else(|e| panic!("{e}"));
    let mut p = ModelPreview {
        object: "SH025".into(),
        geometry_key: SHIP_KEY.into(),
        bounds: Rect::new(20.0, 30.0, 200.0, 100.0),
        orientation: Orientation::default(),
    };
    assert!(m.matches(&p));
    let mut out = Vec::with_capacity(MAX_MODEL_VERTICES);
    for o in [
        Orientation {
            pitch: 0.0,
            roll: 0.0,
            yaw: 0.0,
        },
        Orientation {
            pitch: 1.0,
            roll: 2.0,
            yaw: 3.0,
        },
    ] {
        p.orientation = o;
        append_model(&mut out, &p, &m, Rect::new(0.0, 0.0, 640.0, 360.0))
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(
            out.iter()
                .all(|v| v.position.into_iter().all(f32::is_finite))
        );
        out.clear();
    }
    p.object = "SH026".into();
    assert!(!m.matches(&p));
    p.object = "SH025".into();
    p.geometry_key = "wrong".into();
    assert!(!m.matches(&p));
}
fn context<'a>(
    world: &'a meridian_core::World,
    catalogue: &'a crate::docked_screens::Catalogue,
) -> crate::docked_screens::Context<'a> {
    crate::docked_screens::Context {
        world,
        catalogue,
        stamp: crate::docked_screens::Stamp {
            session: 1,
            revision: 1,
        },
        current_hull: "SH025",
        paused: false,
        current_reach_ly: Some(9.0),
        full_reach_ly: Some(9.0),
        route_targets: &[meridian_core::navigation::SystemId::AVENOR],
        models: &MODELS,
        planet_previews: true,
        navigation_bodies: &[],
        exchanges: &[],
        equipment: None,
        review: None,
        bindings: &[],
    }
}
#[test]
fn all_five_real_screen_projections_are_finite_bounded_and_reuse_render_storage() {
    let world = meridian_core::World::new_commander(95001).unwrap_or_else(|e| panic!("{e}"));
    let before = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
    let catalogue = crate::docked_screens::Catalogue::bundled().unwrap_or_else(|e| panic!("{e}"));
    let ctx = context(&world, &catalogue);
    let mut ui = crate::docked_screens::DockedScreens::new(
        meridian_core::navigation::SystemId::AVENOR,
        "SH025",
    );
    let model = Model::sh025().unwrap_or_else(|e| panic!("{e}"));
    let mut geometry = Geometry::with_capacity();
    let capacities = [
        geometry.shapes.capacity(),
        geometry.text.capacity(),
        geometry.models.capacity(),
    ];
    for [w, h] in [[640.0, 360.0], [1280.0, 800.0], [1920.0, 1080.0]] {
        for screen in crate::docked_screens::Screen::ALL {
            ui.open(screen);
            let frame = ui.frame(
                &ctx,
                crate::docked_screens::Viewport::new(w, h).unwrap_or_else(|| panic!("viewport")),
            );
            geometry
                .compose(&frame, &model, true, None)
                .unwrap_or_else(|e| panic!("{screen:?}: {e}"));
            assert!(!geometry.text.is_empty());
            assert!(geometry.shapes.iter().chain(&geometry.models).all(|v| {
                v.position
                    .into_iter()
                    .all(|p| p.is_finite() && p.abs() <= 1.00001)
            }));
            assert!(geometry.text.iter().all(|v| {
                v.position
                    .into_iter()
                    .all(|p| p.is_finite() && p.abs() <= 1.00001)
            }));
            assert_eq!(
                capacities,
                [
                    geometry.shapes.capacity(),
                    geometry.text.capacity(),
                    geometry.models.capacity()
                ]
            );
        }
    }
    assert_eq!(world.save_bytes().unwrap_or_else(|e| panic!("{e}")), before);
}
#[test]
fn actual_hover_geometry_and_pointer_coordinates_share_the_same_logical_frame_at_retina_scale() {
    let world = meridian_core::World::new_commander(95002).unwrap_or_else(|e| panic!("{e}"));
    let catalogue = crate::docked_screens::Catalogue::bundled().unwrap_or_else(|e| panic!("{e}"));
    let ctx = context(&world, &catalogue);
    let mut ui = crate::docked_screens::DockedScreens::new(
        meridian_core::navigation::SystemId::AVENOR,
        "SH025",
    );
    let view = crate::docked_screens::Viewport::new(640.0, 360.0).unwrap_or_else(|| panic!("view"));
    let frame = ui.frame(&ctx, view);
    let bounds = frame
        .actionable_regions()
        .next()
        .map(|(r, _)| r)
        .unwrap_or_else(|| panic!("action"));
    let point = bounds.centre();
    let ndc = position(point, [640.0, 360.0]);
    let physical = [
        (f64::from(ndc[0]) + 1.0) * 640.0,
        (1.0 - f64::from(ndc[1])) * 360.0,
    ];
    let logical = physical.map(|v| v / 2.0);
    assert!(bounds.contains(logical));
    ui.handle(
        crate::docked_screens::Input::PointerMoved(logical),
        &frame,
        &ctx,
    );
    let painted = ui.frame(&ctx, view);
    assert_eq!(painted.hovered, Some(bounds));
    let mut geometry = Geometry::with_capacity();
    geometry
        .compose(
            &painted,
            &Model::sh025().unwrap_or_else(|e| panic!("{e}")),
            true,
            None,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        geometry
            .shapes
            .iter()
            .any(|v| v.color == colour(Ink::Yellow, true))
    );
}
#[test]
fn oversized_inputs_refuse_before_buffer_growth_and_unknown_model_draws_no_substitute() {
    let world = meridian_core::World::new_commander(95003).unwrap_or_else(|e| panic!("{e}"));
    let catalogue = crate::docked_screens::Catalogue::bundled().unwrap_or_else(|e| panic!("{e}"));
    let ctx = context(&world, &catalogue);
    let ui = crate::docked_screens::DockedScreens::new(
        meridian_core::navigation::SystemId::AVENOR,
        "SH025",
    );
    let mut frame = ui.frame(
        &ctx,
        crate::docked_screens::Viewport::new(640.0, 360.0).unwrap_or_else(|| panic!("view")),
    );
    let model = Model::sh025().unwrap_or_else(|e| panic!("{e}"));
    let mut geometry = Geometry::with_capacity();
    frame.models = vec![ModelPreview {
        object: "SH162".into(),
        geometry_key: SHIP_KEY.into(),
        bounds: Rect::new(20.0, 120.0, 200.0, 100.0),
        orientation: Orientation::default(),
    }];
    geometry
        .compose(&frame, &model, true, None)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(geometry.unavailable_models, 1);
    assert!(geometry.models.is_empty());
    let capacity = geometry.text.capacity();
    frame.labels[0].text = "A".repeat(MAX_GLYPHS * 4 + 1);
    assert!(geometry.compose(&frame, &model, true, None).is_err());
    assert_eq!(geometry.text.capacity(), capacity);
}
#[test]
fn current_system_planet_preview_uses_actual_world_recipe_and_never_a_foreign_world_default() {
    let world = meridian_core::World::new_commander(95004).unwrap_or_else(|e| panic!("{e}"));
    let mut adapter = crate::celestial::WorldAdapter::default();
    let actual = world
        .snapshot()
        .navigation
        .unwrap_or_else(|| panic!("nav"))
        .system;
    let scene = adapter
        .preview(world.celestial_snapshot(), actual)
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|| panic!("actual planet"));
    assert_eq!(scene.planet.as_ref().map(|p| p.id.system), Some(actual));
    assert!(
        adapter
            .preview(
                world.celestial_snapshot(),
                meridian_core::navigation::SystemId::RENAS
            )
            .unwrap_or_else(|e| panic!("{e}"))
            .is_none()
    );
}

#[test]
fn selecting_goshawk_routes_the_original_preview_without_granting_exchange() {
    assert_inspection_preview("Goshawk Explorer", "SH008", "assets/fleet/sh008/visual.bin");
}

#[test]
fn selecting_mk_ii_custom_routes_its_own_preview_without_granting_exchange() {
    assert_inspection_preview(
        "Goshawk Mk II Custom",
        "SH010",
        "assets/fleet/sh010/visual.bin",
    );
}

#[test]
fn selecting_medusa_routes_the_original_preview_without_granting_exchange() {
    assert_inspection_preview("Medusa", "SH171", "assets/fleet/sh171/visual.bin");
}

fn assert_inspection_preview(name: &str, id: &str, key: &str) {
    use crate::docked_screens::{Catalogue, DockedScreens, Input, Key, Outcome, Screen, Viewport};
    let world = meridian_core::World::new_commander(95006).unwrap_or_else(|e| panic!("{e}"));
    let save = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
    let catalogue = Catalogue::bundled().unwrap_or_else(|e| panic!("{e}"));
    let mut ui = DockedScreens::new(meridian_core::navigation::SystemId::AVENOR, "SH025");
    let ctx = context(&world, &catalogue);
    let viewport = Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
    ui.open(Screen::Showroom);
    let frame = ui.frame(&ctx, viewport);
    assert!(matches!(
        ui.handle(Input::Key(Key::Search), &frame, &ctx),
        Outcome::Updated
    ));
    assert!(ui.search_active());
    let frame = ui.frame(&ctx, viewport);
    assert!(matches!(
        ui.handle(Input::Text(name.into()), &frame, &ctx),
        Outcome::Updated
    ));
    assert_eq!(ui.selected_ship(), id);
    let frame = ui.frame(&ctx, viewport);
    assert!(matches!(
        ui.handle(Input::Key(Key::Confirm), &frame, &ctx),
        Outcome::Updated
    ));
    assert!(!ui.search_active());
    assert!(!ui.details_open());

    // Compare starts enabled. Search changes the selected ship, not the current-hull baseline.
    let comparison = ui.frame(&ctx, viewport);
    let [baseline, selected] = comparison.models.as_slice() else {
        panic!("expected exactly two previews in default Compare");
    };
    let current_model = Model::sh025().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(baseline.object, "SH025");
    assert_eq!(baseline.geometry_key, SHIP_KEY);
    assert!(current_model.matches(baseline));
    assert_eq!(selected.object, id);
    assert_eq!(selected.geometry_key, key);
    assert!(crate::mule_render::showroom::matches(selected));
    assert!(baseline.bounds.x + baseline.bounds.width <= selected.bounds.x);
    let mut geometry = Geometry::with_capacity();
    geometry
        .compose(&comparison, &current_model, true, None)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(geometry.unavailable_models, 0);
    assert!(
        !geometry.models.is_empty(),
        "SH025 baseline was not composed"
    );

    assert!(matches!(
        ui.handle(Input::Key(Key::Compare), &comparison, &ctx),
        Outcome::Updated
    ));
    let frame = ui.frame(&ctx, viewport);
    let [solo] = frame.models.as_slice() else {
        panic!("expected exactly one preview with Compare off");
    };
    assert_eq!(solo.object, selected.object);
    assert_eq!(solo.geometry_key, selected.geometry_key);
    assert!(crate::mule_render::showroom::matches(solo));
    if id == "SH171" {
        assert!(
            frame
                .labels
                .iter()
                .any(|label| label.text == "Inspection only. Flight unavailable.")
        );
        assert!(
            frame
                .labels
                .iter()
                .any(|label| label.text == "PLUGH fabrication is unavailable.")
        );
        assert!(
            !frame
                .labels
                .iter()
                .any(|label| label.text.contains("Authored flyable hull"))
        );
        assert!(
            frame
                .labels
                .iter()
                .any(|label| label.text == "Empty stock dry mass: 40712 kg")
        );
        assert!(
            frame
                .labels
                .iter()
                .any(|label| label.text == "Stock figures exclude paid upgrades.")
        );
    }
    assert!(solo.bounds.width > selected.bounds.width);
    assert!(
        frame
            .labels
            .iter()
            .any(|label| label.text.contains("Exchange unavailable"))
    );
    geometry
        .compose(&frame, &current_model, true, None)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(geometry.unavailable_models, 0);
    assert!(
        geometry.models.is_empty(),
        "{id} entered the legacy hull path"
    );

    let outcome = ui.handle(Input::Key(Key::Exchange), &frame, &ctx);
    assert!(matches!(
        outcome,
        Outcome::Refused(reason) if reason == "No admitted ship exchange offer for this hull"
    ));
    assert_eq!(ui.selected_ship(), id);
    assert_eq!(ui.frame(&ctx, viewport), frame);
    assert_eq!(world.save_bytes().unwrap_or_else(|e| panic!("{e}")), save);
}

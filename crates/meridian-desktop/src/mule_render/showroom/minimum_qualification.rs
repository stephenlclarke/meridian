//! Check the original detailed model against the real minimum-size Showroom frame.
use super::{EMPTY_PIECE, GOSHAWK_PIECES, View, VisualVertex, matches, visual};
use crate::{
    app::MIN_WINDOW_LOGICAL,
    docked::Docked,
    docked_screens::{Frame, Input, Key, ModelPreview, Rect, Screen, Viewport},
    profile::tests::{TestDirectory, open},
};

pub(super) const POSES: [&str; 3] = ["front", "oblique", "upper"];

fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}

pub(super) struct Frames {
    pub(super) solo: Frame,
    pub(super) comparison: Frame,
}

pub(super) fn station_frames() -> Vec<Frames> {
    station_frames_for("Goshawk Explorer")
}

pub(super) fn station_frames_for(name: &str) -> Vec<Frames> {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(97009)));
    let before = ok(profile.world.save_bytes());
    let mut menu = ok(Docked::new(&profile.world));
    ok(menu.open(&profile.world, Screen::Showroom));
    let [width, height] = MIN_WINDOW_LOGICAL;
    let viewport = Viewport::new(width, height).unwrap_or_else(|| panic!("invalid window minimum"));
    let mut frames = Vec::with_capacity(POSES.len());
    {
        let mut input = |action| {
            let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
            menu.presented(frame);
            ok(menu.input(&mut profile, action));
        };
        input(Input::Key(Key::Search));
        input(Input::Text(name.into()));
        input(Input::Key(Key::Confirm));
        input(Input::Key(Key::ResetView));
    }
    for pose in POSES {
        let mut input = |action| {
            let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
            menu.presented(frame);
            ok(menu.input(&mut profile, Input::Key(action)));
        };
        match pose {
            "oblique" => {
                for _ in 0..2 {
                    input(Key::TurnRight);
                }
            }
            "upper" => {
                input(Key::NextAxis);
                for _ in 0..4 {
                    input(Key::TurnLeft);
                }
            }
            _ => {}
        }
        let comparison = ok(menu.build_frame(&profile.world, viewport, 0.0));
        menu.presented(comparison.clone());
        ok(menu.input(&mut profile, Input::Key(Key::Compare)));
        let solo = ok(menu.build_frame(&profile.world, viewport, 0.0));
        menu.presented(solo.clone());
        ok(menu.input(&mut profile, Input::Key(Key::Compare)));
        frames.push(Frames { solo, comparison });
    }
    assert_eq!(ok(profile.world.save_bytes()), before);
    frames
}

/// Exercise ordinary station Pin Baseline and search in both orders and duplicate identities.
pub(super) fn peregrine_sibling_compare_frames() -> [Frame; 4] {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(97022)));
    let before = ok(profile.world.save_bytes());
    let mut menu = ok(Docked::new(&profile.world));
    ok(menu.open(&profile.world, Screen::Showroom));
    let [width, height] = MIN_WINDOW_LOGICAL;
    let viewport = Viewport::new(width, height).unwrap_or_else(|| panic!("invalid window minimum"));
    let frames = [
        ("Peregrine Mk I", "Peregrine Mk III", ["SH022", "SH023"]),
        ("Peregrine Mk III", "Peregrine Mk I", ["SH023", "SH022"]),
        ("Peregrine Mk I", "Peregrine Mk I", ["SH022", "SH022"]),
        ("Peregrine Mk III", "Peregrine Mk III", ["SH023", "SH023"]),
    ]
    .map(|(baseline, selected, expected)| {
        {
            let mut input = |action| {
                let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
                menu.presented(frame);
                ok(menu.input(&mut profile, action));
            };
            for action in [
                Input::Key(Key::Search),
                Input::Key(Key::ClearSearch),
                Input::Text(baseline.into()),
                Input::Key(Key::Confirm),
                Input::Key(Key::PinBaseline),
                Input::Key(Key::Search),
                Input::Key(Key::ClearSearch),
                Input::Text(selected.into()),
                Input::Key(Key::Confirm),
                Input::Key(Key::ResetView),
            ] {
                input(action);
            }
        }
        let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
        let [left, right] = frame.models.as_slice() else {
            panic!("expected pinned Peregrine panes");
        };
        for (model, id) in [left, right].into_iter().zip(expected) {
            assert_eq!(model.object, id);
            let (key, hull) = if id == "SH022" {
                (super::PEREGRINE_I_GEOMETRY_KEY, super::Hull::PeregrineMkI)
            } else {
                (
                    super::PEREGRINE_III_GEOMETRY_KEY,
                    super::Hull::PeregrineMkIII,
                )
            };
            assert_eq!(model.geometry_key, key);
            assert_eq!(super::hull(model), Some(hull));
            assert_model_pane(&frame, model.bounds);
        }
        assert!(!overlaps(left.bounds, right.bounds));
        assert!(left.bounds.x < right.bounds.x);
        assert_eq!(left.orientation, right.orientation);
        frame
    });
    assert_eq!(ok(profile.world.save_bytes()), before);
    frames
}

/// Actual ordinary Pin Baseline for both schema-stock identities, in both orders and duplicates.
pub(super) fn stock_sparrowhawk_kite_compare_frames() -> [Frame; 4] {
    stock_pair_compare_frames(("Sparrowhawk", "SH030"), ("Kite Interceptor", "SH031"))
}

pub(super) fn stock_pair_compare_frames(first: (&str, &str), second: (&str, &str)) -> [Frame; 4] {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(97031)));
    let before = ok(profile.world.save_bytes());
    let mut menu = ok(Docked::new(&profile.world));
    ok(menu.open(&profile.world, Screen::Showroom));
    let [width, height] = MIN_WINDOW_LOGICAL;
    let viewport = Viewport::new(width, height).unwrap_or_else(|| panic!("viewport"));
    let frames = [
        (first.0, second.0, [first.1, second.1]),
        (second.0, first.0, [second.1, first.1]),
        (first.0, first.0, [first.1, first.1]),
        (second.0, second.0, [second.1, second.1]),
    ]
    .map(|(baseline, selected, ids)| {
        for action in [
            Input::Key(Key::Search),
            Input::Key(Key::ClearSearch),
            Input::Text(baseline.into()),
            Input::Key(Key::Confirm),
            Input::Key(Key::PinBaseline),
            Input::Key(Key::Search),
            Input::Key(Key::ClearSearch),
            Input::Text(selected.into()),
            Input::Key(Key::Confirm),
            Input::Key(Key::ResetView),
        ] {
            let frame = ok(menu.build_frame(&profile.world, viewport, 0.));
            menu.presented(frame);
            ok(menu.input(&mut profile, action));
        }
        let frame = ok(menu.build_frame(&profile.world, viewport, 0.));
        let [left, right] = frame.models.as_slice() else {
            panic!("expected actual stock comparison panes");
        };
        for (model, id) in [left, right].into_iter().zip(ids) {
            assert_eq!(model.object, id);
            assert!(ok(crate::authored_hull::stock_visual_for(model)).is_some());
            assert_model_pane(&frame, model.bounds);
        }
        assert!(!overlaps(left.bounds, right.bounds));
        assert!(left.bounds.x < right.bounds.x);
        assert_eq!(left.orientation, right.orientation);
        frame
    });
    assert_eq!(ok(profile.world.save_bytes()), before);
    frames
}

#[test]
fn stock_sparrowhawk_kite_pin_actual_station_compare_in_both_orders_and_duplicates() {
    let frames = stock_sparrowhawk_kite_compare_frames();
    assert_eq!(
        frames.each_ref().map(|frame| frame
            .models
            .iter()
            .map(|model| model.object.as_str())
            .collect::<Vec<_>>()),
        [
            vec!["SH030", "SH031"],
            vec!["SH031", "SH030"],
            vec!["SH030", "SH030"],
            vec!["SH031", "SH031"]
        ]
    );
}

#[test]
fn stock_stag_pins_with_each_stock_peer_in_both_orders_and_duplicates() {
    for (name, id) in [("Sparrowhawk", "SH030"), ("Kite Interceptor", "SH031")] {
        let frames = stock_pair_compare_frames(("Stag Liner", "SH035"), (name, id));
        for (frame, expected) in
            frames
                .iter()
                .zip([["SH035", id], [id, "SH035"], ["SH035", "SH035"], [id, id]])
        {
            assert_eq!(
                frame
                    .models
                    .iter()
                    .map(|model| model.object.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}

#[test]
fn peregrine_siblings_pin_actual_station_compare_in_both_orders_and_duplicates() {
    let mk_i = ok(super::Hull::PeregrineMkI.visual());
    let mk_iii = ok(super::Hull::PeregrineMkIII.visual());
    let mk_i_view = View::new(&mk_i, [EMPTY_PIECE; super::PEREGRINE_I_PIECES]);
    let mk_iii_view = View::new(&mk_iii, [EMPTY_PIECE; super::PEREGRINE_MK3_PIECES]);
    let mut frames = peregrine_sibling_compare_frames();
    // Native cache qualification uses different poses for duplicate pane identities.
    for (frame, name) in frames[2..]
        .iter_mut()
        .zip(["Peregrine Mk I", "Peregrine Mk III"])
    {
        let upper = station_frames_for(name)[2].solo.models[0].orientation;
        frame.models[1].orientation = upper;
        assert_ne!(frame.models[0].orientation, upper);
    }
    for frame in frames {
        for model in &frame.models {
            match super::hull(model) {
                Some(super::Hull::PeregrineMkI) => assert_original_fits(&mk_i_view, &mk_i, model),
                Some(super::Hull::PeregrineMkIII) => {
                    assert_original_fits(&mk_iii_view, &mk_iii, model)
                }
                _ => panic!("unrelated model in pinned Peregrine comparison"),
            }
        }
    }
}

fn encloses(outer: Rect, inner: Rect) -> bool {
    inner.width > 0.0
        && inner.height > 0.0
        && inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

fn assert_model_pane(frame: &Frame, pane: Rect) {
    assert!(encloses(frame.viewport.body(), pane));
    assert!(
        frame
            .actionable_regions()
            .all(|(region, _)| !overlaps(region, pane)),
        "model pane overlaps a station control: {pane:?}"
    );
}

fn assert_original_fits(view: &View, vertices: &[VisualVertex], model: &ModelPreview) {
    let bounds = model.bounds;
    let data = ok(view.uniforms(model.orientation, bounds.width / bounds.height, true));
    let [aspect, focal, near, far] = data.projection;
    let margin = 2.0;
    let padded = Rect::new(
        bounds.x + margin,
        bounds.y + margin,
        bounds.width - margin * 2.0,
        bounds.height - margin * 2.0,
    );
    for vertex in vertices {
        // Same homogeneous projection and viewport transform as the production draw.
        let [a, b, c] = vertex.position;
        let [x, y, z]: [f32; 3] =
            std::array::from_fn(|i| data.origin[i] + a * data.x[i] + b * data.y[i] + c * data.z[i]);
        let clip_z = z * far / (far - near) - near * far / (far - near);
        assert!(z > 0.0 && clip_z > 0.0 && clip_z < z);
        let projected = [
            bounds.x + (1.0 + f64::from(x * focal / aspect / z)) * bounds.width / 2.0,
            bounds.y + (1.0 - f64::from(y * focal / z)) * bounds.height / 2.0,
        ];
        assert!(
            padded.contains(projected),
            "original Goshawk vertex {projected:?} misses padded pane {padded:?}"
        );
    }
}

#[test]
fn original_goshawk_fits_minimum_station_showroom_solo_and_compare() {
    let vertices = ok(visual());
    assert_eq!(vertices.len(), 3_484 * 3);
    // The opaque projection uses the original detailed visual, independently of shimmer.
    let view = View::new(&vertices, [EMPTY_PIECE; GOSHAWK_PIECES]);
    assert_variant_fits_minimum(station_frames(), &view, &vertices, "SH008");
}

#[test]
fn original_goshawk_mk2_fits_minimum_station_showroom_solo_and_compare() {
    let vertices = ok(super::Hull::Mk2.visual());
    assert_eq!(vertices.len(), 3_532 * 3);
    let view = View::new(&vertices, [EMPTY_PIECE; super::GOSHAWK_MK2_PIECES]);
    assert_variant_fits_minimum(
        station_frames_for("Goshawk Mk II"),
        &view,
        &vertices,
        "SH009",
    );
}

#[test]
fn original_custom_fits_minimum_station_showroom_solo_and_compare() {
    let vertices = ok(super::Hull::Custom.visual());
    assert_eq!(vertices.len(), super::CUSTOM_DETAILED_FACES * 3);
    let view = View::new(&vertices, [EMPTY_PIECE; super::CUSTOM_PIECES]);
    assert_variant_fits_minimum(
        station_frames_for("Goshawk Mk II Custom"),
        &view,
        &vertices,
        "SH010",
    );
}

#[test]
fn original_peregrine_mk_i_fits_minimum_station_showroom_solo_and_compare() {
    let vertices = ok(super::Hull::PeregrineMkI.visual());
    assert_eq!(vertices.len(), super::PEREGRINE_I_DETAILED_FACES * 3);
    let view = View::new(&vertices, [EMPTY_PIECE; super::PEREGRINE_I_PIECES]);
    assert_variant_fits_minimum(
        station_frames_for("Peregrine Mk I"),
        &view,
        &vertices,
        "SH022",
    );
}

#[test]
fn original_peregrine_mk_iii_fits_minimum_station_showroom_solo_and_compare() {
    let vertices = ok(super::Hull::PeregrineMkIII.visual());
    assert_eq!(vertices.len(), super::PEREGRINE_MK3_DETAILED_FACES * 3);
    let view = View::new(&vertices, [EMPTY_PIECE; super::PEREGRINE_MK3_PIECES]);
    assert_variant_fits_minimum(
        station_frames_for("Peregrine Mk III"),
        &view,
        &vertices,
        "SH023",
    );
}

fn assert_variant_fits_minimum(
    frames: Vec<Frames>,
    view: &View,
    vertices: &[VisualVertex],
    id: &str,
) {
    for (index, Frames { solo, comparison }) in frames.into_iter().enumerate() {
        let [standalone] = solo.models.as_slice() else {
            panic!("expected solo {id}")
        };
        let [baseline, selected] = comparison.models.as_slice() else {
            panic!("expected baseline and {id} comparison");
        };
        assert_eq!(baseline.object, "SH025");
        assert_eq!(baseline.geometry_key, "assets/legacy/sh025.mesh");
        assert!(matches(standalone) && matches(selected));
        assert_eq!(standalone.object, id);
        assert_eq!(selected.object, id);
        assert_eq!(standalone.orientation, selected.orientation);
        let orientation = standalone.orientation;
        let yaw = if index == 0 {
            0.0
        } else {
            std::f64::consts::PI / 6.0
        };
        let pitch = if index == 2 {
            (0.25 - std::f64::consts::PI / 3.0).rem_euclid(std::f64::consts::TAU)
        } else {
            0.25
        };
        assert!((orientation.yaw - yaw).abs() < 1e-12);
        assert!((orientation.pitch - pitch).abs() < 1e-12);
        assert_eq!(orientation.roll, 0.0);
        for model in [baseline, selected] {
            assert_model_pane(&comparison, model.bounds);
            assert!(encloses(standalone.bounds, model.bounds));
        }
        assert_model_pane(&solo, standalone.bounds);
        assert!(!overlaps(baseline.bounds, selected.bounds));
        assert!(baseline.bounds.x < selected.bounds.x);
        for model in [standalone, selected] {
            assert_original_fits(view, vertices, model);
        }
    }
}

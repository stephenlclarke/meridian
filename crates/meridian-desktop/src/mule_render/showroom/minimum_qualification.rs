//! Check the original detailed model against the real minimum-size Showroom frame.
use super::{EMPTY_PIECE, Hull, View, VisualVertex, visual_assets};
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

pub(super) fn search_name(hull: Hull) -> &'static str {
    match hull {
        Hull::Explorer => "Goshawk Explorer",
        Hull::Mk2 => "Goshawk Mk II",
        Hull::Custom => "Goshawk Mk II Custom",
        Hull::Raven => "Raven",
    }
}

pub(super) fn station_frames(hull: Hull) -> Vec<Frames> {
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
        input(Input::Text(search_name(hull).into()));
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

fn assert_minimum_station_fit(hull: Hull, expected_vertices: usize) {
    let vertices = ok(visual_assets::load(hull.geometry_key()));
    assert_eq!(vertices.len(), expected_vertices);
    // The opaque projection uses the original detailed visual, independently of shimmer.
    let view = View::new(&vertices, &vec![EMPTY_PIECE; hull.piece_count()]);
    for (index, Frames { solo, comparison }) in station_frames(hull).into_iter().enumerate() {
        let [standalone] = solo.models.as_slice() else {
            panic!("expected one original Goshawk model")
        };
        let [baseline, selected] = comparison.models.as_slice() else {
            panic!("expected baseline and original Goshawk comparison");
        };
        assert_eq!(baseline.object, "SH025");
        assert_eq!(baseline.geometry_key, "assets/legacy/sh025.mesh");
        assert_eq!(Hull::from_model(standalone), Some(hull));
        assert_eq!(Hull::from_model(selected), Some(hull));
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
            assert_original_fits(&view, &vertices, model);
        }
    }
}

#[test]
fn original_goshawk_fits_minimum_station_showroom_solo_and_compare() {
    assert_minimum_station_fit(Hull::Explorer, 3_484 * 3);
}

#[test]
fn original_goshawk_mk2_fits_minimum_station_showroom_solo_and_compare() {
    assert_minimum_station_fit(Hull::Mk2, 3_532 * 3);
}

#[test]
fn original_custom_fits_minimum_station_showroom_solo_and_compare() {
    assert_minimum_station_fit(Hull::Custom, super::CUSTOM_DETAILED_FACES * 3);
}

#[test]
fn original_raven_fits_minimum_station_showroom_solo_and_compare() {
    assert_minimum_station_fit(Hull::Raven, super::RAVEN_DETAILED_FACES * 3);
}

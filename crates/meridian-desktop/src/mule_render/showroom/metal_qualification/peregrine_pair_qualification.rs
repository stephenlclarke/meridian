//! Actual SH022/SH023 station pinning, minimum rendering and cache identity evidence.
use super::{
    MIN_WINDOW_LOGICAL, Target,
    custom_qualification::{Capture, original_pass},
    metal_gpu_for, output_directory,
};
use crate::mule_render::showroom::{
    self, Hull,
    minimum_qualification::{self, Frames},
};

#[test]
#[ignore = "Mac owner only: independent SH023 production minimum Showroom RGB evidence"]
fn metal_peregrine_mk_iii_minimum_showroom_model_pass() {
    let output = output_directory("MERIDIAN_SH023_MINIMUM_DIR");
    let gpu = metal_gpu_for("SH023", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH023");
    let mut mk_iii = original_pass(&gpu, Hull::PeregrineMkIII);
    for (pose, Frames { solo, comparison }) in
        minimum_qualification::POSES
            .into_iter()
            .zip(minimum_qualification::station_frames_for(
                "Peregrine Mk III",
            ))
    {
        let [left, right] = comparison.models.as_slice() else {
            panic!("expected native SH025/SH023 Compare");
        };
        assert_eq!(left.object, "SH025");
        assert_eq!(showroom::hull(right), Some(Hull::PeregrineMkIII));
        capture.solo(&format!("{pose}-solo"), &solo, &mut mk_iii);
        capture.comparison(&format!("{pose}-compare"), &comparison, None, &mut mk_iii);
    }
    capture.finish();
}

#[test]
#[ignore = "Mac owner only: actual Peregrine siblings and independent duplicate-pose cache evidence"]
fn metal_peregrine_sibling_cache_orders() {
    let output = output_directory("MERIDIAN_SH022_SH023_CACHE_DIR");
    let gpu = metal_gpu_for("SH022-SH023", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH022-SH023");
    let mut mk_i = original_pass(&gpu, Hull::PeregrineMkI);
    let mut mk_iii = original_pass(&gpu, Hull::PeregrineMkIII);
    let mut mk_i_duplicate = original_pass(&gpu, Hull::PeregrineMkI);
    let mut mk_iii_duplicate = original_pass(&gpu, Hull::PeregrineMkIII);
    let i_frames = minimum_qualification::station_frames_for("Peregrine Mk I");
    let iii_frames = minimum_qualification::station_frames_for("Peregrine Mk III");
    let first = capture.solo("mk-i-before", &i_frames[0].solo, &mut mk_i);
    let second = capture.solo("mk-iii-before", &iii_frames[0].solo, &mut mk_iii);
    assert!(
        first != second,
        "different original Peregrine hulls produced identical RGB"
    );

    let [i_iii, iii_i, mut i_i, mut iii_iii] =
        minimum_qualification::peregrine_sibling_compare_frames();
    for (frame, expected) in [
        (&i_iii, [Hull::PeregrineMkI, Hull::PeregrineMkIII]),
        (&iii_i, [Hull::PeregrineMkIII, Hull::PeregrineMkI]),
    ] {
        let [left, right] = frame.models.as_slice() else {
            panic!("expected two pinned panes");
        };
        assert_eq!(
            [showroom::hull(left), showroom::hull(right)],
            expected.map(Some)
        );
        assert_eq!(left.orientation, right.orientation);
    }
    capture.comparison_by_pane("mk-i-mk-iii", &i_iii, [Some(&mut mk_i), Some(&mut mk_iii)]);
    capture.comparison_by_pane("mk-iii-mk-i", &iii_i, [Some(&mut mk_iii), Some(&mut mk_i)]);

    // The UI synchronizes orientations. Retain its real pinned duplicate identities and
    // bounds but substitute the actual upper pose as a renderer-only isolation fixture.
    for (name, frame, upper, hull, left_ref, right_ref) in [
        (
            "mk-i-mk-i",
            &mut i_i,
            &i_frames[2].solo,
            Hull::PeregrineMkI,
            &mut mk_i,
            &mut mk_i_duplicate,
        ),
        (
            "mk-iii-mk-iii",
            &mut iii_iii,
            &iii_frames[2].solo,
            Hull::PeregrineMkIII,
            &mut mk_iii,
            &mut mk_iii_duplicate,
        ),
    ] {
        let [left, right] = frame.models.as_mut_slice() else {
            panic!("expected same-hull panes");
        };
        let [upper] = upper.models.as_slice() else {
            panic!("expected original upper pose");
        };
        assert_eq!(
            [
                showroom::hull(left),
                showroom::hull(right),
                showroom::hull(upper)
            ],
            [Some(hull); 3]
        );
        assert_eq!(left.orientation, right.orientation);
        right.orientation = upper.orientation;
        assert_ne!(left.orientation, right.orientation);
        capture.comparison_by_pane(name, frame, [Some(left_ref), Some(right_ref)]);
    }
    assert!(first == capture.solo("mk-i-return", &i_frames[0].solo, &mut mk_i));
    assert!(second == capture.solo("mk-iii-return", &iii_frames[0].solo, &mut mk_iii));
    capture.finish();
}

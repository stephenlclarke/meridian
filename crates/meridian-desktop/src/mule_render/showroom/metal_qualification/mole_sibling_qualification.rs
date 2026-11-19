//! Actual SH016/SH017 pinned station comparison and duplicate-pose cache proof.
use super::{
    MIN_WINDOW_LOGICAL, Target,
    custom_qualification::{Capture, original_pass},
    metal_gpu_for, output_directory,
};
use crate::mule_render::showroom::{self, Hull, minimum_qualification};

#[test]
#[ignore = "Mac owner only: actual Mole siblings and independent duplicate-pose cache evidence"]
fn metal_mole_sibling_cache_orders() {
    let output = output_directory("MERIDIAN_SH016_SH017_CACHE_DIR");
    let gpu = metal_gpu_for("SH016-SH017", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH016-SH017");
    let mut miner = original_pass(&gpu, Hull::MoleMiner);
    let mut prospector = original_pass(&gpu, Hull::MoleProspector);
    let miner_frames = minimum_qualification::station_frames_for("Mole Miner");
    let prospector_frames = minimum_qualification::station_frames_for("Mole Prospector");
    let first = capture.solo("miner-before", &miner_frames[0].solo, &mut miner);
    let second = capture.solo(
        "prospector-before",
        &prospector_frames[0].solo,
        &mut prospector,
    );
    assert!(
        first != second,
        "different original Mole hulls produced identical RGB"
    );

    let [
        miner_prospector,
        prospector_miner,
        mut miner_duplicate,
        mut prospector_duplicate,
    ] = minimum_qualification::mole_sibling_compare_frames();
    // Both mixed frames are the actual ordinary station Pin Baseline comparisons.
    for (name, frame, expected) in [
        (
            "miner-prospector",
            &miner_prospector,
            [Hull::MoleMiner, Hull::MoleProspector],
        ),
        (
            "prospector-miner",
            &prospector_miner,
            [Hull::MoleProspector, Hull::MoleMiner],
        ),
    ] {
        let [left, right] = frame.models.as_slice() else {
            panic!("expected two actual pinned panes")
        };
        assert_eq!(
            [showroom::hull(left), showroom::hull(right)],
            expected.map(Some)
        );
        assert_eq!(left.orientation, right.orientation);
        capture.comparison(name, frame, Some(&mut miner), &mut prospector);
    }

    // The real UI synchronizes its two panes. Retain its genuine duplicate model IDs and
    // bounds but substitute the right pane's actual upper pose as a renderer-only cache test.
    for (name, frame, upper, hull) in [
        (
            "miner-miner",
            &mut miner_duplicate,
            &miner_frames[2].solo,
            Hull::MoleMiner,
        ),
        (
            "prospector-prospector",
            &mut prospector_duplicate,
            &prospector_frames[2].solo,
            Hull::MoleProspector,
        ),
    ] {
        let [left, right] = frame.models.as_mut_slice() else {
            panic!("expected duplicate pinned panes")
        };
        let [upper] = upper.models.as_slice() else {
            panic!("expected original upper pose")
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
        capture.comparison(name, frame, Some(&mut miner), &mut prospector);
    }
    assert!(first == capture.solo("miner-return", &miner_frames[0].solo, &mut miner));
    assert!(
        second
            == capture.solo(
                "prospector-return",
                &prospector_frames[0].solo,
                &mut prospector
            )
    );
    capture.finish();
}

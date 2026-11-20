//! SH021 production minimum viewport and cross-hull GPU cache evidence.
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
#[ignore = "Mac owner only: independent SH021 production minimum Showroom RGB evidence"]
fn metal_zebra_minimum_showroom_model_pass() {
    let output = output_directory("MERIDIAN_SH021_MINIMUM_DIR");
    let gpu = metal_gpu_for("SH021", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH021");
    let mut zebra = original_pass(&gpu, Hull::Zebra);
    for (pose, Frames { solo, comparison }) in minimum_qualification::POSES
        .into_iter()
        .zip(minimum_qualification::station_frames_for("Zebra"))
    {
        let [left, right] = comparison.models.as_slice() else {
            panic!("expected native SH025/SH021 Compare")
        };
        assert_eq!(left.object, "SH025");
        assert_eq!(showroom::hull(right), Some(Hull::Zebra));
        capture.solo(&format!("{pose}-solo"), &solo, &mut zebra);
        capture.comparison(&format!("{pose}-compare"), &comparison, None, &mut zebra);
    }
    println!(
        "SH021 minimum Showroom logical={MIN_WINDOW_LOGICAL:?} physical={:?}",
        capture.size()
    );
    capture.finish();
}

#[test]
#[ignore = "Mac owner only: SH021 production cache switch, return and mixed order RGB evidence"]
fn metal_zebra_cache_switching() {
    let output = output_directory("MERIDIAN_SH021_CACHE_DIR");
    let gpu = metal_gpu_for("SH021", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH021");
    let mut explorer_pass = original_pass(&gpu, Hull::Explorer);
    let mut zebra_pass = original_pass(&gpu, Hull::Zebra);
    let explorer = minimum_qualification::station_frames().remove(0);
    let zebra = minimum_qualification::station_frames_for("Zebra").remove(0);
    let original = capture.solo("explorer-before", &explorer.solo, &mut explorer_pass);
    let revised = capture.solo("zebra-before", &zebra.solo, &mut zebra_pass);
    assert!(
        original != revised,
        "distinct original objects rendered identical full-target RGB"
    );
    assert!(original == capture.solo("explorer-return", &explorer.solo, &mut explorer_pass));
    assert!(revised == capture.solo("zebra-return", &zebra.solo, &mut zebra_pass));

    // A renderer-only fixture uses the actual station Compare pane geometry. It is separate
    // from the normal station's SH025 baseline and grants no hull exchange or gameplay state.
    let mut mixed = zebra.comparison.clone();
    let [left, right] = mixed.models.as_mut_slice() else {
        panic!("expected mixed model panes")
    };
    let [source] = explorer.solo.models.as_slice() else {
        panic!("expected one Explorer")
    };
    left.object.clone_from(&source.object);
    left.geometry_key.clone_from(&source.geometry_key);
    left.orientation = right.orientation;
    for (name, expected) in [
        ("explorer-zebra", [Hull::Explorer, Hull::Zebra]),
        ("zebra-explorer", [Hull::Zebra, Hull::Explorer]),
    ] {
        let [left, right] = mixed.models.as_slice() else {
            panic!("expected mixed model panes")
        };
        assert_eq!(
            [showroom::hull(left), showroom::hull(right)],
            expected.map(Some)
        );
        capture.comparison(name, &mixed, Some(&mut explorer_pass), &mut zebra_pass);
        let [left, right] = mixed.models.as_mut_slice() else {
            unreachable!()
        };
        std::mem::swap(&mut left.object, &mut right.object);
        std::mem::swap(&mut left.geometry_key, &mut right.geometry_key);
    }
    assert!(original == capture.solo("explorer-final", &explorer.solo, &mut explorer_pass));
    assert!(revised == capture.solo("zebra-final", &zebra.solo, &mut zebra_pass));
    // Qualify all three registered originals, including the sibling cache transition.
    let mut custom_pass = original_pass(&gpu, Hull::Custom);
    let custom = minimum_qualification::station_frames_for("Goshawk Mk II Custom").remove(0);
    let custom_rgb = capture.solo("custom-before", &custom.solo, &mut custom_pass);
    assert!(original != custom_rgb && revised != custom_rgb);
    assert!(revised == capture.solo("zebra-after-custom", &zebra.solo, &mut zebra_pass));
    let mut mixed = zebra.comparison.clone();
    let [left, right] = mixed.models.as_mut_slice() else {
        panic!("expected mixed model panes")
    };
    let [source] = custom.solo.models.as_slice() else {
        panic!("expected one Custom")
    };
    left.object.clone_from(&source.object);
    left.geometry_key.clone_from(&source.geometry_key);
    left.orientation = right.orientation;
    for (name, expected) in [
        ("custom-zebra", [Hull::Custom, Hull::Zebra]),
        ("zebra-custom", [Hull::Zebra, Hull::Custom]),
    ] {
        let [left, right] = mixed.models.as_slice() else {
            panic!("expected mixed model panes")
        };
        assert_eq!(
            [showroom::hull(left), showroom::hull(right)],
            expected.map(Some)
        );
        capture.comparison(name, &mixed, Some(&mut custom_pass), &mut zebra_pass);
        let [left, right] = mixed.models.as_mut_slice() else {
            unreachable!()
        };
        std::mem::swap(&mut left.object, &mut right.object);
        std::mem::swap(&mut left.geometry_key, &mut right.geometry_key);
    }
    assert!(custom_rgb == capture.solo("custom-final", &custom.solo, &mut custom_pass));
    assert!(revised == capture.solo("zebra-final-after-custom", &zebra.solo, &mut zebra_pass));
    println!(
        "SH021 cache return and both pane identity orders passed at {:?}",
        capture.size()
    );
    capture.finish();
}

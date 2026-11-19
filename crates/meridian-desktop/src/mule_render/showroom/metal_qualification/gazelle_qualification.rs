//! SH019 production minimum viewport and cross-hull GPU cache evidence.
use super::{
    Crop, FORMAT, LOGICAL, MIN_WINDOW_LOGICAL, SCALE, Target,
    custom_qualification::{Capture, original_pass},
    metal_gpu, ok, output_directory, showroom_models,
};
use crate::mule_render::showroom::{
    self, Hull,
    minimum_qualification::{self, Frames},
};

#[test]
#[ignore = "Mac owner only: SH019 original Gazelle material-only RGB"]
fn metal_gazelle_untextured_materials() {
    let output = output_directory("MERIDIAN_SH019_MATERIAL_DIR");
    let gpu = metal_gpu(Hull::Gazelle, false);
    let target = Target::new(&gpu.device, LOGICAL);
    let mut pass = original_pass(&gpu, Hull::Gazelle);
    let [front, oblique, upper] = showroom_models(Hull::Gazelle);
    let mut aft = front.clone();
    aft.orientation.yaw += std::f64::consts::PI;
    let expected = [[116, 191, 204], [185, 235, 217]];
    let mut totals = [0, 0];
    for (name, model) in [
        ("front", front),
        ("oblique", oblique),
        ("upper", upper),
        ("aft", aft),
    ] {
        ok(pass.prepare(&gpu.queue, &model, model.bounds, SCALE, FORMAT.is_srgb()));
        let crop = Crop::from_pass(&pass, &target);
        let rgb = crop.capture_preview(&gpu, &target, &pass, true);
        crop.save(output.join(format!("{name}-material-solid.ppm")), &rgb);
        for (i, colour) in expected.iter().enumerate() {
            totals[i] += rgb
                .as_chunks::<3>()
                .0
                .iter()
                .filter(|pixel| *pixel == colour)
                .count();
        }
    }
    assert!(
        totals.into_iter().all(|count| count > 32),
        "missing original material-only surface: {totals:?}"
    );
    println!("SH019 material-only exact RGB pixels: {totals:?}");
}

#[test]
#[ignore = "Mac owner only: independent SH019 production minimum Showroom RGB evidence"]
fn metal_gazelle_minimum_showroom_model_pass() {
    let output = output_directory("MERIDIAN_SH019_MINIMUM_DIR");
    let gpu = metal_gpu(Hull::Gazelle, false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH019");
    let mut gazelle = original_pass(&gpu, Hull::Gazelle);
    for (pose, Frames { solo, comparison }) in minimum_qualification::POSES
        .into_iter()
        .zip(minimum_qualification::station_frames(Hull::Gazelle))
    {
        let [left, right] = comparison.models.as_slice() else {
            panic!("expected native SH025/SH019 Compare")
        };
        assert_eq!(left.object, "SH025");
        assert_eq!(showroom::hull(right), Some(Hull::Gazelle));
        capture.solo(&format!("{pose}-solo"), &solo, &mut gazelle);
        capture.comparison(&format!("{pose}-compare"), &comparison, None, &mut gazelle);
    }
    println!(
        "SH019 minimum Showroom logical={MIN_WINDOW_LOGICAL:?} physical={:?}",
        capture.size()
    );
    capture.finish();
}

#[test]
#[ignore = "Mac owner only: SH019 production cache switch, return and mixed order RGB evidence"]
fn metal_gazelle_cache_switching() {
    let output = output_directory("MERIDIAN_SH019_CACHE_DIR");
    let gpu = metal_gpu(Hull::Gazelle, false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH019");
    let mut explorer_pass = original_pass(&gpu, Hull::Explorer);
    let mut gazelle_pass = original_pass(&gpu, Hull::Gazelle);
    let explorer = minimum_qualification::station_frames(Hull::Explorer).remove(0);
    let gazelle = minimum_qualification::station_frames(Hull::Gazelle).remove(0);
    let original = capture.solo("explorer-before", &explorer.solo, &mut explorer_pass);
    let revised = capture.solo("gazelle-before", &gazelle.solo, &mut gazelle_pass);
    assert!(
        original != revised,
        "distinct original objects rendered identical full-target RGB"
    );
    assert!(original == capture.solo("explorer-return", &explorer.solo, &mut explorer_pass));
    assert!(revised == capture.solo("gazelle-return", &gazelle.solo, &mut gazelle_pass));

    // A renderer-only fixture uses the actual station Compare pane geometry. It is separate
    // from the normal station's SH025 baseline and grants no hull exchange or gameplay state.
    let mut mixed = gazelle.comparison.clone();
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
        ("explorer-gazelle", [Hull::Explorer, Hull::Gazelle]),
        ("gazelle-explorer", [Hull::Gazelle, Hull::Explorer]),
    ] {
        let [left, right] = mixed.models.as_slice() else {
            panic!("expected mixed model panes")
        };
        assert_eq!(
            [showroom::hull(left), showroom::hull(right)],
            expected.map(Some)
        );
        capture.comparison(name, &mixed, Some(&mut explorer_pass), &mut gazelle_pass);
        let [left, right] = mixed.models.as_mut_slice() else {
            unreachable!()
        };
        std::mem::swap(&mut left.object, &mut right.object);
        std::mem::swap(&mut left.geometry_key, &mut right.geometry_key);
    }
    assert!(original == capture.solo("explorer-final", &explorer.solo, &mut explorer_pass));
    assert!(revised == capture.solo("gazelle-final", &gazelle.solo, &mut gazelle_pass));
    // Retain the original Explorer/Custom/Gazelle cache transitions.
    let mut custom_pass = original_pass(&gpu, Hull::Custom);
    let custom = minimum_qualification::station_frames(Hull::Custom).remove(0);
    let custom_rgb = capture.solo("custom-before", &custom.solo, &mut custom_pass);
    assert!(original != custom_rgb && revised != custom_rgb);
    assert!(revised == capture.solo("gazelle-after-custom", &gazelle.solo, &mut gazelle_pass));
    let mut mixed = gazelle.comparison.clone();
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
        ("custom-gazelle", [Hull::Custom, Hull::Gazelle]),
        ("gazelle-custom", [Hull::Gazelle, Hull::Custom]),
    ] {
        let [left, right] = mixed.models.as_slice() else {
            panic!("expected mixed model panes")
        };
        assert_eq!(
            [showroom::hull(left), showroom::hull(right)],
            expected.map(Some)
        );
        capture.comparison(name, &mixed, Some(&mut custom_pass), &mut gazelle_pass);
        let [left, right] = mixed.models.as_mut_slice() else {
            unreachable!()
        };
        std::mem::swap(&mut left.object, &mut right.object);
        std::mem::swap(&mut left.geometry_key, &mut right.geometry_key);
    }
    assert!(custom_rgb == capture.solo("custom-final", &custom.solo, &mut custom_pass));
    assert!(
        revised
            == capture.solo(
                "gazelle-final-after-custom",
                &gazelle.solo,
                &mut gazelle_pass
            )
    );
    println!(
        "SH019 cache return and both pane identity orders passed at {:?}",
        capture.size()
    );
    capture.finish();
}

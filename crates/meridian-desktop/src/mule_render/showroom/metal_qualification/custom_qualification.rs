//! Production minimum viewport and cross-hull GPU cache evidence shared by original hulls.
use super::{
    Crop, FORMAT, Gpu, MIN_WINDOW_LOGICAL, SCALE, Target, metal_gpu_for, ok, output_directory,
    write_new,
};
use crate::docked_screens::{Frame, ModelPreview};
use crate::mule_render::showroom::{
    self, Hull,
    minimum_qualification::{self, Frames},
    minimum_rgb,
};
use std::{fmt::Write as _, path::Path};

pub(super) struct Capture<'a> {
    object: &'static str,
    gpu: &'a Gpu,
    target: &'a Target,
    crop: Crop,
    output: &'a Path,
    docked: crate::docked_render::Pass,
    panes: String,
    measurements: String,
}

impl<'a> Capture<'a> {
    pub(super) fn new(
        gpu: &'a Gpu,
        target: &'a Target,
        output: &'a Path,
        object: &'static str,
    ) -> Self {
        Self {
            object,
            gpu,
            target,
            output,
            crop: Crop {
                origin: wgpu::Origin3d::ZERO,
                extent: target.extent,
            },
            docked: ok(crate::docked_render::Pass::new(
                &gpu.device,
                &gpu.queue,
                FORMAT,
            )),
            panes: String::from("image\tship\tx\ty\twidth\theight\tpitch\troll\tyaw\n"),
            measurements: String::from(
                "image\tship\tgeometry_key\tnonblack_pixels\tminimum_gutter_logical\n",
            ),
        }
    }

    pub(super) fn size(&self) -> [u32; 2] {
        [self.target.extent.width, self.target.extent.height]
    }

    fn frame(&mut self, name: &str, frame: &Frame) -> Vec<u8> {
        let image = format!("{name}.ppm");
        for model in &frame.models {
            let bounds = model.bounds;
            let orientation = model.orientation;
            ok(writeln!(
                self.panes,
                "{image}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                model.object,
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                orientation.pitch,
                orientation.roll,
                orientation.yaw
            ));
        }
        let size = self.size();
        ok(self.docked.prepare(
            &self.gpu.device,
            &self.gpu.queue,
            frame,
            MIN_WINDOW_LOGICAL,
            size,
            None,
        ));
        let rgb = self.crop.capture(self.gpu, self.target, |render| {
            self.docked.draw_models(render)
        });
        self.crop.save(self.output.join(image), &rgb);
        rgb
    }

    fn own_pixels(&mut self, name: &str, model: &ModelPreview, rgb: &[u8]) -> usize {
        let measured = ok(minimum_rgb::isolated_pane(
            rgb,
            self.size(),
            SCALE,
            model.bounds,
        ));
        ok(writeln!(
            self.measurements,
            "{name}.ppm\t{}\t{}\t{}\t{:.6}",
            model.object, model.geometry_key, measured.nonblack, measured.minimum_gutter_logical
        ));
        measured.nonblack
    }

    // Independent production hull pass, without the docked renderer's cache. The identical
    // pane and orientation must produce exactly the same full target in both routes.
    fn direct(&self, name: &str, model: &ModelPreview, pass: &mut showroom::Pass) -> Vec<u8> {
        assert_eq!(showroom::hull(model), Some(pass.hull()));
        ok(pass.prepare(
            &self.gpu.queue,
            model,
            model.bounds,
            SCALE,
            FORMAT.is_srgb(),
        ));
        assert!(
            pass.shimmer,
            "{} support qualification failed",
            model.object
        );
        let rgb = self
            .crop
            .capture_preview(self.gpu, self.target, pass, false);
        self.crop
            .save(self.output.join(format!("{name}-direct.ppm")), &rgb);
        rgb
    }

    pub(super) fn solo(
        &mut self,
        name: &str,
        frame: &Frame,
        reference: &mut showroom::Pass,
    ) -> Vec<u8> {
        let [model] = frame.models.as_slice() else {
            panic!("expected a single original model")
        };
        let rgb = self.frame(name, frame);
        self.own_pixels(name, model, &rgb);
        ok(minimum_rgb::exact_original(
            &rgb,
            &self.direct(name, model, reference),
            self.size(),
        ));
        rgb
    }

    pub(super) fn comparison(
        &mut self,
        name: &str,
        frame: &Frame,
        alternate: Option<&mut showroom::Pass>,
        selected: &mut showroom::Pass,
    ) -> [usize; 2] {
        let [left, right] = frame.models.as_slice() else {
            panic!("expected two model panes")
        };
        let selected_hull = Some(selected.hull());
        let references = match (showroom::hull(left), showroom::hull(right)) {
            (None, hull) if hull == selected_hull => [None, Some(selected)],
            (hull, None) if hull == selected_hull => [Some(selected), None],
            (left, right) if left == selected_hull && right != selected_hull => {
                [Some(selected), alternate]
            }
            (left, right) if right == selected_hull && left != selected_hull => {
                [alternate, Some(selected)]
            }
            _ => panic!("comparison requires independent pane references for duplicate hulls"),
        };
        self.comparison_by_pane(name, frame, references)
    }

    pub(super) fn comparison_by_pane(
        &mut self,
        name: &str,
        frame: &Frame,
        references: [Option<&mut showroom::Pass>; 2],
    ) -> [usize; 2] {
        let [left, right] = frame.models.as_slice() else {
            panic!("expected two model panes")
        };
        assert!(left.bounds.x + left.bounds.width <= right.bounds.x);
        let combined = self.frame(name, frame);
        let mut images = Vec::with_capacity(2);
        let mut counts = [0; 2];
        for (index, ((model, reference), side)) in [left, right]
            .into_iter()
            .zip(references)
            .zip(["left", "right"])
            .enumerate()
        {
            let mut isolated = frame.clone();
            isolated.models = vec![model.clone()];
            let isolated_name = if left.object == right.object {
                format!("{name}-{side}-{}", model.object)
            } else {
                format!("{name}-{}", model.object)
            };
            let rgb = self.frame(&isolated_name, &isolated);
            counts[index] = self.own_pixels(&isolated_name, model, &rgb);
            if let Some(reference) = reference {
                ok(minimum_rgb::exact_original(
                    &rgb,
                    &self.direct(&isolated_name, model, reference),
                    self.size(),
                ));
            } else {
                assert!(
                    model.object == "SH025" || showroom::hull(model) == Some(Hull::Medusa),
                    "missing independent original reference: {}",
                    model.object
                );
            }
            images.push(rgb);
        }
        ok(minimum_rgb::exact_pane_union(
            &combined,
            [&images[0], &images[1]],
            self.size(),
        ));
        println!(
            "{} {name}: full-target exact isolated RGB union at {:?}",
            self.object,
            self.size()
        );
        counts
    }

    pub(super) fn finish(self) {
        write_new(self.output.join("model-panes.tsv"), self.panes.as_bytes());
        write_new(
            self.output.join("rgb-measurements.tsv"),
            self.measurements.as_bytes(),
        );
    }
}

pub(super) fn original_pass(gpu: &Gpu, hull: Hull) -> showroom::Pass {
    ok(showroom::Pass::for_hull(
        &gpu.device,
        &gpu.queue,
        FORMAT,
        hull,
    ))
}

#[test]
#[ignore = "Mac owner only: independent SH010 production minimum Showroom RGB evidence"]
fn metal_goshawk_custom_minimum_showroom_model_pass() {
    let output = output_directory("MERIDIAN_SH010_MINIMUM_DIR");
    let gpu = metal_gpu_for("SH010", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH010");
    let mut custom = original_pass(&gpu, Hull::Custom);
    for (pose, Frames { solo, comparison }) in
        minimum_qualification::POSES
            .into_iter()
            .zip(minimum_qualification::station_frames_for(
                "Goshawk Mk II Custom",
            ))
    {
        let [left, right] = comparison.models.as_slice() else {
            panic!("expected native SH025/SH010 Compare")
        };
        assert_eq!(left.object, "SH025");
        assert_eq!(showroom::hull(right), Some(Hull::Custom));
        capture.solo(&format!("{pose}-solo"), &solo, &mut custom);
        capture.comparison(&format!("{pose}-compare"), &comparison, None, &mut custom);
    }
    println!(
        "SH010 minimum Showroom logical={MIN_WINDOW_LOGICAL:?} physical={:?}",
        capture.size()
    );
    capture.finish();
}

#[test]
#[ignore = "Mac owner only: SH010 production cache switch, return and mixed order RGB evidence"]
fn metal_goshawk_custom_cache_switching() {
    let output = output_directory("MERIDIAN_SH010_CACHE_DIR");
    let gpu = metal_gpu_for("SH010", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH010");
    let mut explorer_pass = original_pass(&gpu, Hull::Explorer);
    let mut custom_pass = original_pass(&gpu, Hull::Custom);
    let explorer = minimum_qualification::station_frames().remove(0);
    let custom = minimum_qualification::station_frames_for("Goshawk Mk II Custom").remove(0);
    let original = capture.solo("explorer-before", &explorer.solo, &mut explorer_pass);
    let revised = capture.solo("custom-before", &custom.solo, &mut custom_pass);
    assert!(
        original != revised,
        "distinct original objects rendered identical full-target RGB"
    );
    assert!(original == capture.solo("explorer-return", &explorer.solo, &mut explorer_pass));
    assert!(revised == capture.solo("custom-return", &custom.solo, &mut custom_pass));

    // A renderer-only fixture uses the actual station Compare pane geometry. It is separate
    // from the normal station's SH025 baseline and grants no hull exchange or gameplay state.
    let mut mixed = custom.comparison.clone();
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
        ("explorer-custom", [Hull::Explorer, Hull::Custom]),
        ("custom-explorer", [Hull::Custom, Hull::Explorer]),
    ] {
        let [left, right] = mixed.models.as_slice() else {
            panic!("expected mixed model panes")
        };
        assert_eq!(
            [showroom::hull(left), showroom::hull(right)],
            expected.map(Some)
        );
        capture.comparison(name, &mixed, Some(&mut explorer_pass), &mut custom_pass);
        let [left, right] = mixed.models.as_mut_slice() else {
            unreachable!()
        };
        std::mem::swap(&mut left.object, &mut right.object);
        std::mem::swap(&mut left.geometry_key, &mut right.geometry_key);
    }
    assert!(original == capture.solo("explorer-final", &explorer.solo, &mut explorer_pass));
    assert!(revised == capture.solo("custom-final", &custom.solo, &mut custom_pass));
    println!(
        "SH010 cache return and both pane identity orders passed at {:?}",
        capture.size()
    );
    capture.finish();
}

#[test]
#[ignore = "Mac owner only: independent SH009 production minimum Showroom RGB evidence"]
fn metal_goshawk_mk2_minimum_showroom_model_pass() {
    let output = output_directory("MERIDIAN_SH009_MINIMUM_DIR");
    let gpu = metal_gpu_for("SH009", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH009");
    let mut mk2 = original_pass(&gpu, Hull::Mk2);
    let mut report = String::from("pose\tsolo_visible\tleft_visible\tright_visible\tstatus\n");
    for (pose, Frames { solo, comparison }) in minimum_qualification::POSES
        .into_iter()
        .zip(minimum_qualification::station_frames_for("Goshawk Mk II"))
    {
        let [standalone] = solo.models.as_slice() else {
            panic!("expected one Mk II model")
        };
        let [left, right] = comparison.models.as_slice() else {
            panic!("expected native SH025/SH009 Compare")
        };
        assert_eq!(left.object, "SH025");
        assert_eq!(showroom::hull(right), Some(Hull::Mk2));
        let rgb = capture.solo(&format!("{pose}-solo"), &solo, &mut mk2);
        let solo_count = ok(minimum_rgb::isolated_pane(
            &rgb,
            capture.size(),
            SCALE,
            standalone.bounds,
        ))
        .nonblack;
        let [left_count, right_count] =
            capture.comparison(&format!("{pose}-compare"), &comparison, None, &mut mk2);
        ok(writeln!(
            report,
            "{pose}\t{solo_count}\t{left_count}\t{right_count}\tPASS"
        ));
    }
    write_new(output.join("rgb-qualification.tsv"), report.as_bytes());
    capture.finish();
}

#[test]
#[ignore = "Mac owner only: original-hull renderer cache switching and mixed panes"]
fn metal_goshawk_mk2_cache_switching() {
    let output = output_directory("MERIDIAN_SH009_CACHE_DIR");
    let gpu = metal_gpu_for("SH009", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH009");
    let mut explorer_pass = original_pass(&gpu, Hull::Explorer);
    let mut mk2_pass = original_pass(&gpu, Hull::Mk2);
    let explorer = minimum_qualification::station_frames().remove(0);
    let mk2 = minimum_qualification::station_frames_for("Goshawk Mk II").remove(0);
    let original = capture.solo("explorer-before", &explorer.solo, &mut explorer_pass);
    let revised = capture.solo("mk2-before", &mk2.solo, &mut mk2_pass);
    assert!(
        original != revised,
        "distinct original hulls rendered identically"
    );
    assert_eq!(
        original,
        capture.solo("explorer-return", &explorer.solo, &mut explorer_pass)
    );
    assert_eq!(
        revised,
        capture.solo("mk2-return", &mk2.solo, &mut mk2_pass)
    );

    // Renderer-only fixture, with real station pane bounds and independent source-bound passes.
    let mut mixed = mk2.comparison.clone();
    let [left, right] = mixed.models.as_mut_slice() else {
        panic!("expected comparison panes")
    };
    let [source] = explorer.solo.models.as_slice() else {
        panic!("expected one Explorer")
    };
    left.object.clone_from(&source.object);
    left.geometry_key.clone_from(&source.geometry_key);
    left.orientation = right.orientation;
    let mut report = String::from(
        "order\tleft_hull\tright_hull\tleft_visible\tright_visible\tfull_union\tindependent_references\n",
    );
    for (order, ids, expected) in [
        (
            "explorer-mk2",
            ["SH008", "SH009"],
            [Hull::Explorer, Hull::Mk2],
        ),
        (
            "mk2-explorer",
            ["SH009", "SH008"],
            [Hull::Mk2, Hull::Explorer],
        ),
    ] {
        let [left, right] = mixed.models.as_slice() else {
            panic!("expected two panes")
        };
        assert_eq!(
            [showroom::hull(left), showroom::hull(right)],
            expected.map(Some)
        );
        let [a, b] = capture.comparison(order, &mixed, Some(&mut explorer_pass), &mut mk2_pass);
        ok(writeln!(
            report,
            "{order}\t{}\t{}\t{a}\t{b}\tPASS\tPASS",
            ids[0], ids[1]
        ));
        let [left, right] = mixed.models.as_mut_slice() else {
            unreachable!()
        };
        std::mem::swap(&mut left.object, &mut right.object);
        std::mem::swap(&mut left.geometry_key, &mut right.geometry_key);
    }
    assert_eq!(
        original,
        capture.solo("explorer-final", &explorer.solo, &mut explorer_pass)
    );
    assert_eq!(revised, capture.solo("mk2-final", &mk2.solo, &mut mk2_pass));
    write_new(output.join("rgb-qualification.tsv"), report.as_bytes());
    capture.finish();
}

#[test]
#[ignore = "Mac owner only: joined Mk II/Custom mixed and duplicate pane Metal evidence"]
fn metal_goshawk_mk2_custom_cache_orders() {
    let output = output_directory("MERIDIAN_SH009_CUSTOM_CACHE_DIR");
    let gpu = metal_gpu_for("SH009", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH009");
    let mut mk2_pass = original_pass(&gpu, Hull::Mk2);
    let mut mk2_duplicate = original_pass(&gpu, Hull::Mk2);
    let mut custom_pass = original_pass(&gpu, Hull::Custom);
    let mut custom_duplicate = original_pass(&gpu, Hull::Custom);
    let mk2 = minimum_qualification::station_frames_for("Goshawk Mk II");
    let custom = minimum_qualification::station_frames_for("Goshawk Mk II Custom");
    let mut mixed = mk2[0].comparison.clone();
    let first = capture.solo("mk2-before", &mk2[0].solo, &mut mk2_pass);
    let second = capture.solo("custom-before", &custom[0].solo, &mut custom_pass);
    assert!(first != second, "distinct originals produced identical RGB");
    for (name, a, b) in [
        ("mk2-custom", &mk2, &custom),
        ("custom-mk2", &custom, &mk2),
        ("mk2-mk2", &mk2, &mk2),
        ("custom-custom", &custom, &custom),
    ] {
        let [left, right] = mixed.models.as_mut_slice() else {
            panic!("expected two station Compare panes")
        };
        for (pane, pose) in [(left, &a[0]), (right, &b[2])] {
            let [model] = pose.solo.models.as_slice() else {
                panic!("expected one selected hull")
            };
            *pane = ModelPreview {
                bounds: pane.bounds,
                ..model.clone()
            };
        }
        assert_ne!(mixed.models[0].orientation, mixed.models[1].orientation);
        let references = match (
            showroom::hull(&mixed.models[0]),
            showroom::hull(&mixed.models[1]),
        ) {
            (Some(Hull::Mk2), Some(Hull::Custom)) => [Some(&mut mk2_pass), Some(&mut custom_pass)],
            (Some(Hull::Custom), Some(Hull::Mk2)) => [Some(&mut custom_pass), Some(&mut mk2_pass)],
            (Some(Hull::Mk2), Some(Hull::Mk2)) => [Some(&mut mk2_pass), Some(&mut mk2_duplicate)],
            (Some(Hull::Custom), Some(Hull::Custom)) => {
                [Some(&mut custom_pass), Some(&mut custom_duplicate)]
            }
            _ => panic!("unexpected source in Mk II/Custom comparison"),
        };
        capture.comparison_by_pane(name, &mixed, references);
    }
    assert_eq!(
        first,
        capture.solo("mk2-return", &mk2[0].solo, &mut mk2_pass)
    );
    assert_eq!(
        second,
        capture.solo("custom-return", &custom[0].solo, &mut custom_pass)
    );
    capture.finish();
}

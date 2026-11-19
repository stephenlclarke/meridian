//! SH010's production minimum viewport and cross-hull GPU cache evidence.
use super::{
    Crop, FORMAT, Gpu, MIN_WINDOW_LOGICAL, SCALE, Target, metal_gpu, ok, output_directory,
    write_new,
};
use crate::docked_screens::{Frame, ModelPreview};
use crate::mule_render::showroom::{
    self, Hull,
    minimum_qualification::{self, Frames},
    minimum_rgb,
};
use std::{fmt::Write as _, path::Path};

struct Capture<'a> {
    gpu: &'a Gpu,
    target: &'a Target,
    crop: Crop,
    output: &'a Path,
    docked: crate::docked_render::Pass,
    panes: String,
    measurements: String,
}

impl<'a> Capture<'a> {
    fn new(gpu: &'a Gpu, target: &'a Target, output: &'a Path) -> Self {
        Self {
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

    fn size(&self) -> [u32; 2] {
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

    fn own_pixels(&mut self, name: &str, model: &ModelPreview, rgb: &[u8]) {
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

    fn solo(&mut self, name: &str, frame: &Frame, reference: &mut showroom::Pass) -> Vec<u8> {
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

    fn comparison(
        &mut self,
        name: &str,
        frame: &Frame,
        mut explorer: Option<&mut showroom::Pass>,
        custom: &mut showroom::Pass,
    ) {
        let [left, right] = frame.models.as_slice() else {
            panic!("expected two model panes")
        };
        assert!(left.bounds.x + left.bounds.width <= right.bounds.x);
        let combined = self.frame(name, frame);
        let mut images = Vec::with_capacity(2);
        for model in [left, right] {
            let mut isolated = frame.clone();
            isolated.models = vec![model.clone()];
            let isolated_name = format!("{name}-{}", model.object);
            let rgb = self.frame(&isolated_name, &isolated);
            self.own_pixels(&isolated_name, model, &rgb);
            let reference = match showroom::hull(model) {
                Some(Hull::Custom) => Some(&mut *custom),
                Some(Hull::Explorer) => explorer.as_deref_mut(),
                Some(Hull::Mk2) => panic!("unexpected SH009 in SH010 qualification"),
                None => {
                    assert_eq!(model.object, "SH025");
                    None
                }
            };
            if let Some(reference) = reference {
                ok(minimum_rgb::exact_original(
                    &rgb,
                    &self.direct(&isolated_name, model, reference),
                    self.size(),
                ));
            }
            images.push(rgb);
        }
        ok(minimum_rgb::exact_pane_union(
            &combined,
            [&images[0], &images[1]],
            self.size(),
        ));
        println!(
            "SH010 {name}: full-target exact isolated RGB union at {:?}",
            self.size()
        );
    }

    fn finish(self) {
        write_new(self.output.join("model-panes.tsv"), self.panes.as_bytes());
        write_new(
            self.output.join("rgb-measurements.tsv"),
            self.measurements.as_bytes(),
        );
    }
}

fn original_pass(gpu: &Gpu, hull: Hull) -> showroom::Pass {
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
    let gpu = metal_gpu(Hull::Custom, false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output);
    let mut custom = original_pass(&gpu, Hull::Custom);
    for (pose, Frames { solo, comparison }) in minimum_qualification::POSES
        .into_iter()
        .zip(minimum_qualification::station_frames(Hull::Custom))
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
    let gpu = metal_gpu(Hull::Custom, false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output);
    let mut explorer_pass = original_pass(&gpu, Hull::Explorer);
    let mut custom_pass = original_pass(&gpu, Hull::Custom);
    let explorer = minimum_qualification::station_frames(Hull::Explorer).remove(0);
    let custom = minimum_qualification::station_frames(Hull::Custom).remove(0);
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

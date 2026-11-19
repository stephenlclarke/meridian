//! Mac-owner RGB evidence for ordinary stock SH030; independent shared visual reference.
use super::{Crop, FORMAT, MIN_WINDOW_LOGICAL, SCALE, Target, metal_gpu_for, ok, output_directory};
use crate::{
    authored_hull::stock_visual_for,
    mule_render::showroom::{AuthoredPass, minimum_qualification, minimum_rgb},
};

#[test]
#[ignore = "Mac owner only: stock Sparrowhawk ordinary Showroom RGB evidence"]
fn metal_stock_sparrowhawk_normal_showroom() {
    let output = output_directory("MERIDIAN_SH030_STOCK_SHOWROOM_DIR");
    let gpu = metal_gpu_for("SH030 stock", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let crop = Crop {
        origin: wgpu::Origin3d::ZERO,
        extent: target.extent,
    };
    let size = [target.extent.width, target.extent.height];
    let mut docked = ok(crate::docked_render::Pass::new(
        &gpu.device,
        &gpu.queue,
        FORMAT,
    ));
    let mut prior = None;
    for (pose, frames) in minimum_qualification::POSES
        .into_iter()
        .zip(minimum_qualification::station_frames_for("Sparrowhawk"))
    {
        let [solo] = frames.solo.models.as_slice() else {
            panic!("expected ordinary SH030 solo");
        };
        let [baseline, selected] = frames.comparison.models.as_slice() else {
            panic!("expected ordinary SH025/SH030 Compare");
        };
        assert_eq!(baseline.object, "SH025");
        assert_eq!(solo.object, "SH030");
        assert_eq!(selected.object, "SH030");
        let visual = ok(stock_visual_for(solo))
            .unwrap_or_else(|| panic!("missing own original stock visual"));
        let mut reference = ok(AuthoredPass::new(
            &gpu.device,
            &gpu.queue,
            FORMAT,
            solo,
            visual,
        ));
        ok(docked.prepare(
            &gpu.device,
            &gpu.queue,
            &frames.solo,
            MIN_WINDOW_LOGICAL,
            size,
            None,
        ));
        let rgb = crop.capture(&gpu, &target, |pass| docked.draw_models(pass));
        ok(minimum_rgb::isolated_pane(&rgb, size, SCALE, solo.bounds));
        ok(reference.prepare(&gpu.queue, solo, solo.bounds, SCALE, FORMAT.is_srgb()));
        let direct = crop.capture(&gpu, &target, |pass| reference.draw(pass));
        ok(minimum_rgb::exact_original(&rgb, &direct, size));
        crop.save(output.join(format!("{pose}-solo.ppm")), &rgb);
        crop.save(output.join(format!("{pose}-solo-direct.ppm")), &direct);
        if let Some(previous) = prior.replace(rgb) {
            assert_ne!(
                previous, direct,
                "ordinary rotation did not alter stock RGB"
            );
        }

        ok(docked.prepare(
            &gpu.device,
            &gpu.queue,
            &frames.comparison,
            MIN_WINDOW_LOGICAL,
            size,
            None,
        ));
        let combined = crop.capture(&gpu, &target, |pass| docked.draw_models(pass));
        let mut images = Vec::new();
        for model in [baseline, selected] {
            let mut isolated = frames.comparison.clone();
            isolated.models = vec![model.clone()];
            ok(docked.prepare(
                &gpu.device,
                &gpu.queue,
                &isolated,
                MIN_WINDOW_LOGICAL,
                size,
                None,
            ));
            let image = crop.capture(&gpu, &target, |pass| docked.draw_models(pass));
            ok(minimum_rgb::isolated_pane(
                &image,
                size,
                SCALE,
                model.bounds,
            ));
            if model.object == "SH030" {
                ok(reference.prepare(&gpu.queue, model, model.bounds, SCALE, FORMAT.is_srgb()));
                let direct = crop.capture(&gpu, &target, |pass| reference.draw(pass));
                ok(minimum_rgb::exact_original(&image, &direct, size));
                crop.save(
                    output.join(format!("{pose}-compare-SH030-direct.ppm")),
                    &direct,
                );
            }
            crop.save(
                output.join(format!("{pose}-compare-{}.ppm", model.object)),
                &image,
            );
            images.push(image);
        }
        ok(minimum_rgb::exact_pane_union(
            &combined,
            [&images[0], &images[1]],
            size,
        ));
        crop.save(output.join(format!("{pose}-compare.ppm")), &combined);
    }
}

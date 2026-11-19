//! Mac-owner stock Kite and shared two-identity GPU cache evidence.
use super::{
    Crop, FORMAT, MIN_WINDOW_LOGICAL, SCALE, Target, metal_gpu_for, ok, output_directory,
    sparrowhawk_stock_qualification::qualify_stock,
};
use crate::{
    authored_hull::stock_visual_for,
    mule_render::showroom::{AuthoredPass, minimum_qualification, minimum_rgb},
};

#[test]
#[ignore = "Mac owner only: stock Kite ordinary Showroom RGB evidence"]
fn metal_stock_kite_normal_showroom() {
    qualify_stock(
        "Kite Interceptor",
        "SH031",
        "MERIDIAN_SH031_STOCK_SHOWROOM_DIR",
    );
}

#[test]
#[ignore = "Mac owner only: independently rendered stock pair and duplicate pane cache evidence"]
fn metal_stock_sparrowhawk_kite_cache_comparison() {
    qualify_stock_pair(
        ("Sparrowhawk", "SH030", "sparrowhawk"),
        ("Kite Interceptor", "SH031", "kite"),
        "MERIDIAN_SH030_SH031_STOCK_CACHE_DIR",
    );
}

pub(super) fn qualify_stock_pair(
    first: (&str, &str, &str),
    second: (&str, &str, &str),
    variable: &str,
) {
    let output = output_directory(variable);
    let gpu = metal_gpu_for(&format!("{}-{} stock", first.1, second.1), false);
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
    let a = minimum_qualification::station_frames_for(first.0);
    let b = minimum_qualification::station_frames_for(second.0);
    let mut solos = Vec::new();
    for (name, frame) in [
        (format!("{}-before", first.2), &a[0].solo),
        (format!("{}-before", second.2), &b[0].solo),
    ] {
        ok(docked.prepare(
            &gpu.device,
            &gpu.queue,
            frame,
            MIN_WINDOW_LOGICAL,
            size,
            None,
        ));
        let rgb = crop.capture(&gpu, &target, |pass| docked.draw_models(pass));
        ok(minimum_rgb::isolated_pane(
            &rgb,
            size,
            SCALE,
            frame.models[0].bounds,
        ));
        crop.save(output.join(format!("{name}.ppm")), &rgb);
        solos.push(rgb);
    }
    assert_ne!(
        solos[0], solos[1],
        "different original stock ships rendered identical RGB"
    );
    let mut frames =
        minimum_qualification::stock_pair_compare_frames((first.0, first.1), (second.0, second.1));
    for (frame, upper) in frames[2..].iter_mut().zip([
        a[2].solo.models[0].orientation,
        b[2].solo.models[0].orientation,
    ]) {
        frame.models[1].orientation = upper;
        assert_ne!(frame.models[0].orientation, frame.models[1].orientation);
    }
    for (name, frame) in [
        format!("{}-{}", first.2, second.2),
        format!("{}-{}", second.2, first.2),
        format!("{}-{}", first.2, first.2),
        format!("{}-{}", second.2, second.2),
    ]
    .into_iter()
    .zip(frames)
    {
        ok(docked.prepare(
            &gpu.device,
            &gpu.queue,
            &frame,
            MIN_WINDOW_LOGICAL,
            size,
            None,
        ));
        let combined = crop.capture(&gpu, &target, |pass| docked.draw_models(pass));
        let mut images = Vec::new();
        for (side, model) in ["left", "right"].into_iter().zip(&frame.models) {
            let visual =
                ok(stock_visual_for(model)).unwrap_or_else(|| panic!("missing stock original"));
            // Each pane, including duplicate IDs, gets an independent original reference pass.
            let mut reference = ok(AuthoredPass::new(
                &gpu.device,
                &gpu.queue,
                FORMAT,
                model,
                visual,
            ));
            ok(reference.prepare(&gpu.queue, model, model.bounds, SCALE, FORMAT.is_srgb()));
            let direct = crop.capture(&gpu, &target, |pass| reference.draw(pass));
            let mut isolated = frame.clone();
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
            ok(minimum_rgb::exact_original(&image, &direct, size));
            crop.save(output.join(format!("{name}-{side}.ppm")), &image);
            crop.save(output.join(format!("{name}-{side}-direct.ppm")), &direct);
            images.push(image);
        }
        ok(minimum_rgb::exact_pane_union(
            &combined,
            [&images[0], &images[1]],
            size,
        ));
        crop.save(output.join(format!("{name}.ppm")), &combined);
    }
    for (index, (name, frame)) in [
        (format!("{}-return", first.2), &a[0].solo),
        (format!("{}-return", second.2), &b[0].solo),
    ]
    .into_iter()
    .enumerate()
    {
        ok(docked.prepare(
            &gpu.device,
            &gpu.queue,
            frame,
            MIN_WINDOW_LOGICAL,
            size,
            None,
        ));
        let rgb = crop.capture(&gpu, &target, |pass| docked.draw_models(pass));
        assert!(
            rgb == solos[index],
            "stock cache changed returning original {name}"
        );
        crop.save(output.join(format!("{name}.ppm")), &rgb);
    }
}

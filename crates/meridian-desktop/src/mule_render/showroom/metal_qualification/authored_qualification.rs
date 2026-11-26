//! Mac-owner actual generic authored Showroom payload, no original hull substitution.
use super::{Crop, FORMAT, MIN_WINDOW_LOGICAL, SCALE, Target, metal_gpu_for, ok, output_directory};
use crate::{
    authored_hull::{AuthoredHulls, PackagePath, fixture_path},
    docked::Docked,
    docked_screens::{Input, Key, Screen, Viewport},
    profile::tests::{TestDirectory, open},
};
use std::sync::Arc;

#[test]
#[ignore = "Mac owner only: explicit authored package RGB evidence"]
fn metal_authored_package_geometry_and_material_changes_render_in_actual_showroom() {
    let output = output_directory("MERIDIAN_AUTHORED_HULL_EVIDENCE_DIR");
    let gpu = metal_gpu_for("authored package", false);
    let target = Target::new(&gpu.device, MIN_WINDOW_LOGICAL);
    let crop = Crop {
        origin: wgpu::Origin3d::ZERO,
        extent: target.extent,
    };
    let size = [target.extent.width, target.extent.height];
    let mut images = Vec::new();
    for (changed, name) in [(false, "initial"), (true, "changed")] {
        let hulls = Arc::new(ok(AuthoredHulls::load(&[PackagePath::new(
            fixture_path(changed),
            false,
        )])));
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(ok(profile.new_commander(97201)));
        let save = ok(profile.world.save_bytes());
        let mut menu = ok(Docked::with_authored(&profile.world, hulls.clone()));
        ok(menu.open(&profile.world, Screen::Showroom));
        let [width, height] = MIN_WINDOW_LOGICAL;
        let viewport = Viewport::new(width, height).unwrap_or_else(|| panic!("viewport"));
        for input in [
            Input::Key(Key::Search),
            Input::Text("TEST901".into()),
            Input::Key(Key::Confirm),
            Input::Key(Key::Compare),
            Input::Key(Key::ResetView),
            Input::Key(Key::TurnRight),
            Input::Key(Key::TurnRight),
        ] {
            let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
            menu.presented(frame);
            ok(menu.input(&mut profile, input));
        }
        let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
        let [model] = frame.models.as_slice() else {
            panic!("expected actual single authored pane");
        };
        assert_eq!(model.object, "TEST901");
        let mut pass = ok(crate::docked_render::Pass::with_authored(
            &gpu.device,
            &gpu.queue,
            FORMAT,
            hulls,
        ));
        ok(pass.prepare(
            &gpu.device,
            &gpu.queue,
            &frame,
            MIN_WINDOW_LOGICAL,
            size,
            None,
        ));
        let rgb = crop.capture(&gpu, &target, |render| pass.draw_models(render));
        ok(crate::mule_render::showroom::minimum_rgb::isolated_pane(
            &rgb,
            size,
            SCALE,
            model.bounds,
        ));
        assert_eq!(
            rgb,
            crop.capture(&gpu, &target, |render| pass.draw_models(render))
        );
        crop.save(output.join(format!("{name}.ppm")), &rgb);
        assert_eq!(ok(profile.world.save_bytes()), save);
        images.push(rgb);
    }
    assert_ne!(
        images[0], images[1],
        "actual changed geometry/material rendered identical RGB"
    );
}

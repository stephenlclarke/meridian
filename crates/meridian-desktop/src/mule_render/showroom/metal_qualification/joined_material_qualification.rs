//! Direct raw RGB proof for every original no-map face in SH012 and SH019.
use super::{Crop, FORMAT, Gpu, Target, metal_gpu, ok, output_directory, write_new};
use crate::mule_render::{
    Triangle, VisualVertex, gpu as renderer,
    showroom::{Hull, Uniforms, material_witness, shader},
};
use std::{fmt::Write as _, path::Path};

const COLUMNS: usize = 40;
const TILE: usize = 8;

fn raw_face_grid(hull: Hull, gpu: &Gpu, output: &Path) -> usize {
    let target = Target::new(&gpu.device, [160.0, 128.0]);
    let [width, height] = [target.extent.width as usize, target.extent.height as usize];
    assert_eq!([width, height], [320, 256]);
    let faces = material_witness::material_faces(hull);
    let mut vertices = Vec::<VisualVertex>::with_capacity(faces.len() * 3);
    for (slot, (_, face, _)) in faces.iter().enumerate() {
        let [x, y] = [
            slot % COLUMNS * TILE + TILE / 2,
            slot / COLUMNS * TILE + TILE / 2,
        ];
        assert!(x + 3 < width && y + 3 < height);
        // Only move the original triangle corners to an isolated test tile. Retain each
        // original colour, normal, emissive bit, texture marker and UV, without restyling.
        for (original, [px, py]) in face
            .iter()
            .zip([[x - 3, y + 3], [x + 3, y + 3], [x, y - 3]])
        {
            let position = [
                2.0 * (2.0 * px as f32 / width as f32 - 1.0),
                2.0 * (1.0 - 2.0 * py as f32 / height as f32),
                2.0,
            ];
            vertices.push(VisualVertex {
                position,
                ..*original
            });
        }
    }
    let no_pieces = [<Triangle as bytemuck::Zeroable>::zeroed()];
    let hardware = ok(renderer::Hardware::new(
        &gpu.device,
        &gpu.queue,
        FORMAT,
        renderer::Inputs {
            shader: &shader(hull),
            uniform_size: hull.uniform_size() as u64,
            points: &no_pieces,
            visual: &vertices,
        },
    ));
    let mut uniforms = <Uniforms as bytemuck::Zeroable>::zeroed();
    uniforms.projection = [1.0, 1.0, 0.1, 128.0];
    uniforms.origin[3] = 1.0;
    uniforms.x[0] = 1.0;
    uniforms.y[1] = 1.0;
    uniforms.z[2] = 1.0;
    gpu.queue
        .write_buffer(&hardware.uniform, 0, bytemuck::bytes_of(&uniforms));
    let crop = Crop {
        origin: wgpu::Origin3d::ZERO,
        extent: target.extent,
    };
    let rgb = crop.capture(gpu, &target, |render| {
        render.set_pipeline(&hardware.solid);
        render.set_bind_group(0, &hardware.group, &[]);
        render.set_vertex_buffer(0, hardware.vertices.slice(..));
        render.draw(0..vertices.len() as u32, 0..1);
    });
    let id = hull.object_id();
    crop.save(output.join(format!("{id}-material-faces.ppm")), &rgb);
    let mut receipt = String::from("source_face\tx\ty\traw_expected_rgb\tactual_rgb\n");
    let mut failures = 0;
    for (slot, (index, _, expected)) in faces.iter().enumerate() {
        let [x, y] = [
            slot % COLUMNS * TILE + TILE / 2,
            slot / COLUMNS * TILE + TILE / 2,
        ];
        let offset = (y * width + x) * 3;
        let actual = &rgb[offset..offset + 3];
        ok(writeln!(
            receipt,
            "{index}\t{x}\t{y}\t{},{},{}\t{},{},{}",
            expected[0], expected[1], expected[2], actual[0], actual[1], actual[2]
        ));
        if actual != expected {
            failures += 1;
        }
    }
    write_new(
        output.join(format!("{id}-material-faces.tsv")),
        receipt.as_bytes(),
    );
    failures
}

#[test]
#[ignore = "Mac owner only: raw source material colour on every joined Sable and Gazelle no-map face"]
fn metal_sable_gazelle_material_only_faces_exact_raw_rgb() {
    let output = output_directory("MERIDIAN_SH012_SH019_MATERIAL_DIR");
    let gpu = metal_gpu(Hull::SableLiner, false);
    let failures = [Hull::SableLiner, Hull::Gazelle].map(|hull| raw_face_grid(hull, &gpu, &output));
    assert_eq!(
        failures,
        [0, 0],
        "SH012 and SH019 per-face raw RGB failures"
    );
}

#[test]
#[ignore = "Mac owner only: actual pinned Sable/Gazelle raw material renderer cache orders"]
fn metal_sable_gazelle_cache_orders() {
    use super::custom_qualification::{Capture, original_pass};
    use crate::mule_render::showroom::minimum_qualification;
    let output = output_directory("MERIDIAN_SH012_SH019_CACHE_DIR");
    let gpu = metal_gpu(Hull::SableLiner, false);
    let target = Target::new(&gpu.device, super::MIN_WINDOW_LOGICAL);
    let mut capture = Capture::new(&gpu, &target, &output, "SH012-SH019");
    let mut sable = original_pass(&gpu, Hull::SableLiner);
    let mut gazelle = original_pass(&gpu, Hull::Gazelle);
    let a = minimum_qualification::station_frames(Hull::SableLiner).remove(0);
    let b = minimum_qualification::station_frames(Hull::Gazelle).remove(0);
    let first = capture.solo("sable-before", &a.solo, &mut sable);
    let second = capture.solo("gazelle-before", &b.solo, &mut gazelle);
    assert!(
        first != second,
        "distinct material-only hulls rendered identically"
    );
    let [sable_gazelle, gazelle_sable] = minimum_qualification::material_sibling_frames();
    capture.comparison(
        "sable-gazelle",
        &sable_gazelle,
        Some(&mut sable),
        &mut gazelle,
    );
    capture.comparison(
        "gazelle-sable",
        &gazelle_sable,
        Some(&mut sable),
        &mut gazelle,
    );
    assert!(first == capture.solo("sable-return", &a.solo, &mut sable));
    assert!(second == capture.solo("gazelle-return", &b.solo, &mut gazelle));
    capture.finish();
}

//! Independent raw and production styled RGB proof for every original no-map face.
use super::{Crop, FORMAT, Gpu, Target, metal_gpu, ok, output_directory, write_new};
use crate::mule_render::{
    Triangle, VisualVertex, gpu as renderer,
    showroom::{Hull, Uniforms, material_witness, shader},
};
use std::{fmt::Write as _, path::Path};

const COLUMNS: usize = 40;
const TILE: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EvidenceMode {
    Raw,
    Styled,
}

fn face_grid(hull: Hull, gpu: &Gpu, output: &Path, mode: EvidenceMode) -> usize {
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
        // original colour, normal, emissive bit, texture marker and UV before upload.
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
    let source_shader = shader(hull);
    let inputs = renderer::Inputs {
        shader: &source_shader,
        uniform_size: hull.uniform_size() as u64,
        points: &no_pieces,
        visual: &vertices,
    };
    let hardware = ok(match mode {
        EvidenceMode::Raw => renderer::Hardware::with_upload(
            &gpu.device,
            &gpu.queue,
            FORMAT,
            inputs,
            renderer::VisualUpload::Raw,
        ),
        EvidenceMode::Styled => renderer::Hardware::new(&gpu.device, &gpu.queue, FORMAT, inputs),
    });
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
    let name = if mode == EvidenceMode::Raw {
        format!("{id}-material-faces")
    } else {
        format!("{id}-styled-material-faces")
    };
    crop.save(output.join(format!("{name}.ppm")), &rgb);
    let mut receipt = String::from(if mode == EvidenceMode::Raw {
        "source_face\tx\ty\traw_expected_rgb\tactual_rgb\n"
    } else {
        "source_face\tx\ty\tsource_raw_rgb\tstyled_expected_rgb\tactual_rgb\n"
    });
    let mut failures = 0;
    for (slot, (index, _, raw)) in faces.iter().enumerate() {
        let [x, y] = [
            slot % COLUMNS * TILE + TILE / 2,
            slot / COLUMNS * TILE + TILE / 2,
        ];
        let offset = (y * width + x) * 3;
        let actual = &rgb[offset..offset + 3];
        let expected = if mode == EvidenceMode::Raw {
            *raw
        } else {
            material_witness::styled_rgb(*raw)
        };
        ok(write!(receipt, "{index}\t{x}\t{y}\t"));
        if mode == EvidenceMode::Styled {
            ok(write!(receipt, "{},{},{}\t", raw[0], raw[1], raw[2]));
        }
        ok(writeln!(
            receipt,
            "{},{},{}\t{},{},{}",
            expected[0], expected[1], expected[2], actual[0], actual[1], actual[2]
        ));
        if actual != expected.as_slice() {
            failures += 1;
        }
    }
    write_new(output.join(format!("{name}.tsv")), receipt.as_bytes());
    failures
}

#[test]
#[ignore = "Mac owner only: raw source material colour on every joined Sable and Gazelle no-map face"]
fn metal_sable_gazelle_material_only_faces_exact_raw_rgb() {
    let output = output_directory("MERIDIAN_SH012_SH019_MATERIAL_DIR");
    let gpu = metal_gpu(Hull::SableLiner, false);
    let failures = [Hull::SableLiner, Hull::Gazelle]
        .map(|hull| face_grid(hull, &gpu, &output, EvidenceMode::Raw));
    assert_eq!(
        failures,
        [0, 0],
        "SH012 and SH019 per-face raw RGB failures"
    );
}

#[test]
#[ignore = "Mac owner only: production PR76 styled RGB on every Sable and Gazelle no-map face"]
fn metal_sable_gazelle_material_only_faces_exact_styled_rgb() {
    let output = output_directory("MERIDIAN_SH012_SH019_STYLED_MATERIAL_DIR");
    let gpu = metal_gpu(Hull::SableLiner, false);
    let failures = [Hull::SableLiner, Hull::Gazelle]
        .map(|hull| face_grid(hull, &gpu, &output, EvidenceMode::Styled));
    assert_eq!(
        failures,
        [0, 0],
        "SH012 and SH019 per-face production styled RGB failures"
    );
}

#[test]
#[ignore = "Mac owner only: actual pinned Sable/Gazelle raw material renderer cache orders"]
fn metal_sable_gazelle_cache_orders() {
    let output = output_directory("MERIDIAN_SH012_SH019_CACHE_DIR");
    cache_orders(&output, EvidenceMode::Raw);
}

#[test]
#[ignore = "Mac owner only: actual styled Sable/Gazelle pinned station cache orders"]
fn metal_sable_gazelle_styled_cache_orders() {
    let output = output_directory("MERIDIAN_SH012_SH019_STYLED_CACHE_DIR");
    cache_orders(&output, EvidenceMode::Styled);
}

fn cache_orders(output: &Path, mode: EvidenceMode) {
    use super::custom_qualification::{Capture, original_pass, original_pass_raw};
    use crate::mule_render::showroom::minimum_qualification;
    let gpu = metal_gpu(Hull::SableLiner, false);
    let target = Target::new(&gpu.device, super::MIN_WINDOW_LOGICAL);
    let (mut capture, mut sable, mut gazelle) = match mode {
        EvidenceMode::Raw => (
            Capture::new_raw(&gpu, &target, output, "SH012-SH019"),
            original_pass_raw(&gpu, Hull::SableLiner),
            original_pass_raw(&gpu, Hull::Gazelle),
        ),
        EvidenceMode::Styled => (
            Capture::new(&gpu, &target, output, "SH012-SH019"),
            original_pass(&gpu, Hull::SableLiner),
            original_pass(&gpu, Hull::Gazelle),
        ),
    };
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
    if mode == EvidenceMode::Styled {
        visible_styled_materials(Hull::SableLiner, &gpu, &mut sable, output);
        visible_styled_materials(Hull::Gazelle, &gpu, &mut gazelle, output);
    }
}

fn visible_styled_materials(
    hull: Hull,
    gpu: &Gpu,
    pass: &mut crate::mule_render::showroom::Pass,
    output: &Path,
) {
    use super::{LOGICAL, SCALE, showroom_models};
    let target = Target::new(&gpu.device, LOGICAL);
    let [front, oblique, upper] = showroom_models(hull);
    let mut aft = front.clone();
    aft.orientation.yaw += std::f64::consts::PI;
    let expected = [[100, 175, 188], [100, 150, 132]];
    let mut totals = [0; 2];
    let mut receipt = String::from("pose\tstyled_glazing_pixels\tstyled_emitter_pixels\n");
    for (name, model) in [
        ("front", front),
        ("oblique", oblique),
        ("upper", upper),
        ("aft", aft),
    ] {
        ok(pass.prepare(&gpu.queue, &model, model.bounds, SCALE, FORMAT.is_srgb()));
        let crop = Crop::from_pass(pass, &target);
        let rgb = crop.capture_preview(gpu, &target, pass, true);
        crop.save(
            output.join(format!("{}-{name}-styled-solid.ppm", hull.object_id())),
            &rgb,
        );
        let counts = expected.map(|colour| {
            rgb.as_chunks::<3>()
                .0
                .iter()
                .filter(|pixel| **pixel == colour)
                .count()
        });
        totals[0] += counts[0];
        totals[1] += counts[1];
        ok(writeln!(receipt, "{name}\t{}\t{}", counts[0], counts[1]));
    }
    write_new(
        output.join(format!("{}-styled-visible.tsv", hull.object_id())),
        receipt.as_bytes(),
    );
    assert!(
        totals.into_iter().all(|count| count > 0),
        "{} missing visible styled material: {totals:?}",
        hull.object_id()
    );
}

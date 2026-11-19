//! Explicit Metal evidence from the production Goshawk Showroom passes.
//! The Mac owner runs SH009's three ignored cases with separate new directories:
//! `MERIDIAN_SH009_EVIDENCE_DIR` (same-pose normal/repeat/phase/solid),
//! `MERIDIAN_SH009_MINIMUM_DIR` (solo and comparison panes), and
//! `MERIDIAN_SH009_CACHE_DIR` (same-hull return and renderer-only mixed panes).
//! The existing same-pose PPM verifier applies the same subtle-shimmer bounds to either hull.
use super::minimum_rgb::{exact_original, exact_pane_union, isolated_pane};
use super::{Hull, Pass, Uniforms};
use crate::{
    app::MIN_WINDOW_LOGICAL,
    docked::Docked,
    docked_screens::{Input, Key, ModelPreview, Screen, Viewport},
    profile::{
        Profile,
        tests::{TestDirectory, open},
    },
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

const LOGICAL: [f64; 2] = [1280.0, 720.0];
const SCALE: [f64; 2] = [2.0; 2];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const TIMEOUT: Duration = Duration::from_secs(30);
const WARMUP: usize = 3;
const SAMPLES: u32 = 16;
const POSES: [&str; 3] = ["front", "oblique", "upper"];
const PHASE_SECONDS: f32 = std::f32::consts::PI / 0.7;

mod custom_qualification;

fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("{error:?}"))
}

fn showroom_models(hull: Hull) -> [ModelPreview; 3] {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(97008)));
    let before = ok(profile.world.save_bytes());
    let mut menu = ok(Docked::new(&profile.world));
    ok(menu.open(&profile.world, Screen::Showroom));
    let viewport = Viewport::new(LOGICAL[0], LOGICAL[1])
        .unwrap_or_else(|| panic!("invalid qualification viewport"));
    let input = |menu: &mut Docked, profile: &mut Profile, input| {
        let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
        menu.presented(frame);
        ok(menu.input(profile, input));
    };
    let preview = |menu: &mut Docked, profile: &Profile| {
        let [model] = ok(menu.build_frame(&profile.world, viewport, 0.0))
            .models
            .try_into()
            .unwrap_or_else(|_| panic!("expected exactly one production Showroom model"));
        assert_eq!(Hull::from_model(&model), Some(hull));
        model
    };
    for action in [
        Input::Key(Key::Search),
        Input::Text(super::minimum_qualification::search_name(hull).into()),
        Input::Key(Key::Confirm),
        Input::Key(Key::Compare),
        Input::Key(Key::ResetView),
    ] {
        input(&mut menu, &mut profile, action);
    }
    let front = preview(&mut menu, &profile);
    // ResetView preserves the initial yaw axis.
    for _ in 0..2 {
        input(&mut menu, &mut profile, Input::Key(Key::TurnRight));
    }
    let oblique = preview(&mut menu, &profile);
    // The next axis after yaw is pitch. Move the viewer above the source hull.
    input(&mut menu, &mut profile, Input::Key(Key::NextAxis));
    for _ in 0..4 {
        input(&mut menu, &mut profile, Input::Key(Key::TurnLeft));
    }
    let upper = preview(&mut menu, &profile);
    assert_eq!(front.bounds, oblique.bounds);
    assert_eq!(front.bounds, upper.bounds);
    assert_eq!(before, ok(profile.world.save_bytes()));
    [front, oblique, upper]
}

fn hull_local_direction(pass: &Pass, model: &ModelPreview) -> [f64; 3] {
    let width = model.bounds.width * SCALE[0];
    let height = model.bounds.height * SCALE[1];
    let data = ok(pass
        .view
        .uniforms(model.orientation, width / height, FORMAT.is_srgb()));
    let direction = [
        f64::from(data.camera[0]) - pass.view.center.x,
        f64::from(data.camera[1]) - pass.view.center.y,
        f64::from(data.camera[2]) - pass.view.center.z,
    ];
    let norm = direction
        .iter()
        .map(|component| component * component)
        .sum::<f64>()
        .sqrt();
    assert!(norm.is_finite() && norm > 0.0);
    direction.map(|component| component / norm)
}

fn write_new(path: PathBuf, data: &[u8]) {
    let mut file = ok(OpenOptions::new().write(true).create_new(true).open(path));
    ok(file.write_all(data));
}

fn read_buffer(
    device: &wgpu::Device,
    submission: wgpu::SubmissionIndex,
    buffer: &wgpu::Buffer,
) -> Vec<u8> {
    let (send, receive) = std::sync::mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        let _ = send.send(result);
    });
    ok(device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: Some(TIMEOUT),
    }));
    ok(ok(receive.recv_timeout(TIMEOUT)));
    let bytes = buffer.get_mapped_range(..).to_vec();
    buffer.unmap();
    bytes
}

struct Target {
    pixels: wgpu::Texture,
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    extent: wgpu::Extent3d,
}
impl Target {
    fn new(device: &wgpu::Device, logical: [f64; 2]) -> Self {
        let extent = wgpu::Extent3d {
            width: (logical[0] * SCALE[0]) as u32,
            height: (logical[1] * SCALE[1]) as u32,
            depth_or_array_layers: 1,
        };
        let texture = |format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Goshawk qualification target"),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let pixels = texture(
            FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let depth = texture(
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .create_view(&Default::default());
        let color = pixels.create_view(&Default::default());
        Self {
            pixels,
            color,
            depth,
            extent,
        }
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        load: bool,
        preserve_depth: bool,
        timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
        draw: impl FnOnce(&mut wgpu::RenderPass<'_>),
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Goshawk Showroom evidence"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.color,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: if load {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: if load {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(1.0)
                    },
                    store: if preserve_depth {
                        wgpu::StoreOp::Store
                    } else {
                        wgpu::StoreOp::Discard
                    },
                }),
                stencil_ops: None,
            }),
            timestamp_writes,
            ..Default::default()
        });
        draw(&mut pass);
    }
}

struct Crop {
    origin: wgpu::Origin3d,
    extent: wgpu::Extent3d,
}
impl Crop {
    fn from_pass(pass: &Pass, target: &Target) -> Self {
        let [x, y, w, h] = pass.viewport;
        let [left, top] = [x.floor() as u32, y.floor() as u32];
        let [right, bottom] = [(x + w).ceil() as u32, (y + h).ceil() as u32];
        assert!(right <= target.extent.width && bottom <= target.extent.height);
        Self {
            origin: wgpu::Origin3d {
                x: left,
                y: top,
                z: 0,
            },
            extent: wgpu::Extent3d {
                width: right - left,
                height: bottom - top,
                depth_or_array_layers: 1,
            },
        }
    }

    fn capture_preview(
        &self,
        gpu: &Gpu,
        target: &Target,
        pass: &Pass,
        solid_only: bool,
    ) -> Vec<u8> {
        self.capture(gpu, target, |render| {
            if solid_only {
                pass.set_viewport(render);
                pass.draw_solid(render);
            } else {
                pass.draw(render);
            }
        })
    }

    fn capture(
        &self,
        gpu: &Gpu,
        target: &Target,
        draw: impl FnOnce(&mut wgpu::RenderPass<'_>),
    ) -> Vec<u8> {
        let row = self.extent.width * 4;
        let stride =
            row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Goshawk screenshot crop"),
            size: u64::from(stride) * u64::from(self.extent.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        target.render(&mut encoder, false, false, None, draw);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.pixels,
                mip_level: 0,
                origin: self.origin,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(self.extent.height),
                },
            },
            self.extent,
        );
        let bytes = read_buffer(&gpu.device, gpu.queue.submit([encoder.finish()]), &readback);
        let mut rgb = Vec::with_capacity((self.extent.width * self.extent.height * 3) as usize);
        for row_bytes in bytes.chunks_exact(stride as usize) {
            for &[b, g, r, a] in row_bytes[..row as usize].as_chunks::<4>().0 {
                assert_eq!(a, 255);
                rgb.extend_from_slice(&[r, g, b]);
            }
        }
        rgb
    }

    fn save(&self, path: PathBuf, rgb: &[u8]) {
        let mut ppm =
            format!("P6\n{} {}\n255\n", self.extent.width, self.extent.height).into_bytes();
        ppm.extend_from_slice(rgb);
        write_new(path, &ppm);
    }
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    timestamps: bool,
}
impl Gpu {
    fn timings(&self, target: &Target, pass: &Pass, trailing_render: bool) -> Option<Timing> {
        if !self.timestamps {
            return None;
        }
        let count = SAMPLES * 2;
        let size = u64::from(count * 8);
        let set = self.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("SH008 shimmer GPU timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count,
        });
        let buffer = |usage| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("SH008 timestamp evidence"),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let resolved = buffer(wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC);
        let readback = buffer(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for _ in 0..WARMUP {
            target.render(&mut encoder, false, false, None, |render| pass.draw(render));
        }
        for sample in 0..SAMPLES {
            target.render(&mut encoder, false, true, None, |render| {
                pass.set_viewport(render);
                pass.draw_solid(render);
            });
            let timestamps = wgpu::RenderPassTimestampWrites {
                query_set: &set,
                beginning_of_pass_write_index: Some(sample * 2),
                end_of_pass_write_index: Some(sample * 2 + 1),
            };
            // Apple GPUs expose pass-boundary queries. Only the production shimmer draw
            // is in this measured pass. Attachment load/store overhead is included.
            target.render(&mut encoder, true, false, Some(timestamps), |render| {
                pass.set_viewport(render);
                pass.draw_shimmer(render);
            });
        }
        if trailing_render {
            target.render(&mut encoder, false, false, None, |render| {
                pass.set_viewport(render);
                pass.draw_solid(render);
            });
        }
        encoder.resolve_query_set(&set, 0..count, &resolved, 0);
        encoder.copy_buffer_to_buffer(&resolved, 0, &readback, 0, size);
        let bytes = read_buffer(
            &self.device,
            self.queue.submit([encoder.finish()]),
            &readback,
        );
        let ticks = bytes
            .as_chunks::<16>()
            .0
            .iter()
            .map(|pair| {
                let [start, end] = pair.as_chunks::<8>().0 else {
                    unreachable!()
                };
                [u64::from_le_bytes(*start), u64::from_le_bytes(*end)]
            })
            .collect();
        Some(Timing {
            period: self.queue.get_timestamp_period(),
            ticks,
        })
    }
}

fn run_pose(
    gpu: &Gpu,
    target: &Target,
    pass: &mut Pass,
    model: &ModelPreview,
    directory: &Path,
    name: &str,
) -> [usize; 2] {
    ok(pass.prepare(&gpu.queue, model, model.bounds, SCALE, FORMAT.is_srgb()));
    assert!(
        pass.shimmer,
        "{} rounded support certification failed",
        model.object
    );
    let crop = Crop::from_pass(pass, target);
    let solid = crop.capture_preview(gpu, target, pass, true);
    let normal = crop.capture_preview(gpu, target, pass, false);
    let repeat = crop.capture_preview(gpu, target, pass, false);
    let phase_offset = (std::mem::offset_of!(Uniforms, camera) + 3 * size_of::<f32>()) as u64;
    gpu.queue.write_buffer(
        &pass.hardware.uniform,
        phase_offset,
        bytemuck::bytes_of(&PHASE_SECONDS),
    );
    let shifted = crop.capture_preview(gpu, target, pass, false);
    gpu.queue.write_buffer(
        &pass.hardware.uniform,
        phase_offset,
        bytemuck::bytes_of(&0.0_f32),
    );
    for (variant, rgb) in [
        ("solid", &solid),
        ("normal", &normal),
        ("repeat", &repeat),
        ("phase", &shifted),
    ] {
        crop.save(directory.join(format!("{name}-{variant}.ppm")), rgb);
    }
    assert!(
        repeat == normal,
        "{name}: identical production samples drifted"
    );
    assert!(
        solid.iter().any(|&channel| channel > 0),
        "{name}: missing solid hull"
    );
    let peak_delta = |from: &[u8], to: &[u8]| {
        from.iter()
            .zip(to)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0)
    };
    assert!(peak_delta(&solid, &normal) <= 24 && peak_delta(&solid, &shifted) <= 24);
    assert!(peak_delta(&normal, &shifted) <= 7);
    let changed = |from: &[u8], to: &[u8]| {
        from.as_chunks::<3>()
            .0
            .iter()
            .zip(to.as_chunks::<3>().0)
            .filter(|(a, b)| a != b)
            .count()
    };
    let changed = [changed(&solid, &normal), changed(&normal, &shifted)];
    println!(
        "{} {name} viewport={:?} orientation={:?} crop={}x{}+{}+{} normal_delta_pixels={} phase_delta_pixels={}",
        model.object,
        pass.viewport,
        model.orientation,
        crop.extent.width,
        crop.extent.height,
        crop.origin.x,
        crop.origin.y,
        changed[0],
        changed[1]
    );
    changed
}

struct Timing {
    period: f32,
    ticks: Vec<[u64; 2]>,
}

fn write_timing(path: PathBuf, timing: Option<&Timing>) {
    let mut raw = String::new();
    if let Some(Timing { period, ticks }) = timing {
        use std::fmt::Write as _;
        ok(writeln!(raw, "sample\tstart_tick\tend_tick\tns_per_tick"));
        for (sample, [start, end]) in ticks.iter().enumerate() {
            ok(writeln!(raw, "{sample}\t{start}\t{end}\t{period}"));
        }
    } else {
        raw.push_str("unsupported: adapter has no TIMESTAMP_QUERY\n");
    }
    write_new(path, raw.as_bytes());
}

fn validate_timings(timings: [Option<Timing>; 3]) {
    let mut last_tick = None;
    let mut queue_period = None;
    for (name, timing) in POSES.into_iter().zip(timings) {
        let Some(Timing { period, ticks }) = timing else {
            continue;
        };
        assert!(
            period.is_finite() && period > 0.0,
            "{name}: invalid GPU timestamp period {period}"
        );
        if let Some(expected) = queue_period {
            assert_eq!(period, expected, "{name}: GPU timestamp period changed");
        }
        queue_period = Some(period);
        assert_eq!(ticks.len(), SAMPLES as usize);
        for (sample, [start, end]) in ticks.into_iter().enumerate() {
            // Metal uses all ones to signal an invalid counter result.
            assert!(
                start > 0 && end > start && end != u64::MAX,
                "{name}: invalid GPU timestamp pair {sample}: {start}..{end}, {period} ns/tick"
            );
            assert!(
                last_tick.is_none_or(|last| start >= last),
                "{name}: GPU timestamp pair {sample} overlaps or moves backwards"
            );
            last_tick = Some(end);
        }
    }
}

fn output_directory(variable: &str) -> PathBuf {
    let output = PathBuf::from(
        std::env::var_os(variable)
            .unwrap_or_else(|| panic!("set {variable} to a new absolute directory")),
    );
    assert!(output.is_absolute());
    ok(fs::create_dir(&output));
    output
}

fn metal_gpu(hull: Hull, request_timing: bool) -> Gpu {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::METAL;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = ok(pollster::block_on(
        instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
    ));
    let info = adapter.get_info();
    assert_eq!(info.backend, wgpu::Backend::Metal);
    let timestamps = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
    let required_features = if request_timing {
        timestamps
    } else {
        wgpu::Features::empty()
    };
    println!(
        "{} adapter={info:?} format={FORMAT:?} logical={LOGICAL:?} scale={SCALE:?} timestamp_available={timestamps:?} timestamp_requested={required_features:?} phase_seconds={PHASE_SECONDS}",
        hull.object_id()
    );
    let (device, queue) = ok(pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_features,
            ..Default::default()
        },
    )));
    Gpu {
        device,
        queue,
        timestamps: required_features.contains(wgpu::Features::TIMESTAMP_QUERY),
    }
}

fn same_pose_shimmer(hull: Hull, directory_variable: &str) {
    let output = output_directory(directory_variable);
    let gpu = metal_gpu(hull, false);
    let target = Target::new(&gpu.device, LOGICAL);
    let mut pass = ok(Pass::for_hull(&gpu.device, &gpu.queue, FORMAT, hull));
    assert_eq!(pass.hull(), hull);
    assert_eq!(
        pass.draw_count,
        match hull {
            Hull::Explorer => 10_452,
            Hull::Mk2 => 10_596,
            Hull::Custom => super::CUSTOM_DETAILED_FACES as u32 * 3,
            Hull::SableLiner => 13_488,
        }
    );
    let mut totals = [0, 0];
    let models = showroom_models(hull);
    let other = match hull {
        Hull::Explorer => Hull::Mk2,
        Hull::Mk2 | Hull::SableLiner => Hull::Explorer,
        Hull::Custom => Hull::Mk2,
    };
    let mut wrong = models[0].clone();
    wrong.object = other.object_id().into();
    wrong.geometry_key = other.geometry_key().into();
    assert!(
        pass.prepare(&gpu.queue, &wrong, wrong.bounds, SCALE, FORMAT.is_srgb())
            .is_err()
    );
    let empty = crate::docked_screens::Rect::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        pass.prepare(&gpu.queue, &models[0], empty, SCALE, FORMAT.is_srgb())
            .is_err()
    );
    assert_eq!(pass.viewport, [0.0; 4]);
    let [front, oblique, upper] = models
        .each_ref()
        .map(|model| hull_local_direction(&pass, model));
    let mut directions = String::from("pose\tx\ty\tz\n");
    for ((name, [x, y, z]), model) in POSES.into_iter().zip([front, oblique, upper]).zip(&models) {
        use std::fmt::Write as _;
        println!(
            "{} {name} orientation={:?} hull_local_direction=[{x:.9}, {y:.9}, {z:.9}]",
            model.object, model.orientation
        );
        ok(writeln!(directions, "{name}\t{x:.9}\t{y:.9}\t{z:.9}"));
    }
    write_new(output.join("hull-local-views.tsv"), directions.as_bytes());

    let [front_pose, oblique_pose, upper_pose] = models.each_ref().map(|model| model.orientation);
    let expected_yaw = std::f64::consts::PI / 6.0;
    let expected_upper_pitch =
        (front_pose.pitch - std::f64::consts::PI / 3.0).rem_euclid(std::f64::consts::TAU);
    assert_eq!(front_pose, crate::docked_screens::Orientation::default());
    assert_eq!(oblique_pose.pitch, front_pose.pitch);
    assert_eq!(oblique_pose.roll, front_pose.roll);
    assert!((oblique_pose.yaw - expected_yaw).abs() < 1e-12);
    assert_eq!(upper_pose.roll, oblique_pose.roll);
    assert_eq!(upper_pose.yaw, oblique_pose.yaw);
    assert!((upper_pose.pitch - expected_upper_pitch).abs() < 1e-12);
    assert!(front[0].abs() < 0.05 && front[2] > 0.9);
    assert!(oblique[0].abs() > 0.4 && oblique[2] > 0.7);
    assert!(upper[1] > 0.6 && upper[2] > 0.4);
    for (name, model) in POSES.into_iter().zip(models) {
        let [effect, phase] = run_pose(&gpu, &target, &mut pass, &model, &output, name);
        totals[0] += effect;
        totals[1] += phase;
    }
    assert!(
        totals[0] > 0,
        "no visible close shimmer in any production pose"
    );
    assert!(
        totals[1] > 0,
        "no temporal modulation resolved at production sRGB precision"
    );
}

#[test]
#[ignore = "Mac owner only: explicit production SH008 visual qualification"]
fn metal_goshawk_same_pose_shimmer() {
    same_pose_shimmer(Hull::Explorer, "MERIDIAN_SH008_EVIDENCE_DIR");
}

#[test]
#[ignore = "Mac owner only: explicit production SH009 visual qualification"]
fn metal_goshawk_mk2_same_pose_shimmer() {
    same_pose_shimmer(Hull::Mk2, "MERIDIAN_SH009_EVIDENCE_DIR");
}

#[test]
#[ignore = "Mac owner only: explicit production SH010 visual qualification"]
fn metal_goshawk_custom_same_pose_shimmer() {
    same_pose_shimmer(Hull::Custom, "MERIDIAN_SH010_EVIDENCE_DIR");
}

#[test]
#[ignore = "Mac owner only: production SH012 subtle shimmer"]
fn metal_sable_liner_same_pose_shimmer() {
    same_pose_shimmer(Hull::SableLiner, "MERIDIAN_SH012_EVIDENCE_DIR");
}

#[test]
#[ignore = "Mac owner only: original untextured SH012 source colour through production shader"]
fn metal_sable_liner_untextured_face_uses_exact_source_colour() {
    use super::super::{Triangle, VisualVertex, gpu as renderer};

    let hull = Hull::SableLiner;
    let device = metal_gpu(hull, false);
    let target = Target::new(&device.device, [32.0, 32.0]);
    let original = ok(super::visual_assets::load(hull.geometry_key()))[2156 * 3];
    assert_eq!(original.texture, u32::MAX);
    // Preserve original face attributes. Put the flat test triangle at a known camera depth.
    let vertices =
        [[-1.0, -1.0, 2.0], [1.0, -1.0, 2.0], [0.0, 1.0, 2.0]].map(|position| VisualVertex {
            position,
            ..original
        });
    let no_pieces = [<Triangle as bytemuck::Zeroable>::zeroed()];
    let hardware = ok(renderer::Hardware::new(
        &device.device,
        &device.queue,
        FORMAT,
        renderer::Inputs {
            shader: &super::shader(hull),
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
    device
        .queue
        .write_buffer(&hardware.uniform, 0, bytemuck::bytes_of(&uniforms));
    let crop = Crop {
        origin: wgpu::Origin3d { x: 32, y: 32, z: 0 },
        extent: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    };
    let center = crop.capture(&device, &target, |render| {
        render.set_pipeline(&hardware.solid);
        render.set_bind_group(0, &hardware.group, &[]);
        render.set_vertex_buffer(0, hardware.vertices.slice(..));
        render.draw(0..3, 0..1);
    });
    assert_eq!(center, [0x74, 0xbf, 0xcc]);
}

#[test]
#[ignore = "Mac owner only: explicit production SH008 GPU timestamp qualification"]
fn metal_goshawk_gpu_cost() {
    let output = output_directory("MERIDIAN_SH008_TIMING_DIR");
    let trailing_render = match std::env::var("MERIDIAN_SH008_TIMING_VARIANT") {
        Err(std::env::VarError::NotPresent) => false,
        Ok(variant) if variant == "trailing-render" => true,
        other => panic!("unknown timestamp diagnostic variant: {other:?}"),
    };
    let variant = if trailing_render {
        "trailing-render"
    } else {
        "original"
    };
    write_new(
        output.join("timing-variant.txt"),
        format!("{variant}\n").as_bytes(),
    );
    println!("SH008 timing variant={variant}");
    let gpu = metal_gpu(Hull::Explorer, true);
    let target = Target::new(&gpu.device, LOGICAL);
    let mut pass = ok(Pass::for_hull(
        &gpu.device,
        &gpu.queue,
        FORMAT,
        Hull::Explorer,
    ));
    assert!(pass.shimmer, "SH008 rounded support certification failed");
    let models = showroom_models(Hull::Explorer);
    let mut names = POSES.into_iter();
    let timings = models.each_ref().map(|model| {
        let name = names.next().unwrap_or_else(|| panic!("missing pose name"));
        ok(pass.prepare(&gpu.queue, model, model.bounds, SCALE, FORMAT.is_srgb()));
        let timing = gpu.timings(&target, &pass, trailing_render);
        write_timing(output.join(format!("{name}-gpu.tsv")), timing.as_ref());
        timing
    });
    // Persist every returned pair from every pose before validation can fail.
    validate_timings(timings);
}

fn minimum_rgb(
    hull: Hull,
    solo: &crate::docked_screens::Frame,
    solo_rgb: &[u8],
    comparison: &crate::docked_screens::Frame,
    combined: &[u8],
    isolated: &[Vec<u8>],
    size: [u32; 2],
) -> Result<[usize; 3], &'static str> {
    let [standalone] = solo.models.as_slice() else {
        return Err("Expected the single original hull pane");
    };
    let [left, right] = comparison.models.as_slice() else {
        return Err("Expected the two ordinary comparison panes");
    };
    let [first, second] = isolated else {
        return Err("Expected two isolated comparison images");
    };
    if left.object != "SH025"
        || Hull::from_model(right) != Some(hull)
        || Hull::from_model(standalone) != Some(hull)
    {
        return Err("Incorrect ordinary original hull comparison identity");
    }
    let solo = isolated_pane(solo_rgb, size, SCALE, standalone.bounds)?;
    let left = isolated_pane(first, size, SCALE, left.bounds)?;
    let right = isolated_pane(second, size, SCALE, right.bounds)?;
    exact_pane_union(combined, [first, second], size)?;
    Ok([solo.nonblack, left.nonblack, right.nonblack])
}

fn minimum_showroom_model_pass(hull: Hull, directory_variable: &str) {
    use super::minimum_qualification::{self, Frames};
    use std::fmt::Write as _;

    let output = output_directory(directory_variable);
    let gpu = metal_gpu(hull, false);
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
    let mut panes = String::from("image\tship\tx\ty\twidth\theight\tpitch\troll\tyaw\n");
    let mut capture = |name: &str, frame: &crate::docked_screens::Frame| {
        let image = format!("{name}.ppm");
        for model in &frame.models {
            let bounds = model.bounds;
            let orientation = model.orientation;
            ok(writeln!(
                panes,
                "{image}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                model.object,
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                orientation.pitch,
                orientation.roll,
                orientation.yaw,
            ));
        }
        ok(docked.prepare(
            &gpu.device,
            &gpu.queue,
            frame,
            MIN_WINDOW_LOGICAL,
            size,
            None,
        ));
        let rgb = crop.capture(&gpu, &target, |render| docked.draw_models(render));
        crop.save(output.join(image), &rgb);
        rgb
    };
    let verify_rgb = matches!(hull, Hull::Mk2 | Hull::SableLiner);
    let mut qualification = Vec::new();
    for (pose, Frames { solo, comparison }) in minimum_qualification::POSES
        .into_iter()
        .zip(minimum_qualification::station_frames(hull))
    {
        let solo_rgb = capture(&format!("{pose}-solo"), &solo);
        let combined = capture(&format!("{pose}-compare"), &comparison);
        let mut isolates = Vec::new();
        for model in &comparison.models {
            // Keep the Compare frame and its original pose and pane. Remove only the peer.
            let mut isolated = comparison.clone();
            isolated.models = vec![model.clone()];
            isolates.push(capture(
                &format!("{pose}-compare-{}", model.object),
                &isolated,
            ));
        }
        if verify_rgb {
            qualification.push((
                pose,
                minimum_rgb(
                    hull,
                    &solo,
                    &solo_rgb,
                    &comparison,
                    &combined,
                    &isolates,
                    size,
                ),
            ));
        }
    }
    write_new(output.join("model-panes.tsv"), panes.as_bytes());
    if verify_rgb {
        let mut report = String::from("pose\tsolo_visible\tleft_visible\tright_visible\tstatus\n");
        for (pose, result) in &qualification {
            match result {
                Ok([solo, left, right]) => {
                    ok(writeln!(report, "{pose}\t{solo}\t{left}\t{right}\tPASS"))
                }
                Err(reason) => ok(writeln!(report, "{pose}\t0\t0\t0\t{reason}")),
            }
        }
        write_new(output.join("rgb-qualification.tsv"), report.as_bytes());
        assert_eq!(qualification.len(), minimum_qualification::POSES.len());
        for (_, result) in qualification {
            ok(result);
        }
    }
    println!(
        "{} minimum Showroom model pass logical={MIN_WINDOW_LOGICAL:?} physical={size:?}",
        hull.object_id()
    );
}

#[test]
#[ignore = "Mac owner only: explicit minimum-size production SH008 Showroom model pass"]
fn metal_goshawk_minimum_showroom_model_pass() {
    minimum_showroom_model_pass(Hull::Explorer, "MERIDIAN_SH008_MINIMUM_DIR");
}

#[test]
#[ignore = "Mac owner only: explicit minimum-size production SH009 Showroom model pass"]
fn metal_goshawk_mk2_minimum_showroom_model_pass() {
    minimum_showroom_model_pass(Hull::Mk2, "MERIDIAN_SH009_MINIMUM_DIR");
}

#[test]
#[ignore = "Mac owner only: minimum-size production SH012 solo and Compare"]
fn metal_sable_liner_minimum_showroom_model_pass() {
    minimum_showroom_model_pass(Hull::SableLiner, "MERIDIAN_SH012_MINIMUM_DIR");
}

#[test]
#[ignore = "Mac owner only: original-hull renderer cache switching and mixed panes"]
fn metal_goshawk_mk2_cache_switching() {
    use super::minimum_qualification::station_frames;
    use crate::docked_screens::Frame;
    use std::fmt::Write as _;

    let output = output_directory("MERIDIAN_SH009_CACHE_DIR");
    let gpu = metal_gpu(Hull::Mk2, false);
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
    let mut capture = |name: &str, frame: &Frame| {
        ok(docked.prepare(
            &gpu.device,
            &gpu.queue,
            frame,
            MIN_WINDOW_LOGICAL,
            size,
            None,
        ));
        let rgb = crop.capture(&gpu, &target, |render| docked.draw_models(render));
        crop.save(output.join(format!("{name}.ppm")), &rgb);
        rgb
    };
    // Separate source-bound GPU passes bypass the docked cache completely. Otherwise an
    // incorrectly reused hull could agree with the same wrong hull in the isolated frame.
    let mut direct_explorer = ok(Pass::for_hull(
        &gpu.device,
        &gpu.queue,
        FORMAT,
        Hull::Explorer,
    ));
    let mut direct_mk2 = ok(Pass::for_hull(&gpu.device, &gpu.queue, FORMAT, Hull::Mk2));
    let mut reference = |name: &str, model: &ModelPreview| {
        let pass = match Hull::from_model(model) {
            Some(Hull::Explorer) => &mut direct_explorer,
            Some(Hull::Mk2) => &mut direct_mk2,
            Some(Hull::Custom | Hull::SableLiner) | None => {
                panic!("unexpected hull in independent reference")
            }
        };
        assert_eq!(Hull::from_model(model), Some(pass.hull()));
        ok(pass.prepare(&gpu.queue, model, model.bounds, SCALE, FORMAT.is_srgb()));
        assert!(
            pass.shimmer,
            "{} support qualification failed",
            model.object
        );
        let rgb = crop.capture_preview(&gpu, &target, pass, false);
        crop.save(output.join(format!("{name}.ppm")), &rgb);
        rgb
    };
    let explorer = station_frames(Hull::Explorer).remove(0);
    let mk2 = station_frames(Hull::Mk2).remove(0);
    let [explorer_source] = explorer.solo.models.as_slice() else {
        panic!("expected one Explorer model")
    };
    let [mk2_source] = mk2.solo.models.as_slice() else {
        panic!("expected one Mk II model")
    };
    let original = capture("explorer-before", &explorer.solo);
    let revised = capture("mk2-before", &mk2.solo);
    ok(isolated_pane(
        &original,
        size,
        SCALE,
        explorer_source.bounds,
    ));
    ok(isolated_pane(&revised, size, SCALE, mk2_source.bounds));
    ok(exact_original(
        &original,
        &reference("explorer-reference", explorer_source),
        size,
    ));
    ok(exact_original(
        &revised,
        &reference("mk2-reference", mk2_source),
        size,
    ));
    assert_ne!(
        original, revised,
        "distinct original hulls rendered identically"
    );
    assert_eq!(original, capture("explorer-return", &explorer.solo));
    assert_eq!(revised, capture("mk2-return", &mk2.solo));

    // A renderer-only mixed-hull frame exercises two simultaneous cached passes. This
    // does not claim that the ordinary station UI offers that particular comparison pair.
    let mut mixed = mk2.comparison.clone();
    let [left, right] = mixed.models.as_mut_slice() else {
        panic!("expected two comparison panes")
    };
    left.object.clone_from(&explorer_source.object);
    left.geometry_key.clone_from(&explorer_source.geometry_key);
    left.orientation = right.orientation;
    let mut report = String::from(
        "order\tleft_hull\tright_hull\tleft_visible\tright_visible\tfull_union\tindependent_references\n",
    );
    let mut previous_combined: Option<Vec<u8>> = None;
    for (order, expected) in [
        ("explorer-mk2", [Hull::Explorer, Hull::Mk2]),
        ("mk2-explorer", [Hull::Mk2, Hull::Explorer]),
    ] {
        assert_eq!(mixed.models.len(), expected.len());
        for (model, hull) in mixed.models.iter().zip(expected) {
            assert_eq!(Hull::from_model(model), Some(hull));
        }
        let combined = capture(order, &mixed);
        let mut independent_images = Vec::new();
        for model in &mixed.models {
            let mut isolated = mixed.clone();
            isolated.models = vec![model.clone()];
            let solo = capture(&format!("{order}-{}", model.object), &isolated);
            let measured = ok(isolated_pane(&solo, size, SCALE, model.bounds));
            let independent = reference(&format!("{order}-reference-{}", model.object), model);
            ok(exact_original(&solo, &independent, size));
            independent_images.push((independent, measured.nonblack));
        }
        let [first, second] = independent_images.as_slice() else {
            panic!("expected two full-target independent references")
        };
        ok(exact_pane_union(&combined, [&first.0, &second.0], size));
        let (left_pixels, right_pixels) = (first.1, second.1);
        if let Some(old_combined) = previous_combined.take() {
            assert!(
                exact_pane_union(&old_combined, [&first.0, &second.0], size).is_err(),
                "the old hull order must fail against the current independent references"
            );
        } else {
            previous_combined = Some(combined);
        }
        ok(writeln!(
            report,
            "{order}\t{}\t{}\t{left_pixels}\t{right_pixels}\tPASS\tPASS",
            expected[0].object_id(),
            expected[1].object_id()
        ));
        // Swap hull identity, preserving the two panes and their synchronized orientation.
        let [left, right] = mixed.models.as_mut_slice() else {
            unreachable!()
        };
        std::mem::swap(&mut left.object, &mut right.object);
        std::mem::swap(&mut left.geometry_key, &mut right.geometry_key);
    }
    assert_eq!(original, capture("explorer-final", &explorer.solo));
    assert_eq!(revised, capture("mk2-final", &mk2.solo));
    write_new(output.join("rgb-qualification.tsv"), report.as_bytes());
}

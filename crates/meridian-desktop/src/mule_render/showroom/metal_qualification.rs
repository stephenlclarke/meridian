//! Explicit Metal evidence from the production Goshawk Showroom pass.
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
mod zebra_qualification;

fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("{error:?}"))
}

fn showroom_models() -> [ModelPreview; 3] {
    showroom_models_for("Goshawk Explorer", Hull::Explorer)
}

fn showroom_models_for(name: &str, expected: Hull) -> [ModelPreview; 3] {
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
        assert_eq!(super::hull(&model), Some(expected));
        model
    };
    for action in [
        Input::Key(Key::Search),
        Input::Text(name.into()),
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
                label: Some("SH008 qualification target"),
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
            label: Some("SH008 Showroom evidence"),
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
            label: Some("SH008 screenshot crop"),
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
        solid.iter().any(|&channel| channel != 0),
        "{name}: missing solid hull"
    );
    let peak_delta = |before: &[u8], after: &[u8]| {
        before
            .iter()
            .zip(after)
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

fn metal_gpu(request_timing: bool) -> Gpu {
    metal_gpu_for("SH008", request_timing)
}

fn metal_gpu_for(object: &str, request_timing: bool) -> Gpu {
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
        "{object} adapter={info:?} format={FORMAT:?} logical={LOGICAL:?} scale={SCALE:?} timestamp_available={timestamps:?} timestamp_requested={required_features:?} phase_seconds={PHASE_SECONDS}"
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

#[test]
#[ignore = "Mac owner only: explicit production SH008 visual qualification"]
fn metal_goshawk_same_pose_shimmer() {
    same_pose_shimmer(
        Hull::Explorer,
        "SH008",
        "Goshawk Explorer",
        "MERIDIAN_SH008_EVIDENCE_DIR",
    );
}

#[test]
#[ignore = "Mac owner only: explicit production SH010 visual qualification"]
fn metal_goshawk_custom_same_pose_shimmer() {
    same_pose_shimmer(
        Hull::Custom,
        "SH010",
        "Goshawk Mk II Custom",
        "MERIDIAN_SH010_EVIDENCE_DIR",
    );
}

#[test]
#[ignore = "Mac owner only: explicit production SH021 visual qualification"]
fn metal_zebra_same_pose_shimmer() {
    same_pose_shimmer(Hull::Zebra, "SH021", "Zebra", "MERIDIAN_SH021_EVIDENCE_DIR");
}

fn same_pose_shimmer(hull: Hull, object: &str, name: &str, variable: &str) {
    let output = output_directory(variable);
    let gpu = metal_gpu_for(object, false);
    let target = Target::new(&gpu.device, LOGICAL);
    let mut pass = ok(Pass::for_hull(&gpu.device, &gpu.queue, FORMAT, hull));
    let mut totals = [0, 0];
    let models = showroom_models_for(name, hull);
    let [front, oblique, upper] = models
        .each_ref()
        .map(|model| hull_local_direction(&pass, model));
    let mut directions = String::from("pose\tx\ty\tz\n");
    for ((name, [x, y, z]), model) in POSES.into_iter().zip([front, oblique, upper]).zip(&models) {
        use std::fmt::Write as _;
        println!(
            "{object} {name} orientation={:?} hull_local_direction=[{x:.9}, {y:.9}, {z:.9}]",
            model.orientation
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
    let gpu = metal_gpu(true);
    let target = Target::new(&gpu.device, LOGICAL);
    let mut pass = ok(Pass::new(&gpu.device, &gpu.queue, FORMAT));
    assert!(pass.shimmer, "SH008 rounded support certification failed");
    let models = showroom_models();
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

#[test]
#[ignore = "Mac owner only: explicit minimum-size production Showroom model pass"]
fn metal_goshawk_minimum_showroom_model_pass() {
    use super::minimum_qualification::{self, Frames};
    use std::fmt::Write as _;

    let output = output_directory("MERIDIAN_SH008_MINIMUM_DIR");
    let gpu = metal_gpu(false);
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
    };
    for (pose, Frames { solo, comparison }) in minimum_qualification::POSES
        .into_iter()
        .zip(minimum_qualification::station_frames())
    {
        capture(&format!("{pose}-solo"), &solo);
        capture(&format!("{pose}-compare"), &comparison);
        for model in &comparison.models {
            // Keep the Compare frame and its original pose and pane. Remove only the peer.
            let mut isolated = comparison.clone();
            isolated.models = vec![model.clone()];
            capture(&format!("{pose}-compare-{}", model.object), &isolated);
        }
    }
    write_new(output.join("model-panes.tsv"), panes.as_bytes());
    println!("SH008 minimum Showroom model pass logical={MIN_WINDOW_LOGICAL:?} physical={size:?}");
}

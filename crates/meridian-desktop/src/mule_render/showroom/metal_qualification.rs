//! Explicit Metal evidence from the production SH008 Showroom pass.
use super::{Pass, Uniforms, matches};
use crate::{
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
const PHASE_SECONDS: f32 = std::f32::consts::PI / 0.7;

fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("{error:?}"))
}

fn showroom_models() -> [ModelPreview; 3] {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(97008)));
    let before = ok(profile.world.save_bytes());
    let mut menu = ok(Docked::new(&profile.world));
    ok(menu.open(&profile.world, Screen::Showroom));
    let viewport = Viewport::new(LOGICAL[0], LOGICAL[1])
        .unwrap_or_else(|| panic!("invalid qualification viewport"));
    let input = |menu: &mut Docked, profile: &mut Profile, input| {
        menu.presented(ok(menu.build_frame(&profile.world, viewport, 0.0)));
        ok(menu.input(profile, input));
    };
    let preview = |menu: &mut Docked, profile: &Profile| {
        let [model] = ok(menu.build_frame(&profile.world, viewport, 0.0))
            .models
            .try_into()
            .unwrap_or_else(|_| panic!("expected exactly one production Showroom model"));
        assert!(matches(&model));
        model
    };
    for action in [
        Input::Key(Key::Search),
        Input::Text("Goshawk Explorer".into()),
        Input::Key(Key::Confirm),
        Input::Key(Key::Compare),
        Input::Key(Key::ResetView),
    ] {
        input(&mut menu, &mut profile, action);
    }
    let front = preview(&mut menu, &profile);
    for _ in 0..2 {
        input(&mut menu, &mut profile, Input::Key(Key::TurnRight));
    }
    let oblique = preview(&mut menu, &profile);
    input(&mut menu, &mut profile, Input::Key(Key::NextAxis));
    for _ in 0..4 {
        input(&mut menu, &mut profile, Input::Key(Key::TurnLeft));
    }
    let upper = preview(&mut menu, &profile);
    assert_eq!(front.bounds, oblique.bounds);
    assert_eq!(front.bounds, upper.bounds);
    assert_ne!(front.orientation, oblique.orientation);
    assert_ne!(oblique.orientation, upper.orientation);
    assert_eq!(before, ok(profile.world.save_bytes()));
    [front, oblique, upper]
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
    fn new(device: &wgpu::Device) -> Self {
        let extent = wgpu::Extent3d {
            width: (LOGICAL[0] * SCALE[0]) as u32,
            height: (LOGICAL[1] * SCALE[1]) as u32,
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

    fn capture(&self, gpu: &Gpu, target: &Target, pass: &Pass, solid_only: bool) -> Vec<u8> {
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
        target.render(&mut encoder, false, false, None, |render| {
            if solid_only {
                pass.set_viewport(render);
                pass.draw_solid(render);
            } else {
                pass.draw(render);
            }
        });
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
    fn timings(&self, target: &Target, pass: &Pass) -> Option<(f32, Vec<[u64; 2]>)> {
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
        Some((self.queue.get_timestamp_period(), ticks))
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
    assert!(pass.shimmer, "SH008 rounded support certification failed");
    let crop = Crop::from_pass(pass, target);
    let solid = crop.capture(gpu, target, pass, true);
    let normal = crop.capture(gpu, target, pass, false);
    let repeat = crop.capture(gpu, target, pass, false);
    let phase_offset = (std::mem::offset_of!(Uniforms, camera) + 3 * size_of::<f32>()) as u64;
    gpu.queue.write_buffer(
        &pass.hardware.uniform,
        phase_offset,
        bytemuck::bytes_of(&PHASE_SECONDS),
    );
    let shifted = crop.capture(gpu, target, pass, false);
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
        "SH008 {name} viewport={:?} orientation={:?} crop={}x{}+{}+{} normal_delta_pixels={} phase_delta_pixels={}",
        pass.viewport,
        model.orientation,
        crop.extent.width,
        crop.extent.height,
        crop.origin.x,
        crop.origin.y,
        changed[0],
        changed[1]
    );
    let mut raw = String::new();
    if let Some((period, ticks)) = gpu.timings(target, pass) {
        use std::fmt::Write as _;
        assert!(period.is_finite() && period > 0.0);
        assert_eq!(ticks.len(), SAMPLES as usize);
        ok(writeln!(raw, "sample\tstart_tick\tend_tick\tns_per_tick"));
        for (sample, [start, end]) in ticks.into_iter().enumerate() {
            assert!(end > start, "invalid GPU timestamp pair {sample}");
            ok(writeln!(raw, "{sample}\t{start}\t{end}\t{period}"));
        }
    } else {
        raw.push_str("unsupported: adapter has no TIMESTAMP_QUERY\n");
    }
    write_new(directory.join(format!("{name}-gpu.tsv")), raw.as_bytes());
    changed
}

#[test]
#[ignore = "Mac owner only: explicit production SH008 pixel and GPU timestamp qualification"]
fn metal_goshawk_same_pose_shimmer_and_gpu_cost() {
    let output = PathBuf::from(
        std::env::var_os("MERIDIAN_SH008_EVIDENCE_DIR").unwrap_or_else(|| {
            panic!("set MERIDIAN_SH008_EVIDENCE_DIR to a new absolute directory")
        }),
    );
    assert!(output.is_absolute());
    ok(fs::create_dir(&output));
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::METAL;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = ok(pollster::block_on(
        instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
    ));
    let info = adapter.get_info();
    assert_eq!(info.backend, wgpu::Backend::Metal);
    let required_features = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
    println!(
        "SH008 adapter={info:?} format={FORMAT:?} logical={LOGICAL:?} scale={SCALE:?} timestamp_queries={:?} phase_seconds={PHASE_SECONDS}",
        required_features
    );
    let (device, queue) = ok(pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_features,
            ..Default::default()
        },
    )));
    let gpu = Gpu {
        device,
        queue,
        timestamps: required_features.contains(wgpu::Features::TIMESTAMP_QUERY),
    };
    let target = Target::new(&gpu.device);
    let mut pass = ok(Pass::new(&gpu.device, &gpu.queue, FORMAT));
    let mut totals = [0, 0];
    for (name, model) in ["front", "oblique", "upper"]
        .into_iter()
        .zip(showroom_models())
    {
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

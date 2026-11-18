//! Explicit G1-only actual-shader fixture. No window, World actor, save or screenshot.
use super::tests::ok;
use super::*;
use wgpu::util::DeviceExt;

#[test]
#[ignore = "G1 sole Metal lane: explicit offscreen shader qualification, not ordinary source checks"]
fn metal_union_overlaps_do_not_brighten_and_depletion_distance_depth_hide() {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::METAL;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = ok(pollster::block_on(
        instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
    ));
    let (device, queue) = ok(pollster::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor::default()),
    ));
    let mut points = Vec::new();
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                points.push(Vec3::new(x, y, z));
            }
        }
    }
    let hull = ok(rounded::build(&points));
    let triangles: Vec<Triangle> = hull
        .faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.0.map(|i| points[i]);
            ok(triangle(a, b, c))
        })
        .collect();
    let storage = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("unit cube source boundary"),
        contents: bytemuck::cast_slice(&triangles),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("unit fixture view"),
        size: std::mem::size_of::<Uniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ACTUAL Mule shader under test"),
        source: wgpu::ShaderSource::Wgsl(include_str!("mule.wgsl").into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("actual one-union shader"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("union_vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("union_fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(false),
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("actual union inputs"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: storage.as_entire_binding(),
            },
        ],
    });
    let size = wgpu::Extent3d {
        width: 256,
        height: 256,
        depth_or_array_layers: 1,
    };
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("unit pixels"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("unit occluder"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bounded unit readback"),
        size: 256 * 256 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let piece = Piece {
        range: [0, triangles.len() as u32, 0, 0],
        low: [-1.0, -1.0, -1.0, 0.02],
        high: [1.0, 1.0, 1.0, 0.0],
    };
    let mut u = Uniforms {
        projection: [1.0, 1.6, 0.1, 20000.0],
        origin: [0.0, 0.0, 3.0, 0.0],
        x: [1.0, 0.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0, 0.0],
        z: [0.0, 0.0, 1.0, 0.0],
        camera: [0.0, 0.0, -3.0, 0.0],
        banks: [1.0, 1.0, 0.018, 64.0],
        low: [-1.021; 4],
        high: [1.021; 4],
        pieces: [piece; 16],
    };
    let render = |u: &Uniforms, depth_clear: f32| {
        queue.write_buffer(&uniform, 0, bytemuck::bytes_of(u));
        let color_view = color.create_view(&Default::default());
        let depth_view = depth.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(depth_clear),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(1024),
                    rows_per_image: Some(256),
                },
            },
            size,
        );
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = tx.send(result);
        });
        ok(device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(10)),
        }));
        ok(ok(rx.recv_timeout(std::time::Duration::from_secs(10))));
        let bytes = readback.get_mapped_range(..).to_vec();
        readback.unmap();
        bytes
    };
    let overlap = render(&u, 1.0);
    assert!(overlap.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
    assert!(overlap.as_chunks::<4>().0.iter().all(|p| p[3] <= 5));
    for item in &mut u.pieces[1..] {
        item.range = [0; 4];
        item.low = [100.0, 100.0, 100.0, 0.02];
        item.high = [101.0, 101.0, 101.0, 0.0];
    }
    let single = render(&u, 1.0);
    assert_eq!(single, overlap, "identical overlap must not add a layer");
    u.banks[1] = 0.0;
    assert!(
        render(&u, 1.0).iter().all(|p| *p == 0),
        "rear depletion hides the rear-facing entry"
    );
    u.banks[1] = 1.0;
    assert!(
        render(&u, 0.0).iter().all(|p| *p == 0),
        "opaque depth occludes shimmer"
    );
    u.camera[2] = -80.0;
    u.origin[2] = 80.0;
    assert!(
        render(&u, 1.0).iter().all(|p| *p == 0),
        "beyond64m stays invisible"
    );
}

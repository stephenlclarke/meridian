use super::{
    Piece, Scene, SmoothPieceArray, TEXTURES, Triangle, Uniforms, VERTEX_COUNT, View, VisualVertex,
    supports, uniforms, visual,
};
use meridian_core::mule::{MULE_PIECES, MULE_PRESENTATION_REVISION};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub struct Pass {
    solid: wgpu::RenderPipeline,
    shimmer: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    cached: Arc<SmoothPieceArray>,
    pieces: [Piece; MULE_PIECES],
    active: bool,
    shimmer_supported: bool,
    srgb: bool,
}
impl Pass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        scene: Scene<'_>,
    ) -> Result<Self, String> {
        let source = meridian_core::mule::MuleHullSource::bundled().map_err(|e| e.to_string())?;
        if source.pieces() != scene.envelope.pieces.as_ref()
            || scene.envelope.revision != MULE_PRESENTATION_REVISION
        {
            return Err("SH001 contact/presentation source mismatch".into());
        }
        let (points, pieces, shimmer_supported) = match supports(scene.envelope) {
            Ok((points, pieces)) => (points, pieces, true),
            Err(reason) => {
                // Uncertified rounding stays invisible; detailed source geometry still draws.
                eprintln!("Meridian Mule shimmer unavailable: {reason}");
                (
                    vec![super::Triangle {
                        a: [0.0; 4],
                        b: [0.0; 4],
                        c: [0.0; 4],
                        normal: [0.0; 4],
                    }],
                    [Piece {
                        range: [0; 4],
                        low: [0.0; 4],
                        high: [0.0; 4],
                    }; MULE_PIECES],
                    false,
                )
            }
        };
        let Hardware {
            solid,
            shimmer,
            uniform,
            group,
            vertices,
        } = Hardware::new(
            device,
            queue,
            format,
            Inputs {
                shader: include_str!("mule.wgsl"),
                uniform_size: size_of::<Uniforms>() as u64,
                points: &points,
                visual: &visual()?,
                authored_materials: false,
            },
        )?;
        Ok(Self {
            solid,
            shimmer,
            uniform,
            group,
            vertices,
            cached: scene.envelope.pieces.clone(),
            pieces,
            active: false,
            shimmer_supported,
            srgb: format.is_srgb(),
        })
    }
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        scene: Option<Scene<'_>>,
        view: View,
    ) -> Result<(), String> {
        self.active = false;
        let Some(scene) = scene else {
            return Ok(());
        };
        if !Arc::ptr_eq(&self.cached, &scene.envelope.pieces)
            && self.cached.as_ref() != scene.envelope.pieces.as_ref()
        {
            return Err("SH001 contact/presentation geometry changed".into());
        }
        let data = uniforms(scene, view, self.srgb, self.pieces)?;
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&data));
        self.active = true;
        Ok(())
    }
    pub fn hide(&mut self) {
        self.active = false;
    }
    pub fn draw_solid(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.active {
            pass.set_pipeline(&self.solid);
            pass.set_bind_group(0, &self.group, &[]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.draw(0..VERTEX_COUNT as u32, 0..1);
        }
    }
    pub fn draw_shimmer(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.active && self.shimmer_supported {
            pass.set_pipeline(&self.shimmer);
            pass.set_bind_group(0, &self.group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

pub(super) struct Hardware {
    pub(super) solid: wgpu::RenderPipeline,
    pub(super) shimmer: wgpu::RenderPipeline,
    pub(super) uniform: wgpu::Buffer,
    pub(super) group: wgpu::BindGroup,
    pub(super) vertices: wgpu::Buffer,
}

pub(super) struct Inputs<'a> {
    pub(super) shader: &'a str,
    pub(super) uniform_size: u64,
    pub(super) points: &'a [Triangle],
    pub(super) visual: &'a [VisualVertex],
    pub(super) authored_materials: bool,
}

impl Hardware {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        inputs: Inputs<'_>,
    ) -> Result<Self, String> {
        let Inputs {
            shader: shader_source,
            uniform_size,
            points,
            visual,
            authored_materials,
        } = inputs;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Original hull material and source-rounded union"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Source hull view"),
            size: uniform_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let points = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Immutable hull contact support triangles"),
            contents: bytemuck::cast_slice(points),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Three original opaque hull diffuse maps"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: 3,
            },
            mip_level_count: 11,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (layer, bytes) in TEXTURES.iter().enumerate() {
            if bytes.len() != 1024 * 1024 * 4 {
                return Err("SH001 texture dimensions changed".into());
            }
            let mut mip = bytes.to_vec();
            let mut side = 1024;
            for level in 0..11 {
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: level,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: layer as u32,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &mip,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(side * 4),
                        rows_per_image: Some(side),
                    },
                    wgpu::Extent3d {
                        width: side,
                        height: side,
                        depth_or_array_layers: 1,
                    },
                );
                if side > 1 {
                    mip = super::mip(&mip, side as usize);
                    side /= 2;
                }
            }
        }
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Original repeated UVs, filtered mips"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Hull immutable presentation inputs"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(uniform_size),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Hull source binding"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: points.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Hull presentation only"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let attributes = wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Float32x4,3=>Float32x2,4=>Uint32,5=>Uint32];
        let vertex_layout = [wgpu::VertexBufferLayout {
            array_stride: 56,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &attributes,
        }];
        let pipeline = |shimmer| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if shimmer {
                    "Hull single-union close grazing shimmer"
                } else {
                    "All original detailed hull faces"
                }),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(if shimmer {
                        "union_vertex"
                    } else if authored_materials {
                        "authored_vertex"
                    } else {
                        "solid_vertex"
                    }),
                    compilation_options: Default::default(),
                    buffers: if shimmer { &[] } else { &vertex_layout },
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if shimmer {
                        "union_fragment"
                    } else if authored_materials {
                        "authored_fragment"
                    } else {
                        "solid_fragment"
                    }),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: if shimmer {
                            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING)
                        } else if authored_materials {
                            Some(wgpu::BlendState::ALPHA_BLENDING)
                        } else {
                            None
                        },
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(!shimmer),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let solid = pipeline(false);
        let shimmer = pipeline(true);
        let painted: Vec<_> = visual
            .iter()
            .copied()
            .map(VisualVertex::with_display_paint)
            .collect();
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Original detailed triangles, materials and UVs"),
            contents: bytemuck::cast_slice(&painted),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Ok(Self {
            solid,
            shimmer,
            uniform,
            group,
            vertices,
        })
    }
}

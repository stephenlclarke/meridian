use super::{
    DockedPreview, MAP_SAMPLES, Projection, Scene, SurfaceData, Uniforms, docked_plan, frame_plan,
};
use meridian_core::{FlightView, Snapshot};
use std::sync::Arc;

/// Two fixed body slots, one fixed terrain buffer, no per-frame GPU allocation/readback.
pub struct Pass {
    solid: wgpu::RenderPipeline,
    halo: wgpu::RenderPipeline,
    uniforms: [wgpu::Buffer; 2],
    groups: [wgpu::BindGroup; 2],
    maps: wgpu::Buffer,
    cached: Option<Arc<SurfaceData>>,
    active: [bool; 2],
    clip: [u32; 4],
    output_srgb: bool,
}
impl Pass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("L337 celestial shading translation"),
            source: wgpu::ShaderSource::Wgsl(include_str!("celestial.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Celestial immutable uniforms and maps"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Uniforms>() as u64
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((MAP_SAMPLES * 16) as u64),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Celestial pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry, blend, write| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(write),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let solid = pipeline("surface_main", None, true);
        let halo = pipeline(
            "halo_main",
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            false,
        );
        let uniforms = std::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Celestial body uniforms"),
                size: std::mem::size_of::<Uniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let maps = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bounded planet terrain/cloud/settlement maps"),
            size: (MAP_SAMPLES * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let groups = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Celestial body inputs"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniforms[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: maps.as_entire_binding(),
                    },
                ],
            })
        });
        Self {
            solid,
            halo,
            uniforms,
            groups,
            maps,
            cached: None,
            active: [false; 2],
            clip: [0; 4],
            output_srgb: format.is_srgb(),
        }
    }
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        scene: Option<&Scene>,
        snapshot: Snapshot,
        view: FlightView,
        p: Projection,
    ) -> Result<(), &'static str> {
        self.active = [false; 2];
        let Some(scene) = scene else {
            self.cached = None;
            return Ok(());
        };
        let plan = frame_plan(scene, snapshot, view, p)?;
        self.clip = plan.clip;
        if let Some(planet) = &scene.planet {
            if plan.uniforms[0].is_some()
                && !self
                    .cached
                    .as_ref()
                    .is_some_and(|old| Arc::ptr_eq(old, &planet.surface))
            {
                queue.write_buffer(&self.maps, 0, bytemuck::cast_slice(&planet.surface.samples));
                self.cached = Some(Arc::clone(&planet.surface));
            }
        } else {
            self.cached = None;
        }
        for (i, u) in plan.uniforms.into_iter().enumerate() {
            if let Some(mut u) = u {
                u.night[3] = if self.output_srgb { 1.0 } else { 0.0 };
                queue.write_buffer(&self.uniforms[i], 0, bytemuck::bytes_of(&u));
                self.active[i] = true;
            }
        }
        Ok(())
    }
    /// Prepare only an actual current-system preview; no fallback celestial data.
    pub fn prepare_docked(
        &mut self,
        queue: &wgpu::Queue,
        input: DockedPreview<'_>,
    ) -> Result<bool, &'static str> {
        self.active = [false; 2];
        let Some(plan) = docked_plan(input)? else {
            return Ok(false);
        };
        let planet = input
            .scene
            .and_then(|s| s.planet.as_ref())
            .ok_or("missing preview planet")?;
        self.clip = plan.clip;
        let mut u = plan.uniforms[0].ok_or("missing preview uniforms")?;
        u.night[3] = if self.output_srgb { 1.0 } else { 0.0 };
        if !self
            .cached
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, &planet.surface))
        {
            queue.write_buffer(&self.maps, 0, bytemuck::cast_slice(&planet.surface.samples));
            self.cached = Some(Arc::clone(&planet.surface));
        }
        queue.write_buffer(&self.uniforms[0], 0, bytemuck::bytes_of(&u));
        self.active[0] = true;
        Ok(true)
    }
    pub fn draw_surfaces(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.draw(pass, &self.solid);
    }
    pub fn draw_halos(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.draw(pass, &self.halo);
    }
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, pipeline: &wgpu::RenderPipeline) {
        if !self.active.into_iter().any(|v| v) {
            return;
        }
        pass.set_pipeline(pipeline);
        pass.set_scissor_rect(self.clip[0], self.clip[1], self.clip[2], self.clip[3]);
        for (i, active) in self.active.into_iter().enumerate() {
            if active {
                pass.set_bind_group(0, &self.groups[i], &[]);
                pass.draw(0..6, 0..1);
            }
        }
    }
}

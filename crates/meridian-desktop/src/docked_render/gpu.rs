use super::{
    Geometry, MAX_GEOMETRY, MAX_GLYPHS, MAX_MODEL_VERTICES, MAX_MODELS, Model, TextVertex, atlas,
    intersection,
};
use crate::{docked_screens::Frame, mesh::Vertex, mule_render::showroom};

fn reusable_slot(
    cached: impl Iterator<Item = showroom::Hull>,
    active: &[Option<usize>; MAX_MODELS],
    hull: showroom::Hull,
) -> Option<usize> {
    cached.enumerate().find_map(|(index, candidate)| {
        (candidate == hull && !active.contains(&Some(index))).then_some(index)
    })
}

pub struct Pass {
    shape_pipeline: wgpu::RenderPipeline,
    model_pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,
    shapes: wgpu::Buffer,
    text: wgpu::Buffer,
    models: wgpu::Buffer,
    glyphs: wgpu::BindGroup,
    frame: Geometry,
    model: Model,
    srgb: bool,
    format: wgpu::TextureFormat,
    goshawk: Vec<showroom::Pass>,
    goshawk_active: [Option<usize>; MAX_MODELS],
}
impl Pass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Result<Self, String> {
        let model = Model::sh025()?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Docked shape and verified model"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../hud.wgsl").into()),
        });
        let pipeline = |depth| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Docked geometry"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: depth,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let shape_pipeline = pipeline(None);
        let model_pipeline = pipeline(Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }));
        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Docked code-drawn glyph atlas"),
            source: wgpu::ShaderSource::Wgsl(include_str!("text.wgsl").into()),
        });
        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Docked clipped labels"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TextVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32x3],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Existing Meridian glyphs, generated at startup"),
            size: wgpu::Extent3d {
                width: 80,
                height: 56,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(80),
                rows_per_image: Some(56),
            },
            wgpu::Extent3d {
                width: 80,
                height: 56,
                depth_or_array_layers: 1,
            },
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let view = texture.create_view(&Default::default());
        let glyphs = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Code glyphs"),
            layout: &text_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let buffer = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let shapes = buffer(
            "Bounded docked shapes",
            (MAX_GEOMETRY * std::mem::size_of::<Vertex>()) as u64,
        );
        let text = buffer(
            "Bounded docked glyphs",
            (MAX_GLYPHS * 6 * std::mem::size_of::<TextVertex>()) as u64,
        );
        let models = buffer(
            "Bounded verified model previews",
            (MAX_MODEL_VERTICES * std::mem::size_of::<Vertex>()) as u64,
        );
        Ok(Self {
            shape_pipeline,
            model_pipeline,
            text_pipeline,
            shapes,
            text,
            models,
            glyphs,
            frame: Geometry::with_capacity(),
            model,
            srgb: format.is_srgb(),
            format,
            goshawk: Vec::with_capacity(showroom::Hull::COUNT * MAX_MODELS),
            goshawk_active: [None; MAX_MODELS],
        })
    }
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &Frame,
        logical_size: [f64; 2],
        physical_size: [u32; 2],
        planet_system: Option<meridian_core::navigation::SystemId>,
    ) -> Result<(), String> {
        self.goshawk_active.fill(None);
        if (frame.viewport.width - logical_size[0]).abs() > 0.01
            || (frame.viewport.height - logical_size[1]).abs() > 0.01
        {
            return Err("docked frame viewport no longer matches the window".into());
        }
        self.frame
            .compose(frame, &self.model, self.srgb, planet_system)?;
        queue.write_buffer(&self.shapes, 0, bytemuck::cast_slice(&self.frame.shapes));
        queue.write_buffer(&self.text, 0, bytemuck::cast_slice(&self.frame.text));
        queue.write_buffer(&self.models, 0, bytemuck::cast_slice(&self.frame.models));
        self.prepare_goshawk(device, queue, frame, logical_size, physical_size)
    }

    fn prepare_goshawk(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &Frame,
        logical_size: [f64; 2],
        physical_size: [u32; 2],
    ) -> Result<(), String> {
        let mut active = [None; MAX_MODELS];
        let scale = std::array::from_fn(|axis| f64::from(physical_size[axis]) / logical_size[axis]);
        for preview in &frame.models {
            let Some(hull) = showroom::hull(preview) else {
                continue;
            };
            let Some(bounds) = intersection(preview.bounds, frame.viewport.canvas()) else {
                continue;
            };
            let active_slot = active
                .iter()
                .position(|slot| slot.is_none())
                .ok_or("Too many original Showroom preview panes")?;
            let cache_slot =
                match reusable_slot(self.goshawk.iter().map(showroom::Pass::hull), &active, hull) {
                    Some(index) => index,
                    None => {
                        if self.goshawk.len() >= showroom::Hull::COUNT * MAX_MODELS {
                            return Err(
                                "Original Showroom pass cache exceeded its hull/pane bound".into(),
                            );
                        }
                        let index = self.goshawk.len();
                        self.goshawk.push(showroom::Pass::for_hull(
                            device,
                            queue,
                            self.format,
                            hull,
                        )?);
                        index
                    }
                };
            self.goshawk[cache_slot].prepare(queue, preview, bounds, scale, self.srgb)?;
            active[active_slot] = Some(cache_slot);
        }
        // Commit only fully prepared panes. Cached passes from previous selections are never drawn.
        self.goshawk_active = active;
        Ok(())
    }
    /// The caller supplies a black-cleared colour target and fresh depth attachment for models.
    pub fn draw_models(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.model_pipeline);
        pass.set_vertex_buffer(0, self.models.slice(..));
        pass.draw(0..self.frame.models.len() as u32, 0..1);
        for index in self.goshawk_active.into_iter().flatten() {
            self.goshawk[index].draw(pass);
        }
    }
    pub fn draw_overlay(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.shape_pipeline);
        pass.set_vertex_buffer(0, self.shapes.slice(..));
        pass.draw(0..self.frame.shapes.len() as u32, 0..1);
        pass.set_pipeline(&self.text_pipeline);
        pass.set_bind_group(0, &self.glyphs, &[]);
        pass.set_vertex_buffer(0, self.text.slice(..));
        pass.draw(0..self.frame.text.len() as u32, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use showroom::Hull::{Custom, Explorer, Mk2, SableLiner};

    #[test]
    fn original_showroom_cache_reuses_only_the_same_hull_and_unused_pane() {
        let mut cached = Vec::new();
        let mut frame = |hulls: &[showroom::Hull]| {
            let mut active = [None; MAX_MODELS];
            for (slot, hull) in hulls.iter().copied().enumerate() {
                let index =
                    reusable_slot(cached.iter().copied(), &active, hull).unwrap_or_else(|| {
                        cached.push(hull);
                        cached.len() - 1
                    });
                active[slot] = Some(index);
            }
            active
        };
        assert_eq!(frame(&[Explorer]), [Some(0), None]);
        assert_eq!(frame(&[Mk2]), [Some(1), None]);
        assert_eq!(frame(&[Explorer]), [Some(0), None]);
        assert_eq!(frame(&[Explorer, Mk2]), [Some(0), Some(1)]);
        assert_eq!(frame(&[Mk2, Explorer]), [Some(1), Some(0)]);
        assert_eq!(frame(&[Mk2, Mk2]), [Some(1), Some(2)]);
        assert_eq!(frame(&[Explorer, Explorer]), [Some(0), Some(3)]);
        assert_eq!(frame(&[Custom, Mk2]), [Some(4), Some(1)]);
        assert_eq!(frame(&[Mk2, Custom]), [Some(1), Some(4)]);
        assert_eq!(frame(&[Custom, Custom]), [Some(4), Some(5)]);
        assert_eq!(frame(&[Explorer, Custom]), [Some(0), Some(4)]);
        assert_eq!(frame(&[Custom, Explorer]), [Some(4), Some(0)]);
        assert_eq!(frame(&[SableLiner]), [Some(6), None]);
        assert_eq!(frame(&[SableLiner, Mk2]), [Some(6), Some(1)]);
        assert_eq!(frame(&[Mk2, SableLiner]), [Some(1), Some(6)]);
        assert_eq!(frame(&[SableLiner, Custom]), [Some(6), Some(4)]);
        assert_eq!(frame(&[Custom, SableLiner]), [Some(4), Some(6)]);
        assert_eq!(frame(&[SableLiner, Explorer]), [Some(6), Some(0)]);
        assert_eq!(frame(&[Explorer, SableLiner]), [Some(0), Some(6)]);
        assert_eq!(frame(&[SableLiner, SableLiner]), [Some(6), Some(7)]);
        assert_eq!(frame(&[]), [None, None]);
        assert_eq!(frame(&[Mk2]), [Some(1), None]);
        assert_eq!(
            cached,
            [
                Explorer, Mk2, Mk2, Explorer, Custom, Custom, SableLiner, SableLiner
            ]
        );
        assert_eq!(cached.len(), showroom::Hull::COUNT * MAX_MODELS);
    }
}

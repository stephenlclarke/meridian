use super::{
    Geometry, MAX_GEOMETRY, MAX_GLYPHS, MAX_MODEL_VERTICES, MAX_MODELS, Model, TextVertex, atlas,
    intersection,
};
use crate::{
    authored_hull::AuthoredHulls, docked_screens::Frame, mesh::Vertex, mule_render::showroom,
};
use std::sync::Arc;

/// Move each matching pass into its requested pane only once. Unrequested passes are
/// released before new GPU resources are allocated for this frame.
fn retain_visible<T>(
    mut previous: [Option<T>; MAX_MODELS],
    requested: [Option<showroom::Hull>; MAX_MODELS],
    hull: impl Fn(&T) -> showroom::Hull,
) -> [Option<T>; MAX_MODELS] {
    requested.map(|wanted| {
        let wanted = wanted?;
        previous
            .iter_mut()
            .find(|slot| slot.as_ref().is_some_and(|pass| hull(pass) == wanted))?
            .take()
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
    authored: Arc<AuthoredHulls>,
    srgb: bool,
    format: wgpu::TextureFormat,
    goshawk: [Option<showroom::Pass>; MAX_MODELS],
    authored_passes: Vec<showroom::AuthoredPass>,
    authored_count: usize,
}
impl Pass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Result<Self, String> {
        Self::with_authored(device, queue, format, Arc::default())
    }

    pub(crate) fn with_authored(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        authored: Arc<AuthoredHulls>,
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
            authored,
            srgb: format.is_srgb(),
            format,
            goshawk: Default::default(),
            authored_passes: Vec::new(),
            authored_count: 0,
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
        if (frame.viewport.width - logical_size[0]).abs() > 0.01
            || (frame.viewport.height - logical_size[1]).abs() > 0.01
        {
            self.goshawk = Default::default();
            return Err("docked frame viewport no longer matches the window".into());
        }
        if let Err(reason) = self.frame.compose_with_authored(
            frame,
            &self.model,
            self.srgb,
            planet_system,
            &self.authored,
        ) {
            self.goshawk = Default::default();
            return Err(reason);
        }
        queue.write_buffer(&self.shapes, 0, bytemuck::cast_slice(&self.frame.shapes));
        queue.write_buffer(&self.text, 0, bytemuck::cast_slice(&self.frame.text));
        queue.write_buffer(&self.models, 0, bytemuck::cast_slice(&self.frame.models));
        self.prepare_goshawk(device, queue, frame, logical_size, physical_size)?;
        let result = self.prepare_authored(device, queue, frame, logical_size, physical_size);
        if result.is_err() {
            self.goshawk = Default::default();
        }
        result
    }

    fn prepare_authored(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &Frame,
        logical_size: [f64; 2],
        physical_size: [u32; 2],
    ) -> Result<(), String> {
        self.authored_count = 0;
        let scale = std::array::from_fn(|axis| f64::from(physical_size[axis]) / logical_size[axis]);
        for preview in &frame.models {
            let visual = match self.authored.visual_for(preview) {
                Some(visual) => Some(visual),
                None => crate::authored_hull::stock_visual_for(preview)?,
            };
            let Some(visual) = visual else {
                continue;
            };
            let Some(bounds) = intersection(preview.bounds, frame.viewport.canvas()) else {
                continue;
            };
            if self.authored_count == self.authored_passes.len() {
                self.authored_passes.push(showroom::AuthoredPass::new(
                    device,
                    queue,
                    self.format,
                    preview,
                    visual,
                )?);
            } else if !self.authored_passes[self.authored_count].matches(preview) {
                self.authored_passes[self.authored_count] =
                    showroom::AuthoredPass::new(device, queue, self.format, preview, visual)?;
            }
            self.authored_passes[self.authored_count]
                .prepare(queue, preview, bounds, scale, self.srgb)?;
            self.authored_count += 1;
        }
        Ok(())
    }

    fn prepare_goshawk(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &Frame,
        logical_size: [f64; 2],
        physical_size: [u32; 2],
    ) -> Result<(), String> {
        let previous = std::mem::take(&mut self.goshawk);
        let mut requests = [None; MAX_MODELS];
        let scale = std::array::from_fn(|axis| f64::from(physical_size[axis]) / logical_size[axis]);
        for preview in &frame.models {
            let Some(hull) = showroom::hull(preview) else {
                continue;
            };
            let Some(bounds) = intersection(preview.bounds, frame.viewport.canvas()) else {
                continue;
            };
            let request = requests
                .iter_mut()
                .find(|request| request.is_none())
                .ok_or("Too many original Showroom preview panes")?;
            *request = Some((hull, preview, bounds));
        }
        let requested = requests
            .each_ref()
            .map(|request| request.map(|(hull, _, _)| hull));
        let mut passes = retain_visible(previous, requested, showroom::Pass::hull);
        for (request, slot) in requests.into_iter().zip(&mut passes) {
            let Some((hull, preview, bounds)) = request else {
                continue;
            };
            if slot.is_none() {
                *slot = Some(showroom::Pass::for_hull(device, queue, self.format, hull)?);
            }
            let pass = slot.as_mut().ok_or("Missing original Showroom pane pass")?;
            pass.prepare(queue, preview, bounds, scale, self.srgb)?;
        }
        // Each active pane has its own uniform buffer. No inactive hull pass survives.
        self.goshawk = passes;
        Ok(())
    }
    /// The caller supplies a black-cleared colour target and fresh depth attachment for models.
    pub fn draw_models(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.model_pipeline);
        pass.set_vertex_buffer(0, self.models.slice(..));
        pass.draw(0..self.frame.models.len() as u32, 0..1);
        for preview in self.goshawk.iter().flatten() {
            preview.draw(pass);
        }
        for preview in &self.authored_passes[..self.authored_count] {
            preview.draw(pass);
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

    #[cfg(all(test, target_os = "macos"))]
    pub(crate) fn retained_original_hulls(&self) -> [Option<showroom::Hull>; MAX_MODELS] {
        self.goshawk
            .each_ref()
            .map(|slot| slot.as_ref().map(showroom::Pass::hull))
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_MODELS, retain_visible, showroom};
    use showroom::Hull::{Custom, Explorer, Medusa, Mk2, PeregrineMkI, PeregrineMkIII};
    use std::{cell::Cell, rc::Rc};

    struct Pane {
        hull: showroom::Hull,
        id: usize,
        live: Rc<Cell<usize>>,
    }

    impl Drop for Pane {
        fn drop(&mut self) {
            self.live.set(self.live.get() - 1);
        }
    }

    #[test]
    fn original_showroom_cache_reuses_only_the_same_hull_and_unused_pane() {
        let live = Rc::new(Cell::new(0));
        let mut cached: [Option<Pane>; MAX_MODELS] = Default::default();
        let mut next_id = 0;
        let mut frame = |requested: [Option<showroom::Hull>; MAX_MODELS]| {
            let mut panes =
                retain_visible(std::mem::take(&mut cached), requested, |pane| pane.hull);
            for (wanted, slot) in requested.into_iter().zip(&mut panes) {
                if let Some(hull) = wanted {
                    if slot.is_none() {
                        // Even during replacement, old inactive passes have already dropped.
                        assert!(live.get() < MAX_MODELS);
                        live.set(live.get() + 1);
                        *slot = Some(Pane {
                            hull,
                            id: next_id,
                            live: Rc::clone(&live),
                        });
                        next_id += 1;
                    }
                    assert_eq!(slot.as_ref().map(|pane| pane.hull), Some(hull));
                } else {
                    assert!(slot.is_none());
                }
            }
            assert_eq!(live.get(), requested.iter().flatten().count());
            cached = panes;
            cached
                .each_ref()
                .map(|slot| slot.as_ref().map(|pane| pane.id))
        };
        assert_eq!(frame([Some(Explorer), None]), [Some(0), None]);
        assert_eq!(frame([Some(Mk2), None]), [Some(1), None]);
        assert_eq!(frame([Some(Explorer), None]), [Some(2), None]);
        assert_eq!(frame([Some(Explorer), Some(Mk2)]), [Some(2), Some(3)]);
        assert_eq!(frame([Some(Mk2), Some(Explorer)]), [Some(3), Some(2)]);
        assert_eq!(frame([Some(Mk2), Some(Mk2)]), [Some(3), Some(4)]);
        assert_eq!(frame([Some(Explorer), Some(Explorer)]), [Some(5), Some(6)]);
        assert_eq!(frame([Some(Custom), Some(Mk2)]), [Some(7), Some(8)]);
        assert_eq!(frame([Some(Mk2), Some(Custom)]), [Some(8), Some(7)]);
        assert_eq!(frame([Some(Custom), Some(Custom)]), [Some(7), Some(9)]);
        for hull in [Medusa, PeregrineMkI, PeregrineMkIII] {
            let duplicate = frame([Some(hull), Some(hull)]);
            assert_ne!(duplicate[0], duplicate[1]);
        }
        assert_eq!(frame([None, None]), [None, None]);
        assert_eq!(live.get(), 0);
    }
}

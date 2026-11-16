use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use meridian_core::docking::{DockingPort, StationPose};
use meridian_core::{FlightView, PracticeBeaconSnapshot, PulseShot, Snapshot};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, event_loop::OwnedDisplayHandle, window::Window};

use crate::hud;
use crate::mesh::{self, Vertex};

pub struct StationScene<'a> {
    pub pose: StationPose,
    pub port: &'a DockingPort,
    pub docked: bool,
}

pub struct Renderer {
    flight_view: FlightView,
    surface: wgpu::Surface<'static>,
    pub window: Arc<Window>,
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    pipeline: wgpu::RenderPipeline,
    projection: wgpu::Buffer,
    projection_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    depth: wgpu::TextureView,
    ship: Vec<Vertex>,
    station: Vec<Vertex>,
    frame: Vec<Vertex>,
    vertex_capacity: usize,
    device_lost: Arc<AtomicBool>,
    hud_pipeline: wgpu::RenderPipeline,
    hud_vertices: wgpu::Buffer,
    hud_frame: Vec<Vertex>,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, display: OwnedDisplayHandle) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display),
        ));
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| e.to_string())?;
        eprintln!("Meridian adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|e| e.to_string())?;
        let device_lost = Arc::new(AtomicBool::new(false));
        let lost = device_lost.clone();
        device.set_device_lost_callback(move |reason, message| {
            eprintln!("Meridian graphics device lost: {reason:?}: {message}");
            lost.store(true, Ordering::Release);
        });
        let failed = device_lost.clone();
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            eprintln!("Meridian GPU error: {error}");
            failed.store(true, Ordering::Release);
        }));
        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("No supported surface configuration")?;
        if size.width > 0 && size.height > 0 {
            surface.configure(&device, &config);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Meridian flight WGSL"),
            source: wgpu::ShaderSource::Wgsl(include_str!("flight.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Meridian opaque pass"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Original Meridian HUD"),
            source: wgpu::ShaderSource::Wgsl(include_str!("hud.wgsl").into()),
        });
        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HUD overlay"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &hud_shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &hud_shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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
        let hud_vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bounded HUD vertices"),
            size: (hud::MAX_VERTICES * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let projection = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Projection"),
            contents: bytemuck::cast_slice(&[
                config.width as f32 / config.height as f32,
                1.6,
                1.0,
                20000.0,
            ]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let projection_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Projection binding"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: projection.as_entire_binding(),
            }],
        });
        let ship = mesh::load_ship()?;
        let station = mesh::load_station()?;
        let vertex_capacity = ship.len()
            + station.len()
            + 3 * 32 * 6
            + mesh::PULSE_VERTEX_COUNT
            + mesh::PRACTICE_VERTEX_COUNT
            + mesh::MISSILE_VERTEX_COUNT;
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reused frame vertices"),
            size: (vertex_capacity * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let depth = Self::depth_texture(&device, &config);
        Ok(Self {
            surface,
            window,
            flight_view: FlightView::Front,
            instance,
            device,
            queue,
            config,
            size,
            pipeline,
            projection,
            projection_group,
            vertices,
            depth,
            ship,
            station,
            frame: Vec::with_capacity(vertex_capacity),
            vertex_capacity,
            device_lost,
            hud_pipeline,
            hud_vertices,
            hud_frame: Vec::with_capacity(hud::MAX_VERTICES),
        })
    }

    fn depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }

    pub fn is_zero_sized(&self) -> bool {
        self.size.width == 0 || self.size.height == 0
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
        if self.is_zero_sized() {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.depth = Self::depth_texture(&self.device, &self.config);
        self.queue.write_buffer(
            &self.projection,
            0,
            bytemuck::cast_slice(&[size.width as f32 / size.height as f32, 1.6, 1.0, 20000.0]),
        );
    }

    pub fn set_flight_view(&mut self, view: FlightView) {
        self.flight_view = view;
    }

    pub fn draw(
        &mut self,
        snapshot: &Snapshot,
        station: Option<StationScene<'_>>,
        pulse: Option<&PulseShot>,
        missiles: Option<&meridian_core::MissileSnapshot>,
        practice: Option<&[PracticeBeaconSnapshot; 3]>,
        hud_lines: &[String],
    ) -> Result<(), String> {
        if self.device_lost.load(Ordering::Acquire) {
            return Err("Graphics device lost; exiting without changing any save".into());
        }
        if self.is_zero_sized() {
            return Ok(());
        }
        let (frame, reconfigure) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .map_err(|e| e.to_string())?;
                self.surface.configure(&self.device, &self.config);
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("Surface validation failed".into());
            }
        };
        let station = station.map(|scene| mesh::StationRender {
            vertices: &self.station,
            pose: scene.pose,
            port: scene.port,
            docked: scene.docked,
        });
        mesh::frame_vertices(snapshot, &self.ship, station, &mut self.frame)?;
        if let (Some(station), Some(beacons)) = (station, practice) {
            mesh::append_practice_beacons(snapshot, station, beacons, &mut self.frame)?;
        }
        if let (Some(station), Some(missiles)) = (station, missiles) {
            mesh::append_missiles(snapshot, station, missiles, &mut self.frame)?;
        }
        if let (Some(station), Some(shot)) = (station, pulse) {
            mesh::append_pulse(snapshot, station, shot, &mut self.frame)?;
        }
        mesh::apply_flight_view(self.flight_view, &mut self.frame);
        if self.frame.len() > self.vertex_capacity {
            return Err("Frame exceeded its declared vertex capacity".into());
        }
        self.queue
            .write_buffer(&self.vertices, 0, bytemuck::cast_slice(&self.frame));
        hud::vertices(
            hud_lines,
            self.config.width,
            self.config.height,
            &mut self.hud_frame,
        );
        self.queue
            .write_buffer(&self.hud_vertices, 0, bytemuck::cast_slice(&self.hud_frame));
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let colors = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.003,
                        g: 0.006,
                        b: 0.018,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &colors,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.projection_group, &[]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.draw(0..self.frame.len() as u32, 0..1);
        }
        {
            let colors = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &colors,
                ..Default::default()
            });
            pass.set_pipeline(&self.hud_pipeline);
            pass.set_vertex_buffer(0, self.hud_vertices.slice(..));
            pass.draw(0..self.hud_frame.len() as u32, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        frame.present();
        if reconfigure {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(())
    }
}

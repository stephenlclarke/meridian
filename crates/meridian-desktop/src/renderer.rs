use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use meridian_core::docking::{DockingPort, StationPose};
use meridian_core::{FlightView, PracticeBeaconSnapshot, PulseShot, Snapshot};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, event_loop::OwnedDisplayHandle, window::Window};

use crate::mesh::{self, Vertex};
use crate::{asteroid_render, hud};

pub struct StationScene<'a> {
    pub pose: StationPose,
    pub port: &'a DockingPort,
    pub docked: bool,
}

pub struct HudScene<'a> {
    pub lines: &'a [String],
    pub cockpit: Option<&'a hud::cockpit::Cockpit>,
    pub help: Option<&'a crate::help::Panel>,
}

pub struct SceneEffects<'a> {
    pub traffic: Option<&'a meridian_core::traffic::TrafficSnapshot>,
    pub pulse: Option<&'a PulseShot>,
    pub mining: Option<asteroid_render::Scene<'a>>,
    pub missiles: &'a [meridian_core::ProjectileSnapshot],
    pub practice: Option<&'a [PracticeBeaconSnapshot; 3]>,
    pub celestial: Option<&'a crate::celestial::Scene>,
    pub docked: Option<&'a crate::docked_screens::Frame>,
    pub docked_planet: Option<&'a crate::celestial::Scene>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presentation {
    Presented,
    Skipped,
}

pub struct Renderer {
    review_capture: crate::review_capture::Capture,
    review_identity: Option<crate::review_capture::ReviewId>,
    review_pixels: crate::review_capture::pixels::Pixels,
    flight_view: FlightView,
    celestial: crate::celestial::Pass,
    docked: crate::docked_render::Pass,
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
        let review_capture = crate::review_capture::Capture::from_environment();
        let lost_capture = review_capture.clone();
        let error_capture = review_capture.clone();
        let device_lost = Arc::new(AtomicBool::new(false));
        let lost = device_lost.clone();
        device.set_device_lost_callback(move |reason, message| {
            lost_capture.asynchronous_error((&reason, &message));
            eprintln!("Meridian graphics device lost: {reason:?}: {message}");
            lost.store(true, Ordering::Release);
        });
        let failed = device_lost.clone();
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            error_capture.asynchronous_error(&error);
            eprintln!("Meridian GPU error: {error}");
            failed.store(true, Ordering::Release);
        }));
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("No supported surface configuration")?;
        let capture_capabilities = review_capture
            .enabled()
            .then(|| surface.get_capabilities(&adapter));
        let review_pixels = crate::review_capture::pixels::Pixels::new(
            review_capture.clone(),
            capture_capabilities.as_ref(),
            &mut config,
        );
        if size.width > 0 && size.height > 0 {
            surface.configure(&device, &config);
        }
        let celestial = crate::celestial::Pass::new(&device, config.format);
        let docked = crate::docked_render::Pass::new(&device, &queue, config.format)?;
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
            + mesh::MISSILE_VERTEX_COUNT
            + asteroid_render::VERTEX_COUNT
            + asteroid_render::BEAM_VERTEX_COUNT
            + mesh::traffic::vertex_capacity(ship.len())
                .ok_or("Traffic vertex capacity overflow")?;
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reused frame vertices"),
            size: (vertex_capacity * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let depth = Self::depth_texture(&device, &config);
        Ok(Self {
            review_capture,
            review_identity: None,
            review_pixels,
            surface,
            window,
            flight_view: FlightView::Front,
            celestial,
            docked,
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

    pub fn logical_viewport(&self) -> [f64; 2] {
        let size = self.size.to_logical::<f64>(self.window.scale_factor());
        [size.width, size.height]
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

    fn prepare_hud_frame(
        lines: &[String],
        cockpit: Option<&hud::cockpit::Cockpit>,
        logical_viewport: impl FnOnce() -> [f64; 2],
        physical_viewport: [u32; 2],
        output: &mut Vec<Vertex>,
    ) {
        if cockpit.is_some() {
            let [width, height] = logical_viewport();
            hud::vertices_with_cockpit(
                lines,
                width.round() as u32,
                height.round() as u32,
                cockpit,
                output,
            );
        } else {
            hud::vertices(lines, physical_viewport[0], physical_viewport[1], output);
        }
    }

    fn prepare_and_submit_hud(
        lines: &[String],
        cockpit: Option<&hud::cockpit::Cockpit>,
        size: PhysicalSize<u32>,
        physical_viewport: [u32; 2],
        scale_factor: impl FnOnce() -> f64,
        output: &mut Vec<Vertex>,
        submit: impl FnOnce(&[u8]),
    ) {
        Self::prepare_hud_frame(
            lines,
            cockpit,
            || {
                let logical = size.to_logical::<f64>(scale_factor());
                [logical.width, logical.height]
            },
            physical_viewport,
            output,
        );
        submit(bytemuck::cast_slice(output.as_slice()));
    }

    fn upload_hud(
        &mut self,
        lines: &[String],
        cockpit: Option<&hud::cockpit::Cockpit>,
        help: Option<&crate::help::Panel>,
    ) {
        let scale = self.window.scale_factor();
        let logical = self.size.to_logical::<f64>(scale);
        if let Some(panel) = help {
            let width = logical.width.round() as u32;
            let height = logical.height.round() as u32;
            hud::vertices_with_cockpit(lines, width, height, cockpit, &mut self.hud_frame);
            hud::append_help(panel, width, height, &mut self.hud_frame);
            self.queue
                .write_buffer(&self.hud_vertices, 0, bytemuck::cast_slice(&self.hud_frame));
        } else {
            Self::prepare_and_submit_hud(
                lines,
                cockpit,
                self.size,
                [self.config.width, self.config.height],
                || scale,
                &mut self.hud_frame,
                |bytes| self.queue.write_buffer(&self.hud_vertices, 0, bytes),
            );
        }
    }

    fn draw_overlay(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        docked: Option<&crate::docked_screens::Frame>,
        help: Option<&crate::help::Panel>,
    ) {
        let colors = [Some(wgpu::RenderPassColorAttachment {
            view,
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
        if docked.is_some() {
            self.docked.draw_overlay(&mut pass);
        }
        if docked.is_none() || help.is_some() {
            pass.set_pipeline(&self.hud_pipeline);
            pass.set_vertex_buffer(0, self.hud_vertices.slice(..));
            pass.draw(0..self.hud_frame.len() as u32, 0..1);
        }
    }

    pub(crate) fn set_review_capture_identity(
        &mut self,
        id: Option<crate::review_capture::ReviewId>,
    ) {
        self.review_identity = id;
    }
    pub(crate) fn capture_review_preparation_failure(
        &self,
        id: Option<crate::review_capture::ReviewId>,
        error: &str,
    ) {
        if let Some(trace) = self.review_capture.begin(id) {
            trace.record("app_prepare_error", error);
        }
    }
    pub(crate) fn request_review_observation(
        &self,
        id: Option<crate::review_capture::ReviewId>,
    ) -> bool {
        !self.review_pixels.busy() && self.review_capture.request_observation(id)
    }
    pub(crate) fn acknowledge_review(
        &self,
        expected: (crate::docked_screens::Stamp, u64),
        actual: Option<(crate::docked_screens::Stamp, u64)>,
        accepted: bool,
    ) {
        self.review_capture.acknowledge(expected, actual, accepted);
    }
    pub fn draw(
        &mut self,
        snapshot: &Snapshot,
        station: Option<StationScene<'_>>,
        effects: SceneEffects<'_>,
        hud: HudScene<'_>,
    ) -> Result<Presentation, String> {
        self.review_pixels.service(&self.device);
        let trace = self.review_capture.begin_draw(self.review_identity.take());
        if let Some(t) = &trace {
            t.record(
                "host_surface",
                (
                    self.size.width,
                    self.size.height,
                    self.window.scale_factor(),
                    format!("{:?}", self.config),
                    snapshot.tick,
                ),
            );
            t.frame(effects.docked);
            t.record("observability", "API calls only; framebuffer readability, GPU completion and compositor display are UNOBSERVABLE; v1 first attempt only; v2 fixed latest ring, App ack and explicit F8 next-frame observation; no per-frame output");
        }
        let result = self.draw_inner(snapshot, station, effects, hud, trace.as_ref());
        if let Some(t) = trace {
            if let Some(tag) = t.pixel_request()
                && !matches!(&result, Ok(Presentation::Presented))
            {
                self.review_capture.event(
                    "pixels_unobservable",
                    (
                        tag,
                        "requested draw skipped/failed before target copy",
                        &result,
                    ),
                );
            }
            t.record("draw_return", &result);
            self.review_capture.finish_draw(&t, result.is_err());
        }
        result
    }
    fn draw_inner(
        &mut self,
        snapshot: &Snapshot,
        station: Option<StationScene<'_>>,
        effects: SceneEffects<'_>,
        hud: HudScene<'_>,
        trace: Option<&crate::review_capture::Attempt>,
    ) -> Result<Presentation, String> {
        let HudScene {
            lines: hud_lines,
            cockpit,
            help,
        } = hud;

        let SceneEffects {
            traffic,
            pulse,
            mining,
            missiles,
            practice,
            celestial,
            docked,
            docked_planet,
        } = effects;
        if self.device_lost.load(Ordering::Acquire) {
            if let Some(t) = trace {
                t.record("preflight", "device_lost_or_uncaptured_error_flag");
            }
            return Err("Graphics device lost; exiting without changing any save".into());
        }
        if self.is_zero_sized() {
            if let Some(t) = trace {
                t.record("preflight", "zero_sized_surface_skip");
            }
            return Ok(Presentation::Skipped);
        }
        if let Some(menu) = docked {
            let scale = self.window.scale_factor();
            let logical = self.size.to_logical::<f64>(scale);
            if let Some(t) = trace {
                t.record("logical_window", [logical.width, logical.height]);
            }
            self.docked.prepare(
                &self.queue,
                menu,
                [logical.width, logical.height],
                docked_planet.and_then(|s| s.planet.as_ref().map(|p| p.id.system)),
                trace,
            )?;
            self.celestial
                .prepare_docked(
                    &self.queue,
                    crate::celestial::DockedPreview {
                        scene: docked_planet,
                        snapshot: *snapshot,
                        request: menu.planets.first(),
                        viewport: [self.config.width, self.config.height],
                        scale,
                    },
                )
                .map_err(str::to_owned)?;
        } else {
            self.celestial
                .prepare(
                    &self.queue,
                    celestial,
                    *snapshot,
                    self.flight_view,
                    crate::celestial::Projection {
                        viewport: [self.config.width, self.config.height],
                        clip: [
                            0.0,
                            0.0,
                            self.config.width as f32,
                            self.config.height as f32,
                        ],
                        center: [
                            self.config.width as f32 / 2.0,
                            self.config.height as f32 / 2.0,
                        ],
                        focal: self.config.height as f32 * 0.8,
                        near: 1.0,
                        far: 20000.0,
                    },
                )
                .map_err(str::to_owned)?;
        }
        let acquired = self.surface.get_current_texture();
        if let Some(t) = trace {
            t.record(
                "surface_acquire",
                match &acquired {
                    wgpu::CurrentSurfaceTexture::Success(_) => "Success",
                    wgpu::CurrentSurfaceTexture::Suboptimal(_) => "Suboptimal",
                    wgpu::CurrentSurfaceTexture::Timeout => "Timeout",
                    wgpu::CurrentSurfaceTexture::Occluded => "Occluded",
                    wgpu::CurrentSurfaceTexture::Outdated => "Outdated",
                    wgpu::CurrentSurfaceTexture::Lost => "Lost",
                    wgpu::CurrentSurfaceTexture::Validation => "Validation",
                },
            );
        }
        let (frame, reconfigure) = match acquired {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(Presentation::Skipped);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                self.window.request_redraw();
                return Ok(Presentation::Skipped);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .map_err(|e| e.to_string())?;
                self.surface.configure(&self.device, &self.config);
                self.window.request_redraw();
                return Ok(Presentation::Skipped);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("Surface validation failed".into());
            }
        };
        if docked.is_none() {
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
            if let Some(station) = station {
                mesh::append_missiles(snapshot, station, missiles, &mut self.frame)?;
            }
            if let (Some(station), Some(traffic)) = (station, traffic) {
                mesh::traffic::append(snapshot, station, &self.ship, traffic, &mut self.frame)?;
            }
            if let (Some(station), Some(shot)) = (station, pulse)
                && !traffic
                    .is_some_and(|scene| mesh::traffic::contains_pulse(scene, snapshot.tick, shot))
            {
                mesh::append_pulse(snapshot, station, shot, &mut self.frame)?;
            }
            if let (Some(station), Some(mining)) = (station, mining) {
                let sun = celestial.and_then(|scene| scene.sun.map(|sun| sun.position));
                mining.append(snapshot, station, sun, &mut self.frame)?;
            }
            mesh::apply_flight_view(self.flight_view, &mut self.frame);
            if self.frame.len() > self.vertex_capacity {
                return Err("Frame exceeded its declared vertex capacity".into());
            }
            self.queue
                .write_buffer(&self.vertices, 0, bytemuck::cast_slice(&self.frame));
        }
        if docked.is_none() {
            self.upload_hud(hud_lines, cockpit, help);
        } else if help.is_some() {
            self.upload_hud(&[], None, help);
        }

        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let colors = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: if docked.is_some() { 0.0 } else { 0.003 },
                        g: if docked.is_some() { 0.0 } else { 0.006 },
                        b: if docked.is_some() { 0.0 } else { 0.018 },
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
            if docked.is_some() {
                self.celestial.draw_surfaces(&mut pass);
                pass.set_scissor_rect(0, 0, self.config.width, self.config.height);
                self.docked.draw_models(&mut pass);
                self.celestial.draw_halos(&mut pass);
            } else {
                self.celestial.draw_surfaces(&mut pass);
                pass.set_scissor_rect(0, 0, self.config.width, self.config.height);
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.projection_group, &[]);
                pass.set_vertex_buffer(0, self.vertices.slice(..));
                pass.draw(0..self.frame.len() as u32, 0..1);
                self.celestial.draw_halos(&mut pass);
            }
        }
        self.draw_overlay(&mut encoder, &view, docked, help);
        if let Some(t) = trace {
            t.record("draw_calls_encoded", (docked.map(|_| self.docked.capture_counts()), self.hud_frame.len(), docked.is_none() || help.is_some(), "docked models/shapes/text vertex ranges; HUD vertex count and HUD draw-enabled; zero counts still encode calls; no GPU completion observed"));
        }
        if let Some(tag) = trace.and_then(|t| t.pixel_request()) {
            self.review_pixels
                .encode(tag, &self.device, &mut encoder, &frame.texture);
        }
        self.queue.submit([encoder.finish()]);
        self.review_pixels.submitted();
        if let Some(t) = trace {
            t.record("queue_submit", "call returned; GPU completion UNOBSERVABLE");
        }
        self.window.pre_present_notify();
        frame.present();
        if let Some(t) = trace {
            t.record(
                "surface_present",
                "call returned (no Result); display/compositor success UNOBSERVABLE",
            );
        }
        if reconfigure {
            self.surface.configure(&self.device, &self.config);
            if let Some(t) = trace {
                t.record(
                    "surface_reconfigure",
                    "call returned after Suboptimal; no completion acknowledgement",
                );
            }
        }
        Ok(Presentation::Presented)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.review_capture.exit();
    }
}

#[cfg(test)]
mod preparation_tests;

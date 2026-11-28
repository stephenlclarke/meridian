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
mod targeting;

pub(crate) fn flight_projection(width: u32, height: u32) -> [f32; 4] {
    [width as f32 / height as f32, 1.6, 1.0, 20000.0]
}

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

struct TargetHud<'a> {
    snapshot: &'a meridian_core::TargetingSnapshot,
    camera: meridian_core::docking::RigidPose,
    details: Option<bool>,
    work_mesh: Option<Vec<Vertex>>,
}

pub struct SceneEffects<'a> {
    pub(crate) mule: Option<crate::mule_render::Placement<'a>>,
    pub traffic: Option<&'a meridian_core::traffic::TrafficSnapshot>,
    pub targeting: Option<&'a meridian_core::TargetingSnapshot>,
    pub target_details: Option<bool>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceRecovery {
    Configure,
    Recreate,
}

/// Resize can arrive repeatedly while a surface cannot present. Retain only the latest
/// CPU projection, rather than enqueueing a new GPU staging allocation for each event.
#[derive(Default)]
struct PendingProjection(Option<[f32; 4]>);

impl PendingProjection {
    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width != 0 && size.height != 0 {
            self.0 = Some(flight_projection(size.width, size.height));
        }
    }

    fn upload(&mut self, write: impl FnOnce([f32; 4])) {
        if let Some(projection) = self.0.take() {
            write(projection);
        }
    }
}

pub struct Renderer {
    flight_view: FlightView,
    player_hull: meridian_core::PlayerHullKind,
    medusa_preview: bool,
    player_medusa: Option<crate::mule_render::showroom::PlayerMedusaPass>,
    celestial: crate::celestial::Pass,
    mule: Option<crate::mule_render::Pass>,
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
    flight_size: PhysicalSize<u32>,
    pending_projection: PendingProjection,
    projection_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    depth: wgpu::TextureView,
    ship: Vec<Vertex>,
    target_mule_mesh: Option<Vec<Vertex>>,
    station: Vec<Vertex>,
    frame: Vec<Vertex>,
    vertex_capacity: usize,
    device_lost: Arc<AtomicBool>,
    hud_pipeline: wgpu::RenderPipeline,
    hud_vertices: wgpu::Buffer,
    hud_frame: Vec<Vertex>,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        display: OwnedDisplayHandle,
        authored: Arc<crate::authored_hull::AuthoredHulls>,
    ) -> Result<Self, String> {
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
        let celestial = crate::celestial::Pass::new(&device, config.format);
        let docked = if authored.is_empty() {
            crate::docked_render::Pass::new(&device, &queue, config.format)?
        } else {
            crate::docked_render::Pass::with_authored(&device, &queue, config.format, authored)?
        };
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
            size: ((hud::MAX_VERTICES + targeting::MAX_VERTICES) * std::mem::size_of::<Vertex>())
                as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let projection = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Projection"),
            contents: bytemuck::cast_slice(&flight_projection(config.width, config.height)),
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
            + asteroid_render::ORE_VERTEX_COUNT
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
            surface,
            window,
            flight_view: FlightView::Front,
            player_hull: meridian_core::PlayerHullKind::Peregrine,
            medusa_preview: false,
            player_medusa: None,
            celestial,
            mule: None,
            docked,
            instance,
            device,
            queue,
            config,
            size,
            pipeline,
            projection,
            flight_size: size,
            pending_projection: PendingProjection::default(),
            projection_group,
            vertices,
            depth,
            ship,
            target_mule_mesh: None,
            station,
            frame: Vec::with_capacity(vertex_capacity),
            vertex_capacity,
            device_lost,
            hud_pipeline,
            hud_vertices,
            hud_frame: Vec::with_capacity(hud::MAX_VERTICES + targeting::MAX_VERTICES),
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
    }

    pub fn set_flight_view(&mut self, view: FlightView) {
        self.flight_view = view;
    }

    pub(crate) fn set_player_hull(
        &mut self,
        hull: meridian_core::PlayerHullKind,
        medusa_preview: bool,
    ) {
        self.medusa_preview = medusa_preview;
        self.player_hull = hull;
    }

    fn targeting_camera(
        &self,
        snapshot: &Snapshot,
        station: &StationScene<'_>,
    ) -> Result<meridian_core::docking::RigidPose, String> {
        mesh::scene_camera_pose(
            snapshot,
            mesh::StationRender {
                vertices: &self.station,
                pose: station.pose,
                port: station.port,
                docked: station.docked,
                medusa_preview: self.medusa_preview,
            },
        )
    }

    pub fn scanner_viewport(&self) -> [u32; 2] {
        self.cockpit_viewport(false)
    }

    fn cockpit_viewport(&self, docked: bool) -> [u32; 2] {
        let size = self.size;
        if self.is_zero_sized() {
            return [1, 1];
        }
        let logical = size.to_logical::<f64>(self.window.scale_factor());
        let height = logical.height.round().max(1.) as u32;
        let width = logical.width.round() as u32;
        let top = if docked && width >= 320 && height >= 240 {
            hud::cockpit::Layout::new(width, height).top
        } else {
            hud::cockpit::flight_height(width, height)
        };
        let fraction = top / height as f32;
        [
            size.width,
            (size.height as f32 * fraction)
                .round()
                .clamp(1., size.height as f32) as u32,
        ]
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
        target: Option<TargetHud<'_>>,
    ) -> Result<(), String> {
        let scale = self.window.scale_factor();
        let logical = self.size.to_logical::<f64>(scale);
        if help.is_some() || target.is_some() {
            let width = logical.width.round() as u32;
            let height = logical.height.round() as u32;
            hud::vertices_with_cockpit(lines, width, height, cockpit, &mut self.hud_frame);
            if let Some(TargetHud {
                snapshot: target,
                camera,
                details,
                work_mesh,
            }) = target
            {
                if target.model == Some("Mule") && self.target_mule_mesh.is_none() {
                    self.target_mule_mesh = Some(crate::mule_render::targeting_vertices()?);
                }
                let model = match target.mode {
                    meridian_core::TargetMode::Station => self.station.as_slice(),
                    meridian_core::TargetMode::Tracking if target.model == Some("Mule") => {
                        self.target_mule_mesh.as_deref().unwrap_or(&[])
                    }
                    meridian_core::TargetMode::Tracking => self.ship.as_slice(),
                    meridian_core::TargetMode::Work(
                        meridian_core::equipment::WorkTargetRole::WreckTracking,
                    ) => self.ship.as_slice(),
                    meridian_core::TargetMode::Work(_) => work_mesh.as_deref().unwrap_or(&[]),
                };
                targeting::append(
                    &mut self.hud_frame,
                    [width, height],
                    targeting::Overlay {
                        target,
                        camera,
                        view: self.flight_view,
                        projection: flight_projection(
                            self.flight_size.width,
                            self.flight_size.height,
                        ),
                        flight_height: self.flight_size.height as f32 * height as f32
                            / self.size.height as f32,
                        details,
                        model,
                    },
                );
            }
            if let Some(panel) = help {
                hud::append_help(panel, width, height, &mut self.hud_frame);
            }
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
        Ok(())
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

    fn prepare_scene_surfaces(
        &mut self,
        snapshot: &Snapshot,
        effects: &SceneEffects<'_>,
    ) -> Result<(), String> {
        let SceneEffects {
            docked,
            celestial,
            docked_planet,
            ..
        } = *effects;
        if let Some(menu) = docked {
            let scale = self.window.scale_factor();
            let logical = self.size.to_logical::<f64>(scale);
            self.docked.prepare(
                &self.device,
                &self.queue,
                menu,
                [logical.width, logical.height],
                [self.size.width, self.size.height],
                docked_planet.and_then(|s| s.planet.as_ref().map(|p| p.id.system)),
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
                        viewport: [self.config.width, self.flight_size.height],
                        clip: [
                            0.0,
                            0.0,
                            self.config.width as f32,
                            self.flight_size.height as f32,
                        ],
                        center: [
                            self.config.width as f32 / 2.0,
                            self.flight_size.height as f32 / 2.0,
                        ],
                        focal: self.flight_size.height as f32 * 0.8,
                        near: 1.0,
                        far: 20000.0,
                    },
                )
                .map_err(str::to_owned)?;
        }
        Ok(())
    }

    fn prepare_flight_geometry(
        &mut self,
        snapshot: &Snapshot,
        station: Option<StationScene<'_>>,
        effects: SceneEffects<'_>,
    ) -> Result<(), String> {
        let SceneEffects {
            mule,
            traffic,
            pulse,
            mining,
            missiles,
            practice,
            celestial,
            ..
        } = effects;
        let station = station.map(|scene| mesh::StationRender {
            vertices: &self.station,
            pose: scene.pose,
            port: scene.port,
            docked: scene.docked,
            medusa_preview: self.medusa_preview,
        });
        if let (Some(placement), Some(station)) = (mule, station) {
            let scene = placement.for_camera(|| mesh::scene_camera_pose(snapshot, station))?;
            let camera = scene.observer;
            if self.mule.is_none() {
                self.mule = Some(crate::mule_render::Pass::new(
                    &self.device,
                    &self.queue,
                    self.config.format,
                    scene,
                )?);
            }
            if let Some(pass) = &mut self.mule {
                pass.prepare(
                    &self.queue,
                    Some(scene),
                    crate::mule_render::View {
                        camera,
                        flight: self.flight_view,
                        projection: flight_projection(
                            self.flight_size.width,
                            self.flight_size.height,
                        ),
                        tick: snapshot.tick,
                    },
                )?;
            }
        } else if let Some(pass) = &mut self.mule {
            pass.hide();
        }
        let internal_observer = mule.and_then(crate::mule_render::Placement::internal_observer);
        let medusa_camera = if let Some(station) = station {
            mesh::medusa_chase_camera(snapshot, station)?
        } else if self.player_hull == meridian_core::PlayerHullKind::Medusa {
            return Err("Active Medusa has no station presentation frame".into());
        } else {
            None
        };
        if let Some(camera) = medusa_camera.filter(|_| internal_observer.is_none()) {
            if self.player_medusa.is_none() {
                self.player_medusa = Some(crate::mule_render::showroom::PlayerMedusaPass::new(
                    &self.device,
                    &self.queue,
                    self.config.format,
                )?);
            }
            if let Some(pass) = &mut self.player_medusa {
                pass.prepare(
                    &self.queue,
                    mesh::player_pose(snapshot),
                    crate::mule_render::View {
                        camera,
                        flight: self.flight_view,
                        projection: flight_projection(
                            self.flight_size.width,
                            self.flight_size.height,
                        ),
                        tick: snapshot.tick,
                    },
                )?;
            }
        } else if let Some(pass) = &mut self.player_medusa {
            pass.hide();
        }
        mesh::frame_vertices_for_observer(
            snapshot,
            &self.ship,
            station,
            internal_observer,
            &mut self.frame,
        )?;
        let additional_start = self.frame.len();
        if let (Some(station), Some(beacons)) = (station, practice) {
            mesh::append_practice_beacons(snapshot, station, beacons, &mut self.frame)?;
        }
        if let Some(station) = station {
            mesh::append_missiles(snapshot, station, missiles, &mut self.frame)?;
        }
        let beam_view = if self.player_hull == meridian_core::PlayerHullKind::Medusa {
            self.flight_view
        } else {
            meridian_core::FlightView::Front
        };
        if let (Some(station), Some(traffic)) = (station, traffic) {
            mesh::traffic::append(
                snapshot,
                station,
                &self.ship,
                traffic,
                beam_view,
                &mut self.frame,
            )?;
        }
        if let (Some(station), Some(shot)) = (station, pulse)
            && !traffic
                .is_some_and(|scene| mesh::traffic::contains_pulse(scene, snapshot.tick, shot))
        {
            mesh::append_pulse(snapshot, station, shot, beam_view, &mut self.frame)?;
        }
        if let (Some(station), Some(mining)) = (station, mining) {
            let sun = celestial.and_then(|scene| scene.sun.map(|sun| sun.position));
            mining.append(snapshot, station, sun, &mut self.frame)?;
        }
        if let (Some(observer), Some(station)) = (internal_observer, station) {
            mesh::reframe_observer(
                &mut self.frame[additional_start..],
                mesh::scene_camera_pose(snapshot, station)?,
                observer,
            )?;
        }
        mesh::apply_flight_view(self.flight_view, &mut self.frame);
        if self.frame.len() > self.vertex_capacity {
            return Err("Frame exceeded its declared vertex capacity".into());
        }
        self.queue
            .write_buffer(&self.vertices, 0, bytemuck::cast_slice(&self.frame));
        Ok(())
    }

    fn encode_scene(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        docked: bool,
    ) {
        let colors = [Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: if docked { 0.0 } else { 0.003 },
                    g: if docked { 0.0 } else { 0.006 },
                    b: if docked { 0.0 } else { 0.018 },
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
        if docked {
            self.celestial.draw_surfaces(&mut pass);
            pass.set_scissor_rect(0, 0, self.config.width, self.config.height);
            self.docked.draw_models(&mut pass);
            pass.set_viewport(
                0.0,
                0.0,
                self.config.width as f32,
                self.config.height as f32,
                0.0,
                1.0,
            );
            self.celestial.draw_halos(&mut pass);
        } else {
            pass.set_viewport(
                0.0,
                0.0,
                self.flight_size.width as f32,
                self.flight_size.height as f32,
                0.0,
                1.0,
            );
            self.celestial.draw_surfaces(&mut pass);
            pass.set_scissor_rect(0, 0, self.config.width, self.config.height);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.projection_group, &[]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.draw(0..self.frame.len() as u32, 0..1);
            if let Some(player) = &self.player_medusa {
                player.draw(&mut pass);
            }
            if let Some(mule) = &self.mule {
                mule.draw_solid(&mut pass);
            }
            self.celestial.draw_halos(&mut pass);
            if let Some(mule) = &self.mule {
                mule.draw_shimmer(&mut pass);
            }
        }
    }

    /// Resolve drawable acquisition and recovery before any frame uploads can allocate staging.
    fn resolve_surface_acquisition(
        current: wgpu::CurrentSurfaceTexture,
        recover: impl FnOnce(SurfaceRecovery) -> Result<(), String>,
    ) -> Result<Option<(wgpu::SurfaceTexture, bool)>, String> {
        match current {
            wgpu::CurrentSurfaceTexture::Success(frame) => Ok(Some((frame, false))),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Ok(Some((frame, true))),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                recover(SurfaceRecovery::Configure)?;
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                recover(SurfaceRecovery::Recreate)?;
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Validation => Err("Surface validation failed".into()),
        }
    }

    fn recover_surface(&mut self, recovery: SurfaceRecovery) -> Result<(), String> {
        if recovery == SurfaceRecovery::Recreate {
            self.surface = self
                .instance
                .create_surface(self.window.clone())
                .map_err(|e| e.to_string())?;
        }
        self.surface.configure(&self.device, &self.config);
        self.window.request_redraw();
        Ok(())
    }

    /// Queue writes own native staging buffers until a submission completes. Nothing in
    /// the prepare callback may run for skipped or unrecoverable acquisitions.
    fn prepare_acquired_frame<T>(
        acquired: Result<Option<T>, String>,
        prepare: impl FnOnce() -> Result<(), String>,
    ) -> Result<Option<T>, String> {
        let Some(frame) = acquired? else {
            return Ok(None);
        };
        prepare()?;
        Ok(Some(frame))
    }

    pub fn draw(
        &mut self,
        snapshot: &Snapshot,
        station: Option<StationScene<'_>>,
        effects: SceneEffects<'_>,
        hud: HudScene<'_>,
    ) -> Result<Presentation, String> {
        let HudScene {
            lines: hud_lines,
            cockpit,
            help,
        } = hud;

        let docked = effects.docked;
        if self.device_lost.load(Ordering::Acquire) {
            return Err("Graphics device lost; exiting without changing any save".into());
        }
        if self.is_zero_sized() {
            return Ok(Presentation::Skipped);
        }
        let surface_result = self.surface.get_current_texture();
        let acquired = Self::resolve_surface_acquisition(surface_result, |recovery| {
            self.recover_surface(recovery)
        });
        let Some((frame, reconfigure)) = Self::prepare_acquired_frame(acquired, || {
            let flight_size = if let Some(instruments) = cockpit {
                let [width, height] = self
                    .cockpit_viewport(instruments.flight.mode == meridian_core::FlightMode::Docked);
                PhysicalSize::new(width, height)
            } else {
                self.size
            };
            if self.flight_size != flight_size {
                self.flight_size = flight_size;
                self.pending_projection.resize(flight_size);
            }
            self.pending_projection.upload(|projection| {
                self.queue
                    .write_buffer(&self.projection, 0, bytemuck::cast_slice(&projection))
            });
            self.prepare_scene_surfaces(snapshot, &effects)?;
            let target = match (effects.targeting, station.as_ref()) {
                (Some(target), Some(station)) => {
                    let work_mesh = match (target.tracked_fragment, target.identity) {
                        (Some(id), _) => effects
                            .mining
                            .as_ref()
                            .and_then(|scene| {
                                scene.belt.pending_ore.iter().find(|ore| ore.id == id)
                            })
                            .map(targeting::ore_model),
                        (
                            None,
                            meridian_core::TargetIdentity::Work(
                                meridian_core::equipment::WorkTargetId::Asteroid(id),
                            ),
                        ) => effects
                            .mining
                            .as_ref()
                            .and_then(|scene| scene.belt.bodies.iter().find(|body| body.id == id))
                            .map(targeting::asteroid_model),
                        _ => None,
                    };
                    Some(TargetHud {
                        snapshot: target,
                        camera: self.targeting_camera(snapshot, station)?,
                        details: effects.target_details,
                        work_mesh,
                    })
                }
                _ => None,
            };
            if docked.is_none() {
                self.prepare_flight_geometry(snapshot, station, effects)?;
            }
            if docked.is_none() {
                self.upload_hud(hud_lines, cockpit, help, target)?;
            } else if help.is_some() {
                self.upload_hud(&[], None, help, None)?;
            }
            Ok(())
        })?
        else {
            return Ok(Presentation::Skipped);
        };

        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.encode_scene(&mut encoder, &view, docked.is_some());
        self.draw_overlay(&mut encoder, &view, docked, help);
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        frame.present();
        if reconfigure {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(Presentation::Presented)
    }
}

#[cfg(test)]
mod preparation_tests;

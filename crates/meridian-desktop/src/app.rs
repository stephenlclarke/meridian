use std::{collections::HashSet, error::Error, sync::Arc, time::Instant};

use meridian_core::{FlightInput, FlightMode, MotionRefusal, StepError, World};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Fullscreen, Window, WindowId},
};

use crate::clock::FixedClock;
use crate::renderer::{Renderer, StationScene};

// This shell owns the World synchronously. No gameplay mutation occurs in renderer code.
struct App {
    world: World,
    renderer: Option<Renderer>,
    keys: HashSet<KeyCode>,
    paused: bool,
    focused: bool,
    occluded: bool,
    last_frame: Instant,
    clock: FixedClock,
    error: Option<String>,
    notice: Option<String>,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            world: World::at_original_station()?,
            renderer: None,
            keys: HashSet::new(),
            paused: false,
            focused: true,
            occluded: false,
            last_frame: Instant::now(),
            clock: FixedClock::default(),
            error: None,
            notice: None,
        })
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl ToString) {
        let message = error.to_string();
        eprintln!("Meridian: {message}");
        self.error = Some(message);
        event_loop.exit();
    }

    fn axis(&self, positive: KeyCode, negative: KeyCode) -> f32 {
        f32::from(u8::from(self.keys.contains(&positive)))
            - f32::from(u8::from(self.keys.contains(&negative)))
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;
        let active = self.focused && !self.occluded && !self.paused;
        let ticks = self.clock.ticks(elapsed, active);
        if active {
            let input = FlightInput {
                pitch: self.axis(KeyCode::ArrowUp, KeyCode::ArrowDown),
                yaw: self.axis(KeyCode::KeyD, KeyCode::KeyA),
                roll: self.axis(KeyCode::KeyE, KeyCode::KeyQ),
                throttle: self.axis(KeyCode::KeyW, KeyCode::KeyS),
                brake: self.keys.contains(&KeyCode::Space),
            };
            for _ in 0..ticks {
                if let Err(error) = self.world.step(input) {
                    if matches!(error, StepError::Collision | StepError::UnsafeEntry) {
                        self.notice = Some(error.to_string());
                        break;
                    }
                    self.fail(event_loop, format!("Simulation refused input: {error}"));
                    return;
                }
            }
        }
        if let Some(renderer) = &mut self.renderer {
            let snapshot = self.world.snapshot();
            let mode = match snapshot.mode {
                FlightMode::Docked => "DOCKED - L launch",
                FlightMode::Launching => "LAUNCHING",
                FlightMode::Flight => "FLIGHT - align with the station entrance to dock",
            };
            let contact_notice = snapshot.motion_refusal.map(|refusal| match refusal {
                MotionRefusal::StationContact => "Station contact: stopped at last clear position",
                MotionRefusal::EntryConditions => {
                    "Entry refused: align with the entrance and slow down"
                }
            });
            renderer.window.set_title(&format!(
                "Meridian | {mode} | speed {:.1} | tick {} | {} | {} | W/S thrust, arrows pitch, A/D yaw, Q/E roll, Space brake, P pause, F11 fullscreen",
                snapshot.speed,
                snapshot.tick,
                if active { "RUNNING" } else { "PAUSED" },
                contact_notice.or(self.notice.as_deref())
                    .unwrap_or("Click inside to control"),
            ));
            let station = snapshot
                .station
                .zip(self.world.station_port())
                .map(|(pose, port)| StationScene {
                    pose,
                    port,
                    docked: snapshot.mode == FlightMode::Docked,
                });
            if let Err(error) = renderer.draw(&snapshot, station) {
                self.fail(event_loop, error);
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Meridian - initializing graphics")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => return self.fail(event_loop, error),
        };
        match pollster::block_on(Renderer::new(window, event_loop.owned_display_handle())) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(error) => self.fail(event_loop, error),
        }
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if !self.renderer.as_ref().is_some_and(|r| r.window.id() == id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.keys.clear();
                self.last_frame = Instant::now();
                self.clock.reset();
                if let Some(renderer) = &self.renderer {
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                self.last_frame = Instant::now();
                self.clock.reset();
                if let Some(renderer) = &self.renderer {
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state == ElementState::Released {
                        self.keys.remove(&code);
                    } else if !event.repeat {
                        self.keys.insert(code);
                        match code {
                            KeyCode::Escape => event_loop.exit(),
                            KeyCode::KeyL => {
                                if self.paused {
                                    self.notice = Some("Resume with P before launching".into());
                                } else {
                                    self.notice = self
                                        .world
                                        .request_launch(self.world.snapshot().tick)
                                        .err()
                                        .map(|error| error.to_string());
                                }
                                if let Some(renderer) = &self.renderer {
                                    renderer.window.request_redraw();
                                }
                            }
                            KeyCode::KeyP => {
                                self.paused = !self.paused;
                                self.keys.clear();
                                self.last_frame = Instant::now();
                                self.clock.reset();
                                if let Some(renderer) = &self.renderer {
                                    renderer.window.request_redraw();
                                }
                            }
                            KeyCode::F11 => {
                                if let Some(renderer) = &self.renderer {
                                    let mode = if renderer.window.fullscreen().is_some() {
                                        None
                                    } else {
                                        Some(Fullscreen::Borderless(None))
                                    };
                                    renderer.window.set_fullscreen(mode);
                                    renderer.window.request_redraw();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(renderer) = &self.renderer {
            if self.occluded || !self.focused || self.paused || renderer.is_zero_sized() {
                event_loop.set_control_flow(ControlFlow::Wait);
            } else {
                event_loop.set_control_flow(ControlFlow::Poll);
                renderer.window.request_redraw();
            }
        }
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new()?;
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(std::io::Error::other(error).into());
    }
    Ok(())
}

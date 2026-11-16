use std::{
    collections::{HashSet, hash_map::RandomState},
    error::Error,
    hash::{BuildHasher, Hasher},
    sync::Arc,
    time::Instant,
};

use meridian_core::{
    FlightInput, FlightMode, MotionRefusal, PulseContact, PulseShot, StepError, World,
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Fullscreen, Window, WindowId},
};

use crate::clock::FixedClock;
use crate::profile::Profile;
use crate::renderer::{Renderer, StationScene};
use crate::session::Action;

// Ephemeral profile IDs are allocated by the host, never by the deterministic core.
fn fresh_profile_seed() -> u64 {
    loop {
        let seed = RandomState::new().build_hasher().finish();
        if seed != 0 && seed <= u64::MAX - 10 {
            return seed;
        }
    }
}

// A catch-up frame must retain a contact reported before a later clear tick.
fn step_frame(
    world: &mut World,
    input: FlightInput,
    ticks: usize,
    fire_held: bool,
    pulse: &mut Option<PulseShot>,
) -> Result<Option<MotionRefusal>, StepError> {
    let mut refusal = None;
    for _ in 0..ticks {
        world.step(input)?;
        let state = world.snapshot();
        refusal = state.motion_refusal.or(refusal);
        if state.mode != FlightMode::Flight
            || state.navigation.is_some_and(|nav| nav.travel.is_some())
        {
            *pulse = None;
        } else if fire_held && let Ok(shot) = world.request_fire_front_pulse(state.tick) {
            *pulse = Some(shot);
        }
    }
    Ok(refusal)
}

// This shell owns the World synchronously. No gameplay mutation occurs in renderer code.
struct App {
    game: Profile,
    quit_failed: bool,
    renderer: Option<Renderer>,
    keys: HashSet<KeyCode>,
    paused: bool,
    focused: bool,
    occluded: bool,
    last_frame: Instant,
    clock: FixedClock,
    error: Option<String>,
    notice: Option<String>,
    pulse: Option<PulseShot>,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            game: Profile::open(crate::profile::directory_from_args(
                std::env::args_os().skip(1),
            )?)?,
            quit_failed: false,
            renderer: None,
            keys: HashSet::new(),
            paused: false,
            focused: true,
            occluded: false,
            last_frame: Instant::now(),
            clock: FixedClock::default(),
            error: None,
            notice: None,
            pulse: None,
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

    fn request_redraw(&self) {
        if let Some(renderer) = &self.renderer {
            renderer.window.request_redraw();
        }
    }

    fn reset_input(&mut self) {
        self.keys.clear();
        self.pulse = None;
        self.last_frame = Instant::now();
        self.clock.reset();
        self.game.session.invalidate();
        if self.game.playing()
            && let Err(error) = self.game.world.request_pause(
                self.paused || !self.focused || self.occluded,
                self.game.world.snapshot().tick,
            )
        {
            self.notice = Some(error.to_string());
        }
        self.request_redraw();
    }

    fn key(&mut self, code: KeyCode, event_loop: &ActiveEventLoop) {
        match code {
            KeyCode::Escape => self.request_quit(event_loop),
            KeyCode::KeyQ if self.quit_failed => event_loop.exit(),
            KeyCode::KeyN if !self.game.playing() => {
                self.notice = Some(match self.game.new_commander(fresh_profile_seed()) {
                    Ok(true) => {
                        self.sync_profile_controls();
                        "New commander ready".into()
                    }
                    Ok(false) => {
                        "Press N again to confirm new game; existing save is not changed yet".into()
                    }
                    Err(error) => error,
                });
            }
            KeyCode::KeyC if !self.game.playing() => self.load_profile(),
            KeyCode::F9 if self.game.playing() => self.load_profile(),
            KeyCode::F5 if self.game.playing() => {
                self.notice = Some(match self.game.save() {
                    Ok(message) => message,
                    Err(error) => error,
                });
            }
            KeyCode::KeyP if self.game.playing() => {
                self.quit_failed = false;
                self.paused = !self.paused;
                self.reset_input();
            }
            KeyCode::F11 => {
                if let Some(renderer) = &self.renderer {
                    let mode = if renderer.window.fullscreen().is_some() {
                        None
                    } else {
                        Some(Fullscreen::Borderless(None))
                    };
                    renderer.window.set_fullscreen(mode);
                }
            }
            _ => self.game_key(code),
        }
        self.request_redraw();
    }

    fn sync_profile_controls(&mut self) {
        self.paused = self.game.world.mission_snapshot().is_some_and(|m| m.paused);
        self.quit_failed = false;
        self.reset_input();
    }

    fn load_profile(&mut self) {
        match self.game.continue_saved() {
            Ok(()) => {
                self.sync_profile_controls();
                self.notice =
                    Some("Commander continued; select the actual current crew with T".into());
            }
            Err(error) => {
                self.notice = Some(format!(
                    "Continue refused; save and current game retained: {error}"
                ))
            }
        }
    }

    fn request_quit(&mut self, event_loop: &ActiveEventLoop) {
        match self.game.prepare_quit() {
            Ok(()) => event_loop.exit(),
            Err(error) => {
                self.paused = true;
                self.reset_input();
                self.quit_failed = true;
                self.notice = Some(format!("P resumes; Q quits without saving again. {error}"));
                self.request_redraw();
            }
        }
    }

    fn game_key(&mut self, code: KeyCode) {
        let action = match code {
            KeyCode::KeyL => Action::Launch,
            KeyCode::KeyJ => Action::Jump,
            KeyCode::KeyT => Action::Contact,
            KeyCode::KeyR => Action::Review,
            KeyCode::KeyV => Action::ReviewAll,
            KeyCode::KeyX => Action::ReviewAbandonment,
            KeyCode::KeyI => Action::MissionDetails,
            KeyCode::F8 => Action::ReviewPracticeReception,
            KeyCode::F10 => Action::ReviewPractice,
            KeyCode::Enter => Action::Confirm,
            KeyCode::KeyB => Action::Back,
            KeyCode::KeyF => Action::Refuel,
            KeyCode::KeyG => Action::BuySelected,
            KeyCode::KeyH => Action::SellSelected,
            KeyCode::KeyM => Action::ToggleMarket,
            KeyCode::KeyO => Action::ToggleEquipment,
            KeyCode::KeyK => Action::PurchaseCargoBay,
            KeyCode::KeyY => Action::FitCargoBay,
            KeyCode::KeyU => Action::UnfitCargoBay,
            KeyCode::BracketLeft => Action::PreviousCommodity,
            KeyCode::BracketRight => Action::NextCommodity,
            _ => return,
        };
        self.notice = Some(match self.game.command(action) {
            Ok(message) => message,
            Err(error) => error,
        });
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;
        let active = self.focused && !self.occluded && !self.paused && self.game.playing();
        let ticks = self.clock.ticks(elapsed, active);
        let mut frame_refusal = None;
        if active {
            let input = FlightInput {
                pitch: self.axis(KeyCode::ArrowUp, KeyCode::ArrowDown),
                yaw: self.axis(KeyCode::KeyD, KeyCode::KeyA),
                roll: self.axis(KeyCode::KeyE, KeyCode::KeyQ),
                throttle: self.axis(KeyCode::KeyW, KeyCode::KeyS),
                brake: self.keys.contains(&KeyCode::Space),
            };
            match step_frame(
                &mut self.game.world,
                input,
                ticks,
                self.keys.contains(&KeyCode::KeyZ),
                &mut self.pulse,
            ) {
                Ok(refusal) => frame_refusal = refusal,
                Err(error) if matches!(error, StepError::Collision | StepError::UnsafeEntry) => {
                    self.notice = Some(error.to_string());
                }
                Err(error) => {
                    self.fail(event_loop, format!("Simulation refused input: {error}"));
                    return;
                }
            }
        }
        if let Some(refusal) = frame_refusal {
            self.notice = Some(
                match refusal {
                    MotionRefusal::StationContact => {
                        "Station contact: stopped at last clear position"
                    }
                    MotionRefusal::EntryConditions => {
                        "Entry refused: align with entrance and slow down"
                    }
                }
                .into(),
            );
        }
        let snapshot = self.game.world.snapshot();
        // The beam is a short-lived view of the committed shot, never saved or re-fired.
        self.pulse = self.pulse.filter(|shot| {
            active
                && snapshot.mode == FlightMode::Flight
                && snapshot.tick.saturating_sub(shot.tick) < 5
        });
        let pulse_notice = self.pulse.map(|shot| match shot.contact {
            PulseContact::Clear => "Pulse fired",
            PulseContact::StationBlocked { .. } => "Pulse stopped by station hull",
            PulseContact::PracticeBeacon {
                first_hit: true, ..
            } => "Practice beacon hit",
            PulseContact::PracticeBeacon {
                first_hit: false, ..
            } => "Practice beacon already counted",
        });
        let notice = pulse_notice.or(self.notice.as_deref());
        let hud_lines = self.game.lines(active, notice);
        if let Some(renderer) = &mut self.renderer {
            let mode = match snapshot.mode {
                FlightMode::Docked => "DOCKED - L launch",
                FlightMode::Launching => "LAUNCHING",
                FlightMode::Flight => "FLIGHT - align with the station entrance to dock",
            };
            let contact_notice =
                frame_refusal
                    .or(snapshot.motion_refusal)
                    .map(|refusal| match refusal {
                        MotionRefusal::StationContact => {
                            "Station contact: stopped at last clear position"
                        }
                        MotionRefusal::EntryConditions => {
                            "Entry refused: align with the entrance and slow down"
                        }
                    });
            renderer.window.set_title(&format!(
                "Meridian | {mode} | speed {:.1} | tick {} | {} | {} | W/S thrust, arrows pitch, A/D yaw, Q/E roll, Space brake, Z fire, P pause, F11 fullscreen",
                snapshot.speed,
                snapshot.tick,
                if active { "RUNNING" } else { "PAUSED" },
                contact_notice.or(notice)
                    .unwrap_or("Click inside to control"),
            ));
            let station =
                snapshot
                    .station
                    .zip(self.game.world.station_port())
                    .map(|(pose, port)| StationScene {
                        pose,
                        port,
                        docked: snapshot.mode == FlightMode::Docked,
                    });
            let practice = self.game.world.practice_snapshot();
            if let Err(error) = renderer.draw(
                &snapshot,
                station,
                self.pulse.as_ref(),
                practice.as_ref().and_then(|state| state.beacons.as_ref()),
                &hud_lines,
            ) {
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
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(640.0, 360.0));
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
            WindowEvent::CloseRequested => self.request_quit(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.reset_input();
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                self.reset_input();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state == ElementState::Released {
                        self.keys.remove(&code);
                    } else if !event.repeat && self.focused && !self.occluded {
                        self.keys.insert(code);
                        self.key(code, event_loop);
                    }
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(renderer) = &self.renderer {
            if self.occluded
                || !self.focused
                || self.paused
                || !self.game.playing()
                || renderer.is_zero_sized()
            {
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

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_core::{FIXED_DELTA_SECONDS, FlightStart};

    #[test]
    fn held_fire_uses_advanced_ticks_and_respects_core_cadence() {
        let mut world = World::new_commander(123).unwrap_or_else(|error| panic!("{error}"));
        world
            .request_launch(world.snapshot().tick)
            .unwrap_or_else(|error| panic!("{error}"));
        let input = FlightInput::default();
        let mut pulse = None;
        step_frame(&mut world, input, 1, false, &mut pulse)
            .unwrap_or_else(|error| panic!("{error}"));
        let tick = world.snapshot().tick;
        step_frame(&mut world, input, 0, true, &mut pulse)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(pulse, None);
        step_frame(&mut world, input, 1, true, &mut pulse)
            .unwrap_or_else(|error| panic!("{error}"));
        let first = pulse.unwrap_or_else(|| panic!("Held trigger did not fire on the next tick"));
        assert_eq!(first.tick, tick + 1);

        let cooldown = usize::from(meridian_core::PULSE_COOLDOWN_TICKS);
        step_frame(&mut world, input, cooldown - 1, true, &mut pulse)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(pulse, Some(first));
        step_frame(&mut world, input, 1, true, &mut pulse)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            pulse.map(|shot| shot.tick),
            Some(first.tick + cooldown as u64)
        );

        let second = pulse;
        step_frame(&mut world, input, cooldown, false, &mut pulse)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(pulse, second);
        assert_eq!(
            world
                .pulse_laser_snapshot()
                .map(|laser| laser.cooldown_ticks),
            Some(0)
        );
    }

    #[test]
    fn catch_up_frame_keeps_entry_refusal_after_a_later_clear_tick() {
        let mut world = World::in_flight_at_original_station(FlightStart {
            position: [0.0, 0.0, 260.0],
            orientation: [0.0, 1.0, 0.0, 0.0],
            speed: 9.0,
        })
        .unwrap_or_else(|error| panic!("{error}"));
        for _ in 0..100 {
            let state = world.snapshot();
            if state.position[2] - state.speed * 18.0 * FIXED_DELTA_SECONDS <= 136.32 {
                break;
            }
            world
                .step(FlightInput::default())
                .unwrap_or_else(|error| panic!("{error}"));
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        let before = world.snapshot();
        let notice = step_frame(&mut world, FlightInput::default(), 2, false, &mut None)
            .unwrap_or_else(|error| panic!("{error}"));
        let after = world.snapshot();
        assert_eq!(notice, Some(MotionRefusal::EntryConditions));
        assert_eq!(after.motion_refusal, None);
        assert_eq!(after.position, before.position);
        assert_eq!(after.speed, 0.0);
        assert_eq!(after.mode, FlightMode::Flight);
        assert_eq!(after.tick, before.tick + 2);
    }
}

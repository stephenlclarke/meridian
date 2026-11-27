mod audio;
mod emp;
mod manual_work;
mod mule_presentation;
mod mule_validation;
mod occupied_cut;
mod target_computer;
use std::{
    collections::{HashSet, hash_map::RandomState},
    error::Error,
    hash::{BuildHasher, Hasher},
    io::Write,
    sync::Arc,
    time::Instant,
};

use meridian_core::{
    FlightInput, FlightMode, FlightView, MotionRefusal, PulseContact, PulseShot, StepError, World,
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::{Fullscreen, Window, WindowId},
};

use crate::audio::host::DesktopAudio;
use crate::clock::FixedClock;
use crate::profile::Profile;
use crate::renderer::{HudScene, Presentation, Renderer, SceneEffects, StationScene};
use crate::session::Action;
use crate::{asteroid_render, authored_hull::AuthoredHulls, docked::Docked, docked_screens as ui};

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
fn player_destroyed(world: &World) -> bool {
    world
        .combat_snapshot()
        .is_some_and(|combat| combat.player.destroyed)
}

fn step_frame(
    world: &mut World,
    input: FlightInput,
    ticks: usize,
    fire_held: bool,
    pulse: &mut Option<PulseShot>,
    mining: &mut Option<asteroid_render::MiningVisual>,
    mut accepted_discharge: impl FnMut(),
) -> Result<Option<MotionRefusal>, StepError> {
    let mut refusal = None;
    for _ in 0..ticks {
        if player_destroyed(world) {
            *pulse = None;
            *mining = None;
            break;
        }
        world.step(input)?;
        let state = world.snapshot();
        refusal = state.motion_refusal.or(refusal);
        if player_destroyed(world) {
            *pulse = None;
            *mining = None;
            break;
        }
        if state.mode != FlightMode::Flight
            || state.navigation.is_some_and(|nav| nav.travel.is_some())
        {
            *pulse = None;
            *mining = None;
        } else if fire_held
            && (world.selected_laser_view() != FlightView::Front
                || (!world.pilot_operates_selected_tool()
                    && occupied_cut::ready_target(world).is_none()))
        {
            if world.selected_laser_view() == FlightView::Front
                && let Ok(tool) = world.mining_tool_authority()
            {
                if let Ok(shot) = world.request_mining(tool, state.tick) {
                    *mining = Some(asteroid_render::MiningVisual {
                        opening: state,
                        shot,
                    });
                    accepted_discharge();
                }
            } else if let Ok(shot) = world.request_fire_selected_laser(state.tick) {
                *pulse = Some(shot);
                accepted_discharge();
            }
        }
    }
    Ok(refusal)
}

struct PreparedPresentation {
    hud_lines: Vec<String>,
    docked_frame: Option<ui::Frame>,
}

/// Acknowledge only the actual requested, authenticated preview after a successful presentation.
fn take_studio_ready(
    pending: &mut Option<String>,
    authored: &AuthoredHulls,
    frame: Option<&ui::Frame>,
    presented: &Result<Presentation, String>,
) -> bool {
    let Some(id) = pending.as_ref() else {
        return false;
    };
    if !matches!(presented, Ok(Presentation::Presented))
        || !frame.is_some_and(|frame| {
            frame.screen == Some(ui::Screen::Showroom)
                && frame.models.last().is_some_and(|model| {
                    model.object.as_str() == id.as_str() && authored.visual_for(model).is_some()
                })
        })
    {
        return false;
    }
    *pending = None;
    true
}

// This shell owns the World synchronously. No gameplay mutation occurs in renderer code.
// Retain the frame-wide contact even when a later catch-up tick is clear.
#[derive(Debug, PartialEq, Eq)]
enum FrameAdvance {
    Continue(Option<MotionRefusal>),
    Fatal(StepError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuitRecovery {
    Resume,
    Discard,
}

const DEFAULT_FLIGHT_WINDOW: [u32; 2] = [1280, 720];
pub(crate) const MIN_WINDOW_LOGICAL: [f64; 2] = [640.0, 360.0];
const DESTROYED_NOTICE: &str =
    "DESTROYED | F9 RESTORE | N NEW (CONFIRM) | F5 SAVE TERMINAL | ESC QUIT";
const DESTROYED_QUIT_NOTICE: &str =
    "DESTROYED | F9 RESTORE | F5 SAVE TERMINAL | Q QUIT WITHOUT SAVE";

#[derive(Clone, Default, Debug, PartialEq, Eq)]
struct Notice {
    text: Option<String>,
    expires: Option<Instant>,
    missile: bool,
}

impl Notice {
    fn set(&mut self, message: String) {
        self.text = Some(message);
        self.expires = Some(Instant::now() + std::time::Duration::from_secs(4));
        self.missile = false;
    }

    fn expire(&mut self, now: Instant) -> bool {
        if self.expires.is_some_and(|deadline| deadline <= now) {
            *self = Self::default();
            return true;
        }
        false
    }
}

struct App {
    game: Profile,
    authored_hulls: Arc<AuthoredHulls>,
    studio_ready: Option<String>,
    audio: Option<DesktopAudio>,
    validation_view: FlightView,
    target_details: Option<bool>,
    validation_timing: Option<mule_validation::FrameTiming>,
    celestial: crate::celestial::WorldAdapter,
    quit_failed: bool,
    terminal_redraw_pending: bool,
    renderer: Option<Renderer>,
    keys: HashSet<KeyCode>,
    paused: bool,
    help: Option<crate::help::Panel>,
    occupied_cut: Option<meridian_core::OccupiedCutReview>,
    xyzzy: Option<crate::xyzzy::Menu>,
    focused: bool,
    docked: Option<Docked>,
    commander_open: bool,
    modifiers: ModifiersState,
    cursor: [f64; 2],
    dragging: bool,
    wheel: f64,
    occluded: bool,
    last_frame: Instant,
    clock: FixedClock,
    error: Option<String>,
    notice: Notice,
    pulse: Option<PulseShot>,
    mining: Option<asteroid_render::MiningVisual>,
    missile_bodies: Vec<meridian_core::ProjectileSnapshot>,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        let mut app =
            Self::from_launch(mule_validation::Launch::parse(std::env::args_os().skip(1))?)?;
        app.open_audio();
        Ok(app)
    }
    fn from_launch(launch: mule_validation::Launch) -> Result<Self, Box<dyn Error>> {
        let view = launch.camera_view();
        let mut app = match launch {
            mule_validation::Launch::Profile(directory) => {
                Self::open_profile(directory, Arc::default())?
            }
            mule_validation::Launch::Authored {
                directory,
                packages,
                open_showroom,
            } => {
                // Reject every invalid/conflicting input before opening a profile or its save.
                let authored = Arc::new(AuthoredHulls::load(&packages)?);
                let mut app = Self::open_profile(directory, authored)?;
                if open_showroom {
                    if !app.has_berth() {
                        return Err("--open-authored-showroom requires a docked commander".into());
                    }
                    let mut menu = app.new_docked_menu()?;
                    menu.open_authored_showroom(&app.game.world)?;
                    app.docked = Some(menu);
                    app.studio_ready = Some(
                        app.authored_hulls
                            .inspection_record()
                            .ok_or("Missing isolated authored hull for Studio readiness")?
                            .id
                            .clone(),
                    );
                    // Initial presentation only; never simulate a key or change World state.
                    app.commander_open = false;
                }
                app
            }
            launch => Self::with_profile(launch.open()?),
        };
        app.validation_view = view;
        Ok(app)
    }

    fn open_profile(
        directory: std::path::PathBuf,
        authored_hulls: Arc<AuthoredHulls>,
    ) -> Result<Self, Box<dyn Error>> {
        let (game, notice) = Profile::open_startup(directory, fresh_profile_seed())?;
        let mut app = Self::with_profile(game);
        app.authored_hulls = authored_hulls;
        app.commander_open = true;
        app.paused = app
            .game
            .world
            .mission_snapshot()
            .is_some_and(|mission| mission.paused);
        if let Some(notice) = notice {
            app.notice.set(notice);
        } else if player_destroyed(&app.game.world) {
            app.notice.set(DESTROYED_NOTICE.into());
        } else if let Some(first) = app.authored_hulls.records().next() {
            app.notice.set(format!(
                "Local hull loaded. At a station press 3, search {}.",
                first.id
            ));
        }
        Ok(app)
    }

    fn with_profile(game: Profile) -> Self {
        let internal = game.is_internal_validation();
        Self {
            game,
            authored_hulls: Arc::default(),
            studio_ready: None,
            audio: None,
            validation_view: FlightView::Front,
            target_details: None,
            validation_timing: internal.then(mule_validation::FrameTiming::default),
            quit_failed: false,
            terminal_redraw_pending: false,
            celestial: crate::celestial::WorldAdapter::default(),
            renderer: None,
            keys: HashSet::new(),
            paused: internal,
            help: None,
            occupied_cut: None,
            xyzzy: None,
            focused: true,
            docked: None,
            commander_open: false,
            modifiers: ModifiersState::empty(),
            cursor: [0.0; 2],
            dragging: false,
            wheel: 0.0,
            occluded: false,
            last_frame: Instant::now(),
            clock: FixedClock::default(),
            error: None,
            notice: Notice::default(),
            pulse: None,
            mining: None,
            missile_bodies: Vec::with_capacity(crate::mesh::MAX_MISSILE_PROJECTILES),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl ToString) {
        let message = error.to_string();
        eprintln!("Meridian: {message}");
        self.error = Some(message);
        self.shutdown_audio();
        event_loop.exit();
    }

    fn axis(&self, positive: KeyCode, negative: KeyCode) -> f32 {
        f32::from(u8::from(self.keys.contains(&positive)))
            - f32::from(u8::from(self.keys.contains(&negative)))
    }

    fn simulation_active(&self) -> bool {
        self.focused
            && !self.occluded
            && !self.paused
            && !self.commander_open
            && !self.modal_open()
            && self.game.playing()
            && !player_destroyed(&self.game.world)
    }

    fn terminal_cockpit_visible(&self) -> bool {
        self.game.playing()
            && !self.commander_open
            && !self.game.is_internal_validation()
            && player_destroyed(&self.game.world)
    }

    fn terminal_retry_needed(&self, zero_sized: bool) -> bool {
        self.terminal_redraw_pending
            && self.terminal_cockpit_visible()
            && !self.occluded
            && !zero_sized
    }
    fn record_terminal_presentation(&mut self, skipped: bool) {
        self.terminal_redraw_pending = self.terminal_cockpit_visible() && skipped;
    }

    fn timed_hud_notices(&self) -> bool {
        self.game.playing()
            && matches!(
                self.game.world.snapshot().mode,
                FlightMode::Docked | FlightMode::Flight
            )
            && !self.game.is_internal_validation()
            && !self.commander_open
            && !self.modal_open()
            && !self.quit_failed
            && !player_destroyed(&self.game.world)
    }

    fn expire_hud_notice(&mut self, now: Instant) -> bool {
        self.timed_hud_notices() && self.notice.expire(now)
    }

    fn modal_open(&self) -> bool {
        self.help.is_some() || self.xyzzy.is_some() || self.occupied_cut.is_some()
    }

    fn request_redraw(&self) {
        if let Some(renderer) = &self.renderer {
            renderer.window.request_redraw();
        }
    }

    fn clear_held_input(&mut self) {
        self.cancel_emp_input();
        self.keys.clear();
        self.dragging = false;
        self.wheel = 0.0;
        self.pulse = None;
        self.mining = None;
        self.last_frame = Instant::now();
        self.clock.reset();
    }

    fn help_panel(&self) -> crate::help::Panel {
        let mut panel = self.game_help_panel();
        panel.lines.push(self.audio_help_line());
        panel
    }

    fn game_help_panel(&self) -> crate::help::Panel {
        if let Some(menu) = &self.xyzzy {
            return menu.help();
        }
        if self.commander_open && self.game.playing() {
            return crate::help::Panel::commander(self.has_berth());
        }
        if !self.game.playing() {
            return crate::help::Panel::title();
        }
        if self.menu_active()
            && let Some(menu) = &self.docked
        {
            return menu.help_panel();
        }
        let review = self.game.session.help_review();
        if self.game.world.snapshot().mode == FlightMode::Docked {
            crate::help::Panel::docked(review)
        } else {
            let world = &self.game.world;
            let facing = world.selected_laser_view();
            let slot = meridian_core::ModuleSlot::laser(facing);
            let weapon = world.modular_equipment_snapshot().and_then(|modules| {
                modules
                    .items
                    .into_iter()
                    .find(|item| item.slot == Some(slot))
                    .map(|item| item.kind.name())
            });
            crate::help::Panel::flight_facing(weapon, review, facing)
        }
    }

    fn route_help(&mut self, code: KeyCode, repeat: bool, text: Option<&str>) -> bool {
        let searching = self.xyzzy.is_none()
            && self.menu_active()
            && self.docked.as_ref().is_some_and(Docked::search_active);
        let question = text == Some("?") || (code == KeyCode::Slash && self.modifiers.shift_key());
        let toggle = question || (code == KeyCode::KeyH && !searching);
        let journal = code == KeyCode::KeyJ && self.modifiers.shift_key() && !searching;
        if self.help.is_some() {
            if !repeat && (toggle || journal || code == KeyCode::KeyH || code == KeyCode::Escape) {
                self.help = None;
                self.clear_held_input();
                self.request_redraw();
            }
            return true;
        }
        if journal && self.game.playing() && self.xyzzy.is_none() {
            if !repeat && let Some(progress) = self.game.world.unwritten_route_clue_progress() {
                self.help = Some(crate::help::Panel::clue_journal(&progress));
                self.clear_held_input();
                self.request_redraw();
            }
            return true;
        }
        if toggle {
            if !repeat {
                self.help = Some(self.help_panel());
                self.clear_held_input();
                self.request_redraw();
            }
            return true;
        }
        false
    }

    fn route_xyzzy(&mut self, code: KeyCode, repeat: bool) -> bool {
        let open =
            code == KeyCode::KeyX && self.modifiers.control_key() && self.modifiers.shift_key();
        if self.xyzzy.is_none() {
            if !open {
                return false;
            }
            if !repeat {
                if self.game.playing() && self.game.world.current_dock_context().is_some() {
                    self.xyzzy = Some(crate::xyzzy::Menu::default());
                    self.clear_held_input();
                } else {
                    self.notice.set(
                        "XYZZY testing requires a commander at an actual station berth".into(),
                    );
                }
                self.request_redraw();
            }
            return true;
        }
        if repeat {
            return true;
        }
        match code {
            KeyCode::Escape | KeyCode::KeyB => {
                if self.xyzzy.as_mut().is_some_and(crate::xyzzy::Menu::back) {
                    self.xyzzy = None;
                    self.clear_held_input();
                }
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                if let Some(menu) = &mut self.xyzzy {
                    menu.select(code == KeyCode::ArrowDown);
                }
            }
            KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                if let Some(menu) = &mut self.xyzzy {
                    menu.select_shield(code == KeyCode::ArrowRight);
                }
            }
            KeyCode::Enter => self.confirm_xyzzy(),
            _ => {}
        }
        self.request_redraw();
        true
    }

    fn confirm_xyzzy(&mut self) {
        let Some(menu) = &mut self.xyzzy else {
            return;
        };
        if !menu.reviewing {
            menu.reviewing = true;
            return;
        }
        let action = menu.action();
        match self.game.xyzzy(action) {
            Ok(message) => {
                self.notice.set(message);
                menu.reviewing = false;
                self.sync_profile_controls();
            }
            Err(message) => self.notice.set(message),
        }
    }

    fn reset_input(&mut self) {
        if (!self.focused || self.occluded)
            && self.cancel_occupied_cut("Occupied contact cancelled on focus loss")
        {
            return;
        }
        self.clear_held_input();
        if player_destroyed(&self.game.world) {
            // Terminal state is evidence. Focus, occlusion, menus and pause keys
            // may clear host controls but cannot rewrite World epochs or guidance.
            self.game.session.invalidate();
            self.request_redraw();
            return;
        }
        if self.modal_open() {
            if !self.focused || self.occluded {
                self.game.world.cancel_auto_docking();
                self.game.world.cancel_targeting();
            }
            self.request_redraw();
            return;
        }
        if let Some(menu) = &mut self.docked {
            menu.focus_lost();
        }
        self.game.session.invalidate();
        if self.game.playing()
            && !self.commander_open
            && !self.game.is_internal_validation()
            && let Err(error) = self.game.world.request_pause(
                self.paused || !self.focused || self.occluded,
                self.game.world.snapshot().tick,
            )
        {
            self.notice.set(error.to_string());
        }
        self.request_redraw();
    }

    fn focus_changed(&mut self, focused: bool) {
        self.focused = focused;
        self.reset_input();
    }

    fn occlusion_changed(&mut self, occluded: bool) {
        self.occluded = occluded;
        self.reset_input();
    }

    fn key(&mut self, code: KeyCode, event_loop: &ActiveEventLoop) {
        if self.game.is_internal_validation() {
            match self.validation_control(code) {
                Ok(mule_validation::Control::Quit) => self.request_quit(event_loop),
                Ok(mule_validation::Control::Updated) => self.request_redraw(),
                Ok(mule_validation::Control::Ignored) => {}
                Err(error) => self.fail(event_loop, error),
            }
            return;
        }
        if let Some(recovery) = self.quit_recovery_key(code) {
            match recovery {
                QuitRecovery::Resume => self.request_redraw(),
                QuitRecovery::Discard => {
                    self.shutdown_audio();
                    event_loop.exit();
                }
            }
            return;
        }
        if self.game.is_medusa_preview() {
            self.medusa_preview_key(code, event_loop);
            self.request_redraw();
            return;
        }
        if code == KeyCode::Digit0 && !self.commander_open && self.has_berth() {
            self.show_station_commander();
            self.request_redraw();
            return;
        }
        if self.commander_open && self.commander_key(code) {
            self.request_redraw();
            return;
        }
        let global = matches!(
            code,
            KeyCode::F5 | KeyCode::F9 | KeyCode::F11 | KeyCode::KeyP
        );
        if self.menu_active() && !global {
            if self.open_menu_key(code) {
                return;
            }
            if let Some(key) = self.docked.as_ref().and_then(|menu| {
                crate::docked::key(code, self.modifiers.shift_key(), menu.screen())
            }) {
                self.menu_input(ui::Input::Key(key));
            }
            return;
        }
        if !self.menu_active() && self.open_menu_key(code) {
            return;
        }
        match code {
            KeyCode::Escape => self.request_quit(event_loop),
            KeyCode::KeyN if !self.game.playing() || player_destroyed(&self.game.world) => {
                self.new_commander_from_key(fresh_profile_seed());
            }
            KeyCode::KeyC if !self.game.playing() => self.load_profile(),
            KeyCode::F9 if self.game.playing() => self.load_profile(),
            KeyCode::F5 if self.game.playing() => self.save_profile(),
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

    fn has_berth(&self) -> bool {
        self.game.playing() && self.game.world.current_dock_context().is_some()
    }

    fn medusa_preview_key(&mut self, code: KeyCode, event_loop: &ActiveEventLoop) {
        use KeyCode::*;
        match code {
            Escape => self.request_quit(event_loop),
            F5 => self.save_profile(),
            F9 => match self.game.continue_saved() {
                Ok(()) => {
                    self.reset_input();
                    self.notice.set("Medusa preview continued".into());
                }
                Err(error) => self
                    .notice
                    .set(format!("Preview Continue refused: {error}")),
            },
            KeyP => {
                self.paused = !self.paused;
                self.reset_input();
            }
            KeyL | Digit0 | Digit1 | Digit2 | Digit3 => {
                let action = match code {
                    KeyL => Action::Launch,
                    Digit0 => Action::SetFlightView(FlightView::Front),
                    Digit1 => Action::SetFlightView(FlightView::Rear),
                    Digit2 => Action::SetFlightView(FlightView::Left),
                    _ => Action::SetFlightView(FlightView::Right),
                };
                self.notice
                    .set(self.game.command(action).unwrap_or_else(|error| error));
            }
            KeyB | BracketLeft | BracketRight => {
                let Some(mut fit) = self.game.world.medusa_preview_fit() else {
                    return;
                };
                match code {
                    KeyB => fit.cargo_expansion = !fit.cargo_expansion,
                    BracketLeft => fit.cabins = fit.cabins.saturating_sub(1),
                    _ => fit.cabins = fit.cabins.saturating_add(1),
                }
                self.notice.set(
                    self.game
                        .medusa_preview_fit(fit)
                        .unwrap_or_else(|error| error),
                );
            }
            _ => {}
        }
    }

    fn show_station_commander(&mut self) {
        if self.menu_active()
            && let Some(menu) = &mut self.docked
            && menu.review_pending()
        {
            menu.feedback("Confirm or cancel the current review first".into());
            return;
        }
        self.commander_open = true;
        self.notice = Notice::default();
        self.clear_held_input();
    }

    fn leave_commander(&mut self) {
        self.commander_open = false;
        self.quit_failed = false;
        self.reset_input();
    }

    // Returns false for shared profile and window shortcuts handled by key().
    fn commander_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Escape | KeyCode::F11 => return false,
            KeyCode::KeyN | KeyCode::KeyC if !self.game.playing() => return false,
            KeyCode::KeyN if player_destroyed(&self.game.world) => return false,
            KeyCode::KeyP if player_destroyed(&self.game.world) => return false,
            KeyCode::KeyP | KeyCode::KeyQ if self.quit_failed => return false,
            KeyCode::F5 | KeyCode::F9 if self.game.playing() => return false,
            KeyCode::Digit1
            | KeyCode::Digit2
            | KeyCode::Digit3
            | KeyCode::Digit4
            | KeyCode::Digit5 => {
                if self.open_menu_key(code)
                    && self
                        .docked
                        .as_ref()
                        .is_some_and(|menu| menu.active(&self.game.world))
                {
                    self.leave_commander();
                }
            }
            KeyCode::KeyL if self.has_berth() => self.launch_from_commander(),
            KeyCode::Enter if self.game.playing() && !self.has_berth() => {
                self.paused = false;
                self.leave_commander();
            }
            _ => {}
        }
        true
    }

    fn launch_from_commander(&mut self) {
        let tick = self.game.world.snapshot().tick;
        match self.game.world.request_launch_from_commander(tick) {
            Ok(()) => {
                self.paused = false;
                self.leave_commander();
                self.notice
                    .set("Launch admitted. Follow the exit corridor.".into());
                self.publish_audio(false);
            }
            Err(error) => self.notice.set(error.to_string()),
        }
    }

    fn save_profile(&mut self) {
        self.notice.set(match self.game.save() {
            Ok(message) => message,
            Err(error) => error,
        });
    }

    fn new_commander_from_key(&mut self, seed: u64) {
        let message = match self.game.new_commander(seed) {
            Ok(true) => {
                self.sync_profile_controls();
                self.begin_audio_session();
                "New commander ready".into()
            }
            Ok(false) => {
                "Press N again to confirm new game; existing save is not changed yet".into()
            }
            Err(error) => error,
        };
        self.notice.set(message);
    }

    fn sync_profile_controls(&mut self) {
        self.paused = self.game.world.mission_snapshot().is_some_and(|m| m.paused);
        self.quit_failed = false;
        self.terminal_redraw_pending = false;
        self.reset_input();
        if self.commander_open {
            self.docked = None;
            return;
        }
        let screen = if self.xyzzy.is_some() {
            self.docked
                .as_ref()
                .map_or(ui::Screen::System, Docked::screen)
        } else {
            ui::Screen::System
        };
        self.docked = match self.new_docked_menu() {
            Ok(mut menu) => {
                if self.game.world.current_dock_context().is_some()
                    && let Err(error) = menu.open(&self.game.world, screen)
                {
                    self.notice.set(error);
                }
                Some(menu)
            }
            Err(error) => {
                self.notice.set(error);
                None
            }
        };
    }

    fn load_profile(&mut self) {
        match self.game.continue_saved() {
            Ok(()) => {
                self.sync_profile_controls();
                self.begin_audio_session();
                self.notice.set(if player_destroyed(&self.game.world) {
                    DESTROYED_NOTICE.into()
                } else if self.commander_open {
                    "Commander loaded".into()
                } else {
                    "Commander continued; select the actual current crew with T".into()
                });
            }
            Err(error) => self.notice.set(format!(
                "Continue refused; save and current game retained: {error}"
            )),
        }
        self.publish_audio(false);
    }

    fn request_quit(&mut self, event_loop: &ActiveEventLoop) {
        if self.modal_open() {
            return;
        }
        match self.game.prepare_quit() {
            Ok(()) => {
                if let Some(timing) = self.validation_timing.take() {
                    eprintln!("{}", timing.report());
                }
                self.shutdown_audio();
                event_loop.exit();
            }
            Err(error) => {
                self.record_quit_failure(&error);
                self.request_redraw();
            }
        }
    }

    fn record_quit_failure(&mut self, error: &str) {
        self.paused = true;
        self.reset_input();
        self.quit_failed = true;
        self.notice.set(if player_destroyed(&self.game.world) {
            DESTROYED_QUIT_NOTICE.into()
        } else {
            format!("P resumes; Q discards unsaved progress and quits. {error}")
        });
    }

    fn quit_recovery_key(&mut self, code: KeyCode) -> Option<QuitRecovery> {
        if !self.quit_failed {
            return None;
        }
        match code {
            KeyCode::KeyP => {
                self.quit_failed = false;
                self.paused = false;
                self.reset_input();
                if player_destroyed(&self.game.world) {
                    self.notice.set(DESTROYED_NOTICE.into());
                }
                Some(QuitRecovery::Resume)
            }
            KeyCode::KeyQ => Some(QuitRecovery::Discard),
            _ => None,
        }
    }

    fn menu_active(&self) -> bool {
        !self.commander_open
            && self
                .docked
                .as_ref()
                .is_some_and(|menu| menu.active(&self.game.world))
    }
    fn new_docked_menu(&self) -> Result<Docked, String> {
        if self.authored_hulls.is_empty() {
            Docked::new(&self.game.world)
        } else {
            Docked::with_authored(&self.game.world, self.authored_hulls.clone())
        }
    }
    fn menu_input(&mut self, input: ui::Input) {
        if self.modal_open() {
            return;
        }
        if let Some(menu) = &mut self.docked
            && let Err(error) = menu.input(&mut self.game, input)
        {
            menu.feedback(error);
        }
        self.request_redraw();
    }
    fn open_menu_key(&mut self, code: KeyCode) -> bool {
        if !self.has_berth() {
            return false;
        }
        let screen = match code {
            KeyCode::Digit1 => ui::Screen::Galaxy,
            KeyCode::Digit2 => ui::Screen::System,
            KeyCode::Digit3 => ui::Screen::Showroom,
            KeyCode::Digit4 | KeyCode::KeyM if !self.menu_active() => ui::Screen::Trading,
            KeyCode::Digit5 | KeyCode::KeyO if !self.menu_active() => ui::Screen::Equipment,
            KeyCode::Digit4 => ui::Screen::Trading,
            KeyCode::Digit5 => ui::Screen::Equipment,
            _ => return false,
        };
        if self.docked.is_none() {
            match self.new_docked_menu() {
                Ok(menu) => self.docked = Some(menu),
                Err(error) => {
                    self.notice.set(error);
                    return true;
                }
            }
        }
        self.keys.clear();
        self.pulse = None;
        self.mining = None;
        self.wheel = 0.0;
        if let Some(menu) = &mut self.docked
            && let Err(error) = menu.open(&self.game.world, screen)
        {
            menu.feedback(error);
        }
        self.request_redraw();
        true
    }
    fn pointer(&mut self, point: [f64; 2]) {
        let previous = self.cursor;
        self.cursor = point;
        if self.modal_open() {
            return;
        }
        if self.menu_active() {
            let input = if self.dragging {
                ui::Input::Drag {
                    from: previous,
                    to: point,
                    roll: self.modifiers.shift_key(),
                }
            } else {
                ui::Input::PointerMoved(point)
            };
            self.menu_input(input);
        }
    }
    fn scroll(&mut self, amount: f64) {
        if self.modal_open() || !self.menu_active() || !amount.is_finite() {
            return;
        }
        self.wheel -= amount;
        let rows = self.wheel.trunc() as i32;
        self.wheel -= f64::from(rows);
        if rows != 0 {
            self.menu_input(ui::Input::Scroll {
                point: self.cursor,
                rows,
            });
        }
    }

    fn game_key(&mut self, code: KeyCode) {
        if player_destroyed(&self.game.world) {
            self.notice.set(DESTROYED_NOTICE.into());
            return;
        }
        if code == KeyCode::KeyU && self.game.world.snapshot().mode == FlightMode::Flight {
            self.notice.set("EMP: hold U for ten seconds; release to cancel. Recent-attacker self-defence only.".into());
            return;
        }
        let in_flight = self.game.world.snapshot().mode == FlightMode::Flight;
        if in_flight {
            match code {
                KeyCode::Tab => return self.cycle_computer_target(),
                KeyCode::KeyF => return self.toggle_computer_assistance(),
                KeyCode::KeyT => return self.lock_computer_target(),
                KeyCode::Backspace | KeyCode::Delete => return self.release_computer_target(),
                KeyCode::KeyI if self.modifiers.shift_key() => return self.toggle_target_details(),
                _ => {}
            }
        }
        let action = match code {
            KeyCode::KeyN if in_flight => Action::ToggleAutoDocking,
            KeyCode::KeyK if in_flight => Action::ActivateEcm,
            KeyCode::Digit0 => Action::SetFlightView(FlightView::Front),
            KeyCode::Digit1 => Action::SetFlightView(FlightView::Rear),
            KeyCode::Digit2 => Action::SetFlightView(FlightView::Left),
            KeyCode::Digit3 => Action::SetFlightView(FlightView::Right),
            KeyCode::Digit4 => Action::ReviewTradeLesson,
            KeyCode::Digit5 => Action::PlanTradeLesson,
            KeyCode::Digit6 => Action::CancelTradeLesson,
            KeyCode::Digit7 => Action::TradeLessonDetails,
            KeyCode::F6 => Action::ReviewFlightLesson,
            KeyCode::F12 => Action::CancelFlightLesson,
            KeyCode::KeyC => Action::FlightLessonDetails,
            KeyCode::F1 => Action::CycleMissileLock,
            KeyCode::F2 => Action::ToggleMissileArm,
            KeyCode::F3 => Action::FireMissile,
            KeyCode::F4 => Action::ClearMissile,
            KeyCode::KeyL => Action::Launch,
            KeyCode::KeyJ => {
                if let Some(route) = self.game.world.active_route_progress() {
                    if let Some(next) = route.next_system {
                        Action::JumpTo(next)
                    } else {
                        use meridian_core::navigation::RouteStatus;
                        let message = match route.status {
                            RouteStatus::RefuelRequired => {
                                "Refuel at this stop before the next jump."
                            }
                            RouteStatus::AwaitingDock => "Dock at this stop to advance the route.",
                            RouteStatus::ComputerUnavailable => {
                                "Fit a navigation computer that supports this route."
                            }
                            RouteStatus::Complete => {
                                "Route complete. Select a new destination at a station."
                            }
                            RouteStatus::Ready => "Route has no next destination.",
                        };
                        self.notice.set(message.into());
                        return;
                    }
                } else {
                    self.docked
                        .as_ref()
                        .and_then(|menu| menu.route)
                        .map_or(Action::Jump, Action::JumpTo)
                }
            }
            KeyCode::KeyT => Action::Contact,
            KeyCode::KeyR => Action::Review,
            KeyCode::KeyV => Action::ReviewAll,
            KeyCode::KeyX => Action::ReviewAbandonment,
            KeyCode::KeyI => Action::MissionDetails,
            KeyCode::F7 => Action::ReviewPracticeClosure,
            KeyCode::F8 => Action::ReviewPracticeReception,
            KeyCode::F10 => Action::ReviewPractice,
            KeyCode::Enter => Action::Confirm,
            KeyCode::KeyB => Action::Back,
            KeyCode::KeyF => Action::Refuel,
            KeyCode::KeyG if self.modifiers.shift_key() => Action::SellSelected,
            KeyCode::KeyG => Action::BuySelected,
            KeyCode::KeyM => Action::ToggleMarket,
            KeyCode::KeyO => Action::ToggleEquipment,
            KeyCode::KeyK => Action::PurchaseCargoBay,
            KeyCode::KeyY => Action::FitCargoBay,
            KeyCode::KeyU => Action::UnfitCargoBay,
            KeyCode::BracketLeft => Action::PreviousCommodity,
            KeyCode::BracketRight => Action::NextCommodity,
            _ => return,
        };
        let mut missile = false;
        let prior_ecm = matches!(action, Action::ActivateEcm)
            .then(|| {
                self.game
                    .world
                    .ecm_snapshot()
                    .and_then(|s| s.last_activation)
            })
            .flatten();
        self.notice.set(match self.game.command(action) {
            Ok(message) => {
                missile = matches!(action, Action::FireMissile);
                message
            }
            Err(error) => error,
        });
        // A directory-sync warning follows actual publication. Clear held EMP
        // only when the World accepted a new field, not merely on UI success.
        if matches!(action, Action::ActivateEcm)
            && self
                .game
                .world
                .ecm_snapshot()
                .and_then(|s| s.last_activation)
                .is_some_and(|a| Some(a) != prior_ecm)
        {
            self.keys.remove(&KeyCode::KeyU);
        }
        self.notice.missile = missile;
        self.publish_audio(false);
    }

    fn advance_frame(&mut self, active: bool, ticks: usize) -> FrameAdvance {
        if self.close_stale_occupied_cut() || (active && self.maybe_open_occupied_cut()) {
            return FrameAdvance::Continue(None);
        }
        let result = if active && !self.modal_open() && self.game.is_internal_validation() {
            self.pulse = None;
            self.mining = None;
            self.advance_validation(ticks)
        } else if active && !self.modal_open() {
            let input = FlightInput {
                pitch: self.axis(KeyCode::ArrowUp, KeyCode::ArrowDown),
                yaw: self.axis(KeyCode::KeyD, KeyCode::KeyA),
                roll: self.axis(KeyCode::KeyE, KeyCode::KeyQ),
                throttle: self.axis(KeyCode::KeyW, KeyCode::KeyS),
                brake: self.keys.contains(&KeyCode::Space),
            };
            self.step_emp_frame(input, ticks)
        } else {
            Ok(None)
        };
        self.frame_feedback(result)
    }

    fn frame_feedback(&mut self, result: Result<Option<MotionRefusal>, StepError>) -> FrameAdvance {
        let frame_refusal = match result {
            Ok(refusal) => refusal,
            Err(StepError::InvalidMode) if player_destroyed(&self.game.world) => None,
            Err(error) if matches!(error, StepError::Collision | StepError::UnsafeEntry) => {
                self.notice.set(error.to_string());
                None
            }
            Err(error) => return FrameAdvance::Fatal(error),
        };
        if player_destroyed(&self.game.world) {
            if self.notice.text.is_none() {
                self.notice.set(DESTROYED_NOTICE.into());
            }
            return FrameAdvance::Continue(frame_refusal);
        }
        if let Some(refusal) = frame_refusal {
            self.notice.set(
                match refusal {
                    MotionRefusal::CelestialContact => {
                        "Celestial contact: stopped before the physical surface"
                    }
                    MotionRefusal::AsteroidContact => {
                        "Asteroid hull contact: stopped at last clear position"
                    }
                    MotionRefusal::NpcContact => "NPC hull contact: stopped at last clear position",
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
        FrameAdvance::Continue(frame_refusal)
    }

    fn keyboard_input(&mut self, event_loop: &ActiveEventLoop, event: winit::event::KeyEvent) {
        if let Some(code) = self.route_keyboard(
            event.physical_key,
            event.state,
            event.repeat,
            event.text.as_deref(),
        ) {
            self.key(code, event_loop);
        }
        self.publish_audio(false);
    }

    // Route held input and menu text without constructing a window or an event loop.
    fn route_keyboard(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        repeat: bool,
        text: Option<&str>,
    ) -> Option<KeyCode> {
        let PhysicalKey::Code(code) = physical_key else {
            return None;
        };
        if self.game.is_internal_validation() {
            self.keys.clear();
            return (state == ElementState::Pressed
                && self.focused
                && !self.occluded
                && !repeat
                && matches!(
                    code,
                    KeyCode::Escape | KeyCode::KeyQ | KeyCode::KeyP | KeyCode::Period
                ))
            .then_some(code);
        }
        if self.game.is_medusa_preview() {
            if state == ElementState::Released {
                self.keys.remove(&code);
                return None;
            }
            if !self.focused || self.occluded || repeat {
                return None;
            }
            let held = matches!(
                code,
                KeyCode::ArrowUp
                    | KeyCode::ArrowDown
                    | KeyCode::KeyA
                    | KeyCode::KeyD
                    | KeyCode::KeyQ
                    | KeyCode::KeyE
                    | KeyCode::KeyW
                    | KeyCode::KeyS
                    | KeyCode::Space
            );
            if held {
                self.keys.insert(code);
            }
            return Some(code);
        }
        if state == ElementState::Released {
            self.keys.remove(&code);
            if code == KeyCode::KeyU && self.occupied_cut.is_none() {
                self.cancel_emp_input();
            }
            return None;
        }
        if !self.focused || self.occluded {
            return None;
        }
        if self.route_occupied_cut(code, repeat) {
            return None;
        }
        if self.route_audio_toggle(code, repeat) {
            return None;
        }
        if self.route_help(code, repeat, text) {
            return None;
        }
        if self.route_xyzzy(code, repeat) {
            return None;
        }
        if self.commander_open {
            return (!repeat).then_some(code);
        }
        if self.menu_active()
            && self.docked.as_ref().is_some_and(Docked::search_active)
            && let Some(text) = text.filter(|s| s.chars().all(|c| !c.is_control()))
        {
            self.menu_input(ui::Input::Text(text.to_string()));
            return None;
        }
        if code == KeyCode::KeyP
            && self.menu_active()
            && self
                .docked
                .as_ref()
                .is_some_and(|menu| menu.screen() == ui::Screen::Galaxy)
        {
            if !repeat {
                self.menu_input(ui::Input::Key(ui::Key::ProfitScan));
            }
            return None;
        }
        if repeat
            && !(self.menu_active()
                && matches!(
                    code,
                    KeyCode::ArrowUp
                        | KeyCode::ArrowDown
                        | KeyCode::PageUp
                        | KeyCode::PageDown
                        | KeyCode::Backspace
                ))
        {
            return None;
        }
        if code == KeyCode::KeyZ && self.simulation_active() && self.use_manual_tool() {
            return None;
        }
        if !self.menu_active() {
            self.keys.insert(code);
        }
        Some(code)
    }

    fn build_docked_frame(
        active: bool,
        viewport: impl FnOnce() -> Option<[f64; 2]>,
        menu: Option<&mut Docked>,
        world: &World,
        elapsed: f64,
    ) -> Result<Option<ui::Frame>, String> {
        if !active {
            return Ok(None);
        }
        let viewport = viewport().and_then(|[width, height]| ui::Viewport::new(width, height));
        match (menu, viewport) {
            (Some(menu), Some(viewport)) => {
                menu.build_frame(world, viewport, elapsed).map(|mut frame| {
                    if world.xyzzy_snapshot().is_some_and(|state| state.active)
                        && let Some(title) = frame.labels.first_mut()
                    {
                        title.text = format!("CLARKE / XYZZY | {}", title.text);
                    }
                    Some(frame)
                })
            }
            _ => Ok(None),
        }
    }

    fn docked_frame(
        &mut self,
        elapsed: f64,
        viewport: impl FnOnce(Option<&Renderer>) -> Option<[f64; 2]>,
    ) -> Result<Option<ui::Frame>, String> {
        let renderer = self.renderer.as_ref();
        if self.commander_open {
            return Ok(viewport(renderer)
                .and_then(|[width, height]| ui::Viewport::new(width, height))
                .map(|viewport| {
                    ui::commander_frame(&self.game, viewport, self.notice.text.as_deref())
                }));
        }
        Self::build_docked_frame(
            self.menu_active(),
            || viewport(renderer),
            self.docked.as_mut(),
            &self.game.world,
            elapsed,
        )
    }

    fn prepare_presentation(
        &mut self,
        active: bool,
        pulse_notice: Option<&str>,
        elapsed: f64,
        viewport: impl FnOnce(Option<&Renderer>) -> Option<[f64; 2]>,
    ) -> Result<PreparedPresentation, String> {
        let hud_lines = self
            .game
            .lines(active, pulse_notice.or(self.notice.text.as_deref()));
        let docked_frame =
            self.docked_frame(if self.modal_open() { 0.0 } else { elapsed }, viewport)?;
        Ok(PreparedPresentation {
            hud_lines,
            docked_frame,
        })
    }

    fn route_presentation(
        prepared: Result<PreparedPresentation, String>,
        fail: impl FnOnce(String),
    ) -> Option<PreparedPresentation> {
        match prepared {
            Ok(frame) => Some(frame),
            Err(error) => {
                fail(error);
                None
            }
        }
    }

    fn complete_presentation(
        presented: Result<Presentation, String>,
        docked: &mut Option<Docked>,
        docked_frame: Option<ui::Frame>,
        zero_sized: bool,
        retry_inactive: bool,
        request_redraw: impl FnOnce(),
    ) -> Result<bool, String> {
        match presented {
            Ok(Presentation::Presented) => {
                if let (Some(menu), Some(frame)) = (docked, docked_frame)
                    && frame.screen.is_some()
                {
                    menu.presented(frame);
                }
                Ok(false)
            }
            Ok(Presentation::Skipped) => {
                if retry_inactive && !zero_sized {
                    request_redraw();
                }
                Ok(true)
            }
            Err(error) => Err(error),
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;
        let active = self.simulation_active();
        let ticks = self.clock.ticks(elapsed, active);
        let frame_refusal = match self.advance_frame(active, ticks) {
            FrameAdvance::Continue(refusal) => refusal,
            FrameAdvance::Fatal(error) => {
                self.fail(event_loop, format!("Simulation refused input: {error}"));
                return;
            }
        };
        let active = self.simulation_active();
        let snapshot = self.game.world.snapshot();
        // The beam is a short-lived view of the committed shot, never saved or re-fired.
        self.pulse = self.pulse.filter(|shot| {
            active
                && snapshot.mode == FlightMode::Flight
                && snapshot.tick.saturating_sub(shot.tick) < 5
        });
        // A contact recorded during this catch-up frame takes precedence over beam feedback.
        let pulse_notice =
            self.pulse
                .filter(|_| frame_refusal.is_none())
                .map(|shot| match shot.contact {
                    PulseContact::AsteroidBlocked { .. } => "Rock obstructs Pulse: no ore yielded",
                    PulseContact::Npc { .. } => "NPC hull struck",
                    PulseContact::Clear => "Pulse fired",
                    PulseContact::Projectile { .. } => "Pulse intercepted missile",
                    PulseContact::StationBlocked { .. } => {
                        "Station struck | legal incident recorded"
                    }
                    PulseContact::PracticeBeacon {
                        first_hit: true, ..
                    } => "Practice beacon hit",
                    PulseContact::PracticeBeacon {
                        first_hit: false, ..
                    } => "Practice beacon already counted",
                });
        let prepared = self.prepare_presentation(active, pulse_notice, elapsed, |r| {
            r.map(Renderer::logical_viewport)
        });
        let Some(prepared) =
            Self::route_presentation(prepared, |error| self.fail(event_loop, error))
        else {
            return;
        };
        self.draw_prepared_frame(event_loop, active, snapshot, prepared);
    }

    fn frame_title(&self, active: bool, mode: FlightMode) -> String {
        let modal_open = self.modal_open();
        let commander = if self.game.is_internal_validation() {
            "Meridian INTERNAL MULE VALIDATION / NO SAVE"
        } else if self.game.is_medusa_preview() {
            "Meridian SH171 MEDUSA / ISOLATED PREVIEW"
        } else if self
            .game
            .world
            .xyzzy_snapshot()
            .is_some_and(|state| state.active)
        {
            "CLARKE / XYZZY"
        } else {
            "Meridian"
        };
        let mode = if player_destroyed(&self.game.world) {
            "DESTROYED"
        } else if self.commander_open && !self.game.playing() {
            "SAVE RECOVERY"
        } else {
            match mode {
                FlightMode::Docked => "DOCKED",
                FlightMode::Launching => "LAUNCHING",
                FlightMode::Flight => "FLIGHT",
            }
        };
        let inspection = self
            .authored_hulls
            .inspection_record()
            .map(|ship| format!(" | {} [{}]", ship.name, ship.id))
            .unwrap_or_default();
        format!(
            "{commander}{inspection} | {mode} | {} | H / ? Help",
            if player_destroyed(&self.game.world) {
                "STOPPED"
            } else if modal_open {
                "MENU"
            } else if self.commander_open {
                "COMMANDER"
            } else if active {
                "RUNNING"
            } else {
                "PAUSED"
            },
        )
    }

    fn draw_prepared_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        active: bool,
        snapshot: meridian_core::Snapshot,
        prepared: PreparedPresentation,
    ) {
        let PreparedPresentation {
            mut hud_lines,
            docked_frame,
        } = prepared;
        if snapshot.mode == FlightMode::Flight && self.game.world.manual_scanner_target() {
            hud_lines.insert(hud_lines.len().min(2), "MANUAL TARGET | F ASSIST".into());
        }
        let belt = if snapshot.mode == FlightMode::Flight {
            self.game.world.mining_belt_snapshot()
        } else {
            None
        };
        if let Some(belt) = &belt {
            match asteroid_render::hud_line(snapshot, belt, None) {
                Ok(Some(line)) => hud_lines.insert(hud_lines.len().min(2), line),
                Ok(None) => {}
                Err(error) => {
                    self.fail(event_loop, error);
                    return;
                }
            }
        }

        let traffic = self.game.world.traffic_snapshot();
        let cockpit = if self.game.playing()
            && !self.commander_open
            && !self.game.is_internal_validation()
            && !self.game.is_medusa_preview()
        {
            match crate::hud::cockpit::Cockpit::capture(&self.game.world, snapshot) {
                Ok(cockpit) => Some(cockpit),
                Err(error) => {
                    self.fail(event_loop, error);
                    return;
                }
            }
        } else {
            None
        };
        self.missile_bodies.clear();
        self.missile_bodies.extend(
            self.game
                .world
                .missile_projectiles()
                .filter(|p| p.ended.is_none()),
        );
        let celestial_scene = match self
            .celestial
            .scene(self.game.world.celestial_snapshot(), snapshot.mode)
        {
            Ok(scene) => scene,
            Err(error) => {
                self.fail(event_loop, error.to_owned());
                return;
            }
        };

        let docked_planet =
            if let Some(request) = docked_frame.as_ref().and_then(|f| f.planets.first()) {
                match self
                    .celestial
                    .preview(self.game.world.celestial_snapshot(), request.system)
                {
                    Ok(scene) => scene,
                    Err(error) => {
                        self.fail(event_loop, error);
                        return;
                    }
                }
            } else {
                None
            };
        let xyzzy_panel = self
            .xyzzy
            .as_ref()
            .map(|menu| menu.panel(&self.game.world, self.notice.text.as_deref()));
        let occupied_panel = self.occupied_cut_panel();
        let title = self.frame_title(active, snapshot.mode);
        let terminal_visible = self.terminal_cockpit_visible();
        let retry_inactive = (terminal_visible && !self.occluded)
            || self.help.is_some()
            || self.xyzzy.is_some()
            || occupied_panel.is_some()
            || docked_frame
                .as_ref()
                .is_some_and(|frame| frame.screen.is_none());
        if let Some(renderer) = &mut self.renderer {
            renderer.window.set_title(&title);
            let station =
                snapshot
                    .station
                    .zip(self.game.world.station_port())
                    .map(|(pose, port)| StationScene {
                        pose,
                        port,
                        docked: snapshot.mode == FlightMode::Docked,
                    });
            let mule_frame = self.game.world.mule_actor_snapshot();
            let mule_scene = mule_presentation::current_placement(
                &self.game.world,
                snapshot,
                mule_frame.as_ref(),
            );
            let practice = self.game.world.practice_snapshot();
            let targeting = Self::target_readout(&self.game.world);
            renderer.set_player_hull(self.game.world.player_hull_kind());
            renderer.set_flight_view(if self.game.is_internal_validation() {
                self.validation_view
            } else {
                self.game.world.flight_view()
            });
            let draw_started = self.validation_timing.is_some().then(Instant::now);
            let presented = renderer.draw(
                &snapshot,
                station,
                SceneEffects {
                    mule: mule_scene,
                    traffic: traffic.as_ref(),
                    targeting: targeting.as_ref(),
                    target_details: self.target_details,
                    pulse: self.pulse.as_ref(),
                    mining: belt.as_ref().map(|belt| asteroid_render::Scene {
                        belt,
                        visual: self.mining,
                    }),
                    missiles: &self.missile_bodies,
                    practice: practice.as_ref().and_then(|state| state.beacons.as_ref()),
                    celestial: celestial_scene.as_ref(),
                    docked: docked_frame.as_ref(),
                    docked_planet: docked_planet.as_ref(),
                },
                HudScene {
                    lines: &hud_lines,
                    cockpit: cockpit.as_ref(),
                    help: occupied_panel
                        .as_ref()
                        .or(self.help.as_ref())
                        .or(xyzzy_panel.as_ref()),
                },
            );
            if let (Some(started), Some(timing)) = (draw_started, &mut self.validation_timing) {
                timing.observe(&presented, started.elapsed());
            }
            let studio_ready = self.help.is_none()
                && self.xyzzy.is_none()
                && self.occupied_cut.is_none()
                && take_studio_ready(
                    &mut self.studio_ready,
                    &self.authored_hulls,
                    docked_frame.as_ref(),
                    &presented,
                );
            match Self::complete_presentation(
                presented,
                &mut self.docked,
                docked_frame,
                renderer.is_zero_sized(),
                retry_inactive,
                || renderer.window.request_redraw(),
            ) {
                Ok(skipped) => {
                    self.record_terminal_presentation(skipped);
                    if studio_ready && !skipped {
                        let mut stdout = std::io::stdout().lock();
                        if let Err(error) = writeln!(stdout, "MERIDIAN_HULL_STUDIO_READY")
                            .and_then(|()| stdout.flush())
                        {
                            self.fail(
                                event_loop,
                                format!("Studio readiness output failed: {error}"),
                            );
                        }
                    }
                }
                Err(error) => self.fail(event_loop, error),
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.resume_audio();
        if self.renderer.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Meridian - initializing graphics")
            .with_inner_size(winit::dpi::LogicalSize::new(
                DEFAULT_FLIGHT_WINDOW[0],
                DEFAULT_FLIGHT_WINDOW[1],
            ))
            .with_min_inner_size(winit::dpi::LogicalSize::new(
                MIN_WINDOW_LOGICAL[0],
                MIN_WINDOW_LOGICAL[1],
            ));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => return self.fail(event_loop, error),
        };
        match pollster::block_on(Renderer::new(
            window,
            event_loop.owned_display_handle(),
            self.authored_hulls.clone(),
        )) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(error) => self.fail(event_loop, error),
        }
        self.last_frame = Instant::now();
        if self.commander_open || self.menu_active() {
            self.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if !self.renderer.as_ref().is_some_and(|r| r.window.id() == id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested if !self.modal_open() => self.request_quit(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(menu) = &mut self.docked {
                    menu.discard_frame();
                }
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::Focused(focused) => {
                self.focus_changed(focused);
            }
            WindowEvent::Occluded(occluded) => {
                self.occlusion_changed(occluded);
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::CursorMoved { position, .. } if self.focused && !self.occluded => {
                let scale = self
                    .renderer
                    .as_ref()
                    .map_or(1.0, |r| r.window.scale_factor());
                self.pointer([position.x / scale, position.y / scale]);
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } if self.focused && !self.occluded => {
                if self.modal_open() {
                    return;
                }
                self.dragging = state == ElementState::Pressed;
                if self.dragging && self.menu_active() {
                    self.menu_input(ui::Input::Click(self.cursor));
                }
            }
            WindowEvent::MouseWheel { delta, .. } if self.focused && !self.occluded => {
                let scale = self
                    .renderer
                    .as_ref()
                    .map_or(1.0, |r| r.window.scale_factor());
                self.scroll(match delta {
                    MouseScrollDelta::LineDelta(_, rows) => f64::from(rows),
                    MouseScrollDelta::PixelDelta(point) => point.y / scale / 20.0,
                });
            }
            WindowEvent::KeyboardInput { event, .. } => self.keyboard_input(event_loop, event),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.publish_audio(false);
        if self.expire_hud_notice(Instant::now()) {
            self.request_redraw();
        }
        if let Some(renderer) = &self.renderer {
            if self.terminal_retry_needed(renderer.is_zero_sized()) {
                renderer.window.request_redraw();
            }
            if !self.simulation_active() || renderer.is_zero_sized() {
                let until = self
                    .notice
                    .expires
                    .filter(|_| self.timed_hud_notices() && !renderer.is_zero_sized());
                event_loop
                    .set_control_flow(until.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
            } else {
                event_loop.set_control_flow(ControlFlow::Poll);
                renderer.window.request_redraw();
            }
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.suspend_audio();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.shutdown_audio();
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let mut app = App::new()?;
    let event_loop = EventLoop::new()?;
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
        step_frame(&mut world, input, 1, false, &mut pulse, &mut None, || {})
            .unwrap_or_else(|error| panic!("{error}"));
        let tick = world.snapshot().tick;
        step_frame(&mut world, input, 0, true, &mut pulse, &mut None, || {})
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(pulse, None);
        step_frame(&mut world, input, 1, true, &mut pulse, &mut None, || {})
            .unwrap_or_else(|error| panic!("{error}"));
        let first = pulse.unwrap_or_else(|| panic!("Held trigger did not fire on the next tick"));
        assert_eq!(first.tick, tick + 1);

        let cooldown = usize::from(meridian_core::PULSE_COOLDOWN_TICKS);
        step_frame(
            &mut world,
            input,
            cooldown - 1,
            true,
            &mut pulse,
            &mut None,
            || {},
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(pulse, Some(first));
        step_frame(&mut world, input, 1, true, &mut pulse, &mut None, || {})
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            pulse.map(|shot| shot.tick),
            Some(first.tick + cooldown as u64)
        );

        let second = pulse;
        step_frame(
            &mut world,
            input,
            cooldown,
            false,
            &mut pulse,
            &mut None,
            || {},
        )
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
        let notice = step_frame(
            &mut world,
            FlightInput::default(),
            2,
            false,
            &mut None,
            &mut None,
            || {},
        )
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

#[cfg(test)]
mod callback_tests;

#[cfg(test)]
mod help_tests;

#[cfg(test)]
mod xyzzy_tests;

#[cfg(test)]
mod authored_hull_tests;
#[cfg(test)]
mod emp_tests;

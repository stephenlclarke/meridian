//! Actual title, profile and save lifecycle used by the window shell and headless process tests.
use crate::{
    session::{Action, PracticeSelection, Session},
    storage::{SaveCommit, Store},
};
use meridian_core::{FlightMode, World, save::MAX_SAVE_BYTES};
use std::{io, path::PathBuf};

pub struct Profile {
    pub world: World,
    pub session: Session,
    store: Option<Store>,
    playing: bool,
    has_save: bool,
    new_armed: bool,
}

impl Profile {
    pub fn open(directory: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let store = Store::open(directory)?;
        let has_save = !matches!(store.load(MAX_SAVE_BYTES), Ok(None));
        Self::with_store(store, has_save)
    }

    fn with_store(store: Store, has_save: bool) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            world: World::at_original_station()?,
            session: Session::default(),
            store: Some(store),
            playing: false,
            has_save,
            new_armed: false,
        })
    }

    pub(crate) fn open_startup(
        directory: PathBuf,
        seed: u64,
    ) -> Result<(Self, Option<String>), Box<dyn std::error::Error>> {
        let store = Store::open(directory)?;
        let slot = store.load(MAX_SAVE_BYTES);
        let mut profile = Self::with_store(store, !matches!(&slot, Ok(None)))?;
        let result = match slot {
            Ok(None) => profile.new_commander(seed).map(|_| ()),
            Ok(Some(bytes)) => profile.accept_save(&bytes),
            Err(error) => Err(error.to_string()),
        };
        let notice = result
            .err()
            .map(|error| format!("Commander startup refused: {error}"));
        Ok((profile, notice))
    }

    /// A fresh World-owned source scenario has no Store, directory resolution or profile lease.
    pub(crate) fn mule_validation(world: World) -> Result<Self, String> {
        let frame = world
            .mule_actor_snapshot()
            .ok_or("Missing actual Mule World scenario")?;
        if world.mule_observer_pose(&frame).is_none()
            || world.commander_id().is_some()
            || world.player_ship_id().is_some()
            || world.traffic_snapshot().is_some()
            || world.save_bytes().is_ok()
        {
            return Err("Internal Mule scenario must have no commander, ordinary population or save authority".into());
        }
        Ok(Self {
            world,
            session: Session::default(),
            store: None,
            playing: true,
            has_save: false,
            new_armed: false,
        })
    }
    pub(crate) fn is_internal_validation(&self) -> bool {
        self.store.is_none()
    }
    fn require_profile(&self) -> Result<(), String> {
        if self.is_internal_validation() {
            return Err(
                "Internal Mule validation has no commander/profile/save or gameplay commands"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn playing(&self) -> bool {
        self.playing
    }

    /// Publish the exact prospective secret transaction before replacing the live commander.
    pub fn xyzzy(&mut self, action: meridian_core::XyzzyAction) -> Result<String, String> {
        self.require_profile()?;
        if !self.playing {
            return Err("Start or continue a commander before opening XYZZY".into());
        }
        let tick = self.world.snapshot().tick;
        let review = self
            .world
            .review_xyzzy(action, tick)
            .map_err(|e| e.to_string())?;
        let prepared = self
            .world
            .prepare_xyzzy(review, tick)
            .map_err(|e| e.to_string())?;
        let published = self
            .store
            .as_ref()
            .ok_or("Internal validation has no profile storage")?
            .save(prepared.bytes(), MAX_SAVE_BYTES)
            .map_err(|e| format!("XYZZY change could not be saved: {e}"))?;
        prepared.commit();
        self.has_save = true;
        self.session = Session::from_world(&self.world);
        // A directory-sync warning follows publication, so the host must refresh its authorities.
        Ok(publication_message(
            published,
            match action {
                meridian_core::XyzzyAction::Enter => {
                    "CLARKE active. Maximum outfit fitted and saved."
                }
                meridian_core::XyzzyAction::MaximumSupportedHardware
                | meridian_core::XyzzyAction::RestoreStarter => {
                    "Outfit refitted, refilled and saved."
                }
                meridian_core::XyzzyAction::ToggleUnlimitedFuel => "Fuel setting saved.",
            }
            .into(),
        )
        .unwrap_or_else(|warning| warning))
    }

    fn emp_equipment(
        &mut self,
        command: meridian_core::EmpEquipmentAction,
    ) -> Result<String, String> {
        let tick = self.world.snapshot().tick;
        let review = self
            .world
            .review_emp_equipment(command, tick)
            .map_err(|e| e.to_string())?;
        let prepared = self
            .world
            .prepare_emp_equipment(review, tick)
            .map_err(|e| e.to_string())?;
        let published = self
            .store
            .as_ref()
            .ok_or("Internal validation has no profile storage")?
            .save(prepared.bytes(), MAX_SAVE_BYTES)
            .map_err(|e| format!("EMP change could not be saved: {e}"))?;
        prepared.commit();
        self.has_save = true;
        self.session.invalidate();
        publication_message(
            published,
            "EMP property change saved; charge is conserved, not refilled.".into(),
        )
    }

    pub fn new_commander(&mut self, seed: u64) -> Result<bool, String> {
        self.require_profile()?;
        if self.playing {
            return Err("Start a new commander from the title screen".into());
        }
        if self.has_save && !self.new_armed {
            self.new_armed = true;
            return Ok(false);
        }
        let world = World::new_commander(seed).map_err(|e| e.to_string())?;
        self.world = world;
        self.session = Session::default();
        self.playing = true;
        self.new_armed = false;
        Ok(true)
    }

    pub fn continue_saved(&mut self) -> Result<(), String> {
        self.require_profile()?;
        let bytes = self
            .store
            .as_ref()
            .ok_or("Internal validation has no profile storage")?
            .load(MAX_SAVE_BYTES)
            .map_err(|e| e.to_string())?
            .ok_or("No saved commander exists in this save slot")?;
        self.accept_save(&bytes)
    }

    fn accept_save(&mut self, bytes: &[u8]) -> Result<(), String> {
        if self.playing {
            self.world.restore_bytes(bytes).map_err(|e| e.to_string())?;
        } else {
            let world = World::from_save_bytes(bytes).map_err(|e| e.to_string())?;
            self.world = world;
        }
        self.session = Session::from_world(&self.world);
        self.playing = true;
        self.has_save = true;
        self.new_armed = false;
        Ok(())
    }

    pub fn command(&mut self, action: Action) -> Result<String, String> {
        self.require_profile()?;
        if !self.playing {
            return Err("N starts a new commander; C continues the existing save".into());
        }
        if let Action::Emp(command) = action {
            return self.emp_equipment(command);
        }
        if let Action::Module(command) = action {
            let tick = self.world.snapshot().tick;
            let review = self
                .world
                .review_module(command, tick)
                .map_err(|e| e.to_string())?;
            let prepared = self
                .world
                .prepare_module(review, tick)
                .map_err(|e| e.to_string())?;
            let message = match command {
                meridian_core::ModuleAction::PurchaseMiningLaser => {
                    "Mining Laser purchased and fitted."
                }
                meridian_core::ModuleAction::PurchaseFuelScoops => {
                    "Fuel Scoops purchased and fitted. Physical ore collection remains unavailable."
                }
                meridian_core::ModuleAction::Purchase { .. } => {
                    "Owned module purchased and fitted."
                }
                meridian_core::ModuleAction::Store { .. } => "Owned module stored.",
                meridian_core::ModuleAction::Install { .. } => "Owned module fitted.",
            };
            let published = self
                .store
                .as_ref()
                .ok_or("Internal validation has no profile storage")?
                .save(prepared.bytes(), MAX_SAVE_BYTES)
                .map_err(|e| format!("Module change could not be saved: {e}"))?;
            prepared.commit();
            self.has_save = true;
            self.session.invalidate();
            return publication_message(published, format!("{message} Change saved."));
        }
        let cargo = match action {
            Action::CargoEquipment(command) => Some(command),
            Action::PurchaseCargoBay => Some(meridian_core::equipment::Command::PurchaseCargoBay),
            Action::FitCargoBay | Action::UnfitCargoBay => {
                let item = self
                    .world
                    .equipment_snapshot()
                    .and_then(|s| s.owned)
                    .ok_or("No owned cargo bay")?
                    .id;
                Some(if matches!(action, Action::FitCargoBay) {
                    meridian_core::equipment::Command::FitCargoBay(item)
                } else {
                    meridian_core::equipment::Command::UnfitCargoBay(item)
                })
            }
            _ => None,
        };
        if let Some(command) = cargo {
            let tick = self.world.snapshot().tick;
            let prepared = self
                .world
                .prepare_equipment(command, tick)
                .map_err(|e| e.to_string())?;
            let event = prepared.event();
            let published = self
                .store
                .as_ref()
                .ok_or("Internal validation has no profile storage")?
                .save(prepared.bytes(), MAX_SAVE_BYTES)
                .map_err(|e| format!("Cargo equipment change could not be saved: {e}"))?;
            prepared.commit();
            self.has_save = true;
            self.session.invalidate();
            return publication_message(
                published,
                match event {
                    meridian_core::equipment::Event::Purchased { paid_tenths, .. } => format!(
                        "Cargo bay purchased and fitted for {}.{} CR. Change saved.",
                        paid_tenths / 10,
                        paid_tenths % 10
                    ),
                    meridian_core::equipment::Event::Fitted { .. } => {
                        "Owned cargo bay fitted. Change saved.".into()
                    }
                    meridian_core::equipment::Event::Stored { .. } => {
                        "Owned cargo bay stored. Change saved.".into()
                    }
                },
            );
        }
        if let Action::StationService(command) = action {
            let tick = self.world.snapshot().tick;
            let prepared = self
                .world
                .prepare_station_service(command, tick)
                .map_err(|e| e.to_string())?;
            let message = crate::market::service_message(prepared.event())?;
            let published = self
                .store
                .as_ref()
                .ok_or("Internal validation has no profile storage")?
                .save(prepared.bytes(), MAX_SAVE_BYTES)
                .map_err(|e| format!("Market change could not be saved: {e}"))?;
            prepared.commit();
            self.has_save = true;
            self.session.invalidate();
            return publication_message(published, message);
        }
        if let Action::Ammunition(command) = action {
            let tick = self.world.snapshot().tick;
            let review = self
                .world
                .review_ammunition(command, tick)
                .map_err(|e| e.to_string())?;
            let prepared = self
                .world
                .prepare_ammunition(review, tick)
                .map_err(|e| e.to_string())?;
            let message = match command {
                meridian_core::AmmunitionAction::Purchase => "Missile purchased and fitted.",
                meridian_core::AmmunitionAction::Store { .. } => "Owned missile stored.",
                meridian_core::AmmunitionAction::Refit { .. } => "Owned missile fitted.",
            };
            let published = self
                .store
                .as_ref()
                .ok_or("Internal validation has no profile storage")?
                .save(prepared.bytes(), MAX_SAVE_BYTES)
                .map_err(|e| format!("Missile equipment change could not be saved: {e}"))?;
            prepared.commit();
            self.has_save = true;
            self.session.invalidate();
            return publication_message(published, format!("{message} Change saved."));
        }
        if let Some(command) = self.session.station_service_command(action) {
            let tick = self.world.snapshot().tick;
            let prepared = self
                .world
                .prepare_station_service(command, tick)
                .map_err(|e| e.to_string())?;
            let message = crate::market::service_message(prepared.event())?;
            let published = self
                .store
                .as_ref()
                .ok_or("Internal validation has no profile storage")?
                .save(prepared.bytes(), MAX_SAVE_BYTES)
                .map_err(|e| format!("Market transaction could not be saved: {e}"))?;
            prepared.commit();
            self.has_save = true;
            return publication_message(published, message);
        }
        if matches!(action, Action::Confirm)
            && let Some(review) = self.session.pending_trade.take()
        {
            let tick = self.world.snapshot().tick;
            let prepared = self
                .world
                .prepare_navigation_trade(review, tick)
                .map_err(|e| e.to_string())?;
            let published = self
                .store
                .as_ref()
                .ok_or("Internal validation has no profile storage")?
                .save(prepared.bytes(), MAX_SAVE_BYTES)
                .map_err(|e| format!("Trade lesson confirmation could not be saved: {e}"))?;
            prepared.commit();
            self.has_save = true;
            return publication_message(
                published,
                "Trade lesson confirmed and saved. 7 shows its state.".into(),
            );
        }
        if matches!(action, Action::Confirm)
            && let Some(review) = self.session.pending_flight.take()
        {
            let prepared = self
                .world
                .prepare_flight_reception(review, self.world.snapshot().tick)
                .map_err(|e| e.to_string())?;
            let published = self
                .store
                .as_ref()
                .ok_or("Internal validation has no profile storage")?
                .save(prepared.bytes(), MAX_SAVE_BYTES)
                .map_err(|e| format!("Flight lesson confirmation could not be saved: {e}"))?;
            prepared.commit();
            self.has_save = true;
            return match published {
                SaveCommit::Published {
                    directory_sync_error: Some(error),
                } => Err(format!(
                    "Flight lesson confirmation was written, but directory synchronization failed: {error}"
                )),
                _ => Ok("Flight lesson confirmed and saved. C shows its current state.".into()),
            };
        }
        if matches!(action, Action::Confirm)
            && let Some(selection) = self.session.pending_practice.take()
        {
            let tick = self.world.snapshot().tick;
            let (prepared, success) = match selection {
                PracticeSelection::Consent(review) => (
                    self.world.prepare_practice_confirmation(review, tick),
                    "Practice confirmed and saved. L launches.",
                ),
                PracticeSelection::Closure(review) => (
                    self.world.prepare_practice_closure(review, tick),
                    "Practice closed and saved. F10 reviews a fresh attempt; ammunition is not refilled.",
                ),
                PracticeSelection::Reception(review) => (
                    self.world.prepare_practice_reception(review, tick),
                    "Practice reception saved. Safe Gunnery Family Trace recorded.",
                ),
            };
            let prepared = prepared.map_err(|error| error.to_string())?;
            let published = self
                .store
                .as_ref()
                .ok_or("Internal validation has no profile storage")?
                .save(prepared.bytes(), MAX_SAVE_BYTES)
                .map_err(|error| format!("Practice confirmation could not be saved: {error}"))?;
            prepared.commit();
            self.has_save = true;
            return match published {
                SaveCommit::Published {
                    directory_sync_error: Some(error),
                } => Err(format!(
                    "Practice confirmation was written, but directory synchronization failed: {error}"
                )),
                _ => Ok(success.into()),
            };
        }
        self.session.command(&mut self.world, action)
    }

    pub fn save(&mut self) -> Result<String, String> {
        self.require_profile()?;
        if !self.playing {
            return Err("No active commander to save".into());
        }
        let bytes = self.world.save_bytes().map_err(|e| e.to_string())?;
        let committed = self
            .store
            .as_ref()
            .ok_or("Internal validation has no profile storage")?
            .save(&bytes, MAX_SAVE_BYTES)
            .map_err(|e| format!("Save failed; previous file retained: {e}"))?;
        self.has_save = true;
        match committed {
            #[cfg(unix)]
            SaveCommit::DirectorySynced => Ok("Commander saved".into()),
            SaveCommit::Published {
                directory_sync_error: None,
            } => Ok("Commander saved".into()),
            SaveCommit::Published {
                directory_sync_error: Some(error),
            } => Err(format!(
                "New save is written, but directory synchronization failed: {error}"
            )),
        }
    }

    pub fn prepare_quit(&mut self) -> Result<(), String> {
        if self.is_internal_validation() {
            return Ok(());
        }
        if self.playing {
            self.save()?;
        }
        Ok(())
    }

    pub fn lines(&self, active: bool, notice: Option<&str>) -> Vec<String> {
        if self.is_internal_validation() {
            let mut lines = vec![
                "INTERNAL MULE VALIDATION | NO COMMANDER / NO SAVE".into(),
                "P RUN / PAUSE | . ONE WORLD TICK WHEN PAUSED | ESC / Q QUIT".into(),
            ];
            if let Some(frame) = self.world.mule_actor_snapshot() {
                lines.push(format!(
                    "SH001 | WORLD TICK {} | GENERATION {} | {}",
                    frame.world_tick(),
                    frame.generation(),
                    if active { "RUNNING" } else { "PAUSED" }
                ));
                lines.push(format!(
                    "MULE PHASE {:?} | SPEED {:.2}",
                    frame.actor().phase,
                    frame.actor().speed_units
                ));
                if let Some(refusal) = frame.motion_refusal() {
                    lines.push(format!("WORLD MOTION REFUSED: {refusal:?}"));
                }
            }
            if let Some(notice) = notice {
                lines.push(notice.into());
            }
            return lines;
        }
        if self.playing {
            let mut lines = self.session.lines(&self.world, active, notice);
            if self.world.snapshot().mode == FlightMode::Docked && !self.session.hud_panel_open() {
                return lines;
            }
            if let Some(secret) = self.world.xyzzy_snapshot().filter(|state| state.active) {
                lines.insert(
                    0,
                    format!(
                        "CLARKE | XYZZY | FUEL {}",
                        if secret.unlimited_fuel {
                            "UNLIMITED"
                        } else {
                            "NORMAL"
                        }
                    ),
                );
            }
            if let Some(emp) = crate::docked_screens::emp::status(&self.world) {
                lines.push(emp);
            }
            return lines;
        }
        vec![
            "MERIDIAN".into(),
            "C CONTINUE | N NEW COMMANDER | H / ? HELP".into(),
            if self.has_save {
                "AN EXISTING SAVE IS RETAINED UNTIL AN EXPLICIT NEW GAME IS SAVED".into()
            } else {
                "NO SAVED COMMANDER IN THIS SLOT - PRESS N TO BEGIN".into()
            },
            if self.new_armed {
                "PRESS N AGAIN TO CONFIRM A NEW COMMANDER; C KEEPS AND CONTINUES THE SAVE".into()
            } else {
                "CLICK INSIDE THE WINDOW TO FOCUS THE CONTROLS".into()
            },
            notice.unwrap_or("H / ? OPENS CONTEXT HELP").into(),
        ]
    }
}

pub fn directory_from_args(args: impl Iterator<Item = std::ffi::OsString>) -> io::Result<PathBuf> {
    let mut args = args;
    if let Some(flag) = args.next() {
        if flag != "--save-dir" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Expected --save-dir PATH",
            ));
        }
        let path = args.next().filter(|p| !p.is_empty()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "--save-dir needs a path")
        })?;
        if args.next().is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unexpected additional argument",
            ));
        }
        return Ok(path.into());
    }
    default_directory()
}

#[cfg(target_os = "windows")]
fn default_directory() -> io::Result<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|p| {
            PathBuf::from(p)
                .join("Meridian Traffic Development")
                .join("saves")
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "LOCALAPPDATA is unavailable; use --save-dir PATH",
            )
        })
}
#[cfg(target_os = "macos")]
fn default_directory() -> io::Result<PathBuf> {
    std::env::var_os("HOME")
        .map(|p| {
            PathBuf::from(p).join("Library/Application Support/Meridian Traffic Development/saves")
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Home directory is unavailable; use --save-dir PATH",
            )
        })
}
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn default_directory() -> io::Result<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME").filter(|p| !p.is_empty()) {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            return Ok(path.join("meridian-traffic-development/saves"));
        }
    }
    std::env::var_os("HOME")
        .map(|p| PathBuf::from(p).join(".local/share/meridian-traffic-development/saves"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Home directory is unavailable; use --save-dir PATH",
            )
        })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use meridian_core::{
        FlightInput, FlightMode,
        navigation::{BASIC_SPOOL_TICKS, DockOrigin, SystemId},
    };
    use std::{
        collections::hash_map::RandomState,
        fs,
        hash::{BuildHasher, Hasher},
        path::Path,
        process::Command,
    };
    pub(crate) struct TestDirectory(pub(crate) PathBuf);
    impl TestDirectory {
        pub(crate) fn new() -> Self {
            let nonce = RandomState::new().build_hasher().finish();
            let path = std::env::temp_dir().join(format!(
                "meridian-profile-test-{}-{nonce:016x}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap_or_else(|e| panic!("{e}"));
            Self(path)
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    pub(crate) fn open(path: &Path) -> Profile {
        Profile::open(path.to_path_buf()).unwrap_or_else(|e| panic!("{e}"))
    }
    pub(super) fn action(profile: &mut Profile, value: Action) {
        profile.command(value).unwrap_or_else(|e| panic!("{e}"));
    }
    fn travel(profile: &mut Profile) {
        action(profile, Action::Launch);
        profile
            .world
            .step(FlightInput::default())
            .unwrap_or_else(|e| panic!("{e}"));
        action(profile, Action::Jump);
        for _ in 0..BASIC_SPOOL_TICKS {
            profile
                .world
                .step(FlightInput::default())
                .unwrap_or_else(|e| panic!("{e}"));
        }
        for _ in 0..9000 {
            let state = profile.world.snapshot();
            if state.mode == FlightMode::Docked {
                break;
            }
            let speed = if state.position[2] > 2500.0 {
                34.3
            } else {
                4.0
            };
            profile
                .world
                .step(FlightInput {
                    throttle: ((speed - state.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                    ..FlightInput::default()
                })
                .unwrap_or_else(|e| panic!("{e}"));
        }
        assert_eq!(
            profile.world.current_dock_context().map(|d| d.origin()),
            Some(DockOrigin::PhysicalEntry)
        );
        assert_eq!(
            profile.world.snapshot().navigation.map(|n| n.system),
            Some(SystemId::RENAS)
        );
    }
    #[test]
    fn missile_cold_continue_child() {
        let Some(path) = std::env::var_os("MERIDIAN_MISSILE_TEST_SLOT") else {
            return;
        };
        let directory = Path::new(&path);
        let before =
            fs::read(directory.join("commander.meridian")).unwrap_or_else(|e| panic!("{e}"));
        let expected = fs::read(directory.join("expected-after-tick.meridian"))
            .unwrap_or_else(|e| panic!("{e}"));
        let mut profile = open(directory);
        profile.continue_saved().unwrap_or_else(|e| panic!("{e}"));
        // Continue intentionally invalidates prior UI reviews. The whole encoded
        // state must match the public loader's one transition, not the old file.
        let continued_before = World::from_save_bytes(&before)
            .unwrap_or_else(|e| panic!("{e}"))
            .save_bytes()
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}")),
            continued_before
        );
        assert_eq!(
            fs::read(directory.join("commander.meridian")).unwrap_or_else(|e| panic!("{e}")),
            before
        );
        assert!(profile.world.mission_snapshot().is_some_and(|m| m.paused));
        let missiles = profile
            .world
            .missile_snapshot()
            .unwrap_or_else(|| panic!("missing missiles"));
        assert_eq!(missiles.rounds.iter().flatten().count(), 2);
        assert_eq!(
            missiles
                .projectiles
                .iter()
                .flatten()
                .filter(|p| p.ended.is_none())
                .count(),
            1
        );
        let expected = World::from_save_bytes(&expected)
            .unwrap_or_else(|e| panic!("{e}"))
            .save_bytes()
            .unwrap_or_else(|e| panic!("{e}"));
        profile
            .world
            .request_pause(false, profile.world.snapshot().tick)
            .unwrap_or_else(|e| panic!("{e}"));
        profile
            .world
            .step(Default::default())
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}")),
            expected
        );
        profile.prepare_quit().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            fs::read(directory.join("commander.meridian")).unwrap_or_else(|e| panic!("{e}")),
            expected
        );
    }

    #[test]
    fn real_process_preserves_paused_live_missile_and_resumes_like_uninterrupted_world() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(
            profile
                .new_commander(9610)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        action(&mut profile, Action::ReviewPractice);
        action(&mut profile, Action::Confirm);
        action(&mut profile, Action::Launch);
        profile
            .world
            .step(Default::default())
            .unwrap_or_else(|e| panic!("{e}"));
        action(&mut profile, Action::CycleMissileLock);
        action(&mut profile, Action::ToggleMissileArm);
        action(&mut profile, Action::FireMissile);
        for _ in 0..6 {
            profile
                .world
                .step(Default::default())
                .unwrap_or_else(|e| panic!("{e}"));
        }
        profile
            .world
            .request_pause(true, profile.world.snapshot().tick)
            .unwrap_or_else(|e| panic!("{e}"));
        let origin = profile
            .world
            .missile_snapshot()
            .unwrap_or_else(|| panic!("missing missiles"));
        profile.prepare_quit().unwrap_or_else(|e| panic!("{e}"));
        profile
            .world
            .request_pause(false, profile.world.snapshot().tick)
            .unwrap_or_else(|e| panic!("{e}"));
        profile
            .world
            .step(Default::default())
            .unwrap_or_else(|e| panic!("{e}"));
        let expected = profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        fs::write(directory.0.join("expected-after-tick.meridian"), &expected)
            .unwrap_or_else(|e| panic!("{e}"));
        let expected_missiles = profile.world.missile_snapshot();
        let expected_practice = profile.world.practice_snapshot();
        let expected_combat = profile.world.combat_snapshot();
        let expected_flight = profile.world.snapshot();
        let expected_mission = profile.world.mission_snapshot();
        let continued_expected = World::from_save_bytes(&expected)
            .unwrap_or_else(|e| panic!("{e}"))
            .save_bytes()
            .unwrap_or_else(|e| panic!("{e}"));
        drop(profile);
        let executable = std::env::current_exe().unwrap_or_else(|e| panic!("{e}"));
        let status = Command::new(executable)
            .args([
                "--exact",
                "profile::tests::missile_cold_continue_child",
                "--nocapture",
            ])
            .env("MERIDIAN_MISSILE_TEST_SLOT", &directory.0)
            .status()
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(status.success());
        assert_eq!(
            fs::read(directory.0.join("commander.meridian")).unwrap_or_else(|e| panic!("{e}")),
            continued_expected
        );
        let mut cold = open(&directory.0);
        cold.continue_saved().unwrap_or_else(|e| panic!("{e}"));
        // These expectations come directly from the uninterrupted producer,
        // independently of the public-loader byte comparison in the child.
        assert_eq!(cold.world.missile_snapshot(), expected_missiles);
        assert_eq!(cold.world.practice_snapshot(), expected_practice);
        assert_eq!(cold.world.combat_snapshot(), expected_combat);
        assert_eq!(cold.world.snapshot(), expected_flight);
        assert_eq!(cold.world.mission_snapshot(), expected_mission);
        let continued = cold
            .world
            .missile_snapshot()
            .unwrap_or_else(|| panic!("missing missiles"));
        assert_eq!(continued.rounds, origin.rounds);
        assert_eq!(
            continued.projectiles.map(|p| p.map(|p| p.origin)),
            origin.projectiles.map(|p| p.map(|p| p.origin))
        );
    }

    #[test]
    fn cold_continue_child() {
        // Normal test discovery does no work; only the parent's explicit fresh child opens its slot.
        let Some(path) = std::env::var_os("MERIDIAN_PROFILE_TEST_SLOT") else {
            return;
        };
        let mut profile = open(Path::new(&path));
        profile.continue_saved().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            profile
                .world
                .mission_snapshot()
                .map(|m| m.freight.held_tonnes),
            Some(1)
        );
        assert_eq!(profile.world.credits_tenths(), Some(7182));
        assert!(profile.world.active_freight_job().is_some());
        action(&mut profile, Action::Contact);
        action(&mut profile, Action::Review);
        action(&mut profile, Action::Confirm);
        assert_eq!(profile.world.credits_tenths(), Some(14482));
        assert!(
            profile
                .world
                .mission_snapshot()
                .and_then(|m| m.freight.clue)
                .is_some()
        );
        profile.prepare_quit().unwrap_or_else(|e| panic!("{e}"));
    }
    #[test]
    fn real_process_cold_continue_completes_eighth_unit_once_and_survives_another_continue() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(!profile.playing());
        assert!(profile.save().is_err());
        assert!(profile.command(Action::Contact).is_err());
        assert!(
            profile
                .new_commander(9301)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        action(&mut profile, Action::BuySelected);
        action(&mut profile, Action::SellSelected);
        for value in [Action::Contact, Action::Review, Action::Confirm] {
            action(&mut profile, value);
        }
        travel(&mut profile);
        action(&mut profile, Action::Contact);
        for _ in 0..7 {
            action(&mut profile, Action::Review);
            action(&mut profile, Action::Confirm);
        }
        profile.save().unwrap_or_else(|e| panic!("{e}"));
        drop(profile);
        let executable = std::env::current_exe().unwrap_or_else(|e| panic!("{e}"));
        let status = Command::new(executable)
            .args([
                "--exact",
                "profile::tests::cold_continue_child",
                "--nocapture",
            ])
            .env("MERIDIAN_PROFILE_TEST_SLOT", &directory.0)
            .status()
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(status.success());
        let mut continued = open(&directory.0);
        continued.continue_saved().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(continued.world.credits_tenths(), Some(14482));
        assert_eq!(
            continued
                .world
                .mission_snapshot()
                .map(|m| (m.freight.held_tonnes, m.freight.completed_jobs)),
            Some((0, 1))
        );
        action(&mut continued, Action::Contact);
        assert!(continued.command(Action::Review).is_err());
        assert!(continued.command(Action::Confirm).is_err());
        assert_eq!(continued.world.credits_tenths(), Some(14482));
    }
    #[test]
    fn corrupt_continue_refuses_without_fallback_and_new_game_needs_explicit_confirmation() {
        let directory = TestDirectory::new();
        let save = directory.0.join("commander.meridian");
        fs::write(&save, b"bad save").unwrap_or_else(|e| panic!("{e}"));
        let mut profile = open(&directory.0);
        let before = profile.world.snapshot();
        assert!(profile.continue_saved().is_err());
        assert_eq!(profile.world.snapshot(), before);
        assert!(!profile.playing());
        assert!(
            !profile
                .new_commander(9401)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        assert_eq!(profile.world.snapshot(), before);
        assert!(
            profile
                .new_commander(9401)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        assert_eq!(
            fs::read(&save).unwrap_or_else(|e| panic!("{e}")),
            b"bad save"
        );
        profile.save().unwrap_or_else(|e| panic!("{e}"));
        let prior = profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        fs::write(&save, b"damaged replacement").unwrap_or_else(|e| panic!("{e}"));
        assert!(profile.continue_saved().is_err());
        assert_eq!(
            profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}")),
            prior
        );
        assert_eq!(
            fs::read(&save).unwrap_or_else(|e| panic!("{e}")),
            b"damaged replacement"
        );
    }
    #[test]
    fn older_formats_refuse_from_title_and_live_reload_without_disk_replacement() {
        let fixtures: [&[u8]; 2] = [
            include_bytes!("../../meridian-core/tests/fixtures/qualified-v15-npc-flight.bin"),
            include_bytes!("../../meridian-core/tests/fixtures/pr44-v16-m02-purchases.bin"),
        ];
        for bytes in fixtures {
            let directory = TestDirectory::new();
            let destination = directory.0.join("commander.meridian");
            fs::write(&destination, bytes).unwrap_or_else(|e| panic!("{e}"));
            let mut profile = open(&directory.0);
            let opening = profile.world.snapshot();
            let error = profile
                .continue_saved()
                .err()
                .unwrap_or_else(|| panic!("an unsupported save was accepted"));
            assert!(error.contains("Unsupported commander save format"));
            assert!(error.contains("format 17"));
            assert!(!profile.playing());
            assert_eq!(profile.world.snapshot(), opening);
            assert_eq!(
                fs::read(&destination).unwrap_or_else(|e| panic!("{e}")),
                bytes
            );
            profile.prepare_quit().unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                fs::read(&destination).unwrap_or_else(|e| panic!("{e}")),
                bytes
            );

            assert!(
                !profile
                    .new_commander(15910)
                    .unwrap_or_else(|e| panic!("{e}"))
            );
            assert!(
                profile
                    .new_commander(15910)
                    .unwrap_or_else(|e| panic!("{e}"))
            );
            let before = profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
            assert!(
                profile
                    .continue_saved()
                    .err()
                    .unwrap_or_else(|| panic!("an unsupported save was accepted"))
                    .contains("format 17")
            );
            assert!(profile.playing());
            assert_eq!(
                profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}")),
                before
            );
            assert_eq!(
                fs::read(&destination).unwrap_or_else(|e| panic!("{e}")),
                bytes
            );
        }
    }

    #[test]
    fn failed_quit_save_cannot_claim_exit_ready_or_change_the_live_world() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(profile.prepare_quit().is_ok());
        assert!(
            profile
                .new_commander(9501)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        let before = profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        let destination = directory.0.join("commander.meridian");
        fs::create_dir(&destination).unwrap_or_else(|e| panic!("{e}"));
        fs::write(destination.join("sentinel"), b"retain").unwrap_or_else(|e| panic!("{e}"));
        assert!(profile.prepare_quit().is_err());
        assert!(profile.playing());
        assert_eq!(
            profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}")),
            before
        );
        assert_eq!(
            fs::read(destination.join("sentinel")).unwrap_or_else(|e| panic!("{e}")),
            b"retain"
        );
    }
    #[test]
    fn practice_confirmation_publishes_after_docked_ticks_and_survives_cold_continue() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(
            profile
                .new_commander(9502)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        action(&mut profile, Action::ReviewPractice);
        action(&mut profile, Action::Back);
        assert!(profile.command(Action::Confirm).is_err());
        assert!(!directory.0.join("commander.meridian").exists());

        action(&mut profile, Action::ReviewPractice);
        profile
            .world
            .step(Default::default())
            .unwrap_or_else(|e| panic!("{e}"));
        action(&mut profile, Action::Confirm);
        assert_eq!(
            profile
                .world
                .practice_snapshot()
                .unwrap_or_else(|| panic!("no practice snapshot"))
                .status,
            meridian_core::PracticeStatus::Consented
        );
        let bytes = profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            fs::read(directory.0.join("commander.meridian")).unwrap_or_else(|e| panic!("{e}")),
            bytes
        );
        drop(profile);
        let mut cold = open(&directory.0);
        cold.continue_saved().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            cold.world
                .practice_snapshot()
                .unwrap_or_else(|| panic!("no practice snapshot"))
                .status,
            meridian_core::PracticeStatus::Consented
        );
        assert!(cold.command(Action::ReviewPractice).is_err());
    }

    #[test]
    fn practice_publication_failure_preserves_the_live_world_and_does_not_consent() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(
            profile
                .new_commander(9503)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        profile.save().unwrap_or_else(|e| panic!("{e}"));
        let destination = directory.0.join("commander.meridian");
        let previous_slot = directory.0.join("previous-slot");
        fs::rename(&destination, &previous_slot).unwrap_or_else(|e| panic!("{e}"));
        fs::create_dir(&destination).unwrap_or_else(|e| panic!("{e}"));
        let before = profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        action(&mut profile, Action::ReviewPractice);
        assert!(profile.command(Action::Confirm).is_err());
        assert_eq!(
            profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}")),
            before
        );
        assert_eq!(
            fs::read(previous_slot).unwrap_or_else(|e| panic!("{e}")),
            before
        );
        assert!(profile.command(Action::Confirm).is_err());
        assert!(profile.command(Action::ReviewPractice).is_ok());
    }

    #[test]
    fn practice_closure_write_failure_preserves_open_attempt_and_prior_slot() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(
            profile
                .new_commander(9505)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        action(&mut profile, Action::ReviewPractice);
        action(&mut profile, Action::Confirm);
        let before = profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        action(&mut profile, Action::ReviewPracticeClosure);
        action(&mut profile, Action::Back);
        assert!(profile.command(Action::Confirm).is_err());
        assert_eq!(
            profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}")),
            before
        );
        let destination = directory.0.join("commander.meridian");
        let retained = directory.0.join("retained-valid-slot");
        fs::rename(&destination, &retained).unwrap_or_else(|e| panic!("{e}"));
        fs::create_dir(&destination).unwrap_or_else(|e| panic!("{e}"));
        fs::write(destination.join("sentinel"), b"retain").unwrap_or_else(|e| panic!("{e}"));
        action(&mut profile, Action::ReviewPracticeClosure);
        assert!(profile.command(Action::Confirm).is_err());
        assert_eq!(
            profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}")),
            before
        );
        assert_eq!(fs::read(retained).unwrap_or_else(|e| panic!("{e}")), before);
        assert_eq!(
            fs::read(destination.join("sentinel")).unwrap_or_else(|e| panic!("{e}")),
            b"retain"
        );
        assert!(profile.command(Action::Confirm).is_err());
        assert!(profile.command(Action::ReviewPractice).is_err());
        assert!(profile.command(Action::ReviewPracticeClosure).is_ok());
    }

    #[test]
    fn practice_lifecycle_cold_child() {
        let Some(path) = std::env::var_os("MERIDIAN_PRACTICE_LIFECYCLE_SLOT") else {
            return;
        };
        let directory = Path::new(&path);
        let mut profile = open(directory);
        profile.continue_saved().unwrap_or_else(|e| panic!("{e}"));
        let ledger = profile
            .world
            .practice_ledger_snapshot()
            .unwrap_or_else(|| panic!("no ledger"));
        assert_eq!(ledger.attempts.len(), 1);
        assert_eq!(ledger.active, None);
        assert_eq!(
            ledger.attempts[0].outcome,
            Some(meridian_core::PracticeOutcome::Abandoned)
        );
        assert_eq!(ledger.trace, None);
        assert!(profile.command(Action::ReviewPracticeReception).is_err());
        assert!(profile.command(Action::ReviewPracticeClosure).is_err());
        action(&mut profile, Action::ReviewPractice);
        action(&mut profile, Action::Confirm);
        let retry = profile
            .world
            .practice_ledger_snapshot()
            .unwrap_or_else(|| panic!("no ledger"));
        assert_eq!(retry.attempts.len(), 2);
        assert_eq!(retry.attempts[0], ledger.attempts[0]);
        assert_ne!(retry.attempts[0].id, retry.attempts[1].id);
        assert_eq!(retry.active, Some(retry.attempts[1].id));
        assert_eq!(retry.trace, None);
        assert!(profile.command(Action::ReviewPractice).is_err());
        profile.prepare_quit().unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn actual_new_process_continues_closed_attempt_and_explicitly_saves_distinct_retry() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(
            profile
                .new_commander(9506)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        action(&mut profile, Action::ReviewPractice);
        action(&mut profile, Action::Confirm);
        let missiles = profile.world.missile_snapshot();
        let combat = profile.world.combat_snapshot();
        let credits = profile.world.credits_tenths();
        action(&mut profile, Action::ReviewPracticeClosure);
        action(&mut profile, Action::Confirm);
        let closed = profile
            .world
            .practice_ledger_snapshot()
            .unwrap_or_else(|| panic!("no ledger"));
        let saved =
            fs::read(directory.0.join("commander.meridian")).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            saved,
            profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}"))
        );
        assert_eq!(
            closed.attempts[0].outcome,
            Some(meridian_core::PracticeOutcome::Abandoned)
        );
        assert_eq!(closed.active, None);
        assert_eq!(profile.world.missile_snapshot(), missiles);
        assert_eq!(profile.world.combat_snapshot(), combat);
        assert_eq!(profile.world.credits_tenths(), credits);
        drop(profile);
        let status = Command::new(std::env::current_exe().unwrap_or_else(|e| panic!("{e}")))
            .args([
                "--exact",
                "profile::tests::practice_lifecycle_cold_child",
                "--nocapture",
            ])
            .env("MERIDIAN_PRACTICE_LIFECYCLE_SLOT", &directory.0)
            .status()
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(status.success());
        let mut cold = open(&directory.0);
        cold.continue_saved().unwrap_or_else(|e| panic!("{e}"));
        let retry = cold
            .world
            .practice_ledger_snapshot()
            .unwrap_or_else(|| panic!("no ledger"));
        assert_eq!(retry.attempts.len(), 2);
        assert_eq!(retry.attempts[0], closed.attempts[0]);
        assert_eq!(retry.attempts[1].outcome, None);
        assert_eq!(retry.active, Some(retry.attempts[1].id));
        assert_eq!(retry.offers_remaining + 1, closed.offers_remaining);
        assert_eq!(retry.trace, None);
        assert_eq!(cold.world.missile_snapshot(), missiles);
        assert_eq!(cold.world.combat_snapshot(), combat);
        assert_eq!(cold.world.credits_tenths(), credits);
    }

    #[test]
    fn premature_reception_clears_consent_review_without_publishing() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(
            profile
                .new_commander(9504)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        profile.save().unwrap_or_else(|e| panic!("{e}"));
        let destination = directory.0.join("commander.meridian");
        let before = fs::read(&destination).unwrap_or_else(|e| panic!("{e}"));

        action(&mut profile, Action::ReviewPractice);
        assert!(profile.command(Action::ReviewPracticeReception).is_err());
        assert!(profile.command(Action::Confirm).is_err());
        assert_eq!(
            fs::read(destination).unwrap_or_else(|e| panic!("{e}")),
            before
        );
        assert_eq!(
            profile.world.save_bytes().unwrap_or_else(|e| panic!("{e}")),
            before
        );
    }

    #[test]
    fn explicit_slot_arguments_are_exact_and_do_not_guess_on_invalid_input() {
        assert_eq!(
            directory_from_args(["--save-dir", "fixture-slot"].map(Into::into).into_iter())
                .unwrap_or_else(|e| panic!("{e}")),
            PathBuf::from("fixture-slot")
        );
        for args in [
            vec!["--save-dir"],
            vec!["--save-dir", ""],
            vec!["unknown"],
            vec!["--save-dir", "slot", "extra"],
        ] {
            assert!(directory_from_args(args.into_iter().map(Into::into)).is_err());
        }
    }
}

#[cfg(test)]
mod flight_lesson_tests;

fn publication_message(published: SaveCommit, message: String) -> Result<String, String> {
    match published {
        SaveCommit::Published {
            directory_sync_error: Some(error),
        } => Err(format!(
            "Transaction was written, but directory synchronization failed: {error}"
        )),
        _ => Ok(message),
    }
}

#[cfg(test)]
mod trade_lesson_tests;

#[cfg(test)]
#[path = "profile/ammunition_tests.rs"]
mod ammunition_tests;

#[cfg(test)]
#[path = "profile/traffic_tests.rs"]
mod traffic_tests;

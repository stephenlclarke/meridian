pub(crate) mod emp;
pub use emp::{
    EmpAxis, EmpChargePhase, EmpEquipmentAction, EmpEquipmentOffer, EmpEquipmentReview, EmpError,
    EmpGrades, EmpOrigin, EmpProvenance, EmpSnapshot, OwnedEmp, PreparedEmpEquipment,
};
mod celestial;
mod damage;
mod goshawk_validation;
pub use goshawk_validation::GoshawkCustomApproachSnapshot;
pub use goshawk_validation::{GoshawkApproachSnapshot, GoshawkApproachStatus};
mod mining;
mod mule_renas;
mod mule_validation;
mod xyzzy;
pub(crate) use xyzzy::{AccountEffect, AccountEvent, STARTING_CREDITS_TENTHS};
pub use xyzzy::{PreparedXyzzy, XyzzyAction, XyzzyError, XyzzyReview, XyzzySnapshot};
mod navigation_trade;
mod route;
pub use route::PreparedNavigationRoute;
mod auto_docking;
mod targeting;
mod traffic;
pub use navigation_trade::{
    NavigationTonneId, NavigationTradeAction, NavigationTradeAttemptSnapshot, NavigationTradeError,
    NavigationTradeId, NavigationTradeMarketSnapshot, NavigationTradeOutcome,
    NavigationTradePlanSnapshot, NavigationTradeReview, NavigationTradeSnapshot,
    NavigationTradeTrace, PreparedNavigationTrade, PreparedStationService,
};
pub use targeting::{TargetIdentity, TargetMode, TargetingSnapshot, TargetingViewport};
mod flight_reception;
pub use flight_reception::{
    FLIGHT_RECEPTION_MAX_BERTH_RECORDS, FlightReceptionAction, FlightReceptionAttemptSnapshot,
    FlightReceptionError, FlightReceptionId, FlightReceptionOutcome, FlightReceptionReview,
    FlightReceptionSnapshot, FlightReceptionTrace, FlightView, PreparedFlightReception,
};
mod modular;
pub use modular::{
    FittedFuelScoops, FittedMiningTool, FuelScoopsSnapshot, ModularEquipmentSnapshot, ModuleAction,
    ModuleError, ModuleKind, ModuleOffer, ModuleProvenance, ModuleReview, ModuleSlot, OwnedModule,
    PreparedModule,
};
mod ammunition;
mod missiles;
pub use ammunition::{
    AmmunitionAction, AmmunitionError, AmmunitionItem, AmmunitionReview, AmmunitionSnapshot,
    MissilePurchase, PreparedAmmunition,
};
pub use missiles::{
    MISSILE_COOLDOWN_TICKS, MISSILE_FACES, MISSILE_LIFETIME_TICKS, MISSILE_MUZZLE_DISTANCE,
    MISSILE_RANGE, MISSILE_SPEED, MISSILE_VERTICES, MissileContact, MissileEnd, MissileError,
    MissileOrigin, MissileRound, MissileSnapshot, MissileTarget, ProjectileId, ProjectileSnapshot,
};
mod combat;
pub use combat::{
    CombatIdentity, CombatSnapshot, OrdinaryImpact, OrdinaryTarget, OrdinaryWeapon,
    PlayerCondition, SHIELD_CAPACITY,
};
mod equipment;
pub use equipment::{EquipmentPreparationError, PreparedEquipment};
mod mission;
mod persistence;
mod practice;
pub use practice::{
    PRACTICE_HALF_SIDE, PRACTICE_MAX_OFFERS, PRACTICE_QUIET_TICKS, PracticeAttemptId,
    PracticeAttemptSnapshot, PracticeBeaconId, PracticeBeaconSnapshot, PracticeClosureReview,
    PracticeError, PracticeLedgerSnapshot, PracticeOutcome, PracticeReceptionReview,
    PracticeReceptionSnapshot, PracticeReview, PracticeSnapshot, PracticeStatus, PracticeTrace,
    PreparedPracticeConfirmation,
};
mod profit_scan;
mod services;
pub use profit_scan::ProfitDestination;
mod weapons;
pub use mission::{MissionError, MissionSnapshot, StationNpc};
pub use services::ServiceSnapshot;
pub use weapons::{
    ENERGY_CAPACITY_HUNDREDTHS, FireError, FlightResourcesSnapshot, FrontLaserSnapshot,
    PULSE_COOLDOWN_TICKS, PULSE_RANGE, PulseContact, PulseLaserSnapshot, PulseShot,
};

use crate::docking::StationPose;
use crate::freight::g02::{CommanderAccount, FreshCommanderIds, StarterComponent};
use crate::navigation::{
    CommanderId, DockContext, NavigationError, NavigationSnapshot, NavigationState,
    OriginalNavigation, PlayerShipId, SystemId,
};
use crate::station::{
    DISTANCE_PER_SPEED_UNIT, FlightMode, FlightStart, MotionRefusal, OriginalStationData,
    StationError, flight_pose,
};
use core::fmt;

pub const FIXED_DELTA_SECONDS: f32 = 1.0 / 60.0;

// SH025 [game], immutable legacy a11fe16924d87d06021816f9037952afcbac08db.
// These are reference gameplay units, not an asserted SI or renderer-space calibration.
pub const MAX_SPEED: f32 = 34.3;
const ACCELERATION: f32 = 9.1;
const PITCH_RATE: f32 = 0.7;
const YAW_RATE: f32 = 0.65;
const ROLL_RATE: f32 = 0.79;

/// Held controls. Zero throttle coasts; negative throttle slows without reversing.
/// Positive pitch raises the nose, positive yaw turns toward local +X, and roll is about +Z.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct FlightInput {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub throttle: f32,
    pub brake: bool,
}

impl FlightInput {
    fn validate(self) -> Result<(), StepError> {
        let controls = [self.pitch, self.yaw, self.roll, self.throttle];
        if controls.iter().any(|value| !value.is_finite()) {
            return Err(StepError::NonFiniteInput);
        }
        if controls.iter().any(|value| !(-1.0..=1.0).contains(value)) {
            return Err(StepError::InputOutOfRange);
        }
        Ok(())
    }
}

/// Presentation value only. Quaternion order is xyzw, local forward +Z and up +Y.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
    pub position: [f32; 3],
    pub orientation: [f32; 4],
    pub speed: f32,
    pub tick: u64,
    pub mode: FlightMode,
    pub station: Option<StationPose>,
    pub motion_refusal: Option<MotionRefusal>,
    pub navigation: Option<NavigationSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepError {
    NonFiniteInput,
    InputOutOfRange,
    TickOverflow,
    NonFiniteState,
    StaleCommand,
    InvalidMode,
    Collision,
    UnsafeEntry,
    UnsafeLaunch,
    Paused,
}

impl fmt::Display for StepError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFiniteInput => "flight input must be finite",
            Self::InputOutOfRange => "held flight controls must be within [-1, 1]",
            Self::TickOverflow => "simulation tick exhausted",
            Self::NonFiniteState => "flight step cannot produce a valid finite pose",
            Self::StaleCommand => "command does not match the current simulation tick",
            Self::InvalidMode => "command is not available in the current flight mode",
            Self::Collision => "continuous native station clearance refused the flight step",
            Self::UnsafeEntry => "original station entry conditions are not satisfied",
            Self::Paused => "World is paused",
            Self::UnsafeLaunch => "source-defined original station launch is unavailable",
        })
    }
}

impl std::error::Error for StepError {}

/// One owner mutates World; callers receive small immutable presentation values.
#[derive(Debug)]
pub struct World {
    state: Snapshot,
    traffic: crate::traffic::State,
    targeting: targeting::State,
    auto_docking: Option<auto_docking::Phase>,
    goshawk_validation: Option<goshawk_validation::Approach>,
    mule_validation: Option<crate::traffic::mule_validation::State>,
    mule_renas: Option<crate::traffic::mule_validation::State>,
    mining: crate::mining::State,
    mining_scope: std::sync::Arc<()>,
    damage: damage::State,
    station_data: Option<OriginalStationData>,
    navigation_catalogue: Option<OriginalNavigation>,
    navigation: Option<NavigationState>,
    navigation_route: Option<crate::navigation::RoutePlan>,
    jump_journal: crate::navigation::JumpJournal,
    commander_account: Option<CommanderAccount>,
    freight_session: Option<mission::FreightSession>,
    station_markets: Option<crate::station_services::StationMarkets>,
    pulse: weapons::PulseResources,
    combat: combat::State,
    missiles: missiles::State,
    ammunition: ammunition::State,
    modular: modular::State,
    xyzzy: xyzzy::State,
    emp: emp::State,
    navigation_trade: navigation_trade::State,
    flight_reception: flight_reception::State,
    practice: Option<practice::Practice>,
    practice_history: Vec<practice::Practice>,
    practice_trace: Option<practice::PracticeAttemptId>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            state: Snapshot {
                position: [0.0, 0.0, -600.0],
                orientation: [0.0, 0.0, 0.0, 1.0],
                speed: 0.0,
                tick: 0,
                mode: FlightMode::Flight,
                station: None,
                motion_refusal: None,
                navigation: None,
            },
            traffic: crate::traffic::State::default(),
            targeting: targeting::State::default(),
            auto_docking: None,
            goshawk_validation: None,
            mule_validation: None,
            mule_renas: None,
            mining: crate::mining::State::legacy_absent(),
            mining_scope: std::sync::Arc::new(()),
            damage: damage::State::default(),
            station_data: None,
            navigation_catalogue: None,
            navigation: None,
            navigation_route: None,
            jump_journal: crate::navigation::JumpJournal::default(),
            commander_account: None,
            freight_session: None,
            station_markets: None,
            pulse: weapons::PulseResources::default(),
            combat: combat::State::default(),
            missiles: missiles::State::default(),
            ammunition: ammunition::State::default(),
            modular: modular::State::default(),
            xyzzy: xyzzy::State::default(),
            emp: emp::State::default(),
            navigation_trade: navigation_trade::State::default(),
            flight_reception: flight_reception::State::default(),
            practice: None,
            practice_history: Vec::new(),
            practice_trace: None,
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let mut snapshot = self.state;
        snapshot.navigation = self
            .navigation
            .zip(self.commander_account.as_ref())
            .map(|(navigation, account)| navigation.snapshot(account.fuel_tenths()));
        snapshot
    }

    /// New profile using original Avenor/SH025/full-tank/owned Basic-drive defaults.
    /// The host owns unique profile seed allocation; the core has no global ID or wall clock.
    pub fn new_commander(seed: u64) -> Result<Self, NavigationError> {
        if seed == 0 || seed.checked_add(10).is_none() {
            return Err(NavigationError::InvalidProfileIdentity);
        }
        let ids = FreshCommanderIds {
            owner: seed,
            hull: seed + 1,
            inventory: seed + 2,
            items: core::array::from_fn(|index| seed + 3 + index as u64),
        };
        let account = CommanderAccount::born_sh025(ids)
            .map_err(|_| NavigationError::InvalidProfileIdentity)?;
        let drive = account
            .starter_items()
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.component() == StarterComponent::BasicDrive)
            })
            .ok_or(NavigationError::InvalidProfileIdentity)?;
        let navigation = NavigationState::new(seed, ids.hull, drive.id());
        let mut world =
            Self::at_original_station().map_err(|_| NavigationError::StationUnavailable)?;
        world.traffic = crate::traffic::State::fresh(seed);
        world.mining = crate::mining::State::default();
        world.damage = damage::State::fresh();
        world.navigation_catalogue = Some(OriginalNavigation::bundled()?);
        world.commander_account = Some(account);
        world.navigation = Some(navigation);
        world.freight_session = navigation.dock().map(mission::FreightSession::fresh);
        world.station_markets = Some(
            crate::station_services::StationMarkets::new_commander(seed)
                .map_err(|_| NavigationError::ServiceUnavailable)?,
        );
        Ok(world)
    }

    pub fn credits_tenths(&self) -> Option<u32> {
        self.commander_account
            .as_ref()
            .map(CommanderAccount::credits_tenths)
    }

    fn has_bound_drive(navigation: NavigationState, account: &CommanderAccount) -> bool {
        let Some(items) = account.starter_items() else {
            return false;
        };
        let owned = |item: &&crate::freight::g02::OwnedStarterItem| {
            item.owner() == navigation.owner().seed()
                && item.fitted_hull() == Some(navigation.ship().ordinal())
        };
        items.iter().filter(owned).any(|item| {
            item.component() == StarterComponent::BasicDrive
                && item.id() == navigation.drive_item_id()
        })
    }

    /// Fitted range before the current fuel limit, from the active hull's owned equipment.
    pub fn jump_reach_ly(&self) -> Option<f64> {
        let account = self.commander_account.as_ref()?;
        let navigation = self.navigation?;
        if !Self::has_bound_drive(navigation, account) {
            return None;
        }
        self.modular
            .ledger(self, account)
            .ok()
            .map(|ledger| ledger.jump_reach_ly())
    }

    pub fn commander_id(&self) -> Option<CommanderId> {
        self.navigation.map(NavigationState::owner)
    }

    pub fn player_ship_id(&self) -> Option<PlayerShipId> {
        self.navigation.map(NavigationState::ship)
    }

    pub fn current_dock_context(&self) -> Option<DockContext> {
        if self.state.mode != FlightMode::Docked {
            return None;
        }
        self.navigation.and_then(NavigationState::dock)
    }

    pub fn dock_context_is_current(&self, context: &DockContext) -> bool {
        self.current_dock_context().as_ref() == Some(context)
    }

    /// Begin a real fitted-drive jump. Fuel is charged once, only on actual arrival.
    pub fn request_jump(
        &mut self,
        target: SystemId,
        expected_tick: u64,
    ) -> Result<(), NavigationError> {
        if expected_tick != self.state.tick {
            return Err(NavigationError::StaleCommand);
        }
        if self.is_paused() {
            return Err(NavigationError::WrongMode);
        }
        if self.state.mode != FlightMode::Flight
            || self.emp.travel_blocked(&self.traffic)
            || self.combat_snapshot().is_some_and(|s| s.player.destroyed)
        {
            return Err(NavigationError::WrongMode);
        }
        let catalogue = self
            .navigation_catalogue
            .as_ref()
            .ok_or(NavigationError::WrongMode)?;
        let mut navigation = self.navigation.ok_or(NavigationError::WrongMode)?;
        let station = self
            .state
            .station
            .ok_or(NavigationError::StationUnavailable)?;
        let position = self.state.position.map(f64::from);
        let center = station.pose.position;
        let distance = (position[0] - center.x)
            .hypot(position[1] - center.y)
            .hypot(position[2] - center.z);
        let account = self
            .commander_account
            .as_ref()
            .ok_or(NavigationError::WrongMode)?;
        let reach = self.jump_reach_ly().ok_or(NavigationError::WrongMode)?;
        let origin = navigation.snapshot(0).system;
        navigation.begin_jump(catalogue, target, distance, account.fuel_tenths(), reach)?;
        if !self.jump_journal.has_room() {
            return Err(NavigationError::SequenceExhausted);
        }
        self.navigation = Some(navigation);
        self.cancel_route_on_manual_jump(origin, target);
        Ok(())
    }

    /// Explicit authored new-commander dock context; no in-flight arrival is inferred.
    pub fn at_original_station() -> Result<Self, StationError> {
        Self::at_station(OriginalStationData::bundled()?)
    }

    /// Desktop can supply the byte-exact original station/hull/port binding explicitly.
    pub fn at_station(data: OriginalStationData) -> Result<Self, StationError> {
        data.launch_start()?;
        let mut world = Self::new();
        world.state.position = [0.0; 3]; // No flight body exists while docked; this is not a bay.
        world.state.mode = FlightMode::Docked;
        world.state.station = Some(data.station_pose());
        world.station_data = Some(data);
        Ok(world)
    }

    /// Validated initial flight scenario/replay input, not a mutation/teleport command.
    pub fn in_flight_at_original_station(start: FlightStart) -> Result<Self, StationError> {
        let data = OriginalStationData::bundled()?;
        let norm = start.orientation.iter().map(|v| v * v).sum::<f32>();
        if !start.speed.is_finite()
            || !(0.0..=MAX_SPEED).contains(&start.speed)
            || !norm.is_finite()
            || (norm - 1.0).abs() > 1e-5
            || !data.initial_flight_is_clear(start)
        {
            return Err(StationError::InvalidPose);
        }
        let mut world = Self::at_station(data)?;
        world.state.position = start.position;
        world.state.orientation = normalize(start.orientation).ok_or(StationError::InvalidPose)?;
        world.state.speed = start.speed;
        world.state.mode = FlightMode::Flight;
        Ok(world)
    }

    pub fn station_port(&self) -> Option<&crate::docking::DockingPort> {
        self.station_data.as_ref().map(OriginalStationData::port)
    }

    /// Stage the source-derived safe exterior spawn, then normal ticks finish launch.
    /// Rejected, duplicate and stale commands leave the complete state unchanged.
    pub fn request_launch(&mut self, expected_tick: u64) -> Result<(), StepError> {
        self.launch(expected_tick, false)
    }

    /// Resume and depart from the current commander berth as one World transaction.
    /// A refused departure also preserves the original pause and review sequence.
    pub fn request_launch_from_commander(&mut self, expected_tick: u64) -> Result<(), StepError> {
        self.launch(expected_tick, true)
    }

    fn launch(&mut self, expected_tick: u64, from_commander: bool) -> Result<(), StepError> {
        if expected_tick != self.state.tick {
            return Err(StepError::StaleCommand);
        }
        if self.is_paused() && !from_commander {
            return Err(StepError::Paused);
        }
        if self.state.mode != FlightMode::Docked
            || (from_commander && self.current_dock_context().is_none())
            || self.combat_snapshot().is_some_and(|s| s.player.destroyed)
        {
            return Err(StepError::InvalidMode);
        }
        let data = self.station_data.as_ref().ok_or(StepError::InvalidMode)?;
        let start = data.launch_start().map_err(|_| StepError::UnsafeLaunch)?;
        let mut next = self.state;
        next.position = start.position;
        next.orientation = start.orientation;
        next.speed = start.speed;
        next.mode = FlightMode::Launching;
        next.motion_refusal = None;
        let launch_pose =
            flight_pose(next.position, next.orientation).ok_or(StepError::UnsafeLaunch)?;
        if !self.traffic_motion_clear(launch_pose, launch_pose) {
            return Err(StepError::UnsafeLaunch);
        }
        let mut navigation = self.navigation;
        if let Some(state) = &mut navigation {
            state.launch().map_err(|_| StepError::TickOverflow)?;
        }
        let mission_epoch = self
            .freight_session
            .as_ref()
            .map(mission::FreightSession::launch_epoch)
            .transpose()
            .map_err(|_| StepError::TickOverflow)?;
        let mule_renas = self
            .prepare_renas_mule(next.mode, navigation)
            .map_err(|_| StepError::UnsafeLaunch)?;
        if let Some(actor) = &mule_renas {
            self.validate_renas_mule_launch(actor, launch_pose)?;
        }
        if let Some(account) = &mut self.commander_account {
            account.record_departure_legality();
        }
        self.state = next;
        self.navigation = navigation;
        self.missiles.berth();
        self.mule_renas = mule_renas;
        if let Some((session, epoch)) = self.freight_session.as_mut().zip(mission_epoch) {
            session.publish_launch(epoch);
        }
        Ok(())
    }

    /// Advance exactly one 1/60-second tick. Errors leave World unchanged.
    /// A motion refusal is an accepted safe stop at the last clear pose,
    /// reported through Snapshot.motion_refusal; damage/contact-response parity is pending.
    /// Rotation is local yaw, then pitch, then roll; translation uses the new pose/speed.
    /// Brake overrides throttle and uses the same reference deceleration magnitude.
    pub fn step(&mut self, input: FlightInput) -> Result<(), StepError> {
        if self.is_paused() {
            return Err(StepError::Paused);
        }
        input.validate()?;
        if let Some(result) = self.controlled_hull_step(input) {
            return result;
        }
        let previous_targeting = self.targeting.clone();
        let previous_docking = self.auto_docking;
        let input = self.flight_computer_input(input);
        let result = self.step_flight(input);
        if result.is_err() {
            self.targeting = previous_targeting;
            self.auto_docking = previous_docking;
        } else if self.state.mode != FlightMode::Flight || self.state.motion_refusal.is_some() {
            self.cancel_auto_docking();
        }
        result
    }

    fn step_flight(&mut self, input: FlightInput) -> Result<(), StepError> {
        let input = if self.emp.electrical_disabled() {
            FlightInput::default()
        } else {
            input
        };
        if self.combat_snapshot().is_some_and(|s| s.player.destroyed) {
            return Err(StepError::InvalidMode);
        }
        if !self.combat.can_advance(self.state.mode) {
            return Err(StepError::TickOverflow);
        }
        let mut next = self.state;
        next.tick = next.tick.checked_add(1).ok_or(StepError::TickOverflow)?;
        next.motion_refusal = None;
        if next.mode == FlightMode::Docked {
            return self.publish_step(next);
        }
        if !(0.0..=MAX_SPEED).contains(&self.state.speed) {
            return Err(StepError::NonFiniteState);
        }

        let yaw = axis_rotation(1, input.yaw * YAW_RATE * FIXED_DELTA_SECONDS);
        let pitch = axis_rotation(0, -input.pitch * PITCH_RATE * FIXED_DELTA_SECONDS);
        let roll = axis_rotation(2, input.roll * ROLL_RATE * FIXED_DELTA_SECONDS);
        next.orientation = normalize(multiply(
            multiply(multiply(next.orientation, yaw), pitch),
            roll,
        ))
        .ok_or(StepError::NonFiniteState)?;

        let throttle = if input.brake { -1.0 } else { input.throttle };
        next.speed =
            (next.speed + throttle * ACCELERATION * FIXED_DELTA_SECONDS).clamp(0.0, MAX_SPEED);
        let forward = forward(next.orientation);
        let distance_scale = if self.station_data.is_some() {
            DISTANCE_PER_SPEED_UNIT
        } else {
            1.0
        };
        for (position, direction) in next.position.iter_mut().zip(forward) {
            *position += direction * next.speed * FIXED_DELTA_SECONDS * distance_scale;
        }
        if !next.speed.is_finite() || next.position.iter().any(|value| !value.is_finite()) {
            return Err(StepError::NonFiniteState);
        }
        if self.celestial_blocks_motion(next.position) {
            let mut stopped = self.state;
            stopped.tick = next.tick;
            stopped.speed = 0.0;
            stopped.motion_refusal = Some(MotionRefusal::CelestialContact);
            return self.publish_step(stopped);
        }
        if let Some(data) = &self.station_data {
            let before = flight_pose(self.state.position, self.state.orientation)
                .ok_or(StepError::NonFiniteState)?;
            let after =
                flight_pose(next.position, next.orientation).ok_or(StepError::NonFiniteState)?;
            let angular_travel = f64::from(
                input.pitch.abs() * PITCH_RATE
                    + input.yaw.abs() * YAW_RATE
                    + input.roll.abs() * ROLL_RATE,
            ) * f64::from(FIXED_DELTA_SECONDS);
            if !self.traffic_and_renas_mule_motion_clear(before, after, angular_travel) {
                let mut stopped = self.state;
                stopped.tick = next.tick;
                stopped.speed = 0.0;
                stopped.motion_refusal = Some(MotionRefusal::NpcContact);
                return self.publish_step(stopped);
            }
            if !self.mining_motion_clear(before, after, angular_travel) {
                let mut stopped = self.state;
                stopped.tick = next.tick;
                stopped.speed = 0.0;
                stopped.motion_refusal = Some(MotionRefusal::AsteroidContact);
                return self.publish_step(stopped);
            }
            if data.crosses_aperture(before, after) {
                if !data.entry_clear(
                    before,
                    after,
                    f64::from(next.speed * DISTANCE_PER_SPEED_UNIT),
                    f64::from(input.roll * ROLL_RATE),
                    angular_travel,
                ) {
                    let mut stopped = self.state;
                    stopped.tick = next.tick;
                    stopped.speed = 0.0;
                    stopped.mode = FlightMode::Flight;
                    stopped.motion_refusal = Some(MotionRefusal::EntryConditions);
                    return self.publish_step(stopped);
                }
                next.mode = FlightMode::Docked;
                next.speed = 0.0;
            } else {
                if !data.motion_clear(before, after, angular_travel) {
                    let mut stopped = self.state;
                    stopped.tick = next.tick;
                    stopped.speed = 0.0;
                    stopped.mode = FlightMode::Flight;
                    stopped.motion_refusal = Some(MotionRefusal::StationContact);
                    return self.publish_step(stopped);
                }
                next.mode = FlightMode::Flight;
            }
        }
        self.publish_step(next)
    }

    fn celestial_blocks_motion(&self, position: [f32; 3]) -> bool {
        if let Some(scene) = self.celestial_snapshot() {
            let hull_radius = self
                .station_data
                .as_ref()
                .map_or(0.0, |data| data.player_radius());
            [scene.planet, scene.sun]
                .into_iter()
                .any(|body| body.blocks_motion(self.state.position, position, hull_radius))
        } else {
            false
        }
    }

    fn publish_step(&mut self, mut next: Snapshot) -> Result<(), StepError> {
        let flight_reception_start = self.flight_reception_start();
        let mut navigation = self.navigation;
        let mut mission_epoch = None;
        let mut planned_debit = None;
        let mut completed_leg = None;
        let entry = next.mode == FlightMode::Docked && self.state.mode != FlightMode::Docked;
        if let Some(state) = &mut navigation {
            let account = self
                .commander_account
                .as_mut()
                .ok_or(StepError::NonFiniteState)?;
            if !Self::has_bound_drive(*state, account) {
                return Err(StepError::NonFiniteState);
            }
            let arrival_debit = state
                .step(entry, next.tick, account.fuel_tenths())
                .map_err(|_| StepError::NonFiniteState)?;
            if entry || arrival_debit.is_some() {
                mission_epoch = self
                    .freight_session
                    .as_ref()
                    .map(|session| session.next_epoch())
                    .transpose()
                    .map_err(|_| StepError::TickOverflow)?;
            }
            if let Some(debit) = arrival_debit {
                let account = self
                    .commander_account
                    .as_ref()
                    .ok_or(StepError::NonFiniteState)?;
                let source = self.navigation.ok_or(StepError::NonFiniteState)?;
                let grade = self
                    .modular
                    .ledger(self, account)
                    .map_err(|_| StepError::NonFiniteState)?
                    .navigation_grade();
                if !self.jump_journal.has_room() {
                    return Err(StepError::TickOverflow);
                }
                completed_leg = Some(crate::navigation::CompletedJump::arrived(
                    source,
                    *state,
                    debit,
                    next.tick,
                    grade,
                    account
                        .revision()
                        .checked_add(1)
                        .ok_or(StepError::TickOverflow)?,
                    self.combat
                        .physical_tick()
                        .checked_add(1)
                        .ok_or(StepError::TickOverflow)?,
                ));
                planned_debit = Some(debit);
                // Source arrival places the station16000 ahead. This is its equivalent
                // station-local frame, not an earned dock or a proximity teleport.
                next.position = [0.0, 0.0, 16000.0];
                next.orientation = [0.0, 1.0, 0.0, 0.0];
                next.mode = FlightMode::Flight;
                next.motion_refusal = None;
            }
        }
        let mining = match (self.combat_identity(), navigation) {
            (Some(source), Some(nav)) => {
                self.mining
                    .prepare_departure(source, nav.snapshot(0).system, next.tick)
            }
            _ => self.mining.clone(),
        };
        let mut prepared = self.prepare_missile_step(next, navigation, &self.traffic)?;
        let mut traffic = self.prepare_traffic_step(next, navigation, &prepared)?;
        if let Some((traffic, _)) = &traffic {
            prepared.traffic = traffic.clone();
        }
        if let Some((traffic, _)) = &traffic {
            for hit in traffic
                .player_hits()
                .iter()
                .skip(self.traffic.player_hits().len())
            {
                prepared
                    .combat
                    .apply_bound_player_damage(&mut prepared.pulse, hit.front, 1500);
            }
        }
        self.clear_missing_missile_lock(
            &mut prepared.missiles,
            &prepared.traffic,
            prepared.practice,
            prepared.pulse.alive(),
        );
        let damage =
            self.damage
                .step(self.state.mode, next.mode, prepared.combat.physical_tick())?;
        let emp = self.emp.prepare_step(
            self,
            emp::Frame {
                next,
                navigation,
                traffic: &prepared.traffic,
                missiles: &prepared.missiles,
                pulse: prepared.pulse,
                combat: prepared.combat,
            },
        )?;
        prepared.traffic = emp.traffic.clone();
        prepared.pulse = emp.pulse;
        prepared.combat = emp.combat;
        if let Some((traffic, _)) = &mut traffic {
            *traffic = prepared.traffic.clone();
        }
        let navigation_trade =
            self.prepare_navigation_trade_step(navigation, planned_debit, next.tick)?;
        if let (Some(practice), Some(nav), Some(data)) =
            (&mut prepared.practice, navigation, &self.station_data)
        {
            practice.after_step(self.state.mode, next, nav, data);
        }
        self.check_step_combat_capacity(
            next,
            navigation,
            &prepared,
            traffic.as_ref(),
            (&damage, &mining, &emp.state),
            navigation_trade.as_ref(),
        )
        .map_err(|_| StepError::TickOverflow)?;
        if let Some(debit) = planned_debit {
            self.commander_account
                .as_mut()
                .ok_or(StepError::NonFiniteState)?
                .debit_fuel(debit)
                .map_err(|_| StepError::NonFiniteState)?;
        }
        // The receipt and the existing navigation, dock and wallet changes publish together.
        if let Some(leg) = completed_leg {
            self.jump_journal.append(leg);
        }
        // No fallible operation follows the first account mutation.
        if let Some((_, markets)) = traffic {
            self.station_markets = Some(markets);
        }
        if let Some(navigation_trade) = navigation_trade {
            self.navigation_trade = navigation_trade;
        }
        self.traffic = prepared.traffic;
        self.missiles = prepared.missiles;
        self.combat = prepared.combat;
        self.pulse = prepared.pulse;
        self.damage = damage;
        self.emp = emp.state;
        self.mining = mining;
        self.practice = prepared.practice;
        if let Some(account) = &mut self.commander_account {
            account.apply_ordinary_legal(prepared.legal);
        }
        self.state = next;
        self.navigation = navigation;
        if entry || completed_leg.is_some() {
            self.reconcile_navigation_route(entry);
        }
        self.refresh_deferred_renas_mule();
        self.observe_flight_reception(flight_reception_start);
        if let Some((session, epoch)) = self.freight_session.as_mut().zip(mission_epoch) {
            session.rebind_dock(navigation.and_then(NavigationState::dock), epoch);
        }
        Ok(())
    }
}

fn axis_rotation(axis: usize, angle: f32) -> [f32; 4] {
    let (sin, cos) = (angle * 0.5).sin_cos();
    let mut quaternion = [0.0, 0.0, 0.0, cos];
    quaternion[axis] = sin;
    quaternion
}

fn multiply(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

fn normalize(quaternion: [f32; 4]) -> Option<[f32; 4]> {
    let squared_length: f32 = quaternion.iter().map(|value| value * value).sum();
    if !squared_length.is_finite() || squared_length <= f32::MIN_POSITIVE {
        return None;
    }
    let inverse_length = squared_length.sqrt().recip();
    let result = quaternion.map(|value| value * inverse_length);
    result
        .iter()
        .all(|value| value.is_finite())
        .then_some(result)
}

fn forward([x, y, z, w]: [f32; 4]) -> [f32; 3] {
    [
        2.0 * (x * z + w * y),
        2.0 * (y * z - w * x),
        1.0 - 2.0 * (x * x + y * y),
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn close(actual: f32, expected: f32, tolerance: f32) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} != {expected}"
        );
    }

    #[test]
    fn starts_stationary_facing_positive_z() {
        let mut world = World::new();
        let initial = world.snapshot();
        assert_eq!(initial.position, [0.0, 0.0, -600.0]);
        assert_eq!(initial.orientation, [0.0, 0.0, 0.0, 1.0]);
        world.step(FlightInput::default()).unwrap();
        assert_eq!(world.snapshot().position, initial.position);
        assert_eq!(world.snapshot().tick, 1);
    }

    #[test]
    fn invalid_controls_reject_the_whole_step_without_mutation() {
        let mut world = World::new();
        world
            .step(FlightInput {
                throttle: 1.0,
                ..FlightInput::default()
            })
            .unwrap();
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.01, -1.01] {
            for field in 0..4 {
                let mut input = FlightInput::default();
                match field {
                    0 => input.pitch = invalid,
                    1 => input.yaw = invalid,
                    2 => input.roll = invalid,
                    _ => input.throttle = invalid,
                }
                let before = world.snapshot();
                let expected = if invalid.is_finite() {
                    StepError::InputOutOfRange
                } else {
                    StepError::NonFiniteInput
                };
                assert_eq!(world.step(input), Err(expected));
                assert_eq!(world.snapshot(), before);
            }
        }
    }

    #[test]
    fn repeated_input_trace_is_deterministic() {
        let mut first = World::new();
        let mut second = World::new();
        for tick in 0..3600 {
            let input = FlightInput {
                pitch: if tick % 120 < 60 { 0.5 } else { -0.5 },
                yaw: 0.25,
                roll: -0.75,
                throttle: 0.8,
                brake: tick % 180 > 150,
            };
            first.step(input).unwrap();
            second.step(input).unwrap();
            assert_eq!(first.snapshot(), second.snapshot());
        }
    }

    #[test]
    fn combined_rotation_stays_normalized_and_finite() {
        let mut world = World::new();
        for _ in 0..10000 {
            world
                .step(FlightInput {
                    pitch: 1.0,
                    yaw: -1.0,
                    roll: 1.0,
                    throttle: 1.0,
                    brake: false,
                })
                .unwrap();
            let snapshot = world.snapshot();
            let norm: f32 = snapshot.orientation.iter().map(|v| v * v).sum();
            close(norm, 1.0, 0.000002);
            assert!(snapshot.position.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn reference_acceleration_reaches_bounded_speed_and_idle_coasts() {
        let mut world = World::new();
        let accelerate = FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        };
        for _ in 0..60 {
            world.step(accelerate).unwrap();
        }
        close(world.snapshot().speed, 9.1, 0.00002);
        assert_eq!(world.snapshot().position[0], 0.0);
        assert_eq!(world.snapshot().position[1], 0.0);
        assert!(world.snapshot().position[2] > -600.0);
        for _ in 0..600 {
            world.step(accelerate).unwrap();
        }
        close(world.snapshot().speed, 34.3, 0.00001);
        let cruising = world.snapshot();
        world.step(FlightInput::default()).unwrap();
        assert_eq!(world.snapshot().speed, cruising.speed);
        assert!(world.snapshot().position[2] > cruising.position[2]);
    }

    #[test]
    fn brake_overrides_throttle_stops_and_cannot_reverse() {
        let mut world = World::new();
        for _ in 0..120 {
            world
                .step(FlightInput {
                    throttle: 1.0,
                    ..FlightInput::default()
                })
                .unwrap();
        }
        let before = world.snapshot().speed;
        let brake = FlightInput {
            throttle: 1.0,
            brake: true,
            ..FlightInput::default()
        };
        world.step(brake).unwrap();
        close(world.snapshot().speed, before - 9.1 / 60.0, 0.00001);
        for _ in 0..300 {
            world.step(brake).unwrap();
        }
        assert_eq!(world.snapshot().speed, 0.0);
        let stopped = world.snapshot().position;
        world.step(brake).unwrap();
        assert_eq!(world.snapshot().position, stopped);
        world
            .step(FlightInput {
                throttle: -1.0,
                ..FlightInput::default()
            })
            .unwrap();
        assert_eq!(world.snapshot().speed, 0.0);
    }

    #[test]
    fn local_pitch_and_yaw_steer_forward_with_declared_signs() {
        let mut pitch = World::new();
        let mut yaw = World::new();
        for _ in 0..60 {
            pitch
                .step(FlightInput {
                    pitch: 1.0,
                    throttle: 1.0,
                    ..FlightInput::default()
                })
                .unwrap();
            yaw.step(FlightInput {
                yaw: 1.0,
                throttle: 1.0,
                ..FlightInput::default()
            })
            .unwrap();
        }
        assert!(pitch.snapshot().position[1] > 0.0);
        assert!(yaw.snapshot().position[0] > 0.0);
        assert_eq!(pitch.snapshot().position[0], 0.0);
        assert_eq!(yaw.snapshot().position[1], 0.0);
        close(
            pitch.snapshot().orientation[0],
            -(0.7_f32 / 2.0).sin(),
            0.000002,
        );
        close(
            yaw.snapshot().orientation[1],
            (0.65_f32 / 2.0).sin(),
            0.000002,
        );
    }

    #[test]
    fn overflow_or_nonfinite_candidate_does_not_publish() {
        let mut world = World::new();
        world.state.tick = u64::MAX;
        let before = world.snapshot();
        assert_eq!(
            world.step(FlightInput::default()),
            Err(StepError::TickOverflow)
        );
        assert_eq!(world.snapshot(), before);
        world.state.tick = 0;
        world.state.orientation = [0.0; 4];
        let before = world.snapshot();
        assert_eq!(
            world.step(FlightInput::default()),
            Err(StepError::NonFiniteState)
        );
        assert_eq!(world.snapshot(), before);
    }
}

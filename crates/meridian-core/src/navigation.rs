//! Original G01 coordinates and stock SH025/Basic-drive travel rules from immutable a11.
//! Descriptive catalogue/route values never constitute docking or mission authority.
use core::fmt;
use std::collections::VecDeque;

const CATALOGUE: &[u8] = include_bytes!("../../../assets/legacy/g01.navigation");
pub const FUEL_CAPACITY: u16 = 130;
pub const STOCK_JUMP_REACH_LY: f64 = 9.0;
// HyperspaceDriveTier.basic.activationSeconds (not the old ship [game].spool value).
pub const BASIC_SPOOL_TICKS: u32 = 600;
pub const BASIC_MASS_LOCK_DISTANCE: f64 = 600.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemId {
    pub galaxy: u8,
    pub system: u16,
}
impl SystemId {
    pub const AVENOR: Self = Self {
        galaxy: 0,
        system: 7,
    };
    pub const RENAS: Self = Self {
        galaxy: 0,
        system: 185,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginalSystem {
    pub id: SystemId,
    pub name: &'static str,
    pub x_ly: f64,
    pub y_ly: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationError {
    ContentMismatch,
    InvalidCatalogue,
    UnknownSystem,
    InvalidReach,
    InvalidDistance,
    NoRoute,
    WrongMode,
    StaleCommand,
    AlreadyTravelling,
    MassLocked,
    OutOfRange,
    InsufficientFuel,
    SequenceExhausted,
    StationUnavailable,
    InvalidProfileIdentity,
    ServiceUnavailable,
}
impl fmt::Display for NavigationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ContentMismatch => "navigation bytes differ from the original catalogue",
            Self::InvalidCatalogue => "invalid original G01 catalogue",
            Self::UnknownSystem => "system is outside the original G01 catalogue",
            Self::InvalidReach => "jump reach must be finite and positive",
            Self::InvalidDistance => "jump distance must be finite and nonnegative",
            Self::NoRoute => "no route within the supplied reach",
            Self::WrongMode => "travel is unavailable in the current flight mode",
            Self::StaleCommand => "travel command does not match the current tick",
            Self::AlreadyTravelling => "a jump is already spooling",
            Self::MassLocked => "actual station geometry is within mass-lock distance",
            Self::OutOfRange => "target exceeds the stock fitted drive/navigation reach",
            Self::InsufficientFuel => "the current tank cannot fund this journey",
            Self::SequenceExhausted => "travel or actor sequence exhausted",
            Self::StationUnavailable => "original station content is unavailable",
            Self::ServiceUnavailable => "original station service content is unavailable",
            Self::InvalidProfileIdentity => {
                "fresh profile IDs must be nonzero, distinct and unexhausted"
            }
        })
    }
}
impl std::error::Error for NavigationError {}

#[derive(Debug)]
pub struct OriginalNavigation {
    systems: Vec<OriginalSystem>,
}
impl OriginalNavigation {
    pub fn bundled() -> Result<Self, NavigationError> {
        Self::from_bytes(CATALOGUE)
    }
    pub fn from_bytes(input: &[u8]) -> Result<Self, NavigationError> {
        if input != CATALOGUE {
            return Err(NavigationError::ContentMismatch);
        }
        let bytes = CATALOGUE;
        if bytes.get(..10) != Some(b"MNAV0001\x00\x01".as_slice()) {
            return Err(NavigationError::InvalidCatalogue);
        }
        let mut offset = 10usize;
        let mut systems = Vec::with_capacity(256);
        for expected in 0..256u16 {
            let row = bytes
                .get(offset..offset + 19)
                .ok_or(NavigationError::InvalidCatalogue)?;
            let id = u16::from_le_bytes([row[0], row[1]]);
            let x = f64::from_le_bytes([
                row[2], row[3], row[4], row[5], row[6], row[7], row[8], row[9],
            ]);
            let y = f64::from_le_bytes([
                row[10], row[11], row[12], row[13], row[14], row[15], row[16], row[17],
            ]);
            let length = usize::from(row[18]);
            offset += 19;
            let name = std::str::from_utf8(
                bytes
                    .get(offset..offset + length)
                    .ok_or(NavigationError::InvalidCatalogue)?,
            )
            .map_err(|_| NavigationError::InvalidCatalogue)?;
            offset += length;
            if id != expected || !x.is_finite() || !y.is_finite() || name.is_empty() {
                return Err(NavigationError::InvalidCatalogue);
            }
            systems.push(OriginalSystem {
                id: SystemId {
                    galaxy: 0,
                    system: id,
                },
                name,
                x_ly: x,
                y_ly: y,
            });
        }
        if offset != bytes.len() {
            return Err(NavigationError::InvalidCatalogue);
        }
        Ok(Self { systems })
    }
    pub fn system(&self, id: SystemId) -> Result<OriginalSystem, NavigationError> {
        if id.galaxy != 0 {
            return Err(NavigationError::UnknownSystem);
        }
        self.systems
            .get(usize::from(id.system))
            .copied()
            .ok_or(NavigationError::UnknownSystem)
    }
    /// OriginalWorldCatalogue.distanceLY: Cartesian coordinates, no classic anisotropic conversion.
    pub fn distance_ly(&self, from: SystemId, to: SystemId) -> Result<f64, NavigationError> {
        let a = self.system(from)?;
        let b = self.system(to)?;
        Ok((a.x_ly - b.x_ly).hypot(a.y_ly - b.y_ly))
    }
    /// Minimum hops in stable original-system order. Later legs explicitly assume refuelling.
    pub fn fewest_jumps(
        &self,
        from: SystemId,
        to: SystemId,
        first_reach: f64,
        full_tank_reach: f64,
    ) -> Result<Vec<SystemId>, NavigationError> {
        self.system(from)?;
        self.system(to)?;
        if !first_reach.is_finite()
            || first_reach <= 0.0
            || !full_tank_reach.is_finite()
            || full_tank_reach <= 0.0
        {
            return Err(NavigationError::InvalidReach);
        }
        let mut previous = [None; 256];
        let mut seen = [false; 256];
        let mut queue = VecDeque::new();
        queue.push_back(from.system);
        seen[usize::from(from.system)] = true;
        while let Some(current) = queue.pop_front() {
            if current == to.system {
                let mut path = vec![to];
                let mut node = to.system;
                while node != from.system {
                    node = previous[usize::from(node)].ok_or(NavigationError::NoRoute)?;
                    path.push(SystemId {
                        galaxy: 0,
                        system: node,
                    });
                }
                path.reverse();
                return Ok(path);
            }
            let reach = if current == from.system {
                first_reach
            } else {
                full_tank_reach
            };
            for next in 0..256u16 {
                if !seen[usize::from(next)]
                    && self.distance_ly(
                        SystemId {
                            galaxy: 0,
                            system: current,
                        },
                        SystemId {
                            galaxy: 0,
                            system: next,
                        },
                    )? <= reach
                {
                    seen[usize::from(next)] = true;
                    previous[usize::from(next)] = Some(current);
                    queue.push_back(next);
                }
            }
        }
        Err(NavigationError::NoRoute)
    }
}

/// FuelStorage.jumpFuelCost for the actual stock multiplier1. Positive ties round away from zero.
pub fn stock_jump_fuel_cost(distance_ly: f64) -> Result<u16, NavigationError> {
    let cost = (distance_ly * 10.0).round();
    if !distance_ly.is_finite()
        || distance_ly < 0.0
        || !cost.is_finite()
        || cost > f64::from(u16::MAX)
    {
        return Err(NavigationError::InvalidDistance);
    }
    Ok(cost as u16)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TravelProgress {
    pub destination: SystemId,
    pub remaining_ticks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationSnapshot {
    pub system: SystemId,
    pub fuel_units: u16,
    pub visit: u64,
    pub travel: Option<TravelProgress>,
}

/// Deterministic new-profile identity. The host supplies a unique seed per new commander.
/// Reusing a seed deliberately recreates the same replay, not a second independent profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommanderId(u64);
impl CommanderId {
    pub fn seed(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerShipId {
    owner: CommanderId,
    ordinal: u64,
}
impl PlayerShipId {
    pub fn owner(self) -> CommanderId {
        self.owner
    }
    pub fn ordinal(self) -> u64 {
        self.ordinal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockOrigin {
    InitialStart,
    PhysicalEntry,
}

/// A World-created binding, never reconstructed from a snapshot or caller-supplied location.
/// A retained value must be checked against the owning World's current context before use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockContext {
    owner: CommanderId,
    ship: PlayerShipId,
    system: SystemId,
    visit: u64,
    sequence: u64,
    tick: u64,
    origin: DockOrigin,
}
impl DockContext {
    pub fn owner(self) -> CommanderId {
        self.owner
    }
    pub fn ship(self) -> PlayerShipId {
        self.ship
    }
    pub fn system(self) -> SystemId {
        self.system
    }
    /// NativeMainStationDockReceipt uses actor entity1, distinct from SpaceBody index0.
    pub fn actor_entity(self) -> u64 {
        1
    }
    pub fn visit(self) -> u64 {
        self.visit
    }
    pub fn sequence(self) -> u64 {
        self.sequence
    }
    pub fn tick(self) -> u64 {
        self.tick
    }
    pub fn origin(self) -> DockOrigin {
        self.origin
    }
    pub fn station_class(self) -> &'static str {
        "main-orbital"
    }
}

// This is an actual starter inventory binding created with a fresh commander, not a label
// interpreted as installed equipment. Paid upgrades/storage are deliberately absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StarterBasicDrive {
    owner: CommanderId,
    ship: PlayerShipId,
    item_ordinal: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingJump {
    destination: SystemId,
    remaining_ticks: u32,
    fuel_debit: u16,
    source: SystemId,
    source_visit: u64,
    drive: StarterBasicDrive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NavigationState {
    owner: CommanderId,
    ship: PlayerShipId,
    drive: StarterBasicDrive,
    system: SystemId,
    visit: u64,
    dock_sequence: u64,
    dock: Option<DockContext>,
    pending: Option<PendingJump>,
}
impl NavigationState {
    pub(crate) fn new(seed: u64, hull: u64, drive_item: u64) -> Self {
        let owner = CommanderId(seed);
        let ship = PlayerShipId {
            owner,
            ordinal: hull,
        };
        let mut state = Self {
            owner,
            ship,
            drive: StarterBasicDrive {
                owner,
                ship,
                item_ordinal: drive_item,
            },
            system: SystemId::AVENOR,
            visit: 1,
            dock_sequence: 1,
            dock: None,
            pending: None,
        };
        state.dock = Some(state.context(0, DockOrigin::InitialStart));
        state
    }
    fn context(self, tick: u64, origin: DockOrigin) -> DockContext {
        DockContext {
            owner: self.owner,
            ship: self.ship,
            system: self.system,
            visit: self.visit,
            sequence: self.dock_sequence,
            tick,
            origin,
        }
    }
    pub(crate) fn snapshot(self, fuel: u16) -> NavigationSnapshot {
        NavigationSnapshot {
            system: self.system,
            fuel_units: fuel,
            visit: self.visit,
            travel: self.pending.map(|jump| TravelProgress {
                destination: jump.destination,
                remaining_ticks: jump.remaining_ticks,
            }),
        }
    }
    pub(crate) fn dock(self) -> Option<DockContext> {
        self.dock
    }
    pub(crate) fn owner(self) -> CommanderId {
        self.owner
    }
    pub(crate) fn ship(self) -> PlayerShipId {
        self.ship
    }
    pub(crate) fn drive_item_id(self) -> u64 {
        self.drive.item_ordinal
    }
    pub(crate) fn launch(&mut self) -> Result<(), NavigationError> {
        self.visit = self
            .visit
            .checked_add(1)
            .ok_or(NavigationError::SequenceExhausted)?;
        self.dock = None;
        self.pending = None;
        Ok(())
    }
    pub(crate) fn begin_jump(
        &mut self,
        catalogue: &OriginalNavigation,
        target: SystemId,
        station_distance: f64,
        fuel: u16,
    ) -> Result<(), NavigationError> {
        if self.pending.is_some() {
            return Err(NavigationError::AlreadyTravelling);
        }
        if self.drive.owner != self.owner || self.drive.ship != self.ship {
            return Err(NavigationError::WrongMode);
        }
        let distance = catalogue.distance_ly(self.system, target)?;
        if target == self.system {
            return Err(NavigationError::InvalidDistance);
        }
        if !station_distance.is_finite() || station_distance < BASIC_MASS_LOCK_DISTANCE {
            return Err(NavigationError::MassLocked);
        }
        if distance > STOCK_JUMP_REACH_LY {
            return Err(NavigationError::OutOfRange);
        }
        // Source admission uses unrounded current-fuel reach as well as rounded debit.
        let debit = stock_jump_fuel_cost(distance)?;
        if distance > f64::from(fuel) / 10.0 || debit > fuel {
            return Err(NavigationError::InsufficientFuel);
        }
        self.visit
            .checked_add(1)
            .ok_or(NavigationError::SequenceExhausted)?;
        self.pending = Some(PendingJump {
            destination: target,
            remaining_ticks: BASIC_SPOOL_TICKS,
            fuel_debit: debit,
            source: self.system,
            source_visit: self.visit,
            drive: self.drive,
        });
        Ok(())
    }
    /// Advance a candidate only. World publishes flight and this state together after success.
    /// Returns the once-only fuel debit for this tick's actual hyperspace transition.
    pub(crate) fn step(
        &mut self,
        physical_entry: bool,
        tick: u64,
        fuel: u16,
    ) -> Result<Option<u16>, NavigationError> {
        if physical_entry {
            self.dock_sequence = self
                .dock_sequence
                .checked_add(1)
                .ok_or(NavigationError::SequenceExhausted)?;
            self.dock = Some(self.context(tick, DockOrigin::PhysicalEntry));
            self.pending = None; // Game.dock cancels the countdown without charging fuel.
            return Ok(None);
        }
        let Some(mut jump) = self.pending else {
            return Ok(None);
        };
        if jump.source != self.system || jump.source_visit != self.visit || jump.drive != self.drive
        {
            return Err(NavigationError::WrongMode);
        }
        if jump.remaining_ticks > 1 {
            jump.remaining_ticks -= 1;
            self.pending = Some(jump);
            return Ok(None);
        }
        fuel.checked_sub(jump.fuel_debit)
            .ok_or(NavigationError::InsufficientFuel)?;
        let visit = self
            .visit
            .checked_add(1)
            .ok_or(NavigationError::SequenceExhausted)?;
        self.visit = visit;
        self.system = jump.destination;
        self.pending = None;
        self.dock = None;
        Ok(Some(jump.fuel_debit))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{FlightInput, FlightMode, StepError, World};

    fn navigation(world: &World) -> NavigationSnapshot {
        world.snapshot().navigation.unwrap()
    }

    fn launched(seed: u64) -> World {
        let mut world = World::new_commander(seed).unwrap();
        world.request_launch(0).unwrap();
        world.step(FlightInput::default()).unwrap();
        world
    }

    fn jump(world: &mut World, target: SystemId) {
        world.request_jump(target, world.snapshot().tick).unwrap();
        for _ in 0..BASIC_SPOOL_TICKS {
            world.step(FlightInput::default()).unwrap();
        }
    }

    fn approach(world: &mut World, stop_z: f32) {
        for _ in 0..9000 {
            let before = world.snapshot();
            if before.mode == FlightMode::Docked || before.position[2] <= stop_z {
                return;
            }
            let target_speed = if before.position[2] > 2500.0 {
                34.3
            } else {
                4.0
            };
            world
                .step(FlightInput {
                    throttle: ((target_speed - before.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                    ..FlightInput::default()
                })
                .unwrap();
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        panic!("public flight controls did not reach the approach target");
    }

    fn turn_inward(world: &mut World) {
        for _ in 0..80 {
            world
                .step(FlightInput {
                    brake: true,
                    ..FlightInput::default()
                })
                .unwrap();
        }
        for _ in 0..400 {
            let q = world.snapshot().orientation;
            let angle = 2.0 * q[1].atan2(q[3]);
            let remaining = std::f32::consts::PI - angle;
            if remaining.abs() < 1e-6 {
                return;
            }
            world
                .step(FlightInput {
                    yaw: (remaining / (0.65 / 60.0)).clamp(-1.0, 1.0),
                    ..FlightInput::default()
                })
                .unwrap();
        }
        panic!("public controls did not turn toward the actual port");
    }

    #[test]
    fn exact_g01_coordinates_route_and_stock_fuel_rules() {
        let catalogue = OriginalNavigation::bundled().unwrap();
        let a = catalogue.system(SystemId::AVENOR).unwrap();
        let r = catalogue.system(SystemId::RENAS).unwrap();
        assert_eq!((a.name, a.x_ly, a.y_ly), ("Avenor", 52.4955, 51.0662));
        assert_eq!((r.name, r.x_ly, r.y_ly), ("Renas", 54.3368, 48.2497));
        let distance = catalogue.distance_ly(a.id, r.id).unwrap();
        assert!((distance - 1.8413_f64.hypot(2.8165)).abs() < 1e-12);
        assert_eq!(stock_jump_fuel_cost(distance), Ok(34));
        assert_eq!(stock_jump_fuel_cost(0.05), Ok(1));
        assert_eq!(stock_jump_fuel_cost(0.0), Ok(0));
        for distance in [-1.0, f64::NAN, f64::INFINITY, 6554.0] {
            assert_eq!(
                stock_jump_fuel_cost(distance),
                Err(NavigationError::InvalidDistance)
            );
        }
        assert_eq!(
            catalogue.fewest_jumps(a.id, r.id, 9.0, 9.0).unwrap(),
            vec![a.id, r.id]
        );
        assert_eq!(
            catalogue.fewest_jumps(a.id, a.id, 9.0, 9.0).unwrap(),
            vec![a.id]
        );
        assert_eq!(
            catalogue.fewest_jumps(a.id, r.id, 0.00001, 9.0),
            Err(NavigationError::NoRoute)
        );
        for reach in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                catalogue.fewest_jumps(a.id, r.id, reach, 9.0),
                Err(NavigationError::InvalidReach)
            );
        }
        for id in [
            SystemId {
                galaxy: 1,
                system: 7,
            },
            SystemId {
                galaxy: 0,
                system: 256,
            },
        ] {
            assert_eq!(catalogue.system(id), Err(NavigationError::UnknownSystem));
        }
        let mut forged = CATALOGUE.to_vec();
        forged[10] ^= 1;
        assert!(matches!(
            OriginalNavigation::from_bytes(&forged),
            Err(NavigationError::ContentMismatch)
        ));
    }

    #[test]
    fn new_profile_binds_source_location_ship_and_initial_context_only() {
        for seed in [0, u64::MAX - 9, u64::MAX] {
            assert!(matches!(
                World::new_commander(seed),
                Err(NavigationError::InvalidProfileIdentity)
            ));
        }
        let mut world = World::new_commander(17).unwrap();
        assert_eq!(world.credits_tenths(), Some(7200));
        let context = world.current_dock_context().unwrap();
        assert_eq!(context.owner(), world.commander_id().unwrap());
        assert_eq!(context.owner().seed(), 17);
        assert_eq!(context.ship(), world.player_ship_id().unwrap());
        assert_eq!(context.ship().ordinal(), 18);
        assert_eq!(context.ship().owner(), context.owner());
        assert_eq!(context.system(), SystemId::AVENOR);
        assert_eq!(
            (context.actor_entity(), context.visit(), context.sequence()),
            (1, 1, 1)
        );
        assert_eq!(
            (context.station_class(), context.origin(), context.tick()),
            ("main-orbital", DockOrigin::InitialStart, 0)
        );
        assert_eq!(navigation(&world).fuel_units, FUEL_CAPACITY);
        assert!(world.dock_context_is_current(&context));
        let mut snapshot = world.snapshot();
        snapshot.navigation.as_mut().unwrap().system = SystemId::RENAS;
        assert_eq!(navigation(&world).system, SystemId::AVENOR);
        assert!(
            !World::new_commander(18)
                .unwrap()
                .dock_context_is_current(&context)
        );
        assert_eq!(
            world.request_jump(SystemId::RENAS, 0),
            Err(NavigationError::WrongMode)
        );
        world.request_launch(0).unwrap();
        assert_eq!(world.current_dock_context(), None);
        assert!(!world.dock_context_is_current(&context));
        assert_eq!(navigation(&world).visit, 2);
        for prototype in [World::new(), World::at_original_station().unwrap()] {
            assert_eq!(prototype.commander_id(), None);
            assert_eq!(prototype.player_ship_id(), None);
            assert_eq!(prototype.current_dock_context(), None);
            assert_eq!(prototype.snapshot().navigation, None);
        }
    }

    #[test]
    fn actual_jump_countdown_debits_once_and_arrives_in_flight_without_dock_authority() {
        let mut world = launched(31);
        let before = world.snapshot();
        assert_eq!(
            world.request_jump(SystemId::RENAS, 0),
            Err(NavigationError::StaleCommand)
        );
        assert_eq!(world.snapshot(), before);
        world.request_jump(SystemId::RENAS, before.tick).unwrap();
        let spooling = world.snapshot();
        assert_eq!(
            world.request_jump(SystemId::RENAS, before.tick),
            Err(NavigationError::AlreadyTravelling)
        );
        assert_eq!(world.snapshot(), spooling);
        assert_eq!(
            world.step(FlightInput {
                yaw: f32::NAN,
                ..FlightInput::default()
            }),
            Err(StepError::NonFiniteInput)
        );
        assert_eq!(world.snapshot(), spooling);
        for _ in 1..BASIC_SPOOL_TICKS {
            world.step(FlightInput::default()).unwrap();
        }
        let pending = navigation(&world);
        assert_eq!(
            (pending.system, pending.fuel_units),
            (SystemId::AVENOR, 130)
        );
        assert_eq!(pending.travel.unwrap().remaining_ticks, 1);
        world.step(FlightInput::default()).unwrap();
        let arrived = world.snapshot();
        assert_eq!(arrived.mode, FlightMode::Flight);
        assert_eq!(arrived.position, [0.0, 0.0, 16000.0]);
        assert_eq!(arrived.orientation, [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(arrived.speed, before.speed);
        assert_eq!(
            navigation(&world),
            NavigationSnapshot {
                system: SystemId::RENAS,
                fuel_units: 96,
                visit: 3,
                travel: None
            }
        );
        assert_eq!(world.current_dock_context(), None);
        world.step(FlightInput::default()).unwrap();
        assert_eq!(navigation(&world).fuel_units, 96);
        assert_eq!(world.credits_tenths(), Some(7200));
        assert!(world.snapshot().position[2] < 16000.0);
    }

    #[test]
    fn destination_context_requires_actual_public_flight_and_physical_entry() {
        let mut world = World::new_commander(41).unwrap();
        let source = world.current_dock_context().unwrap();
        world.request_launch(0).unwrap();
        world.step(FlightInput::default()).unwrap();
        jump(&mut world, SystemId::RENAS);
        assert_eq!(world.current_dock_context(), None);
        approach(&mut world, 0.0);
        assert_eq!(world.snapshot().mode, FlightMode::Docked);
        let destination = world.current_dock_context().unwrap();
        assert_eq!(destination.system(), SystemId::RENAS);
        assert_eq!(destination.origin(), DockOrigin::PhysicalEntry);
        assert_eq!(
            (destination.owner(), destination.ship()),
            (source.owner(), source.ship())
        );
        assert_eq!(
            (
                destination.actor_entity(),
                destination.visit(),
                destination.sequence()
            ),
            (1, 3, 2)
        );
        assert_eq!(destination.tick(), world.snapshot().tick);
        assert!(world.dock_context_is_current(&destination));
        assert!(!world.dock_context_is_current(&source));
        assert_eq!(navigation(&world).fuel_units, 96);
        world.request_launch(world.snapshot().tick).unwrap();
        assert!(!world.dock_context_is_current(&destination));
        assert_eq!(navigation(&world).visit, 4);
    }

    #[test]
    fn physical_docking_cancels_spool_before_fuel_debit() {
        let mut world = launched(51);
        turn_inward(&mut world);
        approach(&mut world, 700.0);
        assert!(world.snapshot().position[2] >= BASIC_MASS_LOCK_DISTANCE as f32);
        world
            .request_jump(SystemId::RENAS, world.snapshot().tick)
            .unwrap();
        let start = world.snapshot().tick;
        approach(&mut world, 0.0);
        assert!(world.snapshot().tick - start < u64::from(BASIC_SPOOL_TICKS));
        assert_eq!(world.snapshot().mode, FlightMode::Docked);
        assert_eq!(
            (
                navigation(&world).system,
                navigation(&world).fuel_units,
                navigation(&world).travel
            ),
            (SystemId::AVENOR, 130, None)
        );
        assert_eq!(
            world.current_dock_context().unwrap().origin(),
            DockOrigin::PhysicalEntry
        );
    }

    #[test]
    fn actual_station_mass_lock_range_unknown_target_and_tank_exhaustion_refuse_atomically() {
        let mut world = launched(61);
        let before = world.snapshot();
        assert_eq!(
            world.request_jump(SystemId::AVENOR, before.tick),
            Err(NavigationError::InvalidDistance)
        );
        assert_eq!(world.snapshot(), before);
        assert_eq!(
            world.request_jump(
                SystemId {
                    galaxy: 1,
                    system: 185
                },
                before.tick
            ),
            Err(NavigationError::UnknownSystem)
        );
        let catalogue = OriginalNavigation::bundled().unwrap();
        let far = catalogue
            .systems
            .iter()
            .find(|system| catalogue.distance_ly(SystemId::AVENOR, system.id).unwrap() > 9.0)
            .unwrap()
            .id;
        assert_eq!(
            world.request_jump(far, before.tick),
            Err(NavigationError::OutOfRange)
        );
        assert_eq!(world.snapshot(), before);
        turn_inward(&mut world);
        approach(&mut world, 590.0);
        let near = world.snapshot();
        assert_eq!(
            world.request_jump(SystemId::RENAS, near.tick),
            Err(NavigationError::MassLocked)
        );
        assert_eq!(world.snapshot(), near);
        // A second actual profile performs three charged journeys without free refuelling.
        let mut world = launched(62);
        for target in [SystemId::RENAS, SystemId::AVENOR, SystemId::RENAS] {
            jump(&mut world, target);
        }
        assert_eq!(navigation(&world).fuel_units, 28);
        let before = world.snapshot();
        assert_eq!(
            world.request_jump(SystemId::AVENOR, before.tick),
            Err(NavigationError::InsufficientFuel)
        );
        assert_eq!(world.snapshot(), before);
    }

    #[test]
    fn route_uses_first_leg_fuel_and_stable_minimum_hops_without_minting_refuels() {
        let catalogue = OriginalNavigation::bundled().unwrap();
        let target = SystemId {
            galaxy: 0,
            system: 0,
        };
        let route = catalogue
            .fewest_jumps(SystemId::AVENOR, target, 4.0, 9.0)
            .unwrap();
        assert!(route.len() > 2);
        assert_eq!(
            (route[0], *route.last().unwrap()),
            (SystemId::AVENOR, target)
        );
        for (index, leg) in route.windows(2).enumerate() {
            assert!(
                catalogue.distance_ly(leg[0], leg[1]).unwrap()
                    <= if index == 0 { 4.0 } else { 9.0 }
            );
        }
        assert_eq!(
            route,
            catalogue
                .fewest_jumps(SystemId::AVENOR, target, 4.0, 9.0)
                .unwrap()
        );
        let mut state = NavigationState::new(1, 2, 7);
        state.launch().unwrap();
        let before = state;
        assert_eq!(
            state.begin_jump(&catalogue, SystemId::RENAS, 700.0, 33),
            Err(NavigationError::InsufficientFuel)
        );
        assert_eq!(state, before);
    }
}

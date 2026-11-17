//! Finite, deterministic local traffic. Only World owns this mutable actor/economy state.
//! SH025 is the actual admitted hull; unadmitted populations receive no substitute body.
mod combat;
mod economy;
pub(crate) use economy::Flows;
mod motion;
mod persistence;
mod player_hits;
#[cfg(test)]
mod player_hits_tests;

use crate::docking::{Basis3, RigidPose, Vec3};
use crate::navigation::SystemId;
use crate::save::SaveError;
use crate::station::OriginalStationData;
use crate::station_services::{CommodityId, MarketSite, StationMarkets};
use std::sync::Arc;

pub const MAX_TRAFFIC_ACTORS: usize = 4;
pub const MAX_TRAFFIC_SHOTS: usize = 128;
const CAPITAL: u32 = 120_000;
pub(crate) const CONTENT_RULES:&[u8]=b"local-traffic-v1;SH025-trader-pirate;no-SH031-substitution;finite-capital120000;manifest-native1to12;local-birth480-pirate900;2systems2actors;maxshots128;pulse15-energy1-heat10-cool18;SH025-energy380-shield300;speed34.3-accel9.1-yaw.65;NPC-bank-and-player-custody-separate;historical-flow-world-ticks;waypoints-3500-2400x9000-0x9000-0x3500-doorMinus300;final-align;native-entry-before-after-receipt;no-respawn;npc-player-native-contact;exact-resource-events;source-resource-clock;";
const INVALID: SaveError = SaveError::InvalidState("traffic identity, resources or finite custody");
const SH025_ID: &str = "SH025";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Population {
    Trader,
    Pirate,
    Police,
    Miner,
    BountyHunter,
    Alien,
    AiConstruct,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopulationGate {
    ImplementedSourceUnqualified,
    RequiredHullUnavailable,
    ControllerAndEffectsPending,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulationScope {
    pub population: Population,
    pub gate: PopulationGate,
    pub required_source: &'static str,
}
pub const POPULATIONS: [PopulationScope; 7] = [
    PopulationScope {
        population: Population::Trader,
        gate: PopulationGate::ImplementedSourceUnqualified,
        required_source: SH025_ID,
    },
    PopulationScope {
        population: Population::Pirate,
        gate: PopulationGate::ImplementedSourceUnqualified,
        required_source: SH025_ID,
    },
    PopulationScope {
        population: Population::Police,
        gate: PopulationGate::RequiredHullUnavailable,
        required_source: "StationPoliceSource: SH031",
    },
    PopulationScope {
        population: Population::Miner,
        gate: PopulationGate::RequiredHullUnavailable,
        required_source: "qualified mining hull and actual work/cargo path",
    },
    PopulationScope {
        population: Population::BountyHunter,
        gate: PopulationGate::ControllerAndEffectsPending,
        required_source: "SH025 role is declared; hunter behaviour/settlement pending",
    },
    PopulationScope {
        population: Population::Alien,
        gate: PopulationGate::RequiredHullUnavailable,
        required_source: "qualified original alien hull and actor origin",
    },
    PopulationScope {
        population: Population::AiConstruct,
        gate: PopulationGate::RequiredHullUnavailable,
        required_source: "qualified original AI construct hull and actor origin",
    },
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcId {
    profile: u64,
    system: SystemId,
    serial: u8,
}
impl NpcId {
    pub fn profile(self) -> u64 {
        self.profile
    }
    pub fn system(self) -> SystemId {
        self.system
    }
    pub fn serial(self) -> u8 {
        self.serial
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcPhase {
    Departing,
    Outbound,
    ReturnAlign,
    Inbound,
    Approach,
    FinalAlign,
    Complete,
    Hunting,
    Retreating,
    Destroyed,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NpcSnapshot {
    pub id: NpcId,
    pub population: Population,
    pub hull_id: &'static str,
    pub pose: RigidPose,
    pub speed_units: f64,
    pub phase: NpcPhase,
    pub energy_hundredths: u16,
    pub front_shield_hundredths: u16,
    pub rear_shield_hundredths: u16,
    pub heat_fifths: u16,
    pub cooldown_ticks: u8,
    pub cash_tenths: u32,
    pub cargo: Option<(CommodityId, u8)>,
    pub wanted: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeamSource {
    Npc(NpcId),
    Player {
        owner: u64,
        hull: u64,
        item: u64,
        emission: u64,
    },
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficBeam {
    pub source: BeamSource,
    pub victim: NpcId,
    pub world_tick: u64,
    pub local_tick: u64,
    pub origin: Vec3,
    pub direction: Vec3,
    pub point: Vec3,
    pub victim_pose: RigidPose,
    pub front: bool,
}
/// A real NPC Pulse contact with the owned player hull. Descriptive, never command authority.
#[derive(Debug, Clone, PartialEq)]
pub struct NpcPlayerHit {
    pub attacker: NpcId,
    pub player_hull: u64,
    pub visit: u64,
    pub world_tick: u64,
    pub physical_tick: u64,
    pub local_tick: u64,
    pub source_pose: RigidPose,
    pub player_pose: RigidPose,
    pub point: Vec3,
    pub front: bool,
    peers: [Option<(NpcId, RigidPose)>; MAX_TRAFFIC_ACTORS],
    pub(crate) missiles: Arc<Vec<(crate::ProjectileId, RigidPose)>>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficContact {
    pub id: NpcId,
    pub distance: f64,
    pub point: Vec3,
}
#[derive(Debug, Clone, PartialEq)]
pub struct TrafficSnapshot {
    pub actors: Vec<NpcSnapshot>,
    pub last_beam: Option<TrafficBeam>,
    pub last_player_hit: Option<NpcPlayerHit>,
    pub verified_crimes: u16,
    pub police_required_hull: Option<&'static str>,
    pub populations: [PopulationScope; 7],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficError {
    InvalidState,
    ReceiptCapacity,
    SequenceExhausted,
}
impl core::fmt::Display for TrafficError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "traffic refused: {self:?}")
    }
}
impl std::error::Error for TrafficError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Resources {
    energy: u16,
    front: u16,
    rear: u16,
    heat: u16,
    cooldown: u8,
    shots: u16,
    incoming: u16,
    energy_regen: u64,
    front_regen: u64,
    rear_regen: u64,
    heat_cooled: u64,
    absorbed_front: u64,
    absorbed_rear: u64,
    hull_damage: u64,
    overkill: u64,
}
impl Resources {
    fn fresh() -> Self {
        Self {
            energy: 38_000,
            front: 30_000,
            rear: 30_000,
            ..Self::default()
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct BerthReceipt {
    before: Vec3,
    yaw: f64,
    speed: f64,
    angular: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Actor {
    id: NpcId,
    phase: NpcPhase,
    position: Vec3,
    yaw: f64,
    speed: f64,
    birth_clock: u64,
    born_world_tick: u64,
    sold_world_tick: Option<u64>,
    manifest: CommodityId,
    units: u8,
    wallet: u32,
    sold: bool,
    resources: Resources,
    resource_clock: u64,
    aggressor: Option<NpcId>,
    berth: Option<BerthReceipt>,
}
impl Actor {
    fn population(self) -> Population {
        if self.id.serial == 1 {
            Population::Trader
        } else {
            Population::Pirate
        }
    }
    fn solid(self) -> bool {
        self.phase != NpcPhase::Complete
    }
    fn airborne(self) -> bool {
        !matches!(self.phase, NpcPhase::Complete | NpcPhase::Destroyed)
    }
    fn pose(self) -> RigidPose {
        pose(self.position, self.yaw)
    }
    fn snapshot(self) -> NpcSnapshot {
        NpcSnapshot {
            id: self.id,
            population: self.population(),
            hull_id: SH025_ID,
            pose: self.pose(),
            speed_units: self.speed,
            phase: self.phase,
            energy_hundredths: self.resources.energy,
            front_shield_hundredths: self.resources.front,
            rear_shield_hundredths: self.resources.rear,
            heat_fifths: self.resources.heat,
            cooldown_ticks: self.resources.cooldown,
            cash_tenths: self.wallet,
            cargo: (self.id.serial == 1 && !self.sold && self.phase != NpcPhase::Destroyed)
                .then_some((self.manifest, self.units)),
            wanted: if self.id.serial == 2 { 16 } else { 0 },
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct TickContext<'a> {
    pub(crate) system: SystemId,
    pub(crate) data: &'a OriginalStationData,
    pub(crate) player_before: RigidPose,
    pub(crate) player_after: RigidPose,
    pub(crate) world_tick: u64,
    pub(crate) protected: bool,
    pub(crate) celestial_clear: &'a dyn Fn(Vec3, Vec3, f64) -> bool,
    pub(crate) beam_clear: &'a dyn Fn(Vec3, Vec3, f64) -> bool,
    pub(crate) player_target: Option<PlayerTarget>,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlayerTarget {
    pub(crate) hull: u64,
    pub(crate) visit: u64,
    pub(crate) physical_tick: u64,
    pub(crate) cargo: bool,
}
pub(crate) struct PlayerBeamInput {
    pub(crate) system: SystemId,
    pub(crate) source: BeamSource,
    pub(crate) origin: Vec3,
    pub(crate) direction: Vec3,
    pub(crate) contact: TrafficContact,
    pub(crate) world_tick: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct State {
    enabled: bool,
    seed: u64,
    clocks: [u64; 2],
    actors: [Option<Actor>; MAX_TRAFFIC_ACTORS],
    shots: Arc<Vec<TrafficBeam>>,
    player_hits: Arc<Vec<NpcPlayerHit>>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            enabled: false,
            seed: 0,
            clocks: [0; 2],
            actors: [None; MAX_TRAFFIC_ACTORS],
            shots: Arc::new(Vec::new()),
            player_hits: Arc::new(Vec::new()),
        }
    }
}
impl State {
    pub(crate) fn fresh(seed: u64) -> Self {
        Self {
            enabled: true,
            seed,
            ..Self::default()
        }
    }
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }
    pub(crate) fn snapshot(&self, system: SystemId) -> Option<TrafficSnapshot> {
        if !self.enabled {
            return None;
        }
        Some(TrafficSnapshot {
            actors: self
                .actors
                .iter()
                .flatten()
                .filter(|a| a.id.system == system)
                .map(|a| a.snapshot())
                .collect(),
            last_beam: self
                .shots
                .last()
                .copied()
                .filter(|s| s.victim.system == system),
            last_player_hit: self
                .player_hits
                .last()
                .cloned()
                .filter(|h| h.attacker.system == system),
            verified_crimes: self.shots.iter().filter(|s| s.victim.serial == 1).count() as u16,
            police_required_hull: self
                .shots
                .iter()
                .any(|s| s.victim.serial == 1 && s.victim.system == system)
                .then_some("SH031"),
            populations: POPULATIONS,
        })
    }
    pub(crate) fn beam(
        &self,
        data: &OriginalStationData,
        system: SystemId,
        origin: Vec3,
        direction: Vec3,
        range: f64,
    ) -> Option<TrafficContact> {
        self.beam_except(data, system, origin, direction, range, None)
    }
    fn beam_except(
        &self,
        data: &OriginalStationData,
        system: SystemId,
        origin: Vec3,
        direction: Vec3,
        range: f64,
        except: Option<NpcId>,
    ) -> Option<TrafficContact> {
        self.actors
            .iter()
            .flatten()
            .filter(|a| a.id.system == system && a.solid() && Some(a.id) != except)
            .filter_map(|a| {
                data.player_beam_distance(a.pose(), origin, direction, range)
                    .ok()
                    .flatten()
                    .map(|distance| TrafficContact {
                        id: a.id,
                        distance,
                        point: add(origin, scale(direction, distance)),
                    })
            })
            .min_by(|a, b| {
                a.distance
                    .total_cmp(&b.distance)
                    .then_with(|| a.id.serial.cmp(&b.id.serial))
            })
    }
    pub(crate) fn player_motion_clear(
        &self,
        system: SystemId,
        before: Vec3,
        after: Vec3,
        radius: f64,
    ) -> bool {
        self.actors
            .iter()
            .flatten()
            .filter(|a| a.id.system == system && a.solid())
            .all(|a| segment_distance(before, after, a.position) > radius * 2.0 + 1.0)
    }
    fn actor_index(&self, id: NpcId) -> Option<usize> {
        self.actors
            .iter()
            .position(|a| a.is_some_and(|a| a.id == id))
    }
}
fn system_index(system: SystemId) -> Option<usize> {
    match system {
        SystemId::AVENOR => Some(0),
        SystemId::RENAS => Some(1),
        _ => None,
    }
}
fn site(system: SystemId) -> Option<MarketSite> {
    match system {
        SystemId::AVENOR => Some(MarketSite::Avenor),
        SystemId::RENAS => Some(MarketSite::Renas),
        _ => None,
    }
}
fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
fn manifest(seed: u64, system: SystemId) -> (CommodityId, u8) {
    let choices = [
        CommodityId::Food,
        CommodityId::Textiles,
        CommodityId::Liquor,
        CommodityId::Luxuries,
        CommodityId::Computers,
        CommodityId::Machinery,
        CommodityId::Alloys,
        CommodityId::Furs,
        CommodityId::Minerals,
        CommodityId::Gold,
        CommodityId::Platinum,
    ];
    let word = mix(seed ^ u64::from(system.system) ^ 0x5452_4144_4552);
    (
        choices[(word % choices.len() as u64) as usize],
        1 + (word.rotate_left(17) % 12) as u8,
    )
}
fn pirate_present(seed: u64, system: SystemId) -> bool {
    mix(seed ^ u64::from(system.system) ^ 0x5049_5241_5445) % 4 == 0
}
fn pose(position: Vec3, yaw: f64) -> RigidPose {
    let (s, c) = yaw.sin_cos();
    RigidPose {
        position,
        orientation: Basis3 {
            x: Vec3::new(c, 0.0, -s),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(s, 0.0, c),
        },
    }
}
fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn scale(a: Vec3, s: f64) -> Vec3 {
    Vec3::new(a.x * s, a.y * s, a.z * s)
}
fn length(a: Vec3) -> f64 {
    a.dot(a).sqrt()
}
fn unit(a: Vec3) -> Option<Vec3> {
    let n = length(a);
    (n.is_finite() && n > 1e-9).then(|| scale(a, 1.0 / n))
}
fn finite(a: Vec3) -> bool {
    [a.x, a.y, a.z].into_iter().all(f64::is_finite)
}
fn segment_distance(a: Vec3, b: Vec3, p: Vec3) -> f64 {
    let d = sub(b, a);
    let t = if d.dot(d) > 0.0 {
        (sub(p, a).dot(d) / d.dot(d)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    length(sub(p, add(a, scale(d, t))))
}

#[cfg(test)]
mod tests;

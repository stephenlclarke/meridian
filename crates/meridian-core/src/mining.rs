//! Meridian's authored Avenor belt segment. The same mesh is used by work rays, collision and
//! renderer snapshots. All quantities use the existing market commodity's native unit.
use crate::docking::{RigidPose, Vec3};
use std::sync::Arc;
mod ledger;
mod ore;
mod persistence;
use crate::navigation::SystemId;
use crate::save::{Reader, SaveError, Writer};
use crate::station_services::CommodityId;
pub(crate) use ledger::MiningStroke;
pub(crate) use ore::Collection;
pub use ore::{OreId, OreSnapshot, SCOOP_DISTANCE, SCOOP_MAX_RELATIVE_SPEED};

pub const BELT_BODY_COUNT: usize = 12;
pub const MINING_RANGE: f64 = 6000.0;
pub const MINING_DAMAGE: u16 = 50;
pub const MAX_PENDING_ORE: usize = 256;
pub const BELT_NAME: &str = "Avenor ore belt";
pub const BELT_CENTRE: Vec3 = Vec3::new(4000.0, 0.0, 8000.0);
/// Closed outward-wound octahedron. Body snapshots supply its actual world-space vertices.
pub const ASTEROID_FACES: [[usize; 3]; 8] = [
    [0, 2, 4],
    [2, 1, 4],
    [1, 3, 4],
    [3, 0, 4],
    [2, 0, 5],
    [1, 2, 5],
    [3, 1, 5],
    [0, 3, 5],
];
const OFFSETS: [[i32; 3]; BELT_BODY_COUNT] = [
    [0, 0, -2000],
    [1000, 100, -1732],
    [1732, -100, -1000],
    [2000, 0, 0],
    [1732, 100, 1000],
    [1000, -100, 1732],
    [0, 0, 2000],
    [-1000, 100, 1732],
    [-1732, -100, 1000],
    [-2000, 0, 0],
    [-1732, 100, -1000],
    [-1000, -100, -1732],
];
const INITIAL_UNITS: [u8; BELT_BODY_COUNT] = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 1];
// Canonical f64 values of the frozen source units^(1/3), avoiding platform libm drift.
const SOURCE_CUBE_ROOTS: [f64; BELT_BODY_COUNT] = [
    1.0,
    1.7099759466766968,
    2.154434690031884,
    2.46621207433047,
    2.7144176165949063,
    2.924017738212866,
    3.1072325059538586,
    3.2710663101885897,
    3.4199518933533937,
    3.5568933044900626,
    3.6840314986403864,
    1.0,
];
const INVALID: SaveError = SaveError::InvalidState("mining body identity/depletion conservation");

/// A stable descriptive address, not a work capability or an ownership claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsteroidId {
    system: SystemId,
    ordinal: u8,
}
impl AsteroidId {
    pub fn system(self) -> SystemId {
        self.system
    }
    pub fn original_field(self) -> u16 {
        4 + u16::from(self.ordinal / 4)
    }
    pub fn original_rock(self) -> u16 {
        u16::from(self.ordinal % 4)
    }
    pub fn ordinal(self) -> u8 {
        self.ordinal
    }
    pub(crate) fn from_ordinal(ordinal: u8) -> Result<Self, SaveError> {
        if usize::from(ordinal) >= BELT_BODY_COUNT {
            return Err(INVALID);
        }
        Ok(Self {
            system: SystemId::AVENOR,
            ordinal,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeltDestination {
    pub name: &'static str,
    pub system: SystemId,
    /// Coordinates in the actual main-station local frame; never screen or chart pixels.
    pub position: Vec3,
}
pub(crate) fn belt_destination(system: SystemId) -> Option<BeltDestination> {
    (system == SystemId::AVENOR).then_some(BeltDestination {
        name: BELT_NAME,
        system,
        position: BELT_CENTRE,
    })
}
/// Canonical immutable inputs for the new-format content boundary. A build changing positions,
/// shape, material, reserve or work rules cannot silently reinterpret an existing body ledger.
pub(crate) fn content_bytes() -> Vec<u8> {
    let mut w = Writer::new();
    w.bytes
        .extend(b"meridian-avenor-belt-v2;legacy-a11fe169-mining50;energy1;heat10;cooldown18;openingHeat230;terminal-pending-ore;no-free-scoops;");
    w.u64(MINING_RANGE.to_bits());
    w.u16(MINING_DAMAGE);
    w.u16(MAX_PENDING_ORE as u16);
    for body in State::default().bodies() {
        w.u64(u64::from(body.id.system.galaxy));
        w.u64(u64::from(body.id.system.system));
        w.u8(body.id.ordinal);
        w.u8(body.commodity as u8);
        w.u8(body.initial_units);
        w.u64(body.energy.to_bits());
        for point in std::iter::once(body.position).chain(body.vertices) {
            for value in [point.x, point.y, point.z] {
                w.u64(value.to_bits());
            }
        }
    }
    for face in ASTEROID_FACES {
        for index in face {
            w.u8(index as u8);
        }
    }
    w.bytes
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AsteroidSnapshot {
    pub id: AsteroidId,
    pub position: Vec3,
    pub vertices: [Vec3; 6],
    pub commodity: CommodityId,
    pub initial_units: u8,
    pub remaining_units: u8,
    pub extracted_units: u8,
    pub energy: f64,
    pub remaining_energy: f64,
}
impl AsteroidSnapshot {
    pub fn bounding_radius(self) -> f64 {
        self.vertices[4].z - self.position.z
    }
    pub(crate) fn contains_point(self, point: Vec3) -> bool {
        (point.x - self.position.x).abs() / (self.vertices[0].x - self.position.x)
            + (point.y - self.position.y).abs() / (self.vertices[2].y - self.position.y)
            + (point.z - self.position.z).abs() / (self.vertices[4].z - self.position.z)
            <= 1.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MiningContact {
    pub asteroid: AsteroidId,
    pub distance: f64,
    pub point: Vec3,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MiningShot {
    pub contact: MiningContact,
    pub item_id: u64,
    pub commodity: CommodityId,
    /// Units released to pending ownership, never directly credited to player cargo.
    pub extracted_units: u8,
    pub remaining_energy: f64,
    pub tick: u64,
}
/// A one-account-revision work capability. Only the account/equipment integration inside core
/// can produce it, after checking actual paid or explicit maximum-owned equipment and fit.
/// No constructor, decoding, UI-selected item or caller-supplied availability flag exists.
#[derive(Debug)]
pub struct MiningToolAuthority {
    pub(crate) owner: u64,
    pub(crate) hull: u64,
    pub(crate) account_revision: u64,
    pub(crate) item_id: u64,
    pub(crate) epoch: u64,
    pub(crate) opening_emission: u64,
    pub(crate) scope: std::sync::Weak<()>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct BeltSnapshot {
    pub destination: BeltDestination,
    pub bodies: [AsteroidSnapshot; BELT_BODY_COUNT],
    pub distance_from_player: f64,
    pub front_contact: Option<MiningContact>,
    pub pending_ore: Vec<OreSnapshot>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningError {
    StaleCommand,
    Paused,
    NoCommander,
    NotInFlight,
    NoCompatibleFittedTool,
    NoScoops,
    NoEligibleOreBody,
    ProtectedActivity,
    ProofCapacity,
    NoTarget,
    Blocked,
    Depleted,
    Resources,
    HoldFull,
    InvalidState,
}
impl core::fmt::Display for MiningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "mining refused: {self:?}")
    }
}
impl std::error::Error for MiningError {}

/// Immutable body addresses and retained accepted work. Terminal ownership survives visits.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct State {
    enabled: bool,
    strokes: Arc<Vec<MiningStroke>>,
    expired: [Option<(u64, u64)>; BELT_BODY_COUNT],
    collected: Arc<Vec<Collection>>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            enabled: true,
            strokes: Arc::new(Vec::new()),
            expired: [None; BELT_BODY_COUNT],
            collected: Arc::new(Vec::new()),
        }
    }
}
impl State {
    pub(crate) fn legacy_absent() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }
    #[cfg(test)]
    pub(crate) fn work_count(&self) -> u64 {
        self.strokes.len() as u64
    }
    pub(crate) fn strokes(&self) -> &[MiningStroke] {
        &self.strokes
    }
    pub(crate) fn body(&self, ordinal: usize) -> Option<AsteroidSnapshot> {
        let offset = OFFSETS.get(ordinal)?;
        let position = add(
            BELT_CENTRE,
            Vec3::new(
                f64::from(offset[0]) * 0.5,
                f64::from(offset[1]),
                f64::from(offset[2]) * 0.5,
            ),
        );
        let size = 18.0 * SOURCE_CUBE_ROOTS[ordinal];
        let vertices = [
            Vec3::new(size, 0.0, 0.0),
            Vec3::new(-size, 0.0, 0.0),
            Vec3::new(0.0, size * 0.8, 0.0),
            Vec3::new(0.0, -size * 0.8, 0.0),
            Vec3::new(0.0, 0.0, size * 1.2),
            Vec3::new(0.0, 0.0, -size * 1.2),
        ]
        .map(|v| add(position, v));
        let field = 4 + ordinal as u64 / 4;
        let rock = ordinal as u64 % 4;
        let roll = source_mix((field << 32) | rock) % 100;
        let commodity = if roll < 64 {
            CommodityId::Minerals
        } else if roll < 89 {
            CommodityId::Alloys
        } else if roll < 97 {
            CommodityId::Gold
        } else {
            CommodityId::Platinum
        };
        let energy = 100.0 * SOURCE_CUBE_ROOTS[ordinal];
        let worked = self
            .strokes
            .iter()
            .filter(|h| usize::from(h.contact.asteroid.ordinal) == ordinal)
            .count();
        let remaining_energy = (energy - f64::from(MINING_DAMAGE) * worked as f64).max(0.0);
        Some(AsteroidSnapshot {
            id: AsteroidId {
                system: SystemId::AVENOR,
                ordinal: ordinal as u8,
            },
            position,
            vertices,
            commodity,
            initial_units: INITIAL_UNITS[ordinal],
            remaining_units: if remaining_energy > 0.0 {
                INITIAL_UNITS[ordinal]
            } else {
                0
            },
            extracted_units: if remaining_energy == 0.0 {
                INITIAL_UNITS[ordinal]
            } else {
                0
            },
            energy,
            remaining_energy,
        })
    }
    pub(crate) fn bodies(&self) -> [AsteroidSnapshot; BELT_BODY_COUNT] {
        // Every index is within the fixed authored table; no fallback body is generated.
        std::array::from_fn(|i| match self.body(i) {
            Some(body) => body,
            None => unreachable!("fixed body table index"),
        })
    }
    /// Conservative moving actor envelope against the same closed native triangles.
    pub(crate) fn segment_clear(&self, a: Vec3, b: Vec3, radius: f64) -> bool {
        if !self.enabled {
            return true;
        }
        if !finite(a) || !finite(b) || !radius.is_finite() || radius < 0.0 {
            return false;
        }
        let movement = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
        let half = Vec3::new(radius, radius, radius);
        self.bodies()
            .into_iter()
            .filter(|body| body.remaining_energy > 0.0)
            .all(|body| {
                !body.contains_point(a)
                    && !body.contains_point(b)
                    && !ASTEROID_FACES.into_iter().any(|face| {
                        crate::station::swept_box_hits_triangle(
                            a,
                            half,
                            movement,
                            [
                                body.vertices[face[0]],
                                body.vertices[face[1]],
                                body.vertices[face[2]],
                            ],
                        )
                    })
            })
    }
    pub(crate) fn beam(&self, origin: Vec3, direction: Vec3, range: f64) -> Option<MiningContact> {
        if !self.enabled
            || !finite(origin)
            || !finite(direction)
            || (direction.dot(direction) - 1.0).abs() > 1e-9
            || !range.is_finite()
            || range <= 0.0
        {
            return None;
        }
        self.bodies()
            .into_iter()
            .filter(|body| body.remaining_energy > 0.0)
            .filter_map(|body| {
                ASTEROID_FACES
                    .into_iter()
                    .filter_map(|face| {
                        crate::station::beam::asteroid_triangle_distance(
                            origin,
                            direction,
                            face.map(|index| body.vertices[index]),
                            range,
                        )
                    })
                    .min_by(f64::total_cmp)
                    .map(|distance| MiningContact {
                        asteroid: body.id,
                        distance,
                        point: add(origin, scale(direction, distance)),
                    })
            })
            .min_by(|a, b| {
                a.distance
                    .total_cmp(&b.distance)
                    .then_with(|| a.asteroid.ordinal.cmp(&b.asteroid.ordinal))
            })
    }
}
fn source_mix(salt: u64) -> u64 {
    let mut value = salt ^ 0x4245_4c54_4f52_4531 ^ 7_u64.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
fn finite(v: Vec3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}
fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn scale(a: Vec3, f: f64) -> Vec3 {
    Vec3::new(a.x * f, a.y * f, a.z * f)
}

#[cfg(test)]
mod tests;

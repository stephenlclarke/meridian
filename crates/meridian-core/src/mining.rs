//! Meridian's authored Avenor belt segment. The same mesh is used by work rays, collision and
//! renderer snapshots. All quantities use the existing market commodity's native unit.
use crate::docking::Vec3;
use crate::navigation::SystemId;
use crate::save::{Reader, SaveError, Writer};
use crate::station_services::CommodityId;

pub const BELT_BODY_COUNT: usize = 12;
pub const MINING_RANGE: f64 = 800.0;
pub const MINING_MAX_SPEED: f32 = 2.0;
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
const INITIAL_UNITS: [u8; BELT_BODY_COUNT] = [8, 6, 4, 5, 3, 7, 8, 6, 4, 5, 3, 7];
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
        .extend(b"meridian-avenor-belt-v1;energy1;heat10;cooldown18;openingHeat230;yield-native1;");
    w.u64(MINING_RANGE.to_bits());
    w.f32(MINING_MAX_SPEED);
    for body in State::default().bodies() {
        w.u64(u64::from(body.id.system.galaxy));
        w.u64(u64::from(body.id.system.system));
        w.u8(body.id.ordinal);
        w.u8(body.commodity as u8);
        w.u8(body.initial_units);
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
    pub extracted_units: u8,
    pub tick: u64,
}
/// A one-account-revision work capability. Only the account/equipment integration inside core
/// can produce it, after checking genuine paid ownership, mining compatibility and actual fit.
/// No constructor, decoding, UI-selected item or caller-supplied availability flag exists.
#[derive(Debug)]
pub struct MiningToolAuthority {
    pub(crate) owner: u64,
    pub(crate) hull: u64,
    pub(crate) account_revision: u64,
    pub(crate) item_id: u64,
    pub(crate) epoch: u64,
    pub(crate) scope: std::sync::Weak<()>,
    /// Core equipment owner's real read-only fitted mining-item lookup. This is not supplied
    /// by UI callers and cannot mutate World. Root joins the actual G3 getter when minting.
    pub(crate) current_item: fn(&crate::World) -> Option<u64>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct BeltSnapshot {
    pub destination: BeltDestination,
    pub bodies: [AsteroidSnapshot; BELT_BODY_COUNT],
    pub distance_from_player: f64,
    pub front_contact: Option<MiningContact>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningError {
    StaleCommand,
    Paused,
    NoCommander,
    NotInFlight,
    NoCompatibleFittedTool,
    TooFast,
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

/// Depleted bodies remain collision geometry. Only their finite extractable material changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct State {
    enabled: bool,
    remaining: [u8; BELT_BODY_COUNT],
    extracted: [u8; BELT_BODY_COUNT],
}
impl Default for State {
    fn default() -> Self {
        Self {
            enabled: true,
            remaining: INITIAL_UNITS,
            extracted: [0; BELT_BODY_COUNT],
        }
    }
}
impl State {
    pub(crate) fn legacy_absent() -> Self {
        Self {
            enabled: false,
            remaining: [0; BELT_BODY_COUNT],
            extracted: [0; BELT_BODY_COUNT],
        }
    }
    pub(crate) fn enabled(self) -> bool {
        self.enabled
    }
    pub(crate) fn extracted_quantities(self) -> [u64; 16] {
        let mut quantities = [0; 16];
        for body in self.bodies() {
            quantities[body.commodity.index() - 1] += u64::from(body.extracted_units);
        }
        quantities
    }
    pub(crate) fn work_count(self) -> u64 {
        self.extracted.into_iter().map(u64::from).sum()
    }
    pub(crate) fn body(self, ordinal: usize) -> Option<AsteroidSnapshot> {
        let offset = OFFSETS.get(ordinal)?;
        let position = add(
            BELT_CENTRE,
            Vec3::new(
                f64::from(offset[0]),
                f64::from(offset[1]),
                f64::from(offset[2]),
            ),
        );
        let size = 60.0 + ordinal as f64 * 2.0;
        let vertices = [
            Vec3::new(size, 0.0, 0.0),
            Vec3::new(-size, 0.0, 0.0),
            Vec3::new(0.0, size * 0.8, 0.0),
            Vec3::new(0.0, -size * 0.8, 0.0),
            Vec3::new(0.0, 0.0, size * 1.2),
            Vec3::new(0.0, 0.0, -size * 1.2),
        ]
        .map(|v| add(position, v));
        let commodity = match ordinal % 6 {
            2 => CommodityId::Alloys,
            4 => CommodityId::Gold,
            5 => CommodityId::Platinum,
            _ => CommodityId::Minerals,
        };
        Some(AsteroidSnapshot {
            id: AsteroidId {
                system: SystemId::AVENOR,
                ordinal: ordinal as u8,
            },
            position,
            vertices,
            commodity,
            initial_units: INITIAL_UNITS[ordinal],
            remaining_units: self.remaining[ordinal],
            extracted_units: self.extracted[ordinal],
        })
    }
    pub(crate) fn bodies(self) -> [AsteroidSnapshot; BELT_BODY_COUNT] {
        // Every index is within the fixed authored table; no fallback body is generated.
        std::array::from_fn(|i| match self.body(i) {
            Some(body) => body,
            None => unreachable!("fixed body table index"),
        })
    }
    pub(crate) fn prepare_extract(
        self,
        id: AsteroidId,
    ) -> Result<(Self, CommodityId), MiningError> {
        if !self.enabled {
            return Err(MiningError::NoTarget);
        }
        self.validate().map_err(|_| MiningError::InvalidState)?;
        let body = self
            .body(usize::from(id.ordinal))
            .filter(|b| b.id == id)
            .ok_or(MiningError::InvalidState)?;
        if body.remaining_units == 0 {
            return Err(MiningError::Depleted);
        }
        let mut candidate = self;
        candidate.remaining[usize::from(id.ordinal)] -= 1;
        candidate.extracted[usize::from(id.ordinal)] += 1;
        Ok((candidate, body.commodity))
    }
    pub(crate) fn encode(self, w: &mut Writer) {
        w.bool(self.enabled);
        for (remaining, extracted) in self.remaining.into_iter().zip(self.extracted) {
            w.u8(remaining);
            w.u8(extracted);
        }
    }
    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let mut value = Self {
            enabled: r.bool()?,
            ..Self::default()
        };
        for i in 0..BELT_BODY_COUNT {
            value.remaining[i] = r.u8()?;
            value.extracted[i] = r.u8()?;
        }
        value.validate()?;
        Ok(value)
    }
    pub(crate) fn validate(self) -> Result<(), SaveError> {
        if !self.enabled {
            return if self.remaining == [0; BELT_BODY_COUNT]
                && self.extracted == [0; BELT_BODY_COUNT]
            {
                Ok(())
            } else {
                Err(INVALID)
            };
        }
        for (i, initial) in INITIAL_UNITS.into_iter().enumerate() {
            if u16::from(self.remaining[i]) + u16::from(self.extracted[i]) != u16::from(initial) {
                return Err(INVALID);
            }
        }
        Ok(())
    }
    pub(crate) fn beam(self, origin: Vec3, direction: Vec3, range: f64) -> Option<MiningContact> {
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
            .filter_map(|body| {
                ASTEROID_FACES
                    .into_iter()
                    .filter_map(|face| {
                        crate::station::beam::triangle_distance(
                            origin,
                            direction,
                            body.vertices[face[0]],
                            body.vertices[face[1]],
                            body.vertices[face[2]],
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

//! Finite released ore identities. No cargo credit or scoop installation is inferred.
use super::{
    AsteroidId, CommodityId, INVALID, MiningError, Reader, RigidPose, SaveError, State, Vec3,
    Writer, add, scale,
};
use crate::{CombatIdentity, station_services::CargoIntake};
use std::sync::Arc;

pub const SCOOP_DISTANCE: f64 = 85.0;
pub const SCOOP_MAX_RELATIVE_SPEED: f64 = 15.0;
const MAX_COLLECTIONS: usize = 277;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OreId {
    asteroid: AsteroidId,
    fragment: u8,
}
impl OreId {
    pub fn asteroid(self) -> AsteroidId {
        self.asteroid
    }
    pub fn fragment(self) -> u8 {
        self.fragment
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OreSnapshot {
    pub id: OreId,
    pub commodity: CommodityId,
    pub quantity: u8,
    pub position: Vec3,
    pub velocity: Vec3,
    pub released_tick: u64,
    pub released_physical_tick: u64,
}
impl OreSnapshot {
    /// World vertices shared by the Meridian fragment mesh and automatic scoop checks.
    pub fn vertices(self) -> [Vec3; 6] {
        [
            Vec3::new(7.0, 1.0, -2.0),
            Vec3::new(-5.0, -1.0, 2.0),
            Vec3::new(1.0, 10.0, 1.0),
            Vec3::new(-2.0, -6.0, -1.0),
            Vec3::new(2.0, 3.0, 6.0),
            Vec3::new(-1.0, 0.0, -8.0),
        ]
        .map(|offset| add(self.position, offset))
    }
    pub(crate) fn scoop_admissible(self, pose: RigidPose, speed: f32, automatic: bool) -> bool {
        let displacement = |p: Vec3| {
            Vec3::new(
                p.x - pose.position.x,
                p.y - pose.position.y,
                p.z - pose.position.z,
            )
        };
        let centre = displacement(self.position);
        if !pose.orientation.is_valid_rotation()
            || !super::finite(pose.position)
            || !speed.is_finite()
            || !(0.0..=crate::flight::MAX_SPEED).contains(&speed)
            || centre.dot(centre) >= SCOOP_DISTANCE * SCOOP_DISTANCE
            || centre.dot(pose.orientation.y) >= 0.0
            || centre.dot(pose.orientation.z) <= 0.0
        {
            return false;
        }
        if !automatic {
            return true;
        }
        let ship_speed = f64::from(speed * crate::station::DISTANCE_PER_SPEED_UNIT);
        let relative = add(self.velocity, scale(pose.orientation.z, -ship_speed));
        relative.dot(relative) <= SCOOP_MAX_RELATIVE_SPEED * SCOOP_MAX_RELATIVE_SPEED
            && self.vertices().into_iter().all(|p| {
                let point = displacement(p);
                point.dot(pose.orientation.y) < 0.0
                    && point.dot(pose.orientation.z) > 0.0
                    && point.dot(point) < SCOOP_DISTANCE * SCOOP_DISTANCE
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Collection {
    pub(crate) id: OreId,
    pub(crate) owner: CombatIdentity,
    pub(crate) item: u64,
    pub(crate) revision: u64,
    pub(crate) world_tick: u64,
    pub(crate) physical_tick: u64,
    pub(crate) pose: RigidPose,
    pub(crate) speed: f32,
    pub(crate) automatic: bool,
}
impl State {
    pub(crate) fn pending_count(&self) -> usize {
        self.bodies()
            .into_iter()
            .filter(|b| self.expired[usize::from(b.id.ordinal)].is_none())
            .map(|b| usize::from(b.extracted_units))
            .sum::<usize>()
            .saturating_sub(
                self.collected
                    .iter()
                    .filter(|c| self.expired[usize::from(c.id.asteroid.ordinal)].is_none())
                    .count(),
            )
    }
    pub(crate) fn pending_ore(&self, physical_tick: u64) -> Vec<OreSnapshot> {
        let mut ore = Vec::with_capacity(self.pending_count());
        for body in self.bodies() {
            let slot = usize::from(body.id.ordinal);
            if self.expired[slot].is_some() {
                continue;
            }
            if self.terminal_stroke(slot).is_none() {
                continue;
            }
            for fragment in 0..body.initial_units {
                let id = OreId {
                    asteroid: body.id,
                    fragment,
                };
                if self.collected.iter().any(|c| c.id == id) {
                    continue;
                }
                if let Some(snapshot) = self.fragment_at(id, physical_tick) {
                    ore.push(snapshot);
                }
            }
        }
        ore
    }
    fn fragment_at(&self, id: OreId, physical_tick: u64) -> Option<OreSnapshot> {
        let body = self.body(usize::from(id.asteroid.ordinal))?;
        let release = self.terminal_stroke(usize::from(id.asteroid.ordinal))?;
        if body.id != id.asteroid
            || id.fragment >= body.initial_units
            || physical_tick < release.physical_tick
        {
            return None;
        }
        let elapsed = (physical_tick - release.physical_tick) as f64 / 60.0;
        let columns = body.initial_units.min(5);
        let rows = body.initial_units / columns;
        let fragment = id.fragment;
        // Original MiningLedger.yield grid: cargo fallback radius48 => spacing112.
        let center = Vec3::new(
            f64::from(columns - 1) * 6.0,
            f64::from(rows - 1) * 6.0,
            10.0,
        );
        let offset = Vec3::new(
            (f64::from(fragment % columns) - f64::from(columns - 1) / 2.0) * 112.0,
            (f64::from(fragment / columns) - f64::from(rows - 1) / 2.0) * 112.0,
            0.0,
        );
        let velocity = Vec3::new(f64::from(fragment % 5), 2.0, 3.0);
        Some(OreSnapshot {
            id,
            commodity: body.commodity,
            quantity: 1,
            position: add(
                add(body.position, add(center, offset)),
                scale(velocity, elapsed),
            ),
            velocity,
            released_tick: release.world_tick,
            released_physical_tick: release.physical_tick,
        })
    }
    pub(crate) fn collections(&self) -> &[Collection] {
        &self.collected
    }
    pub(crate) fn cargo_intakes(&self) -> impl Iterator<Item = CargoIntake> + '_ {
        self.collected.iter().filter_map(|receipt| {
            self.body(usize::from(receipt.id.asteroid.ordinal))
                .map(|body| CargoIntake {
                    revision: receipt.revision,
                    commodity: body.commodity,
                })
        })
    }
    pub(crate) fn prepare_collection(&self, collection: Collection) -> Result<Self, MiningError> {
        let mut candidate = self.clone();
        Arc::make_mut(&mut candidate.collected).push(collection);
        candidate
            .validate_collections()
            .map_err(|_| MiningError::InvalidState)?;
        Ok(candidate)
    }
    pub(super) fn validate_collections(&self) -> Result<(), SaveError> {
        if self.collected.len() > MAX_COLLECTIONS {
            return Err(INVALID);
        }
        let mut previous: Option<&Collection> = None;
        for (index, receipt) in self.collected.iter().enumerate() {
            let terminal = self
                .terminal_stroke(usize::from(receipt.id.asteroid.ordinal))
                .ok_or(INVALID)?;
            let ore = self
                .fragment_at(receipt.id, receipt.physical_tick)
                .ok_or(INVALID)?;
            if receipt.owner.owner != terminal.owner.owner
                || receipt.owner.hull != terminal.owner.hull
                || receipt.owner.system != terminal.owner.system
                || receipt.owner.visit < terminal.owner.visit
                || receipt.item == 0
                || receipt.revision <= terminal.account_revision
                || receipt.world_tick < terminal.world_tick
                || receipt.physical_tick > receipt.world_tick
                || receipt.physical_tick - terminal.physical_tick
                    > receipt.world_tick - terminal.world_tick
                || self.expired[usize::from(receipt.id.asteroid.ordinal)]
                    .is_some_and(|(tick, _)| receipt.world_tick >= tick)
                || self.collected[..index]
                    .iter()
                    .any(|old| old.id == receipt.id)
                || previous.is_some_and(|old| {
                    old.revision >= receipt.revision
                        || old.world_tick > receipt.world_tick
                        || old.physical_tick > receipt.physical_tick
                })
                || !ore.scoop_admissible(receipt.pose, receipt.speed, receipt.automatic)
            {
                return Err(INVALID);
            }
            previous = Some(receipt);
        }
        Ok(())
    }
    pub(crate) fn encode_collections(&self, w: &mut Writer) {
        w.u16(self.collected.len() as u16);
        for c in self.collected.iter() {
            w.u8(c.id.asteroid.ordinal);
            w.u8(c.id.fragment);
            for value in [
                c.owner.owner,
                c.owner.hull,
                c.owner.visit,
                c.item,
                c.revision,
                c.world_tick,
                c.physical_tick,
            ] {
                w.u64(value);
            }
            super::persistence::rigid(w, c.pose);
            w.f32(c.speed);
            w.bool(c.automatic);
        }
    }
    pub(crate) fn decode_collections(&mut self, r: &mut Reader<'_>) -> Result<(), SaveError> {
        let count = usize::from(r.u16()?);
        if count == 0 || count > MAX_COLLECTIONS {
            return Err(INVALID);
        }
        let mut receipts = Vec::with_capacity(count);
        for _ in 0..count {
            let id = OreId {
                asteroid: AsteroidId::from_ordinal(r.u8()?)?,
                fragment: r.u8()?,
            };
            let owner = CombatIdentity {
                owner: r.u64()?,
                hull: r.u64()?,
                visit: r.u64()?,
                system: id.asteroid.system(),
            };
            receipts.push(Collection {
                id,
                owner,
                item: r.u64()?,
                revision: r.u64()?,
                world_tick: r.u64()?,
                physical_tick: r.u64()?,
                pose: super::persistence::read_rigid(r)?,
                speed: r.f32()?,
                automatic: r.bool()?,
            });
        }
        self.collected = Arc::new(receipts);
        self.validate()
    }
}

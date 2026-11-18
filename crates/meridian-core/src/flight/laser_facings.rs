//! SH025's four admitted local laser directions. No caller supplies a ray or mount offset.
use super::{ModuleSlot, OrdinaryWeapon, World};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::save::SaveError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaserFacing {
    Front,
    Left,
    Right,
    Rear,
}
impl LaserFacing {
    pub const ALL: [Self; 4] = [Self::Front, Self::Left, Self::Right, Self::Rear];
    pub fn slot(self) -> ModuleSlot {
        match self {
            Self::Front => ModuleSlot::FrontLaser,
            Self::Left => ModuleSlot::LeftLaser,
            Self::Right => ModuleSlot::RightLaser,
            Self::Rear => ModuleSlot::RearLaser,
        }
    }
    pub(super) fn from_slot(slot: ModuleSlot) -> Option<Self> {
        Self::ALL.into_iter().find(|facing| facing.slot() == slot)
    }
    pub(super) fn tag(self) -> u8 {
        match self {
            Self::Front => 0,
            Self::Left => 1,
            Self::Right => 2,
            Self::Rear => 3,
        }
    }
    pub(super) fn from_tag(tag: u8) -> Result<Self, SaveError> {
        match tag {
            0 => Ok(Self::Front),
            1 => Ok(Self::Left),
            2 => Ok(Self::Right),
            3 => Ok(Self::Rear),
            _ => Err(SaveError::InvalidEncoding),
        }
    }
    pub(super) fn ray(self, body: RigidPose) -> RigidPose {
        let neg = |v: Vec3| Vec3::new(-v.x, -v.y, -v.z);
        let b = body.orientation;
        let (x, z) = match self {
            Self::Front => (b.x, b.z),
            Self::Left => (b.z, neg(b.x)),
            Self::Right => (neg(b.z), b.x),
            Self::Rear => (neg(b.x), neg(b.z)),
        };
        RigidPose {
            position: body.position,
            orientation: Basis3 { x, y: b.y, z },
        }
    }
}

/// Immutable command-time body/facing/clock evidence, never accepted as firing authority.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MountedLaserEmission {
    pub body_pose: RigidPose,
    pub facing: LaserFacing,
    pub physical_tick: u64,
}
impl MountedLaserEmission {
    pub(crate) fn matches_ray(self, origin: Vec3, direction: Vec3) -> bool {
        self.body_pose.orientation.is_valid_rotation()
            && [
                self.body_pose.position.x,
                self.body_pose.position.y,
                self.body_pose.position.z,
            ]
            .into_iter()
            .all(f64::is_finite)
            && origin == self.body_pose.position
            && direction == self.facing.ray(self.body_pose).orientation.z
    }
    pub(crate) fn encode(self, w: &mut crate::save::Writer) {
        w.u64(self.physical_tick);
        w.u8(self.facing.tag());
        for v in [
            self.body_pose.position,
            self.body_pose.orientation.x,
            self.body_pose.orientation.y,
            self.body_pose.orientation.z,
        ] {
            w.u64(v.x.to_bits());
            w.u64(v.y.to_bits());
            w.u64(v.z.to_bits());
        }
    }
    pub(crate) fn decode(r: &mut crate::save::Reader<'_>) -> Result<Self, SaveError> {
        let physical_tick = r.u64()?;
        let facing = LaserFacing::from_tag(r.u8()?)?;
        let vectors = r.array(|r| {
            Ok(Vec3::new(
                f64::from_bits(r.u64()?),
                f64::from_bits(r.u64()?),
                f64::from_bits(r.u64()?),
            ))
        })?;
        let [position, x, y, z]: [Vec3; 4] = vectors;
        let result = Self {
            physical_tick,
            facing,
            body_pose: RigidPose {
                position,
                orientation: Basis3 { x, y, z },
            },
        };
        if !result.matches_ray(position, facing.ray(result.body_pose).orientation.z) {
            return Err(SaveError::InvalidState("mounted laser body pose"));
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FittedLaserSnapshot {
    pub facing: LaserFacing,
    pub fitted_item_id: u64,
    pub weapon: OrdinaryWeapon,
    pub damage_hundredths: u16,
    pub range: u16,
    pub cooldown_ticks_per_shot: u8,
}
impl World {
    pub(super) fn mounted_laser_emission(
        &self,
        facing: LaserFacing,
    ) -> Result<MountedLaserEmission, super::FireError> {
        Ok(MountedLaserEmission {
            body_pose: super::flight_pose(self.state.position, self.state.orientation)
                .ok_or(super::FireError::InvalidGeometry)?,
            facing,
            physical_tick: self.combat.physical_tick(),
        })
    }
    pub(super) fn validate_mounted_laser_emission(
        &self,
        emission: MountedLaserEmission,
        item: u64,
        weapon: OrdinaryWeapon,
        tick: u64,
        origin: Vec3,
        direction: Vec3,
    ) -> Result<(), SaveError> {
        if !emission.matches_ray(origin, direction)
            || self.laser_profile_at_tick(item, tick, emission.facing) != Some(weapon)
            || emission.physical_tick > self.combat.physical_tick()
            || emission.physical_tick > tick
            || tick > self.state.tick
            || (tick == self.state.tick
                && (super::flight_pose(self.state.position, self.state.orientation)
                    != Some(emission.body_pose)
                    || emission.physical_tick != self.combat.physical_tick()))
        {
            return Err(SaveError::InvalidState("mounted laser emission custody"));
        }
        Ok(())
    }
    pub fn selected_laser_facing(&self) -> LaserFacing {
        match self.flight_view() {
            super::FlightView::Front => LaserFacing::Front,
            super::FlightView::Left => LaserFacing::Left,
            super::FlightView::Right => LaserFacing::Right,
            super::FlightView::Rear => LaserFacing::Rear,
        }
    }
    pub fn selected_laser_snapshot(&self) -> Option<FittedLaserSnapshot> {
        self.laser_snapshot(self.selected_laser_facing())
    }
    pub fn request_fire_selected_laser(
        &mut self,
        expected_tick: u64,
    ) -> Result<super::PulseShot, super::FireError> {
        self.request_fire_laser(self.selected_laser_facing(), expected_tick)
    }
    pub fn laser_snapshot(&self, facing: LaserFacing) -> Option<FittedLaserSnapshot> {
        let (fitted_item_id, weapon) = self.fitted_laser(facing)?;
        Some(FittedLaserSnapshot {
            facing,
            fitted_item_id,
            weapon,
            damage_hundredths: weapon.damage_hundredths(),
            range: 6000,
            cooldown_ticks_per_shot: weapon.cooldown_ticks(),
        })
    }
    pub(super) fn fitted_laser(&self, facing: LaserFacing) -> Option<(u64, OrdinaryWeapon)> {
        let item = self
            .modular_equipment_snapshot()?
            .items
            .into_iter()
            .find(|item| item.slot == Some(facing.slot()))?;
        Some((item.id, OrdinaryWeapon::from_module(item.kind)?))
    }
    pub(super) fn fitted_laser_item(&self, id: u64) -> Option<(LaserFacing, OrdinaryWeapon)> {
        LaserFacing::ALL.into_iter().find_map(|facing| {
            let (item, weapon) = self.fitted_laser(facing)?;
            (item == id).then_some((facing, weapon))
        })
    }
}

#[cfg(test)]
mod tests;

//! Immutable a11fe169 project EMP balance; these are project values, not recovered designer data.
//! Electrical amounts use 1/3000 of a native energy unit. All published grade rates and
//! the 80 percent transfer are exact at 60 Hz; geometry attenuation rounds drain down.
use super::{EmpAxis, EmpError};
pub(super) const SCALE: u32 = 3000;
pub(super) const MAIN_CAPACITY: u32 = 380 * SCALE;
pub(super) const PULSE_COST: u32 = 8 * SCALE;
pub(super) const WARMUP_TICKS: u64 = 600;
pub(super) const RECOVERY_TICKS: u64 = 1800;
pub(super) const HEAT_FIFTHS: u16 = 90;
pub(super) const COOLDOWN_TICKS: u8 = 90;
pub(super) const BASE_PRICE: u32 = 30_000;
pub(super) const BASE_TECHNOLOGY: u8 = 9;
pub(super) const MAX_GRADE: u8 = 3;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EmpGrades {
    pub power: u8,
    pub capacity: u8,
    pub recharge: u8,
}
impl EmpGrades {
    pub(super) fn valid(self) -> bool {
        self.power <= MAX_GRADE && self.capacity <= MAX_GRADE && self.recharge <= MAX_GRADE
    }
    pub(super) fn level(self, axis: EmpAxis) -> u8 {
        match axis {
            EmpAxis::Power => self.power,
            EmpAxis::Capacity => self.capacity,
            EmpAxis::Recharge => self.recharge,
        }
    }
    pub(super) fn advancing(self, axis: EmpAxis) -> Result<Self, EmpError> {
        if !self.valid() || self.level(axis) == MAX_GRADE {
            return Err(EmpError::MaximumGrade);
        }
        let mut next = self;
        match axis {
            EmpAxis::Power => next.power += 1,
            EmpAxis::Capacity => next.capacity += 1,
            EmpAxis::Recharge => next.recharge += 1,
        }
        Ok(next)
    }
    pub(super) fn upgrade_offer(self, axis: EmpAxis) -> Result<(u32, u8), EmpError> {
        let level = self.level(axis);
        if !self.valid() || level >= MAX_GRADE {
            return Err(EmpError::MaximumGrade);
        }
        let price = if axis == EmpAxis::Power {
            30_000
        } else {
            15_000
        };
        Ok((price << level, [10, 12, 14][usize::from(level)]))
    }
    pub(super) fn bank_capacity(self) -> u32 {
        (8 * SCALE) << self.capacity
    }
    pub(super) fn gain_per_tick(self) -> u32 {
        20 << self.recharge
    }
    pub(super) fn profile(self, energy_stage: u8) -> Result<Profile, EmpError> {
        if !self.valid() || energy_stage > 3 {
            return Err(EmpError::IncompatibleEnergy);
        }
        Ok(Profile {
            power: (16 * u32::from(2 + energy_stage) * SCALE) << self.power,
            range: 1500.0 * f64::from(2 + energy_stage),
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Profile {
    pub(super) power: u32,
    pub(super) range: f64,
}
impl Profile {
    pub(super) fn splash(self, distance: f64) -> Result<u32, EmpError> {
        if !distance.is_finite() || distance < 0.0 {
            return Err(EmpError::InvalidContact);
        }
        Ok(
            (f64::from(self.power) * (1.0 - distance / (self.range / 3.0)).clamp(0.0, 1.0)).floor()
                as u32,
        )
    }
}

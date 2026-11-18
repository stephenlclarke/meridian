//! Paid ECM hardware and the original finite standard-missile defence field.
mod persistence;
mod preparation;
pub use preparation::PreparedEcm;
mod validation;

use super::{CombatIdentity, FlightMode, MissileEnd, ModuleKind, ModuleSlot, World};
use crate::save::{Reader, SaveError, Writer};
use std::sync::Arc;

pub const ECM_PRICE_TENTHS: u32 = 21_000;
pub const ECM_REQUIRED_TECHNOLOGY: u8 = 5;
pub const ECM_ENERGY_COST_HUNDREDTHS: u16 = 2_200;
pub const ECM_FIELD_TICKS: u64 = 32;
const MAX_ACTIVATIONS: usize = 128;
const INVALID: SaveError = SaveError::InvalidState("ECM field, paid fit or missile termination");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcmError {
    NoCommander,
    StaleCommand,
    Paused,
    NotInFlight,
    NoFittedHardware,
    ElectricalDisabled,
    FieldActive,
    InsufficientEnergy,
    Capacity,
    Save(SaveError),
}
impl core::fmt::Display for EcmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ECM refused: {self:?}")
    }
}
impl std::error::Error for EcmError {}

/// The actual emitter and one activation, independent of the attack emission sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcmOrigin {
    pub owner: CombatIdentity,
    pub activation: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcmTermination {
    pub origin: EcmOrigin,
    /// The ECM effect followed this many already accepted attack emissions.
    pub after_emission: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcmSnapshot {
    pub fitted_item: Option<u64>,
    pub field: Option<EcmOrigin>,
    pub remaining_ticks: u8,
    pub activations: u16,
    pub terminated_missiles: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Activation {
    origin: EcmOrigin,
    item: u64,
    world_tick: u64,
    physical_tick: u64,
    after_emission: u64,
    closed: Option<FieldBoundary>,
}
/// First cold restore consumes live field permission, while preserving the timer and history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FieldBoundary {
    world_tick: u64,
    physical_tick: u64,
    after_emission: u64,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct State {
    activations: Arc<Vec<Activation>>,
}
impl State {
    pub(in crate::flight) fn activation_clocks(&self) -> impl Iterator<Item = u64> + '_ {
        self.activations
            .iter()
            .map(|activation| activation.physical_tick)
    }
    fn timer(&self, owner: CombatIdentity, clock: u64) -> Option<Activation> {
        self.activations.last().copied().filter(|activation| {
            activation.origin.owner == owner
                && clock >= activation.physical_tick
                && clock - activation.physical_tick < ECM_FIELD_TICKS
        })
    }
    fn active(&self, owner: CombatIdentity, clock: u64) -> Option<Activation> {
        self.timer(owner, clock)
            .filter(|activation| activation.closed.is_none())
    }
    pub(in crate::flight) fn close_restored(
        &mut self,
        owner: CombatIdentity,
        world_tick: u64,
        physical_tick: u64,
        after_emission: u64,
    ) {
        if self.active(owner, physical_tick).is_some()
            && let Some(activation) = Arc::make_mut(&mut self.activations).last_mut()
        {
            activation.closed = Some(FieldBoundary {
                world_tick,
                physical_tick,
                after_emission,
            });
        }
    }
    /// Every currently supported projectile is a genuine standard missile. The original
    /// player field affects both ownership sides in its current system visit.
    pub(in crate::flight) fn terminate_live(
        &self,
        missiles: &mut super::missiles::State,
        owner: CombatIdentity,
        tick: u64,
        clock: u64,
        after_emission: u64,
    ) {
        let Some(activation) = self.active(owner, clock) else {
            return;
        };
        for projectile in missiles.projectiles.iter_mut().flatten() {
            if projectile.ended.is_none()
                && projectile.origin.owner.system == owner.system
                && projectile.origin.owner.visit == owner.visit
            {
                projectile.ended = Some(MissileEnd::Ecm);
                projectile.last_tick = tick;
                projectile.ecm = Some(EcmTermination {
                    origin: activation.origin,
                    after_emission,
                });
            }
        }
    }
}
impl World {
    pub fn ecm_snapshot(&self) -> Option<EcmSnapshot> {
        let owner = self.combat_identity()?;
        let fitted_item = self.fitted_ecm_item();
        let timer = self.ecm.timer(owner, self.combat.physical_tick());
        let active = self.ecm.active(owner, self.combat.physical_tick());
        Some(EcmSnapshot {
            fitted_item,
            field: active.map(|activation| activation.origin),
            remaining_ticks: timer.map_or(0, |activation| {
                (ECM_FIELD_TICKS - (self.combat.physical_tick() - activation.physical_tick)) as u8
            }),
            activations: self.ecm.activations.len() as u16,
            terminated_missiles: self
                .missiles
                .projectiles
                .iter()
                .flatten()
                .filter(|projectile| projectile.ecm.is_some())
                .count() as u16,
        })
    }
    fn fitted_ecm_item(&self) -> Option<u64> {
        self.modular
            .ledger(self, self.commander_account.as_ref()?)
            .ok()?
            .items
            .iter()
            .find(|item| {
                item.kind == ModuleKind::EcmSystem && item.slot == Some(ModuleSlot::UtilityEcm)
            })
            .map(|item| item.id)
    }
    pub fn request_ecm(&mut self, tick: u64) -> Result<EcmOrigin, EcmError> {
        Ok(self.prepare_ecm(tick)?.commit())
    }
    /// Prepare the actual command and exact prospective save without publishing live state.
    pub fn prepare_ecm(&mut self, tick: u64) -> Result<PreparedEcm<'_>, EcmError> {
        if tick != self.state.tick {
            return Err(EcmError::StaleCommand);
        }
        if self.is_paused() {
            return Err(EcmError::Paused);
        }
        let owner = self.combat_identity().ok_or(EcmError::NoCommander)?;
        if self.state.mode != FlightMode::Flight || !self.pulse.alive() {
            return Err(EcmError::NotInFlight);
        }
        if self.emp.electrical_disabled() {
            return Err(EcmError::ElectricalDisabled);
        }
        let item = self.fitted_ecm_item().ok_or(EcmError::NoFittedHardware)?;
        let clock = self.combat.physical_tick();
        if self.ecm.timer(owner, clock).is_some() {
            return Err(EcmError::FieldActive);
        }
        if self.ecm.activations.len() >= MAX_ACTIVATIONS {
            return Err(EcmError::Capacity);
        }
        let pulse = self
            .pulse
            .prepare_ecm_discharge()
            .map_err(|_| EcmError::InsufficientEnergy)?;
        let origin = EcmOrigin {
            owner,
            activation: (self.ecm.activations.len() + 1) as u16,
        };
        let mut ecm = self.ecm.clone();
        Arc::make_mut(&mut ecm.activations).push(Activation {
            origin,
            item,
            world_tick: tick,
            physical_tick: clock,
            after_emission: self.combat.emissions,
            closed: None,
        });
        let mut missiles = self.missiles.clone();
        ecm.terminate_live(&mut missiles, owner, tick, clock, self.combat.emissions);
        let mut practice = self.practice;
        if let Some(attempt) = &mut practice {
            attempt.pending_missiles = missiles.pending_for_lesson(*attempt);
        }
        let mut emp = self.emp.clone();
        emp.cancel(clock);

        preparation::prepare(self, ecm, missiles, pulse, practice, emp, origin)
    }
}

#[cfg(test)]
mod tests;

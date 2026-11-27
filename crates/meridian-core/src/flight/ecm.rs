//! Stock fictional E.C.M. field. The ordnance owner, not the field, owns projectile outcomes.
use super::{CombatIdentity, FlightMode, OwnedModule, World, weapons::PulseResources};
use crate::navigation::SystemId;
use crate::save::{Reader, SaveError, Writer};
use std::sync::Arc;
mod field;
mod local;
pub use field::EcmTermination;

pub const ECM_ENERGY_HUNDREDTHS: u16 = 2_200;
pub const ECM_FIELD_TICKS: u64 = 32;
const MAX_ACTIVATIONS: usize = 512;
const INVALID: SaveError = SaveError::InvalidState("stock ECM activation history or ownership");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcmTrigger {
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcmActivation {
    pub sequence: u16,
    pub owner: CombatIdentity,
    pub item: u64,
    pub world_tick: u64,
    pub physical_tick: u64,
    pub trigger: EcmTrigger,
}
impl EcmActivation {
    fn expires(self) -> u64 {
        self.physical_tick.saturating_add(ECM_FIELD_TICKS)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcmSnapshot {
    pub fitted: Option<u64>,
    pub active: Option<EcmActivation>,
    pub remaining_ticks: u64,
    pub last_activation: Option<EcmActivation>,
    pub terminated_projectiles: u16,
    /// Saved policy intent; no incoming missile producer exists yet.
    pub xyzzy_automatic_requested: bool,
    /// Transient station assistance intent, not an activation or a missile outcome.
    pub docking_automatic_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcmError {
    StaleCommand,
    Paused,
    FlightRequired,
    MissingFit,
    ElectricalDisabled,
    FieldActive,
    InsufficientEnergy,
    Capacity,
    Save(SaveError),
}
impl core::fmt::Display for EcmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::StaleCommand => "E.C.M. command is stale",
            Self::Paused => "World is paused",
            Self::FlightRequired => "E.C.M. requires active local flight",
            Self::MissingFit => "Fit an owned E.C.M. System",
            Self::ElectricalDisabled => "E.C.M. is electrically disabled",
            Self::FieldActive => "E.C.M. field or cooldown is still active",
            Self::InsufficientEnergy => "E.C.M. requires more than 22 energy",
            Self::Capacity => "E.C.M. activation cannot be retained",
            Self::Save(error) => return write!(f, "E.C.M. activation cannot be saved: {error}"),
        })
    }
}
impl std::error::Error for EcmError {}

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct State {
    activations: Arc<Vec<EcmActivation>>,
    local: local::Local,
    field: field::History,
}
impl State {
    pub(super) fn is_empty(&self) -> bool {
        self.activations.is_empty() && self.local.is_empty() && !self.field.has_fields()
    }
    pub(super) fn unchanged_since(&self, old: &Self) -> bool {
        Arc::ptr_eq(&self.activations, &old.activations)
            && self.local.unchanged_since(&old.local)
            && self.field.unchanged_since(&old.field)
    }
    fn timer(&self, owner: CombatIdentity, clock: u64) -> Option<EcmActivation> {
        self.activations.last().copied().filter(|activation| {
            activation.owner == owner
                && clock >= activation.physical_tick
                && clock < activation.expires()
        })
    }
    fn active(&self, owner: CombatIdentity, clock: u64) -> Option<EcmActivation> {
        self.timer(owner, clock).filter(|&a| self.field.live(a))
    }
    pub(super) fn has_field_history(&self) -> bool {
        self.field.has_fields()
    }
    pub(super) fn close_restored(
        &mut self,
        owner: CombatIdentity,
        tick: u64,
        clock: u64,
        after_emission: u64,
    ) {
        if let Some(a) = self.active(owner, clock) {
            self.field.close(a, tick, clock, after_emission);
        }
    }
    pub(super) fn terminate_live(
        &mut self,
        missiles: &mut super::missiles::State,
        owner: CombatIdentity,
        tick: u64,
        clock: u64,
        after_emission: u64,
    ) -> Result<usize, EcmError> {
        let Some(a) = self.active(owner, clock) else {
            return Ok(0);
        };
        self.field
            .terminate(a, missiles, tick, clock, after_emission)
    }
    pub(super) fn termination(&self, id: super::ProjectileId) -> Option<EcmTermination> {
        self.field.termination(id)
    }
    pub(super) fn validate_emission(&self, clock: u64, emission: u64) -> Result<(), SaveError> {
        self.field
            .validate_emission(&self.activations, clock, emission)
    }
    pub(super) fn activations(&self) -> &[EcmActivation] {
        &self.activations
    }
    pub(super) fn encoded_len(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            let mut writer = Writer::new();
            self.encode(&mut writer);
            8 + writer.bytes.len()
        }
    }
    pub(super) fn prepare_launch(
        &self,
        world: &World,
        navigation: Option<super::NavigationState>,
    ) -> Result<Self, SaveError> {
        let mut next = self.clone();
        if let Some(item) = world.fitted_ecm() {
            let owner = world.combat_identity().ok_or(INVALID)?;
            let nav = navigation.ok_or(INVALID)?.snapshot(0);
            next.local.begin_launch(
                CombatIdentity {
                    system: nav.system,
                    visit: nav.visit,
                    ..owner
                },
                item.id,
                world.state.tick,
                world.combat.physical_tick(),
            )?;
        }
        Ok(next)
    }
    pub(super) fn prepare_departure(
        &self,
        owner: CombatIdentity,
        start: Option<crate::navigation::EcmJumpStart>,
    ) -> Result<Self, SaveError> {
        let mut next = self.clone();
        if let Some(start) = start {
            next.local.begin_departure(
                owner,
                start.destination,
                start.tick,
                start.clock,
                start.spool,
                start.before_activation,
            )?;
        }
        Ok(next)
    }
    pub(super) fn preceding_activation(
        &self,
        owner: CombatIdentity,
        tick: u64,
        clock: u64,
    ) -> Option<u16> {
        self.activations
            .last()
            .filter(|a| a.owner == owner && a.world_tick == tick && a.physical_tick == clock)
            .map(|a| a.sequence)
    }
    pub(super) fn prepare_step(
        &self,
        world: &World,
        next: super::Snapshot,
        navigation: Option<super::NavigationState>,
        clock: u64,
        motion: Option<super::solar::Motion>,
    ) -> Result<Self, SaveError> {
        let mut candidate = self.clone();
        candidate
            .local
            .step(world, next, navigation, clock, motion)?;
        Ok(candidate)
    }
    fn prepare(
        &self,
        owner: CombatIdentity,
        item: u64,
        world_tick: u64,
        physical_tick: u64,
        pulse: PulseResources,
        after_emission: u64,
    ) -> Result<(Self, PulseResources, EcmActivation), EcmError> {
        if self.timer(owner, physical_tick).is_some() {
            return Err(EcmError::FieldActive);
        }
        let pulse = pulse
            .prepare_ecm_discharge()
            .map_err(|_| EcmError::InsufficientEnergy)?;
        if self.activations.len() >= MAX_ACTIVATIONS
            || physical_tick.checked_add(ECM_FIELD_TICKS).is_none()
        {
            return Err(EcmError::Capacity);
        }
        let activation = EcmActivation {
            sequence: (self.activations.len() + 1) as u16,
            owner,
            item,
            world_tick,
            physical_tick,
            trigger: EcmTrigger::Manual,
        };
        let mut candidate = self.clone();
        Arc::make_mut(&mut candidate.activations).push(activation);
        candidate.field.activate(activation, after_emission);
        Ok((candidate, pulse, activation))
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.u16(self.activations.len() as u16);
        for activation in self.activations.iter() {
            w.u16(activation.sequence);
            w.u64(activation.owner.owner);
            w.u64(activation.owner.hull);
            w.u8(activation.owner.system.galaxy);
            w.u16(activation.owner.system.system);
            w.u64(activation.owner.visit);
            w.u64(activation.item);
            w.u64(activation.world_tick);
            w.u64(activation.physical_tick);
            w.u8(match activation.trigger {
                EcmTrigger::Manual => 0,
            });
        }
        self.local.encode(w);
        if self.has_field_history() {
            self.field.encode(w);
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>, field_history: bool) -> Result<Self, SaveError> {
        let count = usize::from(r.u16()?);
        if count > MAX_ACTIVATIONS {
            return Err(INVALID);
        }
        let mut activations = Vec::with_capacity(count);
        for _ in 0..count {
            activations.push(EcmActivation {
                sequence: r.u16()?,
                owner: CombatIdentity {
                    owner: r.u64()?,
                    hull: r.u64()?,
                    system: SystemId {
                        galaxy: r.u8()?,
                        system: r.u16()?,
                    },
                    visit: r.u64()?,
                },
                item: r.u64()?,
                world_tick: r.u64()?,
                physical_tick: r.u64()?,
                trigger: match r.u8()? {
                    0 => EcmTrigger::Manual,
                    _ => return Err(INVALID),
                },
            });
        }
        let state = Self {
            activations: Arc::new(activations),
            local: local::Local::decode(r)?,
            field: if field_history {
                field::History::decode(r)?
            } else {
                field::History::legacy(count)
            },
        };
        if state.is_empty() {
            return Err(INVALID);
        }
        Ok(state)
    }
    pub(super) fn validate(
        &self,
        world: &World,
        modular: &super::modular::State,
    ) -> Result<(), SaveError> {
        self.local.validate(world, modular, &self.activations)?;
        let owner = world.combat_identity().ok_or(INVALID)?;
        let end = world.combat.physical_tick();
        let mut previous: Option<EcmActivation> = None;
        for (index, &activation) in self.activations.iter().enumerate() {
            let at = activation.physical_tick;
            let tick = activation.world_tick;
            let visit = activation.owner.visit;
            if usize::from(activation.sequence) != index + 1
                || activation.owner.owner != owner.owner
                || activation.owner.hull != owner.hull
                || visit < 2
                || visit > owner.visit
                || tick > world.state.tick
                || at > end
                || at > tick
                || tick - at > world.state.tick.checked_sub(end).ok_or(INVALID)?
                || at.checked_add(ECM_FIELD_TICKS).is_none()
                || (activation.owner == owner && at == end && world.emp.electrical_disabled())
                || world.jump_journal.system_at_visit(visit, tick)? != activation.owner.system
                || modular
                    .ecm_item_at(owner.owner, tick, visit)
                    .is_none_or(|item| item.id != activation.item)
                || previous.is_some_and(|p| {
                    p.world_tick > tick
                        || p.physical_tick > at
                        || p.owner.visit > visit
                        || (p.owner.visit == visit && at < p.expires())
                })
            {
                return Err(INVALID);
            }
            let (jumps, arrival) = world.jump_journal.visit_origin(visit);
            let launch = visit
                .checked_sub(1)
                .and_then(|v| v.checked_sub(jumps))
                .ok_or(INVALID)?;
            let berth = world.damage.berth_clock(launch).ok_or(INVALID)?;
            if arrival.map_or(at <= berth, |arrival| at < arrival)
                || world
                    .damage
                    .berth_clock(launch.checked_add(1).ok_or(INVALID)?)
                    .is_some_and(|next| at >= next)
            {
                return Err(INVALID);
            }
            previous = Some(activation);
        }
        self.field.validate(world, &self.activations)?;
        if self.has_field_history() {
            world.damage.validate_ecm_emissions(world)?;
        }
        Ok(())
    }
}

impl World {
    pub(super) fn check_ecm_command_capacity(
        &self,
        candidate: &State,
        navigation: Option<super::NavigationState>,
        extra_save_bytes: usize,
    ) -> Result<(), SaveError> {
        if *candidate == self.ecm && extra_save_bytes == 0 {
            return Ok(());
        }
        let navigation_size = |navigation: Option<super::NavigationState>| {
            let mut w = Writer::new();
            if let Some(nav) = navigation {
                nav.encode_save(&mut w);
            }
            w.bytes.len()
        };
        let size = self
            .encoded_current_size()?
            .checked_sub(self.ecm.encoded_len())
            .and_then(|n| n.checked_add(candidate.encoded_len()))
            .and_then(|n| n.checked_sub(navigation_size(self.navigation)))
            .and_then(|n| n.checked_add(navigation_size(navigation)))
            .and_then(|n| n.checked_add(extra_save_bytes))
            .ok_or(SaveError::TooLarge)?;
        if size > crate::save::MAX_SAVE_BYTES {
            return Err(SaveError::TooLarge);
        }
        Ok(())
    }
    pub fn fitted_ecm(&self) -> Option<OwnedModule> {
        self.modular
            .ledger(self, self.commander_account.as_ref()?)
            .ok()?
            .fitted(super::ModuleSlot::UtilityEcm)
    }
    pub(super) fn active_ecm(&self) -> Option<EcmActivation> {
        if self.state.mode != FlightMode::Flight || !self.pulse.alive() {
            return None;
        }
        self.ecm
            .active(self.combat_identity()?, self.combat.physical_tick())
    }
    pub(super) fn ecm_timer(&self) -> Option<EcmActivation> {
        if self.state.mode != FlightMode::Flight || !self.pulse.alive() {
            return None;
        }
        self.ecm
            .timer(self.combat_identity()?, self.combat.physical_tick())
    }
    pub fn ecm_termination(&self, id: super::ProjectileId) -> Option<EcmTermination> {
        self.ecm.termination(id)
    }
    pub fn ecm_snapshot(&self) -> Option<EcmSnapshot> {
        self.commander_account.as_ref()?;
        let active = self.active_ecm();
        Some(EcmSnapshot {
            fitted: self.fitted_ecm().map(|item| item.id),
            remaining_ticks: self
                .ecm_timer()
                .map_or(0, |a| a.expires() - self.combat.physical_tick()),
            active,
            last_activation: self.ecm.activations().last().copied(),
            terminated_projectiles: self.ecm.field.count(),
            xyzzy_automatic_requested: self.xyzzy.automatic_ecm(),
            docking_automatic_requested: self.auto_docking.is_some(),
        })
    }
    /// One explicit command at the current physical clock; never advances the simulation.
    pub fn request_ecm(&mut self, tick: u64) -> Result<EcmActivation, EcmError> {
        Ok(self.prepare_ecm(tick)?.commit())
    }
    /// Stage the exact game state and checked save. The caller may atomically
    /// publish these bytes before commit; dropping preparation changes nothing.
    pub fn prepare_ecm(&mut self, tick: u64) -> Result<PreparedEcm<'_>, EcmError> {
        if tick != self.state.tick {
            return Err(EcmError::StaleCommand);
        }
        if self.is_paused() {
            return Err(EcmError::Paused);
        }
        if self.state.mode != FlightMode::Flight
            || !self.pulse.alive()
            || !self.damage.enabled()
            || self
                .navigation
                .is_some_and(|n| n.snapshot(0).travel.is_some())
        {
            return Err(EcmError::FlightRequired);
        }
        if self.emp.electrical_disabled() {
            return Err(EcmError::ElectricalDisabled);
        }
        let owner = self.combat_identity().ok_or(EcmError::FlightRequired)?;
        let item = self.fitted_ecm().ok_or(EcmError::MissingFit)?;
        let (mut candidate, pulse, activation) = self.ecm.prepare(
            owner,
            item.id,
            tick,
            self.combat.physical_tick(),
            self.pulse,
            self.combat.emissions,
        )?;
        let mut missiles = self.missiles.clone();
        let terminated = candidate.terminate_live(
            &mut missiles,
            owner,
            tick,
            activation.physical_tick,
            self.combat.emissions,
        )?;
        let mut practice = self.practice;
        if let Some(attempt) = &mut practice {
            attempt.pending_missiles = missiles.pending_for_lesson(*attempt);
            if terminated > 0 {
                attempt.observe_discharge(tick);
            }
        }
        let mut emp = self.emp.clone();
        emp.cancel_for_ecm(activation.physical_tick);
        let old_ecm = std::mem::replace(&mut self.ecm, candidate);
        let old_missiles = std::mem::replace(&mut self.missiles, missiles);
        let old_pulse = std::mem::replace(&mut self.pulse, pulse);
        let old_emp = std::mem::replace(&mut self.emp, emp);
        let old_practice = std::mem::replace(&mut self.practice, practice);
        let result = self.save_bytes();
        let candidate = std::mem::replace(&mut self.ecm, old_ecm);
        let missiles = std::mem::replace(&mut self.missiles, old_missiles);
        let pulse = std::mem::replace(&mut self.pulse, old_pulse);
        let emp = std::mem::replace(&mut self.emp, old_emp);
        let practice = std::mem::replace(&mut self.practice, old_practice);
        let bytes = result.map_err(|error| {
            if error == SaveError::TooLarge {
                EcmError::Capacity
            } else {
                EcmError::Save(error)
            }
        })?;
        Ok(PreparedEcm {
            world: self,
            candidate,
            missiles,
            pulse,
            emp,
            practice,
            activation,
            terminated,
            bytes,
        })
    }
}

pub struct PreparedEcm<'a> {
    world: &'a mut World,
    candidate: State,
    missiles: super::missiles::State,
    pulse: PulseResources,
    emp: super::emp::State,
    practice: Option<super::practice::Practice>,
    activation: EcmActivation,
    terminated: usize,
    bytes: Vec<u8>,
}
impl PreparedEcm<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn activation(&self) -> EcmActivation {
        self.activation
    }
    pub fn terminated(&self) -> usize {
        self.terminated
    }
    pub fn publish_with<E>(
        self,
        writer: impl FnOnce(&[u8]) -> Result<(), E>,
    ) -> Result<EcmActivation, E> {
        writer(&self.bytes)?;
        Ok(self.commit())
    }
    pub fn commit(self) -> EcmActivation {
        self.world.ecm = self.candidate;
        self.world.missiles = self.missiles;
        self.world.pulse = self.pulse;
        self.world.emp = self.emp;
        self.world.practice = self.practice;
        self.activation
    }
}

#[cfg(test)]
mod tests;

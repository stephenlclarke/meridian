//! Stock fictional E.C.M. field. The ordnance owner, not the field, owns projectile outcomes.
use super::{CombatIdentity, FlightMode, OwnedModule, World, weapons::PulseResources};
use crate::navigation::SystemId;
use crate::save::{Reader, SaveError, Writer};
use std::sync::Arc;
mod local;

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
}
impl core::fmt::Display for EcmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::StaleCommand => "E.C.M. command is stale",
            Self::Paused => "World is paused",
            Self::FlightRequired => "E.C.M. requires active local flight",
            Self::MissingFit => "Fit an owned E.C.M. System",
            Self::ElectricalDisabled => "E.C.M. is electrically disabled",
            Self::FieldActive => "E.C.M. field is already active",
            Self::InsufficientEnergy => "E.C.M. requires more than 22 energy",
            Self::Capacity => "E.C.M. activation cannot be retained",
        })
    }
}
impl std::error::Error for EcmError {}

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct State {
    activations: Arc<Vec<EcmActivation>>,
    local: local::Local,
}
impl State {
    pub(super) fn is_empty(&self) -> bool {
        self.activations.is_empty() && self.local.is_empty()
    }
    pub(super) fn unchanged_since(&self, old: &Self) -> bool {
        Arc::ptr_eq(&self.activations, &old.activations) && self.local.unchanged_since(&old.local)
    }
    pub(super) fn active(&self, owner: CombatIdentity, clock: u64) -> Option<EcmActivation> {
        self.activations.last().copied().filter(|activation| {
            activation.owner == owner
                && clock >= activation.physical_tick
                && clock < activation.expires()
        })
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
        fitted: bool,
        destination: SystemId,
        tick: u64,
        clock: u64,
        spool: u32,
    ) -> Result<Self, SaveError> {
        let mut next = self.clone();
        if fitted {
            let before = self
                .activations
                .last()
                .filter(|a| a.owner == owner && a.world_tick == tick && a.physical_tick == clock)
                .map(|a| a.sequence);
            next.local
                .begin_departure(owner, destination, tick, clock, spool, before)?;
        }
        Ok(next)
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
        trigger: EcmTrigger,
        pulse: PulseResources,
    ) -> Result<(Self, PulseResources, EcmActivation), EcmError> {
        if self.active(owner, physical_tick).is_some() {
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
            trigger,
        };
        let mut candidate = self.clone();
        Arc::make_mut(&mut candidate.activations).push(activation);
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
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
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
        Ok(())
    }
}

impl World {
    pub(super) fn check_ecm_command_capacity(
        &self,
        candidate: &State,
        navigation: Option<super::NavigationState>,
    ) -> Result<(), SaveError> {
        if *candidate == self.ecm {
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
    pub fn ecm_snapshot(&self) -> Option<EcmSnapshot> {
        self.commander_account.as_ref()?;
        let active = self.active_ecm();
        Some(EcmSnapshot {
            fitted: self.fitted_ecm().map(|item| item.id),
            remaining_ticks: active.map_or(0, |a| a.expires() - self.combat.physical_tick()),
            active,
            last_activation: self.ecm.activations().last().copied(),
            xyzzy_automatic_requested: self.xyzzy.automatic_ecm(),
            docking_automatic_requested: self.auto_docking.is_some(),
        })
    }
    /// One explicit command at the current physical clock; never advances the simulation.
    pub fn request_ecm(&mut self, tick: u64) -> Result<EcmActivation, EcmError> {
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
        let (candidate, pulse, activation) = self.ecm.prepare(
            owner,
            item.id,
            tick,
            self.combat.physical_tick(),
            EcmTrigger::Manual,
            self.pulse,
        )?;
        let size = self
            .encoded_current_size()
            .map_err(|_| EcmError::Capacity)?;
        let mut emp = self.emp.clone();
        emp.cancel_for_ecm(activation.physical_tick);
        let emp_size = |state: &super::emp::State| {
            let mut writer = Writer::new();
            state.encode(&mut writer);
            writer.bytes.len()
        };
        if size
            .checked_sub(self.ecm.encoded_len())
            .and_then(|n| n.checked_add(candidate.encoded_len()))
            .and_then(|n| n.checked_sub(emp_size(&self.emp)))
            .and_then(|n| n.checked_add(emp_size(&emp)))
            .is_none_or(|n| n > crate::save::MAX_SAVE_BYTES)
        {
            return Err(EcmError::Capacity);
        }
        self.ecm = candidate;
        self.pulse = pulse;
        self.emp = emp;
        Ok(activation)
    }
}

#[cfg(test)]
mod tests;

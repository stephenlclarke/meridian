//! EMP drains shields then independent electrical power. It never pays fatal hull damage.
use super::{
    Arc, INVALID, MissileScene, NpcId, OriginalStationData, Resources, SaveError, SceneStage,
    State, SystemId, TrafficError, system_index,
};
use crate::flight::emp::{EMP_RESOURCE_SCALE, EmpOrigin};
use crate::save::{Reader, Writer};
pub(super) const MAX_EMP_EVENTS: usize = 128;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Electrical {
    pub(crate) remaining: u32,
    pub(crate) disabled: bool,
    pub(crate) crew_aboard: u8,
}
impl Electrical {
    fn fresh() -> Self {
        Self {
            remaining: 380 * EMP_RESOURCE_SCALE,
            disabled: false,
            crew_aboard: 1,
        }
    }
    pub(super) fn advance(&mut self, ticks: u64) {
        if !self.disabled {
            self.remaining = (u64::from(self.remaining).saturating_add(ticks.saturating_mul(210)))
                .min(u64::from(380 * EMP_RESOURCE_SCALE)) as u32;
        }
    }
    pub(super) fn encode(self, w: &mut Writer) {
        w.u32(self.remaining);
        w.bool(self.disabled);
        w.u8(self.crew_aboard);
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let pool = Self {
            remaining: r.u32()?,
            disabled: r.bool()?,
            crew_aboard: r.u8()?,
        };
        if pool.crew_aboard != 1
            || pool.remaining > 380 * EMP_RESOURCE_SCALE
            || (pool.disabled && pool.remaining != 0)
        {
            return Err(INVALID);
        }
        Ok(pool)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmpResourceAction {
    Drain { front: bool, power_hundredths: u16 },
    Recover,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EmpResourceEvent {
    pub(crate) origin: EmpOrigin,
    pub(crate) victim: NpcId,
    pub(crate) local_tick: u64,
    pub(crate) world_tick: u64,
    pub(crate) physical_tick: u64,
    pub(crate) action: EmpResourceAction,
}
impl EmpResourceEvent {
    pub(super) fn encode(self, w: &mut Writer) {
        self.origin.encode(w);
        self.victim.encode(w);
        w.u64(self.local_tick);
        w.u64(self.world_tick);
        w.u64(self.physical_tick);
        match self.action {
            EmpResourceAction::Drain {
                front,
                power_hundredths,
            } => {
                w.u8(0);
                w.bool(front);
                w.u16(power_hundredths);
            }
            EmpResourceAction::Recover => w.u8(1),
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            origin: EmpOrigin::decode(r)?,
            victim: NpcId::decode(r)?,
            local_tick: r.u64()?,
            world_tick: r.u64()?,
            physical_tick: r.u64()?,
            action: match r.u8()? {
                0 => EmpResourceAction::Drain {
                    front: r.bool()?,
                    power_hundredths: r.u16()?,
                },
                1 => EmpResourceAction::Recover,
                _ => return Err(INVALID),
            },
        })
    }
}
impl Resources {
    pub(super) fn apply_emp(&mut self, action: EmpResourceAction) -> Result<(), TrafficError> {
        if self.energy == 0 {
            return Err(TrafficError::InvalidState);
        }
        match action {
            EmpResourceAction::Drain {
                front,
                power_hundredths,
            } => {
                let pool = self.electrical.get_or_insert_with(Electrical::fresh);
                if pool.disabled || power_hundredths == 0 {
                    return Err(TrafficError::InvalidState);
                }
                let shield = if front {
                    &mut self.front
                } else {
                    &mut self.rear
                };
                let absorbed = (*shield).min(power_hundredths);
                *shield -= absorbed;
                let total = if front {
                    &mut self.absorbed_front
                } else {
                    &mut self.absorbed_rear
                };
                *total = total
                    .checked_add(u64::from(absorbed))
                    .ok_or(TrafficError::SequenceExhausted)?;
                self.emp_absorbed = self
                    .emp_absorbed
                    .checked_add(u64::from(absorbed))
                    .ok_or(TrafficError::SequenceExhausted)?;
                pool.remaining = pool.remaining.saturating_sub(
                    u32::from(power_hundredths - absorbed) * (EMP_RESOURCE_SCALE / 100),
                );
                if pool.remaining == 0 {
                    pool.disabled = true;
                }
            }
            EmpResourceAction::Recover => {
                let pool = self
                    .electrical
                    .as_mut()
                    .filter(|pool| pool.disabled)
                    .ok_or(TrafficError::InvalidState)?;
                pool.remaining = 210;
                pool.disabled = false;
            }
        }
        Ok(())
    }
    pub(super) fn emp_disabled(self) -> bool {
        self.electrical.is_some_and(|pool| pool.disabled)
    }
}
impl State {
    pub(crate) fn emp_resources(&self, id: NpcId) -> Option<(u16, u16, u16, Option<Electrical>)> {
        let a = self.actors[self.actor_index(id)?]?;
        Some((
            a.resources.energy,
            a.resources.front,
            a.resources.rear,
            a.resources.electrical,
        ))
    }
    pub(crate) fn emp_events(&self) -> &[EmpResourceEvent] {
        &self.emp_events
    }
    pub(crate) fn emp_event(
        &mut self,
        origin: EmpOrigin,
        id: NpcId,
        world_tick: u64,
        physical_tick: u64,
        action: EmpResourceAction,
    ) -> Result<(), TrafficError> {
        if self.emp_events.len() >= MAX_EMP_EVENTS {
            return Err(TrafficError::ReceiptCapacity);
        }
        let index = self.actor_index(id).ok_or(TrafficError::InvalidState)?;
        let a = self.actors[index]
            .as_mut()
            .ok_or(TrafficError::InvalidState)?;
        if !a.airborne() {
            return Err(TrafficError::InvalidState);
        }
        a.resources.apply_emp(action)?;
        Arc::make_mut(&mut self.emp_events).push(EmpResourceEvent {
            origin,
            victim: id,
            local_tick: a.resource_clock,
            world_tick,
            physical_tick,
            action,
        });
        Ok(())
    }
}
impl State {
    pub(crate) fn emp_clock(&self, system: SystemId) -> Option<u64> {
        system_index(system).map(|si| self.clocks[si])
    }
    /// Reuse actual native resource ordering, without allocating or aliasing an ordinary shot.
    pub(crate) fn emp_before(
        &self,
        id: NpcId,
        clock: u64,
        world: u64,
        before_pulse: u16,
    ) -> Result<(u16, u16, u16, Option<Electrical>), SaveError> {
        let a = self.actors[self.actor_index(id).ok_or(INVALID)?].ok_or(INVALID)?;
        if a.born_world_tick > world {
            return Err(INVALID);
        }
        let si = system_index(id.system).ok_or(INVALID)?;
        if clock > self.clocks[si] || clock < a.birth_clock {
            return Err(INVALID);
        }
        let clock = clock.min(a.resource_clock);
        let mut prefix = self.clone();
        prefix.emp_events = Arc::new(
            self.emp_events
                .iter()
                .filter(|e| e.origin.pulse < before_pulse)
                .copied()
                .collect(),
        );
        let resources = prefix.resources_at(a, clock, world, SceneStage::Command(u64::MAX))?;
        Ok((
            resources.energy,
            resources.front,
            resources.rear,
            resources.electrical,
        ))
    }
}

impl State {
    pub(crate) fn validate_emp_scene(
        &self,
        scene: &MissileScene,
        end: u64,
        data: &OriginalStationData,
        before_pulse: u16,
    ) -> Result<(), SaveError> {
        let mut prefix = self.clone();
        prefix.emp_events = Arc::new(
            self.emp_events
                .iter()
                .filter(|event| event.origin.pulse < before_pulse)
                .copied()
                .collect(),
        );
        prefix.validate_missile_scene(scene, end, data)
    }
}

impl State {
    pub(crate) fn emp_before_recovery(
        &self,
        id: NpcId,
        clock: u64,
        tick: u64,
        origin: EmpOrigin,
    ) -> Result<(u16, u16, u16, Option<Electrical>), SaveError> {
        let mut prefix = self.clone();
        prefix.emp_events = Arc::new(
            self.emp_events
                .iter()
                .filter(|event| {
                    !(event.origin == origin
                        && event.victim == id
                        && event.action == EmpResourceAction::Recover)
                })
                .copied()
                .collect(),
        );
        prefix.emp_before(id, clock, tick, origin.pulse.checked_add(1).ok_or(INVALID)?)
    }
}

impl State {
    pub(crate) fn emp_dead_by(&self, id: NpcId, local: u64) -> bool {
        self.actor_index(id)
            .and_then(|i| self.actors[i])
            .is_some_and(|a| a.resources.energy == 0 && a.resource_clock <= local)
    }
}

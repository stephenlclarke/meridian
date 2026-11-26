//! Actual local standard-projectile outcomes and cold field custody. No NPC
//! missile producer or automatic activation is represented by this history.
use super::{EcmActivation, EcmError, INVALID, MAX_ACTIVATIONS, World};
use crate::flight::{MissileEnd, ProjectileId, ProjectileSnapshot, missiles};
use crate::save::{Reader, SaveError, Writer};
use std::sync::Arc;

const MAX_TERMINATIONS: usize = 256; // ProjectileId uses a retained u8 ordinal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcmTermination {
    pub projectile: ProjectileId,
    pub activation_sequence: u16,
    pub world_tick: u64,
    pub physical_tick: u64,
    pub after_emission: u64,
}

#[cfg(test)]
mod tests;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Boundary {
    tick: u64,
    clock: u64,
    after_emission: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Field {
    sequence: u16,
    after_emission: u64,
    closed: Option<Boundary>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct History {
    // Genuine v2 stock-ECM saves had only the manual timer, no game ordnance
    // effect. An imported prefix never gains a new live field or retroactive kill.
    legacy_prefix: u16,
    fields: Arc<Vec<Field>>,
    terminations: Arc<Vec<EcmTermination>>,
}
impl History {
    pub(super) fn legacy(count: usize) -> Self {
        Self {
            legacy_prefix: count as u16,
            ..Self::default()
        }
    }
    pub(super) fn has_fields(&self) -> bool {
        !self.fields.is_empty() || !self.terminations.is_empty()
    }
    pub(super) fn unchanged_since(&self, old: &Self) -> bool {
        self.legacy_prefix == old.legacy_prefix
            && Arc::ptr_eq(&self.fields, &old.fields)
            && Arc::ptr_eq(&self.terminations, &old.terminations)
    }
    pub(super) fn activate(&mut self, activation: EcmActivation, after_emission: u64) {
        Arc::make_mut(&mut self.fields).push(Field {
            sequence: activation.sequence,
            after_emission,
            closed: None,
        });
    }
    fn field(&self, sequence: u16) -> Option<Field> {
        self.fields.iter().find(|f| f.sequence == sequence).copied()
    }
    pub(super) fn live(&self, activation: EcmActivation) -> bool {
        self.field(activation.sequence)
            .is_some_and(|f| f.closed.is_none())
    }
    pub(super) fn close(
        &mut self,
        activation: EcmActivation,
        tick: u64,
        clock: u64,
        after_emission: u64,
    ) {
        if self.live(activation)
            && let Some(field) = Arc::make_mut(&mut self.fields).last_mut()
        {
            field.closed = Some(Boundary {
                tick,
                clock,
                after_emission,
            });
        }
    }
    pub(super) fn termination(&self, id: ProjectileId) -> Option<EcmTermination> {
        self.terminations
            .iter()
            .find(|t| t.projectile == id)
            .copied()
    }
    pub(super) fn count(&self) -> u16 {
        self.terminations.len() as u16
    }
    pub(super) fn terminate(
        &mut self,
        activation: EcmActivation,
        missiles: &mut missiles::State,
        tick: u64,
        clock: u64,
        after_emission: u64,
    ) -> Result<usize, EcmError> {
        let mut count = 0;
        for projectile in missiles.projectiles.iter_mut().flatten() {
            if projectile.ended.is_none() && projectile.origin.owner == activation.owner {
                if self.terminations.len() >= MAX_TERMINATIONS {
                    return Err(EcmError::Capacity);
                }
                projectile.ended = Some(MissileEnd::Ecm);
                projectile.last_tick = tick;
                Arc::make_mut(&mut self.terminations).push(EcmTermination {
                    projectile: projectile.origin.projectile,
                    activation_sequence: activation.sequence,
                    world_tick: tick,
                    physical_tick: clock,
                    after_emission,
                });
                count += 1;
            }
        }
        Ok(count)
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.u16(self.legacy_prefix);
        w.u16(self.fields.len() as u16);
        for f in self.fields.iter() {
            w.u16(f.sequence);
            w.u64(f.after_emission);
            w.bool(f.closed.is_some());
            // Fixed width reserves the first cold boundary before the field is used.
            let b = f.closed.unwrap_or(Boundary {
                tick: 0,
                clock: 0,
                after_emission: 0,
            });
            w.u64(b.tick);
            w.u64(b.clock);
            w.u64(b.after_emission);
        }
        w.u16(self.terminations.len() as u16);
        for t in self.terminations.iter() {
            t.projectile.encode(w);
            w.u16(t.activation_sequence);
            w.u64(t.world_tick);
            w.u64(t.physical_tick);
            w.u64(t.after_emission);
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let legacy_prefix = r.u16()?;
        let count = usize::from(r.u16()?);
        if count == 0
            || count > MAX_ACTIVATIONS
            || usize::from(legacy_prefix) + count > MAX_ACTIVATIONS
        {
            return Err(INVALID);
        }
        let mut fields = Vec::with_capacity(count);
        for _ in 0..count {
            let sequence = r.u16()?;
            let after_emission = r.u64()?;
            let closed = r.bool()?;
            let boundary = Boundary {
                tick: r.u64()?,
                clock: r.u64()?,
                after_emission: r.u64()?,
            };
            if !closed
                && boundary
                    != (Boundary {
                        tick: 0,
                        clock: 0,
                        after_emission: 0,
                    })
            {
                return Err(INVALID);
            }
            fields.push(Field {
                sequence,
                after_emission,
                closed: closed.then_some(boundary),
            });
        }
        let count = usize::from(r.u16()?);
        if count > MAX_TERMINATIONS {
            return Err(INVALID);
        }
        let mut terminations = Vec::with_capacity(count);
        for _ in 0..count {
            terminations.push(EcmTermination {
                projectile: ProjectileId::decode(r)?,
                activation_sequence: r.u16()?,
                world_tick: r.u64()?,
                physical_tick: r.u64()?,
                after_emission: r.u64()?,
            });
        }
        Ok(Self {
            legacy_prefix,
            fields: Arc::new(fields),
            terminations: Arc::new(terminations),
        })
    }
    pub(super) fn validate_emission(
        &self,
        activations: &[EcmActivation],
        clock: u64,
        emission: u64,
    ) -> Result<(), SaveError> {
        let crosses =
            |at, after| (clock < at && emission > after) || (clock > at && emission <= after);
        for field in self.fields.iter() {
            let a = activations
                .get(usize::from(field.sequence.checked_sub(1).ok_or(INVALID)?))
                .ok_or(INVALID)?;
            if crosses(a.physical_tick, field.after_emission)
                || field
                    .closed
                    .is_some_and(|b| crosses(b.clock, b.after_emission))
            {
                return Err(INVALID);
            }
        }
        Ok(())
    }
    fn expected(
        &self,
        activation: EcmActivation,
        field: Field,
        projectile: ProjectileSnapshot,
    ) -> Option<EcmTermination> {
        let birth = projectile.origin;
        if birth.owner != activation.owner {
            return None;
        }
        if field.closed.is_some_and(|b| {
            birth.launched_tick > b.tick
                || (birth.launched_tick == b.tick && birth.emission > b.after_emission)
        }) {
            return None;
        }
        let (tick, clock, after) = if birth.launched_tick > activation.world_tick
            || (birth.launched_tick == activation.world_tick
                && birth.emission > field.after_emission)
        {
            if birth.physical_tick < activation.physical_tick
                || birth.physical_tick >= activation.expires()
            {
                return None;
            }
            (birth.launched_tick, birth.physical_tick, birth.emission)
        } else {
            let ended_before = projectile.ended.is_some()
                && (projectile.last_tick < activation.world_tick
                    || (projectile.last_tick == activation.world_tick
                        && match projectile.ended {
                            Some(MissileEnd::Ecm) => self
                                .termination(birth.projectile)
                                .is_some_and(|t| t.activation_sequence < activation.sequence),
                            Some(MissileEnd::Intercepted) => projectile
                                .intercepting_emission
                                .is_some_and(|e| e <= field.after_emission),
                            _ => true,
                        }));
            if ended_before {
                return None;
            }
            (
                activation.world_tick,
                activation.physical_tick,
                field.after_emission,
            )
        };
        Some(EcmTermination {
            projectile: birth.projectile,
            activation_sequence: activation.sequence,
            world_tick: tick,
            physical_tick: clock,
            after_emission: after,
        })
    }
    pub(super) fn validate(
        &self,
        world: &World,
        activations: &[EcmActivation],
    ) -> Result<(), SaveError> {
        if usize::from(self.legacy_prefix) + self.fields.len() != activations.len() {
            return Err(INVALID);
        }
        let mut previous: Option<(EcmActivation, Field)> = None;
        for (offset, field) in self.fields.iter().copied().enumerate() {
            let index = usize::from(self.legacy_prefix) + offset;
            let a = activations[index];
            if usize::from(field.sequence) != index + 1
                || field.sequence != a.sequence
                || field.after_emission > world.combat.emissions
                || previous.is_some_and(|(_, p)| {
                    p.after_emission > field.after_emission
                        || p.closed.is_some_and(|b| {
                            b.after_emission > field.after_emission
                                || b.clock > a.physical_tick
                                || b.tick > a.world_tick
                        })
                })
            {
                return Err(INVALID);
            }
            if let Some(b) = field.closed
                && (b.tick < a.world_tick
                    || b.tick > world.state.tick
                    || b.clock < a.physical_tick
                    || b.clock >= a.expires()
                    || b.clock > world.combat.physical_tick()
                    || b.tick - a.world_tick != b.clock - a.physical_tick
                    || b.after_emission < field.after_emission
                    || b.after_emission > world.combat.emissions
                    || world.jump_journal.system_at_visit(a.owner.visit, b.tick)? != a.owner.system)
            {
                return Err(INVALID);
            }
            previous = Some((a, field));
        }
        let mut prior = None;
        for t in self.terminations.iter() {
            let order = (
                t.activation_sequence,
                t.physical_tick,
                t.after_emission,
                t.projectile.ordinal(),
            );
            if prior.is_some_and(|p| p >= order)
                || world
                    .missile_projectiles()
                    .all(|p| p.origin.projectile != t.projectile)
            {
                return Err(INVALID);
            }
            prior = Some(order);
        }
        for p in world.missile_projectiles() {
            let expected = self.fields.iter().find_map(|field| {
                activations
                    .get(usize::from(field.sequence - 1))
                    .and_then(|&a| self.expected(a, *field, p))
            });
            let actual = self.termination(p.origin.projectile);
            match (expected, actual, p.ended) {
                (None, None, end) if end != Some(MissileEnd::Ecm) => {}
                (Some(expected), Some(actual), Some(MissileEnd::Ecm))
                    if expected == actual
                        && p.last_tick == actual.world_tick
                        && p.contact.is_none()
                        && p.intercepting_emission.is_none()
                        && actual.physical_tick
                            == p.origin
                                .physical_tick
                                .checked_add(u64::from(p.age_ticks))
                                .ok_or(INVALID)? => {}
                _ => return Err(INVALID),
            }
        }
        if self.terminations.len()
            != world
                .missile_projectiles()
                .filter(|p| p.ended == Some(MissileEnd::Ecm))
                .count()
        {
            return Err(INVALID);
        }
        Ok(())
    }
}

//! Actual accepted work, separate from ore cargo ownership.
use super::{
    Arc, BELT_BODY_COUNT, INVALID, MAX_PENDING_ORE, MINING_DAMAGE, MINING_RANGE, MiningContact,
    MiningError, RigidPose, SaveError, State, SystemId, finite,
};
const MAX_STROKES: usize = BELT_BODY_COUNT * 8;
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MiningStroke {
    pub(crate) owner: crate::CombatIdentity,
    pub(crate) item: u64,
    pub(crate) account_revision: u64,
    pub(crate) world_tick: u64,
    pub(crate) physical_tick: u64,
    pub(crate) emission: u64,
    pub(crate) pose: RigidPose,
    pub(crate) speed: f32,
    pub(crate) contact: MiningContact,
    pub(crate) peers: crate::traffic::MissileScene,
    pub(crate) missiles: Arc<Vec<(crate::ProjectileId, RigidPose)>>,
}
impl State {
    pub(crate) fn unchanged_since(&self, other: &Self) -> bool {
        self.enabled == other.enabled
            && self.expired == other.expired
            && Arc::ptr_eq(&self.strokes, &other.strokes)
    }
    pub(crate) fn before_emission(&self, emission: u64) -> Self {
        Self {
            enabled: self.enabled,
            strokes: Arc::new(
                self.strokes
                    .iter()
                    .take_while(|s| s.emission < emission)
                    .cloned()
                    .collect(),
            ),
            expired: self.expired,
        }
    }
    pub(crate) fn at_step(&self, tick: u64) -> Self {
        Self {
            enabled: self.enabled,
            strokes: Arc::new(
                self.strokes
                    .iter()
                    .take_while(|s| s.world_tick < tick)
                    .cloned()
                    .collect(),
            ),
            expired: self.expired,
        }
    }
    pub(crate) fn expiry_receipts(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.expired.iter().flatten().copied()
    }
    pub(crate) fn prepare_work(&self, stroke: MiningStroke) -> Result<(Self, u8), MiningError> {
        if !self.enabled {
            return Err(MiningError::NoTarget);
        }
        self.validate().map_err(|_| MiningError::InvalidState)?;
        if self.strokes.len() >= MAX_STROKES {
            return Err(MiningError::ProofCapacity);
        }
        let body = self
            .body(usize::from(stroke.contact.asteroid.ordinal))
            .filter(|b| b.id == stroke.contact.asteroid)
            .ok_or(MiningError::InvalidState)?;
        if body.remaining_energy <= 0.0 {
            return Err(MiningError::Depleted);
        }
        let released = if body.remaining_energy <= f64::from(MINING_DAMAGE) {
            body.initial_units
        } else {
            0
        };
        if self.pending_count() + usize::from(released) > MAX_PENDING_ORE {
            return Err(MiningError::ProofCapacity);
        }
        if self.beam(
            stroke.pose.position,
            stroke.pose.orientation.z,
            MINING_RANGE,
        ) != Some(stroke.contact)
        {
            return Err(MiningError::InvalidState);
        }
        let mut candidate = self.clone();
        Arc::make_mut(&mut candidate.strokes).push(stroke);
        Ok((candidate, released))
    }
    pub(crate) fn prepare_departure(
        &self,
        source: crate::CombatIdentity,
        destination: SystemId,
        tick: u64,
    ) -> Self {
        let mut next = self.clone();
        if self.enabled && source.system == SystemId::AVENOR && destination != source.system {
            for body in self
                .bodies()
                .into_iter()
                .filter(|b| b.remaining_energy == 0.0)
            {
                let slot = usize::from(body.id.ordinal);
                if next.expired[slot].is_none() {
                    next.expired[slot] = Some((tick, source.visit));
                }
            }
        }
        next
    }
    pub(crate) fn validate(&self) -> Result<(), SaveError> {
        if !self.enabled {
            return if self.strokes.is_empty() && self.expired == [None; BELT_BODY_COUNT] {
                Ok(())
            } else {
                Err(INVALID)
            };
        }
        if self.strokes.len() > MAX_STROKES || self.pending_count() > MAX_PENDING_ORE {
            return Err(INVALID);
        }
        let mut prior_world = 0;
        let mut prior_clock = 0;
        let mut prior_emission = 0;
        let mut counts = [0_u8; BELT_BODY_COUNT];
        let mut before = Self {
            enabled: true,
            strokes: Arc::new(Vec::new()),
            expired: self.expired,
        };
        for hit in self.strokes.iter() {
            let slot = usize::from(hit.contact.asteroid.ordinal);
            let body = self
                .body(slot)
                .filter(|b| b.id == hit.contact.asteroid)
                .ok_or(INVALID)?;
            let prior_energy = body.energy - f64::from(counts[slot]) * f64::from(MINING_DAMAGE);
            if prior_energy <= 0.0
                || hit.owner.system != SystemId::AVENOR
                || hit.world_tick == 0
                || hit.world_tick < prior_world
                || (prior_world > 0
                    && hit.physical_tick.saturating_sub(prior_clock) > hit.world_tick - prior_world)
                || hit.physical_tick <= prior_clock
                || hit.physical_tick > hit.world_tick
                || hit.emission <= prior_emission
                || hit.item == 0
                || !hit.speed.is_finite()
                || !(0.0..=crate::flight::MAX_SPEED).contains(&hit.speed)
                || !hit.pose.orientation.is_valid_rotation()
                || !finite(hit.pose.position)
                || hit.owner.owner == 0
                || hit.owner.hull != hit.owner.owner.checked_add(1).ok_or(INVALID)?
                || hit.owner.visit < 2
            {
                return Err(INVALID);
            }
            if before.beam(hit.pose.position, hit.pose.orientation.z, MINING_RANGE)
                != Some(hit.contact)
            {
                return Err(INVALID);
            }
            if prior_energy <= f64::from(MINING_DAMAGE) {
                let pending = before
                    .bodies()
                    .into_iter()
                    .filter(|b| {
                        b.extracted_units > 0
                            && self.expired[usize::from(b.id.ordinal)]
                                .is_none_or(|(tick, _)| tick > hit.world_tick)
                    })
                    .map(|b| usize::from(b.extracted_units))
                    .sum::<usize>();
                if pending + usize::from(body.initial_units) > MAX_PENDING_ORE {
                    return Err(INVALID);
                }
            }
            Arc::make_mut(&mut before.strokes).push(hit.clone());
            counts[slot] += 1;
            prior_world = hit.world_tick;
            prior_clock = hit.physical_tick;
            prior_emission = hit.emission;
        }
        for (slot, expired) in self.expired.iter().enumerate() {
            if let Some((tick, visit)) = expired {
                let terminal = self.terminal_stroke(slot).ok_or(INVALID)?;
                if *tick <= terminal.world_tick || *visit < terminal.owner.visit {
                    return Err(INVALID);
                }
            }
        }
        Ok(())
    }
    pub(super) fn terminal_stroke(&self, slot: usize) -> Option<&MiningStroke> {
        let body = self.body(slot)?;
        (body.remaining_energy == 0.0)
            .then(|| {
                self.strokes
                    .iter()
                    .rfind(|s| usize::from(s.contact.asteroid.ordinal) == slot)
            })
            .flatten()
    }
    #[cfg(test)]
    pub(crate) fn has_history(&self) -> bool {
        !self.strokes.is_empty()
    }
}

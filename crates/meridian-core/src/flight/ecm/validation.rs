//! Independent saved-field and standard-missile effect joins.
use super::{
    Activation, ECM_FIELD_TICKS, INVALID, MAX_ACTIVATIONS, MissileEnd, SaveError, State, World,
};
use crate::flight::ProjectileSnapshot;

impl Activation {
    fn effect(&self, projectile: ProjectileSnapshot) -> Option<(u64, u64)> {
        if projectile.origin.owner.system != self.origin.owner.system
            || projectile.origin.owner.visit != self.origin.owner.visit
        {
            return None;
        }
        let birth = projectile.origin;
        if self.closed.is_some_and(|boundary| {
            birth.launched_tick > boundary.world_tick
                || (birth.launched_tick == boundary.world_tick
                    && birth.emission > boundary.after_emission)
        }) {
            return None;
        }
        if birth.launched_tick > self.world_tick
            || (birth.launched_tick == self.world_tick && birth.emission > self.after_emission)
        {
            return (birth.physical_tick >= self.physical_tick
                && birth.physical_tick - self.physical_tick < ECM_FIELD_TICKS)
                .then_some((birth.launched_tick, birth.emission));
        }
        let ended_before = projectile.ended.is_some()
            && (projectile.last_tick < self.world_tick
                || (projectile.last_tick == self.world_tick
                    && match projectile.ended {
                        Some(MissileEnd::Ecm) => projectile
                            .ecm
                            .is_some_and(|end| end.origin.activation < self.origin.activation),
                        Some(MissileEnd::Intercepted) => projectile
                            .intercepting_emission
                            .is_some_and(|emission| emission <= self.after_emission),
                        _ => true,
                    }));
        (!ended_before).then_some((self.world_tick, self.after_emission))
    }
}
impl State {
    pub(in crate::flight) fn validate_emission(
        &self,
        clock: u64,
        emission: u64,
    ) -> Result<(), SaveError> {
        if self.activations.iter().any(|activation| {
            let crosses =
                |at, after| (clock < at && emission > after) || (clock > at && emission <= after);
            crosses(activation.physical_tick, activation.after_emission)
                || activation.closed.is_some_and(|boundary| {
                    crosses(boundary.physical_tick, boundary.after_emission)
                })
        }) {
            return Err(INVALID);
        }
        Ok(())
    }
    pub(in crate::flight) fn validate_terminal(
        &self,
        world: &World,
        projectile: ProjectileSnapshot,
    ) -> Result<(), SaveError> {
        let expected = self.activations.iter().find_map(|activation| {
            activation
                .effect(projectile)
                .map(|effect| (activation, effect))
        });
        match (expected, projectile.ecm) {
            (None, None) if projectile.ended != Some(MissileEnd::Ecm) => Ok(()),
            (Some((activation, (tick, after))), Some(end))
                if projectile.ended == Some(MissileEnd::Ecm)
                    && end.origin == activation.origin
                    && end.after_emission == after
                    && projectile.last_tick == tick
                    && projectile.contact.is_none()
                    && projectile.intercepting_emission.is_none()
                    && projectile.origin.owner.owner == end.origin.owner.owner
                    && projectile.origin.owner.hull == end.origin.owner.hull
                    && tick <= world.state.tick =>
            {
                Ok(())
            }
            _ => Err(INVALID),
        }
    }
    pub(in crate::flight) fn validate(
        &self,
        world: &World,
        modules: &super::super::modular::State,
    ) -> Result<(), SaveError> {
        if self.activations.len() > MAX_ACTIVATIONS {
            return Err(INVALID);
        }
        let mut previous: Option<Activation> = None;
        for (index, activation) in self.activations.iter().enumerate() {
            if activation.origin.activation != index as u16 + 1 {
                return Err(INVALID);
            }
            activation.validate_world(world, modules)?;
            activation.validate_closed(world)?;
            if let Some(prior) = previous {
                activation.validate_after(prior)?;
            }
            previous = Some(*activation);
        }
        for projectile in world.missiles.projectiles.iter().flatten() {
            self.validate_emission(projectile.origin.physical_tick, projectile.origin.emission)?;
            self.validate_terminal(world, *projectile)?;
        }
        Ok(())
    }
}

impl Activation {
    fn validate_world(
        &self,
        world: &World,
        modules: &super::super::modular::State,
    ) -> Result<(), SaveError> {
        let owner = self.origin.owner;
        let current = world.combat_identity().ok_or(INVALID)?;
        if owner.owner != current.owner
            || owner.hull != current.hull
            || owner.visit < 2
            || owner.visit > current.visit
            || (owner.visit == current.visit && owner.system != current.system)
            || self.world_tick == 0
            || self.world_tick > world.state.tick
            || self.physical_tick > self.world_tick
            || self.physical_tick > world.combat.physical_tick()
            || self.after_emission > world.combat.emissions
            || !modules.ecm_was_fitted(world, self.item, self.world_tick, self.physical_tick, owner)
        {
            return Err(INVALID);
        }
        if self.world_tick == world.state.tick
            && (owner != current
                || world.state.mode != super::FlightMode::Flight
                || self.physical_tick != world.combat.physical_tick())
        {
            return Err(INVALID);
        }
        if owner.visit == current.visit
            && world.state.mode == super::FlightMode::Flight
            && world.state.tick - self.world_tick
                != world.combat.physical_tick() - self.physical_tick
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_after(&self, prior: Self) -> Result<(), SaveError> {
        if self.world_tick <= prior.world_tick
            || self.physical_tick < prior.physical_tick
            || self.after_emission < prior.after_emission
            || self.origin.owner.visit < prior.origin.owner.visit
        {
            return Err(INVALID);
        }
        if prior.closed.is_some_and(|boundary| {
            boundary.world_tick > self.world_tick
                || boundary.physical_tick > self.physical_tick
                || boundary.after_emission > self.after_emission
        }) {
            return Err(INVALID);
        }
        if self.origin.owner.visit == prior.origin.owner.visit
            && (self.origin.owner.system != prior.origin.owner.system
                || self.physical_tick - prior.physical_tick < ECM_FIELD_TICKS
                || self.world_tick - prior.world_tick != self.physical_tick - prior.physical_tick)
        {
            return Err(INVALID);
        }
        Ok(())
    }
}

impl Activation {
    fn validate_closed(&self, world: &World) -> Result<(), SaveError> {
        let Some(boundary) = self.closed else {
            return Ok(());
        };
        if boundary.world_tick < self.world_tick
            || boundary.world_tick > world.state.tick
            || boundary.physical_tick < self.physical_tick
            || boundary.physical_tick > world.combat.physical_tick()
            || boundary.physical_tick - self.physical_tick >= ECM_FIELD_TICKS
            || boundary.physical_tick - self.physical_tick > boundary.world_tick - self.world_tick
            || boundary.after_emission < self.after_emission
            || boundary.after_emission > world.combat.emissions
        {
            return Err(INVALID);
        }
        Ok(())
    }
}

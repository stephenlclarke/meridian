//! Explicit source validation World; no normal population, commander or save entitlement.
use super::{FlightInput, FlightStart, StepError, World, flight_pose};
use crate::mule::MuleHullSource;
use crate::station::StationError;
use crate::traffic::{MuleActorSnapshot, mule_validation::State};

impl World {
    /// Actual World-owned Mule NPC with a stationary physical SH025 observer.
    /// Source catalogue placement remains pending: this creates no ordinary population,
    /// commander profile, paid equipment, crew, blueprint, cargo or save authority.
    pub fn mule_validation_scenario(
        seed: u64,
        observer: FlightStart,
    ) -> Result<Self, StationError> {
        if seed == 0 || observer.speed != 0.0 {
            return Err(StationError::InvalidPose);
        }
        let mut world = Self::in_flight_at_original_station(observer)?;
        let data = world
            .station_data
            .as_ref()
            .ok_or(StationError::InvalidPose)?;
        let source = MuleHullSource::bundled().map_err(|_| StationError::InvalidMesh)?;
        let start = data.mule_launch_start(&source)?;
        let pose =
            flight_pose(start.position, start.orientation).ok_or(StationError::InvalidPose)?;
        let observer = flight_pose(world.state.position, world.state.orientation)
            .ok_or(StationError::InvalidPose)?;
        // Initial disjoint enclosing spheres rule out starting wholly inside either body.
        // Subsequent native observer-triangle sweeps prevent entry without surface contact.
        let delta = crate::docking::Vec3::new(
            pose.position.x - observer.position.x,
            pose.position.y - observer.position.y,
            pose.position.z - observer.position.z,
        );
        let radius = source.maximum_origin_radius() + data.player_radius() * 1.001 + 0.001;
        if !delta.dot(delta).is_finite()
            || delta.dot(delta) <= radius * radius
            || !data.mule_observer_clear(&source, pose, pose, observer)
        {
            return Err(StationError::InvalidPose);
        }
        world.mule_validation = Some(State::new(seed, source, pose, f64::from(start.speed))?);
        Ok(world)
    }
    /// One coherent immutable actor/surface publication; renderer cannot manufacture its scope.
    pub fn mule_actor_snapshot(&self) -> Option<MuleActorSnapshot> {
        self.mule_validation.as_ref().map(State::snapshot)
    }
    /// Validate immediately before adapting the publication for a render frame.
    pub fn mule_snapshot_is_current(&self, frame: &MuleActorSnapshot) -> bool {
        self.mule_validation
            .as_ref()
            .is_some_and(|state| state.is_current(frame))
            && self.state.tick == frame.world_tick()
    }
    /// Exact stationary observer pose from this World, after publication-scope validation.
    pub fn mule_observer_pose(
        &self,
        frame: &MuleActorSnapshot,
    ) -> Option<crate::docking::RigidPose> {
        if !self.mule_snapshot_is_current(frame) {
            return None;
        }
        flight_pose(self.state.position, self.state.orientation)
    }
    pub(super) fn step_mule_validation(&mut self, input: FlightInput) -> Result<(), StepError> {
        // This controlled route freezes the observer. Ordinary player flight and paid
        // interaction with Mule require their later real acquisition/contact/effect route.
        if input != FlightInput::default() || self.state.speed != 0.0 {
            return Err(StepError::InvalidMode);
        }
        let next_tick = self
            .state
            .tick
            .checked_add(1)
            .ok_or(StepError::TickOverflow)?;
        let observer = flight_pose(self.state.position, self.state.orientation)
            .ok_or(StepError::NonFiniteState)?;
        let next = self
            .mule_validation
            .as_ref()
            .ok_or(StepError::InvalidMode)?
            .advance(
                next_tick,
                self.station_data.as_ref().ok_or(StepError::InvalidMode)?,
                observer,
            )?;
        self.mule_validation = Some(next);
        self.state.tick = next_tick;
        Ok(())
    }
}

#[cfg(test)]
mod tests;

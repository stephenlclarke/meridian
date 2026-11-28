//! Ordinary one-time SH001 encounter derived from the durable G02 and navigation receipts.
use super::{FlightMode, NavigationState, StationError, StepError, SystemId, World};
use crate::docking::RigidPose;
use crate::mule::MuleHullSource;
use crate::traffic::mule_validation::State;

pub(super) struct HistoricMuleCollider {
    source: MuleHullSource,
    pose: RigidPose,
}

impl World {
    fn renas_departure(
        &self,
        mode: FlightMode,
        navigation: Option<NavigationState>,
    ) -> Option<(u64, u64)> {
        if !matches!(mode, FlightMode::Launching | FlightMode::Flight) {
            return None;
        }
        let nav = navigation?;
        let (owner, hull, berth, arrival_visit) = self
            .freight_session
            .as_ref()?
            .state
            .snapshot()
            .clue?
            .completion()
            .renas_departure_binding()?;
        let visit = arrival_visit.checked_add(1)?;
        let current = nav.snapshot(0);
        // The completion visit is the actual dock context, not the receiving NPC.
        // A launch advances visit exactly once. Any later dock or jump consumes this window.
        (owner == nav.owner().seed()
            && hull == nav.ship().ordinal()
            && current.system == SystemId::RENAS
            && current.visit == visit
            && nav.dock_sequence() == berth
            && nav.dock().is_none())
        .then_some((owner, visit))
    }

    pub(super) fn prepare_renas_mule(
        &self,
        mode: FlightMode,
        navigation: Option<NavigationState>,
    ) -> Result<Option<State>, StationError> {
        let Some((owner, visit)) = self.renas_departure(mode, navigation) else {
            return Ok(None);
        };
        let data = self
            .station_data
            .as_ref()
            .ok_or(StationError::InvalidPose)?;
        let source = MuleHullSource::bundled().map_err(|_| StationError::InvalidMesh)?;
        let pose = data.renas_mule_berth(&source)?;
        State::renas_encounter(owner, visit, source, pose).map(Some)
    }

    pub(super) fn validate_renas_mule_launch(
        &self,
        actor: &State,
        launch: RigidPose,
    ) -> Result<(), StepError> {
        let data = self.station_data.as_ref().ok_or(StepError::UnsafeLaunch)?;
        if !data.mule_ordinary_player_clear(actor.source(), actor.pose(), launch, launch, 0.0)
            || !self.traffic.player_motion_clear(
                SystemId::RENAS,
                actor.pose().position,
                actor.pose().position,
                actor.source().maximum_origin_radius() * 2.0,
            )
        {
            return Err(StepError::UnsafeLaunch);
        }
        Ok(())
    }

    fn renas_restored_player_clear(&self, actor: &State) -> bool {
        super::flight_pose(self.state.position, self.state.orientation).is_some_and(|pose| {
            self.station_data.as_ref().is_some_and(|station| {
                station.mule_ordinary_player_clear(actor.source(), actor.pose(), pose, pose, 0.0)
            })
        })
    }

    pub(super) fn restore_renas_mule(&mut self) -> Result<(), StationError> {
        let mut actor = self.prepare_renas_mule(self.state.mode, self.navigation)?;
        if let Some(actor) = &mut actor {
            // Older v17 saves had no collider at the berth. Preserve an
            // intersecting saved pose and let ordinary flight clear it first.
            actor.defer_renas(!self.renas_restored_player_clear(actor));
        }
        self.mule_renas = actor;
        Ok(())
    }

    pub(super) fn refresh_deferred_renas_mule(&mut self) {
        let Some(actor) = self
            .mule_renas
            .as_ref()
            .filter(|actor| actor.renas_is_deferred())
        else {
            return;
        };
        if self
            .renas_departure(self.state.mode, self.navigation)
            .is_some()
            && self.renas_restored_player_clear(actor)
            && let Some(actor) = self.mule_renas.as_mut()
        {
            actor.defer_renas(false);
        }
    }

    pub(super) fn active_renas_mule(&self) -> Option<&State> {
        self.renas_departure(self.state.mode, self.navigation)?;
        self.mule_renas
            .as_ref()
            .filter(|actor| !actor.renas_is_deferred())
    }

    pub(super) fn traffic_and_renas_mule_motion_clear(
        &self,
        before: RigidPose,
        after: RigidPose,
        angular_travel: f64,
    ) -> bool {
        self.traffic_motion_clear(before, after)
            && self.active_renas_mule().is_none_or(|actor| {
                self.station_data.as_ref().is_some_and(|station| {
                    station.mule_ordinary_player_clear(
                        actor.source(),
                        actor.pose(),
                        before,
                        after,
                        angular_travel,
                    )
                })
            })
    }

    pub(super) fn historic_renas_mule_collider(
        &self,
        system: SystemId,
        visit: u64,
    ) -> Result<Option<HistoricMuleCollider>, StationError> {
        let Some((owner, hull, _, arrival_visit)) = self
            .freight_session
            .as_ref()
            .and_then(|session| session.state.snapshot().clue)
            .and_then(|clue| clue.completion().renas_departure_binding())
        else {
            return Ok(None);
        };
        let Some(identity) = self.combat_identity() else {
            return Ok(None);
        };
        if system != SystemId::RENAS
            || arrival_visit.checked_add(1) != Some(visit)
            || owner != identity.owner
            || hull != identity.hull
        {
            return Ok(None);
        }
        let data = self
            .station_data
            .as_ref()
            .ok_or(StationError::InvalidPose)?;
        let source = MuleHullSource::bundled().map_err(|_| StationError::InvalidMesh)?;
        let pose = data.renas_mule_berth(&source)?;
        Ok(Some(HistoricMuleCollider { source, pose }))
    }

    pub(super) fn historic_renas_mule_motion_blocked(
        &self,
        collider: Option<&HistoricMuleCollider>,
        before: RigidPose,
        after: RigidPose,
        angular_travel: f64,
    ) -> Result<bool, StationError> {
        let Some(collider) = collider else {
            return Ok(false);
        };
        let data = self
            .station_data
            .as_ref()
            .ok_or(StationError::InvalidPose)?;
        // A genuine old overlapping save defers the collider until it is physically clear.
        // A refused attempt must start at a clear pose before crossing the exact fixed hull.
        Ok(
            data.mule_ordinary_player_clear(&collider.source, collider.pose, before, before, 0.0)
                && !data.mule_ordinary_player_clear(
                    &collider.source,
                    collider.pose,
                    before,
                    after,
                    angular_travel,
                ),
        )
    }
}

#[cfg(test)]
mod tests;

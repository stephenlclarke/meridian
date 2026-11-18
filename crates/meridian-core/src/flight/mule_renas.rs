//! Ordinary one-time SH001 encounter derived from the durable G02 and navigation receipts.
use super::{FlightMode, NavigationState, StationError, StepError, SystemId, World};
use crate::docking::RigidPose;
use crate::mule::MuleHullSource;
use crate::traffic::mule_validation::State;

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
                actor.source().maximum_origin_radius(),
            )
        {
            return Err(StepError::UnsafeLaunch);
        }
        Ok(())
    }

    pub(super) fn active_renas_mule(&self) -> Option<&State> {
        self.renas_departure(self.state.mode, self.navigation)?;
        self.mule_renas.as_ref()
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
}

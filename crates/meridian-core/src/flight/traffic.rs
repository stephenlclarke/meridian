//! Actual World admission, physical queries and staging for finite local NPCs.
use super::{
    FireError, FlightMode, NavigationState, PulseContact, PulseShot, Snapshot, StepError, World,
    flight_pose,
};
use crate::docking::RigidPose;
use crate::station_services::StationMarkets;
use crate::traffic::{self, BeamSource, TrafficContact, TrafficSnapshot};
impl World {
    pub fn traffic_snapshot(&self) -> Option<TrafficSnapshot> {
        self.traffic.snapshot(self.combat_identity()?.system)
    }
    pub(super) fn traffic_beam_contact(
        &self,
        pose: RigidPose,
        range: f64,
    ) -> Option<TrafficContact> {
        self.traffic.beam(
            self.station_data.as_ref()?,
            self.combat_identity()?.system,
            pose.position,
            pose.orientation.z,
            range,
        )
    }
    pub(super) fn traffic_motion_clear(&self, before: RigidPose, after: RigidPose) -> bool {
        let Some(identity) = self.combat_identity() else {
            return true;
        };
        self.station_data.as_ref().is_some_and(|data| {
            self.traffic.player_motion_clear(
                identity.system,
                before.position,
                after.position,
                data.player_clearance_radius(),
            )
        })
    }
    pub(super) fn prepare_traffic_step(
        &self,
        next: Snapshot,
        navigation: Option<NavigationState>,
    ) -> Result<Option<(traffic::State, StationMarkets)>, StepError> {
        let Some(markets) = self.station_markets.as_ref() else {
            return Ok(None);
        };
        let Some(current) = self.navigation else {
            return Ok(None);
        };
        let next_nav = navigation.ok_or(StepError::NonFiniteState)?;
        let fuel = self
            .commander_account
            .as_ref()
            .ok_or(StepError::NonFiniteState)?
            .fuel_tenths();
        let current = current.snapshot(fuel);
        let future = next_nav.snapshot(fuel);
        if self.state.mode == FlightMode::Docked
            || next.mode == FlightMode::Docked
            || current.system != future.system
            || current.travel.is_some()
            || future.travel.is_some()
        {
            return Ok(Some((self.traffic.clone(), markets.traffic_candidate())));
        }
        let data = self
            .station_data
            .as_ref()
            .ok_or(StepError::NonFiniteState)?;
        let before = flight_pose(self.state.position, self.state.orientation)
            .ok_or(StepError::NonFiniteState)?;
        let after =
            flight_pose(next.position, next.orientation).ok_or(StepError::NonFiniteState)?;
        let protected = self.flight_reception.occupancy().1 > 0
            || self.navigation_trade.occupancy().1 > 0
            || self.practice.is_some_and(|p| !p.reception_is_terminal())
            || self
                .mission_snapshot()
                .is_some_and(|s| s.freight.held_tonnes > 0);
        let celestial = self.celestial_snapshot();
        let celestial_clear = |a: crate::docking::Vec3, b: crate::docking::Vec3, radius: f64| {
            celestial.is_none_or(|scene| {
                [scene.planet, scene.sun].into_iter().all(|body| {
                    !body.blocks_motion(
                        [a.x as f32, a.y as f32, a.z as f32],
                        [b.x as f32, b.y as f32, b.z as f32],
                        radius,
                    )
                })
            })
        };
        let beam_clear =
            |origin: crate::docking::Vec3, direction: crate::docking::Vec3, range: f64| {
                let yaw = direction.x.atan2(direction.z);
                let (s, c) = yaw.sin_cos();
                let ray = RigidPose {
                    position: origin,
                    orientation: crate::docking::Basis3 {
                        x: crate::docking::Vec3::new(c, 0.0, -s),
                        y: crate::docking::Vec3::new(0.0, 1.0, 0.0),
                        z: direction,
                    },
                };
                self.nearest_missile(ray, range).is_none()
                    && celestial_clear(
                        origin,
                        crate::docking::Vec3::new(
                            origin.x + direction.x * range,
                            origin.y + direction.y * range,
                            origin.z + direction.z * range,
                        ),
                        0.0,
                    )
            };
        self.traffic
            .prepare_step(
                traffic::TickContext {
                    system: current.system,
                    data,
                    player_before: before,
                    player_after: after,
                    world_tick: next.tick,
                    protected,
                    celestial_clear: &celestial_clear,
                    beam_clear: &beam_clear,
                },
                markets,
            )
            .map(Some)
            .map_err(|_| StepError::NonFiniteState)
    }
    pub(super) fn prepare_traffic_pulse(
        &self,
        shot: PulseShot,
        emission: u64,
    ) -> Result<traffic::State, FireError> {
        let PulseContact::Npc { id, distance } = shot.contact else {
            return Ok(self.traffic.clone());
        };
        if self.practice.is_some_and(|p| !p.reception_is_terminal()) {
            return Err(FireError::ProtectedTraffic);
        }
        let owner = self.combat_identity().ok_or(FireError::NoCommander)?;
        let contact = TrafficContact {
            id,
            distance,
            point: shot.end,
        };
        self.traffic
            .prepare_player_hit(
                self.station_data
                    .as_ref()
                    .ok_or(FireError::InvalidGeometry)?,
                traffic::PlayerBeamInput {
                    system: owner.system,
                    source: BeamSource::Player {
                        owner: owner.owner,
                        hull: owner.hull,
                        item: shot.fitted_item_id,
                        emission,
                    },
                    origin: shot.origin,
                    direction: shot.direction,
                    contact,
                    world_tick: shot.tick,
                },
            )
            .map_err(|error| match error {
                traffic::TrafficError::ReceiptCapacity => FireError::SequenceExhausted,
                _ => FireError::InvalidGeometry,
            })
    }
}

#[cfg(test)]
mod tests;

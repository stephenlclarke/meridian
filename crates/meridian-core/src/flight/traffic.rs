//! Actual World admission, physical queries and staging for finite local NPCs.
use super::{
    FireError, FlightMode, NavigationState, PulseContact, PulseShot, Snapshot, StepError, World,
    flight_pose,
};
use crate::docking::RigidPose;
use crate::station_services::StationMarkets;
use crate::traffic::{self, BeamSource, TrafficContact, TrafficSnapshot};
impl World {
    pub(super) fn validate_traffic_player_scenes(&self) -> Result<(), crate::save::SaveError> {
        let invalid = crate::save::SaveError::InvalidState("NPC player scene receipt");
        for shot in self.traffic.beam_receipts() {
            match shot.source {
                BeamSource::Npc(_) if shot.weapon != super::OrdinaryWeapon::FrontPulseLaser => {
                    return Err(invalid);
                }
                BeamSource::Player { item, .. }
                    if self.laser_weapon_at_tick(item, shot.world_tick) != Some(shot.weapon) =>
                {
                    return Err(invalid);
                }
                _ => {}
            }
            if !self.mining_segment_at_step(
                shot.victim.system(),
                shot.world_tick,
                shot.origin,
                shot.point,
                0.0,
            ) || !self.traffic_beam_celestial_clear(
                shot.victim.system(),
                shot.world_tick,
                shot.origin,
                shot.point,
            ) {
                return Err(invalid);
            }
        }
        for hit in self.traffic.player_hits() {
            self.missiles.validate_npc_beam_scene(hit)?;
            let system = self
                .navigation_catalogue
                .as_ref()
                .ok_or(invalid)?
                .system(hit.attacker.system())
                .map_err(|_| invalid)?;
            let anchor = self.state.station.ok_or(invalid)?.pose.position;
            let scene = crate::celestial::scene(system, hit.visit, hit.world_tick, anchor);
            let a = hit.source_pose.position;
            let b = hit.point;
            if !self.mining_segment_at_step(hit.attacker.system(), hit.world_tick, a, b, 0.0)
                || [scene.planet, scene.sun].into_iter().any(|body| {
                    body.blocks_motion(
                        [a.x as f32, a.y as f32, a.z as f32],
                        [b.x as f32, b.y as f32, b.z as f32],
                        0.0,
                    )
                })
                || (hit.world_tick == self.state.tick
                    && super::flight_pose(self.state.position, self.state.orientation)
                        != Some(hit.player_pose))
            {
                return Err(invalid);
            }
        }
        Ok(())
    }
    /// The same real planet/sun segment guard at command and retained-receipt boundaries.
    /// Scene positions are immutable station-local geometry; visit affects identity only.
    pub(super) fn traffic_beam_celestial_clear(
        &self,
        system: crate::navigation::SystemId,
        tick: u64,
        origin: crate::docking::Vec3,
        point: crate::docking::Vec3,
    ) -> bool {
        let Some(catalogue) = self.navigation_catalogue.as_ref() else {
            return false;
        };
        let Ok(system) = catalogue.system(system) else {
            return false;
        };
        let Some(data) = self.station_data.as_ref() else {
            return false;
        };
        let scene = crate::celestial::scene(system, 0, tick, data.station_pose().pose.position);
        [scene.planet, scene.sun].into_iter().all(|body| {
            !body.blocks_motion(
                [origin.x as f32, origin.y as f32, origin.z as f32],
                [point.x as f32, point.y as f32, point.z as f32],
                0.0,
            )
        })
    }
    /// Inspect actual local destroyed ordinary trade custody. No owned cargo, pod or salvage
    /// permission can be returned by this read-only boundary.
    pub fn wreck_manifest(
        &self,
        source: traffic::NpcId,
    ) -> Result<traffic::WreckManifest, traffic::WreckManifestError> {
        use traffic::WreckManifestError;
        let owner = self
            .combat_identity()
            .ok_or(WreckManifestError::NoCommander)?;
        if source.profile() != owner.owner {
            return Err(WreckManifestError::ForeignActor);
        }
        if source.system() != owner.system {
            return Err(WreckManifestError::OutsideSystem);
        }
        self.traffic.wreck_manifest(source)
    }
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
        prepared: &super::missiles::PreparedStep,
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
            return Ok(Some((
                prepared.traffic.clone(),
                markets.traffic_candidate(),
            )));
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
            self.mining_segment_at_step(current.system, next.tick, a, b, radius)
                && celestial.is_none_or(|scene| {
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
                prepared
                    .missiles
                    .nearest_beam_body(current.visit, ray, range)
                    .is_none()
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
        let (mut traffic, markets) = prepared
            .traffic
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
                    player_target: if self.damage.enabled() && prepared.pulse.alive() && !protected
                    {
                        Some(traffic::PlayerTarget {
                            hull: self
                                .combat_identity()
                                .ok_or(StepError::NonFiniteState)?
                                .hull,
                            visit: current.visit,
                            physical_tick: prepared.combat.physical_tick(),
                            cargo: self.reserved_service_projection().is_ok_and(|p| {
                                p.food_tonnes > 0 || p.other_commodities.iter().any(|n| *n > 0)
                            }),
                        })
                    } else {
                        None
                    },
                },
                markets,
            )
            .map_err(|_| StepError::NonFiniteState)?;
        if traffic.player_hits().len() > self.traffic.player_hits().len() {
            traffic.bind_player_hit_scene(
                self.traffic.player_hits().len(),
                prepared.missiles.beam_scene(current.visit),
            );
        }
        Ok(Some((traffic, markets)))
    }
    pub(super) fn prepare_traffic_pulse(
        &self,
        shot: PulseShot,
        emission: u64,
    ) -> Result<traffic::State, FireError> {
        let PulseContact::Npc { id, distance } = shot.contact else {
            return Ok(self.traffic.clone());
        };
        if !self.traffic_beam_celestial_clear(id.system(), shot.tick, shot.origin, shot.end) {
            return Err(FireError::InvalidGeometry);
        }
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
                    weapon: self
                        .fitted_laser_item(shot.fitted_item_id)
                        .ok_or(FireError::NoFittedFrontLaser)?
                        .1,
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

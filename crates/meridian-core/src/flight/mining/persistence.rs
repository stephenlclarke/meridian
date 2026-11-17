//! Closed work/source/resource boundary. Loading never mines or collects anything.
use super::*;
use crate::save::{MAX_SAVE_BYTES, SaveError, Writer};
impl World {
    pub(super) fn check_mining_capacity(
        &self,
        mining: &crate::mining::State,
        damage: &super::super::damage::State,
    ) -> Result<(), SaveError> {
        let mut old = Writer::new();
        self.mining.encode(&mut old);
        self.damage.encode(&mut old);
        let mut new = Writer::new();
        mining.encode(&mut new);
        damage.encode(&mut new);
        let size = self
            .encoded_current_size()?
            .checked_sub(old.bytes.len())
            .and_then(|n| n.checked_add(new.bytes.len()))
            .ok_or(SaveError::TooLarge)?;
        if size > MAX_SAVE_BYTES {
            return Err(SaveError::TooLarge);
        }
        Ok(())
    }
    pub(in crate::flight) fn validate_mining_work(
        &self,
        modules: &super::super::modular::State,
    ) -> Result<(), SaveError> {
        let invalid = SaveError::InvalidState("mining source, contact or discharge provenance");
        self.mining.validate()?;
        let owner = self.combat_identity().ok_or(invalid)?;
        let data = self.station_data.as_ref().ok_or(invalid)?;
        if self
            .mining
            .expiry_receipts()
            .any(|(tick, visit)| tick > self.state.tick || visit >= owner.visit)
        {
            return Err(invalid);
        }
        for hit in self.mining.strokes() {
            if hit.owner.owner != owner.owner
                || hit.owner.hull != owner.hull
                || hit.owner.visit > owner.visit
                || hit.world_tick > self.state.tick
                || hit.physical_tick > self.combat.physical_tick()
                || hit.emission > self.combat.emissions
                || self
                    .commander_account
                    .as_ref()
                    .is_none_or(|a| hit.account_revision > a.revision())
                || !modules.mining_was_fitted(hit)
                || hit.peers.system != hit.owner.system
                || hit.peers.world_tick != hit.world_tick
                || hit.peers.stage != crate::traffic::SceneStage::Command(hit.emission)
                || !self.mining_motion_at_source(hit.pose, hit.emission)
                || (hit.world_tick == self.state.tick
                    && (flight_pose(self.state.position, self.state.orientation) != Some(hit.pose)
                        || self.state.speed != hit.speed))
            {
                return Err(invalid);
            }
            self.traffic
                .validate_missile_scene(&hit.peers, self.state.tick, data)?;
            if hit
                .peers
                .contact(
                    data,
                    hit.pose.position,
                    hit.pose.orientation.z,
                    MINING_RANGE,
                )
                .is_some_and(|c| c.distance <= hit.contact.distance)
                || data
                    .beam_block_distance(
                        data.station_pose(),
                        hit.pose.position,
                        hit.pose.orientation.z,
                        MINING_RANGE,
                    )
                    .map_err(|_| invalid)?
                    .is_some_and(|d| d <= hit.contact.distance)
            {
                return Err(invalid);
            }
            let system = self
                .navigation_catalogue
                .as_ref()
                .ok_or(invalid)?
                .system(hit.owner.system)
                .map_err(|_| invalid)?;
            let scene = crate::celestial::scene(
                system,
                hit.owner.visit,
                hit.world_tick,
                data.station_pose().pose.position,
            );
            let a = hit.pose.position;
            let b = hit.contact.point;
            if [scene.planet, scene.sun].into_iter().any(|body| {
                body.blocks_motion(
                    [a.x as f32, a.y as f32, a.z as f32],
                    [b.x as f32, b.y as f32, b.z as f32],
                    0.0,
                )
            }) {
                return Err(invalid);
            }
            self.missiles
                .validate_retained_beam(super::super::missiles::RetainedBeam {
                    system: hit.owner.system,
                    visit: hit.owner.visit,
                    world_tick: hit.world_tick,
                    physical_tick: hit.physical_tick,
                    command_emission: Some(hit.emission),
                    ray: hit.pose,
                    point: hit.contact.point,
                    bodies: &hit.missiles,
                })?;
        }
        // With no admitted Fuel Scoops producer, there is deliberately no ore cargo credit,
        // market/revision adjustment or collected receipt to synthesize during load.
        if self.state.mode != FlightMode::Docked
            && !self.mining_motion_clear(
                flight_pose(self.state.position, self.state.orientation).ok_or(invalid)?,
                flight_pose(self.state.position, self.state.orientation).ok_or(invalid)?,
                0.0,
            )
        {
            return Err(invalid);
        }
        Ok(())
    }
    fn mining_motion_at_source(&self, pose: RigidPose, emission: u64) -> bool {
        // Historical native contact lies outside the actual source mesh. A later terminal
        // tombstone must not legitimize having fired from inside an intact original rock.
        let original = self.mining.before_emission(emission);
        let Some(data) = &self.station_data else {
            return false;
        };
        original
            .bodies()
            .into_iter()
            .filter(|body| body.remaining_energy > 0.0)
            .all(|body| {
                !body.contains_point(pose.position)
                    && data.asteroid_motion_clear(pose, pose, 0.0, body)
            })
    }
}

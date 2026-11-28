//! Separate NPC -> player identity, native contact and retained scene evidence.
use super::{
    Actor, Arc, BeamSource, INVALID, MAX_TRAFFIC_ACTORS, NpcPhase, NpcPlayerHit,
    OriginalStationData, PlayerTarget, RigidPose, SaveError, SceneStage, State, TrafficBeam,
    TrafficError, Vec3, add, finite, length, scale, sub, system_index, unit,
};
pub(super) struct PlayerEmission<'a> {
    pub(super) data: &'a OriginalStationData,
    pub(super) player: RigidPose,
    pub(super) target: PlayerTarget,
    pub(super) world_tick: u64,
    pub(super) beam_clear: &'a dyn Fn(Vec3, Vec3, f64) -> bool,
}

impl State {
    pub(super) fn validate_resource_replay(&self, actor: Actor) -> Result<(), SaveError> {
        let expected = self.resources_at(
            actor,
            actor.resource_clock,
            u64::MAX,
            SceneStage::Command(u64::MAX),
        )?;
        if expected != actor.resources {
            return Err(INVALID);
        }
        Ok(())
    }
    pub(crate) fn beam_receipts(&self) -> &[TrafficBeam] {
        &self.shots
    }
    pub(crate) fn player_hits(&self) -> &[NpcPlayerHit] {
        &self.player_hits
    }
    pub(crate) fn bind_player_hit_scene(
        &mut self,
        first: usize,
        missiles: Vec<(crate::ProjectileId, RigidPose)>,
    ) {
        if self.player_hits.len() == first {
            return;
        }
        let missiles = Arc::new(missiles);
        for hit in Arc::make_mut(&mut self.player_hits).iter_mut().skip(first) {
            hit.missiles = missiles.clone();
        }
    }
    pub(crate) fn receipt_count(&self) -> usize {
        self.shots.len() + self.player_hits.len() + self.missile_hits.len()
    }
    pub(crate) fn proof_grows_from(&self, old: &Self) -> bool {
        self.receipt_count() > old.receipt_count()
            || self.emp_events.len() > old.emp_events.len()
            || self.actors.iter().zip(&old.actors).any(|(a, b)| {
                (a.is_some() && b.is_none())
                    || a.is_some_and(|a| a.berth.is_some()) && b.is_some_and(|b| b.berth.is_none())
            })
    }
    pub(super) fn player_goal(
        &self,
        actor: Actor,
        player: Vec3,
        target: PlayerTarget,
    ) -> Option<Vec3> {
        Self::player_goal_in(
            actor,
            &self.actors,
            player,
            target,
            &self.shots,
            self.missile_hits
                .iter()
                .any(|h| h.victim == actor.id && h.origin.owner.hull == target.hull),
        )
    }
    pub(super) fn player_goal_in(
        actor: Actor,
        actors: &[Option<Actor>; MAX_TRAFFIC_ACTORS],
        player: Vec3,
        target: PlayerTarget,
        shots: &[TrafficBeam],
        missile_aggression: bool,
    ) -> Option<Vec3> {
        if actor.id.serial != 2
            || actor.phase != NpcPhase::Hunting
            || actor.resources.emp_disabled()
            || actor.resources.energy < 9500
            || length(sub(player, actor.position)) > 12000.0
        {
            return None;
        }
        let retaliating = missile_aggression
            || shots.iter().any(|s| {
                s.victim == actor.id
                    && matches!(s.source, BeamSource::Player { hull, .. } if hull == target.hull)
            });
        let closer = super::motion::target(actor, actors).is_none_or(|trader| {
            length(sub(player, actor.position)) < length(sub(trader.position, actor.position))
        });
        (retaliating || (target.cargo && closer)).then_some(player)
    }
    pub(super) fn fire_at_player(
        &mut self,
        index: usize,
        mut actor: Actor,
        emission: PlayerEmission<'_>,
    ) -> Result<(), TrafficError> {
        let PlayerEmission {
            data,
            player,
            target,
            world_tick,
            beam_clear,
        } = emission;
        let ray = actor.pose();
        let Some(point) = data
            .player_planar_aimpoint(player, ray.position)
            .map_err(|_| TrafficError::InvalidState)?
        else {
            return Ok(());
        };
        if unit(sub(point, ray.position)).is_none_or(|aim| aim.dot(ray.orientation.z) < 0.995) {
            return Ok(());
        }
        let Some(distance) = data
            .player_beam_distance(player, ray.position, ray.orientation.z, 6000.0)
            .map_err(|_| TrafficError::InvalidState)?
        else {
            return Ok(());
        };
        if data
            .beam_block_distance(data.station_pose(), ray.position, ray.orientation.z, 6000.0)
            .map_err(|_| TrafficError::InvalidState)?
            .is_some_and(|d| d <= distance)
            || self
                .beam_except(
                    data,
                    actor.id.system,
                    ray.position,
                    ray.orientation.z,
                    6000.0,
                    Some(actor.id),
                )
                .is_some_and(|c| c.distance <= distance)
            || !beam_clear(ray.position, ray.orientation.z, distance)
        {
            return Ok(());
        }
        let hit = NpcPlayerHit {
            attacker: actor.id,
            player_hull: target.hull,
            visit: target.visit,
            world_tick,
            physical_tick: target.physical_tick,
            local_tick: self.clocks
                [system_index(actor.id.system).ok_or(TrafficError::InvalidState)?],
            source_pose: ray,
            player_pose: player,
            point: add(ray.position, scale(ray.orientation.z, distance)),
            front: player.orientation.z.dot(sub(ray.position, player.position)) >= 0.0,
            peers: self.actors.map(|a| {
                a.filter(|a| a.solid() && a.id.system == actor.id.system && a.id != actor.id)
                    .map(|a| (a.id, a.pose()))
            }),
            missiles: Arc::new(Vec::new()),
        };
        actor.resources.fire()?;
        self.actors[index] = Some(actor);
        Arc::make_mut(&mut self.player_hits).push(hit);
        Ok(())
    }
    pub(super) fn validate_player_hits(
        &self,
        world_tick: u64,
        scene: &OriginalStationData,
        issue_berth: Option<u64>,
    ) -> Result<(), SaveError> {
        let mut prior_world = 0;
        let mut prior_physical = 0;
        for hit in self.player_hits.iter() {
            let data = scene.player_geometry_at(hit.visit, issue_berth);
            let index = self.actor_index(hit.attacker).ok_or(INVALID)?;
            let actor = self.actors[index].ok_or(INVALID)?;
            let si = system_index(hit.attacker.system).ok_or(INVALID)?;
            let ray = hit.source_pose;
            if hit.attacker.serial != 2
                || hit.player_hull != self.seed.checked_add(1).ok_or(INVALID)?
                || hit.visit < 2
                || hit.world_tick < actor.born_world_tick
                || hit.world_tick > world_tick
                || hit.world_tick < prior_world
                || hit.physical_tick == 0
                || hit.physical_tick < prior_physical
                || hit.physical_tick > hit.world_tick
                || hit.local_tick < actor.birth_clock
                || hit.local_tick > self.clocks[si]
                || hit.local_tick > hit.physical_tick
                || (hit.world_tick == world_tick && actor.pose() != hit.source_pose)
                || !ray.orientation.is_valid_rotation()
                || !hit.player_pose.orientation.is_valid_rotation()
                || !finite(ray.position)
                || !finite(hit.player_pose.position)
                || !finite(hit.point)
                || data
                    .player_planar_aimpoint(hit.player_pose, ray.position)
                    .map_err(|_| INVALID)?
                    .and_then(|point| unit(sub(point, ray.position)))
                    .is_none_or(|aim| aim.dot(ray.orientation.z) < 0.995)
            {
                return Err(INVALID);
            }
            let distance = data
                .player_beam_distance(hit.player_pose, ray.position, ray.orientation.z, 6000.0)
                .map_err(|_| INVALID)?
                .ok_or(INVALID)?;
            if length(sub(
                add(ray.position, scale(ray.orientation.z, distance)),
                hit.point,
            )) > 1e-6
                || hit.front
                    != (hit
                        .player_pose
                        .orientation
                        .z
                        .dot(sub(ray.position, hit.player_pose.position))
                        >= 0.0)
                || data
                    .beam_block_distance(
                        data.station_pose(),
                        ray.position,
                        ray.orientation.z,
                        6000.0,
                    )
                    .map_err(|_| INVALID)?
                    .is_some_and(|d| d <= distance)
            {
                return Err(INVALID);
            }
            for (slot, peer) in hit.peers.iter().enumerate() {
                if let Some((id, pose)) = peer {
                    if *id == hit.attacker
                        || id.system != hit.attacker.system
                        || self.actor_index(*id) != Some(slot)
                        || !finite(pose.position)
                        || !pose.orientation.is_valid_rotation()
                        || scene
                            .npc_geometry()
                            .player_beam_distance(*pose, ray.position, ray.orientation.z, 6000.0)
                            .map_err(|_| INVALID)?
                            .is_some_and(|d| d <= distance)
                    {
                        return Err(INVALID);
                    }
                } else if self.actors[slot].is_some_and(|a| {
                    a.id != hit.attacker
                        && a.id.system == hit.attacker.system
                        && a.born_world_tick <= hit.world_tick
                        && a.sold_world_tick.is_none_or(|t| t > hit.world_tick)
                }) {
                    return Err(INVALID);
                }
            }
            prior_world = hit.world_tick;
            prior_physical = hit.physical_tick;
        }
        Ok(())
    }
}

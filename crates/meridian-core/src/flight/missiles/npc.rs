//! Typed NPC targeting and retained native final-segment replay, never a practice proxy.
use super::*;
use crate::save::{Reader, SaveError, Writer};
use crate::traffic::{MissileScene, NpcId, SceneStage};
const INVALID: SaveError =
    SaveError::InvalidState("NPC missile launch, native contact or reciprocal damage");
#[derive(Debug, Clone, PartialEq)]
pub(super) struct NpcLaunch {
    pub(super) scene: MissileScene,
    pub(super) dock_sequence: u64,
    pub(super) loss: Option<MissileScene>,
}
impl NpcLaunch {
    pub(super) fn encode(&self, w: &mut Writer) {
        self.scene.encode(w);
        w.u64(self.dock_sequence);
        w.option(self.loss.as_ref(), |w, s| s.encode(w));
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            scene: MissileScene::decode(r)?,
            dock_sequence: r.u64()?,
            loss: r.option(MissileScene::decode)?,
        })
    }
}
impl World {
    pub(super) fn npc_missile_protected(&self) -> bool {
        self.flight_reception.occupancy().1 > 0
            || self.navigation_trade.occupancy().1 > 0
            || self.practice.is_some_and(|p| !p.reception_is_terminal())
            || self
                .mission_snapshot()
                .is_some_and(|s| s.freight.held_tonnes > 0)
    }
    pub(super) fn resolve_missile_target(
        &self,
        target: MissileTarget,
        traffic: &crate::traffic::State,
        practice: Option<super::super::practice::Practice>,
    ) -> Option<Vec3> {
        match target {
            MissileTarget::Practice(id) => practice
                .and_then(|p| p.missile_target(id))
                .map(|p| p.position),
            MissileTarget::Npc(id) => traffic
                .missile_scene(
                    id.system(),
                    self.state.tick,
                    SceneStage::Command(self.combat.emissions),
                )
                .target(id)
                .map(|b| b.pose.position),
        }
    }
    pub fn request_npc_missile_lock(
        &mut self,
        target: Option<NpcId>,
        tick: u64,
    ) -> Result<(), MissileError> {
        let owner = self.missile_command(tick)?;
        if target.is_some() && self.npc_missile_protected() {
            return Err(MissileError::ProtectedTraffic);
        }
        if target.is_some_and(|id| {
            id.profile() != owner.owner
                || id.system() != owner.system
                || self
                    .resolve_missile_target(MissileTarget::Npc(id), &self.traffic, self.practice)
                    .is_none()
        }) {
            return Err(MissileError::NoLiveLock);
        }
        self.missiles.lock = target.map(MissileTarget::Npc);
        Ok(())
    }
    pub(in crate::flight) fn validate_npc_missile_receipts(&self) -> Result<(), SaveError> {
        for p in self.missiles.projectiles.iter().flatten() {
            let Some(MissileContact::Npc(id)) = p.contact else {
                continue;
            };
            let m = self.missiles.motions[usize::from(p.origin.projectile.ordinal)]
                .as_ref()
                .ok_or(INVALID)?;
            let scene = m.npc_scene.as_ref().ok_or(INVALID)?;
            let body = scene.bodies.iter().find(|b| b.id == id).ok_or(INVALID)?;
            let hit = self
                .traffic
                .missile_hits()
                .iter()
                .filter(|h| h.origin.projectile == p.origin.projectile)
                .collect::<Vec<_>>();
            let live = body.phase != crate::traffic::NpcPhase::Destroyed;
            if hit.len() != usize::from(live) {
                return Err(INVALID);
            }
            if let Some(h) = hit.first() {
                if h.origin != p.origin
                    || h.victim != id
                    || h.point != p.position
                    || h.world_tick != p.last_tick
                    || h.local_tick != scene.local_tick
                    || h.victim_pose != body.pose
                    || h.front
                        != (sub(p.position, body.pose.position).dot(body.pose.orientation.z) >= 0.0)
                {
                    return Err(INVALID);
                }
            }
        }
        for h in self.traffic.missile_hits() {
            if self.missiles.projectiles.iter().flatten().all(|p| {
                p.origin != h.origin
                    || p.contact != Some(MissileContact::Npc(h.victim))
                    || p.ended != Some(MissileEnd::Impact)
            }) {
                return Err(INVALID);
            }
        }
        Ok(())
    }
}
/// Same native scene query for runtime and retained segment replay. Station/player/projectiles
/// already occupy best; ties preserve their existing order before a newly added NPC surface.
pub(super) fn choose_scene(
    world: &World,
    p: &ProjectileSnapshot,
    scene: &MissileScene,
    ray: crate::docking::RigidPose,
    travel: f64,
    best: &mut Option<(f64, MissileContact)>,
) -> Result<(), StepError> {
    let data = world
        .station_data
        .as_ref()
        .ok_or(StepError::NonFiniteState)?;
    if let Some(c) = scene.contact(data, ray.position, ray.orientation.z, travel) {
        choose(best, c.distance, MissileContact::Npc(c.id));
    }
    let system = world
        .navigation_catalogue
        .as_ref()
        .ok_or(StepError::NonFiniteState)?
        .system(p.origin.owner.system)
        .map_err(|_| StepError::NonFiniteState)?;
    let cosmic = crate::celestial::scene(
        system,
        p.origin.owner.visit,
        scene.world_tick,
        world
            .state
            .station
            .ok_or(StepError::NonFiniteState)?
            .pose
            .position,
    );
    for body in [cosmic.planet, cosmic.sun] {
        if let Some(d) = sphere_distance(
            ray.position,
            ray.orientation.z,
            body.position,
            body.radius,
            travel,
        ) {
            choose(best, d, MissileContact::Celestial(body.id));
        }
    }
    Ok(())
}
fn sphere_distance(
    origin: Vec3,
    direction: Vec3,
    center: Vec3,
    radius: f64,
    range: f64,
) -> Option<f64> {
    let offset = sub(origin, center);
    let b = offset.dot(direction);
    let c = offset.dot(offset) - radius * radius;
    if c <= 0.0 {
        return Some(0.0);
    }
    let d = b * b - c;
    if d < 0.0 {
        return None;
    }
    let t = -b - d.sqrt();
    (t >= 0.0 && t <= range).then_some(t)
}
impl State {
    pub(super) fn validate_npc_projectile(
        &self,
        world: &World,
        p: ProjectileSnapshot,
        i: usize,
        id: NpcId,
    ) -> Result<(), SaveError> {
        let e = self
            .npc_launches
            .get(i)
            .and_then(|e| e.as_ref())
            .ok_or(INVALID)?;
        let data = world.station_data.as_ref().ok_or(INVALID)?;
        world
            .traffic
            .validate_missile_scene(&e.scene, world.state.tick, data)?;
        let target = e.scene.target(id).ok_or(INVALID)?;
        if id.profile() != p.origin.owner.owner
            || id.system() != p.origin.owner.system
            || e.scene.world_tick != p.origin.launched_tick
            || e.scene.system != id.system()
            || e.scene.stage
                != SceneStage::Command(p.origin.emission.checked_sub(1).ok_or(INVALID)?)
            || e.dock_sequence == 0
            || sub(target.pose.position, p.origin.launch_position).dot(p.origin.orientation.z)
                <= 0.0
        {
            return Err(INVALID);
        }
        let nav = world.navigation.ok_or(INVALID)?.validate_save(
            world.navigation_catalogue.as_ref().ok_or(INVALID)?,
            world.state.mode,
            world.state.tick,
            world
                .commander_account
                .as_ref()
                .ok_or(INVALID)?
                .fuel_tenths(),
        )?;
        if e.dock_sequence > nav.dock_sequence {
            return Err(INVALID);
        }
        if p.origin.launched_tick == world.state.tick {
            let pose = super::super::flight_pose(world.state.position, world.state.orientation)
                .ok_or(INVALID)?;
            if p.origin.launch_position != pose.position
                || p.origin.orientation != pose.orientation
                || p.origin.physical_tick != world.combat.physical_tick()
                || world.state.mode != FlightMode::Flight
            {
                return Err(INVALID);
            }
        }
        let motion_tick = p
            .origin
            .launched_tick
            .checked_add(u64::from(p.age_ticks))
            .ok_or(INVALID)?;
        if p.age_ticks == 0 {
            self.validate_newborn_motion(p, i)?;
        } else {
            let m = self.motions[i].as_ref().ok_or(INVALID)?;
            super::super::persistence::validate_flight(&m.flight)?;
            if m.flight.tick != motion_tick
                || m.flight.mode != FlightMode::Flight
                || m.age.checked_add(1) != Some(p.age_ticks)
                || !m.travel.is_finite()
                || (m.travel - f64::from(m.age) * (MISSILE_SPEED / 60.0)).abs() > 1e-7
                || !finite(m.position)
                || (nav.dock_sequence == e.dock_sequence && world.pulse.alive() && !m.player_alive)
                || length(sub(m.position, p.origin.muzzle)) > m.travel + 1e-7
                || (m.flight.tick == world.state.tick
                    && (m.flight.position != world.state.position
                        || m.flight.orientation != world.state.orientation))
            {
                return Err(INVALID);
            }
            let scene = m.npc_scene.as_ref().ok_or(INVALID)?;
            if scene.world_tick != motion_tick
                || scene.system != p.origin.owner.system
                || scene.stage != SceneStage::Step(i as u8)
            {
                return Err(INVALID);
            }
            world
                .traffic
                .validate_missile_scene(scene, world.state.tick, data)?;
            let target = scene.target(id).ok_or(INVALID)?;
            let direction = unit(sub(target.pose.position, m.position)).ok_or(INVALID)?;
            if direction != p.direction {
                return Err(INVALID);
            }
            let ray = pose(m.position, direction).ok_or(INVALID)?;
            let travel = (MISSILE_SPEED / 60.0).min((MISSILE_RANGE - m.travel).max(0.0));
            let player = super::super::flight_pose(m.flight.position, m.flight.orientation)
                .ok_or(INVALID)?;
            let cleared = m.cleared
                || length(sub(m.position, player.position)) > data.player_clearance_radius() + 1e-6;
            let station = data
                .beam_block_distance(
                    world.state.station.ok_or(INVALID)?,
                    ray.position,
                    direction,
                    travel,
                )
                .map_err(|_| INVALID)?;
            let mut best = station.map(|d| (d, MissileContact::Station));
            if cleared
                && m.player_alive
                && let Some(d) = data
                    .player_beam_distance(player, ray.position, direction, travel)
                    .map_err(|_| INVALID)?
            {
                choose(
                    &mut best,
                    d,
                    MissileContact::Player {
                        front: sub(add(ray.position, scale(direction, d)), player.position)
                            .dot(player.orientation.z)
                            >= 0.0,
                    },
                );
            }
            self.validate_motion_bodies(p, i, m, ray, travel, &mut best)?;
            choose_scene(world, &p, scene, ray, travel, &mut best).map_err(|_| INVALID)?;
            let used = best.map_or(travel, |(d, _)| d);
            if p.position != add(m.position, scale(direction, used))
                || p.propulsion_travel != m.travel + used
                || p.contact != best.map(|(_, c)| c)
                || p.muzzle_cleared
                    != (cleared
                        || (best.is_none()
                            && length(sub(p.position, player.position))
                                > data.player_clearance_radius() + 1e-6))
            {
                return Err(INVALID);
            }
        }
        match p.ended {
            None => {
                if p.last_tick != motion_tick
                    || p.last_tick != world.state.tick
                    || p.origin.owner != world.combat_identity().ok_or(INVALID)?
                    || world.state.mode != FlightMode::Flight
                    || p.propulsion_travel >= MISSILE_RANGE
                {
                    return Err(INVALID);
                }
            }
            Some(MissileEnd::Impact) => {
                if p.last_tick != motion_tick || p.contact.is_none() {
                    return Err(INVALID);
                }
            }
            Some(MissileEnd::RangeExpired) => {
                if p.propulsion_travel != MISSILE_RANGE
                    || p.age_ticks != 455
                    || p.last_tick != motion_tick
                {
                    return Err(INVALID);
                }
            }
            Some(MissileEnd::LifetimeExpired) => return Err(INVALID),
            Some(MissileEnd::TargetLost) => {
                let loss = e.loss.as_ref().ok_or(INVALID)?;
                world
                    .traffic
                    .validate_missile_scene(loss, world.state.tick, data)?;
                if loss.world_tick != p.last_tick
                    || loss.system != p.origin.owner.system
                    || p.last_tick != motion_tick + 1
                    || loss.stage != SceneStage::Step(i as u8)
                    || loss.target(id).is_some()
                {
                    return Err(INVALID);
                }
            }
            Some(MissileEnd::Culled) => {
                if p.last_tick != motion_tick + 1
                    || !(nav.dock_sequence > e.dock_sequence
                        || nav.visit > p.origin.owner.visit
                        || world.state.mode == FlightMode::Docked)
                {
                    return Err(INVALID);
                }
            }
            Some(MissileEnd::Intercepted) => self.validate_interception(world, None, p, i)?,
        }
        if (p.ended == Some(MissileEnd::Impact)) != p.contact.is_some()
            || e.loss.is_some() != (p.ended == Some(MissileEnd::TargetLost))
        {
            return Err(INVALID);
        }
        Ok(())
    }
    /// Complete retained incoming missile history for MC1's born-resource journal.
    /// That replay includes real berth resets, so historical hits must stay present.
    pub(in crate::flight) fn player_resource_hits(&self) -> Vec<super::super::OrdinaryImpact> {
        self.impacts
            .iter()
            .flatten()
            .copied()
            .filter(|h| matches!(h.target, super::super::OrdinaryTarget::Player { .. }))
            .collect()
    }
    /// Read-only adapter for the legacy, since-last-berth shield fallback.
    /// All returned hits already belong to actual retained player-owned missiles; no new reducer.
    pub(in crate::flight) fn retained_player_missile_hits(
        &self,
        practice: Option<super::super::practice::Practice>,
        history: &[super::super::practice::Practice],
        dock_sequence: u64,
    ) -> Vec<super::super::OrdinaryImpact> {
        self.player_resource_hits()
            .into_iter()
            .filter(|h| {
                self.projectiles
                    .iter()
                    .flatten()
                    .find(|p| p.origin.emission == h.emission)
                    .is_some_and(|p| !self.docked_since(*p, practice, history, dock_sequence))
            })
            .collect()
    }
}
fn finite(p: Vec3) -> bool {
    [p.x, p.y, p.z].into_iter().all(f64::is_finite)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod join_tests;

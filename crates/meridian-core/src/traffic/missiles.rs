//! Actual NPC surfaces and distinct retained player-missile damage; no award path.
use super::{
    Actor, Arc, Basis3, BeamSource, INVALID, MAX_TRAFFIC_ACTORS, MAX_TRAFFIC_SHOTS, NpcId,
    NpcPhase, OriginalStationData, Resources, RigidPose, SaveError, State, SystemId,
    TrafficContact, TrafficError, Vec3, add, finite, length, pose, scale, sub, system_index,
};
use crate::flight::MissileOrigin;
use crate::save::{Reader, Writer};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SceneStage {
    Command(u64),
    Step(u8),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NpcBody {
    pub(crate) id: NpcId,
    pub(crate) pose: RigidPose,
    pub(crate) phase: NpcPhase,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MissileScene {
    pub(crate) system: SystemId,
    pub(crate) world_tick: u64,
    pub(crate) local_tick: u64,
    pub(crate) stage: SceneStage,
    pub(crate) bodies: Vec<NpcBody>,
}
impl MissileScene {
    pub(crate) fn target(&self, id: NpcId) -> Option<NpcBody> {
        self.bodies
            .iter()
            .copied()
            .find(|b| b.id == id && !matches!(b.phase, NpcPhase::Destroyed | NpcPhase::Complete))
    }
    pub(crate) fn contact(
        &self,
        data: &OriginalStationData,
        origin: Vec3,
        direction: Vec3,
        range: f64,
    ) -> Option<TrafficContact> {
        self.bodies
            .iter()
            .filter_map(|b| {
                data.player_beam_distance(b.pose, origin, direction, range)
                    .ok()
                    .flatten()
                    .map(|distance| TrafficContact {
                        id: b.id,
                        distance,
                        point: add(origin, scale(direction, distance)),
                    })
            })
            .min_by(|a, b| {
                a.distance
                    .total_cmp(&b.distance)
                    .then_with(|| a.id.serial.cmp(&b.id.serial))
            })
    }
    pub(crate) fn encode(&self, w: &mut Writer) {
        w.u8(self.system.galaxy);
        w.u16(self.system.system);
        w.u64(self.world_tick);
        w.u64(self.local_tick);
        match self.stage {
            SceneStage::Command(e) => {
                w.u8(0);
                w.u64(e);
            }
            SceneStage::Step(i) => {
                w.u8(1);
                w.u8(i);
            }
        }
        w.u8(self.bodies.len() as u8);
        for b in &self.bodies {
            b.id.encode(w);
            rigid(w, b.pose);
            w.u8(phase(b.phase));
        }
    }
    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let system = SystemId {
            galaxy: r.u8()?,
            system: r.u16()?,
        };
        let world_tick = r.u64()?;
        let local_tick = r.u64()?;
        let stage = match r.u8()? {
            0 => SceneStage::Command(r.u64()?),
            1 => SceneStage::Step(r.u8()?),
            _ => return Err(INVALID),
        };
        let n = usize::from(r.u8()?);
        if n > MAX_TRAFFIC_ACTORS {
            return Err(INVALID);
        }
        let mut bodies = Vec::new();
        for _ in 0..n {
            bodies.push(NpcBody {
                id: NpcId::decode(r)?,
                pose: read_rigid(r)?,
                phase: read_phase(r.u8()?)?,
            });
        }
        Ok(Self {
            system,
            world_tick,
            local_tick,
            stage,
            bodies,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NpcMissileHit {
    pub(crate) origin: MissileOrigin,
    pub(crate) victim: NpcId,
    pub(crate) world_tick: u64,
    pub(crate) local_tick: u64,
    pub(crate) point: Vec3,
    pub(crate) victim_pose: RigidPose,
    pub(crate) front: bool,
}
impl NpcMissileHit {
    pub(super) fn encode(self, w: &mut Writer) {
        self.origin.encode(w);
        self.victim.encode(w);
        w.u64(self.world_tick);
        w.u64(self.local_tick);
        point(w, self.point);
        rigid(w, self.victim_pose);
        w.bool(self.front);
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            origin: MissileOrigin::decode(r)?,
            victim: NpcId::decode(r)?,
            world_tick: r.u64()?,
            local_tick: r.u64()?,
            point: read_point(r)?,
            victim_pose: read_rigid(r)?,
            front: r.bool()?,
        })
    }
}
enum ResourceAction {
    Fire,
    Pulse(bool),
    Missile(bool),
}
type ResourceEvent = (u64, u64, i32, usize, ResourceAction);

impl State {
    pub(crate) fn combat_proof_count(&self) -> usize {
        self.receipt_count()
    }
    pub(crate) fn missile_scene(
        &self,
        system: SystemId,
        world_tick: u64,
        stage: SceneStage,
    ) -> MissileScene {
        MissileScene {
            system,
            world_tick,
            local_tick: system_index(system).map_or(0, |i| self.clocks[i]),
            stage,
            bodies: self
                .actors
                .iter()
                .flatten()
                .filter(|a| a.id.system == system && a.solid())
                .map(|a| NpcBody {
                    id: a.id,
                    pose: a.pose(),
                    phase: a.phase,
                })
                .collect(),
        }
    }
    pub(crate) fn validate_missile_scene(
        &self,
        s: &MissileScene,
        world_tick: u64,
        data: &OriginalStationData,
    ) -> Result<(), SaveError> {
        let si = system_index(s.system).ok_or(INVALID)?;
        if s.world_tick > world_tick
            || s.local_tick > self.clocks[si]
            || self.clocks[si].saturating_sub(s.local_tick)
                > (world_tick - s.world_tick)
                    .saturating_add(u64::from(matches!(s.stage, SceneStage::Step(_))))
            || s.bodies.len() > MAX_TRAFFIC_ACTORS
        {
            return Err(INVALID);
        }
        for a in self
            .actors
            .iter()
            .flatten()
            .filter(|a| a.id.system == s.system)
        {
            let before_move = matches!(s.stage, SceneStage::Step(_));
            let expected = (a.born_world_tick < s.world_tick
                || (a.born_world_tick == s.world_tick && !before_move))
                && a.sold_world_tick
                    .is_none_or(|t| t > s.world_tick || (t == s.world_tick && before_move));
            let body = s.bodies.iter().filter(|b| b.id == a.id).collect::<Vec<_>>();
            if body.len() != usize::from(expected) {
                return Err(INVALID);
            }
            if let Some(b) = body.first() {
                let resources = self.resources_at(*a, s.local_tick, s.world_tick, s.stage)?;
                let yaw = b.pose.orientation.z.x.atan2(b.pose.orientation.z.z);
                let planar = pose(b.pose.position, yaw);
                let birth_position = if a.id.serial == 1 {
                    let launch = data.launch_start().map_err(|_| INVALID)?;
                    crate::station::flight_pose(launch.position, launch.orientation)
                        .ok_or(INVALID)?
                        .position
                } else {
                    Vec3::new(2400.0, 0.0, 10500.0)
                };
                if s.local_tick < a.birth_clock
                    || s.local_tick - a.birth_clock > s.world_tick - a.born_world_tick
                    || b.pose.position.y != 0.0
                    || length(sub(b.pose.orientation.x, planar.orientation.x)) > 1e-12
                    || length(sub(b.pose.orientation.y, planar.orientation.y)) > 1e-12
                    || length(sub(b.pose.orientation.z, planar.orientation.z)) > 1e-12
                    || length(sub(b.pose.position, birth_position))
                        > ((s.local_tick - a.birth_clock + 1) as f64) * 10.29 + 1e-7
                    || !finite(b.pose.position)
                    || !b.pose.orientation.is_valid_rotation()
                    || matches!(b.phase, NpcPhase::Complete)
                    || (b.phase == NpcPhase::Destroyed) != (resources.energy == 0)
                    || !data.motion_clear(b.pose, b.pose, 0.0)
                    || (s.world_tick == world_tick && !before_move && b.pose != a.pose())
                {
                    return Err(INVALID);
                }
            }
        }
        if s.bodies.iter().any(|b| {
            b.id.profile != self.seed || b.id.system != s.system || self.actor_index(b.id).is_none()
        }) {
            return Err(INVALID);
        }
        Ok(())
    }
    pub(crate) fn prepare_missile_contact(
        &self,
        data: &OriginalStationData,
        scene: &MissileScene,
        origin: MissileOrigin,
        ray: RigidPose,
        distance: f64,
        victim: NpcId,
    ) -> Result<Self, TrafficError> {
        if scene
            != &self.missile_scene(
                origin.owner.system,
                scene.world_tick,
                SceneStage::Step(origin.projectile.ordinal()),
            )
            || origin.owner.owner != self.seed
            || origin.item == 0
            || origin.emission == 0
        {
            return Err(TrafficError::InvalidState);
        }
        let c = scene
            .contact(data, ray.position, ray.orientation.z, distance)
            .ok_or(TrafficError::InvalidState)?;
        if c.id != victim || c.distance != distance {
            return Err(TrafficError::InvalidState);
        }
        let index = self.actor_index(victim).ok_or(TrafficError::InvalidState)?;
        let mut next = self.clone();
        let mut a = next.actors[index].ok_or(TrafficError::InvalidState)?;
        if a.phase == NpcPhase::Destroyed {
            return Ok(next);
        }
        if !a.airborne() {
            return Err(TrafficError::InvalidState);
        }
        if self.combat_proof_count() >= MAX_TRAFFIC_SHOTS {
            return Err(TrafficError::ReceiptCapacity);
        }
        let front = sub(c.point, a.position).dot(a.pose().orientation.z) >= 0.0;
        let hit = NpcMissileHit {
            origin,
            victim,
            world_tick: scene.world_tick,
            local_tick: scene.local_tick,
            point: c.point,
            victim_pose: a.pose(),
            front,
        };
        a.resources.hit_missile(front)?;
        a.resource_clock = scene.local_tick;
        if a.resources.energy == 0 {
            a.phase = NpcPhase::Destroyed;
            a.speed = 0.0;
        }
        next.actors[index] = Some(a);
        Arc::make_mut(&mut next.missile_hits).push(hit);
        Ok(next)
    }
    pub(crate) fn missile_hits(&self) -> &[NpcMissileHit] {
        &self.missile_hits
    }
    pub(super) fn validate_missile_resource_replay(&self) -> Result<(), SaveError> {
        for (i, h) in self.missile_hits.iter().enumerate() {
            let a = self.actors[self.actor_index(h.victim).ok_or(INVALID)?].ok_or(INVALID)?;
            let si = system_index(h.victim.system).ok_or(INVALID)?;
            if h.origin.owner.owner != self.seed
                || h.origin.owner.hull != self.seed.checked_add(1).ok_or(INVALID)?
                || h.origin.owner.system != h.victim.system
                || h.origin.item == 0
                || h.origin.emission == 0
                || h.world_tick < h.origin.launched_tick
                || h.world_tick < a.born_world_tick
                || h.local_tick < a.birth_clock
                || h.local_tick > self.clocks[si]
                || !finite(h.point)
                || !h.victim_pose.orientation.is_valid_rotation()
                || h.front
                    != (sub(h.point, h.victim_pose.position).dot(h.victim_pose.orientation.z)
                        >= 0.0)
                || self.missile_hits[..i]
                    .iter()
                    .any(|x| x.origin.projectile == h.origin.projectile)
            {
                return Err(INVALID);
            }
        }
        for a in self.actors.iter().flatten() {
            self.validate_resource_replay(*a)?;
        }
        Ok(())
    }
    fn beam_resource_actions(
        &self,
        a: Actor,
        clock: u64,
        world: u64,
        stage: SceneStage,
        actions: &mut Vec<ResourceEvent>,
    ) {
        for (i, b) in self.shots.iter().enumerate() {
            if b.local_tick > clock || b.world_tick > world {
                continue;
            }
            let order = if matches!(b.source, BeamSource::Npc(_)) {
                0
            } else {
                2
            };
            let included = b.world_tick < world
                || match (stage, b.source) {
                    (SceneStage::Command(_), BeamSource::Npc(_)) => true,
                    (SceneStage::Command(e), BeamSource::Player { emission, .. }) => emission <= e,
                    _ => false,
                };
            if !included {
                continue;
            }
            if b.source == BeamSource::Npc(a.id) {
                actions.push((b.local_tick, b.world_tick, order, i, ResourceAction::Fire));
            }
            if b.victim == a.id {
                actions.push((
                    b.local_tick,
                    b.world_tick,
                    order,
                    i,
                    ResourceAction::Pulse(b.front),
                ));
            }
        }
    }
    pub(super) fn resources_at(
        &self,
        a: Actor,
        clock: u64,
        world: u64,
        stage: SceneStage,
    ) -> Result<Resources, SaveError> {
        let mut actions = Vec::new();
        self.beam_resource_actions(a, clock, world, stage, &mut actions);
        for (i, h) in self.player_hits.iter().enumerate() {
            if h.attacker == a.id
                && h.local_tick <= clock
                && (h.world_tick < world
                    || (h.world_tick == world && matches!(stage, SceneStage::Command(_))))
            {
                actions.push((h.local_tick, h.world_tick, 0, i, ResourceAction::Fire));
            }
        }
        for (i, h) in self.missile_hits.iter().enumerate() {
            let included = h.world_tick < world
                || match stage {
                    SceneStage::Command(_) => true,
                    SceneStage::Step(index) => h.origin.projectile.ordinal() < index,
                };
            if h.victim == a.id && h.local_tick <= clock && h.world_tick <= world && included {
                actions.push((
                    h.local_tick,
                    h.world_tick,
                    1,
                    i,
                    ResourceAction::Missile(h.front),
                ));
            }
        }
        actions.sort_by_key(|(t, w, order, i, _)| (*t, *w, *order, *i));
        let mut resources = Resources::fresh();
        let mut cursor = a.birth_clock.checked_sub(1).ok_or(INVALID)?;
        for (tick, _, _, _, action) in actions {
            let n = tick.checked_sub(cursor).ok_or(INVALID)?;
            if resources.energy == 0 {
                return Err(INVALID);
            }
            resources.advance_exact(n)?;
            match action {
                ResourceAction::Fire => resources.fire(),
                ResourceAction::Pulse(front) => resources.hit(front),
                ResourceAction::Missile(front) => resources.hit_missile(front),
            }
            .map_err(|_| INVALID)?;
            cursor = tick;
        }
        let n = clock.checked_sub(cursor).ok_or(INVALID)?;
        if resources.energy == 0 && n != 0 {
            return Err(INVALID);
        }
        resources.advance_exact(n)?;
        Ok(resources)
    }
}
impl Resources {
    fn advance_exact(&mut self, n: u64) -> Result<(), SaveError> {
        self.advance_many(n).map_err(|_| INVALID)
    }
    fn hit_missile(&mut self, front: bool) -> Result<(), TrafficError> {
        let shield = if front {
            &mut self.front
        } else {
            &mut self.rear
        };
        let absorbed = (*shield).min(16000);
        *shield -= absorbed;
        let total = if front {
            &mut self.absorbed_front
        } else {
            &mut self.absorbed_rear
        };
        *total = total
            .checked_add(u64::from(absorbed))
            .ok_or(TrafficError::SequenceExhausted)?;
        let remaining = 16000 - absorbed;
        let damage = self.energy.min(remaining);
        self.energy -= damage;
        self.hull_damage = self
            .hull_damage
            .checked_add(u64::from(damage))
            .ok_or(TrafficError::SequenceExhausted)?;
        self.overkill = self
            .overkill
            .checked_add(u64::from(remaining - damage))
            .ok_or(TrafficError::SequenceExhausted)?;
        self.incoming_missiles = self
            .incoming_missiles
            .checked_add(1)
            .ok_or(TrafficError::SequenceExhausted)?;
        Ok(())
    }
}
fn point(w: &mut Writer, p: Vec3) {
    for n in [p.x, p.y, p.z] {
        w.u64(n.to_bits());
    }
}
fn read_point(r: &mut Reader<'_>) -> Result<Vec3, SaveError> {
    Ok(Vec3::new(
        f64::from_bits(r.u64()?),
        f64::from_bits(r.u64()?),
        f64::from_bits(r.u64()?),
    ))
}
fn rigid(w: &mut Writer, p: RigidPose) {
    point(w, p.position);
    point(w, p.orientation.x);
    point(w, p.orientation.y);
    point(w, p.orientation.z);
}
fn read_rigid(r: &mut Reader<'_>) -> Result<RigidPose, SaveError> {
    Ok(RigidPose {
        position: read_point(r)?,
        orientation: Basis3 {
            x: read_point(r)?,
            y: read_point(r)?,
            z: read_point(r)?,
        },
    })
}
fn phase(p: NpcPhase) -> u8 {
    match p {
        NpcPhase::Departing => 0,
        NpcPhase::Outbound => 1,
        NpcPhase::ReturnAlign => 2,
        NpcPhase::Inbound => 3,
        NpcPhase::Complete => 4,
        NpcPhase::Hunting => 5,
        NpcPhase::Retreating => 6,
        NpcPhase::Destroyed => 7,
        NpcPhase::Approach => 8,
        NpcPhase::FinalAlign => 9,
    }
}
fn read_phase(x: u8) -> Result<NpcPhase, SaveError> {
    Ok(match x {
        0 => NpcPhase::Departing,
        1 => NpcPhase::Outbound,
        2 => NpcPhase::ReturnAlign,
        3 => NpcPhase::Inbound,
        4 => NpcPhase::Complete,
        5 => NpcPhase::Hunting,
        6 => NpcPhase::Retreating,
        7 => NpcPhase::Destroyed,
        8 => NpcPhase::Approach,
        9 => NpcPhase::FinalAlign,
        _ => return Err(INVALID),
    })
}

#[cfg(test)]
impl State {
    /// Negative save fixtures only; never used to create an accepted combat positive.
    pub(crate) fn corrupt_missile_hits(&mut self, corrupt: impl FnOnce(&mut Vec<NpcMissileHit>)) {
        corrupt(Arc::make_mut(&mut self.missile_hits));
    }
}
#[cfg(test)]
mod tests;

//! A physical query snapshot. It is never an EMP release capability or an ordinary weapon origin.
use super::{EmpError, INVALID, SaveError, World, policy};
use crate::docking::{RigidPose, Vec3};
use crate::traffic::{NpcId, NpcPhase};
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Body {
    pub(super) id: NpcId,
    pub(super) pose: RigidPose,
    pub(super) phase: NpcPhase,
}
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Scene {
    pub(super) owner: super::super::CombatIdentity,
    pub(super) ordinary_cursor: u64,
    pub(super) missiles: Vec<(crate::flight::ProjectileId, RigidPose)>,
    pub(super) local_tick: u64,
    pub(super) world_tick: u64,
    pub(super) physical_tick: u64,
    pub(super) player: RigidPose,
    pub(super) bodies: Vec<Body>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Victim {
    Npc(NpcId),
    Player,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Contact {
    pub(super) victim: Victim,
    pub(super) point: Vec3,
    pub(super) front: bool,
    pub(super) power_hundredths: u16,
}
impl Scene {
    pub(super) fn capture(
        traffic: &crate::traffic::State,
        missiles: &super::super::missiles::State,
        ordinary_cursor: u64,
        owner: super::super::CombatIdentity,
        player: RigidPose,
        world_tick: u64,
        physical_tick: u64,
    ) -> Result<Self, EmpError> {
        let snapshot = traffic.snapshot(owner.system).ok_or(EmpError::NoTarget)?;
        Ok(Self {
            owner,
            ordinary_cursor,
            missiles: missiles.beam_scene(owner.visit),
            local_tick: traffic
                .emp_clock(owner.system)
                .ok_or(EmpError::InvalidContact)?,
            world_tick,
            physical_tick,
            player,
            bodies: snapshot
                .actors
                .into_iter()
                .filter(|a| a.phase != NpcPhase::Complete)
                .map(|a| Body {
                    id: a.id,
                    pose: a.pose,
                    phase: a.phase,
                })
                .collect(),
        })
    }
    fn nearest(
        &self,
        world: &World,
        origin: Vec3,
        direction: Vec3,
        range: f64,
        except: Option<NpcId>,
        include_player: bool,
    ) -> Result<Option<(Victim, Vec3, f64)>, EmpError> {
        let data = world
            .station_data
            .as_ref()
            .ok_or(EmpError::InvalidContact)?;
        let mut contacts = Vec::new();
        for body in &self.bodies {
            if Some(body.id) == except {
                continue;
            }
            if let Some(distance) = data
                .player_beam_distance(body.pose, origin, direction, range)
                .map_err(|_| EmpError::InvalidContact)?
            {
                contacts.push((
                    Victim::Npc(body.id),
                    add(origin, scale(direction, distance)),
                    distance,
                ));
            }
        }
        if include_player
            && let Some(distance) = data
                .player_beam_distance(self.player, origin, direction, range)
                .map_err(|_| EmpError::InvalidContact)?
        {
            contacts.push((
                Victim::Player,
                add(origin, scale(direction, distance)),
                distance,
            ));
        }
        contacts.sort_by(|a, b| a.2.total_cmp(&b.2));
        let Some(contact) = contacts.first().copied() else {
            return Ok(None);
        };
        // Actual station, celestial and mining surfaces occlude the pulse; none is a victim alias.
        let station = world.state.station.ok_or(EmpError::InvalidContact)?;
        let station_blocked = data
            .beam_block_distance(station, origin, direction, contact.2)
            .map_err(|_| EmpError::InvalidContact)?
            .is_some_and(|distance| distance < contact.2);
        let missile_blocked = self.missiles.iter().any(|(_, body)| {
            World::emp_missile_distance(*body, origin, direction, contact.2).is_some()
        });
        if station_blocked
            || missile_blocked
            || !world.traffic_beam_celestial_clear(
                self.owner.system,
                self.world_tick,
                origin,
                contact.1,
            )
            || !world.mining_segment_at_step(
                self.owner.system,
                self.world_tick,
                origin,
                contact.1,
                0.0,
            )
        {
            return Ok(None);
        }
        Ok(Some(contact))
    }
    pub(super) fn contacts(
        &self,
        world: &World,
        profile: policy::Profile,
        target: NpcId,
    ) -> Result<Vec<Contact>, EmpError> {
        let primary = self
            .nearest(
                world,
                self.player.position,
                self.player.orientation.z,
                profile.range,
                None,
                false,
            )?
            .ok_or(EmpError::NoTarget)?;
        if primary.0 != Victim::Npc(target) {
            return Err(EmpError::NoTarget);
        }
        let body = self
            .bodies
            .iter()
            .find(|b| b.id == target && b.phase != NpcPhase::Destroyed)
            .ok_or(EmpError::NoTarget)?;
        let front = sub(primary.1, body.pose.position).dot(body.pose.orientation.z) >= 0.0;
        let mut contacts = vec![Contact {
            victim: primary.0,
            point: primary.1,
            front,
            power_hundredths: u16::try_from(profile.power / (policy::SCALE / 100))
                .map_err(|_| EmpError::InvalidContact)?,
        }];
        // Target-centred splash uses a fresh nearest-surface query, including the player.
        for candidate in self
            .bodies
            .iter()
            .filter(|b| b.id != target && b.phase != NpcPhase::Destroyed)
        {
            let delta = sub(candidate.pose.position, primary.1);
            let Some(direction) = unit(delta) else {
                continue;
            };
            if let Some((victim, point, distance)) = self.nearest(
                world,
                primary.1,
                direction,
                profile.range / 3.0,
                Some(target),
                true,
            )? && victim == Victim::Npc(candidate.id)
            {
                let power = profile.splash(distance)? / (policy::SCALE / 100);
                if power > 0 {
                    contacts.push(Contact {
                        victim,
                        point,
                        front: sub(point, candidate.pose.position)
                            .dot(candidate.pose.orientation.z)
                            >= 0.0,
                        power_hundredths: u16::try_from(power)
                            .map_err(|_| EmpError::InvalidContact)?,
                    });
                }
            }
        }
        // The model centre can be the native fore/aft opening. Select a real hull
        // triangle, then retain the same nearest-surface, range and occlusion query.
        let player_surface = world
            .station_data
            .as_ref()
            .ok_or(EmpError::InvalidContact)?
            .player_missile_aimpoint(self.player, primary.1)
            .map_err(|_| EmpError::InvalidContact)?;
        if let Some(direction) = unit(sub(player_surface, primary.1))
            && let Some((Victim::Player, point, distance)) = self.nearest(
                world,
                primary.1,
                direction,
                profile.range / 3.0,
                Some(target),
                true,
            )?
        {
            let power = profile.splash(distance)? / (policy::SCALE / 100);
            if power > 0 {
                contacts.push(Contact {
                    victim: Victim::Player,
                    point,
                    front: sub(point, self.player.position).dot(self.player.orientation.z) >= 0.0,
                    power_hundredths: u16::try_from(power).map_err(|_| EmpError::InvalidContact)?,
                });
            }
        }
        Ok(contacts)
    }
}
fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn scale(v: Vec3, n: f64) -> Vec3 {
    Vec3::new(v.x * n, v.y * n, v.z * n)
}
fn unit(v: Vec3) -> Option<Vec3> {
    let n = v.dot(v).sqrt();
    (n.is_finite() && n > 1e-9).then(|| scale(v, 1.0 / n))
}

impl Scene {
    pub(super) fn validate(&self, world: &World, before_pulse: u16) -> Result<(), SaveError> {
        let owner = world.combat_identity().ok_or(INVALID)?;
        if self.owner.owner != owner.owner
            || self.owner.hull != owner.hull
            || self.owner.visit < 2
            || self.owner.visit > owner.visit
            || self.world_tick > world.state.tick
            || self.physical_tick > world.combat.physical_tick()
            || self.physical_tick > self.world_tick
            || self.ordinary_cursor > world.combat.emissions
            || !self.player.orientation.is_valid_rotation()
            || ![
                self.player.position.x,
                self.player.position.y,
                self.player.position.z,
            ]
            .into_iter()
            .all(|v| v.is_finite() && v.abs() <= 1e12)
            || (self.world_tick == world.state.tick
                && super::super::flight_pose(world.state.position, world.state.orientation)
                    != Some(self.player))
        {
            return Err(INVALID);
        }
        let data = world.station_data.as_ref().ok_or(INVALID)?;
        world.traffic.validate_emp_scene(
            &crate::traffic::MissileScene {
                system: self.owner.system,
                world_tick: self.world_tick,
                local_tick: self.local_tick,
                stage: crate::traffic::SceneStage::Command(u64::MAX),
                bodies: self
                    .bodies
                    .iter()
                    .map(|b| crate::traffic::NpcBody {
                        id: b.id,
                        pose: b.pose,
                        phase: b.phase,
                    })
                    .collect(),
            },
            world.state.tick,
            data,
            before_pulse,
        )?;
        // Membership and kinematic bounds come from the actual native missile producer. A
        // zero-length read query checks the scene without imposing a particular EMP ray.
        world.missiles.validate_retained_beam(
            &world.ecm,
            super::super::missiles::RetainedBeam {
                system: self.owner.system,
                visit: self.owner.visit,
                world_tick: self.world_tick,
                physical_tick: self.physical_tick,
                command_emission: Some(self.ordinary_cursor.checked_add(1).ok_or(INVALID)?),
                ecm_after_scene: true,
                ray: self.player,
                point: self.player.position,
                bodies: &self.missiles,
            },
        )?;
        Ok(())
    }
}

//! Explicit legacy/current retained contact evidence. Decode never emits gameplay events.
use super::geometry::{add, length, mesh_distance, pose, scale, sub, unit};
use super::{
    Beam, Body, CombatIdentity, FlightMode, MISSILE_COOLDOWN_TICKS, MISSILE_LIFETIME_TICKS,
    MISSILE_MUZZLE_DISTANCE, MISSILE_RANGE, MISSILE_SPEED, MissileContact, MissileEnd,
    MissileOrigin, MissileRound, MissileTarget, Motion, PracticeBeaconId, ProjectileId,
    ProjectileSnapshot, State, World, choose,
};
use crate::docking::{Basis3, RigidPose, StationPose, Vec3};
use crate::flight::practice::lifecycle::{lesson_for_target as legacy_lesson_for_target, lessons};
use crate::station::OriginalStationData;
fn lesson_for_target(
    p: Option<super::super::practice::Practice>,
    h: &[super::super::practice::Practice],
    t: MissileTarget,
) -> Option<super::super::practice::Practice> {
    t.practice()
        .and_then(|id| legacy_lesson_for_target(p, h, id))
}
use crate::flight::{OrdinaryImpact, OrdinaryTarget, OrdinaryWeapon, PulseContact, PulseShot};
use crate::navigation::SystemId;
use crate::save::{Reader, SaveError, Writer};
const INVALID: SaveError = SaveError::InvalidState("missile custody, motion or contact evidence");
type SolidContact = (Option<f64>, Option<(f64, MissileContact)>, Option<f64>);

/// Both retained homing paths query station first, then the eligible native player mesh.
/// A tie keeps the station; the caller retains its own exact save refusal boundary.
pub(super) fn retained_solid_contact(
    data: &OriginalStationData,
    station_pose: StationPose,
    ray: RigidPose,
    travel: f64,
    player: RigidPose,
    (cleared, player_alive): (bool, bool),
    invalid: SaveError,
) -> Result<SolidContact, SaveError> {
    let station = data
        .beam_block_distance(station_pose, ray.position, ray.orientation.z, travel)
        .map_err(|_| invalid)?;
    let best = station.map(|distance| (distance, MissileContact::Station));
    let player_contact = if cleared && player_alive {
        data.player_beam_distance(player, ray.position, ray.orientation.z, travel)
            .map_err(|_| invalid)?
    } else {
        None
    };
    Ok((station, best, player_contact))
}

fn scalar(w: &mut Writer, x: f64) {
    w.u64(x.to_bits());
}
fn number(r: &mut Reader<'_>) -> Result<f64, SaveError> {
    Ok(f64::from_bits(r.u64()?))
}
fn vector(w: &mut Writer, v: Vec3) {
    for x in [v.x, v.y, v.z] {
        scalar(w, x);
    }
}
fn read_vector(r: &mut Reader<'_>) -> Result<Vec3, SaveError> {
    Ok(Vec3::new(number(r)?, number(r)?, number(r)?))
}
fn basis(w: &mut Writer, b: Basis3) {
    for v in [b.x, b.y, b.z] {
        vector(w, v);
    }
}
fn read_basis(r: &mut Reader<'_>) -> Result<Basis3, SaveError> {
    Ok(Basis3 {
        x: read_vector(r)?,
        y: read_vector(r)?,
        z: read_vector(r)?,
    })
}
impl ProjectileId {
    pub(crate) fn encode(self, w: &mut Writer) {
        w.u64(self.owner);
        w.u64(self.visit);
        w.u8(self.ordinal);
    }
    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            owner: r.u64()?,
            visit: r.u64()?,
            ordinal: r.u8()?,
        })
    }
}
impl MissileOrigin {
    pub(crate) fn encode(self, w: &mut Writer) {
        self.projectile.encode(w);
        w.u64(self.owner.owner);
        w.u64(self.owner.hull);
        w.u8(self.owner.system.galaxy);
        w.u16(self.owner.system.system);
        w.u64(self.owner.visit);
        w.u64(self.emission);
        w.u64(self.item);
        w.u64(self.launched_tick);
        w.u8(self.legal_at_launch);
        w.u64(self.physical_tick);
        vector(w, self.launch_position);
        vector(w, self.muzzle);
        basis(w, self.orientation);
    }
    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            projectile: ProjectileId::decode(r)?,
            owner: CombatIdentity {
                owner: r.u64()?,
                hull: r.u64()?,
                system: SystemId {
                    galaxy: r.u8()?,
                    system: r.u16()?,
                },
                visit: r.u64()?,
            },
            emission: r.u64()?,
            item: r.u64()?,
            launched_tick: r.u64()?,
            legal_at_launch: r.u8()?,
            physical_tick: r.u64()?,
            launch_position: read_vector(r)?,
            muzzle: read_vector(r)?,
            orientation: read_basis(r)?,
        })
    }
}
fn contact(w: &mut Writer, c: MissileContact) {
    match c {
        MissileContact::Practice { id, first_hit } => {
            w.u8(0);
            id.encode(w);
            w.bool(first_hit);
        }
        MissileContact::Station => w.u8(1),
        MissileContact::Asteroid(id) => {
            w.u8(6);
            w.u8(id.ordinal());
        }
        MissileContact::Player { front } => {
            w.u8(2);
            w.bool(front);
        }
        MissileContact::Projectile(id) => {
            w.u8(3);
            id.encode(w);
        }
        MissileContact::Npc(id) => {
            w.u8(4);
            id.encode(w);
        }
        MissileContact::Celestial(id) => {
            w.u8(5);
            w.u8(id.system.galaxy);
            w.u16(id.system.system);
            w.u64(id.visit);
            w.u64(id.entity);
        }
    }
}
fn read_contact(r: &mut Reader<'_>) -> Result<MissileContact, SaveError> {
    Ok(match r.u8()? {
        0 => MissileContact::Practice {
            id: PracticeBeaconId::decode(r)?,
            first_hit: r.bool()?,
        },
        1 => MissileContact::Station,
        6 => MissileContact::Asteroid(crate::mining::AsteroidId::from_ordinal(r.u8()?)?),
        2 => MissileContact::Player { front: r.bool()? },
        3 => MissileContact::Projectile(ProjectileId::decode(r)?),
        4 => MissileContact::Npc(crate::traffic::NpcId::decode(r)?),
        5 => MissileContact::Celestial(crate::celestial::BodyId {
            system: SystemId {
                galaxy: r.u8()?,
                system: r.u16()?,
            },
            visit: r.u64()?,
            entity: r.u64()?,
        }),
        _ => return Err(SaveError::InvalidEncoding),
    })
}
fn end_tag(e: MissileEnd) -> u8 {
    match e {
        MissileEnd::Impact => 0,
        MissileEnd::RangeExpired => 1,
        MissileEnd::LifetimeExpired => 2,
        MissileEnd::TargetLost => 3,
        MissileEnd::Culled => 4,
        MissileEnd::Intercepted => 5,
        MissileEnd::Ecm => 6,
    }
}
fn read_end(r: &mut Reader<'_>) -> Result<MissileEnd, SaveError> {
    Ok(match r.u8()? {
        0 => MissileEnd::Impact,
        1 => MissileEnd::RangeExpired,
        2 => MissileEnd::LifetimeExpired,
        3 => MissileEnd::TargetLost,
        4 => MissileEnd::Culled,
        5 => MissileEnd::Intercepted,
        6 => MissileEnd::Ecm,
        _ => return Err(SaveError::InvalidEncoding),
    })
}
fn projectile(w: &mut Writer, p: &ProjectileSnapshot, version: u16) {
    p.origin.encode(w);
    write_target(w, p.target, version);
    vector(w, p.position);
    vector(w, p.direction);
    scalar(w, p.propulsion_travel);
    w.u16(p.age_ticks);
    w.bool(p.muzzle_cleared);
    w.option(p.ended.as_ref(), |w, e| w.u8(end_tag(*e)));
    w.option(p.contact.as_ref(), |w, c| contact(w, *c));
    w.u64(p.last_tick);
    w.option(p.intercepting_emission.as_ref(), |w, e| w.u64(*e));
    if version >= 17 {
        w.option(p.ecm.as_ref(), |w, end| end.encode(w));
    }
}
fn read_projectile(r: &mut Reader<'_>, version: u16) -> Result<ProjectileSnapshot, SaveError> {
    Ok(ProjectileSnapshot {
        origin: MissileOrigin::decode(r)?,
        target: read_target(r, version)?,
        position: read_vector(r)?,
        direction: read_vector(r)?,
        propulsion_travel: number(r)?,
        age_ticks: r.u16()?,
        muzzle_cleared: r.bool()?,
        ended: r.option(read_end)?,
        contact: r.option(read_contact)?,
        last_tick: r.u64()?,
        intercepting_emission: r.option(|r| r.u64())?,
        ecm: if version >= 17 {
            r.option(super::super::ecm::EcmTermination::decode)?
        } else {
            None
        },
    })
}
impl State {
    /// A frozen historical scene is bound to real launch/end custody. It is not a
    /// reconstruction of every intervening homing trajectory.
    pub(in crate::flight) fn validate_npc_beam_scene(
        &self,
        hit: &crate::traffic::NpcPlayerHit,
    ) -> Result<(), SaveError> {
        self.validate_retained_beam(super::RetainedBeam {
            system: hit.attacker.system(),
            visit: hit.visit,
            world_tick: hit.world_tick,
            physical_tick: hit.physical_tick,
            command_emission: None,
            ray: hit.source_pose,
            point: hit.point,
            bodies: &hit.missiles,
        })
    }
    pub(in crate::flight) fn validate_retained_beam(
        &self,
        hit: super::RetainedBeam<'_>,
    ) -> Result<(), SaveError> {
        let expected: Vec<_> = self
            .projectiles
            .iter()
            .flatten()
            .filter(|projectile| retained_beam_body(**projectile, &hit))
            .collect();
        if hit.bodies.len() != expected.len() {
            return Err(INVALID);
        }
        let distance = length(sub(hit.point, hit.ray.position));
        for ((id, body), projectile) in hit.bodies.iter().zip(expected) {
            let elapsed = hit
                .physical_tick
                .checked_sub(projectile.origin.physical_tick)
                .ok_or(INVALID)?;
            if *id != projectile.origin.projectile
                || body.orientation != projectile.origin.orientation
                || ![body.position.x, body.position.y, body.position.z]
                    .into_iter()
                    .all(f64::is_finite)
                || elapsed > u64::from(MISSILE_LIFETIME_TICKS)
                || length(sub(body.position, projectile.origin.muzzle))
                    > elapsed as f64 * MISSILE_SPEED / 60.0 + 1e-5
                || (projectile.last_tick == hit.world_tick && body.position != projectile.position)
                || mesh_distance(*body, hit.ray, distance).is_some()
            {
                return Err(INVALID);
            }
        }
        Ok(())
    }
    /// All retained player contacts for the one born-resource journal, including hits before berth.
    /// Legacy since-berth filtering is a separate adapter; contact validation stays here.
    pub(in crate::flight) fn player_resource_hits(&self) -> Vec<OrdinaryImpact> {
        self.impacts
            .iter()
            .copied()
            .flatten()
            .filter(|h| matches!(h.target, OrdinaryTarget::Player { .. }))
            .collect()
    }
    pub(in crate::flight) fn encode(&self, w: &mut Writer) {
        self.encode_version(w, crate::save::SAVE_VERSION);
    }
    pub(in crate::flight) fn encode_version(&self, w: &mut Writer, version: u16) {
        if version >= 12 {
            w.u16(self.projectiles.len() as u16);
        }
        w.bool(self.armed);
        w.option(self.lock.as_ref(), |w, id| write_target(w, *id, version));
        w.u8(self.cooldown);
        for i in 0..self.projectiles.len() {
            w.option(self.projectiles[i].as_ref(), |w, p| {
                projectile(w, p, version)
            });
            w.option(self.motions[i].as_ref(), |w, m| {
                vector(w, m.position);
                scalar(w, m.travel);
                w.u16(m.age);
                w.bool(m.cleared);
                super::super::persistence::write_flight(w, &m.flight);
                w.bool(m.player_alive);
                if version >= 12 {
                    w.u16(m.bodies.len() as u16);
                }
                for body in &m.bodies {
                    w.option(body.as_ref(), |w, b| {
                        b.id.encode(w);
                        vector(w, b.position);
                    });
                }
                if version >= 15 {
                    w.option(m.npc_scene.as_ref(), |w, s| s.encode(w));
                }
            });
            w.option(self.impacts[i].as_ref(), |w, h| h.encode(w));
            w.option(self.beams[i].as_ref(), |w, b| {
                let s = b.shot;
                w.u64(b.emission);
                w.u64(s.tick);
                w.u64(s.fitted_item_id);
                vector(w, s.origin);
                vector(w, s.direction);
                vector(w, s.end);
                if version >= 12 {
                    w.u16(b.bodies.len() as u16);
                }
                for body in &b.bodies {
                    w.option(body.as_ref(), |w, b| {
                        b.id.encode(w);
                        vector(w, b.position);
                    });
                }
                if let PulseContact::Projectile { id, distance } = s.contact {
                    id.encode(w);
                    scalar(w, distance);
                }
                if version >= 15 {
                    w.option(b.npc_scene.as_ref(), |w, s| s.encode(w));
                }
            });
            if version >= 15 {
                w.option(self.npc_launches[i].as_ref(), |w, e| e.encode(w));
            }
        }
    }
    pub(in crate::flight) fn decode(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        let count = if version >= 12 {
            usize::from(r.u16()?)
        } else {
            3
        };
        if !(3..=256).contains(&count) {
            return Err(INVALID);
        }
        let mut state = Self {
            armed: r.bool()?,
            lock: r.option(|r| read_target(r, version))?,
            cooldown: r.u8()?,
            ..Self::default()
        };
        state.projectiles.resize(count, None);
        state.motions.resize(count, None);
        state.impacts.resize(count, None);
        state.beams.resize(count, None);
        state.npc_launches.resize(count, None);
        for i in 0..count {
            state.projectiles[i] = r.option(|r| read_projectile(r, version))?;
            state.motions[i] = r.option(|r| {
                Ok(Motion {
                    position: read_vector(r)?,
                    travel: number(r)?,
                    age: r.u16()?,
                    cleared: r.bool()?,
                    flight: super::super::persistence::read_flight(r, version)?,
                    player_alive: r.bool()?,
                    bodies: read_bodies(r, version, count)?,
                    npc_scene: if version >= 15 {
                        r.option(crate::traffic::MissileScene::decode)?
                    } else {
                        None
                    },
                })
            })?;
            state.impacts[i] = r.option(|r| OrdinaryImpact::decode_version(r, version))?;
            state.beams[i] = r.option(|r| {
                let emission = r.u64()?;
                let tick = r.u64()?;
                let fitted_item_id = r.u64()?;
                let origin = read_vector(r)?;
                let direction = read_vector(r)?;
                let end = read_vector(r)?;
                let bodies = read_bodies(r, version, count)?;
                let shot = PulseShot {
                    tick,
                    fitted_item_id,
                    origin,
                    direction,
                    end,
                    contact: PulseContact::Projectile {
                        id: ProjectileId::decode(r)?,
                        distance: number(r)?,
                    },
                };
                let npc_scene = if version >= 15 {
                    r.option(crate::traffic::MissileScene::decode)?
                } else {
                    None
                };
                Ok(Beam {
                    bodies,
                    emission,
                    shot,
                    npc_scene,
                })
            })?;
            if version >= 15 {
                state.npc_launches[i] = r.option(super::npc::NpcLaunch::decode)?;
            }
        }
        Ok(state)
    }
    pub(in crate::flight) fn used(&self) -> usize {
        self.projectiles.iter().flatten().count()
    }
    pub(in crate::flight) fn has_player_hit(&self) -> bool {
        self.projectiles
            .iter()
            .flatten()
            .any(|p| matches!(p.contact, Some(MissileContact::Player { .. })))
    }
    fn find(&self, id: ProjectileId) -> Option<ProjectileSnapshot> {
        self.projectiles
            .iter()
            .flatten()
            .find(|p| p.origin.projectile == id)
            .copied()
    }
    pub(in crate::flight) fn has_undocked_player_hit(
        &self,
        practice: Option<super::super::practice::Practice>,
        history: &[super::super::practice::Practice],
        dock_sequence: u64,
    ) -> bool {
        self.projectiles.iter().flatten().any(|p| {
            matches!(p.contact, Some(MissileContact::Player { .. }))
                && !self.docked_since(*p, practice, history, dock_sequence)
        })
    }
    pub(in crate::flight) fn validate_ordinary(
        &self,
        hit: OrdinaryImpact,
        world: &World,
    ) -> Result<(), SaveError> {
        let i = self
            .impacts
            .iter()
            .position(|h| *h == Some(hit))
            .ok_or(INVALID)?;
        let p = self.projectiles[i].ok_or(INVALID)?;
        let expected = match p.contact {
            Some(MissileContact::Station) => OrdinaryTarget::MainStation,
            Some(MissileContact::Player { front }) => OrdinaryTarget::Player { front },
            _ => return Err(INVALID),
        };
        let m = self.motions[i].as_ref().ok_or(INVALID)?;
        if p.ended != Some(MissileEnd::Impact)
            || hit.weapon != OrdinaryWeapon::StandardMissile
            || hit.target != expected
            || hit.attacker != p.origin.owner
            || hit.item != p.origin.item
            || hit.emission != p.origin.emission
            || hit.tick != p.last_tick
            || hit.point != p.position
            || hit.physical_tick
                != p.origin
                    .physical_tick
                    .checked_add(u64::from(p.age_ticks))
                    .ok_or(INVALID)?
            || hit.incident == 0
            || hit.incident > world.combat.incidents
            || hit.physical_tick > world.combat.physical_tick()
            || hit.victim_entity
                != if expected == OrdinaryTarget::MainStation {
                    1
                } else {
                    0
                }
            || !hit.matches_flight(m.flight)
        {
            return Err(INVALID);
        }
        Ok(())
    }
    pub(in crate::flight) fn validate(
        &self,
        world: &World,
        practice: Option<super::super::practice::Practice>,
        history: &[super::super::practice::Practice],
        dock_sequence: u64,
    ) -> Result<(), SaveError> {
        if self.projectiles.len() < 3
            || self.projectiles.len() > 256
            || self.motions.len() != self.projectiles.len()
            || self.impacts.len() != self.projectiles.len()
            || self.beams.len() != self.projectiles.len()
            || self.npc_launches.len() != self.projectiles.len()
            || self
                .motions
                .iter()
                .flatten()
                .any(|m| m.bodies.len() > self.projectiles.len())
            || self
                .beams
                .iter()
                .flatten()
                .any(|b| b.bodies.len() > self.projectiles.len())
        {
            return Err(INVALID);
        }
        let owner = world.combat_identity().ok_or(INVALID)?;
        let snapshot = world.missile_snapshot().ok_or(INVALID)?;
        let used = self.used();
        if self.projectiles.len() != used.max(3) {
            return Err(INVALID);
        }
        let mut prior: Option<MissileOrigin> = None;
        self.validate_launcher(
            world,
            practice,
            core::array::from_fn(|i| {
                if i < 3 {
                    snapshot.rounds[i]
                } else {
                    snapshot.additional_rounds[i - 3]
                }
            }),
            used,
        )?;
        for i in 0..self.projectiles.len() {
            let Some(p) = self.projectiles[i] else {
                if i < used
                    || self.motions[i].is_some()
                    || self.impacts[i].is_some()
                    || self.beams[i].is_some()
                    || self.npc_launches[i].is_some()
                {
                    return Err(INVALID);
                }
                continue;
            };
            if world.traffic.enabled()
                && (self.motions[i]
                    .as_ref()
                    .is_some_and(|m| m.npc_scene.is_none())
                    || self.beams[i]
                        .as_ref()
                        .is_some_and(|b| b.npc_scene.is_none()))
            {
                return Err(INVALID);
            }
            world.ecm.validate_terminal(world, p)?;
            let o = p.origin;
            Self::validate_projectile_identity(world, owner, p, o, i, used)?;
            Self::validate_launch_order(o, prior)?;
            prior = Some(o);
            if let MissileTarget::Npc(id) = p.target {
                self.validate_npc_projectile(world, p, i, id)?;
                self.validate_contact_receipts(world, p, i)?;
                self.validate_unique_receipts(i)?;
                continue;
            }
            if self.npc_launches[i].is_some() {
                return Err(INVALID);
            }
            let lesson = lesson_for_target(practice, history, p.target).ok_or(INVALID)?;
            Self::validate_launch_context(world, owner, lesson, p)?;
            self.validate_motion(world, lesson, p, i)?;
            self.validate_end(world, owner, lesson, p, i)?;
            self.validate_contact_receipts(world, p, i)?;
            self.validate_unique_receipts(i)?;
        }
        self.validate_shields(world, practice, history, dock_sequence)?;
        self.validate_cooldown(world, practice, history, dock_sequence, prior)?;
        for lesson in lessons(practice, history) {
            self.validate_lesson_receipts(lesson)?;
        }
        Ok(())
    }
    fn validate_launcher(
        &self,
        world: &World,
        practice: Option<super::super::practice::Practice>,
        rounds: [Option<MissileRound>; 9],
        used: usize,
    ) -> Result<(), SaveError> {
        if self.cooldown > MISSILE_COOLDOWN_TICKS
            || (self.armed && rounds.iter().all(Option::is_none))
        {
            return Err(INVALID);
        }
        if self.lock.is_some_and(|id| match id {
            MissileTarget::Practice(id) => practice.and_then(|p| p.missile_target(id)).is_none(),
            MissileTarget::Npc(id) => world
                .traffic
                .missile_scene(
                    id.system(),
                    world.state.tick,
                    crate::traffic::SceneStage::Command(world.combat.emissions),
                )
                .bodies
                .iter()
                .all(|b| b.id != id),
        }) || ((self.armed || self.lock.is_some()) && world.state.mode != FlightMode::Flight)
        {
            return Err(INVALID);
        }
        // Custody is replayed against the actual finite born and paid item graph by
        // World validation. This launcher predicate retains arm/lock/cooldown checks.
        let _ = used;

        Ok(())
    }

    fn validate_projectile_identity(
        world: &World,
        owner: CombatIdentity,
        p: ProjectileSnapshot,
        o: MissileOrigin,
        i: usize,
        used: usize,
    ) -> Result<(), SaveError> {
        if i >= used
            || o.projectile
                != (ProjectileId {
                    owner: owner.owner,
                    visit: o.owner.visit,
                    ordinal: i as u8,
                })
            || o.owner.owner != owner.owner
            || o.owner.hull != owner.hull
            || (matches!(p.target, MissileTarget::Practice(_))
                && o.owner.system != SystemId::AVENOR)
            || o.owner.visit < 2
            || o.owner.visit > owner.visit
            || (o.owner.visit == owner.visit && o.owner.system != owner.system)
            || o.legal_at_launch > world.legal_value().ok_or(INVALID)?
            || o.emission == 0
            || o.emission > world.combat.emissions
            || o.launched_tick == 0
            || o.launched_tick > world.state.tick
            || o.physical_tick > o.launched_tick
            || o.physical_tick > world.combat.physical_tick()
            || !valid_vector(o.launch_position)
            || length(o.launch_position) > 30000.0 + o.launched_tick as f64 * 4.0
            || !valid_basis(o.orientation)
            || o.muzzle
                != add(
                    o.launch_position,
                    scale(o.orientation.z, MISSILE_MUZZLE_DISTANCE),
                )
            || !valid_vector(p.position)
            || !valid_vector(p.direction)
            || (length(p.direction) - 1.0).abs() > 1e-9
            || !p.propulsion_travel.is_finite()
            || !(0.0..=MISSILE_RANGE).contains(&p.propulsion_travel)
            || p.age_ticks > 455
            || p.last_tick < o.launched_tick
            || p.last_tick > world.state.tick
            || length(sub(p.position, o.muzzle)) > p.propulsion_travel + 1e-7
        {
            return Err(INVALID);
        }

        Ok(())
    }

    fn validate_launch_order(
        o: MissileOrigin,
        prior: Option<MissileOrigin>,
    ) -> Result<(), SaveError> {
        if let Some(previous) = prior
            && (o.emission <= previous.emission
                || o.launched_tick <= previous.launched_tick
                || o.physical_tick <= previous.physical_tick
                || o.owner.visit < previous.owner.visit
                || (o.owner.visit == previous.owner.visit
                    && (o.launched_tick - previous.launched_tick
                        < u64::from(MISSILE_COOLDOWN_TICKS)
                        || o.physical_tick - previous.physical_tick
                            < u64::from(MISSILE_COOLDOWN_TICKS))))
        {
            return Err(INVALID);
        }

        Ok(())
    }

    fn validate_launch_context(
        world: &World,
        owner: CombatIdentity,
        lesson: super::super::practice::Practice,
        p: ProjectileSnapshot,
    ) -> Result<(), SaveError> {
        let o = p.origin;
        if !lesson.missile_owner_matches(o.owner)
            || lesson
                .retained_missile_target(p.target.practice().ok_or(INVALID)?)
                .is_none()
            || !lesson.missile_provider_interval(
                o.launched_tick,
                o.launched_tick
                    .checked_add(u64::from(p.age_ticks))
                    .ok_or(INVALID)?,
            )
        {
            return Err(INVALID);
        }
        let target = lesson
            .retained_missile_target(p.target.practice().ok_or(INVALID)?)
            .ok_or(INVALID)?;
        if sub(target.position, o.launch_position).dot(o.orientation.z) <= 0.0 {
            return Err(INVALID);
        }
        if o.launched_tick == world.state.tick {
            let ship = super::super::flight_pose(world.state.position, world.state.orientation)
                .ok_or(INVALID)?;
            // Same-tick commands may fire Pulse or pause, but cannot move the craft
            // or advance the physical clock. Historical origins stay independent.
            if o.launch_position != ship.position
                || o.orientation != ship.orientation
                || o.physical_tick != world.combat.physical_tick()
                || o.owner != owner
                || world.state.mode != FlightMode::Flight
            {
                return Err(INVALID);
            }
        }

        Ok(())
    }

    fn validate_end(
        &self,
        world: &World,
        owner: CombatIdentity,
        lesson: super::super::practice::Practice,
        p: ProjectileSnapshot,
        i: usize,
    ) -> Result<(), SaveError> {
        match p.ended {
            None => {
                if p.last_tick != world.state.tick
                    || p.origin.owner != owner
                    || world.state.mode != FlightMode::Flight
                    || lesson
                        .missile_target(p.target.practice().ok_or(INVALID)?)
                        .is_none()
                {
                    return Err(INVALID);
                }
            }
            Some(MissileEnd::RangeExpired) => {
                if p.propulsion_travel != MISSILE_RANGE || p.age_ticks != 455 {
                    return Err(INVALID);
                }
            }
            Some(MissileEnd::LifetimeExpired) => return Err(INVALID), // finite source range ends first
            Some(MissileEnd::TargetLost) => {
                if lesson
                    .missile_provider_ended_at()
                    .is_none_or(|tick| p.last_tick != tick && p.last_tick != tick.saturating_add(1))
                {
                    return Err(INVALID);
                }
            }
            Some(MissileEnd::Culled) => {
                if !lesson.missile_retired_at(p.last_tick) {
                    return Err(INVALID);
                }
            }
            Some(MissileEnd::Impact) => {}
            Some(MissileEnd::Ecm) => world.ecm.validate_terminal(world, p)?,
            Some(MissileEnd::Intercepted) => {
                self.validate_interception(world, Some(lesson), p, i)?
            }
        }

        Ok(())
    }

    fn validate_contact_receipts(
        &self,
        world: &World,
        p: ProjectileSnapshot,
        i: usize,
    ) -> Result<(), SaveError> {
        let o = p.origin;
        if p.ended != Some(MissileEnd::Intercepted)
            && (self.beams[i].is_some() || p.intercepting_emission.is_some())
        {
            return Err(INVALID);
        }
        if let Some(MissileContact::Projectile(id)) = p.contact {
            let victim = self.find(id).ok_or(INVALID)?;
            if victim.ended != Some(MissileEnd::Intercepted)
                || victim.last_tick != p.last_tick
                || victim.intercepting_emission != Some(o.emission)
            {
                return Err(INVALID);
            }
        }
        let ordinary = matches!(
            p.contact,
            Some(MissileContact::Station | MissileContact::Player { .. })
        );
        if ordinary != self.impacts[i].is_some() {
            return Err(INVALID);
        }
        if let Some(hit) = self.impacts[i] {
            self.validate_ordinary(hit, world)?;
        }

        Ok(())
    }

    fn validate_unique_receipts(&self, i: usize) -> Result<(), SaveError> {
        for j in 0..i {
            if self.beams[i].as_ref().is_some_and(|b| {
                self.beams[j]
                    .as_ref()
                    .is_some_and(|a| a.emission == b.emission)
            }) {
                return Err(INVALID);
            }
            if let (Some(a), Some(b)) = (self.impacts[j], self.impacts[i])
                && (a.incident == b.incident
                    || (a.tick < b.tick && a.incident > b.incident)
                    || (a.tick > b.tick && a.incident < b.incident))
            {
                return Err(INVALID);
            }
        }

        Ok(())
    }

    fn validate_shields(
        &self,
        world: &World,
        practice: Option<super::super::practice::Practice>,
        history: &[super::super::practice::Practice],
        dock_sequence: u64,
    ) -> Result<(), SaveError> {
        if world.damage.enabled() {
            world.validate_practice_incidents(practice, history)?;
            return world.damage.validate_resources(world, dock_sequence);
        }
        // The only admitted shield damage has all of its bounded missile receipts retained.
        // Check the numeric result; loading never applies these effects to the World.
        let mut shields = [30_000_u16; 2];
        {
            let mut hits = self.retained_player_missile_hits(practice, history, dock_sequence);
            hits.sort_by_key(|h| h.incident);
            let mut clock = 0;
            for hit in hits {
                if let OrdinaryTarget::Player { front } = hit.target {
                    if hit.physical_tick < clock {
                        return Err(INVALID);
                    }
                    let elapsed = hit.physical_tick - clock;
                    for shield in &mut shields {
                        *shield = u64::from(*shield)
                            .saturating_add(elapsed.saturating_mul(3))
                            .min(30_000) as u16;
                    }
                    let side = usize::from(!front);
                    shields[side] = shields[side].saturating_sub(16_000);
                    clock = hit.physical_tick;
                }
            }
            let elapsed = world
                .combat
                .physical_tick()
                .checked_sub(clock)
                .ok_or(INVALID)?;
            for shield in &mut shields {
                *shield = u64::from(*shield)
                    .saturating_add(elapsed.saturating_mul(3))
                    .min(30_000) as u16;
            }
        }
        let condition = world.combat_snapshot().ok_or(INVALID)?.player;
        if [
            condition.front_shield_hundredths,
            condition.rear_shield_hundredths,
        ] != shields
        {
            return Err(INVALID);
        }

        Ok(())
    }

    fn validate_cooldown(
        &self,
        world: &World,
        practice: Option<super::super::practice::Practice>,
        history: &[super::super::practice::Practice],
        dock_sequence: u64,
        prior: Option<MissileOrigin>,
    ) -> Result<(), SaveError> {
        let expected_cooldown = if world.state.mode == FlightMode::Docked
            || self
                .projectiles
                .iter()
                .rev()
                .find_map(|p| *p)
                .is_some_and(|p| self.docked_since(p, practice, history, dock_sequence))
        {
            0
        } else {
            prior.map_or(0, |o| {
                u64::from(MISSILE_COOLDOWN_TICKS)
                    .saturating_sub(world.combat.physical_tick() - o.physical_tick)
                    as u8
            })
        };
        if self.cooldown != expected_cooldown {
            return Err(INVALID);
        }

        Ok(())
    }

    fn validate_lesson_receipts(
        &self,
        lesson: super::super::practice::Practice,
    ) -> Result<(), SaveError> {
        if lesson.pending_missiles != self.pending_for_lesson(lesson) {
            return Err(INVALID);
        }
        let projectiles = self
            .projectiles
            .iter()
            .map(|p| p.filter(|p| lesson.missile_owner_matches(p.origin.owner)))
            .collect::<Vec<_>>();
        for (id, receipt) in lesson.missile_receipts() {
            if let Some((tick, origin)) = receipt {
                let p = self.find(origin.projectile).ok_or(INVALID)?;
                if p.origin != origin
                    || p.last_tick != tick
                    || p.contact
                        != Some(MissileContact::Practice {
                            id,
                            first_hit: true,
                        })
                {
                    return Err(INVALID);
                }
            }
        }
        for p in projectiles.iter().flatten() {
            if let Some(MissileContact::Practice {
                id,
                first_hit: true,
            }) = p.contact
                && !lesson
                    .missile_receipts()
                    .contains(&(id, Some((p.last_tick, p.origin))))
            {
                return Err(INVALID);
            }
        }
        for hit in self.impacts.iter().copied().flatten() {
            if lesson.accepts_incident(hit)
                && lesson.ordinary_hit.is_none_or(|first| {
                    first.incident > hit.incident
                        || (first.incident == hit.incident && first != hit)
                })
            {
                return Err(INVALID);
            }
        }
        lesson.validate_missile_quiet(&projectiles)?;

        Ok(())
    }

    fn validate_motion(
        &self,
        world: &World,
        lesson: super::super::practice::Practice,
        p: ProjectileSnapshot,
        i: usize,
    ) -> Result<(), SaveError> {
        let o = p.origin;
        let motion_tick = o
            .launched_tick
            .checked_add(u64::from(p.age_ticks))
            .ok_or(INVALID)?;
        Self::validate_motion_clock(p, motion_tick)?;
        if p.age_ticks == 0 {
            return self.validate_newborn_motion(p, i);
        }
        let m = self.motions[i].as_ref().ok_or(INVALID)?;
        self.validate_motion_record(world, lesson, p, i, m, motion_tick)?;
        let target = lesson
            .retained_missile_target(p.target.practice().ok_or(INVALID)?)
            .ok_or(INVALID)?;
        let direction = unit(sub(target.position, m.position)).ok_or(INVALID)?;
        if direction != p.direction {
            return Err(INVALID);
        }
        let player =
            super::super::flight_pose(m.flight.position, m.flight.orientation).ok_or(INVALID)?;
        let data = world.station_data.as_ref().ok_or(INVALID)?;
        let cleared = m.cleared
            || length(sub(m.position, player.position)) > data.player_clearance_radius() + 1e-6;
        let travel = (MISSILE_SPEED / 60.0).min((MISSILE_RANGE - m.travel).max(0.0));
        let ray = pose(m.position, direction).ok_or(INVALID)?;
        let (station, mut best, player_contact) = retained_solid_contact(
            data,
            world.state.station.ok_or(INVALID)?,
            ray,
            travel,
            player,
            (cleared, m.player_alive),
            INVALID,
        )?;
        if let Some(d) = player_contact {
            choose(
                &mut best,
                d,
                MissileContact::Player {
                    front: sub(
                        add(ray.position, scale(ray.orientation.z, d)),
                        player.position,
                    )
                    .dot(player.orientation.z)
                        >= 0.0,
                },
            );
        }
        self.validate_motion_bodies(p, i, m, ray, travel, &mut best)?;
        if let Some((id, d)) = lesson.retained_missile_contact(
            ray,
            best.map_or(travel, |(d, _)| d.min(travel)),
            station,
        ) {
            choose(
                &mut best,
                d,
                MissileContact::Practice {
                    id,
                    first_hit: lesson
                        .missile_first_hit_status(id, o, p.last_tick)
                        .ok_or(INVALID)?,
                },
            );
        }
        if let Some(scene) = &m.npc_scene {
            if scene.world_tick != m.flight.tick
                || scene.stage != crate::traffic::SceneStage::Step(i as u8)
                || scene.system != p.origin.owner.system
            {
                return Err(INVALID);
            }
            world
                .traffic
                .validate_missile_scene(scene, world.state.tick, data)?;
            super::npc::choose_scene(world, &p, scene, ray, travel, &mut best)
                .map_err(|_| INVALID)?;
        }
        let used = best.map_or(travel, |(d, _)| d);
        if p.position != super::projectile_point(p, ray, used, best.map(|(_, contact)| contact))
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
        if p.ended.is_none()
            && (p.propulsion_travel >= MISSILE_RANGE || p.age_ticks >= MISSILE_LIFETIME_TICKS)
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_motion_clock(p: ProjectileSnapshot, motion_tick: u64) -> Result<(), SaveError> {
        let no_motion_end = matches!(p.ended, Some(MissileEnd::Culled | MissileEnd::TargetLost));
        let intercept = p.ended == Some(MissileEnd::Intercepted);
        let expected_tick = if no_motion_end {
            motion_tick.checked_add(1).ok_or(INVALID)?
        } else {
            motion_tick
        };
        if (!intercept && p.last_tick != expected_tick)
            || (intercept
                && (p.last_tick < motion_tick || p.last_tick > motion_tick.saturating_add(1)))
            || (p.ended == Some(MissileEnd::Impact)) != p.contact.is_some()
        {
            return Err(INVALID);
        }

        Ok(())
    }

    pub(super) fn validate_newborn_motion(
        &self,
        p: ProjectileSnapshot,
        i: usize,
    ) -> Result<(), SaveError> {
        let o = p.origin;
        if self.motions[i].is_some()
            || p.position != o.muzzle
            || p.direction != o.orientation.z
            || p.propulsion_travel != 0.0
            || p.muzzle_cleared
        {
            return Err(INVALID);
        }

        Ok(())
    }

    fn validate_motion_record(
        &self,
        world: &World,
        lesson: super::super::practice::Practice,
        p: ProjectileSnapshot,
        i: usize,
        m: &Motion,
        motion_tick: u64,
    ) -> Result<(), SaveError> {
        let o = p.origin;
        super::super::persistence::validate_flight(&m.flight)?;
        if m.flight.tick != motion_tick
            || m.flight.mode != FlightMode::Flight
            || m.age.checked_add(1) != Some(p.age_ticks)
            || !valid_vector(m.position)
            || !m.travel.is_finite()
            || (m.travel - f64::from(m.age) * (MISSILE_SPEED / 60.0)).abs() > 1e-7
            || length(sub(m.position, o.muzzle)) > m.travel + 1e-7
            || (m.flight.tick == world.state.tick
                && (m.flight.position != world.state.position
                    || m.flight.orientation != world.state.orientation))
        {
            return Err(INVALID);
        }
        if let Some(hit) = lesson.ordinary_hit
            && hit.weapon == OrdinaryWeapon::StandardMissile
            && hit.tick == m.flight.tick
            && self
                .impacts
                .iter()
                .position(|h| *h == Some(hit))
                .is_some_and(|slot| slot < i)
        {
            return Err(INVALID);
        }

        Ok(())
    }

    fn motion_body_expected(&self, slot: usize, i: usize, m: &Motion) -> bool {
        self.projectiles[slot].is_some_and(|other| {
            if other.origin.launched_tick >= m.flight.tick {
                return false;
            }
            match other.ended {
                None => true,
                Some(_) if other.last_tick > m.flight.tick => true,
                Some(_) if other.last_tick < m.flight.tick => false,
                Some(MissileEnd::Ecm) => true,
                Some(MissileEnd::Intercepted) => {
                    if self.beams[slot].is_some() {
                        return true;
                    }
                    self.projectiles
                        .iter()
                        .flatten()
                        .find(|p| Some(p.origin.emission) == other.intercepting_emission)
                        .is_some_and(|p| usize::from(p.origin.projectile.ordinal) >= i)
                }
                Some(_) => slot >= i,
            }
        })
    }

    pub(super) fn validate_motion_bodies(
        &self,
        p: ProjectileSnapshot,
        i: usize,
        m: &Motion,
        ray: RigidPose,
        travel: f64,
        best: &mut Option<(f64, MissileContact)>,
    ) -> Result<(), SaveError> {
        let o = p.origin;
        for slot in m.bodies.len()..self.projectiles.len() {
            if self.motion_body_expected(slot, i, m) {
                return Err(INVALID);
            }
        }
        for (slot, body) in m.bodies.iter().copied().enumerate() {
            let expected = self.motion_body_expected(slot, i, m);

            if body.is_some() != expected {
                return Err(INVALID);
            }
            let Some(body) = body else {
                continue;
            };
            let other = self.find(body.id).ok_or(INVALID)?;
            if usize::from(body.id.ordinal) != slot
                || other.origin.launched_tick > m.flight.tick
                || (other.ended.is_some() && other.last_tick < m.flight.tick)
                || !valid_vector(body.position)
                || length(sub(body.position, other.origin.muzzle)) > other.propulsion_travel + 1e-7
            {
                return Err(INVALID);
            }
            if slot == i {
                if body.position != m.position {
                    return Err(INVALID);
                }
                continue;
            }
            if other.origin.owner.visit == o.owner.visit
                && let Some(d) = mesh_distance(
                    RigidPose {
                        position: body.position,
                        orientation: other.origin.orientation,
                    },
                    ray,
                    travel,
                )
            {
                choose(best, d, MissileContact::Projectile(body.id));
            }
        }
        if m.bodies.get(i).copied().flatten().is_none() {
            return Err(INVALID);
        }

        Ok(())
    }

    pub(super) fn validate_interception(
        &self,
        world: &World,
        lesson: Option<super::super::practice::Practice>,
        p: ProjectileSnapshot,
        i: usize,
    ) -> Result<(), SaveError> {
        let emission = p.intercepting_emission.ok_or(INVALID)?;
        if emission == 0
            || emission == p.origin.emission
            || emission > world.combat.emissions
            || p.contact.is_some()
        {
            return Err(INVALID);
        }
        if let Some(b) = self.beams[i].as_ref() {
            let shot = b.shot;
            let PulseContact::Projectile { id, distance } = shot.contact else {
                return Err(INVALID);
            };
            self.validate_beam_identity(world, b, p, emission, id)?;
            let mut ray = pose(shot.origin, shot.direction).ok_or(INVALID)?;
            // Pulse uses the exact quaternion-derived forward column without normalization.
            ray.orientation.z = shot.direction;
            let d = mesh_distance(
                RigidPose {
                    position: p.position,
                    orientation: p.origin.orientation,
                },
                ray,
                super::super::PULSE_RANGE,
            )
            .ok_or(INVALID)?;
            if distance != d || shot.end != add(shot.origin, scale(shot.direction, d)) {
                return Err(INVALID);
            }
            self.validate_intercepted_bodies(b, p, i, emission, (ray, d, id))?;
            let station = world
                .station_data
                .as_ref()
                .ok_or(INVALID)?
                .beam_block_distance(
                    world.state.station.ok_or(INVALID)?,
                    shot.origin,
                    shot.direction,
                    super::super::PULSE_RANGE,
                )
                .map_err(|_| INVALID)?;
            if let Some(scene) = &b.npc_scene {
                if scene.world_tick != shot.tick
                    || scene.system != p.origin.owner.system
                    || scene.stage != crate::traffic::SceneStage::Command(emission - 1)
                {
                    return Err(INVALID);
                }
                world.traffic.validate_missile_scene(
                    scene,
                    world.state.tick,
                    world.station_data.as_ref().ok_or(INVALID)?,
                )?;
                let mut blocker = None;
                super::npc::choose_scene(world, &p, scene, ray, d, &mut blocker)
                    .map_err(|_| INVALID)?;
                if blocker.is_some() {
                    return Err(INVALID);
                }
            }
            if station.is_some_and(|s| s <= d)
                || lesson
                    .and_then(|lesson| lesson.retained_missile_contact(ray, d, station))
                    .is_some_and(|(_, s)| s <= d)
            {
                return Err(INVALID);
            }
        } else if !self.projectiles.iter().flatten().any(|other| {
            other.origin.emission == emission
                && other.last_tick == p.last_tick
                && other.contact == Some(MissileContact::Projectile(p.origin.projectile))
        }) {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_intercepted_bodies(
        &self,
        b: &Beam,
        p: ProjectileSnapshot,
        i: usize,
        emission: u64,
        (ray, d, id): (RigidPose, f64, ProjectileId),
    ) -> Result<(), SaveError> {
        let shot = b.shot;
        for other in self.projectiles.iter().skip(b.bodies.len()).flatten() {
            if other.origin.emission < emission && beam_body_before_ecm(*other, shot.tick, emission)
            {
                return Err(INVALID);
            }
        }
        for (slot, body) in b.bodies.iter().copied().enumerate() {
            let expected = self.projectiles[slot].is_some_and(|other| {
                other.origin.emission < emission
                    && (slot == i || beam_body_before_ecm(other, shot.tick, emission))
            });
            if body.is_some() != expected {
                return Err(INVALID);
            }
            if let Some(body) = body {
                self.validate_beam_body(body, slot, i, p.position, ray, (d, id))?;
            }
        }
        Ok(())
    }
    fn validate_beam_identity(
        &self,
        world: &World,
        b: &Beam,
        p: ProjectileSnapshot,
        emission: u64,
        id: ProjectileId,
    ) -> Result<(), SaveError> {
        let shot = b.shot;
        if self
            .projectiles
            .iter()
            .flatten()
            .any(|p| p.origin.emission == emission)
        {
            return Err(INVALID);
        }
        if b.emission != emission
            || emission <= p.origin.emission
            || shot.tick != p.last_tick
            || world
                .laser_weapon_at_tick(shot.fitted_item_id, shot.tick)
                .is_none()
            || id != p.origin.projectile
            || !valid_vector(shot.origin)
            || !valid_vector(shot.direction)
            || (length(shot.direction) - 1.0).abs() > 1e-9
        {
            return Err(INVALID);
        }
        Ok(())
    }

    fn validate_beam_body(
        &self,
        body: Body,
        slot: usize,
        i: usize,
        position: Vec3,
        ray: RigidPose,
        nearest: (f64, ProjectileId),
    ) -> Result<(), SaveError> {
        let (d, id) = nearest;

        let other = self.find(body.id).ok_or(INVALID)?;
        if usize::from(body.id.ordinal) != slot
            || !valid_vector(body.position)
            || length(sub(body.position, other.origin.muzzle)) > other.propulsion_travel + 1e-7
        {
            return Err(INVALID);
        }
        if slot == i {
            if body.position != position {
                return Err(INVALID);
            }
        } else if mesh_distance(
            RigidPose {
                position: body.position,
                orientation: other.origin.orientation,
            },
            ray,
            d,
        )
        .is_some_and(|near| near < d || (near == d && body.id.entity() < id.entity()))
        {
            return Err(INVALID);
        }

        Ok(())
    }
}
fn valid_vector(v: Vec3) -> bool {
    [v.x, v.y, v.z]
        .iter()
        .all(|n| n.is_finite() && n.abs() <= 1e12)
}
fn valid_basis(b: Basis3) -> bool {
    b.is_valid_rotation()
        && valid_vector(b.x)
        && valid_vector(b.y)
        && valid_vector(b.z)
        && (b.x.dot(cross(b.y, b.z)) - 1.0).abs() < 1e-5
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn read_bodies(
    r: &mut Reader<'_>,
    version: u16,
    count: usize,
) -> Result<Vec<Option<Body>>, SaveError> {
    let len = if version >= 12 {
        usize::from(r.u16()?)
    } else {
        3
    };
    if len < 3 || len > count {
        return Err(INVALID);
    }
    (0..len)
        .map(|_| {
            r.option(|r| {
                Ok(Body {
                    id: ProjectileId::decode(r)?,
                    position: read_vector(r)?,
                })
            })
        })
        .collect()
}

fn write_target(w: &mut Writer, target: MissileTarget, version: u16) {
    match target {
        MissileTarget::Practice(id) => {
            if version >= 15 {
                w.u8(0);
            }
            id.encode(w);
        }
        MissileTarget::Npc(id) => {
            w.u8(1);
            id.encode(w);
        }
    }
}
fn read_target(r: &mut Reader<'_>, version: u16) -> Result<MissileTarget, SaveError> {
    if version < 15 {
        return Ok(MissileTarget::Practice(PracticeBeaconId::decode(r)?));
    }
    match r.u8()? {
        0 => Ok(MissileTarget::Practice(PracticeBeaconId::decode(r)?)),
        1 => Ok(MissileTarget::Npc(crate::traffic::NpcId::decode(r)?)),
        _ => Err(INVALID),
    }
}
impl State {
    pub(super) fn docked_since(
        &self,
        p: ProjectileSnapshot,
        practice: Option<super::super::practice::Practice>,
        history: &[super::super::practice::Practice],
        sequence: u64,
    ) -> bool {
        match p.target {
            MissileTarget::Practice(_) => lesson_for_target(practice, history, p.target)
                .is_some_and(|l| l.missile_docked_since_consent(sequence)),
            MissileTarget::Npc(_) => self
                .npc_launches
                .get(usize::from(p.origin.projectile.ordinal))
                .and_then(|e| e.as_ref())
                .is_some_and(|e| sequence > e.dock_sequence),
        }
    }
}

fn retained_beam_body(projectile: ProjectileSnapshot, hit: &super::RetainedBeam<'_>) -> bool {
    if projectile.origin.owner.visit != hit.visit || projectile.origin.owner.system != hit.system {
        return false;
    }
    let born = projectile.origin.launched_tick < hit.world_tick
        || (projectile.origin.launched_tick == hit.world_tick
            && hit
                .command_emission
                .is_some_and(|emission| projectile.origin.emission < emission));
    let live = projectile.ended.is_none()
        || projectile.last_tick > hit.world_tick
        || (projectile.ended == Some(MissileEnd::Ecm)
            && projectile.last_tick == hit.world_tick
            && hit.command_emission.is_none_or(|emission| {
                projectile
                    .ecm
                    .is_some_and(|end| end.after_emission >= emission)
            }))
        || (projectile.ended == Some(MissileEnd::Intercepted)
            && projectile.last_tick == hit.world_tick
            && hit.command_emission.is_none_or(|emission| {
                projectile
                    .intercepting_emission
                    .is_some_and(|end| end > emission)
            }));
    born && live
}

/// A manual ECM command follows the already accepted attack emissions at that tick.
fn beam_body_before_ecm(projectile: ProjectileSnapshot, tick: u64, emission: u64) -> bool {
    projectile.ended.is_none()
        || projectile.last_tick > tick
        || (projectile.ended == Some(MissileEnd::Ecm)
            && projectile.last_tick == tick
            && projectile
                .ecm
                .is_some_and(|end| end.after_emission >= emission))
}

#[cfg(test)]
mod solid_contact_tests;

#[cfg(test)]
mod legacy_motion_tests {
    use super::*;

    #[test]
    fn literal_v15_missile_history_refuses_before_candidate_replacement() {
        const V15: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/qualified-v15-npc-flight.bin"
        ));
        assert!(matches!(
            World::from_save_bytes(V15),
            Err(SaveError::UnsupportedVersion)
        ));
        let mut live = World::new_commander(15920).unwrap_or_else(|error| panic!("{error:?}"));
        let before = live
            .save_bytes()
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(live.restore_bytes(V15), Err(SaveError::UnsupportedVersion));
        assert_eq!(
            live.save_bytes()
                .unwrap_or_else(|error| panic!("{error:?}")),
            before
        );
    }
}

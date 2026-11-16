//! Version-eight bounded state and retained contact evidence. Decode never emits gameplay events.
use super::*;
use crate::docking::{Basis3, RigidPose};
use crate::flight::{OrdinaryImpact, OrdinaryTarget, OrdinaryWeapon, PulseContact, PulseShot};
use crate::navigation::SystemId;
use crate::save::{Reader, SaveError, Writer};
const INVALID: SaveError = SaveError::InvalidState("missile custody, motion or contact evidence");
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
    fn encode(self, w: &mut Writer) {
        w.u64(self.owner);
        w.u64(self.visit);
        w.u8(self.ordinal);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            owner: r.u64()?,
            visit: r.u64()?,
            ordinal: r.u8()?,
        })
    }
}
impl MissileOrigin {
    pub(in crate::flight) fn encode(self, w: &mut Writer) {
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
    pub(in crate::flight) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
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
        MissileContact::Player { front } => {
            w.u8(2);
            w.bool(front);
        }
        MissileContact::Projectile(id) => {
            w.u8(3);
            id.encode(w);
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
        2 => MissileContact::Player { front: r.bool()? },
        3 => MissileContact::Projectile(ProjectileId::decode(r)?),
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
        _ => return Err(SaveError::InvalidEncoding),
    })
}
fn projectile(w: &mut Writer, p: &ProjectileSnapshot) {
    p.origin.encode(w);
    p.target.encode(w);
    vector(w, p.position);
    vector(w, p.direction);
    scalar(w, p.propulsion_travel);
    w.u16(p.age_ticks);
    w.bool(p.muzzle_cleared);
    w.option(p.ended.as_ref(), |w, e| w.u8(end_tag(*e)));
    w.option(p.contact.as_ref(), |w, c| contact(w, *c));
    w.u64(p.last_tick);
    w.option(p.intercepting_emission.as_ref(), |w, e| w.u64(*e));
}
fn read_projectile(r: &mut Reader<'_>) -> Result<ProjectileSnapshot, SaveError> {
    Ok(ProjectileSnapshot {
        origin: MissileOrigin::decode(r)?,
        target: PracticeBeaconId::decode(r)?,
        position: read_vector(r)?,
        direction: read_vector(r)?,
        propulsion_travel: number(r)?,
        age_ticks: r.u16()?,
        muzzle_cleared: r.bool()?,
        ended: r.option(read_end)?,
        contact: r.option(read_contact)?,
        last_tick: r.u64()?,
        intercepting_emission: r.option(|r| r.u64())?,
    })
}
impl State {
    pub(in crate::flight) fn encode(self, w: &mut Writer) {
        w.bool(self.armed);
        w.option(self.lock.as_ref(), |w, id| id.encode(w));
        w.u8(self.cooldown);
        for i in 0..3 {
            w.option(self.projectiles[i].as_ref(), projectile);
            w.option(self.motions[i].as_ref(), |w, m| {
                vector(w, m.position);
                scalar(w, m.travel);
                w.u16(m.age);
                w.bool(m.cleared);
                super::super::persistence::write_flight(w, &m.flight);
                w.bool(m.player_alive);
                for body in m.bodies {
                    w.option(body.as_ref(), |w, b| {
                        b.id.encode(w);
                        vector(w, b.position);
                    });
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
                for body in b.bodies {
                    w.option(body.as_ref(), |w, b| {
                        b.id.encode(w);
                        vector(w, b.position);
                    });
                }
                if let PulseContact::Projectile { id, distance } = s.contact {
                    id.encode(w);
                    scalar(w, distance);
                }
            });
        }
    }
    pub(in crate::flight) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let mut state = Self {
            armed: r.bool()?,
            lock: r.option(PracticeBeaconId::decode)?,
            cooldown: r.u8()?,
            ..Self::default()
        };
        for i in 0..3 {
            state.projectiles[i] = r.option(read_projectile)?;
            state.motions[i] = r.option(|r| {
                Ok(Motion {
                    position: read_vector(r)?,
                    travel: number(r)?,
                    age: r.u16()?,
                    cleared: r.bool()?,
                    flight: super::super::persistence::read_flight(r)?,
                    player_alive: r.bool()?,
                    bodies: r.array(|r| {
                        r.option(|r| {
                            Ok(Body {
                                id: ProjectileId::decode(r)?,
                                position: read_vector(r)?,
                            })
                        })
                    })?,
                })
            })?;
            state.impacts[i] = r.option(OrdinaryImpact::decode)?;
            state.beams[i] = r.option(|r| {
                let emission = r.u64()?;
                let tick = r.u64()?;
                let fitted_item_id = r.u64()?;
                let origin = read_vector(r)?;
                let direction = read_vector(r)?;
                let end = read_vector(r)?;
                let bodies = r.array(|r| {
                    r.option(|r| {
                        Ok(Body {
                            id: ProjectileId::decode(r)?,
                            position: read_vector(r)?,
                        })
                    })
                })?;
                Ok(Beam {
                    bodies,
                    emission,
                    shot: PulseShot {
                        tick,
                        fitted_item_id,
                        origin,
                        direction,
                        end,
                        contact: PulseContact::Projectile {
                            id: ProjectileId::decode(r)?,
                            distance: number(r)?,
                        },
                    },
                })
            })?;
        }
        Ok(state)
    }
    pub(in crate::flight) fn used(self) -> usize {
        self.projectiles.iter().flatten().count()
    }
    pub(in crate::flight) fn has_player_hit(self) -> bool {
        self.projectiles
            .iter()
            .flatten()
            .any(|p| matches!(p.contact, Some(MissileContact::Player { .. })))
    }
    fn find(self, id: ProjectileId) -> Option<ProjectileSnapshot> {
        self.projectiles
            .iter()
            .flatten()
            .find(|p| p.origin.projectile == id)
            .copied()
    }
    pub(in crate::flight) fn validate_ordinary(
        self,
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
        let m = self.motions[i].ok_or(INVALID)?;
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
        self,
        world: &World,
        practice: Option<super::super::practice::Practice>,
        dock_sequence: u64,
    ) -> Result<(), SaveError> {
        let owner = world.combat_identity().ok_or(INVALID)?;
        let snapshot = world.missile_snapshot().ok_or(INVALID)?;
        let used = self.used();
        let mut prior: Option<MissileOrigin> = None;
        if self.cooldown > MISSILE_COOLDOWN_TICKS
            || (self.armed && snapshot.rounds.iter().all(Option::is_none))
        {
            return Err(INVALID);
        }
        if self
            .lock
            .is_some_and(|id| practice.and_then(|p| p.missile_target(id)).is_none())
            || ((self.armed || self.lock.is_some()) && world.state.mode != FlightMode::Flight)
        {
            return Err(INVALID);
        }
        // Each actual removal has exactly one origin; there is no decode-time birth/refill.
        for (pylon, round) in snapshot.rounds.iter().enumerate() {
            if round.is_some() != (pylon < 3 - used) {
                return Err(INVALID);
            }
        }
        for i in 0..3 {
            let Some(p) = self.projectiles[i] else {
                if i < used
                    || self.motions[i].is_some()
                    || self.impacts[i].is_some()
                    || self.beams[i].is_some()
                {
                    return Err(INVALID);
                }
                continue;
            };
            let o = p.origin;
            if i >= used
                || o.projectile
                    != (ProjectileId {
                        owner: owner.owner,
                        visit: o.owner.visit,
                        ordinal: i as u8,
                    })
                || o.owner.owner != owner.owner
                || o.owner.hull != owner.hull
                || o.owner.system != SystemId::AVENOR
                || o.owner.visit < 2
                || o.owner.visit > owner.visit
                || (o.owner.visit == owner.visit && o.owner.system != owner.system)
                || o.item != owner.owner.checked_add(10 - i as u64).ok_or(INVALID)?
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
            if let Some(previous) = prior {
                if o.emission <= previous.emission
                    || o.launched_tick.saturating_sub(previous.launched_tick)
                        < u64::from(MISSILE_COOLDOWN_TICKS)
                    || o.physical_tick.saturating_sub(previous.physical_tick)
                        < u64::from(MISSILE_COOLDOWN_TICKS)
                    || o.owner != previous.owner
                {
                    return Err(INVALID);
                }
            }
            prior = Some(o);
            let lesson = practice.ok_or(INVALID)?;
            if !lesson.missile_owner_matches(o.owner)
                || lesson.retained_missile_target(p.target).is_none()
                || !lesson.missile_provider_interval(
                    o.launched_tick,
                    o.launched_tick
                        .checked_add(u64::from(p.age_ticks))
                        .ok_or(INVALID)?,
                )
            {
                return Err(INVALID);
            }
            let target = lesson.retained_missile_target(p.target).ok_or(INVALID)?;
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
            self.validate_motion(world, lesson, p, i)?;
            match p.ended {
                None => {
                    if p.last_tick != world.state.tick
                        || p.origin.owner != owner
                        || world.state.mode != FlightMode::Flight
                        || lesson.missile_target(p.target).is_none()
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
                    if lesson.missile_provider_ended_at().is_none_or(|tick| {
                        p.last_tick != tick && p.last_tick != tick.saturating_add(1)
                    }) {
                        return Err(INVALID);
                    }
                }
                Some(MissileEnd::Culled) => {
                    if !lesson.missile_retired_at(p.last_tick) {
                        return Err(INVALID);
                    }
                }
                Some(MissileEnd::Impact) => {}
                Some(MissileEnd::Intercepted) => self.validate_interception(world, lesson, p, i)?,
            }
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
            for j in 0..i {
                if self.beams[i]
                    .is_some_and(|b| self.beams[j].is_some_and(|a| a.emission == b.emission))
                {
                    return Err(INVALID);
                }
                if let (Some(a), Some(b)) = (self.impacts[j], self.impacts[i]) {
                    if a.incident == b.incident
                        || (a.tick < b.tick && a.incident > b.incident)
                        || (a.tick > b.tick && a.incident < b.incident)
                    {
                        return Err(INVALID);
                    }
                }
            }
        }
        // The only admitted shield damage has all of its bounded missile receipts retained.
        // Check the numeric result; loading never applies these effects to the World.
        let mut shields = [30_000_u16; 2];
        if !practice.is_some_and(|p| p.missile_docked_since_consent(dock_sequence)) {
            let mut hits = self.impacts;
            hits.sort_by_key(|h| h.map_or(u64::MAX, |h| h.incident));
            let mut clock = 0;
            for hit in hits.into_iter().flatten() {
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
        let expected_cooldown = if world.state.mode == FlightMode::Docked
            || practice.is_some_and(|p| p.missile_docked_since_consent(dock_sequence))
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
        if let Some(lesson) = practice {
            if lesson.pending_missiles != self.pending() {
                return Err(INVALID);
            }
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
            for p in self.projectiles.iter().flatten() {
                if let Some(MissileContact::Practice {
                    id,
                    first_hit: true,
                }) = p.contact
                {
                    if !lesson
                        .missile_receipts()
                        .contains(&(id, Some((p.last_tick, p.origin))))
                    {
                        return Err(INVALID);
                    }
                }
            }
            lesson.validate_missile_quiet(self.projectiles)?;
        } else if used != 0 {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_motion(
        self,
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
        if p.age_ticks == 0 {
            if self.motions[i].is_some()
                || p.position != o.muzzle
                || p.direction != o.orientation.z
                || p.propulsion_travel != 0.0
                || p.muzzle_cleared
            {
                return Err(INVALID);
            }
            return Ok(());
        }
        let m = self.motions[i].ok_or(INVALID)?;
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
        let target = lesson.retained_missile_target(p.target).ok_or(INVALID)?;
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
        let station = data
            .beam_block_distance(
                world.state.station.ok_or(INVALID)?,
                ray.position,
                ray.orientation.z,
                travel,
            )
            .map_err(|_| INVALID)?;
        let mut best = station.map(|d| (d, MissileContact::Station));
        if cleared
            && m.player_alive
            && let Some(d) = data
                .player_beam_distance(player, ray.position, ray.orientation.z, travel)
                .map_err(|_| INVALID)?
        {
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
        for (slot, body) in m.bodies.into_iter().enumerate() {
            let expected = self.projectiles[slot].is_some_and(|other| {
                if other.origin.launched_tick >= m.flight.tick {
                    return false;
                }
                match other.ended {
                    None => true,
                    Some(_) if other.last_tick > m.flight.tick => true,
                    Some(_) if other.last_tick < m.flight.tick => false,
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
            });
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
                choose(&mut best, d, MissileContact::Projectile(body.id));
            }
        }
        if m.bodies[i].is_none() {
            return Err(INVALID);
        }
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
        if p.ended.is_none()
            && (p.propulsion_travel >= MISSILE_RANGE || p.age_ticks >= MISSILE_LIFETIME_TICKS)
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_interception(
        self,
        world: &World,
        lesson: super::super::practice::Practice,
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
        if let Some(b) = self.beams[i] {
            let shot = b.shot;
            let PulseContact::Projectile { id, distance } = shot.contact else {
                return Err(INVALID);
            };
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
                || shot.fitted_item_id != p.origin.owner.owner.checked_add(7).ok_or(INVALID)?
                || id != p.origin.projectile
                || !valid_vector(shot.origin)
                || !valid_vector(shot.direction)
                || (length(shot.direction) - 1.0).abs() > 1e-9
            {
                return Err(INVALID);
            }
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
            for (slot, body) in b.bodies.into_iter().enumerate() {
                let expected = self.projectiles[slot].is_some_and(|other| {
                    other.origin.emission < emission
                        && (slot == i || other.ended.is_none() || other.last_tick > shot.tick)
                });
                if body.is_some() != expected {
                    return Err(INVALID);
                }
                if let Some(body) = body {
                    let other = self.find(body.id).ok_or(INVALID)?;
                    if usize::from(body.id.ordinal) != slot
                        || !valid_vector(body.position)
                        || length(sub(body.position, other.origin.muzzle))
                            > other.propulsion_travel + 1e-7
                    {
                        return Err(INVALID);
                    }
                    if slot == i {
                        if body.position != p.position {
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
                }
            }
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
            if station.is_some_and(|s| s <= d)
                || lesson
                    .retained_missile_contact(ray, d, station)
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

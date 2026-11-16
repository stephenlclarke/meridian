//! Finite owned starter rounds and World-derived original practice ordnance.
use super::{CombatIdentity, FlightMode, PracticeBeaconId, Snapshot, StepError, World};
use crate::docking::Vec3;
use core::fmt;
mod geometry;
mod persistence;
pub use geometry::{MISSILE_FACES, MISSILE_VERTICES};
use geometry::{add, length, mesh_distance, pose, scale, sub, unit};
pub const MISSILE_SPEED: f64 = 792.0;
pub const MISSILE_RANGE: f64 = 6000.0;
pub const MISSILE_LIFETIME_TICKS: u16 = 1440;
pub const MISSILE_COOLDOWN_TICKS: u8 = 60;
pub const MISSILE_MUZZLE_DISTANCE: f64 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissileError {
    StaleCommand,
    Paused,
    NoCommander,
    NotInFlight,
    NoAmmunition,
    NoLiveLock,
    TargetBehind,
    NotArmed,
    CoolingDown,
    InvalidGeometry,
    SequenceExhausted,
    SaveRefused,
}
impl fmt::Display for MissileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::StaleCommand => "Missile command does not match the current tick",
            Self::Paused => "World is paused",
            Self::NoCommander => "An owned commander is required",
            Self::NotInFlight => "Missiles require active flight",
            Self::NoAmmunition => "No owned compatible pylon round remains",
            Self::NoLiveLock => "Lock an actual current practice beacon",
            Self::TargetBehind => "The locked target is behind the craft",
            Self::NotArmed => "Missile is not armed",
            Self::CoolingDown => "Missile launcher is cooling down",
            Self::InvalidGeometry => "Missile physical scene is unavailable",
            Self::SaveRefused => "Missile discharge cannot be retained in a valid save",
            Self::SequenceExhausted => "Missile identity sequence exhausted",
        })
    }
}
impl std::error::Error for MissileError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileId {
    owner: u64,
    visit: u64,
    ordinal: u8,
}
impl ProjectileId {
    pub fn entity(self) -> u64 {
        5 + u64::from(self.ordinal)
    }
    pub fn visit(self) -> u64 {
        self.visit
    }
    pub fn owner(self) -> u64 {
        self.owner
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MissileOrigin {
    pub projectile: ProjectileId,
    pub owner: CombatIdentity,
    pub emission: u64,
    pub item: u64,
    pub launched_tick: u64,
    pub legal_at_launch: u8,
    pub physical_tick: u64,
    pub launch_position: Vec3,
    pub muzzle: Vec3,
    pub orientation: crate::docking::Basis3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissileEnd {
    Impact,
    RangeExpired,
    LifetimeExpired,
    TargetLost,
    Culled,
    Intercepted,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MissileContact {
    Practice {
        id: PracticeBeaconId,
        first_hit: bool,
    },
    Station,
    Asteroid(crate::mining::AsteroidId),
    Player {
        front: bool,
    },
    Projectile(ProjectileId),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectileSnapshot {
    pub origin: MissileOrigin,
    pub target: PracticeBeaconId,
    pub position: Vec3,
    pub direction: Vec3,
    pub propulsion_travel: f64,
    pub age_ticks: u16,
    pub muzzle_cleared: bool,
    pub ended: Option<MissileEnd>,
    pub contact: Option<MissileContact>,
    pub last_tick: u64,
    pub intercepting_emission: Option<u64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissileRound {
    pub item: u64,
    pub pylon: u8,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MissileSnapshot {
    pub additional_rounds: [Option<MissileRound>; 6],
    pub rounds: [Option<MissileRound>; 3],
    pub armed: bool,
    pub lock: Option<PracticeBeaconId>,
    pub cooldown_ticks: u8,
    pub projectiles: [Option<ProjectileSnapshot>; 3],
}
#[derive(Debug, Clone, PartialEq)]
pub(super) struct State {
    armed: bool,
    lock: Option<PracticeBeaconId>,
    cooldown: u8,
    pub(super) projectiles: Vec<Option<ProjectileSnapshot>>,
    motions: Vec<Option<Motion>>,
    impacts: Vec<Option<super::OrdinaryImpact>>,
    beams: Vec<Option<Beam>>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Body {
    id: ProjectileId,
    position: Vec3,
}
#[derive(Debug, Clone, PartialEq)]
struct Motion {
    position: Vec3,
    travel: f64,
    age: u16,
    cleared: bool,
    flight: Snapshot,
    player_alive: bool,
    bodies: Vec<Option<Body>>,
}
#[derive(Debug, Clone, PartialEq)]
struct Beam {
    shot: super::PulseShot,
    emission: u64,
    bodies: Vec<Option<Body>>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            armed: false,
            lock: None,
            cooldown: 0,
            projectiles: vec![None; 3],
            motions: vec![None; 3],
            impacts: vec![None; 3],
            beams: vec![None; 3],
        }
    }
}
impl State {
    fn bodies(&self) -> Vec<Option<Body>> {
        self.projectiles
            .iter()
            .map(|p| {
                p.filter(|p| p.ended.is_none()).map(|p| Body {
                    id: p.origin.projectile,
                    position: p.position,
                })
            })
            .collect()
    }
    fn extend_for_launch(&mut self, index: usize) {
        if index == self.projectiles.len() {
            self.projectiles.push(None);
            self.motions.push(None);
            self.impacts.push(None);
            self.beams.push(None);
        }
    }
}
impl State {
    #[cfg(test)]
    pub(super) fn save_supported(&self) -> bool {
        !self.armed && self.lock.is_none() && self.projectiles.iter().all(Option::is_none)
    }
    pub(super) fn intercept(
        &mut self,
        id: ProjectileId,
        tick: u64,
        emission: u64,
        shot: super::PulseShot,
    ) -> bool {
        let bodies = self.bodies();
        for p in self.projectiles.iter_mut().flatten() {
            if p.origin.projectile == id && p.ended.is_none() {
                p.ended = Some(MissileEnd::Intercepted);
                p.last_tick = tick;
                p.intercepting_emission = Some(emission);
                self.beams[usize::from(id.ordinal)] = Some(Beam {
                    shot,
                    emission,
                    bodies,
                });
                return true;
            }
        }
        false
    }
    pub(super) fn berth(&mut self) {
        self.armed = false;
        self.lock = None;
        self.cooldown = 0;
    }
    pub(super) fn pending(&self) -> u8 {
        self.projectiles
            .iter()
            .flatten()
            .filter(|p| p.ended.is_none())
            .count() as u8
    }
}
pub(super) struct PreparedStep {
    pub(super) missiles: State,
    pub(super) combat: super::combat::State,
    pub(super) pulse: super::weapons::PulseResources,
    pub(super) practice: Option<super::practice::Practice>,
    pub(super) legal: u8,
}
impl World {
    /// Historical T01 sight reuses the exact native missile mesh and retained launch orientation.
    pub(super) fn retained_flight_body_distance(
        &self,
        ordinal: usize,
        position: crate::docking::Vec3,
        ray: crate::docking::RigidPose,
        limit: f64,
    ) -> Option<f64> {
        let projectile = self.missiles.projectiles.get(ordinal).copied().flatten()?;
        mesh_distance(
            crate::docking::RigidPose {
                position,
                orientation: projectile.origin.orientation,
            },
            ray,
            limit,
        )
    }
    pub(super) fn nearest_missile(
        &self,
        ray: crate::docking::RigidPose,
        limit: f64,
    ) -> Option<(ProjectileId, f64)> {
        let owner = self.combat_identity()?;
        let mut best = None;
        for p in self.missiles.projectiles.iter().flatten() {
            if p.ended.is_some() || p.origin.owner.visit != owner.visit {
                continue;
            }
            let body = crate::docking::RigidPose {
                position: p.position,
                orientation: p.origin.orientation,
            };
            if let Some(d) = mesh_distance(body, ray, limit)
                && best.is_none_or(|(_, old)| d < old)
            {
                best = Some((p.origin.projectile, d));
            }
        }
        best
    }
    fn missile_command(&self, tick: u64) -> Result<CombatIdentity, MissileError> {
        if tick != self.state.tick {
            return Err(MissileError::StaleCommand);
        }
        if self.is_paused() {
            return Err(MissileError::Paused);
        }
        let owner = self.combat_identity().ok_or(MissileError::NoCommander)?;
        if self.state.mode != FlightMode::Flight
            || self.combat_snapshot().is_some_and(|s| s.player.destroyed)
        {
            return Err(MissileError::NotInFlight);
        }
        Ok(owner)
    }
    /// Every retained origin, including paid rounds beyond the original three.
    pub fn missile_projectiles(&self) -> impl Iterator<Item = ProjectileSnapshot> + '_ {
        self.missiles.projectiles.iter().flatten().copied()
    }
    pub fn missile_rounds(&self) -> Option<[Option<MissileRound>; 9]> {
        self.ammunition_rounds()
    }
    pub fn missile_snapshot(&self) -> Option<MissileSnapshot> {
        self.combat_identity()?;
        let rounds = self.ammunition_rounds()?;
        Some(MissileSnapshot {
            rounds: core::array::from_fn(|i| rounds[i]),
            armed: self.missiles.armed,
            lock: self.missiles.lock,
            cooldown_ticks: self.missiles.cooldown,
            projectiles: core::array::from_fn(|i| self.missiles.projectiles[i]),
            additional_rounds: core::array::from_fn(|i| rounds[3 + i]),
        })
    }
    pub fn request_missile_lock(
        &mut self,
        target: Option<PracticeBeaconId>,
        tick: u64,
    ) -> Result<(), MissileError> {
        self.missile_command(tick)?;
        if let Some(id) = target
            && self.practice.and_then(|p| p.missile_target(id)).is_none()
        {
            return Err(MissileError::NoLiveLock);
        }
        self.missiles.lock = target;
        Ok(())
    }
    pub fn request_missile_arm(&mut self, armed: bool, tick: u64) -> Result<(), MissileError> {
        self.missile_command(tick)?;
        if armed
            && self
                .ammunition_rounds()
                .is_none_or(|r| r.iter().all(Option::is_none))
        {
            return Err(MissileError::NoAmmunition);
        }
        self.missiles.armed = armed;
        Ok(())
    }
    /// Admission removes the actual highest occupied compatible pylon once.
    /// No target position, launcher pose, velocity, actor or ammo count is accepted.
    pub fn request_fire_missile(&mut self, tick: u64) -> Result<MissileOrigin, MissileError> {
        let owner = self.missile_command(tick)?;
        if !self.missiles.armed {
            return Err(MissileError::NotArmed);
        }
        if self.missiles.cooldown != 0 {
            return Err(MissileError::CoolingDown);
        }
        let target = self.missiles.lock.ok_or(MissileError::NoLiveLock)?;
        let target_pose = self
            .practice
            .and_then(|p| p.missile_target(target))
            .ok_or(MissileError::NoLiveLock)?;
        let player = super::flight_pose(self.state.position, self.state.orientation)
            .ok_or(MissileError::InvalidGeometry)?;
        if sub(target_pose.position, player.position).dot(player.orientation.z) <= 0.0 {
            return Err(MissileError::TargetBehind);
        }
        let round = self
            .ammunition_rounds()
            .and_then(|r| r.into_iter().rev().flatten().next())
            .ok_or(MissileError::NoAmmunition)?;
        let paid_round = self
            .ammunition_snapshot()
            .ok_or(MissileError::NoCommander)?
            .items
            .iter()
            .any(|i| i.item == round.item && i.purchase.is_some());
        let account = self
            .commander_account
            .as_ref()
            .ok_or(MissileError::NoCommander)?
            .prepare_ammunition(0, Some(round.item), paid_round)
            .map_err(round_error)?;
        let index = self.missiles.used();
        let ordinal = u8::try_from(index).map_err(|_| MissileError::SequenceExhausted)?;
        let (combat, emission) = self.combat.reserve_missile_emission()?;
        let origin = MissileOrigin {
            projectile: ProjectileId {
                owner: owner.owner,
                visit: owner.visit,
                ordinal,
            },
            owner,
            emission,
            item: round.item,
            launched_tick: tick,
            legal_at_launch: self.legal_value().ok_or(MissileError::NoCommander)?,
            physical_tick: self.combat.physical_tick(),
            launch_position: player.position,
            orientation: player.orientation,
            muzzle: add(
                player.position,
                scale(player.orientation.z, MISSILE_MUZZLE_DISTANCE),
            ),
        };
        if [origin.muzzle.x, origin.muzzle.y, origin.muzzle.z]
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1e12)
        {
            return Err(MissileError::InvalidGeometry);
        }
        let projectile = ProjectileSnapshot {
            origin,
            target,
            position: origin.muzzle,
            direction: player.orientation.z,
            propulsion_travel: 0.0,
            age_ticks: 0,
            muzzle_cleared: false,
            ended: None,
            contact: None,
            last_tick: tick,
            intercepting_emission: None,
        };
        let old_account = self.commander_account.replace(account);
        let old_combat = self.combat;
        let old_missiles = self.missiles.clone();
        let old_practice = self.practice;
        self.combat = combat;
        self.missiles.extend_for_launch(index);
        self.missiles.projectiles[index] = Some(projectile);
        self.missiles.armed = false;
        self.missiles.cooldown = MISSILE_COOLDOWN_TICKS;
        if let Some(p) = &mut self.practice {
            p.pending_missiles = self.missiles.pending();
            p.observe_discharge(tick);
        }
        if self.save_bytes().is_err() {
            self.commander_account = old_account;
            self.combat = old_combat;
            self.missiles = old_missiles;
            self.practice = old_practice;
            return Err(MissileError::SaveRefused);
        }
        Ok(origin)
    }
    /// All fallible scene/counter work precedes the outer arrival fuel debit.
    pub(super) fn prepare_missile_step(
        &self,
        next: Snapshot,
        navigation: Option<super::NavigationState>,
    ) -> Result<PreparedStep, StepError> {
        let mut prepared = PreparedStep {
            missiles: self.missiles.clone(),
            combat: self.combat,
            pulse: self.pulse,
            practice: self.practice,
            legal: self.legal_value().unwrap_or(0),
        };
        prepared.combat.advance(self.state.mode, next.mode);
        prepared.pulse.advance(self.state.mode, next.mode);
        if self.state.mode != FlightMode::Docked {
            prepared.missiles.cooldown = prepared.missiles.cooldown.saturating_sub(1);
        }
        let visit = navigation.map(|n| n.snapshot(0).visit);
        for index in 0..prepared.missiles.projectiles.len() {
            let Some(mut projectile) = prepared.missiles.projectiles[index] else {
                continue;
            };
            if projectile.ended.is_some() {
                continue;
            }
            if next.mode == FlightMode::Docked || visit != Some(projectile.origin.owner.visit) {
                projectile.ended = Some(MissileEnd::Culled);
                projectile.last_tick = next.tick;
            } else if let Some(target) = prepared
                .practice
                .and_then(|p| p.missile_target(projectile.target))
            {
                prepared.missiles.motions[index] = Some(Motion {
                    position: projectile.position,
                    travel: projectile.propulsion_travel,
                    age: projectile.age_ticks,
                    cleared: projectile.muzzle_cleared,
                    flight: next,
                    player_alive: prepared.pulse.alive(),
                    bodies: prepared.missiles.bodies(),
                });
                advance_projectile(
                    self,
                    &mut prepared,
                    &mut projectile,
                    index,
                    target.position,
                    next,
                )?;
            } else {
                projectile.ended = Some(MissileEnd::TargetLost);
                projectile.last_tick = next.tick;
            }
            prepared.missiles.projectiles[index] = Some(projectile);
            if projectile.ended.is_some()
                && let Some(p) = &mut prepared.practice
            {
                p.observe_discharge(next.tick);
            }
        }
        if next.mode == FlightMode::Docked {
            prepared.missiles.berth();
        } else if navigation.map(|n| n.snapshot(0).visit)
            != self.navigation.map(|n| n.snapshot(0).visit)
        {
            prepared.missiles.armed = false;
            prepared.missiles.lock = None;
        }
        if prepared.missiles.lock.is_some_and(|id| {
            prepared
                .practice
                .and_then(|p| p.missile_target(id))
                .is_none()
        }) {
            prepared.missiles.lock = None;
            prepared.missiles.armed = false;
        }
        if let Some(p) = &mut prepared.practice {
            p.pending_missiles = prepared.missiles.pending();
        }
        Ok(prepared)
    }
}

fn round_error(error: crate::freight::g02::Error) -> MissileError {
    match error {
        crate::freight::g02::Error::MissingEquipment => MissileError::NoAmmunition,
        crate::freight::g02::Error::SequenceExhausted => MissileError::SequenceExhausted,
        _ => MissileError::InvalidGeometry,
    }
}

fn advance_projectile(
    world: &World,
    prepared: &mut PreparedStep,
    p: &mut ProjectileSnapshot,
    index: usize,
    target: Vec3,
    next: Snapshot,
) -> Result<(), StepError> {
    let data = world
        .station_data
        .as_ref()
        .ok_or(StepError::NonFiniteState)?;
    let player =
        super::flight_pose(next.position, next.orientation).ok_or(StepError::NonFiniteState)?;
    if length(sub(p.position, player.position)) > data.player_clearance_radius() + 1e-6 {
        p.muzzle_cleared = true;
    }
    if let Some(direction) = unit(sub(target, p.position)) {
        p.direction = direction;
    }
    let travel = (MISSILE_SPEED / 60.0)
        .min((MISSILE_RANGE - p.propulsion_travel).max(0.0))
        .min(f64::from(MISSILE_LIFETIME_TICKS.saturating_sub(p.age_ticks)) * MISSILE_SPEED / 60.0);
    if travel <= 0.0 {
        p.last_tick = next.tick;
        p.ended = Some(if p.propulsion_travel >= MISSILE_RANGE {
            MissileEnd::RangeExpired
        } else {
            MissileEnd::LifetimeExpired
        });
        return Ok(());
    }
    let ray = pose(p.position, p.direction).ok_or(StepError::NonFiniteState)?;
    let best = query_contact(world, prepared, p, index, ray, travel, next)?;
    let used = best.map_or(travel, |(distance, _)| distance);
    p.position = add(p.position, scale(p.direction, used));
    p.propulsion_travel += used;
    p.age_ticks = p.age_ticks.saturating_add(1);
    p.last_tick = next.tick;
    if let Some((_, mut contact)) = best {
        contact = apply_contact(prepared, p, next, contact)?;
        p.contact = Some(contact);
        p.ended = Some(MissileEnd::Impact);
    } else {
        if length(sub(p.position, player.position)) > data.player_clearance_radius() + 1e-6 {
            p.muzzle_cleared = true;
        }
        if p.propulsion_travel >= MISSILE_RANGE {
            p.ended = Some(MissileEnd::RangeExpired);
        } else if p.age_ticks >= MISSILE_LIFETIME_TICKS {
            p.ended = Some(MissileEnd::LifetimeExpired);
        }
    }
    Ok(())
}
fn query_contact(
    world: &World,
    prepared: &PreparedStep,
    p: &ProjectileSnapshot,
    index: usize,
    ray: crate::docking::RigidPose,
    travel: f64,
    next: Snapshot,
) -> Result<Option<(f64, MissileContact)>, StepError> {
    let data = world
        .station_data
        .as_ref()
        .ok_or(StepError::NonFiniteState)?;
    let player =
        super::flight_pose(next.position, next.orientation).ok_or(StepError::NonFiniteState)?;
    let station = next.station.ok_or(StepError::NonFiniteState)?;
    let station_distance = data
        .beam_block_distance(station, ray.position, ray.orientation.z, travel)
        .map_err(|_| StepError::NonFiniteState)?;
    let mut best = station_distance.map(|d| (d, MissileContact::Station));
    if let Some(contact) = world.asteroid_beam_in_system(p.origin.owner.system, ray, travel) {
        choose(
            &mut best,
            contact.distance,
            MissileContact::Asteroid(contact.asteroid),
        );
    }
    if p.muzzle_cleared
        && prepared.pulse.alive()
        && let Some(d) = data
            .player_beam_distance(player, ray.position, ray.orientation.z, travel)
            .map_err(|_| StepError::NonFiniteState)?
        && best.is_none_or(|(old, _)| d < old)
    {
        let point = add(ray.position, scale(ray.orientation.z, d));
        best = Some((
            d,
            MissileContact::Player {
                front: sub(point, player.position).dot(player.orientation.z) >= 0.0,
            },
        ));
    }
    for other_index in 0..prepared.missiles.projectiles.len() {
        if other_index == index {
            continue;
        }
        if let Some(other) = prepared.missiles.projectiles[other_index]
            && other.ended.is_none()
            && other.origin.owner.visit == p.origin.owner.visit
            && let Some(d) = mesh_distance(
                crate::docking::RigidPose {
                    position: other.position,
                    orientation: other.origin.orientation,
                },
                ray,
                travel,
            )
        {
            choose(
                &mut best,
                d,
                MissileContact::Projectile(other.origin.projectile),
            );
        }
    }
    if let Some((id, d)) = prepared.practice.and_then(|practice| {
        practice.missile_contact(
            ray,
            best.map_or(travel, |(d, _)| d.min(travel)),
            station_distance,
        )
    }) {
        choose(
            &mut best,
            d,
            MissileContact::Practice {
                id,
                first_hit: false,
            },
        );
    }
    Ok(best)
}

fn apply_contact(
    prepared: &mut PreparedStep,
    p: &ProjectileSnapshot,
    next: Snapshot,
    mut contact: MissileContact,
) -> Result<MissileContact, StepError> {
    match contact {
        MissileContact::Practice { id, .. } => {
            let first_hit = prepared
                .practice
                .as_mut()
                .is_some_and(|practice| practice.record_missile_hit(id, p.origin, next));
            contact = MissileContact::Practice { id, first_hit };
        }
        MissileContact::Station | MissileContact::Player { .. } => {
            let target = match contact {
                MissileContact::Station => super::OrdinaryTarget::MainStation,
                MissileContact::Player { front } => super::OrdinaryTarget::Player { front },
                _ => return Err(StepError::NonFiniteState),
            };
            let (hit, penalty) = prepared.combat.record_missile_impact(
                &mut prepared.pulse,
                p.origin,
                next,
                p.position,
                target,
            )?;
            prepared.missiles.impacts[usize::from(p.origin.projectile.ordinal)] = Some(hit);
            prepared.legal = prepared.legal.saturating_add(penalty);
            if let Some(practice) = &mut prepared.practice {
                practice.observe_ordinary_hit(hit);
            }
        }
        // A physical rock stops the missile; it yields no mined material or station legal event.
        MissileContact::Asteroid(_) => {}
        MissileContact::Projectile(id) => {
            for other in prepared.missiles.projectiles.iter_mut().flatten() {
                if other.origin.projectile == id && other.ended.is_none() {
                    other.ended = Some(MissileEnd::Intercepted);
                    other.last_tick = next.tick;
                    other.intercepting_emission = Some(p.origin.emission);
                }
            }
        }
    }
    Ok(contact)
}

fn choose(best: &mut Option<(f64, MissileContact)>, distance: f64, contact: MissileContact) {
    fn order(contact: MissileContact) -> (u8, u64) {
        match contact {
            MissileContact::Station => (0, 0),
            MissileContact::Asteroid(id) => (0, 1 + u64::from(id.ordinal())),
            MissileContact::Practice { id, .. } => (1, u64::from(id.ordinal())),
            MissileContact::Projectile(id) => (2, id.entity()),
            MissileContact::Player { .. } => (3, 0),
        }
    }
    if best.is_none_or(|(old, previous)| {
        distance < old || (distance == old && order(contact) < order(previous))
    }) {
        *best = Some((distance, contact));
    }
}

#[cfg(test)]
pub(super) mod tests;

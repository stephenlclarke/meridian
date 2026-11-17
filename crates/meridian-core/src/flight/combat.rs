//! World-owned ordinary impacts. Presentation values confer no contact authority.
use super::{FireError, FlightMode, PulseContact, PulseShot, Snapshot, World, flight_pose};
use crate::docking::Vec3;
use crate::navigation::SystemId;
use crate::save::{Reader, SaveError, Writer};
pub const SHIELD_CAPACITY: u16 = 30_000;
const SHIELD_RECHARGE: u16 = 3;
const ATTACK_WINDOW: u64 = 600;

/// Descriptive target and weapon classes. These values alone grant no impact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryTarget {
    MainStation,
    Player { front: bool },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryWeapon {
    FrontPulseLaser,
    FrontMiningLaser,
    FrontBeamLaser,
    FrontMilitaryLaser,
    FrontNavalLaser,
    StandardMissile,
}

impl OrdinaryWeapon {
    pub fn label(self) -> &'static str {
        match self {
            Self::FrontPulseLaser => "PULSE",
            Self::FrontMiningLaser => "MINING",
            Self::FrontBeamLaser => "BEAM",
            Self::FrontMilitaryLaser => "MILITARY",
            Self::FrontNavalLaser => "NAVAL",
            Self::StandardMissile => "MISSILE",
        }
    }
    pub fn damage_hundredths(self) -> u16 {
        match self {
            Self::FrontPulseLaser => 1500,
            Self::FrontMiningLaser => 5000,
            Self::FrontBeamLaser => 1500,
            Self::FrontMilitaryLaser => 2300,
            Self::FrontNavalLaser => 6400,
            Self::StandardMissile => 16_000,
        }
    }
    pub(super) fn from_module(kind: super::modular::ModuleKind) -> Option<Self> {
        match kind {
            super::modular::ModuleKind::PulseLaser => Some(Self::FrontPulseLaser),
            super::modular::ModuleKind::MiningLaser => Some(Self::FrontMiningLaser),
            super::modular::ModuleKind::BeamLaser => Some(Self::FrontBeamLaser),
            super::modular::ModuleKind::MilitaryLaser => Some(Self::FrontMilitaryLaser),
            super::modular::ModuleKind::NavalLaser => Some(Self::FrontNavalLaser),
            _ => None,
        }
    }
    pub(crate) fn is_laser(self) -> bool {
        self != Self::StandardMissile
    }
    pub fn cooldown_ticks(self) -> u8 {
        match self {
            Self::FrontBeamLaser | Self::FrontMilitaryLaser | Self::FrontNavalLaser => 6,
            Self::StandardMissile => 0,
            _ => 18,
        }
    }
    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::FrontPulseLaser => 0,
            Self::StandardMissile => 1,
            Self::FrontMiningLaser => 2,
            Self::FrontBeamLaser => 3,
            Self::FrontMilitaryLaser => 4,
            Self::FrontNavalLaser => 5,
        }
    }
    pub(crate) fn from_tag(tag: u8) -> Result<Self, SaveError> {
        match tag {
            0 => Ok(Self::FrontPulseLaser),
            1 => Ok(Self::StandardMissile),
            2 => Ok(Self::FrontMiningLaser),
            3 => Ok(Self::FrontBeamLaser),
            4 => Ok(Self::FrontMilitaryLaser),
            5 => Ok(Self::FrontNavalLaser),
            _ => Err(SaveError::InvalidEncoding),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatIdentity {
    pub owner: u64,
    pub hull: u64,
    pub system: SystemId,
    pub visit: u64,
}
/// Descriptive only. No public operation accepts this as an impact capability.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrdinaryImpact {
    pub attacker: CombatIdentity,
    pub emission: u64,
    pub incident: u64,
    pub item: u64,
    pub weapon: OrdinaryWeapon,
    pub target: OrdinaryTarget,
    pub victim_entity: u64,
    pub tick: u64,
    pub physical_tick: u64,
    pub point: Vec3,
    position: [f32; 3],
    orientation: [f32; 4],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCondition {
    pub front_shield_hundredths: u16,
    pub rear_shield_hundredths: u16,
    pub energy_hundredths: u16,
    pub destroyed: bool,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CombatSnapshot {
    pub player: PlayerCondition,
    pub last_ordinary_impact: Option<OrdinaryImpact>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AttackWindow {
    visit: u64,
    physical_tick: u64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct State {
    pub(super) emissions: u64,
    pub(super) incidents: u64,
    physical_tick: u64,
    front: u16,
    rear: u16,
    windows: [Option<AttackWindow>; 2],
    penalties: u8,
    unbound_player_impact: bool,
    last: Option<OrdinaryImpact>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            emissions: 0,
            incidents: 0,
            physical_tick: 0,
            front: SHIELD_CAPACITY,
            rear: SHIELD_CAPACITY,
            windows: [None; 2],
            penalties: 0,
            unbound_player_impact: false,
            last: None,
        }
    }
}
/// Only the verified current command in this module can create this transaction.
pub(super) struct PreparedPulseImpact {
    opening: State,
    flight: Snapshot,
    identity: CombatIdentity,
    opening_legal: u8,
    item: u64,
    weapon: OrdinaryWeapon,
    opening_pulse: super::weapons::PulseResources,
    opening_damage: super::damage::State,
    damage: super::damage::State,
    pulse: super::weapons::PulseResources,
    epoch: u64,
    next: State,
    legal: u8,
    hit: Option<OrdinaryImpact>,
}
impl State {
    pub(super) fn reserve_mining_emission(self) -> Result<(Self, u64), FireError> {
        let mut next = self;
        next.emissions = next
            .emissions
            .checked_add(1)
            .ok_or(FireError::SequenceExhausted)?;
        Ok((next, next.emissions))
    }
    pub(super) fn shields(self) -> [u16; 2] {
        [self.front, self.rear]
    }
    /// Only sealed World producers call this; it never creates a player/station incident.
    pub(super) fn apply_bound_player_damage(
        &mut self,
        pulse: &mut super::weapons::PulseResources,
        front: bool,
        damage: u16,
    ) {
        let shield = if front {
            &mut self.front
        } else {
            &mut self.rear
        };
        let absorbed = (*shield).min(damage);
        *shield -= absorbed;
        pulse.damage_energy(damage - absorbed);
    }
    pub(super) fn last_impact(self) -> Option<OrdinaryImpact> {
        self.last
    }
    pub(super) fn physical_tick(self) -> u64 {
        self.physical_tick
    }
    pub(super) fn reserve_missile_emission(self) -> Result<(Self, u64), super::MissileError> {
        let mut next = self;
        next.emissions = next
            .emissions
            .checked_add(1)
            .ok_or(super::MissileError::SequenceExhausted)?;
        Ok((next, next.emissions))
    }
    pub(super) fn record_missile_impact(
        &mut self,
        pulse: &mut super::weapons::PulseResources,
        origin: super::MissileOrigin,
        flight: Snapshot,
        point: Vec3,
        target: OrdinaryTarget,
    ) -> Result<(OrdinaryImpact, u8), super::StepError> {
        let incident = self
            .incidents
            .checked_add(1)
            .ok_or(super::StepError::TickOverflow)?;
        let mut penalty = 0;
        if target == OrdinaryTarget::MainStation {
            let index =
                station_index(origin.owner.system).ok_or(super::StepError::NonFiniteState)?;
            if self.windows[index].is_none_or(|last| {
                last.visit != origin.owner.visit
                    || self.physical_tick.saturating_sub(last.physical_tick) > ATTACK_WINDOW
            }) {
                penalty = 64;
                self.penalties = self.penalties.saturating_add(1).min(4);
            }
            self.windows[index] = Some(AttackWindow {
                visit: origin.owner.visit,
                physical_tick: self.physical_tick,
            });
        }
        self.incidents = incident;
        let unsupported_before = self.unbound_player_impact;
        self.resolve_impact(pulse, target, OrdinaryWeapon::StandardMissile);
        // This contact has an actual retained ordnance origin. Keep any older
        // unsupported marker; the missile controller separately refuses v7 saves.
        self.unbound_player_impact = unsupported_before;
        let hit = OrdinaryImpact {
            attacker: origin.owner,
            emission: origin.emission,
            incident,
            item: origin.item,
            weapon: OrdinaryWeapon::StandardMissile,
            target,
            victim_entity: if target == OrdinaryTarget::MainStation {
                1
            } else {
                0
            },
            tick: flight.tick,
            physical_tick: self.physical_tick,
            point,
            position: flight.position,
            orientation: flight.orientation,
        };
        self.last = Some(hit);
        Ok((hit, penalty))
    }
    pub(super) fn advance(&mut self, opening: FlightMode, next: FlightMode) {
        if opening == FlightMode::Docked {
            return;
        }
        self.physical_tick += 1;
        if next == FlightMode::Docked {
            self.front = SHIELD_CAPACITY;
            self.rear = SHIELD_CAPACITY;
        } else {
            self.front = (self.front + SHIELD_RECHARGE).min(SHIELD_CAPACITY);
            self.rear = (self.rear + SHIELD_RECHARGE).min(SHIELD_CAPACITY);
        }
    }
    pub(super) fn can_advance(self, opening: FlightMode) -> bool {
        opening == FlightMode::Docked || self.physical_tick < u64::MAX
    }
    pub(super) fn penalties(self) -> u8 {
        self.penalties
    }
    // A future ordnance producer must first bind its own retained physical contact.
    // Keeping this reducer private prevents the host from submitting raw damage.
    fn resolve_impact(
        &mut self,
        pulse: &mut super::weapons::PulseResources,
        target: OrdinaryTarget,
        weapon: OrdinaryWeapon,
    ) {
        if let OrdinaryTarget::Player { front } = target {
            self.unbound_player_impact = true;
            let damage = weapon.damage_hundredths();
            self.apply_bound_player_damage(pulse, front, damage);
        }
        // The supported original-main actor has no authored nativeStationIdentity:
        // the original station susceptibility guard admits no hull damage.
    }
    pub(super) fn encode(self, w: &mut Writer) {
        self.encode_version(w, 8);
    }
    #[cfg(test)]
    pub(super) fn encode_legacy_version(self, w: &mut Writer, version: u16) {
        self.encode_version(w, version);
    }
    fn encode_version(self, w: &mut Writer, version: u16) {
        w.u64(self.emissions);
        w.u64(self.incidents);
        w.u64(self.physical_tick);
        w.u16(self.front);
        w.u16(self.rear);
        w.u8(self.penalties);
        w.bool(self.unbound_player_impact);
        for window in self.windows {
            w.option(window.as_ref(), |w, x| {
                w.u64(x.visit);
                w.u64(x.physical_tick);
            });
        }
        w.option(self.last.as_ref(), |w, hit| hit.encode_version(w, version));
    }
    pub(super) fn decode(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        Ok(Self {
            emissions: r.u64()?,
            incidents: r.u64()?,
            physical_tick: r.u64()?,
            front: r.u16()?,
            rear: r.u16()?,
            penalties: r.u8()?,
            unbound_player_impact: r.bool()?,
            windows: r.array(|r| {
                r.option(|r| {
                    Ok(AttackWindow {
                        visit: r.u64()?,
                        physical_tick: r.u64()?,
                    })
                })
            })?,
            last: r.option(|r| OrdinaryImpact::decode_version(r, version))?,
        })
    }
    pub(super) fn validate_damage_boundary(self, world: &World) -> Result<(), SaveError> {
        if self.unbound_player_impact
            || self.front > SHIELD_CAPACITY
            || self.rear > SHIELD_CAPACITY
            || ((self.front != SHIELD_CAPACITY || self.rear != SHIELD_CAPACITY)
                && !world.missiles.has_player_hit()
                && world.traffic.player_hits().is_empty())
        {
            return Err(SaveError::InvalidState(
                "unsupported ordnance damage boundary",
            ));
        }
        Ok(())
    }
    pub(super) fn validate(self, world: &World) -> Result<(), SaveError> {
        let invalid = SaveError::InvalidState("ordinary combat identity or resources");
        let identity = world.combat_identity().ok_or(invalid)?;
        self.validate_damage_boundary(world)?;
        let shortest_cooldown = if world.damage.enabled() { 6 } else { 18 };
        let maximum_emissions = if world.state.tick == 0 {
            0
        } else {
            1 + (world.state.tick - 1) / shortest_cooldown
        };
        if self.physical_tick > world.state.tick
            || self.emissions > maximum_emissions.saturating_add(world.missiles.used() as u64)
            || self.incidents > self.emissions
            || self.penalties > 4
            || u64::from(self.penalties) > self.incidents
            || (self.incidents == 0) != self.last.is_none()
            || (self.incidents == 0 && self.windows.iter().any(Option::is_some))
            || (self.incidents != 0 && self.penalties == 0 && !world.missiles.has_player_hit())
        {
            return Err(invalid);
        }
        for window in self.windows.into_iter().flatten() {
            if window.visit < 2
                || window.visit > identity.visit
                || window.physical_tick > self.physical_tick
            {
                return Err(invalid);
            }
        }
        if let Some(hit) = self.last {
            hit.validate(world)?;
            if hit.incident != self.incidents
                || hit.emission > self.emissions
                || hit.physical_tick > self.physical_tick
            {
                return Err(invalid);
            }
            let elapsed_global = world.state.tick - hit.tick;
            let elapsed_physical = self.physical_tick - hit.physical_tick;
            let uninterrupted =
                hit.attacker.visit == identity.visit && world.state.mode == FlightMode::Flight;
            // The physical clock advances only on a subset of global steps.
            // At the receipt's own tick neither clock can have moved past it.
            if elapsed_physical > elapsed_global
                || (elapsed_global == 0 && !uninterrupted)
                || (hit.weapon == OrdinaryWeapon::FrontPulseLaser
                    && !world.damage.enabled()
                    && !world.pulse.fits_ordinary_hit(
                        elapsed_physical,
                        uninterrupted,
                        world.missiles.has_player_hit() || !world.traffic.player_hits().is_empty(),
                    ))
            {
                return Err(invalid);
            }
            if hit.target != OrdinaryTarget::MainStation {
                return Ok(());
            }
            let window =
                self.windows[station_index(hit.attacker.system).ok_or(invalid)?].ok_or(invalid)?;
            // Every refresh is an accepted ordinary incident and replaces last.
            // Its own station window therefore has exactly the same endpoint.
            if window.visit != hit.attacker.visit || window.physical_tick != hit.physical_tick {
                return Err(invalid);
            }
        }
        Ok(())
    }
}
impl World {
    pub(super) fn combat_identity(&self) -> Option<CombatIdentity> {
        let nav = self.navigation?;
        let snapshot = nav.snapshot(self.commander_account.as_ref()?.fuel_tenths());
        Some(CombatIdentity {
            owner: nav.owner().seed(),
            hull: nav.ship().ordinal(),
            system: snapshot.system,
            visit: snapshot.visit,
        })
    }
    pub fn combat_snapshot(&self) -> Option<CombatSnapshot> {
        let laser = self.flight_resources_snapshot()?;
        Some(CombatSnapshot {
            player: PlayerCondition {
                front_shield_hundredths: self.combat.front,
                rear_shield_hundredths: self.combat.rear,
                energy_hundredths: laser.energy_hundredths,
                destroyed: laser.energy_hundredths == 0,
            },
            last_ordinary_impact: self.combat.last,
        })
    }
    pub(super) fn prepare_pulse_impact(
        &self,
        shot: PulseShot,
    ) -> Result<PreparedPulseImpact, FireError> {
        let identity = self.combat_identity().ok_or(FireError::NoCommander)?;
        let fitted = self
            .fitted_front_laser()
            .ok_or(FireError::NoFittedFrontLaser)?;
        let legal = self.legal_value().ok_or(FireError::NoCommander)?;
        let pose = flight_pose(self.state.position, self.state.orientation)
            .ok_or(FireError::InvalidGeometry)?;
        if shot.tick != self.state.tick
            || self.state.mode != FlightMode::Flight
            || self.is_paused()
            || shot.origin != pose.position
            || shot.direction != pose.orientation.z
            || fitted.0 != shot.fitted_item_id
        {
            return Err(FireError::StaleCommand);
        }
        let (_, expected, distance) = self.pulse_scene(pose)?;
        if shot.contact != expected || shot.end != add_scaled(shot.origin, shot.direction, distance)
        {
            return Err(FireError::InvalidGeometry);
        }
        let mut pulse = self.pulse.prepare_laser_discharge(fitted.1)?;
        let mut next = self.combat;
        next.emissions = next
            .emissions
            .checked_add(1)
            .ok_or(FireError::SequenceExhausted)?;
        let mut penalty = 0;
        let mut hit = None;
        if matches!(shot.contact, PulseContact::StationBlocked { .. }) {
            let index = station_index(identity.system).ok_or(FireError::InvalidGeometry)?;
            next.incidents = next
                .incidents
                .checked_add(1)
                .ok_or(FireError::SequenceExhausted)?;
            if next.windows[index].is_none_or(|last| {
                last.visit != identity.visit
                    || next.physical_tick.saturating_sub(last.physical_tick) > ATTACK_WINDOW
            }) {
                penalty = 64;
                next.penalties = next.penalties.saturating_add(1).min(4);
            }
            next.windows[index] = Some(AttackWindow {
                visit: identity.visit,
                physical_tick: next.physical_tick,
            });
            let receipt = OrdinaryImpact {
                attacker: identity,
                emission: next.emissions,
                incident: next.incidents,
                item: shot.fitted_item_id,
                weapon: fitted.1,
                target: OrdinaryTarget::MainStation,
                victim_entity: 1,
                tick: shot.tick,
                physical_tick: next.physical_tick,
                point: shot.end,
                position: self.state.position,
                orientation: self.state.orientation,
            };
            next.resolve_impact(&mut pulse, OrdinaryTarget::MainStation, fitted.1);
            next.last = Some(receipt);
            hit = Some(receipt);
        }
        Ok(PreparedPulseImpact {
            opening: self.combat,
            flight: self.state,
            identity,
            opening_legal: legal,
            item: shot.fitted_item_id,
            weapon: fitted.1,
            opening_pulse: self.pulse,
            opening_damage: self.damage.clone(),
            damage: self.damage.laser(self.combat.physical_tick(), fitted.1)?,
            pulse,
            epoch: self
                .freight_session
                .as_ref()
                .ok_or(FireError::NoCommander)?
                .epoch(),
            next,
            legal: legal.saturating_add(penalty),
            hit,
        })
    }
}
impl PreparedPulseImpact {
    pub(super) fn damage_candidate(&self) -> &super::damage::State {
        &self.damage
    }
    pub(super) fn resource_candidates(
        &self,
        mut practice: Option<super::practice::Practice>,
    ) -> (
        State,
        super::weapons::PulseResources,
        Option<super::practice::Practice>,
    ) {
        if let (Some(p), Some(hit)) = (&mut practice, self.hit) {
            p.observe_ordinary_hit(hit);
        }
        (self.next, self.pulse, practice)
    }
    pub(super) fn emission(&self) -> u64 {
        self.next.emissions
    }
    pub(super) fn commit(self, world: &mut World) -> Result<(), FireError> {
        if world
            .freight_session
            .as_ref()
            .is_none_or(|s| s.epoch() != self.epoch)
            || world.is_paused()
            || world.state != self.flight
            || world.combat != self.opening
            || world.pulse != self.opening_pulse
            || world.damage != self.opening_damage
            || world.combat_identity() != Some(self.identity)
            || world.legal_value() != Some(self.opening_legal)
            || world.fitted_front_laser() != Some((self.item, self.weapon))
        {
            return Err(FireError::StaleCommand);
        }
        let (_, _, practice) = self.resource_candidates(world.practice);
        world
            .check_fire_combat_capacity(
                &world.traffic,
                &self.damage,
                &self.next,
                &self.pulse,
                &world.missiles,
                practice,
            )
            .map_err(|_| FireError::SequenceExhausted)?;
        world
            .commander_account
            .as_mut()
            .ok_or(FireError::NoCommander)?
            .apply_ordinary_legal(self.legal);
        world.combat = self.next;
        world.pulse = self.pulse;
        world.damage = self.damage;
        if let (Some(practice), Some(hit)) = (&mut world.practice, self.hit) {
            practice.observe_ordinary_hit(hit);
        }
        Ok(())
    }
}
impl OrdinaryImpact {
    pub(super) fn matches_flight(self, flight: Snapshot) -> bool {
        self.position == flight.position && self.orientation == flight.orientation
    }
    pub(super) fn encode(self, w: &mut Writer) {
        self.encode_version(w, 8);
    }
    pub(super) fn encode_version(self, w: &mut Writer, version: u16) {
        w.u64(self.victim_entity);
        w.u64(self.attacker.owner);
        w.u64(self.attacker.hull);
        w.u8(self.attacker.system.galaxy);
        w.u16(self.attacker.system.system);
        for n in [
            self.attacker.visit,
            self.emission,
            self.incident,
            self.item,
            self.tick,
            self.physical_tick,
        ] {
            w.u64(n);
        }
        for n in [self.point.x, self.point.y, self.point.z] {
            w.u64(n.to_bits());
        }
        for n in self.position {
            w.f32(n);
        }
        for n in self.orientation {
            w.f32(n);
        }
        if version >= 8 {
            w.u8(self.weapon.tag());
            w.u8(match self.target {
                OrdinaryTarget::MainStation => 0,
                OrdinaryTarget::Player { front: true } => 1,
                OrdinaryTarget::Player { front: false } => 2,
            });
        }
    }
    pub(super) fn decode_version(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        let victim_entity = r.u64()?;
        let mut hit = Self {
            victim_entity,
            weapon: OrdinaryWeapon::FrontPulseLaser,
            target: OrdinaryTarget::MainStation,
            attacker: CombatIdentity {
                owner: r.u64()?,
                hull: r.u64()?,
                system: SystemId {
                    galaxy: r.u8()?,
                    system: r.u16()?,
                },
                visit: r.u64()?,
            },
            emission: r.u64()?,
            incident: r.u64()?,
            item: r.u64()?,
            tick: r.u64()?,
            physical_tick: r.u64()?,
            point: Vec3::new(
                f64::from_bits(r.u64()?),
                f64::from_bits(r.u64()?),
                f64::from_bits(r.u64()?),
            ),
            position: r.array(|r| r.f32())?,
            orientation: r.array(|r| r.f32())?,
        };
        if version >= 8 {
            let tag = r.u8()?;
            if version < 16 && tag > 1 {
                return Err(SaveError::InvalidEncoding);
            }
            hit.weapon = OrdinaryWeapon::from_tag(tag)?;
            hit.target = match r.u8()? {
                0 => OrdinaryTarget::MainStation,
                1 => OrdinaryTarget::Player { front: true },
                2 => OrdinaryTarget::Player { front: false },
                _ => return Err(SaveError::InvalidEncoding),
            };
        }
        Ok(hit)
    }
    pub(super) fn validate(self, world: &World) -> Result<(), SaveError> {
        let invalid = SaveError::InvalidState("ordinary impact receipt");
        let current = world.combat_identity().ok_or(invalid)?;
        if self.weapon == OrdinaryWeapon::StandardMissile {
            return world.missiles.validate_ordinary(self, world);
        }
        let pose = flight_pose(self.position, self.orientation).ok_or(invalid)?;
        let station = world.state.station.ok_or(invalid)?;
        let distance = world
            .station_data
            .as_ref()
            .ok_or(invalid)?
            .beam_block_distance(
                station,
                pose.position,
                pose.orientation.z,
                super::PULSE_RANGE,
            )
            .map_err(|_| invalid)?
            .ok_or(invalid)?;
        if !self.weapon.is_laser() || self.target != OrdinaryTarget::MainStation {
            return Err(SaveError::InvalidState(
                "unsupported missile impact save boundary",
            ));
        }
        if self.victim_entity != 1
            || self.attacker.owner != current.owner
            || self.attacker.hull != current.hull
            || world.owned_front_laser_at(self.item, self.tick, self.attacker) != Some(self.weapon)
            || station_index(self.attacker.system).is_none()
            || self.attacker.visit < 2
            || self.attacker.visit > current.visit
            || (self.attacker.visit == current.visit && self.attacker.system != current.system)
            || self.tick == 0
            || self.tick > world.state.tick
            || self.incident == 0
            || self.emission < self.incident
            || self.physical_tick > self.tick
            || !world.mining_segment_at_step(
                self.attacker.system,
                self.tick,
                pose.position,
                self.point,
                0.0,
            )
            || self.point != add_scaled(pose.position, pose.orientation.z, distance)
            || (self.tick == world.state.tick
                && (self.position != world.state.position
                    || self.orientation != world.state.orientation
                    || self.attacker != current))
        {
            return Err(invalid);
        }
        Ok(())
    }
}
fn station_index(system: SystemId) -> Option<usize> {
    match system {
        SystemId::AVENOR => Some(0),
        SystemId::RENAS => Some(1),
        _ => None,
    }
}
fn add_scaled(a: Vec3, b: Vec3, scale: f64) -> Vec3 {
    Vec3::new(a.x + b.x * scale, a.y + b.y * scale, a.z + b.z * scale)
}

pub(super) fn legal_reachable(legal: u8, possible: u8, penalties: u8) -> bool {
    let mut reachable = [false; 256];
    reachable[0] = true;
    for round in 0..=penalties {
        let before = reachable;
        for (value, present) in before.into_iter().enumerate() {
            if present {
                for mask in 0..=possible {
                    if mask & !possible == 0 {
                        reachable[value | usize::from(mask)] = true;
                    }
                }
            }
        }
        if round < penalties {
            let mut next = [false; 256];
            for (value, present) in reachable.into_iter().enumerate() {
                if present {
                    next[(value + 64).min(255)] = true;
                }
            }
            reachable = next;
        }
    }
    reachable[usize::from(legal)]
}
#[cfg(test)]
mod tests;

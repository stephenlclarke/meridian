//! Fixed-tick front Pulse Laser resources and World-bound discharge.
use super::{FlightMode, World, flight_pose};
use crate::docking::Vec3;
use crate::save::{Reader, SaveError, Writer};
use core::fmt;

pub const PULSE_RANGE: f64 = 6000.0;
pub const PULSE_COOLDOWN_TICKS: u8 = 18;
pub(super) const MEDUSA_ENERGY: u16 = 41_000;
pub const ENERGY_CAPACITY_HUNDREDTHS: u16 = 38_000;
const ENERGY_COST_HUNDREDTHS: u16 = 100;
pub(super) const ENERGY_RECHARGE_HUNDREDTHS: u16 = 7;
const HEAT_LIMIT_FIFTHS: u16 = 1200;
const HEAT_COST_FIFTHS: u16 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireError {
    StaleCommand,
    Paused,
    NoCommander,
    NotInFlight,
    NoFittedFrontPulseLaser,
    NoFittedFrontLaser,
    NoCompatibleMiningTool,
    CoolingDown,
    Overheated,
    InsufficientEnergy,
    InvalidGeometry,
    ProtectedTraffic,
    SequenceExhausted,
}
impl fmt::Display for FireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::StaleCommand => "Fire command does not match the current tick",
            Self::Paused => "World is paused",
            Self::NoCommander => "An owned commander profile is required",
            Self::NotInFlight => "The front Pulse Laser requires active flight",
            Self::NoFittedFrontLaser => "No owned compatible front laser is fitted",
            Self::NoCompatibleMiningTool => "Fit a paid compatible mining laser",
            Self::NoFittedFrontPulseLaser => "No owned front Pulse Laser is fitted",
            Self::CoolingDown => "Pulse Laser is cooling down",
            Self::Overheated => "Pulse Laser is overheated",
            Self::InsufficientEnergy => "Pulse Laser has insufficient energy",
            Self::SequenceExhausted => "Combat identity sequence exhausted",
            Self::ProtectedTraffic => "NPC firing is outside the active lesson admission",
            Self::InvalidGeometry => "Pulse Laser could not query the physical scene",
        })
    }
}
impl std::error::Error for FireError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PulseContact {
    AsteroidBlocked {
        id: crate::mining::AsteroidId,
        distance: f64,
    },
    Npc {
        id: crate::traffic::NpcId,
        distance: f64,
    },
    Projectile {
        id: super::ProjectileId,
        distance: f64,
    },
    Clear,
    StationBlocked {
        distance: f64,
    },
    PracticeBeacon {
        id: super::PracticeBeaconId,
        distance: f64,
        first_hit: bool,
    },
}

/// A committed discharge and its first physical beam endpoint. Presentation only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PulseShot {
    pub tick: u64,
    pub fitted_item_id: u64,
    pub origin: Vec3,
    pub direction: Vec3,
    pub end: Vec3,
    pub contact: PulseContact,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PulseLaserSnapshot {
    pub fitted_item_id: u64,
    pub energy_hundredths: u16,
    pub heat_fifths: u16,
    pub cooldown_ticks: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontLaserSnapshot {
    pub fitted_item_id: u64,
    pub weapon: super::OrdinaryWeapon,
    pub damage_hundredths: u16,
    pub range: u16,
    pub cooldown_ticks_per_shot: u8,
}

/// Shared electrical/weapon condition remains readable when a particular laser is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlightResourcesSnapshot {
    pub energy_capacity_hundredths: u16,
    pub energy_recharge_hundredths_per_tick: u16,
    pub energy_hundredths: u16,
    pub heat_fifths: u16,
    pub cooldown_ticks: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PulseResources {
    energy: u16,
    heat: u16,
    cooldown: u8,
}
impl Default for PulseResources {
    fn default() -> Self {
        Self {
            energy: ENERGY_CAPACITY_HUNDREDTHS,
            heat: 0,
            cooldown: 0,
        }
    }
}
/// Recharge in whole hundredths between two physics ticks. Keep the fractional
/// part across ticks so the Medusa's original 4.7 energy/s is exact over a second.
fn recovered_energy(start: u64, elapsed: u64, peregrine_rate: u16, capacity: u16) -> u64 {
    if capacity != MEDUSA_ENERGY {
        return elapsed.saturating_mul(u64::from(peregrine_rate));
    }
    let per_second = u128::from(peregrine_rate / ENERGY_RECHARGE_HUNDREDTHS) * 470;
    let phase = u128::from(start % 60);
    let amount = ((phase + u128::from(elapsed)) * per_second) / 60 - phase * per_second / 60;
    u64::try_from(amount).unwrap_or(u64::MAX)
}

impl PulseResources {
    #[cfg(test)]
    pub(super) fn replay_advance(&mut self, elapsed: u64) {
        self.replay_advance_with_rate(0, elapsed, 7, ENERGY_CAPACITY_HUNDREDTHS);
    }
    pub(super) fn replay_advance_with_rate(
        &mut self,
        start: u64,
        elapsed: u64,
        recharge: u16,
        capacity: u16,
    ) {
        self.energy = u64::from(self.energy)
            .saturating_add(recovered_energy(start, elapsed, recharge, capacity))
            .min(u64::from(capacity)) as u16;
        self.heat = u64::from(self.heat).saturating_sub(elapsed) as u16;
        self.cooldown = u64::from(self.cooldown).saturating_sub(elapsed) as u8;
    }
    pub(super) fn replay_berth(&mut self, capacity: u16) {
        self.energy = capacity;
        self.heat = 0;
    }
    pub(super) fn advance(
        &mut self,
        opening: FlightMode,
        next: FlightMode,
        recharge: u16,
        capacity: u16,
    ) {
        if opening == FlightMode::Docked {
            return;
        }
        self.cooldown = self.cooldown.saturating_sub(1);
        if next == FlightMode::Docked {
            // Source docking resets energy and heat, retaining laser cooldown.
            self.energy = capacity;
            self.heat = 0;
        } else {
            self.energy = (self.energy + recharge).min(capacity);
            self.heat = self.heat.saturating_sub(1);
        }
    }
    pub(super) fn prepare_emp_discharge(self) -> Result<Self, FireError> {
        if self.cooldown != 0 {
            return Err(FireError::CoolingDown);
        }
        if self
            .heat
            .checked_add(super::emp::EMP_HEAT_FIFTHS)
            .is_none_or(|heat| heat >= HEAT_LIMIT_FIFTHS)
        {
            return Err(FireError::Overheated);
        }
        if !self.alive() {
            return Err(FireError::InsufficientEnergy);
        }
        Ok(Self {
            energy: self.energy,
            heat: self.heat + super::emp::EMP_HEAT_FIFTHS,
            cooldown: super::emp::EMP_COOLDOWN_TICKS,
        })
    }
    pub(super) fn prepare_ecm_discharge(self) -> Result<Self, FireError> {
        if self.energy <= super::ecm::ECM_ENERGY_HUNDREDTHS {
            return Err(FireError::InsufficientEnergy);
        }
        Ok(Self {
            energy: self.energy - super::ecm::ECM_ENERGY_HUNDREDTHS,
            ..self
        })
    }
    pub(super) fn prepare_discharge(self) -> Result<Self, FireError> {
        if self.cooldown != 0 {
            return Err(FireError::CoolingDown);
        }
        if self.heat >= HEAT_LIMIT_FIFTHS {
            return Err(FireError::Overheated);
        }
        if self.energy <= ENERGY_COST_HUNDREDTHS {
            return Err(FireError::InsufficientEnergy);
        }
        Ok(Self {
            energy: self.energy - ENERGY_COST_HUNDREDTHS,
            heat: self.heat + HEAT_COST_FIFTHS,
            cooldown: PULSE_COOLDOWN_TICKS,
        })
    }
    pub(super) fn prepare_mining_discharge(self) -> Result<Self, FireError> {
        // Original selected mining work uses <=230 heat, sharing energy/heat/cooldown with lasers.
        if self.heat > 1150 {
            return Err(FireError::Overheated);
        }
        self.prepare_discharge()
    }
    pub(super) fn alive(self) -> bool {
        self.energy > 0
    }
    pub(super) fn damage_energy(&mut self, damage: u16) {
        self.energy = self.energy.saturating_sub(damage);
    }
    pub(super) fn remaining_cooldown(self) -> u8 {
        self.cooldown
    }

    /// During one uninterrupted flight the weapon clock advances once per
    /// published tick. A quiet anchor upper-bounds the last possible shot.
    pub(super) fn fits_quiet_anchor(self, tick: u64, anchor: u64) -> bool {
        let Some(elapsed) = tick.checked_sub(anchor) else {
            return false;
        };
        u64::from(self.cooldown) <= u64::from(PULSE_COOLDOWN_TICKS).saturating_sub(elapsed)
            && u128::from(self.heat) <= maximum_reachable_heat(tick, elapsed)
    }

    /// An accepted station hit proves an actual Pulse discharge independently
    /// of practice consent. Before its cooldown ends no later Pulse is possible.
    pub(super) fn fits_ordinary_hit(
        self,
        elapsed_physical: u64,
        uninterrupted: bool,
        damaged: bool,
    ) -> bool {
        let remaining = u64::from(PULSE_COOLDOWN_TICKS).saturating_sub(elapsed_physical);
        if u64::from(self.cooldown) < remaining
            || (elapsed_physical < u64::from(PULSE_COOLDOWN_TICKS)
                && u64::from(self.cooldown) != remaining)
        {
            return false;
        }
        if uninterrupted {
            let heat_floor = u64::from(HEAT_COST_FIFTHS).saturating_sub(elapsed_physical);
            if u64::from(self.heat) < heat_floor {
                return false;
            }
            if elapsed_physical < u64::from(PULSE_COOLDOWN_TICKS) {
                // No intervening docking reset or other legal shot can refill
                // or debit this pool during the original shot's cooldown.
                let energy = (u64::from(ENERGY_CAPACITY_HUNDREDTHS - ENERGY_COST_HUNDREDTHS)
                    + elapsed_physical * u64::from(ENERGY_RECHARGE_HUNDREDTHS))
                .min(u64::from(ENERGY_CAPACITY_HUNDREDTHS));
                if (!damaged && u64::from(self.energy) != energy)
                    || (damaged && u64::from(self.energy) > energy)
                {
                    return false;
                }
            }
        }
        true
    }

    pub(super) fn encode_save(self, w: &mut Writer) {
        w.u16(self.energy);
        w.u16(self.heat);
        w.u8(self.cooldown);
    }
    pub(super) fn decode_save(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            energy: r.u16()?,
            heat: r.u16()?,
            cooldown: r.u8()?,
        })
    }
    pub(super) fn validate_damaged_save(
        self,
        tick: u64,
        mode: FlightMode,
        launches: u64,
    ) -> Result<(), SaveError> {
        if mode != FlightMode::Flight && self.energy != ENERGY_CAPACITY_HUNDREDTHS {
            return Err(SaveError::InvalidState("docked damaged player energy"));
        }
        // Damage can only lower energy. Keep the independent heat/cadence checks.
        let mut undamaged = self;
        let elapsed = PULSE_COOLDOWN_TICKS.saturating_sub(self.cooldown);
        undamaged.energy = if self.cooldown == 0 || mode != FlightMode::Flight {
            ENERGY_CAPACITY_HUNDREDTHS
        } else {
            (ENERGY_CAPACITY_HUNDREDTHS - ENERGY_COST_HUNDREDTHS
                + u16::from(elapsed) * ENERGY_RECHARGE_HUNDREDTHS)
                .min(ENERGY_CAPACITY_HUNDREDTHS)
        };
        if self.energy > undamaged.energy {
            return Err(SaveError::InvalidState("damaged player energy"));
        }
        undamaged.validate_save(tick, mode, launches)
    }
    pub(super) fn validate_save(
        self,
        tick: u64,
        mode: FlightMode,
        launches: u64,
    ) -> Result<(), SaveError> {
        let elapsed = PULSE_COOLDOWN_TICKS.saturating_sub(self.cooldown);
        let depleted_energy = (ENERGY_CAPACITY_HUNDREDTHS - ENERGY_COST_HUNDREDTHS
            + u16::from(elapsed) * ENERGY_RECHARGE_HUNDREDTHS)
            .min(ENERGY_CAPACITY_HUNDREDTHS);
        let ready = self.cooldown == 0;
        let quiescent = mode != FlightMode::Flight;
        // Full energy before normal recharge finishes requires a physical docking reset.
        let early_full = !ready
            && depleted_energy < ENERGY_CAPACITY_HUNDREDTHS
            && self.energy == ENERGY_CAPACITY_HUNDREDTHS;
        let can_have_docked_since_shot = match mode {
            FlightMode::Docked => elapsed >= 1 && launches >= 1,
            FlightMode::Launching => elapsed >= 1 && launches >= 2,
            FlightMode::Flight => elapsed >= 2 && launches >= 2,
        };
        if self.cooldown > PULSE_COOLDOWN_TICKS
            || u128::from(self.heat) > maximum_reachable_heat(tick, u64::from(elapsed))
            || (self.energy != ENERGY_CAPACITY_HUNDREDTHS && self.energy != depleted_energy)
            || (early_full && !can_have_docked_since_shot)
            || ((ready || quiescent) && self.energy != ENERGY_CAPACITY_HUNDREDTHS)
            || (quiescent && self.heat != 0)
            || (self.energy < ENERGY_CAPACITY_HUNDREDTHS
                && self.heat < HEAT_COST_FIFTHS.saturating_sub(u16::from(elapsed)))
            || (!ready && tick <= u64::from(elapsed))
            || ((tick == 0 || launches == 0) && self != Self::default())
        {
            return Err(SaveError::InvalidState("front Pulse Laser resources"));
        }
        Ok(())
    }
}

// A shot needs tick 1 or later, and consecutive shots need at least a full
// cooldown of airborne steps. Docked steps enlarge the global tick budget but
// cannot clear cooldown. A docking reset discards all heat before it.
fn maximum_reachable_heat(tick: u64, elapsed_since_shot_at_least: u64) -> u128 {
    let latest_shot = tick.saturating_sub(elapsed_since_shot_at_least);
    if latest_shot == 0 {
        return 0;
    }
    let earlier_shots = (latest_shot - 1) / u64::from(PULSE_COOLDOWN_TICKS);
    // For each earlier shot that still contributes heat, at least one cooldown
    // has cooled it. If any gap clears the heat, discard all preceding shots.
    let net_heat_per_earlier_shot = HEAT_COST_FIFTHS - u16::from(PULSE_COOLDOWN_TICKS);
    let cadence_bound = u128::from(HEAT_COST_FIFTHS)
        + u128::from(earlier_shots) * u128::from(net_heat_per_earlier_shot);
    let threshold_bound = u128::from(HEAT_LIMIT_FIFTHS + HEAT_COST_FIFTHS - 1);
    cadence_bound
        .min(threshold_bound)
        .saturating_sub(u128::from(elapsed_since_shot_at_least))
}

impl World {
    pub(super) fn pulse_scene(
        &self,
        pose: crate::docking::RigidPose,
    ) -> Result<(Option<super::practice::Practice>, PulseContact, f64), FireError> {
        let station = self.state.station.ok_or(FireError::InvalidGeometry)?;
        let distance = self
            .station_data
            .as_ref()
            .ok_or(FireError::InvalidGeometry)?
            .beam_block_distance(station, pose.position, pose.orientation.z, PULSE_RANGE)
            .map_err(|_| FireError::InvalidGeometry)?;
        let npc = self
            .traffic_beam_contact(pose, PULSE_RANGE)
            .filter(|contact| distance.is_none_or(|station| contact.distance < station));
        let first = npc.map(|contact| contact.distance).or(distance);
        let rock = self
            .mining_beam_contact(pose, PULSE_RANGE)
            .filter(|c| first.is_none_or(|d| c.distance < d));
        let solid = rock.map(|c| c.distance).or(first);
        let projectile = self
            .nearest_missile(pose, solid.unwrap_or(PULSE_RANGE))
            .filter(|(_, d)| solid.is_none_or(|wall| *d < wall));
        let cap = projectile.map_or(PULSE_RANGE, |(_, d)| d);
        let mut practice = self.practice;
        let hit = practice
            .as_mut()
            .and_then(|p| p.contact_limited(self.state, pose, cap, solid));
        if let Some((id, distance, first_hit)) = hit {
            return Ok((
                practice,
                PulseContact::PracticeBeacon {
                    id,
                    distance,
                    first_hit,
                },
                distance,
            ));
        }
        if let Some((id, distance)) = projectile {
            return Ok((
                practice,
                PulseContact::Projectile { id, distance },
                distance,
            ));
        }
        if let Some(contact) = rock {
            if !self.traffic_beam_celestial_clear(
                contact.asteroid.system(),
                self.state.tick,
                pose.position,
                contact.point,
            ) {
                return Err(FireError::InvalidGeometry);
            }
            return Ok((
                practice,
                PulseContact::AsteroidBlocked {
                    id: contact.asteroid,
                    distance: contact.distance,
                },
                contact.distance,
            ));
        }
        if let Some(contact) = npc {
            // Refuse before publishing a beam/resource spend through a physical celestial body.
            if !self.traffic_beam_celestial_clear(
                contact.id.system(),
                self.state.tick,
                pose.position,
                contact.point,
            ) {
                return Err(FireError::InvalidGeometry);
            }
            return Ok((
                practice,
                PulseContact::Npc {
                    id: contact.id,
                    distance: contact.distance,
                },
                contact.distance,
            ));
        }
        Ok((
            practice,
            distance.map_or(PulseContact::Clear, |distance| {
                PulseContact::StationBlocked { distance }
            }),
            distance.unwrap_or(PULSE_RANGE),
        ))
    }
    pub(super) fn fitted_front_pulse(&self) -> Option<u64> {
        self.fitted_front_laser()
            .filter(|(_, weapon)| *weapon == super::OrdinaryWeapon::FrontPulseLaser)
            .map(|(item, _)| item)
    }

    pub(super) fn step_energy_recharge(&self, nominal: u16) -> u16 {
        recovered_energy(
            self.combat.physical_tick(),
            1,
            nominal,
            self.energy_capacity(),
        ) as u16
    }
    pub(super) fn energy_capacity(&self) -> u16 {
        if self.xyzzy_medusa_active() {
            MEDUSA_ENERGY
        } else {
            ENERGY_CAPACITY_HUNDREDTHS
        }
    }
    pub fn flight_resources_snapshot(&self) -> Option<FlightResourcesSnapshot> {
        self.commander_account.as_ref()?;
        let nominal = self.energy_recharge_hundredths();
        Some(FlightResourcesSnapshot {
            energy_capacity_hundredths: self.energy_capacity(),
            energy_recharge_hundredths_per_tick: self.step_energy_recharge(nominal),
            energy_hundredths: self.pulse.energy,
            heat_fifths: self.pulse.heat,
            cooldown_ticks: self.pulse.cooldown,
        })
    }
    pub(super) fn fitted_front_laser(&self) -> Option<(u64, super::OrdinaryWeapon)> {
        let item = self.modular_front()?;
        let weapon = super::OrdinaryWeapon::from_module(item.kind)?;
        Some((item.id, weapon))
    }
    pub fn front_laser_snapshot(&self) -> Option<FrontLaserSnapshot> {
        let (fitted_item_id, weapon) = self.fitted_front_laser()?;
        Some(FrontLaserSnapshot {
            fitted_item_id,
            weapon,
            damage_hundredths: weapon.damage_hundredths(),
            range: 6000,
            cooldown_ticks_per_shot: weapon.cooldown_ticks(),
        })
    }

    pub fn pulse_laser_snapshot(&self) -> Option<PulseLaserSnapshot> {
        Some(PulseLaserSnapshot {
            fitted_item_id: self.fitted_front_pulse()?,
            energy_hundredths: self.pulse.energy,
            heat_fifths: self.pulse.heat,
            cooldown_ticks: self.pulse.cooldown,
        })
    }

    /// Attempt one front discharge from the current owned flight pose and fitted item.
    /// The caller supplies no ray, target, mount, damage, or physical step.
    pub fn request_fire_front_pulse(&mut self, expected_tick: u64) -> Result<PulseShot, FireError> {
        self.request_front_discharge(expected_tick, true)
    }
    pub fn request_fire_front_laser(&mut self, expected_tick: u64) -> Result<PulseShot, FireError> {
        self.request_front_discharge(expected_tick, false)
    }
    fn request_front_discharge(
        &mut self,
        expected_tick: u64,
        pulse_only: bool,
    ) -> Result<PulseShot, FireError> {
        if expected_tick != self.state.tick {
            return Err(FireError::StaleCommand);
        }
        if self.is_paused() {
            return Err(FireError::Paused);
        }
        if self.commander_account.is_none() {
            return Err(FireError::NoCommander);
        }
        if self.state.mode != FlightMode::Flight {
            return Err(FireError::NotInFlight);
        }
        if self.emp.input_blocked() {
            return Err(FireError::InsufficientEnergy);
        }
        let (fitted_item_id, weapon) = self.fitted_front_laser().ok_or(if pulse_only {
            FireError::NoFittedFrontPulseLaser
        } else {
            FireError::NoFittedFrontLaser
        })?;
        if pulse_only && weapon != super::OrdinaryWeapon::FrontPulseLaser {
            return Err(FireError::NoFittedFrontPulseLaser);
        }
        self.pulse.prepare_laser_discharge(weapon)?;
        let pose = flight_pose(self.state.position, self.state.orientation)
            .ok_or(FireError::InvalidGeometry)?;
        let origin = pose.position;
        let direction = pose.orientation.z;
        let (mut practice, contact, end_distance) = self.pulse_scene(pose)?;
        let end = Vec3::new(
            origin.x + direction.x * end_distance,
            origin.y + direction.y * end_distance,
            origin.z + direction.z * end_distance,
        );
        let shot = PulseShot {
            tick: self.state.tick,
            fitted_item_id,
            origin,
            direction,
            end,
            contact,
        };
        let impact = self.prepare_pulse_impact(shot)?;
        let traffic = self.prepare_traffic_pulse(shot, impact.emission())?;
        let mut missiles = self.missiles.clone();
        if let PulseContact::Projectile { id, .. } = contact
            && !missiles.intercept(
                id,
                self.state.tick,
                impact.emission(),
                shot,
                self.traffic.missile_scene(
                    self.combat_identity().ok_or(FireError::NoCommander)?.system,
                    self.state.tick,
                    crate::traffic::SceneStage::Command(impact.emission() - 1),
                ),
            )
        {
            return Err(FireError::InvalidGeometry);
        }
        let damage = impact.damage_candidate();
        let (combat, pulse, impact_practice) = impact.resource_candidates(self.practice);
        self.clear_missing_missile_lock(&mut missiles, &traffic, practice, pulse.alive());
        if let (Some(candidate), Some(current)) = (&mut practice, impact_practice) {
            candidate.ordinary_hit = current.ordinary_hit;
            candidate.reception = current.reception;
            candidate.pending_missiles = missiles.pending_for_lesson(*candidate);
        }
        if let Some(practice) = practice.as_mut() {
            practice.observe_discharge(self.state.tick);
        }
        self.check_fire_combat_capacity(&traffic, damage, &combat, &pulse, &missiles, practice)
            .map_err(|_| FireError::SequenceExhausted)?;
        impact.commit(self)?;
        self.traffic = traffic;
        self.missiles = missiles;
        self.practice = practice;

        Ok(shot)
    }
}

#[cfg(test)]
mod tests;

impl PulseResources {
    pub(super) fn prepare_laser_discharge(
        self,
        weapon: super::OrdinaryWeapon,
    ) -> Result<Self, FireError> {
        if !weapon.is_laser() {
            return Err(FireError::NoFittedFrontLaser);
        }
        if weapon.cooldown_ticks() == PULSE_COOLDOWN_TICKS {
            return self.prepare_discharge();
        }
        if self.cooldown != 0 {
            return Err(FireError::CoolingDown);
        }
        if self.heat >= HEAT_LIMIT_FIFTHS {
            return Err(FireError::Overheated);
        }
        if self.energy <= ENERGY_COST_HUNDREDTHS {
            return Err(FireError::InsufficientEnergy);
        }
        Ok(Self {
            energy: self.energy - ENERGY_COST_HUNDREDTHS,
            heat: self.heat + HEAT_COST_FIFTHS,
            cooldown: weapon.cooldown_ticks(),
        })
    }
}

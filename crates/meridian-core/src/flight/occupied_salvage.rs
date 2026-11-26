//! Explicit, fictional Meridian contact with a disabled occupied game actor. It is not
//! abandonment, consent, a beam discharge, crew transfer, or material acquisition.
use super::{CombatIdentity, FlightMode, World};
use crate::{
    docking::{RigidPose, Vec3},
    navigation::SystemId,
    save::{Reader, SaveError, Writer},
    traffic::{MissileScene, NpcId, NpcPhase, Population, SceneStage},
};
use std::sync::Arc;

const INVALID: SaveError = SaveError::InvalidEncoding;
const MAX_CONTACTS: usize = 32;
// New Meridian game rule, not an original equipment performance measurement.
const CONTACT_INTERVAL: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OccupiedCutError {
    FlightRequired,
    MissingFit,
    SelectionChanged,
    NotDisabledOccupied,
    UnsafeHold,
    ContactUnavailable,
    StaleReview,
    Cooldown,
    Capacity,
    Save(SaveError),
}

impl core::fmt::Display for OccupiedCutError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::FlightRequired => "Occupied salvage requires active local flight",
            Self::MissingFit => "Fit Salvage Module, Cutter and Fuel Scoops",
            Self::SelectionChanged => "Occupied salvage target changed",
            Self::NotDisabledOccupied => "Target is not a proved disabled occupied ship",
            Self::UnsafeHold => "Establish the safe salvage hold first",
            Self::ContactUnavailable => "Selected ship surface contact is unavailable",
            Self::StaleReview => "Occupied salvage review expired; request again",
            Self::Cooldown => "Cutter contact interval has not elapsed",
            Self::Capacity => "Occupied contact history is full",
            Self::Save(error) => return write!(f, "Occupied contact cannot be saved: {error}"),
        })
    }
}
impl std::error::Error for OccupiedCutError {}

/// A fresh warning is not permission from the ship or its crew. Only a separate explicit
/// confirmation can prepare a contact. Dropping or cancelling it changes no World state.
#[derive(Debug)]
pub struct OccupiedCutReview {
    owner: CombatIdentity,
    target: NpcId,
    item: u64,
    tick: u64,
    epoch: u64,
    scope: Arc<()>,
    unlawful: bool,
}
impl OccupiedCutReview {
    pub fn target(&self) -> NpcId {
        self.target
    }
    pub fn is_unlawful(&self) -> bool {
        self.unlawful
    }
    pub fn warning(&self) -> &'static str {
        if self.unlawful {
            "OCCUPIED / CLAIMED: deliberate contact is an unlawful attack. No verified material."
        } else {
            "OCCUPIED / CLAIMED: deliberate contact is attributed to you. No verified material."
        }
    }
    pub fn cancel(self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OccupiedCutContact {
    pub target: NpcId,
    pub cutter_item: u64,
    pub world_tick: u64,
    pub physical_tick: u64,
    pub unlawful: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Receipt {
    owner: CombatIdentity,
    contact: OccupiedCutContact,
    position: [f32; 3],
    orientation: [f32; 4],
    speed: f32,
    target_speed: f64,
    before_motion: MissileScene,
    scene: MissileScene,
}

#[derive(Debug, Clone, Default)]
pub(super) struct State {
    contacts: Vec<Receipt>,
    // Only an accepted local step can furnish this transient opening. It is
    // retained in the save solely when the pilot confirms a contact.
    pub(super) before_motion: Option<MissileScene>,
}
impl State {
    #[cfg(test)]
    pub(super) fn alter_first_target_speed_for_test(&mut self, speed: f64, radius: f64) {
        let receipt = &mut self.contacts[0];
        // Keep the other validation envelope satisfied; the mutation must
        // fail precisely because it disagrees with measured motion.
        let Some(pose) = super::flight_pose(receipt.position, receipt.orientation) else {
            panic!("the recorded cutter contact requires a valid ship pose");
        };
        let Some(body) = receipt.scene.target(receipt.contact.target) else {
            panic!("the recorded encounter must contain the selected ship");
        };
        assert!(working_hold(pose, receipt.speed, body.pose, speed, radius));
        assert_ne!(receipt.target_speed, speed);
        receipt.target_speed = speed;
    }

    pub(super) fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }
    pub(super) fn crimes(&self) -> u16 {
        self.contacts.iter().filter(|r| r.contact.unlawful).count() as u16
    }
    pub(super) fn crime_in(&self, system: SystemId) -> bool {
        self.contacts
            .iter()
            .any(|r| r.contact.unlawful && r.owner.system == system)
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.u8(self.contacts.len() as u8);
        for r in &self.contacts {
            w.u64(r.owner.owner);
            w.u64(r.owner.hull);
            w.u8(r.owner.system.galaxy);
            w.u16(r.owner.system.system);
            w.u64(r.owner.visit);
            r.contact.target.encode(w);
            w.u64(r.contact.cutter_item);
            w.u64(r.contact.world_tick);
            w.u64(r.contact.physical_tick);
            w.bool(r.contact.unlawful);
            for v in r.position {
                w.f32(v);
            }
            for v in r.orientation {
                w.f32(v);
            }
            w.f32(r.speed);
            w.u64(r.target_speed.to_bits());
            r.before_motion.encode(w);
            r.scene.encode(w);
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let n = usize::from(r.u8()?);
        if n == 0 || n > MAX_CONTACTS {
            return Err(INVALID);
        }
        let mut contacts = Vec::with_capacity(n);
        for _ in 0..n {
            let owner = CombatIdentity {
                owner: r.u64()?,
                hull: r.u64()?,
                system: SystemId {
                    galaxy: r.u8()?,
                    system: r.u16()?,
                },
                visit: r.u64()?,
            };
            let contact = OccupiedCutContact {
                target: NpcId::decode(r)?,
                cutter_item: r.u64()?,
                world_tick: r.u64()?,
                physical_tick: r.u64()?,
                unlawful: r.bool()?,
            };
            let position = [r.f32()?, r.f32()?, r.f32()?];
            let orientation = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
            let speed = r.f32()?;
            let target_speed = f64::from_bits(r.u64()?);
            contacts.push(Receipt {
                owner,
                contact,
                position,
                orientation,
                speed,
                target_speed,
                before_motion: MissileScene::decode(r)?,
                scene: MissileScene::decode(r)?,
            });
        }
        Ok(Self {
            contacts,
            before_motion: None,
        })
    }

    pub(super) fn validate(
        &self,
        world: &World,
        modular: &super::modular::State,
    ) -> Result<(), SaveError> {
        if self.is_empty() {
            return Ok(());
        }
        let owner = world.combat_identity().ok_or(INVALID)?;
        let end = world.combat.physical_tick();
        let data = world.station_data.as_ref().ok_or(INVALID)?;
        let mut previous: Option<&Receipt> = None;
        for receipt in &self.contacts {
            let contact = receipt.contact;
            let tick = contact.world_tick;
            let physical = contact.physical_tick;
            let visit = receipt.owner.visit;
            if receipt.owner.owner != owner.owner
                || receipt.owner.hull != owner.hull
                || contact.target.profile() != owner.owner
                || contact.target.system() != receipt.owner.system
                || visit < 2
                || visit > owner.visit
                || tick > world.state.tick
                || physical > end
                || physical > tick
                || tick - physical > world.state.tick.checked_sub(end).ok_or(INVALID)?
                || world.jump_journal.system_at_visit(visit, tick)? != receipt.owner.system
                || modular
                    .salvage_cut_fit_at(owner.owner, tick, visit, contact.target)
                    .is_none_or(|item| item.id != contact.cutter_item)
                || previous.is_some_and(|p| {
                    p.owner.visit > visit
                        || p.contact.world_tick > tick
                        || physical.saturating_sub(p.contact.physical_tick) < CONTACT_INTERVAL
                })
                || receipt.scene.system != receipt.owner.system
                || receipt.scene.world_tick != tick
                || !matches!(receipt.scene.stage, SceneStage::Command(n) if n <= world.combat.emissions)
                || !receipt.speed.is_finite()
                || receipt.speed < 0.0
                || !receipt.target_speed.is_finite()
                || !(0.0..=34.3).contains(&receipt.target_speed)
            {
                return Err(INVALID);
            }
            // The same accepted visit/physical departure and arrival boundary as other
            // commander-owned local receipts. A station or later visit cannot mint contact.
            let (jumps, arrival) = world.jump_journal.visit_origin(visit);
            let launch = visit
                .checked_sub(1)
                .and_then(|v| v.checked_sub(jumps))
                .ok_or(INVALID)?;
            let berth = world.damage.berth_clock(launch).ok_or(INVALID)?;
            if arrival.map_or(physical <= berth, |arrived| physical < arrived)
                || world
                    .damage
                    .berth_clock(launch.checked_add(1).ok_or(INVALID)?)
                    .is_some_and(|next| physical >= next)
            {
                return Err(INVALID);
            }
            world
                .traffic
                .validate_missile_scene(&receipt.scene, world.state.tick, data)?;
            world
                .traffic
                .validate_missile_scene(&receipt.before_motion, world.state.tick, data)?;
            if !motion_matches(
                contact.target,
                &receipt.before_motion,
                &receipt.scene,
                receipt.target_speed,
            ) || world
                .traffic
                .occupied_disabled_at(contact.target, &receipt.before_motion)
                .is_err()
            {
                return Err(INVALID);
            }
            let population = world
                .traffic
                .occupied_disabled_at(contact.target, &receipt.scene)?;
            if !matches!(population, Population::Trader | Population::Pirate)
                || contact.unlawful != (population == Population::Trader)
            {
                return Err(INVALID);
            }
            let pose = super::flight_pose(receipt.position, receipt.orientation).ok_or(INVALID)?;
            let body = receipt.scene.target(contact.target).ok_or(INVALID)?;
            if tick == world.state.tick
                && world
                    .traffic_snapshot()
                    .and_then(|snapshot| {
                        snapshot
                            .actors
                            .into_iter()
                            .find(|actor| actor.id == contact.target)
                    })
                    .is_none_or(|actor| actor.speed_units != receipt.target_speed)
            {
                return Err(INVALID);
            }
            if !working_hold(
                pose,
                receipt.speed,
                body.pose,
                receipt.target_speed,
                data.player_clearance_radius(),
            ) {
                return Err(INVALID);
            }
            let range = distance(pose.position, body.pose.position);
            let hit = receipt
                .scene
                .contact(data, pose.position, pose.orientation.z, range)
                .ok_or(INVALID)?;
            if hit.id != contact.target
                || !world.traffic_beam_celestial_clear(
                    receipt.owner.system,
                    tick,
                    pose.position,
                    hit.point,
                )
                || !world.mining_segment_at_step(
                    receipt.owner.system,
                    tick,
                    pose.position,
                    hit.point,
                    0.0,
                )
                || data
                    .beam_block_distance(
                        data.station_pose(),
                        pose.position,
                        pose.orientation.z,
                        hit.distance,
                    )
                    .map_err(|_| INVALID)?
                    .is_some()
                || (tick == world.state.tick
                    && (receipt.position != world.state.position
                        || receipt.orientation != world.state.orientation
                        || receipt.speed != world.state.speed))
            {
                return Err(INVALID);
            }
            previous = Some(receipt);
        }
        Ok(())
    }
}

fn distance(a: Vec3, b: Vec3) -> f64 {
    let d = Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z);
    d.dot(d).sqrt()
}

// Both endpoints come from the real game traffic step, not from a speed
// field supplied by the caller. A disabled actor either coasts on its fixed
// orientation or remains at the opening position when motion is refused.
fn motion_matches(target: NpcId, before: &MissileScene, after: &MissileScene, speed: f64) -> bool {
    if before.system != after.system
        || before.world_tick.checked_add(1) != Some(after.world_tick)
        || before.local_tick.checked_add(1) != Some(after.local_tick)
        || !matches!((before.stage, after.stage), (SceneStage::Command(a), SceneStage::Command(b)) if a <= b)
        || !(0.0..=34.3).contains(&speed)
    {
        return false;
    }
    let (Some(a), Some(b)) = (before.target(target), after.target(target)) else {
        return false;
    };
    let travel = speed * 18.0 / 60.0;
    let expected = Vec3::new(
        a.pose.position.x + b.pose.orientation.z.x * travel,
        a.pose.position.y + b.pose.orientation.z.y * travel,
        a.pose.position.z + b.pose.orientation.z.z * travel,
    );
    distance(a.pose.orientation.z, b.pose.orientation.z) <= 1e-12
        && distance(expected, b.pose.position) <= 1e-8
}

fn working_hold(
    player: RigidPose,
    speed: f32,
    target: RigidPose,
    target_speed: f64,
    radius: f64,
) -> bool {
    let range = distance(player.position, target.position);
    let velocity = |pose: RigidPose, units: f64| {
        let v = units * f64::from(super::DISTANCE_PER_SPEED_UNIT);
        Vec3::new(
            pose.orientation.z.x * v,
            pose.orientation.z.y * v,
            pose.orientation.z.z * v,
        )
    };
    (range - (radius * 2.0 + 2.0)).abs() <= 0.5
        && distance(
            velocity(player, f64::from(speed)),
            velocity(target, target_speed),
        ) <= 1.0
}

impl World {
    pub(super) fn occupied_motion_opening(&self) -> Option<MissileScene> {
        self.selected_disabled_salvage_target()?;
        let owner = self.combat_identity()?;
        Some(self.traffic.missile_scene(
            owner.system,
            self.state.tick,
            SceneStage::Command(self.combat.emissions),
        ))
    }

    pub fn occupied_cut_contacts(&self) -> impl Iterator<Item = OccupiedCutContact> + '_ {
        self.occupied_salvage.contacts.iter().map(|r| r.contact)
    }

    fn occupied_cut_opening(
        &self,
        target: NpcId,
        tick: u64,
    ) -> Result<OccupiedCutReview, OccupiedCutError> {
        if tick != self.state.tick {
            return Err(OccupiedCutError::StaleReview);
        }
        if self.is_paused()
            || self.state.mode != FlightMode::Flight
            || !self.pulse.alive()
            || self.emp.electrical_disabled()
            || self
                .snapshot()
                .navigation
                .is_some_and(|n| n.travel.is_some())
        {
            return Err(OccupiedCutError::FlightRequired);
        }
        let owner = self
            .combat_identity()
            .ok_or(OccupiedCutError::FlightRequired)?;
        if self.selected_disabled_salvage_target() != Some(target) {
            return Err(OccupiedCutError::SelectionChanged);
        }
        let actor = self
            .traffic_snapshot()
            .and_then(|s| s.actors.into_iter().find(|a| a.id == target))
            .ok_or(OccupiedCutError::NotDisabledOccupied)?;
        if target.profile() != owner.owner
            || target.system() != owner.system
            || !matches!(actor.population, Population::Trader | Population::Pirate)
            || matches!(actor.phase, NpcPhase::Destroyed | NpcPhase::Complete)
            || !actor.electrical_disabled
            || actor.emp_crew_aboard != Some(1)
        {
            return Err(OccupiedCutError::NotDisabledOccupied);
        }
        if !self.work_targeting_fit().salvage || self.fitted_fuel_scoops().is_none() {
            return Err(OccupiedCutError::MissingFit);
        }
        if self
            .modular
            .salvage_exchange_source()
            .is_some_and(|id| id != target)
        {
            return Err(OccupiedCutError::MissingFit);
        }
        let item = self
            .fitted_salvage_cutter()
            .ok_or(OccupiedCutError::MissingFit)?
            .id;
        self.occupied_cut_scene(target, tick, owner, actor.speed_units)?;
        Ok(OccupiedCutReview {
            owner,
            target,
            item,
            tick,
            epoch: self
                .freight_session
                .as_ref()
                .ok_or(OccupiedCutError::FlightRequired)?
                .epoch(),
            scope: self.mining_scope.clone(),
            unlawful: actor.population == Population::Trader,
        })
    }

    pub fn review_occupied_cut(
        &self,
        target: NpcId,
        expected_tick: u64,
    ) -> Result<OccupiedCutReview, OccupiedCutError> {
        self.occupied_cut_opening(target, expected_tick)
    }

    pub fn prepare_occupied_cut(
        &mut self,
        review: OccupiedCutReview,
    ) -> Result<PreparedOccupiedCut<'_>, OccupiedCutError> {
        let current = self.occupied_cut_opening(review.target, review.tick)?;
        if current.owner != review.owner
            || current.item != review.item
            || current.epoch != review.epoch
            || current.unlawful != review.unlawful
            || !Arc::ptr_eq(&current.scope, &review.scope)
        {
            return Err(OccupiedCutError::StaleReview);
        }
        let target_speed = self
            .traffic_snapshot()
            .and_then(|s| s.actors.into_iter().find(|a| a.id == review.target))
            .ok_or(OccupiedCutError::NotDisabledOccupied)?
            .speed_units;
        let (scene, before_motion) =
            self.occupied_cut_scene(review.target, review.tick, review.owner, target_speed)?;
        let physical = self.combat.physical_tick();
        let contact = OccupiedCutContact {
            target: review.target,
            cutter_item: review.item,
            world_tick: review.tick,
            physical_tick: physical,
            unlawful: review.unlawful,
        };
        let mut candidate = self.occupied_salvage.clone();
        candidate.contacts.push(Receipt {
            owner: review.owner,
            contact,
            position: self.state.position,
            orientation: self.state.orientation,
            speed: self.state.speed,
            target_speed,
            before_motion,
            scene,
        });
        let original = std::mem::replace(&mut self.occupied_salvage, candidate);
        let result = self.save_bytes();
        let candidate = std::mem::replace(&mut self.occupied_salvage, original);
        let bytes = result.map_err(OccupiedCutError::Save)?;
        Ok(PreparedOccupiedCut {
            world: self,
            candidate,
            contact,
            bytes,
        })
    }

    fn occupied_cut_scene(
        &self,
        target: NpcId,
        tick: u64,
        owner: CombatIdentity,
        target_speed: f64,
    ) -> Result<(MissileScene, MissileScene), OccupiedCutError> {
        let physical = self.combat.physical_tick();
        let old = &self.occupied_salvage;
        if old.contacts.len() >= MAX_CONTACTS {
            return Err(OccupiedCutError::Capacity);
        }
        if old
            .contacts
            .last()
            .is_some_and(|r| physical.saturating_sub(r.contact.physical_tick) < CONTACT_INTERVAL)
        {
            return Err(OccupiedCutError::Cooldown);
        }
        let data = self
            .station_data
            .as_ref()
            .ok_or(OccupiedCutError::ContactUnavailable)?;
        let pose = super::flight_pose(self.state.position, self.state.orientation)
            .ok_or(OccupiedCutError::ContactUnavailable)?;
        let scene = self.traffic.missile_scene(
            owner.system,
            tick,
            SceneStage::Command(self.combat.emissions),
        );
        let body = scene
            .target(target)
            .ok_or(OccupiedCutError::ContactUnavailable)?;
        let before = self
            .occupied_salvage
            .before_motion
            .as_ref()
            .filter(|before| motion_matches(target, before, &scene, target_speed))
            .ok_or(OccupiedCutError::ContactUnavailable)?;
        self.traffic
            .occupied_disabled_at(target, before)
            .map_err(|_| OccupiedCutError::ContactUnavailable)?;
        if !working_hold(
            pose,
            self.state.speed,
            body.pose,
            target_speed,
            data.player_clearance_radius(),
        ) {
            return Err(OccupiedCutError::UnsafeHold);
        }
        // Share the existing world geometry query, but never publish its laser or damage result.
        let (_, first, hit_distance) = self
            .pulse_scene(pose)
            .map_err(|_| OccupiedCutError::ContactUnavailable)?;
        if !matches!(first, crate::PulseContact::Npc { id, .. } if id == target)
            || hit_distance > distance(pose.position, body.pose.position)
        {
            return Err(OccupiedCutError::ContactUnavailable);
        }
        Ok((scene, before.clone()))
    }
}

pub struct PreparedOccupiedCut<'a> {
    world: &'a mut World,
    candidate: State,
    contact: OccupiedCutContact,
    bytes: Vec<u8>,
}
impl PreparedOccupiedCut<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn publish_with<E>(
        self,
        writer: impl FnOnce(&[u8]) -> Result<(), E>,
    ) -> Result<OccupiedCutContact, E> {
        writer(&self.bytes)?;
        Ok(self.commit())
    }
    pub fn commit(self) -> OccupiedCutContact {
        self.world.occupied_salvage = self.candidate;
        self.contact
    }
}

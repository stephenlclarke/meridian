//! Original T02 uncrewed practice provider for the current Avenor sortie.
use super::{FlightMode, Snapshot, World, flight_pose};
use crate::docking::{RigidPose, Vec3};
use crate::navigation::{DockContext, NavigationState, SystemId};
use crate::station::OriginalStationData;

mod geometry;
pub(super) mod lifecycle;
pub use lifecycle::{
    PRACTICE_MAX_OFFERS, PracticeAttemptId, PracticeAttemptSnapshot, PracticeClosureReview,
    PracticeLedgerSnapshot, PracticeOutcome,
};
mod persistence;
pub(super) mod reception;
pub use reception::{
    PRACTICE_QUIET_TICKS, PracticeReceptionReview, PracticeReceptionSnapshot, PracticeTrace,
};

pub const PRACTICE_HALF_SIDE: f64 = 3.0;
const OFFSETS: [Vec3; 3] = [
    Vec3::new(-120.0, 0.0, 1500.0),
    Vec3::new(0.0, 0.0, 1800.0),
    Vec3::new(120.0, 0.0, 2100.0),
];
const VOLUME_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 1800.0);
const VOLUME_RADIUS: f64 = 650.0;
const FLASH_TICKS: u64 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeReview {
    dock: DockContext,
    tick: u64,
    epoch: u64,
    fingerprint: u64,
    attempt: PracticeAttemptId,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeError {
    StaleReview,
    Paused,
    NoCommander,
    AvenorBerthRequired,
    AlreadyConsented,
    ReceptionNotReady,
    AlreadyReceived,
    NoActiveAttempt,
    OfferCapacity,
    MissionCapacity,
    SharedCapacity,
    Save(crate::save::SaveError),
}
impl core::fmt::Display for PracticeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::StaleReview => "Practice review is no longer current",
            Self::Paused => "Resume before confirming practice",
            Self::NoCommander => "An owned commander profile is required",
            Self::AvenorBerthRequired => "Practice consent requires the actual Avenor main berth",
            Self::AlreadyConsented => "This commander already has a practice attempt",
            Self::ReceptionNotReady => "Reception requires three distinct hits, three quiet flight seconds and a subsequent Avenor entry",
            Self::AlreadyReceived => "This attempt already received verified practice",
            Self::NoActiveAttempt => "There is no open practice attempt to close",
            Self::OfferCapacity => "The retained Avenor practice offer capacity is full",
            Self::MissionCapacity => "The native mission record capacity is full",
            Self::SharedCapacity => "Three Original contracts or services are already active",
            Self::Save(error) => return write!(f, "Practice save preparation failed: {error}"),
        })
    }
}
impl std::error::Error for PracticeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeBeaconId {
    owner: u64,
    visit: u64,
    ordinal: u8,
}
impl PracticeBeaconId {
    pub(super) fn encode(self, w: &mut crate::save::Writer) {
        w.u64(self.owner);
        w.u64(self.visit);
        w.u8(self.ordinal);
    }
    pub(super) fn decode(r: &mut crate::save::Reader<'_>) -> Result<Self, crate::save::SaveError> {
        Ok(Self {
            owner: r.u64()?,
            visit: r.u64()?,
            ordinal: r.u8()?,
        })
    }
    pub fn ordinal(self) -> u8 {
        self.ordinal
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeStatus {
    Unaccepted,
    Consented,
    Active,
    Retired,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PracticeBeaconSnapshot {
    pub id: PracticeBeaconId,
    pub pose: RigidPose,
    pub hit: bool,
    pub flashing: bool,
    pub missile_hit: Option<super::MissileOrigin>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PracticeSnapshot {
    pub status: PracticeStatus,
    pub beacons: Option<[PracticeBeaconSnapshot; 3]>,
    pub first_hits: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RecordedPose {
    position: [f32; 3],
    orientation: [f32; 4],
}
impl RecordedPose {
    fn from_snapshot(snapshot: Snapshot) -> Self {
        Self {
            position: snapshot.position,
            orientation: snapshot.orientation,
        }
    }
    fn pose(self) -> Option<RigidPose> {
        flight_pose(self.position, self.orientation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FirstHit {
    tick: u64,
    firing_pose: RecordedPose,
    missile: Option<super::MissileOrigin>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Range {
    opening: RecordedPose,
    opened_tick: u64,
    hits: [Option<FirstHit>; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Retirement {
    tick: u64,
    visit: u64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Practice {
    attempt: u8,
    settlement: Option<PracticeOutcome>,
    cancel: Option<lifecycle::Closure>,
    consent: DockContext,
    confirmed_tick: u64,
    range: Option<Range>,
    retired: Option<Retirement>,
    pub(super) reception: reception::Reception,
    pub(super) ordinary_hit: Option<super::OrdinaryImpact>,
    pub(super) pending_missiles: u8,
}
impl Practice {
    pub(super) fn consent_visit(self) -> u64 {
        self.consent.visit()
    }
    pub(super) fn confirmation_tick(self) -> u64 {
        self.confirmed_tick
    }
    pub(super) fn observe_ordinary_hit(&mut self, hit: super::OrdinaryImpact) {
        if self.ordinary_hit.is_none()
            && !self.reception_is_terminal()
            && hit.attacker.owner == self.consent.owner().seed()
            && hit.attacker.hull == self.consent.ship().ordinal()
            && hit.attacker.visit > self.consent.visit()
            && hit.tick >= self.confirmed_tick
        {
            self.ordinary_hit = Some(hit);
            self.clear_failed_reception();
        }
    }
    fn sortie(self) -> Option<u64> {
        self.consent.visit().checked_add(1)
    }
    fn id(self, ordinal: usize) -> PracticeBeaconId {
        PracticeBeaconId {
            owner: self.consent.owner().seed(),
            visit: self.consent.visit() + 1,
            ordinal: ordinal as u8,
        }
    }
    fn active_range(self) -> Option<Range> {
        (self.retired.is_none() && self.ordinary_hit.is_none() && !self.reception_is_terminal())
            .then_some(self.range)
            .flatten()
    }
    pub(super) fn after_step(
        &mut self,
        opening: FlightMode,
        next: Snapshot,
        nav: NavigationState,
        data: &OriginalStationData,
    ) {
        if self.reception_is_terminal() || self.retired.is_some() || opening == FlightMode::Docked {
            return;
        }
        let nav = nav.snapshot(0);
        if Some(nav.visit) != self.sortie()
            || nav.system != SystemId::AVENOR
            || next.mode == FlightMode::Docked
        {
            self.retired = Some(Retirement {
                tick: next.tick,
                visit: nav.visit,
            });
            return;
        }
        if self.range.is_none() && next.mode == FlightMode::Flight && nav.travel.is_none() {
            let opening = RecordedPose::from_snapshot(next);
            if let Some(pose) = opening.pose()
                && next
                    .station
                    .is_some_and(|station| geometry::range_clear(data, station, pose))
            {
                self.range = Some(Range {
                    opening,
                    opened_tick: next.tick,
                    hits: [None; 3],
                });
            }
        }
        self.observe_quiet_step(next.tick);
    }
    pub(super) fn contact_limited(
        &mut self,
        shot: Snapshot,
        pose: RigidPose,
        limit: f64,
        station_distance: Option<f64>,
    ) -> Option<(PracticeBeaconId, f64, bool)> {
        let range = self.active_range()?;
        let (ordinal, distance) =
            geometry::nearest_segment(range.opening.pose()?, pose, limit, station_distance)?;
        let id = self.id(ordinal);
        let slot = &mut self.range.as_mut()?.hits[ordinal];
        let first_hit = slot.is_none();
        if first_hit {
            *slot = Some(FirstHit {
                tick: shot.tick,
                firing_pose: RecordedPose::from_snapshot(shot),
                missile: None,
            });
        }
        Some((id, distance, first_hit))
    }
    pub(super) fn missile_owner_matches(self, owner: super::CombatIdentity) -> bool {
        owner.owner == self.consent.owner().seed()
            && owner.hull == self.consent.ship().ordinal()
            && owner.system == SystemId::AVENOR
            && Some(owner.visit) == self.sortie()
    }
    pub(super) fn missile_docked_since_consent(self, sequence: u64) -> bool {
        sequence > self.consent.sequence()
    }
    pub(super) fn missile_provider_ended_at(self) -> Option<u64> {
        self.ordinary_hit.map(|h| h.tick)
    }
    pub(super) fn missile_retired_at(self, tick: u64) -> bool {
        self.retired.is_some_and(|r| r.tick == tick)
    }
    pub(super) fn retained_missile_target(self, id: PracticeBeaconId) -> Option<RigidPose> {
        let frame = self.range?.opening.pose()?;
        (0..3)
            .find(|i| self.id(*i) == id)
            .map(|i| geometry::beacon_pose(frame, i))
    }
    pub(super) fn retained_missile_contact(
        self,
        ray: RigidPose,
        limit: f64,
        station: Option<f64>,
    ) -> Option<(PracticeBeaconId, f64)> {
        let frame = self.range?.opening.pose()?;
        let (ordinal, distance) = geometry::nearest_segment(frame, ray, limit, station)?;
        Some((self.id(ordinal), distance))
    }
    pub(super) fn missile_first_hit_status(
        self,
        id: PracticeBeaconId,
        origin: super::MissileOrigin,
        tick: u64,
    ) -> Option<bool> {
        let i = (0..3).find(|i| self.id(*i) == id)?;
        let hit = self.range?.hits[i]?;
        if hit.tick == tick && hit.missile == Some(origin) {
            return Some(true);
        }
        if hit.tick < tick
            || (hit.tick == tick
                && hit
                    .missile
                    .is_some_and(|other| other.projectile.entity() < origin.projectile.entity()))
        {
            return Some(false);
        }
        None
    }
    pub(super) fn missile_receipts(
        self,
    ) -> [(PracticeBeaconId, Option<(u64, super::MissileOrigin)>); 3] {
        core::array::from_fn(|i| {
            (
                self.id(i),
                self.range
                    .and_then(|r| r.hits[i])
                    .and_then(|h| h.missile.map(|o| (h.tick, o))),
            )
        })
    }
    pub(super) fn missile_provider_interval(self, launch: u64, last_motion: u64) -> bool {
        self.range.is_some_and(|r| launch >= r.opened_tick)
            && launch > self.confirmed_tick
            && self.retired.is_none_or(|r| last_motion < r.tick)
            && self.ordinary_hit.is_none_or(|h| last_motion <= h.tick)
    }
    pub(super) fn missile_target(self, id: PracticeBeaconId) -> Option<RigidPose> {
        let range = self.active_range()?;
        let frame = range.opening.pose()?;
        (0..3)
            .find(|ordinal| self.id(*ordinal) == id)
            .map(|ordinal| geometry::beacon_pose(frame, ordinal))
    }
    pub(super) fn missile_contact(
        self,
        ray: RigidPose,
        range: f64,
        obstacle: Option<f64>,
    ) -> Option<(PracticeBeaconId, f64)> {
        let active = self.active_range()?;
        let (ordinal, distance) =
            geometry::nearest_segment(active.opening.pose()?, ray, range, obstacle)?;
        Some((self.id(ordinal), distance))
    }
    pub(super) fn record_missile_hit(
        &mut self,
        id: PracticeBeaconId,
        origin: super::MissileOrigin,
        flight: Snapshot,
    ) -> bool {
        let Some(ordinal) = (0..3).find(|ordinal| self.id(*ordinal) == id) else {
            return false;
        };
        if self.active_range().is_none() {
            return false;
        }
        let Some(range) = self.range.as_mut() else {
            return false;
        };
        let slot = &mut range.hits[ordinal];
        if slot.is_none() {
            *slot = Some(FirstHit {
                tick: flight.tick,
                firing_pose: RecordedPose::from_snapshot(flight),
                missile: Some(origin),
            });
            return true;
        }
        false
    }
    fn snapshot(self, tick: u64) -> PracticeSnapshot {
        let beacons = self.active_range().and_then(|range| {
            let frame = range.opening.pose()?;
            Some(core::array::from_fn(|ordinal| {
                let hit = range.hits[ordinal];
                PracticeBeaconSnapshot {
                    id: self.id(ordinal),
                    pose: geometry::beacon_pose(frame, ordinal),
                    hit: hit.is_some(),
                    missile_hit: range.hits[ordinal].and_then(|hit| hit.missile),
                    flashing: hit
                        .is_some_and(|receipt| tick.saturating_sub(receipt.tick) <= FLASH_TICKS),
                }
            }))
        });
        let status = if self.ordinary_hit.is_some() {
            PracticeStatus::Failed
        } else if self.retired.is_some() || self.cancel.is_some() {
            PracticeStatus::Retired
        } else if beacons.is_some() {
            PracticeStatus::Active
        } else {
            PracticeStatus::Consented
        };
        PracticeSnapshot {
            status,
            beacons,
            first_hits: self
                .range
                .map_or(0, |range| range.hits.iter().flatten().count() as u8),
        }
    }
}

/// Dropping this value discards consent. The exclusive borrow prevents the
/// live World changing between preparation, file publication and commit.
pub struct PreparedPracticeConfirmation<'a> {
    world: &'a mut World,
    practice: Practice,
    bytes: Vec<u8>,
    history: Vec<Practice>,
    trace: Option<PracticeAttemptId>,
}
impl PreparedPracticeConfirmation<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Call after the host has published the prepared bytes.
    pub fn commit(self) {
        self.world.practice = Some(self.practice);
        self.world.practice_history = self.history;
        self.world.practice_trace = self.trace;
    }
}

impl World {
    pub fn review_practice(&self, expected_tick: u64) -> Result<PracticeReview, PracticeError> {
        if expected_tick != self.state.tick {
            return Err(PracticeError::StaleReview);
        }
        let session = self
            .freight_session
            .as_ref()
            .ok_or(PracticeError::NoCommander)?;
        if self.is_paused() {
            return Err(PracticeError::Paused);
        }
        let dock = self
            .current_dock_context()
            .filter(|dock| dock.system() == SystemId::AVENOR)
            .ok_or(PracticeError::AvenorBerthRequired)?;
        if self.practice.is_some_and(|p| !p.reception_is_terminal()) {
            return Err(PracticeError::AlreadyConsented);
        }
        self.check_practice_capacity()?;
        Ok(PracticeReview {
            dock,
            tick: expected_tick,
            epoch: session.epoch(),
            fingerprint: self.practice_fingerprint(),
            attempt: lifecycle::next_attempt_id(
                dock.owner().seed(),
                self.practice_history.len() + usize::from(self.practice.is_some()),
            ),
        })
    }

    /// Stage the exact new save while leaving the live World untouched.
    /// Give the bytes to the owned profile store, then commit after publication.
    pub fn prepare_practice_confirmation(
        &mut self,
        review: PracticeReview,
        expected_tick: u64,
    ) -> Result<PreparedPracticeConfirmation<'_>, PracticeError> {
        let current = self.review_practice(expected_tick)?;
        if review.dock != current.dock
            || review.epoch != current.epoch
            || review.tick > current.tick
            || review.fingerprint != current.fingerprint
            || review.attempt != current.attempt
        {
            return Err(PracticeError::StaleReview);
        }
        let mut history = self.practice_history.clone();
        if let Some(previous) = self.practice {
            history.push(previous);
        }
        let practice = Practice {
            attempt: current.attempt.ordinal(),
            settlement: None,
            cancel: None,
            consent: current.dock,
            confirmed_tick: expected_tick,
            range: None,
            retired: None,
            reception: reception::Reception::default(),
            ordinary_hit: None,
            pending_missiles: 0,
        };
        let bytes = self
            .save_bytes_with_practice_history(Some(practice), &history, self.practice_trace)
            .map_err(PracticeError::Save)?;
        let trace = self.practice_trace;
        Ok(PreparedPracticeConfirmation {
            world: self,
            practice,
            bytes,
            history,
            trace,
        })
    }

    pub fn practice_snapshot(&self) -> Option<PracticeSnapshot> {
        self.commander_account.as_ref()?;
        Some(self.practice.map_or(
            PracticeSnapshot {
                status: PracticeStatus::Unaccepted,
                beacons: None,
                first_hits: 0,
            },
            |practice| practice.snapshot(self.state.tick),
        ))
    }
}

#[cfg(test)]
pub(super) mod tests;

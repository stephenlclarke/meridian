//! Original T02 uncrewed practice provider for the current Avenor sortie.
use super::{FlightMode, Snapshot, World, flight_pose};
use crate::docking::{RigidPose, Vec3};
use crate::navigation::{DockContext, NavigationState, SystemId};
use crate::station::OriginalStationData;

mod geometry;
mod persistence;

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
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeError {
    StaleReview,
    Paused,
    NoCommander,
    AvenorBerthRequired,
    AlreadyConsented,
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
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PracticeBeaconSnapshot {
    pub id: PracticeBeaconId,
    pub pose: RigidPose,
    pub hit: bool,
    pub flashing: bool,
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
    consent: DockContext,
    confirmed_tick: u64,
    range: Option<Range>,
    retired: Option<Retirement>,
}
impl Practice {
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
        self.retired.is_none().then_some(self.range).flatten()
    }
    pub(super) fn after_step(
        &mut self,
        opening: FlightMode,
        next: Snapshot,
        nav: NavigationState,
        data: &OriginalStationData,
    ) {
        if self.retired.is_some() || opening == FlightMode::Docked {
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
    }
    pub(super) fn contact(
        &mut self,
        shot: Snapshot,
        pose: RigidPose,
        station_distance: Option<f64>,
    ) -> Option<(PracticeBeaconId, f64, bool)> {
        let range = self.active_range()?;
        let (ordinal, distance) = geometry::nearest(range.opening.pose()?, pose, station_distance)?;
        let id = self.id(ordinal);
        let slot = &mut self.range.as_mut()?.hits[ordinal];
        let first_hit = slot.is_none();
        if first_hit {
            *slot = Some(FirstHit {
                tick: shot.tick,
                firing_pose: RecordedPose::from_snapshot(shot),
            });
        }
        Some((id, distance, first_hit))
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
                    flashing: hit
                        .is_some_and(|receipt| tick.saturating_sub(receipt.tick) <= FLASH_TICKS),
                }
            }))
        });
        let status = if self.retired.is_some() {
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
}
impl PreparedPracticeConfirmation<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Call after the host has published the prepared bytes.
    pub fn commit(self) {
        self.world.practice = Some(self.practice);
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
        if self.practice.is_some() {
            return Err(PracticeError::AlreadyConsented);
        }
        Ok(PracticeReview {
            dock,
            tick: expected_tick,
            epoch: session.epoch(),
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
        {
            return Err(PracticeError::StaleReview);
        }
        let practice = Practice {
            consent: current.dock,
            confirmed_tick: expected_tick,
            range: None,
            retired: None,
        };
        let bytes = self
            .save_bytes_with_practice(Some(practice))
            .map_err(PracticeError::Save)?;
        Ok(PreparedPracticeConfirmation {
            world: self,
            practice,
            bytes,
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
mod tests;

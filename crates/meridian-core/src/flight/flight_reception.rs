//! Original T01. Only World flight steps create approach evidence; reviews confer no flight authority.
use super::{FlightMode, Snapshot, World, flight_pose};
use crate::docking::{RigidPose, Vec3};
use crate::navigation::{DockContext, DockOrigin, PlayerShipId, SavedNavigationSummary};
use crate::save::{Reader, SaveError, Writer};

mod persistence;
mod sight;

pub const FLIGHT_RECEPTION_MAX_BERTH_RECORDS: usize = 16;
const MAX_RECORDS: usize = 128;
const MAX_ACTIVE: usize = 3;
const INVALID: SaveError =
    SaveError::InvalidState("T01 permission, approach or terminal chronology");

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FlightView {
    #[default]
    Front,
    Rear,
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightReceptionAction {
    Accept,
    Receive,
    Cancel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightReceptionOutcome {
    Complete,
    Abandoned,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightReceptionError {
    NoCommander,
    StaleReview,
    Paused,
    BerthRequired,
    AlreadyActive,
    NoActiveAttempt,
    NotReady,
    BerthCapacity,
    MissionCapacity,
    SharedCapacity,
    Save(SaveError),
}
impl core::fmt::Display for FlightReceptionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "flight practice refused: {self:?}")
    }
}
impl std::error::Error for FlightReceptionError {}

/// A retained ID is descriptive, never an acceptance or reception capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlightReceptionId {
    owner: u64,
    ordinal: u8,
}
impl FlightReceptionId {
    pub fn owner(self) -> u64 {
        self.owner
    }
    pub fn ordinal(self) -> u8 {
        self.ordinal
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlightReceptionReview {
    action: FlightReceptionAction,
    dock: DockContext,
    tick: u64,
    epoch: u64,
    fingerprint: u64,
}
impl FlightReceptionReview {
    pub fn action(self) -> FlightReceptionAction {
        self.action
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlightReceptionTrace {
    pub source_id: &'static str,
    pub clue_id: &'static str,
    pub receipt_id: &'static str,
    pub attempt: FlightReceptionId,
    pub dock_sequence: u64,
    pub confirmed_tick: u64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightReceptionAttemptSnapshot {
    pub id: FlightReceptionId,
    pub issuing_dock: DockContext,
    pub accepted_tick: u64,
    pub measured_ticks: Option<[u64; 2]>,
    pub measured_ranges: Option<[f64; 2]>,
    pub measured_visit: Option<u64>,
    pub outcome: Option<FlightReceptionOutcome>,
    pub terminal_tick: Option<u64>,
    pub terminal_dock: Option<DockContext>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct FlightReceptionSnapshot {
    pub view: FlightView,
    pub attempts: Vec<FlightReceptionAttemptSnapshot>,
    pub active: Option<FlightReceptionId>,
    pub ready: bool,
    pub trace: Option<FlightReceptionTrace>,
    pub retained_mission_records: usize,
    pub active_services: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fit {
    inventory: u64,
    items: [Option<u64>; 8],
    equipment_revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct SightBody {
    ordinal: u8,
    position: Vec3,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Sample {
    flight: Snapshot,
    visit: u64,
    physical_tick: u64,
    bodies: [Option<SightBody>; 3],
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Reading {
    first: Sample,
    last: Sample,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Terminal {
    outcome: FlightReceptionOutcome,
    tick: u64,
    dock: Option<DockContext>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Attempt {
    dock: DockContext,
    accepted_tick: u64,
    fit: Fit,
    reading: Option<Reading>,
    terminal: Option<Terminal>,
}
#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct State {
    view: FlightView,
    attempts: Vec<Attempt>,
    trace: Option<u8>,
}
impl State {
    fn active(&self) -> Option<&Attempt> {
        self.attempts.last().filter(|a| a.terminal.is_none())
    }
    pub(super) fn occupancy(&self) -> (usize, usize) {
        (self.attempts.len(), usize::from(self.active().is_some()))
    }
    fn id(&self, ordinal: usize) -> FlightReceptionId {
        FlightReceptionId {
            owner: self.attempts[ordinal].dock.owner().seed(),
            ordinal: ordinal as u8,
        }
    }
    fn trace_snapshot(&self) -> Option<FlightReceptionTrace> {
        let ordinal = usize::from(self.trace?);
        let terminal = self.attempts.get(ordinal)?.terminal?;
        Some(FlightReceptionTrace {
            source_id: "l337.training/flight-reception",
            clue_id: "flight-reception-family-trace",
            receipt_id: "l337.unwritten-route/clue/flight-reception-family-trace",
            attempt: self.id(ordinal),
            dock_sequence: terminal.dock?.sequence(),
            confirmed_tick: terminal.tick,
        })
    }
}
impl Attempt {
    fn must_fail(self, carrier: Option<PlayerShipId>, destroyed: bool) -> bool {
        carrier != Some(self.dock.ship()) || destroyed
    }
    fn return_ready(self, dock: DockContext) -> bool {
        self.terminal.is_none()
            && self.reading.is_some_and(|reading| {
                dock.origin() == DockOrigin::PhysicalEntry
                    && same_berth(self.dock, dock)
                    && dock.sequence() > self.dock.sequence()
                    && dock.visit() >= reading.last.visit
                    && dock.tick() > reading.last.flight.tick
            })
    }
}
fn same_berth(a: DockContext, b: DockContext) -> bool {
    a.owner() == b.owner()
        && a.ship() == b.ship()
        && a.system() == b.system()
        && a.actor_entity() == b.actor_entity()
}

/// Host publishes bytes atomically before commit. Dropping this borrow changes nothing.
pub struct PreparedFlightReception<'a> {
    world: &'a mut World,
    candidate: State,
    bytes: Vec<u8>,
}
impl PreparedFlightReception<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn commit(self) {
        self.world.flight_reception = self.candidate;
    }
}
impl World {
    fn flight_reception_fit(&self) -> Option<Fit> {
        let (inventory, items, equipment_revision) =
            self.commander_account.as_ref()?.owned_fit_identity()?;
        Some(Fit {
            inventory,
            items,
            equipment_revision,
        })
    }
    pub fn flight_view(&self) -> FlightView {
        self.flight_reception.view
    }
    /// Normal view command. No caller-supplied measurement or presentation flag is accepted.
    pub fn request_flight_view(
        &mut self,
        view: FlightView,
        expected_tick: u64,
    ) -> Result<(), FlightReceptionError> {
        self.flight_reception_admission(expected_tick)?;
        self.flight_reception.view = view;
        Ok(())
    }
    fn flight_reception_admission(&self, tick: u64) -> Result<(), FlightReceptionError> {
        if tick != self.state.tick {
            return Err(FlightReceptionError::StaleReview);
        }
        if self.commander_account.is_none() {
            return Err(FlightReceptionError::NoCommander);
        }
        if self.is_paused() {
            return Err(FlightReceptionError::Paused);
        }
        Ok(())
    }
    // Only implemented owners count: original G02, T02, T01 and T04.
    pub(super) fn original_service_occupancy(&self) -> (usize, usize) {
        let (records, active) = self.flight_reception.occupancy();
        let (t04_records, t04_active) = self.navigation_trade.occupancy();
        let practice_count = self.practice_history.len() + usize::from(self.practice.is_some());
        (
            records
                + t04_records
                + practice_count
                + self
                    .freight_session
                    .as_ref()
                    .map_or(0, |s| s.state.retained_mission_records()),
            active
                + t04_active
                + usize::from(self.active_freight_job().is_some())
                + usize::from(self.practice.is_some_and(|p| !p.reception_is_terminal())),
        )
    }
    pub(super) fn original_service_has_capacity(&self) -> bool {
        let (records, active) = self.original_service_occupancy();
        records < MAX_RECORDS && active < MAX_ACTIVE
    }
    fn flight_reception_berth(&self) -> Option<DockContext> {
        let dock = self.current_dock_context()?;
        let station = self.station_data.as_ref()?;
        // Supported original sites use the admitted immutable, live SH028 main-station scene.
        // There is no caller-set powered flag and no unsupported destructible station is admitted.
        if self.state.station != Some(station.station_pose())
            || ![
                crate::navigation::SystemId::AVENOR,
                crate::navigation::SystemId::RENAS,
            ]
            .contains(&dock.system())
            || self.combat_snapshot()?.player.destroyed
        {
            return None;
        }
        Some(dock)
    }
    pub fn review_flight_reception(
        &self,
        action: FlightReceptionAction,
        expected_tick: u64,
    ) -> Result<FlightReceptionReview, FlightReceptionError> {
        self.flight_reception_admission(expected_tick)?;
        let dock = self
            .flight_reception_berth()
            .ok_or(FlightReceptionError::BerthRequired)?;
        self.flight_reception_fit()
            .ok_or(FlightReceptionError::NoCommander)?;
        match action {
            FlightReceptionAction::Accept => self.check_flight_reception_capacity(dock)?,
            FlightReceptionAction::Cancel | FlightReceptionAction::Receive => {
                let attempt = self
                    .flight_reception
                    .active()
                    .ok_or(FlightReceptionError::NoActiveAttempt)?;
                if !same_berth(attempt.dock, dock) {
                    return Err(FlightReceptionError::BerthRequired);
                }
                if action == FlightReceptionAction::Receive && !attempt.return_ready(dock) {
                    return Err(FlightReceptionError::NotReady);
                }
            }
        }
        let fingerprint = self
            .flight_reception_material_fingerprint()
            .map_err(FlightReceptionError::Save)?;
        Ok(FlightReceptionReview {
            action,
            dock,
            tick: expected_tick,
            epoch: self
                .freight_session
                .as_ref()
                .ok_or(FlightReceptionError::NoCommander)?
                .epoch(),
            fingerprint,
        })
    }
    fn check_flight_reception_capacity(
        &self,
        dock: DockContext,
    ) -> Result<(), FlightReceptionError> {
        if self.flight_reception.active().is_some() {
            return Err(FlightReceptionError::AlreadyActive);
        }
        if self
            .flight_reception
            .attempts
            .iter()
            .filter(|a| a.dock.system() == dock.system())
            .count()
            >= FLIGHT_RECEPTION_MAX_BERTH_RECORDS
        {
            return Err(FlightReceptionError::BerthCapacity);
        }
        let (records, active) = self.original_service_occupancy();
        if records >= MAX_RECORDS {
            return Err(FlightReceptionError::MissionCapacity);
        }
        if active >= MAX_ACTIVE {
            return Err(FlightReceptionError::SharedCapacity);
        }
        Ok(())
    }
    pub fn prepare_flight_reception(
        &mut self,
        review: FlightReceptionReview,
        expected_tick: u64,
    ) -> Result<PreparedFlightReception<'_>, FlightReceptionError> {
        let current = self.review_flight_reception(review.action, expected_tick)?;
        if review.dock != current.dock
            || review.epoch != current.epoch
            || review.tick > current.tick
            || review.fingerprint != current.fingerprint
        {
            return Err(FlightReceptionError::StaleReview);
        }
        let mut candidate = self.flight_reception.clone();
        match review.action {
            FlightReceptionAction::Accept => candidate.attempts.push(Attempt {
                dock: current.dock,
                accepted_tick: current.tick,
                fit: self
                    .flight_reception_fit()
                    .ok_or(FlightReceptionError::NoCommander)?,
                reading: None,
                terminal: None,
            }),
            FlightReceptionAction::Receive | FlightReceptionAction::Cancel => {
                let ordinal = candidate.attempts.len() - 1;
                let outcome = if review.action == FlightReceptionAction::Receive {
                    FlightReceptionOutcome::Complete
                } else {
                    FlightReceptionOutcome::Abandoned
                };
                candidate.attempts[ordinal].terminal = Some(Terminal {
                    outcome,
                    tick: current.tick,
                    dock: Some(current.dock),
                });
                if outcome == FlightReceptionOutcome::Complete && candidate.trace.is_none() {
                    candidate.trace = Some(ordinal as u8);
                }
            }
        }
        let bytes = self
            .save_bytes_with_flight_reception(&candidate)
            .map_err(FlightReceptionError::Save)?;
        Ok(PreparedFlightReception {
            world: self,
            candidate,
            bytes,
        })
    }
    pub fn flight_reception_snapshot(&self) -> Option<FlightReceptionSnapshot> {
        self.commander_account.as_ref()?;
        let state = &self.flight_reception;
        let (records, active) = self.original_service_occupancy();
        Some(FlightReceptionSnapshot {
            view: state.view,
            attempts: state
                .attempts
                .iter()
                .enumerate()
                .map(|(i, a)| FlightReceptionAttemptSnapshot {
                    id: state.id(i),
                    issuing_dock: a.dock,
                    accepted_tick: a.accepted_tick,
                    measured_ticks: a.reading.map(|r| [r.first.flight.tick, r.last.flight.tick]),
                    measured_ranges: a.reading.map(|r| [r.first.range(), r.last.range()]),
                    measured_visit: a.reading.map(|r| r.last.visit),
                    outcome: a.terminal.map(|t| t.outcome),
                    terminal_tick: a.terminal.map(|t| t.tick),
                    terminal_dock: a.terminal.and_then(|t| t.dock),
                })
                .collect(),
            active: state.active().map(|_| state.id(state.attempts.len() - 1)),
            ready: state
                .active()
                .zip(self.flight_reception_berth())
                .is_some_and(|(a, d)| a.return_ready(d)),
            trace: state.trace_snapshot(),
            retained_mission_records: records,
            active_services: active,
        })
    }
    pub(super) fn flight_reception_start(&self) -> Option<Sample> {
        let attempt = self.flight_reception.active()?;
        if attempt.reading.is_some()
            || self.flight_reception.view != FlightView::Front
            || self.flight_reception_fit()? != attempt.fit
            || self.is_paused()
        {
            return None;
        }
        let nav = self.navigation?;
        let identity = self.combat_identity()?;
        if identity.owner != attempt.dock.owner().seed()
            || identity.hull != attempt.dock.ship().ordinal()
            || identity.system != attempt.dock.system()
            || identity.visit <= attempt.dock.visit()
            || nav.snapshot(0).travel.is_some()
            || self.combat_snapshot()?.player.destroyed
        {
            return None;
        }
        let sample = self.flight_reception_sample(identity.visit)?;
        (sample.qualifies() && self.flight_sample_sight_clear(sample)).then_some(sample)
    }
    pub(super) fn observe_flight_reception(&mut self, start: Option<Sample>) {
        let Some(attempt) = self.flight_reception.active().copied() else {
            return;
        };
        if attempt.must_fail(
            self.player_ship_id(),
            self.combat_snapshot().is_some_and(|s| s.player.destroyed),
        ) {
            let last = self.flight_reception.attempts.len() - 1;
            self.flight_reception.attempts[last].terminal = Some(Terminal {
                outcome: FlightReceptionOutcome::Failed,
                tick: self.state.tick,
                dock: None,
            });
            return;
        }
        if let (Some(first), Some(last)) = (start, self.flight_reception_start())
            && Reading::qualifies(first, last)
        {
            let index = self.flight_reception.attempts.len() - 1;
            self.flight_reception.attempts[index].reading = Some(Reading { first, last });
        }
    }
}

impl Sample {
    fn pose(self) -> Option<RigidPose> {
        flight_pose(self.flight.position, self.flight.orientation)
    }
    fn range(self) -> f64 {
        self.pose().map_or(f64::NAN, |p| length(p.position))
    }
    fn qualifies(self) -> bool {
        let Some(pose) = self.pose() else {
            return false;
        };
        let distance = length(pose.position);
        let Some(forward) = unit(pose.orientation.z) else {
            return false;
        };
        let toward = scale(pose.position, -1.0 / distance);
        self.flight.mode == FlightMode::Flight
            && self.flight.motion_refusal.is_none()
            && (1000.0..=1500.0).contains(&distance)
            && self.flight.speed > 0.0
            && toward.dot(forward) >= (2.0 * core::f64::consts::PI / 180.0).cos()
            && self.visit > 0
    }
}
impl Reading {
    fn step_matches(first: Snapshot, last: Snapshot) -> bool {
        let mut expected = first.position;
        for (position, direction) in expected.iter_mut().zip(super::forward(last.orientation)) {
            *position += direction
                * last.speed
                * super::FIXED_DELTA_SECONDS
                * crate::station::DISTANCE_PER_SPEED_UNIT;
        }
        expected == last.position
            && (last.speed - first.speed).abs()
                <= super::ACCELERATION * super::FIXED_DELTA_SECONDS + 1e-5
    }
    fn qualifies(first: Sample, last: Sample) -> bool {
        // A World step is exactly 1/60s (0.01666666753590107 as stored f32), below0.1s.
        // Consecutive physical and simulation clocks forbid docked, paused or skipped intervals.
        first.qualifies()
            && last.qualifies()
            && first.visit == last.visit
            && first.flight.tick.checked_add(1) == Some(last.flight.tick)
            && first.physical_tick.checked_add(1) == Some(last.physical_tick)
            && last.range() < first.range()
            && Self::step_matches(first.flight, last.flight)
            && first
                .pose()
                .zip(last.pose())
                .is_some_and(|(a, b)| length(sub(b.position, a.position)) > 1e-9)
    }
}
fn length(v: Vec3) -> f64 {
    v.dot(v).sqrt()
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn scale(a: Vec3, s: f64) -> Vec3 {
    Vec3::new(a.x * s, a.y * s, a.z * s)
}
fn unit(v: Vec3) -> Option<Vec3> {
    let n = length(v);
    (n.is_finite() && n > 1e-9).then(|| scale(v, n.recip()))
}

#[cfg(test)]
mod tests;

//! Retained original Avenor attempts. Reviews confer no terminal or allocation authority alone.
use super::*;
use crate::navigation::SavedNavigationSummary;
use crate::save::{Reader, SaveError, Writer, checksum};
pub const PRACTICE_MAX_OFFERS: usize = 16;
const MAX_MISSION_RECORDS: usize = 128;
const MAX_ACTIVE_SERVICES: usize = 3;
const INVALID: SaveError = SaveError::InvalidState("practice attempt history or terminal receipt");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeAttemptId {
    pub(super) owner: u64,
    pub(super) ordinal: u8,
}
impl PracticeAttemptId {
    pub fn owner(self) -> u64 {
        self.owner
    }
    pub fn ordinal(self) -> u8 {
        self.ordinal
    }
    pub(in crate::flight) fn encode(self, w: &mut Writer) {
        w.u64(self.owner);
        w.u8(self.ordinal);
    }
    pub(in crate::flight) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            owner: r.u64()?,
            ordinal: r.u8()?,
        })
    }
}
pub(super) fn next_attempt_id(owner: u64, ordinal: usize) -> PracticeAttemptId {
    PracticeAttemptId {
        owner,
        ordinal: ordinal as u8,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeOutcome {
    Complete,
    Failed,
    Abandoned,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Closure {
    pub(super) attempt: PracticeAttemptId,
    pub(super) dock: DockContext,
    pub(super) tick: u64,
    pub(super) outcome: PracticeOutcome,
}
impl Closure {
    pub(super) fn encode(self, w: &mut Writer) {
        self.attempt.encode(w);
        self.dock.encode_save(w);
        w.u64(self.tick);
        w.u8(match self.outcome {
            PracticeOutcome::Failed => 1,
            PracticeOutcome::Abandoned => 2,
            PracticeOutcome::Complete => 0,
        });
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            attempt: PracticeAttemptId::decode(r)?,
            dock: DockContext::decode_save(r)?,
            tick: r.u64()?,
            outcome: match r.u8()? {
                1 => PracticeOutcome::Failed,
                2 => PracticeOutcome::Abandoned,
                _ => return Err(INVALID),
            },
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeClosureReview {
    dock: DockContext,
    tick: u64,
    epoch: u64,
    fingerprint: u64,
    attempt: PracticeAttemptId,
    outcome: PracticeOutcome,
}
impl PracticeClosureReview {
    pub fn outcome(self) -> PracticeOutcome {
        self.outcome
    }
    pub fn attempt(self) -> PracticeAttemptId {
        self.attempt
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PracticeAttemptSnapshot {
    pub id: PracticeAttemptId,
    pub outcome: Option<PracticeOutcome>,
    pub confirmed_tick: u64,
    pub terminal_tick: Option<u64>,
    pub consent_dock_sequence: u64,
    pub targets: Option<[PracticeBeaconId; 3]>,
    pub first_hits: u8,
    pub ordinary_hit: Option<crate::OrdinaryImpact>,
    pub awarded_first_trace: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub struct PracticeLedgerSnapshot {
    pub attempts: Vec<PracticeAttemptSnapshot>,
    pub active: Option<PracticeAttemptId>,
    pub offers_remaining: usize,
    pub retained_mission_records: usize,
    pub active_services: usize,
    pub trace: Option<PracticeTrace>,
}

impl Practice {
    pub(in crate::flight) fn is_cancelled(self) -> bool {
        self.cancel.is_some()
    }
    pub(in crate::flight) fn attempt_id(self) -> PracticeAttemptId {
        next_attempt_id(self.consent.owner().seed(), usize::from(self.attempt))
    }
    pub(in crate::flight) fn outcome(self) -> Option<PracticeOutcome> {
        self.settlement
    }
    pub(in crate::flight) fn terminal_tick(self) -> Option<u64> {
        self.cancel.map(|c| c.tick).or(self.completed_tick())
    }
    pub(in crate::flight) fn accepts_incident(self, hit: crate::OrdinaryImpact) -> bool {
        hit.tick >= self.confirmation_tick()
            && hit.attacker.visit > self.consent_visit()
            && self.terminal_tick().is_none_or(|end| hit.tick <= end)
    }
    pub(in crate::flight) fn encode_full(self, w: &mut Writer) {
        self.encode_reception(w);
        self.encode(w);
        w.option(self.ordinary_hit.as_ref(), |w, h| h.encode(w));
    }
    pub(in crate::flight) fn decode_full(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let reception = reception::Reception::decode(r)?;
        let mut p = Self::decode(r, 9)?;
        p.reception = reception;
        p.ordinary_hit = r.option(super::super::combat::OrdinaryImpact::decode)?;
        Ok(p)
    }
    pub(in crate::flight) fn validate_closure(
        self,
        nav: SavedNavigationSummary,
        tick: u64,
    ) -> Result<(), SaveError> {
        let expected = self
            .cancel
            .map(|c| c.outcome)
            .or(self.completed_tick().map(|_| PracticeOutcome::Complete));
        if self.settlement != expected {
            return Err(INVALID);
        }
        if let Some(c) = self.cancel {
            let d = c.dock;
            if self.completed_tick().is_some()
                || c.attempt != self.attempt_id()
                || c.tick < self.confirmed_tick
                || c.tick > tick
                || c.outcome
                    != if self.ordinary_hit.is_some() {
                        PracticeOutcome::Failed
                    } else {
                        PracticeOutcome::Abandoned
                    }
                || d.owner() != self.consent.owner()
                || d.ship() != self.consent.ship()
                || d.system() != SystemId::AVENOR
                || d.actor_entity() != self.consent.actor_entity()
                || d.sequence() < self.consent.sequence()
                || d.sequence() > nav.dock_sequence
                || d.visit() < self.consent.visit()
                || d.visit() < d.sequence()
                || d.visit() > nav.visit
                || d.tick() > c.tick
                || d.tick() < self.consent.tick()
                || (d.sequence() == self.consent.sequence() && d != self.consent)
                || (d.sequence() > self.consent.sequence()
                    && d.origin() != crate::navigation::DockOrigin::PhysicalEntry)
                || self
                    .retired
                    .is_some_and(|r| r.tick > c.tick || r.visit > d.visit())
                || (self.range.is_some()
                    && (self.retired.is_none() || d.sequence() <= self.consent.sequence()))
            {
                return Err(INVALID);
            }
            if self.ordinary_hit.is_some_and(|h| h.tick > c.tick) {
                return Err(INVALID);
            }
        }
        Ok(())
    }
}

pub(in crate::flight) fn lessons(
    current: Option<Practice>,
    history: &[Practice],
) -> impl Iterator<Item = Practice> + '_ {
    history.iter().copied().chain(current)
}
pub(in crate::flight) fn lesson_for_target(
    current: Option<Practice>,
    history: &[Practice],
    target: PracticeBeaconId,
) -> Option<Practice> {
    lessons(current, history).find(|p| p.retained_missile_target(target).is_some())
}
pub(in crate::flight) fn encode_history(
    w: &mut Writer,
    history: &[Practice],
    trace: Option<PracticeAttemptId>,
) {
    w.u8(history.len() as u8);
    for p in history {
        p.encode_full(w);
    }
    w.option(trace.as_ref(), |w, id| id.encode(w));
}
pub(in crate::flight) fn decode_history(
    r: &mut Reader<'_>,
) -> Result<(Vec<Practice>, Option<PracticeAttemptId>), SaveError> {
    let count = usize::from(r.u8()?);
    if count >= PRACTICE_MAX_OFFERS {
        return Err(INVALID);
    }
    let mut history = Vec::with_capacity(count);
    for _ in 0..count {
        history.push(Practice::decode_full(r)?);
    }
    Ok((history, r.option(PracticeAttemptId::decode)?))
}

impl World {
    fn practice_occupancy(
        &self,
        current: Option<Practice>,
        history: &[Practice],
    ) -> (usize, usize) {
        let records = self
            .freight_session
            .as_ref()
            .map_or(0, |s| s.state.retained_mission_records());
        let active = usize::from(self.active_freight_job().is_some())
            + usize::from(current.is_some_and(|p| !p.reception_is_terminal()));
        (
            records + history.len() + usize::from(current.is_some()),
            active,
        )
    }
    pub(in crate::flight) fn check_practice_capacity(&self) -> Result<(), PracticeError> {
        if self.practice_history.len() + usize::from(self.practice.is_some()) >= PRACTICE_MAX_OFFERS
        {
            return Err(PracticeError::OfferCapacity);
        }
        let (records, active) = self.practice_occupancy(self.practice, &self.practice_history);
        if records >= MAX_MISSION_RECORDS {
            return Err(PracticeError::MissionCapacity);
        }
        if active >= MAX_ACTIVE_SERVICES {
            return Err(PracticeError::SharedCapacity);
        }
        Ok(())
    }
    pub(in crate::flight) fn practice_fingerprint(&self) -> u64 {
        let mut w = Writer::new();
        encode_history(&mut w, &self.practice_history, self.practice_trace);
        w.option(self.practice.as_ref(), |w, p| p.encode_full(w));
        if let Some(account) = &self.commander_account {
            account.encode_save(&mut w);
        }
        if let Some(session) = &self.freight_session {
            session.state.encode_save(&mut w);
        }
        self.missiles.encode(&mut w);
        self.combat.encode(&mut w);
        checksum(&w.bytes)
    }
    pub(in crate::flight) fn first_practice_trace(&self) -> Option<PracticeTrace> {
        let id = self.practice_trace?;
        lessons(self.practice, &self.practice_history)
            .find(|p| p.attempt_id() == id)?
            .reception_snapshot(self.state.tick)
            .trace
    }
    pub fn practice_ledger_snapshot(&self) -> Option<PracticeLedgerSnapshot> {
        self.commander_account.as_ref()?;
        let (records, active) = self.practice_occupancy(self.practice, &self.practice_history);
        Some(PracticeLedgerSnapshot {
            attempts: lessons(self.practice, &self.practice_history)
                .map(|p| PracticeAttemptSnapshot {
                    id: p.attempt_id(),
                    outcome: p.outcome(),
                    confirmed_tick: p.confirmed_tick,
                    terminal_tick: p.terminal_tick(),
                    consent_dock_sequence: p.consent.sequence(),
                    targets: p.range.map(|_| core::array::from_fn(|i| p.id(i))),
                    first_hits: p.range.map_or(0, |r| r.hits.iter().flatten().count() as u8),
                    ordinary_hit: p.ordinary_hit,
                    awarded_first_trace: self.practice_trace == Some(p.attempt_id()),
                })
                .collect(),
            active: self
                .practice
                .filter(|p| !p.reception_is_terminal())
                .map(Practice::attempt_id),
            offers_remaining: PRACTICE_MAX_OFFERS
                .saturating_sub(self.practice_history.len() + usize::from(self.practice.is_some())),
            retained_mission_records: records,
            active_services: active,
            trace: self.first_practice_trace(),
        })
    }
    pub fn review_practice_closure(
        &self,
        expected_tick: u64,
    ) -> Result<PracticeClosureReview, PracticeError> {
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
            .filter(|d| d.system() == SystemId::AVENOR)
            .ok_or(PracticeError::AvenorBerthRequired)?;
        let p = self
            .practice
            .filter(|p| !p.reception_is_terminal())
            .ok_or(PracticeError::NoActiveAttempt)?;
        if dock.owner() != p.consent.owner()
            || dock.ship() != p.consent.ship()
            || dock.actor_entity() != p.consent.actor_entity()
        {
            return Err(PracticeError::AvenorBerthRequired);
        }
        Ok(PracticeClosureReview {
            dock,
            tick: expected_tick,
            epoch: session.epoch(),
            fingerprint: self.practice_fingerprint(),
            attempt: p.attempt_id(),
            outcome: if p.ordinary_hit.is_some() {
                PracticeOutcome::Failed
            } else {
                PracticeOutcome::Abandoned
            },
        })
    }
    pub fn prepare_practice_closure(
        &mut self,
        review: PracticeClosureReview,
        expected_tick: u64,
    ) -> Result<PreparedPracticeConfirmation<'_>, PracticeError> {
        let current = self.review_practice_closure(expected_tick)?;
        if review.dock != current.dock
            || review.epoch != current.epoch
            || review.tick > current.tick
            || review.fingerprint != current.fingerprint
            || review.attempt != current.attempt
            || review.outcome != current.outcome
        {
            return Err(PracticeError::StaleReview);
        }
        let mut practice = self.practice.ok_or(PracticeError::NoActiveAttempt)?;
        practice.settlement = Some(current.outcome);
        practice.cancel = Some(Closure {
            attempt: current.attempt,
            dock: current.dock,
            tick: current.tick,
            outcome: current.outcome,
        });
        let history = self.practice_history.clone();
        let trace = self.practice_trace;
        let bytes = self
            .save_bytes_with_practice_history(Some(practice), &history, trace)
            .map_err(PracticeError::Save)?;
        Ok(PreparedPracticeConfirmation {
            world: self,
            practice,
            bytes,
            history,
            trace,
        })
    }
    pub(in crate::flight) fn validate_practice_ledger(
        &self,
        current: Option<Practice>,
        history: &[Practice],
        trace: Option<PracticeAttemptId>,
        nav: SavedNavigationSummary,
    ) -> Result<(), SaveError> {
        let count = history.len() + usize::from(current.is_some());
        let (records, active) = self.practice_occupancy(current, history);
        if count > PRACTICE_MAX_OFFERS
            || records > MAX_MISSION_RECORDS
            || active > MAX_ACTIVE_SERVICES
            || (!history.is_empty() && current.is_none())
        {
            return Err(INVALID);
        }
        let mut previous: Option<Practice> = None;
        let mut first = None;
        for (i, p) in lessons(current, history).enumerate() {
            if usize::from(p.attempt) != i
                || p.consent.owner().seed() != nav.owner
                || (i < history.len() && !p.reception_is_terminal())
            {
                return Err(INVALID);
            }
            p.validate_closure(nav, self.state.tick)?;
            if let (Some(c), Some(dock)) = (p.cancel, self.navigation.and_then(|n| n.dock()))
                && c.dock.sequence() == dock.sequence()
                && c.dock != dock
            {
                return Err(INVALID);
            }
            if let Some(old) = previous
                && (old
                    .terminal_tick()
                    .is_none_or(|tick| tick > p.confirmed_tick)
                    || old.consent.sequence() > p.consent.sequence()
                    || (old.range.is_some() && old.consent.sequence() >= p.consent.sequence()))
            {
                return Err(INVALID);
            }
            if p.completed_tick().is_some() && first.is_none() {
                first = Some(p.attempt_id());
            }
            previous = Some(p);
        }
        if trace != first {
            return Err(INVALID);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

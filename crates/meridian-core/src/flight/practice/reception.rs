use super::*;
use crate::flight::weapons::PulseResources;
use crate::navigation::DockOrigin;
use crate::save::{Reader, SaveError, Writer};

pub const PRACTICE_QUIET_TICKS: u16 = 180;
const SOURCE_ID: &str = "l337.training/safe-gunnery";
const TRACE_ID: &str = "safe-gunnery-family-trace";
const RECEIPT_ID: &str = "l337.unwritten-route/clue/safe-gunnery-family-trace";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeReceptionReview {
    dock: DockContext,
    tick: u64,
    epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeTrace {
    pub source_id: &'static str,
    pub clue_id: &'static str,
    pub receipt_id: &'static str,
    pub owner_seed: u64,
    pub carrier_ordinal: u64,
    pub dock_sequence: u64,
    pub confirmed_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PracticeReceptionSnapshot {
    pub quiet_ticks: u16,
    pub ready: bool,
    pub trace: Option<PracticeTrace>,
    pub ordinary_hit: Option<crate::OrdinaryImpact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Completion {
    dock: DockContext,
    tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::flight) struct Reception {
    anchor: Option<u64>,
    completion: Option<Completion>,
}

impl Range {
    fn third_hit(self) -> Option<u64> {
        let [first, second, third] = self.hits;
        Some(first?.tick.max(second?.tick).max(third?.tick))
    }
}

impl Practice {
    pub(in crate::flight) fn validate_missile_quiet(
        self,
        projectiles: [Option<crate::flight::ProjectileSnapshot>; 3],
    ) -> Result<(), crate::save::SaveError> {
        let invalid =
            crate::save::SaveError::InvalidState("missile ceasefire endpoint or lesson failure");
        if let Some(anchor) = self.reception.anchor
            && let Some(third) = self.range.and_then(Range::third_hit)
        {
            for p in projectiles.into_iter().flatten() {
                let endpoint = if p.ended.is_some() {
                    p.last_tick
                } else {
                    p.origin.launched_tick
                };
                if endpoint >= third && anchor < endpoint {
                    return Err(invalid);
                }
            }
        }
        for p in projectiles.into_iter().flatten() {
            if matches!(
                p.contact,
                Some(
                    crate::flight::MissileContact::Station
                        | crate::flight::MissileContact::Player { .. }
                )
            ) && !self.reception_is_terminal()
                && p.last_tick >= self.confirmed_tick
                && self.ordinary_hit.is_none()
            {
                return Err(invalid);
            }
        }
        Ok(())
    }

    pub(in crate::flight) fn reception_is_terminal(self) -> bool {
        self.reception.completion.is_some()
    }
    pub(in crate::flight) fn clear_failed_reception(&mut self) {
        self.reception.anchor = None;
    }
    /// The active range is pinned to one uninterrupted Avenor flight visit.
    /// Neither a docked step nor the step that ends that visit earns quiet time.
    fn quiet_ticks(self, tick: u64) -> u16 {
        if self.ordinary_hit.is_some() || self.pending_missiles != 0 {
            return 0;
        }
        let Some(anchor) = self.reception.anchor else {
            return 0;
        };
        let last_active = self.retired.map_or(tick, |end| end.tick.saturating_sub(1));
        last_active
            .saturating_sub(anchor)
            .min(u64::from(PRACTICE_QUIET_TICKS)) as u16
    }

    pub(super) fn observe_quiet_step(&mut self, tick: u64) {
        if self.ordinary_hit.is_none()
            && self.pending_missiles == 0
            && self.reception.anchor.is_none()
            && self.active_range().and_then(Range::third_hit).is_some()
        {
            // A v5 save has receipts, but no proven interval. Start at the
            // first newly observed active endpoint and credit only later steps.
            self.reception.anchor = Some(tick);
        }
    }

    pub(in crate::flight) fn observe_discharge(&mut self, tick: u64) {
        if self.ordinary_hit.is_none()
            && self.reception.completion.is_none()
            && self.range.and_then(Range::third_hit).is_some()
        {
            self.reception.anchor = Some(tick);
        }
    }

    fn reception_dock_matches(self, dock: DockContext) -> bool {
        let Some(end) = self.retired else {
            return false;
        };
        dock.origin() == DockOrigin::PhysicalEntry
            && dock.owner() == self.consent.owner()
            && dock.ship() == self.consent.ship()
            && dock.system() == self.consent.system()
            && dock.actor_entity() == self.consent.actor_entity()
            && dock.sequence() > self.consent.sequence()
            && dock.visit() >= end.visit
            && dock.tick() >= end.tick
    }

    pub(super) fn reception_snapshot(self, tick: u64) -> PracticeReceptionSnapshot {
        let quiet_ticks = self.quiet_ticks(tick);
        PracticeReceptionSnapshot {
            ordinary_hit: self.ordinary_hit,
            quiet_ticks,
            ready: quiet_ticks == PRACTICE_QUIET_TICKS && self.reception.completion.is_none(),
            trace: self.reception.completion.map(|completed| PracticeTrace {
                source_id: SOURCE_ID,
                clue_id: TRACE_ID,
                receipt_id: RECEIPT_ID,
                owner_seed: completed.dock.owner().seed(),
                carrier_ordinal: completed.dock.ship().ordinal(),
                dock_sequence: completed.dock.sequence(),
                confirmed_tick: completed.tick,
            }),
        }
    }

    pub(in crate::flight) fn encode_reception(self, w: &mut Writer) {
        w.option(self.reception.anchor.as_ref(), |w, tick| w.u64(*tick));
        w.option(self.reception.completion.as_ref(), |w, completion| {
            completion.dock.encode_save(w);
            w.u64(completion.tick);
        });
    }
    pub(super) fn validate_reception(
        self,
        nav: crate::navigation::SavedNavigationSummary,
        flight: Snapshot,
        pulse: PulseResources,
        current_dock: Option<DockContext>,
    ) -> Result<(), SaveError> {
        let invalid = SaveError::InvalidState("T02 ceasefire or reception");
        if let Some(hit) = self.ordinary_hit {
            if self.reception.anchor.is_some()
                || self.reception.completion.is_some()
                || hit.attacker.owner != self.consent.owner().seed()
                || hit.attacker.hull != self.consent.ship().ordinal()
                || hit.attacker.visit <= self.consent.visit()
                || hit.tick < self.confirmed_tick
                || hit.tick > flight.tick
            {
                return Err(invalid);
            }
            return Ok(());
        }
        let Some(anchor) = self.reception.anchor else {
            return if self.reception.completion.is_none() {
                Ok(())
            } else {
                Err(invalid)
            };
        };
        let third_hit = self.range.and_then(Range::third_hit).ok_or(invalid)?;
        if anchor < third_hit || anchor > flight.tick {
            return Err(invalid);
        }

        let ready = self.quiet_ticks(flight.tick) == PRACTICE_QUIET_TICKS;
        if let Some(completed) = self.reception.completion {
            if !ready
                || !self.reception_dock_matches(completed.dock)
                || completed.dock.tick() > completed.tick
                || completed.tick > flight.tick
                || completed.dock.sequence() > nav.dock_sequence
                || completed.dock.visit() > nav.visit
                || completed.dock.visit() < completed.dock.sequence()
                || current_dock.is_some_and(|dock| {
                    dock.sequence() == completed.dock.sequence() && dock != completed.dock
                })
            {
                return Err(invalid);
            }
            return Ok(());
        }
        if (ready && pulse.remaining_cooldown() != 0)
            || (self.retired.is_none() && !pulse.fits_quiet_anchor(flight.tick, anchor))
        {
            return Err(invalid);
        }
        Ok(())
    }
}

impl Reception {
    pub(in crate::flight) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            anchor: r.option(|r| r.u64())?,
            completion: r.option(|r| {
                Ok(Completion {
                    dock: DockContext::decode_save(r)?,
                    tick: r.u64()?,
                })
            })?,
        })
    }
}
impl World {
    pub fn practice_reception_snapshot(&self) -> Option<PracticeReceptionSnapshot> {
        Some(self.practice?.reception_snapshot(self.state.tick))
    }
    pub fn review_practice_reception(
        &self,
        expected_tick: u64,
    ) -> Result<PracticeReceptionReview, PracticeError> {
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
        let practice = self.practice.ok_or(PracticeError::ReceptionNotReady)?;
        if practice.reception.completion.is_some() {
            return Err(PracticeError::AlreadyReceived);
        }
        if !practice.reception_dock_matches(dock)
            || practice.quiet_ticks(expected_tick) != PRACTICE_QUIET_TICKS
        {
            return Err(PracticeError::ReceptionNotReady);
        }
        Ok(PracticeReceptionReview {
            dock,
            tick: expected_tick,
            epoch: session.epoch(),
        })
    }
    pub fn prepare_practice_reception(
        &mut self,
        review: PracticeReceptionReview,
        expected_tick: u64,
    ) -> Result<PreparedPracticeConfirmation<'_>, PracticeError> {
        let current = self.review_practice_reception(expected_tick)?;
        if review.dock != current.dock
            || review.epoch != current.epoch
            || review.tick > current.tick
        {
            return Err(PracticeError::StaleReview);
        }
        let mut practice = self.practice.ok_or(PracticeError::ReceptionNotReady)?;
        practice.reception.completion = Some(Completion {
            dock: current.dock,
            tick: current.tick,
        });
        let bytes = self
            .save_bytes_with_practice(Some(practice))
            .map_err(PracticeError::Save)?;
        Ok(PreparedPracticeConfirmation {
            world: self,
            practice,
            bytes,
        })
    }
}

#[cfg(test)]
mod tests;

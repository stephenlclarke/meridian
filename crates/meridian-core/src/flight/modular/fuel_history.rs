//! Exact cargo and shared fitting chronology after the first explicit four-tank test fit.
use super::{
    CommanderAccount, DockContext, INVALID, Ledger, ModuleSlot, Reader, SaveError, State, World,
    Writer,
};
use crate::flight::{AccountEffect, AccountEvent, emp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CargoFit {
    dock: DockContext,
    tick: u64,
    account_revision: u64,
    equipment_revision: u64,
    fitted: bool,
}

impl State {
    pub(super) fn cargo_anchor_is_proven(&self, revision: u64, equipment_revision: u64) -> bool {
        equipment_revision <= 1
            || self.cargo_fit_at(revision).is_some_and(|receipt| {
                receipt.equipment_revision == equipment_revision
                    && receipt.fitted == (equipment_revision % 2 == 1)
            })
    }
    /// The independently retained test-tank and passenger cargo receipts must agree if
    /// they cover the same refit; the first MAX entry is only a baseline, not a refit.
    pub(super) fn validate_passenger_cargo_fit_overlap(
        &self,
        revision: u64,
        equipment_revision: u64,
        dock: DockContext,
    ) -> Result<(), SaveError> {
        if let Some(first) = self.cargo_fit_history.first()
            && revision > first.account_revision
        {
            let receipt = self
                .cargo_fit_history
                .iter()
                .find(|event| event.account_revision == revision)
                .ok_or(INVALID)?;
            if receipt.equipment_revision != equipment_revision
                || receipt.dock != dock
                || receipt.fitted != (equipment_revision % 2 == 1)
            {
                return Err(INVALID);
            }
        }
        Ok(())
    }
    pub(in crate::flight) fn has_test_cargo_history(&self) -> bool {
        !self.cargo_fit_history.is_empty()
    }

    /// At the first explicit MAX this records the known fitted cargo baseline, not a second
    /// purchase. Later entries correspond to real fit/store account transactions.
    pub(in crate::flight) fn record_test_cargo_fit(
        &mut self,
        account: &CommanderAccount,
        dock: DockContext,
        tick: u64,
    ) -> Result<(), SaveError> {
        if self.cargo_fit_history.len() >= crate::save::MAX_SAVE_BYTES / 64 {
            return Err(SaveError::TooLarge);
        }
        let equipment = account.equipment_state();
        self.cargo_fit_history.push(CargoFit {
            dock,
            tick,
            account_revision: account.revision(),
            equipment_revision: equipment.revision(),
            fitted: equipment.units_used() == 1,
        });
        Ok(())
    }

    pub(super) fn cargo_account_events(&self) -> impl Iterator<Item = AccountEvent> + '_ {
        self.cargo_fit_history
            .iter()
            .skip(1)
            .map(|event| AccountEvent {
                revision: event.account_revision,
                effect: AccountEffect::Debit(0),
            })
    }

    pub(super) fn encode_cargo_history(&self, w: &mut Writer) {
        w.u32(self.cargo_fit_history.len() as u32);
        for event in &self.cargo_fit_history {
            event.dock.encode_save(w);
            w.u64(event.tick);
            w.u64(event.account_revision);
            w.u64(event.equipment_revision);
            w.bool(event.fitted);
        }
    }
    pub(super) fn decode_cargo_history(r: &mut Reader<'_>) -> Result<Vec<CargoFit>, SaveError> {
        let count = r.u32()? as usize;
        if count == 0 || count > crate::save::MAX_SAVE_BYTES / 64 {
            return Err(INVALID);
        }
        let mut events = Vec::with_capacity(count);
        for _ in 0..count {
            events.push(CargoFit {
                dock: DockContext::decode_save(r)?,
                tick: r.u64()?,
                account_revision: r.u64()?,
                equipment_revision: r.u64()?,
                fitted: r.bool()?,
            });
        }
        Ok(events)
    }

    fn cargo_fit_at(&self, revision: u64) -> Option<CargoFit> {
        self.cargo_fit_history
            .iter()
            .take_while(|event| event.account_revision <= revision)
            .last()
            .copied()
    }

    pub(super) fn known_cargo_mounts_at(&self, account: &CommanderAccount, revision: u64) -> u16 {
        if let Some(event) = self.cargo_fit_at(revision) {
            return u16::from(event.fitted);
        }
        let equipment = account.equipment_state();
        // The legacy cargo codec retains acquisition but not intermediate ordinary fit/store
        // revisions before the first MAX. A sole acquisition proves the historical fit.
        u16::from(
            equipment.revision() == 1
                && equipment
                    .account_event()
                    .is_some_and(|event| event.revision <= revision),
        )
    }

    pub(in crate::flight) fn validate_test_custody_history(
        &self,
        world: &World,
        account: &CommanderAccount,
        emp: &emp::State,
        lesson: &crate::flight::navigation_trade::State,
    ) -> Result<(), SaveError> {
        let intervals = world.xyzzy.max_tank_intervals().collect::<Vec<_>>();
        let Some(&(_, first_end, first_dock, first_tick)) = intervals.first() else {
            if self.cargo_fit_history.is_empty() {
                return Ok(());
            }
            return Err(INVALID);
        };
        let first = self.cargo_fit_history.first().ok_or(INVALID)?;
        if (first.account_revision, first.dock, first.tick) != (first_end, first_dock, first_tick)
            || !first.fitted
            || first.equipment_revision == 0
        {
            return Err(INVALID);
        }
        let mut previous: Option<CargoFit> = None;
        for event in &self.cargo_fit_history {
            crate::flight::ammunition::validate_berth_event(
                world,
                account,
                event.dock,
                event.tick,
                event.account_revision,
                previous.map(|prior| (prior.dock.sequence(), prior.tick, prior.account_revision)),
            )?;
            if previous.is_some_and(|prior| {
                prior.equipment_revision.checked_add(1) != Some(event.equipment_revision)
                    || prior.fitted == event.fitted
            }) {
                return Err(INVALID);
            }
            previous = Some(*event);
        }
        let last = previous.ok_or(INVALID)?;
        if last.equipment_revision != account.equipment_state().revision()
            || last.fitted != (account.equipment_state().units_used() == 1)
            || intervals
                .iter()
                .any(|&(_, end, _, _)| self.cargo_fit_at(end).is_none_or(|event| !event.fitted))
        {
            return Err(INVALID);
        }
        let owner = world.combat_identity().ok_or(INVALID)?.owner;
        let freight = &world.freight_session.as_ref().ok_or(INVALID)?.state;
        if !freight.post_test_issues_have_markers(first_dock.sequence())
            || !lesson.post_test_acceptances_have_markers(first_dock.sequence(), first_tick)
        {
            return Err(INVALID);
        }
        let budget = crate::equipment::definition().map_err(|_| INVALID)?;
        for revision in freight
            .known_issue_revisions()
            .chain(lesson.known_acceptance_revisions())
        {
            if intervals
                .iter()
                .any(|&(start, end, _, _)| revision >= start && revision < end)
            {
                return Err(INVALID);
            }
            let Some(cargo) = self.cargo_fit_at(revision) else {
                continue;
            };
            let mut ledger = Ledger::born(owner)?;
            let mut counters = [0; 2];
            for event in self
                .events
                .iter()
                .take_while(|event| event.revision <= revision)
            {
                ledger.replay(*event, &mut counters).map_err(|_| INVALID)?;
            }
            let cargo = u16::from(cargo.fitted);
            let mounts = u16::from(ledger.fuel_storage_count())
                + u16::from(ledger.fitted(ModuleSlot::UtilityFuelScoops).is_some())
                + u16::from(ledger.fitted(ModuleSlot::UtilityEcm).is_some())
                + cargo
                + emp.utility_mounts_at_revision(revision);
            if mounts > budget.utility_mounts_capacity
                || ledger.fitted_units() + cargo > budget.equipment_units_capacity
            {
                return Err(INVALID);
            }
        }
        Ok(())
    }
}

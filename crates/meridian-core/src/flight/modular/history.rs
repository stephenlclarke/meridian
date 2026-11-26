//! Independent accepted berth and combined accommodation/cargo history.
use super::{
    CommanderAccount, INVALID, Ledger, ModuleAction, ModuleError, ModuleKind, State, World,
};
use crate::{
    navigation::DockContext,
    save::{Reader, SaveError, Writer},
    station_services::StationMarkets,
};

const MAX_BERTHS: usize = crate::save::MAX_SAVE_BYTES / 52;
const MAX_CHANGES: usize = crate::save::MAX_SAVE_BYTES / 68;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Anchor {
    revision: u64,
    bay_revision: u64,
    freight_sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StationChange {
    revision: u64,
    sequence: u64,
    dock: DockContext,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct History {
    anchor: Option<Anchor>,
    // First entry is the actual currently admitted berth at activation; all later ones are
    // appended solely through the accepted World physical-entry transition.
    berths: Vec<DockContext>,
    bay: Vec<StationChange>,
    freight: Vec<StationChange>,
}

impl History {
    pub(super) fn active(&self) -> bool {
        self.anchor.is_some()
    }

    pub(super) fn activate(
        &mut self,
        world: &World,
        account: &CommanderAccount,
        dock: DockContext,
    ) -> Result<(), ModuleError> {
        if self.active() {
            return Ok(());
        }
        let freight = &world
            .freight_session
            .as_ref()
            .ok_or(ModuleError::NoCommander)?
            .state;
        // Legacy issue and cancellation receipts contain no account revision. Only a completed
        // exact transfer proves all preceding old custody operations finished before this anchor.
        let events = freight.cargo_mass_events();
        if freight.snapshot().held_tonnes != 0
            || events.last().is_some_and(|event| event.revision.is_none())
            || account.equipment_state().revision() > 1
        {
            return Err(ModuleError::PassageHistory);
        }
        self.anchor = Some(Anchor {
            revision: account.revision(),
            bay_revision: account.equipment_state().revision(),
            freight_sequence: freight.cargo_sequence(),
        });
        self.berths.push(dock);
        Ok(())
    }

    pub(super) fn has_berth_room(&self) -> bool {
        !self.active() || self.berths.len() < MAX_BERTHS
    }

    pub(super) fn record_berth(&mut self, dock: DockContext) {
        if self.active() {
            self.berths.push(dock);
        }
    }

    pub(super) fn record_bay(&mut self, account: &CommanderAccount, dock: DockContext) {
        if self.active() {
            self.bay.push(StationChange {
                revision: account.revision(),
                sequence: account.equipment_state().revision(),
                dock,
            });
        }
    }

    pub(super) fn record_freight(&mut self, revision: u64, sequence: u64, dock: DockContext) {
        if self.active() {
            self.freight.push(StationChange {
                revision,
                sequence,
                dock,
            });
        }
    }

    pub(super) fn owns_berth(&self, dock: DockContext) -> bool {
        self.berths.contains(&dock)
    }

    #[cfg(test)]
    pub(super) fn remove_last_berth_for_test(&mut self) {
        self.berths.pop();
    }

    #[cfg(test)]
    pub(super) fn remove_last_bay_for_test(&mut self) {
        self.bay.pop();
    }

    #[cfg(test)]
    pub(super) fn remove_last_freight_for_test(&mut self) {
        self.freight.pop();
    }

    fn validate_berths(&self, world: &World, account: &CommanderAccount) -> Result<(), SaveError> {
        let first = *self.berths.first().ok_or(INVALID)?;
        let latest = *self.berths.last().ok_or(INVALID)?;
        let nav = world.navigation.ok_or(INVALID)?;
        if latest.sequence() != nav.dock_sequence()
            || world
                .current_dock_context()
                .is_some_and(|dock| dock != latest)
        {
            return Err(INVALID);
        }
        let mut previous = None;
        for dock in &self.berths {
            if previous.is_some_and(|last: DockContext| {
                last.sequence().checked_add(1) != Some(dock.sequence())
                    || last.visit() >= dock.visit()
                    || last.tick() >= dock.tick()
            }) {
                return Err(INVALID);
            }
            if dock.owner() != nav.owner()
                || dock.ship() != nav.ship()
                || dock.visit() > nav.snapshot(account.fuel_tenths()).visit
                || dock.sequence() > dock.visit()
                || dock.tick() > world.state.tick
                || match dock.origin() {
                    crate::navigation::DockOrigin::InitialStart => {
                        dock.sequence() != 1 || dock.visit() != 1 || dock.tick() != 0
                    }
                    crate::navigation::DockOrigin::PhysicalEntry => {
                        dock.sequence() <= 1 || dock.tick() == 0
                    }
                }
            {
                return Err(INVALID);
            }
            world
                .jump_journal
                .validate_passenger_berth(*dock, dock.tick())?;
            if world.damage.enabled()
                && world
                    .damage
                    .berth_clock(dock.sequence())
                    .is_none_or(|clock| clock > dock.tick())
            {
                return Err(INVALID);
            }
            previous = Some(*dock);
        }
        if first.sequence() == 0 {
            return Err(INVALID);
        }
        Ok(())
    }

    pub(super) fn encode(&self, w: &mut Writer) {
        if let Some(anchor) = self.anchor {
            w.u64(anchor.revision);
            w.u64(anchor.bay_revision);
            w.u64(anchor.freight_sequence);
            w.u16(self.berths.len() as u16);
            for dock in &self.berths {
                dock.encode_save(w);
            }
            for changes in [&self.bay, &self.freight] {
                w.u16(changes.len() as u16);
                for change in changes {
                    w.u64(change.revision);
                    w.u64(change.sequence);
                    change.dock.encode_save(w);
                }
            }
        }
    }

    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let anchor = Anchor {
            revision: r.u64()?,
            bay_revision: r.u64()?,
            freight_sequence: r.u64()?,
        };
        let count = usize::from(r.u16()?);
        if count == 0 || count > MAX_BERTHS {
            return Err(SaveError::InvalidEncoding);
        }
        let berths = (0..count)
            .map(|_| DockContext::decode_save(r))
            .collect::<Result<Vec<_>, _>>()?;
        fn changes(r: &mut Reader<'_>) -> Result<Vec<StationChange>, SaveError> {
            let count = usize::from(r.u16()?);
            if count > MAX_CHANGES {
                return Err(SaveError::TooLarge);
            }
            (0..count)
                .map(|_| {
                    Ok(StationChange {
                        revision: r.u64()?,
                        sequence: r.u64()?,
                        dock: DockContext::decode_save(r)?,
                    })
                })
                .collect()
        }
        Ok(Self {
            anchor: Some(anchor),
            berths,
            bay: changes(r)?,
            freight: changes(r)?,
        })
    }
}

#[derive(Clone, Copy)]
enum Change {
    Cargo(i16),
    Bay(u64),
    Fit(u16, u16),
}

impl State {
    pub(in crate::flight) fn extra_capacity_account_events(
        &self,
        freight: &crate::freight::g02::State,
    ) -> Vec<crate::flight::AccountEvent> {
        let Some(anchor) = self.history.anchor else {
            return Vec::new();
        };
        let mut revisions = Vec::new();
        revisions.extend(
            self.history
                .bay
                .iter()
                .filter(|record| record.sequence > 1)
                .map(|record| record.revision),
        );
        let completed = freight.account_events();
        revisions.extend(
            self.history
                .freight
                .iter()
                .filter(|record| {
                    !completed
                        .iter()
                        .any(|event| event.revision == record.revision)
                })
                .map(|record| record.revision),
        );
        revisions.extend(
            self.events
                .iter()
                .filter(|event| {
                    event.revision > anchor.revision
                        && matches!(
                            event.action,
                            ModuleAction::Store { .. } | ModuleAction::Install { .. }
                        )
                })
                .map(|event| event.revision),
        );
        revisions
            .into_iter()
            .map(|revision| crate::flight::AccountEvent {
                revision,
                effect: crate::flight::AccountEffect::Debit(0),
            })
            .collect()
    }
    pub(in crate::flight) fn history_active(&self) -> bool {
        self.history.active()
    }
    pub(in crate::flight) fn has_berth_room(&self) -> bool {
        self.history.has_berth_room()
    }
    pub(in crate::flight) fn record_berth(&mut self, dock: DockContext) {
        self.history.record_berth(dock);
    }
    pub(in crate::flight) fn record_bay(&mut self, account: &CommanderAccount, dock: DockContext) {
        self.history.record_bay(account, dock);
    }
    pub(in crate::flight) fn record_freight(
        &mut self,
        revision: u64,
        sequence: u64,
        dock: DockContext,
    ) {
        self.history.record_freight(revision, sequence, dock);
    }
    pub(in crate::flight) fn validate_capacity_history(
        &self,
        world: &World,
        account: &CommanderAccount,
        markets: &StationMarkets,
    ) -> Result<(), SaveError> {
        let first = self
            .events
            .iter()
            .find(|event| requires_history(event.action));
        let Some(anchor) = self.history.anchor else {
            return if first.is_none() {
                Ok(())
            } else {
                Err(INVALID)
            };
        };
        let first = first.ok_or(INVALID)?;
        if first.revision.checked_sub(1) != Some(anchor.revision)
            || self.history.berths.first() != Some(&first.dock)
        {
            return Err(INVALID);
        }
        self.history.validate_berths(world, account)?;
        let bay = account.equipment_state();
        let freight = &world.freight_session.as_ref().ok_or(INVALID)?.state;
        let mut changes = Vec::new();
        let mut cargo = 0_i16;
        for (revision, delta) in markets.cargo_mass_events() {
            if revision <= anchor.revision {
                cargo = cargo.checked_add(delta).ok_or(INVALID)?;
            } else {
                changes.push((revision, Change::Cargo(delta)));
            }
        }
        let purchase = bay.account_event().map(|event| event.revision);
        if anchor.bay_revision > 1
            || (anchor.bay_revision > 0)
                != purchase.is_some_and(|revision| revision <= anchor.revision)
            || anchor.bay_revision > bay.revision()
        {
            return Err(INVALID);
        }
        self.history.validate_sequence(
            anchor.revision,
            anchor.bay_revision,
            bay.revision(),
            &self.history.bay,
            account.revision(),
        )?;
        for change in &self.history.bay {
            if change.sequence == 1
                && (purchase != Some(change.revision)
                    || super::super::services::service_site(change.dock.system())
                        != bay.owned().map(|item| item.purchase_site))
            {
                return Err(INVALID);
            }
            changes.push((change.revision, Change::Bay(change.sequence)));
        }
        self.history.validate_sequence(
            anchor.revision,
            anchor.freight_sequence,
            freight.cargo_sequence(),
            &self.history.freight,
            account.revision(),
        )?;
        let mut old_freight = 0_i16;
        let mut last_exact = anchor.freight_sequence == 0;
        for event in freight.cargo_mass_events() {
            let (sequence, receipt, delta) = (event.sequence, event.revision, event.delta);
            if sequence <= anchor.freight_sequence {
                if receipt.is_some_and(|revision| revision > anchor.revision) {
                    return Err(INVALID);
                }
                old_freight = old_freight.checked_add(delta).ok_or(INVALID)?;
                last_exact = receipt.is_some();
            } else {
                let record = self
                    .history
                    .freight
                    .iter()
                    .find(|record| record.sequence == sequence)
                    .ok_or(INVALID)?;
                let revision = record.revision;
                if receipt.is_some_and(|recorded| recorded != revision)
                    || record.dock.system().galaxy != event.system.galaxy
                    || record.dock.system().system != event.system.system
                    || event
                        .dock_sequence
                        .is_some_and(|sequence| sequence != record.dock.sequence())
                {
                    return Err(INVALID);
                }
                changes.push((revision, Change::Cargo(delta)));
            }
        }
        if old_freight != 0 || !last_exact {
            return Err(INVALID);
        }
        let mut ledger = Ledger::born(first.dock.owner().seed())?;
        let mut counters = [0; 2];
        let mut fitted_units = 0;
        let mut cabin_mass = 0;
        for event in &self.events {
            ledger.replay(*event, &mut counters).map_err(|_| INVALID)?;
            if event.revision <= anchor.revision {
                fitted_units = ledger.fitted_units();
                cabin_mass = ledger.cabin_tonnes();
            } else {
                if !self.history.owns_berth(event.dock) {
                    return Err(INVALID);
                }
                changes.push((
                    event.revision,
                    Change::Fit(ledger.fitted_units(), ledger.cabin_tonnes()),
                ));
            }
        }
        changes.sort_unstable_by_key(|event| event.0);
        let mut bay_sequence = anchor.bay_revision;
        let budget = crate::equipment::definition()
            .map_err(|_| INVALID)?
            .equipment_units_capacity;
        let mut previous = anchor.revision;
        check_fit(cargo, cabin_mass, fitted_units, bay_sequence, budget)?;
        for (revision, change) in changes {
            if revision <= previous || revision > account.revision() {
                return Err(INVALID);
            }
            previous = revision;
            match change {
                Change::Cargo(delta) => cargo = cargo.checked_add(delta).ok_or(INVALID)?,
                Change::Bay(sequence) => bay_sequence = sequence,
                Change::Fit(units, mass) => {
                    fitted_units = units;
                    cabin_mass = mass;
                }
            }
            check_fit(cargo, cabin_mass, fitted_units, bay_sequence, budget)?;
        }
        let projection = account.service_projection(freight).map_err(|_| INVALID)?;
        if u16::try_from(cargo).ok() != Some(projection.used_tonnes) {
            return Err(INVALID);
        }
        Ok(())
    }
}

impl History {
    fn validate_sequence(
        &self,
        anchor_revision: u64,
        anchor: u64,
        final_value: u64,
        records: &[StationChange],
        account_revision: u64,
    ) -> Result<(), SaveError> {
        let mut previous_revision = anchor_revision;
        let mut previous_sequence = anchor;
        let mut previous_dock = self.berths.first().ok_or(INVALID)?.sequence();
        if records.len() > MAX_CHANGES {
            return Err(INVALID);
        }
        for change in records {
            if previous_sequence.checked_add(1) != Some(change.sequence)
                || change.revision <= previous_revision
                || change.revision > account_revision
                || change.dock.sequence() < previous_dock
                || !self.owns_berth(change.dock)
            {
                return Err(INVALID);
            }
            previous_sequence = change.sequence;
            previous_revision = change.revision;
            previous_dock = change.dock.sequence();
        }
        if previous_sequence != final_value {
            return Err(INVALID);
        }
        Ok(())
    }
}

fn check_fit(
    cargo: i16,
    cabin_mass: u16,
    units: u16,
    bay_sequence: u64,
    budget: u16,
) -> Result<(), SaveError> {
    let bay = bay_sequence % 2 == 1;
    let capacity = if bay {
        crate::equipment::EXPANDED_HOLD_TONNES
    } else {
        crate::equipment::STANDARD_HOLD_TONNES
    };
    if cargo < 0
        || u32::from(cabin_mass) + u32::from(cargo as u16) > u32::from(capacity)
        || units + u16::from(bay) > budget
    {
        return Err(INVALID);
    }
    Ok(())
}

pub(super) fn requires_history(action: ModuleAction) -> bool {
    matches!(
        action,
        ModuleAction::Purchase {
            kind: ModuleKind::Cabin(_)
        } | ModuleAction::Passenger(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_prefix_must_fit_even_when_a_later_action_releases_capacity() {
        assert_eq!(check_fit(0, 90, 18, 0, 24), Ok(()));
        assert_eq!(check_fit(0, 95, 19, 0, 24), Err(INVALID));
        assert_eq!(check_fit(0, 125, 25, 1, 24), Err(INVALID));
        assert_eq!(check_fit(2, 120, 24, 1, 24), Err(INVALID));
        assert_eq!(check_fit(2, 115, 23, 1, 24), Ok(()));
    }
}

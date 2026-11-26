//! Replay independently validated transaction receipts from the ordinary birth account.
use super::{INVALID, STARTING_CREDITS_TENTHS, State, World};
use crate::flight::persistence::Commerce;
use crate::save::SaveError;
#[derive(Debug, Clone, Copy)]
pub(crate) struct AccountEvent {
    pub(crate) revision: u64,
    pub(crate) effect: AccountEffect,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum AccountEffect {
    Debit(u32),
    Reward(u32),
    Sale { bid: u32, credited: u32 },
    Refill { units: u16, paid: u32 },
    TankFit { count: u8, paid: u32 },
    Solar { before: u16, units: u16 },
    Jump(u16),
    Secret(usize),
    SecretRefill(usize),
}
impl State {
    fn apply_secret_prefix(
        &self,
        index: usize,
        world: &World,
        cash: &mut u32,
        fuel: &mut u16,
        capacity: u16,
        spent: u64,
    ) -> Result<bool, SaveError> {
        let r = self.records.get(index).ok_or(INVALID)?;
        let expected_spent = world.jump_journal.spent_at(r.dock, r.tick)?;
        let deferred = r.action == super::XyzzyAction::MaximumFuelStorage;
        let refill = r.action.refills(r.unlimited_fuel) && !deferred;
        let expected_added = if refill {
            capacity.checked_sub(*fuel).ok_or(INVALID)?
        } else {
            0
        };
        if r.fuel_spent != spent
            || r.fuel_spent != expected_spent
            || (!deferred && r.fuel_added != expected_added)
            || (index == 0 && r.cash_before_reset != Some(*cash))
        {
            return Err(INVALID);
        }
        if index == 0 {
            *cash = STARTING_CREDITS_TENTHS;
        }
        if refill {
            *fuel = capacity;
        }
        Ok(r.unlimited_fuel)
    }
    pub(super) fn validate_account_prefix(
        &self,
        world: &World,
        account: &crate::freight::g02::CommanderAccount,
        commerce: &Commerce<'_>,
    ) -> Result<(), SaveError> {
        let mut events = commerce.markets.ok_or(INVALID)?.account_events();
        events.extend(
            commerce
                .modular
                .account_events(world.combat_identity().ok_or(INVALID)?.owner)?,
        );
        events.extend(commerce.emp.account_events()?);
        events.extend(commerce.ammunition.account_events());
        events.extend(account.equipment_state().account_event());
        events.extend(
            world
                .freight_session
                .as_ref()
                .ok_or(INVALID)?
                .state
                .account_events(),
        );
        events.extend(
            self.records
                .iter()
                .enumerate()
                .filter(|(_, record)| record.action == super::XyzzyAction::MaximumFuelStorage)
                .map(|(index, record)| AccountEvent {
                    revision: record.end_revision,
                    effect: AccountEffect::SecretRefill(index),
                }),
        );
        events.extend(world.jump_journal.account_events());
        events.extend(world.solar.receipts().iter().map(|receipt| AccountEvent {
            revision: receipt.revision,
            effect: AccountEffect::Solar {
                before: receipt.before,
                units: receipt.added,
            },
        }));
        events.extend(
            self.records
                .iter()
                .enumerate()
                .map(|(index, r)| AccountEvent {
                    revision: r.revision,
                    effect: AccountEffect::Secret(index),
                }),
        );
        events.sort_unstable_by_key(|event| event.revision);
        let mut cash = 7200_u32;
        let mut fuel = crate::navigation::FUEL_CAPACITY;
        let mut capacity = crate::navigation::FUEL_CAPACITY;
        let mut spent = 0_u64;
        let mut unlimited = false;
        let mut previous = 0;
        let mut solar_anchors = world.solar.opening_fuel_anchors().peekable();
        for event in events {
            if event.revision <= previous || event.revision > account.revision() {
                return Err(INVALID);
            }
            while let Some((revision, opening)) = solar_anchors.peek().copied() {
                if revision >= event.revision {
                    break;
                }
                if revision < previous || opening != fuel {
                    return Err(INVALID);
                }
                let _ = solar_anchors.next();
            }
            match event.effect {
                AccountEffect::Debit(paid) => cash = cash.checked_sub(paid).ok_or(INVALID)?,
                AccountEffect::Reward(credit) => cash = cash.checked_add(credit).ok_or(INVALID)?,
                AccountEffect::Sale { bid, credited } => {
                    let after = cash.saturating_add(bid);
                    if after - cash != credited {
                        return Err(INVALID);
                    }
                    cash = after;
                }
                AccountEffect::Refill { units, paid } => {
                    if units != capacity.checked_sub(fuel).ok_or(INVALID)?
                        || paid != u32::from(units) * crate::station_services::FUEL_PRICE_TENTHS
                    {
                        return Err(INVALID);
                    }
                    cash = cash.checked_sub(paid).ok_or(INVALID)?;
                    fuel = capacity;
                }
                AccountEffect::TankFit { count, paid } => {
                    capacity = crate::navigation::fuel_capacity(count);
                    if fuel > capacity {
                        return Err(INVALID);
                    }
                    cash = cash.checked_sub(paid).ok_or(INVALID)?;
                }
                AccountEffect::Solar { before, units } => {
                    if fuel != before || units == 0 {
                        return Err(INVALID);
                    }
                    fuel = fuel
                        .checked_add(units)
                        .filter(|&value| value <= capacity)
                        .ok_or(INVALID)?;
                }
                AccountEffect::Jump(debit) => {
                    if fuel < debit {
                        return Err(INVALID);
                    }
                    spent = spent.checked_add(u64::from(debit)).ok_or(INVALID)?;
                    if !unlimited {
                        fuel -= debit;
                    }
                }
                AccountEffect::Secret(index) => {
                    unlimited = self
                        .apply_secret_prefix(index, world, &mut cash, &mut fuel, capacity, spent)?;
                }
                AccountEffect::SecretRefill(index) => {
                    let record = self.records.get(index).ok_or(INVALID)?;
                    if capacity != crate::navigation::MAX_FUEL_CAPACITY
                        || record.fuel_added != capacity.checked_sub(fuel).ok_or(INVALID)?
                    {
                        return Err(INVALID);
                    }
                    fuel = capacity;
                }
            }
            previous = event.revision;
        }
        for (revision, opening) in solar_anchors {
            if revision < previous || revision > account.revision() || opening != fuel {
                return Err(INVALID);
            }
        }
        if cash != account.credits_tenths()
            || fuel != account.fuel_tenths()
            || capacity != account.fuel_capacity()
            || spent != world.navigation.ok_or(INVALID)?.fuel_spent()
        {
            return Err(INVALID);
        }
        Ok(())
    }
}

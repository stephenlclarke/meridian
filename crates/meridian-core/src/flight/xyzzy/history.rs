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
    ObserveFuel(u16),
    Sale { bid: u32, credited: u32 },
    Refill { units: u16, paid: u32 },
    Jump(u16),
    Secret(usize),
}
impl State {
    fn apply_secret_prefix(
        &self,
        index: usize,
        world: &World,
        cash: &mut u32,
        fuel: &mut u16,
        spent: u64,
    ) -> Result<bool, SaveError> {
        let r = self.records.get(index).ok_or(INVALID)?;
        let expected_spent = world.jump_journal.spent_at(r.dock, r.tick)?;
        let refill = r.action.refills(r.unlimited_fuel);
        let expected_added = if refill {
            crate::navigation::FUEL_CAPACITY
                .checked_sub(*fuel)
                .ok_or(INVALID)?
        } else {
            0
        };
        if r.fuel_spent != spent
            || r.fuel_spent != expected_spent
            || r.fuel_added != expected_added
            || (index == 0 && r.cash_before_reset != Some(*cash))
        {
            return Err(INVALID);
        }
        if index == 0 {
            *cash = STARTING_CREDITS_TENTHS;
        }
        if refill {
            *fuel = crate::navigation::FUEL_CAPACITY;
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
        events.extend(commerce.modular.account_events());
        events.extend(
            commerce.modular.extra_capacity_account_events(
                &world.freight_session.as_ref().ok_or(INVALID)?.state,
            ),
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
        events.extend(world.jump_journal.account_events());
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
        let mut spent = 0_u64;
        let mut unlimited = false;
        let mut previous = 0;
        for event in events {
            if event.revision <= previous || event.revision > account.revision() {
                return Err(INVALID);
            }
            match event.effect {
                AccountEffect::Debit(paid) => cash = cash.checked_sub(paid).ok_or(INVALID)?,
                AccountEffect::Reward(credit) => cash = cash.checked_add(credit).ok_or(INVALID)?,
                AccountEffect::ObserveFuel(observed) => {
                    if observed != fuel {
                        return Err(INVALID);
                    }
                }
                AccountEffect::Sale { bid, credited } => {
                    let after = cash.saturating_add(bid);
                    if after - cash != credited {
                        return Err(INVALID);
                    }
                    cash = after;
                }
                AccountEffect::Refill { units, paid } => {
                    if units
                        != crate::navigation::FUEL_CAPACITY
                            .checked_sub(fuel)
                            .ok_or(INVALID)?
                        || paid != u32::from(units) * crate::station_services::FUEL_PRICE_TENTHS
                    {
                        return Err(INVALID);
                    }
                    cash = cash.checked_sub(paid).ok_or(INVALID)?;
                    fuel = crate::navigation::FUEL_CAPACITY;
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
                    unlimited =
                        self.apply_secret_prefix(index, world, &mut cash, &mut fuel, spent)?;
                }
            }
            previous = event.revision;
        }
        if cash != account.credits_tenths()
            || fuel != account.fuel_tenths()
            || spent != world.navigation.ok_or(INVALID)?.fuel_spent()
        {
            return Err(INVALID);
        }
        Ok(())
    }
}

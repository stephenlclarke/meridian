mod quantities;
pub use quantities::PreparedCommodityTrade;

use super::{
    Balance, Fuel, Journey, MAX_RECEIPTS, NavigationTonneId, NavigationTradeError as Error,
    PreparedStationService, State, Trade, World, quantity, site,
};
use crate::freight::g02::CommanderAccount;
use crate::navigation::NavigationState;
use crate::station_services::{self as services, Command, Event, Reservations, StationMarkets};
use crate::{MissionError, StepError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Purpose {
    Ordinary,
    Buy,
    Sell,
}
impl World {
    fn navigation_market_admission(
        &self,
        command: Command,
    ) -> Result<(Reservations, Purpose), Error> {
        let reservations = self.navigation_trade.reservations();
        let Some(a) = self.navigation_trade.active() else {
            return Ok((reservations, Purpose::Ordinary));
        };
        let Some(plan) = a.plan else {
            return Ok((reservations, Purpose::Ordinary));
        };
        let id = match command {
            Command::BuyFood | Command::SellFood => services::CommodityId::Food,
            Command::BuyCommodity(id) | Command::SellCommodity(id) => id,
            Command::RefillFuel => return Ok((reservations, Purpose::Ordinary)),
        };
        if id != plan.commodity {
            return Ok((reservations, Purpose::Ordinary));
        }
        let buying = matches!(command, Command::BuyFood | Command::BuyCommodity(_));
        if buying && a.buy.is_none() {
            let dock = self.current_dock_context().ok_or(Error::BerthRequired)?;
            let p = self.raw_service_projection().map_err(Error::Service)?;
            let quote = self.commodity_quote(id).ok_or(Error::BerthRequired)?;
            if !super::same_carrier(a.dock, dock)
                || dock.system() != a.dock.system()
                || self.navigation_trade_fit() != Some(plan.fit)
                || p.hold_capacity_tonnes != plan.capacity
                || p.used_tonnes != plan.cargo
                || quantity(p, id) != 0
                || quote.ask_tenths != plan.price
                || quote.stock != plan.stock
            {
                return Err(Error::FitOrStockChanged);
            }
            return Ok((Reservations::default(), Purpose::Buy));
        }
        if !buying && a.held() {
            let dock = self.current_dock_context().ok_or(Error::BerthRequired)?;
            let qualified = a.selected_journey().is_some()
                && dock.system() == plan.destination
                && a.journeys.last().is_some_and(|j| {
                    j.destination == plan.destination && j.arrival_visit == dock.visit()
                });
            if qualified {
                let quote = self.commodity_quote(id).ok_or(Error::BerthRequired)?;
                if self
                    .credits_tenths()
                    .and_then(|c| c.checked_add(quote.bid_tenths))
                    .is_none()
                {
                    return Err(Error::CreditOverflow);
                }
                return Ok((Reservations::default(), Purpose::Sell));
            }
        }
        Ok((reservations, Purpose::Ordinary))
    }
    pub(in crate::flight) fn raw_service_projection(
        &self,
    ) -> Result<services::AccountProjection, services::Error> {
        self.commander_account
            .as_ref()
            .ok_or(services::Error::InvalidAccount)?
            .service_projection(
                &self
                    .freight_session
                    .as_ref()
                    .ok_or(services::Error::InvalidAccount)?
                    .state,
            )
    }
    pub(in crate::flight) fn reserved_service_projection(
        &self,
    ) -> Result<services::AccountProjection, services::Error> {
        self.navigation_trade
            .reservations()
            .apply(self.raw_service_projection()?)
    }
    pub fn prepare_station_service(
        &mut self,
        command: Command,
        tick: u64,
    ) -> Result<PreparedStationService<'_>, MissionError> {
        let facts = self.current_service_facts(tick)?;
        let (reservations, purpose) = self
            .navigation_market_admission(command)
            .map_err(MissionError::NavigationTrade)?;
        let account = self
            .commander_account
            .as_ref()
            .ok_or(MissionError::NoCommander)?;
        let session = self
            .freight_session
            .as_ref()
            .ok_or(MissionError::NoCommander)?;
        let markets = self
            .station_markets
            .as_ref()
            .ok_or(MissionError::NoCommander)?;
        let (account, markets, event) = account
            .prepare_station_service(&session.state, markets, facts, command, reservations)
            .map_err(MissionError::Service)?;
        let lesson = self
            .navigation_trade
            .after_service(self, &account, &markets, purpose, event)
            .map_err(MissionError::NavigationTrade)?;
        let bytes = self
            .save_bytes_with_navigation_trade(&lesson, &account, &markets)
            .map_err(|e| MissionError::NavigationTrade(Error::Save(e)))?;
        Ok(PreparedStationService {
            world: self,
            account,
            markets,
            lesson,
            event,
            bytes,
        })
    }
    pub(in crate::flight) fn prepare_navigation_trade_step(
        &self,
        next: Option<NavigationState>,
        debit: Option<u16>,
        tick: u64,
    ) -> Result<Option<State>, StepError> {
        let Some(debit) = debit else {
            return Ok(None);
        };
        let Some(active) = self.navigation_trade.active().filter(|a| a.buy.is_some()) else {
            return Ok(None);
        };
        if active.journeys.len() >= MAX_RECEIPTS {
            return Err(StepError::TickOverflow);
        }
        let mut candidate = self.navigation_trade.clone();
        let a = candidate
            .attempts
            .last_mut()
            .ok_or(StepError::NonFiniteState)?;
        let before = self.navigation.ok_or(StepError::NonFiniteState)?;
        let after = next.ok_or(StepError::NonFiniteState)?;
        let fuel = self
            .commander_account
            .as_ref()
            .ok_or(StepError::NonFiniteState)?
            .fuel_tenths();
        let account_revision = self
            .raw_service_projection()
            .map_err(|_| StepError::NonFiniteState)?
            .revision
            .checked_add(1)
            .ok_or(StepError::TickOverflow)?;
        a.journeys.push(Journey {
            tick,
            account_revision,
            jump: after.jump_sequence(),
            origin: before.snapshot(fuel).system,
            destination: after.snapshot(fuel).system,
            source_visit: before.snapshot(fuel).visit,
            arrival_visit: after.snapshot(fuel).visit,
            fuel_before: fuel,
            fuel_after: fuel.checked_sub(debit).ok_or(StepError::NonFiniteState)?,
            debit,
        });
        Ok(Some(candidate))
    }
}
impl State {
    fn after_service(
        &self,
        world: &World,
        account: &CommanderAccount,
        markets: &StationMarkets,
        purpose: Purpose,
        event: Event,
    ) -> Result<Self, Error> {
        let mut candidate = self.clone();
        let ordinal = candidate.attempts.len().saturating_sub(1);
        let Some(a) = candidate
            .attempts
            .last_mut()
            .filter(|a| a.terminal.is_none() && a.plan.is_some())
        else {
            return Ok(candidate);
        };
        let dock = world.current_dock_context().ok_or(Error::BerthRequired)?;
        let plan = a.plan.ok_or(Error::PlanRequired)?;
        let before = world.raw_service_projection().map_err(Error::Service)?;
        let after = account
            .service_projection(
                &world
                    .freight_session
                    .as_ref()
                    .ok_or(Error::NoCommander)?
                    .state,
            )
            .map_err(Error::Service)?;
        let transaction = markets.transaction_revision();
        if purpose != Purpose::Ordinary {
            let before_markets = world.station_markets.as_ref().ok_or(Error::NoCommander)?;
            let quote = before_markets.commodity_quote(
                site(dock.system()).ok_or(Error::BerthRequired)?,
                plan.commodity,
            );
            let tonne = if purpose == Purpose::Buy {
                NavigationTonneId {
                    owner: dock.owner().seed(),
                    attempt: ordinal as u8,
                    transaction,
                }
            } else {
                a.buy.ok_or(Error::NotReady)?.tonne
            };
            let trade = Trade {
                tonne,
                dock,
                tick: world.state.tick,
                transaction,
                account_revision: after.revision,
                jump_sequence: world.navigation.ok_or(Error::NoCommander)?.jump_sequence(),
                fuel_at_trade: before.fuel_tenths,
                price: if purpose == Purpose::Buy {
                    quote.ask_tenths
                } else {
                    quote.bid_tenths
                },
                before: balance(before_markets, before, dock, plan.commodity)?,
                after: balance(markets, after, dock, plan.commodity)?,
            };
            if purpose == Purpose::Buy {
                a.buy = Some(trade);
            } else {
                a.sale = Some(trade);
            }
        }
        if let Event::Refuelled {
            added_fuel_tenths,
            paid_tenths,
            ..
        } = event
            && added_fuel_tenths > 0
        {
            if a.fuel.len() >= MAX_RECEIPTS {
                return Err(Error::ReceiptCapacity);
            }
            a.fuel.push(Fuel {
                tick: world.state.tick,
                dock,
                transaction,
                account_revision: after.revision,
                ordinal: markets.fuel_accounting().1,
                units_before: before.fuel_tenths,
                units_after: after.fuel_tenths,
                credits_before: before.credits_tenths,
                credits_after: after.credits_tenths,
                paid: paid_tenths,
            });
        }
        Ok(candidate)
    }
}
fn balance(
    markets: &StationMarkets,
    p: services::AccountProjection,
    dock: crate::navigation::DockContext,
    id: services::CommodityId,
) -> Result<Balance, Error> {
    let site = site(dock.system()).ok_or(Error::BerthRequired)?;
    let (bought, sold) = markets.commodity_counts(site, id);
    Ok(Balance {
        stock: markets.commodity_quote(site, id).stock,
        cargo: quantity(p, id),
        credits: p.credits_tenths,
        bought,
        sold,
    })
}

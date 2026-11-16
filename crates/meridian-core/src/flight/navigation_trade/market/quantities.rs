//! Quantity trading shares World's actual berth, account, market and T04 admission.
use super::{Purpose, World};
use crate::freight::g02::CommanderAccount;
use crate::station_services::{CommodityTrade, CommodityTradeEvent, Error, StationMarkets};
use crate::{MissionError, NavigationTradeError};

/// Complete prospective account/market/lesson bytes under one exclusive World borrow.
/// Dropping or failed publication leaves the live World unchanged.
pub struct PreparedCommodityTrade<'a> {
    world: &'a mut World,
    account: CommanderAccount,
    markets: StationMarkets,
    lesson: super::State,
    event: CommodityTradeEvent,
    bytes: Vec<u8>,
}
impl PreparedCommodityTrade<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn event(&self) -> CommodityTradeEvent {
        self.event
    }
    pub fn publish_with<E>(
        self,
        writer: impl FnOnce(&[u8]) -> Result<(), E>,
    ) -> Result<CommodityTradeEvent, E> {
        writer(&self.bytes)?;
        Ok(self.commit())
    }
    pub fn commit(self) -> CommodityTradeEvent {
        self.world.commander_account = Some(self.account);
        self.world.station_markets = Some(self.markets);
        self.world.navigation_trade = self.lesson;
        self.event
    }
}
impl World {
    /// Immediate core transaction. Hosts needing durability use prepare_commodity_trade and
    /// publish complete bytes successfully before committing, as for ordinary single services.
    pub fn request_commodity_trade(
        &mut self,
        trade: CommodityTrade,
        tick: u64,
    ) -> Result<CommodityTradeEvent, MissionError> {
        self.prepare_commodity_trade(trade, tick)
            .map(|prepared| prepared.commit())
    }
    pub fn prepare_commodity_trade(
        &mut self,
        trade: CommodityTrade,
        tick: u64,
    ) -> Result<PreparedCommodityTrade<'_>, MissionError> {
        let facts = self.current_service_facts(tick)?;
        if trade.quantity == 0 {
            return Err(MissionError::Service(Error::ZeroQuantity));
        }
        let (reservations, purpose) = self
            .navigation_market_admission(trade.single_command())
            .map_err(MissionError::NavigationTrade)?;
        // The lesson identifies exactly one original tonne and its one-unit buy/sale receipt.
        // Never disguise a bulk action as that receipt or release its reservation for a batch.
        if trade.quantity != 1 && purpose != Purpose::Ordinary {
            return Err(MissionError::NavigationTrade(
                NavigationTradeError::SingleUnitRequired,
            ));
        }
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
            .prepare_commodity_trade(&session.state, markets, facts, trade, reservations)
            .map_err(MissionError::Service)?;
        let lesson = if trade.quantity == 1 {
            self.navigation_trade
                .after_service(
                    self,
                    &account,
                    &markets,
                    purpose,
                    event.single_event().map_err(MissionError::Service)?,
                )
                .map_err(MissionError::NavigationTrade)?
        } else {
            self.navigation_trade.clone()
        };
        let bytes = self
            .save_bytes_with_navigation_trade(&lesson, &account, &markets)
            .map_err(|error| MissionError::NavigationTrade(NavigationTradeError::Save(error)))?;
        Ok(PreparedCommodityTrade {
            world: self,
            account,
            markets,
            lesson,
            event,
            bytes,
        })
    }
}

#[cfg(test)]
mod tests;

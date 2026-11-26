//! Quantity trading shares the actual berth, account, market, cabin and T04 admission.
use super::{Purpose, World};
use crate::freight::g02::CommanderAccount;
use crate::station_services::{CommodityTrade, CommodityTradeEvent, Error, StationMarkets};
use crate::{MissionError, NavigationTradeError};

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
    pub fn commit(self) -> CommodityTradeEvent {
        self.world.commander_account = Some(self.account);
        self.world.station_markets = Some(self.markets);
        self.world.navigation_trade = self.lesson;
        self.event
    }
    pub fn publish_with<E>(
        self,
        writer: impl FnOnce(&[u8]) -> Result<(), E>,
    ) -> Result<CommodityTradeEvent, E> {
        writer(&self.bytes)?;
        Ok(self.commit())
    }
}

impl World {
    pub fn request_commodity_trade(
        &mut self,
        trade: CommodityTrade,
        tick: u64,
    ) -> Result<CommodityTradeEvent, MissionError> {
        self.prepare_commodity_trade(trade, tick)
            .map(PreparedCommodityTrade::commit)
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::station_services::{CommodityId, MAX_TRADE_QUANTITY, TradeSide};

    fn order(side: TradeSide, commodity: CommodityId, quantity: u64) -> CommodityTrade {
        CommodityTrade {
            side,
            commodity,
            quantity,
        }
    }

    #[test]
    fn bulk_trade_exact_per_unit_history_matches_singles_and_failed_writer_is_atomic() {
        let mut bulk = World::new_commander(61101).unwrap();
        let mut singles = World::new_commander(61101).unwrap();
        let before = bulk.save_bytes().unwrap();
        let tick = bulk.snapshot().tick;
        let purchase = order(TradeSide::Buy, CommodityId::Food, 11);
        let failed = bulk
            .prepare_commodity_trade(purchase, tick)
            .unwrap()
            .publish_with(|_| Err("disk"));
        assert_eq!(failed, Err("disk"));
        assert_eq!(bulk.save_bytes().unwrap(), before);
        let event = bulk.request_commodity_trade(purchase, tick).unwrap();
        assert_eq!(event.quantity, 11);
        assert_eq!(event.gross_tenths, u64::from(event.unit_price_tenths) * 11);
        assert_eq!(event.paid_tenths, event.gross_tenths as u32);
        for _ in 0..11 {
            singles
                .request_station_service(purchase.single_command(), tick)
                .unwrap();
        }
        assert_eq!(bulk.save_bytes().unwrap(), singles.save_bytes().unwrap());
        assert_eq!(bulk.commodity_quantity(CommodityId::Food), Some(11));
        let sale = order(TradeSide::Sell, CommodityId::Food, 5);
        bulk.request_commodity_trade(sale, tick).unwrap();
        for _ in 0..5 {
            singles
                .request_station_service(sale.single_command(), tick)
                .unwrap();
        }
        let saved = bulk.save_bytes().unwrap();
        assert_eq!(saved, singles.save_bytes().unwrap());
        assert_eq!(
            World::from_save_bytes(&saved)
                .unwrap()
                .save_bytes()
                .unwrap(),
            saved
        );
    }

    #[test]
    fn bulk_trade_native_units_and_over_limit_requests_leave_world_intact() {
        let mut world = World::new_commander(61102).unwrap();
        let tick = world.snapshot().tick;
        let before = world.save_bytes().unwrap();
        let id = CommodityId::Food;
        assert_eq!(
            world.request_commodity_trade(order(TradeSide::Buy, id, 0), tick),
            Err(MissionError::Service(Error::ZeroQuantity))
        );
        assert_eq!(
            world.request_commodity_trade(order(TradeSide::Buy, id, MAX_TRADE_QUANTITY + 1), tick),
            Err(MissionError::Service(Error::QuantityLimit))
        );
        assert!(
            world
                .request_commodity_trade(order(TradeSide::Buy, id, 1000), tick)
                .is_err()
        );
        assert_eq!(world.save_bytes().unwrap(), before);
        let hold = world.service_snapshot().unwrap().used_tonnes;
        let id = CommodityId::GemStones;
        let quote = world.commodity_quote(id).unwrap();
        assert!(
            quote.stock >= 3
                && u64::from(quote.ask_tenths) * 3 <= u64::from(world.credits_tenths().unwrap())
        );
        world
            .request_commodity_trade(order(TradeSide::Buy, id, 3), tick)
            .unwrap();
        assert_eq!(world.commodity_quantity(id), Some(3));
        assert_eq!(world.service_snapshot().unwrap().used_tonnes, hold);
        let saved = world.save_bytes().unwrap();
        assert!(
            world
                .request_commodity_trade(order(TradeSide::Sell, id, 4), tick)
                .is_err()
        );
        assert_eq!(world.save_bytes().unwrap(), saved);
    }
}

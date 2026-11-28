//! One atomic ordinary trade; retain current native per-unit receipt chronology.
use super::{
    AccountProjection, CommodityId, Error, Event, MarketSite, ServiceFacts, StationMarkets,
    history, validate_account, validate_service,
};

/// Maximum bounded request. Remaining receipt capacity may impose a smaller limit.
pub const MAX_TRADE_QUANTITY: u64 = history::MAX_RECEIPTS as u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommodityTrade {
    pub side: TradeSide,
    pub commodity: CommodityId,
    pub quantity: u64,
}

impl CommodityTrade {
    pub(crate) fn single_command(self) -> super::Command {
        match self.side {
            TradeSide::Buy => super::Command::BuyCommodity(self.commodity),
            TradeSide::Sell => super::Command::SellCommodity(self.commodity),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommodityTradeEvent {
    pub site: MarketSite,
    pub side: TradeSide,
    pub commodity: CommodityId,
    pub quantity: u64,
    pub unit_price_tenths: u32,
    pub gross_tenths: u64,
    pub paid_tenths: u32,
    pub credited_tenths: u32,
}

impl CommodityTradeEvent {
    /// Only the quantity-one T04 path may obtain its exact ordinary event.
    pub(crate) fn single_event(self) -> Result<Event, Error> {
        if self.quantity != 1 {
            return Err(Error::InvalidAccount);
        }
        Ok(match (self.side, self.commodity) {
            (TradeSide::Buy, CommodityId::Food) => Event::BoughtFood {
                site: self.site,
                paid_tenths: self.paid_tenths,
            },
            (TradeSide::Sell, CommodityId::Food) => Event::SoldFood {
                site: self.site,
                bid_tenths: self.unit_price_tenths,
                credited_tenths: self.credited_tenths,
            },
            (TradeSide::Buy, commodity) => Event::BoughtCommodity {
                site: self.site,
                commodity,
                paid_tenths: self.paid_tenths,
            },
            (TradeSide::Sell, commodity) => Event::SoldCommodity {
                site: self.site,
                commodity,
                bid_tenths: self.unit_price_tenths,
                credited_tenths: self.credited_tenths,
            },
        })
    }
}

impl StationMarkets {
    /// The existing single-unit kernel is evaluated on local prospective values. One
    /// bounded receipt vector is copied, then all units are staged before any publication.
    pub(crate) fn prepare_commodity_trade(
        &self,
        account: AccountProjection,
        facts: ServiceFacts,
        trade: CommodityTrade,
    ) -> Result<(Self, AccountProjection, CommodityTradeEvent), Error> {
        validate_account(account)?;
        let facts = validate_service(self.data.owner, account, Some(facts))?;
        if trade.quantity == 0 {
            return Err(Error::ZeroQuantity);
        }
        if trade.quantity > MAX_TRADE_QUANTITY {
            return Err(Error::QuantityLimit);
        }
        let count = usize::try_from(trade.quantity).map_err(|_| Error::QuantityLimit)?;
        if count > history::MAX_RECEIPTS.saturating_sub(self.receipts.len()) {
            return Err(Error::SequenceExhausted);
        }
        let quote = self.commodity_quote(facts.site, trade.commodity);
        let unit_price = match trade.side {
            TradeSide::Buy => quote.ask_tenths,
            TradeSide::Sell => quote.bid_tenths,
        };
        let gross = u64::from(unit_price)
            .checked_mul(trade.quantity)
            .ok_or(Error::PriceOverflow)?;
        let mut candidate = Self {
            data: self.data,
            receipts: self.receipts.clone(),
        };
        let mut receipts = self.receipts.as_ref().clone();
        let mut after = account;
        for _ in 0..count {
            // No caller prices or projection are installed into the real market. The
            // same single-unit stock, cash, hold, reserved cargo and saturation rules run.
            let unit = candidate.prepare(after, Some(facts), trade.single_command())?;
            candidate.data = unit.next;
            after = unit.next_account;
            receipts.push(history::Receipt {
                revision: after.revision,
                event: unit.event,
            });
        }
        validate_account(after)?;
        candidate.receipts = std::sync::Arc::new(receipts);
        Ok((
            candidate,
            after,
            CommodityTradeEvent {
                site: facts.site,
                side: trade.side,
                commodity: trade.commodity,
                quantity: trade.quantity,
                unit_price_tenths: unit_price,
                gross_tenths: gross,
                paid_tenths: account.credits_tenths.saturating_sub(after.credits_tenths),
                credited_tenths: after.credits_tenths.saturating_sub(account.credits_tenths),
            },
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn bulk_trade_preserves_reserved_food_and_saturating_per_unit_sale_receipts() {
        let markets = StationMarkets::new_commander(61103).unwrap();
        let mut account = AccountProjection {
            owner: 61103,
            hull: 61104,
            revision: 3,
            credits_tenths: u32::MAX - 1,
            fuel_tenths: 130,
            fuel_capacity_tenths: 130,
            food_tonnes: 2,
            reserved_food_tonnes: 1,
            used_tonnes: 2,
            hold_capacity_tonnes: 92,
            other_commodities: [0; 16],
            reserved_commodities: [0; 16],
        };
        let facts = ServiceFacts {
            owner: 61103,
            hull: 61104,
            site: MarketSite::Avenor,
            station_entity: 1,
            visit: 1,
            dock_sequence: 1,
            epoch: 1,
            paused: false,
        };
        let trade = CommodityTrade {
            side: TradeSide::Sell,
            commodity: CommodityId::Food,
            quantity: 2,
        };
        assert!(matches!(
            markets.prepare_commodity_trade(account, facts, trade),
            Err(Error::NoSellableCargo)
        ));
        assert!(markets.receipts.is_empty());
        account.reserved_food_tonnes = 0;
        let (candidate, after, event) = markets
            .prepare_commodity_trade(account, facts, trade)
            .unwrap();
        assert_eq!(event.credited_tenths, 1);
        assert_eq!(after.credits_tenths, u32::MAX);
        assert_eq!(after.food_tonnes, 0);
        assert_eq!(candidate.receipts.len(), 2);
        assert!(matches!(
            candidate.receipts[0].event,
            Event::SoldFood {
                credited_tenths: 1,
                ..
            }
        ));
        assert!(matches!(
            candidate.receipts[1].event,
            Event::SoldFood {
                credited_tenths: 0,
                ..
            }
        ));
        assert_eq!(
            (
                candidate.receipts[0].revision,
                candidate.receipts[1].revision
            ),
            (4, 5)
        );
    }
}

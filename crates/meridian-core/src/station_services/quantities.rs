//! One atomic ordinary trade, using the existing native units and finite ledgers.
use super::{
    AccountProjection, CargoUnit, CommodityId, CommodityQuote, Error, Event, MAX_MARKET_QUANTITY,
    MarketData, MarketSite, Prepared, ServiceFacts, StationMarkets, validate_account,
    validate_service,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeSide {
    Buy,
    Sell,
}
/// Player intent only. World obtains the actual station, prices, custody and reservations.
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
/// Exact prospective/committed totals. Ordinary sales retain the existing UInt32 cash cap;
/// gross bid value and actual credited amount are separate so the host can show both.
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
    /// Used only by World's quantity-one path to retain the exact original lesson receipt.
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
struct QuantityChange {
    account: AccountProjection,
    held: u64,
    stock: u64,
    paid: u32,
    credited: u32,
}
impl StationMarkets {
    pub(crate) fn prepare_commodity_trade(
        &self,
        account: AccountProjection,
        facts: Option<ServiceFacts>,
        trade: CommodityTrade,
    ) -> Result<Prepared<CommodityTradeEvent>, Error> {
        validate_account(account)?;
        let facts = validate_service(self.data.owner, account, facts)?;
        if trade.quantity == 0 {
            return Err(Error::ZeroQuantity);
        }
        if trade.quantity > MAX_MARKET_QUANTITY {
            return Err(Error::QuantityLimit);
        }
        let quote = self.commodity_quote(facts.site, trade.commodity);
        let unit_price = match trade.side {
            TradeSide::Buy => quote.ask_tenths,
            TradeSide::Sell => quote.bid_tenths,
        };
        let gross = u64::from(unit_price)
            .checked_mul(trade.quantity)
            .ok_or(Error::PriceOverflow)?;
        let mut next = self.data;
        let mut after = account;
        // Save counters count native units, not UI confirmations: same accounting as N singles.
        next.revision = next
            .revision
            .checked_add(trade.quantity)
            .ok_or(Error::SequenceExhausted)?;
        after.revision = after
            .revision
            .checked_add(trade.quantity)
            .ok_or(Error::SequenceExhausted)?;
        let change = match trade.side {
            TradeSide::Buy => prepare_buy(after, quote, trade, gross)?,
            TradeSide::Sell => prepare_sell(after, quote, trade, gross)?,
        };
        let after = record(&mut next, facts.site, trade, change)?;
        validate_account(after)?;
        Ok(Prepared {
            before: self.data,
            account,
            facts,
            next,
            next_account: after,
            event: CommodityTradeEvent {
                site: facts.site,
                side: trade.side,
                commodity: trade.commodity,
                quantity: trade.quantity,
                unit_price_tenths: unit_price,
                gross_tenths: gross,
                paid_tenths: account.credits_tenths.saturating_sub(after.credits_tenths),
                credited_tenths: after.credits_tenths.saturating_sub(account.credits_tenths),
            },
        })
    }
}
fn native_quantity(account: AccountProjection, id: CommodityId) -> (u64, u64) {
    if id == CommodityId::Food {
        (
            u64::from(account.food_tonnes),
            u64::from(account.reserved_food_tonnes),
        )
    } else {
        let index = id.index() - 1;
        (
            account.other_commodities[index],
            account.reserved_commodities[index],
        )
    }
}
fn prepare_buy(
    mut account: AccountProjection,
    quote: CommodityQuote,
    trade: CommodityTrade,
    gross: u64,
) -> Result<QuantityChange, Error> {
    let stock = quote
        .stock
        .checked_sub(trade.quantity)
        .ok_or(Error::OutOfStock)?;
    let held = native_quantity(account, trade.commodity)
        .0
        .checked_add(trade.quantity)
        .filter(|q| *q <= MAX_MARKET_QUANTITY)
        .ok_or(Error::QuantityLimit)?;
    if trade.commodity.unit() == CargoUnit::Tonne {
        let used = u64::from(account.used_tonnes)
            .checked_add(trade.quantity)
            .ok_or(Error::HoldFull)?;
        if used > u64::from(account.hold_capacity_tonnes) {
            return Err(Error::HoldFull);
        }
        account.used_tonnes = u16::try_from(used).map_err(|_| Error::HoldFull)?;
    }
    let paid = u32::try_from(gross).map_err(|_| Error::InsufficientFunds)?;
    account.credits_tenths = account
        .credits_tenths
        .checked_sub(paid)
        .ok_or(Error::InsufficientFunds)?;
    Ok(QuantityChange {
        account,
        held,
        stock,
        paid,
        credited: 0,
    })
}
fn prepare_sell(
    mut account: AccountProjection,
    quote: CommodityQuote,
    trade: CommodityTrade,
    gross: u64,
) -> Result<QuantityChange, Error> {
    let (held, reserved) = native_quantity(account, trade.commodity);
    let held = held
        .checked_sub(trade.quantity)
        .filter(|q| *q >= reserved)
        .ok_or(Error::NoSellableCargo)?;
    let stock = quote
        .stock
        .checked_add(trade.quantity)
        .filter(|q| *q <= MAX_MARKET_QUANTITY)
        .ok_or(Error::QuantityLimit)?;
    if trade.commodity.unit() == CargoUnit::Tonne {
        let used = u64::from(account.used_tonnes)
            .checked_sub(trade.quantity)
            .ok_or(Error::InvalidAccount)?;
        account.used_tonnes = u16::try_from(used).map_err(|_| Error::InvalidAccount)?;
    }
    // Source ordinary-market saturation, not wrapping or a mission reward overflow waiver.
    let room = u64::from(u32::MAX - account.credits_tenths);
    let credited = u32::try_from(gross.min(room)).map_err(|_| Error::PriceOverflow)?;
    account.credits_tenths += credited;
    Ok(QuantityChange {
        account,
        held,
        stock,
        paid: 0,
        credited,
    })
}
fn record(
    next: &mut MarketData,
    site: MarketSite,
    trade: CommodityTrade,
    change: QuantityChange,
) -> Result<AccountProjection, Error> {
    let mut after = change.account;
    let i = site.index();
    let count = if trade.commodity == CommodityId::Food {
        after.food_tonnes = u16::try_from(change.held).map_err(|_| Error::HoldFull)?;
        next.quotes[i].stock = change.stock;
        match trade.side {
            TradeSide::Buy => &mut next.bought[i],
            TradeSide::Sell => &mut next.sold[i],
        }
    } else {
        let index = trade.commodity.index() - 1;
        after.other_commodities[index] = change.held;
        next.other_quotes[i][index].stock = change.stock;
        match trade.side {
            TradeSide::Buy => &mut next.other_bought[i][index],
            TradeSide::Sell => &mut next.other_sold[i][index],
        }
    };
    *count = count
        .checked_add(trade.quantity)
        .ok_or(Error::SequenceExhausted)?;
    next.paid_tenths = next
        .paid_tenths
        .checked_add(u64::from(change.paid))
        .ok_or(Error::SequenceExhausted)?;
    next.credited_tenths = next
        .credited_tenths
        .checked_add(u64::from(change.credited))
        .ok_or(Error::SequenceExhausted)?;
    Ok(after)
}

#[cfg(test)]
mod tests;

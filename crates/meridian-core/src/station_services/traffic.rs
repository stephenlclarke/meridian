//! Actual traffic owner stages stock transfers; player custody/payment counters stay separate.
use super::{CommodityId, Error, MAX_MARKET_QUANTITY, MarketSite, StationMarkets};
impl StationMarkets {
    pub(crate) fn traffic_candidate(&self) -> Self {
        Self { data: self.data }
    }
    pub(crate) fn prepare_traffic_stock(
        &self,
        site: MarketSite,
        id: CommodityId,
        quantity: u64,
        buying: bool,
    ) -> Result<Self, Error> {
        if quantity == 0 || quantity > MAX_MARKET_QUANTITY {
            return Err(Error::QuantityLimit);
        }
        let stock = self.commodity_quote(site, id).stock;
        let stock = if buying {
            stock.checked_sub(quantity).ok_or(Error::OutOfStock)?
        } else {
            stock
                .checked_add(quantity)
                .filter(|n| *n <= MAX_MARKET_QUANTITY)
                .ok_or(Error::QuantityLimit)?
        };
        let mut next = self.traffic_candidate();
        if id == CommodityId::Food {
            next.data.quotes[site.index()].stock = stock;
        } else {
            next.data.other_quotes[site.index()][id.index() - 1].stock = stock;
        }
        Ok(next)
    }
}

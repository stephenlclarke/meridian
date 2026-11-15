use super::*;
use crate::save::{Reader, SaveError, Writer};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SavedServiceSummary {
    pub(crate) revision: u64,
    pub(crate) food: u64,
    pub(crate) refilled_units: u64,
    pub(crate) paid: u64,
    pub(crate) credited: u64,
    pub(crate) gross_credit: u128,
}
impl StationMarkets {
    pub(crate) fn encode_save(&self, w: &mut Writer) {
        w.u64(self.data.owner);
        w.u64(self.data.revision);
        for quote in self.data.quotes {
            w.u8(quote.site.index() as u8);
            w.u32(quote.ask_tenths);
            w.u32(quote.bid_tenths);
            w.u64(quote.stock);
        }
        for count in self.data.bought {
            w.u64(count);
        }
        for count in self.data.sold {
            w.u64(count);
        }
        w.u64(self.data.refills);
        w.u64(self.data.refilled_units);
        w.u64(self.data.paid_tenths);
        w.u64(self.data.credited_tenths);
    }
    pub(crate) fn decode_save(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            data: MarketData {
                owner: r.u64()?,
                revision: r.u64()?,
                quotes: r.array(|r| {
                    Ok(FoodQuote {
                        site: match r.u8()? {
                            0 => MarketSite::Avenor,
                            1 => MarketSite::Renas,
                            _ => return Err(SaveError::InvalidEncoding),
                        },
                        ask_tenths: r.u32()?,
                        bid_tenths: r.u32()?,
                        stock: r.u64()?,
                    })
                })?,
                bought: r.array(|r| r.u64())?,
                sold: r.array(|r| r.u64())?,
                refills: r.u64()?,
                refilled_units: r.u64()?,
                paid_tenths: r.u64()?,
                credited_tenths: r.u64()?,
            },
        })
    }
    pub(crate) fn validate_save(&self, owner: u64) -> Result<SavedServiceSummary, SaveError> {
        let invalid = SaveError::InvalidState("market stock or accounting");
        if owner == 0 || self.data.owner != owner {
            return Err(invalid);
        }
        let seeds = read_seeds(SEEDS).map_err(|_| SaveError::ContentMismatch)?;
        let mut bought = 0_u128;
        let mut sold = 0_u128;
        let mut paid = u128::from(self.data.refilled_units) * u128::from(FUEL_PRICE_TENTHS);
        let mut gross_credit = 0_u128;
        for (i, seed) in seeds.iter().enumerate() {
            let quote = self.data.quotes[i];
            let site_bought = u128::from(self.data.bought[i]);
            let site_sold = u128::from(self.data.sold[i]);
            if quote.site != seed.site
                || quote.ask_tenths != seed.ask_tenths
                || quote.bid_tenths != seed.bid_tenths
                || quote.stock > MAX_MARKET_QUANTITY
                || u128::from(seed.stock) + site_sold != u128::from(quote.stock) + site_bought
            {
                return Err(invalid);
            }
            bought += site_bought;
            sold += site_sold;
            paid += site_bought * u128::from(seed.ask_tenths);
            gross_credit += site_sold * u128::from(seed.bid_tenths);
        }
        if sold > bought
            || bought - sold > u128::from(STOCK_HOLD_TONNES)
            || bought + sold + u128::from(self.data.refills) != u128::from(self.data.revision)
            || u128::from(self.data.refilled_units)
                > u128::from(self.data.refills) * u128::from(STOCK_FUEL_TENTHS)
            || paid != u128::from(self.data.paid_tenths)
            || u128::from(self.data.credited_tenths) > gross_credit
        {
            return Err(invalid);
        }
        Ok(SavedServiceSummary {
            revision: self.data.revision,
            food: (bought - sold) as u64,
            refilled_units: self.data.refilled_units,
            paid: self.data.paid_tenths,
            credited: self.data.credited_tenths,
            gross_credit,
        })
    }
}

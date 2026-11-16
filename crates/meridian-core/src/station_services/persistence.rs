use super::*;
use crate::save::{Reader, SaveError, Writer};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SavedServiceSummary {
    pub(crate) revision: u64,
    pub(crate) food: u64,
    pub(crate) other_commodities: [u64; 16],
    pub(crate) possible_legal_bits: u8,
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
        for site in self.data.other_quotes {
            for quote in site {
                w.u64(quote.stock);
            }
        }
        for site in self.data.other_bought {
            for count in site {
                w.u64(count);
            }
        }
        for site in self.data.other_sold {
            for count in site {
                w.u64(count);
            }
        }
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
                other_quotes: decode_other_stocks(r)?,
                other_bought: r.array(|r| r.array(|r| r.u64()))?,
                other_sold: r.array(|r| r.array(|r| r.u64()))?,
            },
        })
    }
    pub(crate) fn validate_save(
        &self,
        owner: u64,
        capacity: u16,
        historical_capacity: u16,
    ) -> Result<SavedServiceSummary, SaveError> {
        let invalid = SaveError::InvalidState("market stock or accounting");
        if owner == 0 || self.data.owner != owner {
            return Err(invalid);
        }
        let food_summary = validate_food_markets(&self.data, capacity)?;
        let food = food_summary.food;
        let mut bought = food_summary.bought;
        let mut sold = food_summary.sold;
        let mut paid = food_summary.paid
            + u128::from(self.data.refilled_units) * u128::from(FUEL_PRICE_TENTHS);
        let mut gross_credit = food_summary.gross_credit;
        let other = validate_other_markets(&self.data, historical_capacity)?;
        bought += other.bought;
        sold += other.sold;
        paid += other.paid;
        gross_credit += other.gross_credit;
        let other_commodities = other.quantities;
        let possible_legal_bits = other.possible_legal_bits;
        if commodities::other_tonnes(other_commodities) + food > u128::from(capacity) {
            return Err(invalid);
        }
        if sold > bought
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
            food: food as u64,
            other_commodities,
            possible_legal_bits,
            refilled_units: self.data.refilled_units,
            paid: self.data.paid_tenths,
            credited: self.data.credited_tenths,
            gross_credit,
        })
    }
}

fn decode_other_stocks(r: &mut Reader<'_>) -> Result<[[CommodityQuote; 16]; 2], SaveError> {
    let mut quotes = commodities::read_market_seeds().map_err(|_| SaveError::ContentMismatch)?;
    for site in &mut quotes {
        for quote in site {
            quote.stock = r.u64()?;
        }
    }
    Ok(quotes)
}

struct OtherSummary {
    quantities: [u64; 16],
    bought: u128,
    sold: u128,
    paid: u128,
    gross_credit: u128,
    possible_legal_bits: u8,
}
fn validate_other_markets(
    data: &MarketData,
    historical_capacity: u16,
) -> Result<OtherSummary, SaveError> {
    let invalid = SaveError::InvalidState("native commodity stock or accounting");
    let mut other_commodities = [0; 16];
    let mut normal_illegal_bought = 0_u128;
    let mut narcotic_bought = 0_u128;
    let mut bought = 0_u128;
    let mut sold = 0_u128;
    let mut paid = 0_u128;
    let mut gross_credit = 0_u128;
    let other_seeds = commodities::read_market_seeds().map_err(|_| SaveError::ContentMismatch)?;
    for id in CommodityId::ALL.into_iter().skip(1) {
        let index = id.index() - 1;
        let mut commodity_bought = 0_u128;
        let mut commodity_sold = 0_u128;
        for (site, seed_rows) in other_seeds.iter().enumerate() {
            let seed = seed_rows[index];
            let quote = data.other_quotes[site][index];
            let b = u128::from(data.other_bought[site][index]);
            let s = u128::from(data.other_sold[site][index]);
            let mut expected = seed;
            expected.stock = quote.stock;
            if quote != expected
                || quote.stock > MAX_MARKET_QUANTITY
                || u128::from(seed.stock) + s != u128::from(quote.stock) + b
            {
                return Err(invalid);
            }
            commodity_bought += b;
            commodity_sold += s;
            paid += b * u128::from(seed.ask_tenths);
            gross_credit += s * u128::from(seed.bid_tenths);
        }
        let held = commodity_bought
            .checked_sub(commodity_sold)
            .ok_or(invalid)?;
        if held > u128::from(MAX_MARKET_QUANTITY) {
            return Err(invalid);
        }
        other_commodities[index] = held as u64;
        match id {
            CommodityId::Slaves | CommodityId::Weapons => normal_illegal_bought += commodity_bought,
            CommodityId::Narcotics => narcotic_bought = commodity_bought,
            _ => {}
        }
        bought += commodity_bought;
        sold += commodity_sold;
    }

    Ok(OtherSummary {
        quantities: other_commodities,
        bought,
        sold,
        paid,
        gross_credit,
        possible_legal_bits: possible_export_bits(
            normal_illegal_bought,
            narcotic_bought,
            historical_capacity,
        ),
    })
}

// Conservative history check, not save authentication: an OR bit must be reachable from
// purchased native illegal tonne goods and this real hull's capacity. Export chronology
// is not reconstructed or reset on Continue.
fn possible_export_bits(normal: u128, narcotics: u128, capacity: u16) -> u8 {
    let normal = normal.min(u128::from(capacity)) as u16;
    let narcotics = narcotics.min(u128::from(capacity)) as u16;
    let mut bits = 0_u8;
    for n in 0..=narcotics {
        for ordinary in 0..=normal.min(capacity - n) {
            bits |= (ordinary + 2 * n) as u8;
        }
    }
    bits
}

struct FoodSummary {
    food: u128,
    bought: u128,
    sold: u128,
    paid: u128,
    gross_credit: u128,
}
fn validate_food_markets(data: &MarketData, capacity: u16) -> Result<FoodSummary, SaveError> {
    let invalid = SaveError::InvalidState("Food stock or accounting");
    let seeds = read_seeds(SEEDS).map_err(|_| SaveError::ContentMismatch)?;
    let mut bought = 0_u128;
    let mut sold = 0_u128;
    let mut paid = 0_u128;
    let mut gross_credit = 0_u128;
    for (i, seed) in seeds.iter().enumerate() {
        let quote = data.quotes[i];
        let site_bought = u128::from(data.bought[i]);
        let site_sold = u128::from(data.sold[i]);
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
    let food = bought.checked_sub(sold).ok_or(invalid)?;
    if food > u128::from(capacity) {
        return Err(invalid);
    }

    Ok(FoodSummary {
        food,
        bought,
        sold,
        paid,
        gross_credit,
    })
}

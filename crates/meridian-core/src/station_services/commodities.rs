//! Original seventeen native identities, units and finite quotes. No caller-defined market data.
use super::*;
use crate::save::{Reader, SaveError};

const CATALOGUE: &[u8] = include_bytes!("../../../../assets/legacy/commodities.catalogue");
const MARKETS: &[u8] = include_bytes!("../../../../assets/legacy/g01-commodities.markets");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommodityId {
    Food,
    Textiles,
    Radioactives,
    Slaves,
    Liquor,
    Luxuries,
    Narcotics,
    Computers,
    Machinery,
    Alloys,
    Weapons,
    Furs,
    Minerals,
    Gold,
    Platinum,
    GemStones,
    AlienItems,
}
impl CommodityId {
    pub const ALL: [Self; 17] = [
        Self::Food,
        Self::Textiles,
        Self::Radioactives,
        Self::Slaves,
        Self::Liquor,
        Self::Luxuries,
        Self::Narcotics,
        Self::Computers,
        Self::Machinery,
        Self::Alloys,
        Self::Weapons,
        Self::Furs,
        Self::Minerals,
        Self::Gold,
        Self::Platinum,
        Self::GemStones,
        Self::AlienItems,
    ];
    pub const fn index(self) -> usize {
        self as usize
    }
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }
    pub const fn unit(self) -> CargoUnit {
        match self {
            Self::Gold | Self::Platinum => CargoUnit::Kilogram,
            Self::GemStones => CargoUnit::Gram,
            _ => CargoUnit::Tonne,
        }
    }
    pub const fn departure_legal_weight(self) -> u8 {
        match self {
            Self::Slaves | Self::Weapons => 1,
            Self::Narcotics => 2,
            _ => 0,
        }
    }
    pub fn definition(self) -> Result<CommodityDefinition, Error> {
        Ok(read_catalogue()?.0[self.index()])
    }
    pub fn display_order() -> Result<[Self; 17], Error> {
        Ok(read_catalogue()?.1)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoUnit {
    Tonne,
    Kilogram,
    Gram,
}
impl CargoUnit {
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Tonne => "t",
            Self::Kilogram => "kg",
            Self::Gram => "g",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommodityDefinition {
    pub id: CommodityId,
    pub name: &'static str,
    pub sector: &'static str,
    pub unit: CargoUnit,
    pub departure_legal_weight: u8,
    pub base_price_cr: f64,
    pub target_stock: u64,
    pub technology: u16,
    pub volatility: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommodityQuote {
    pub site: MarketSite,
    pub commodity: CommodityId,
    pub ask_tenths: u32,
    pub bid_tenths: u32,
    pub stock: u64,
    pub target_stock: u64,
    pub epoch: u64,
}
impl StationMarkets {
    pub fn commodity_quote(&self, site: MarketSite, id: CommodityId) -> CommodityQuote {
        if id == CommodityId::Food {
            let food = self.quote(site);
            // Food remains the single original mutable row; target/epoch are immutable content.
            let seed = food_seed(site);
            CommodityQuote {
                site,
                commodity: id,
                ask_tenths: food.ask_tenths,
                bid_tenths: food.bid_tenths,
                stock: food.stock,
                target_stock: seed.0,
                epoch: seed.1,
            }
        } else {
            self.data.other_quotes[site.index()][id.index() - 1]
        }
    }
}
// These two immutable values are read from the same validated full source; never stock authority.
fn food_seed(site: MarketSite) -> (u64, u64) {
    let row = 14 + site.index() * 563;
    // StationMarkets construction and save decoding validate this immutable 1138-byte layout.
    // There is no fallback quote or restock if content is incompatible.
    let bytes = &MARKETS[row + 17..row + 33];
    let target = u64::from_le_bytes(core::array::from_fn(|index| bytes[index]));
    let epoch = u64::from_le_bytes(core::array::from_fn(|index| bytes[index + 8]));
    (target, epoch)
}
pub(super) fn other_tonnes(quantities: [u64; 16]) -> u128 {
    CommodityId::ALL[1..]
        .iter()
        .zip(quantities)
        .filter(|(id, _)| id.unit() == CargoUnit::Tonne)
        .map(|(_, q)| u128::from(q))
        .sum()
}
pub(super) fn buy(
    next: &mut MarketData,
    account: &mut AccountProjection,
    site: MarketSite,
    id: CommodityId,
) -> Result<Event, Error> {
    let index = id.index().checked_sub(1).ok_or(Error::InvalidContent)?;
    let quote = &mut next.other_quotes[site.index()][index];
    if quote.stock == 0 {
        return Err(Error::OutOfStock);
    }
    if account.other_commodities[index] >= MAX_MARKET_QUANTITY {
        return Err(Error::QuantityLimit);
    }
    if id.unit() == CargoUnit::Tonne && account.used_tonnes >= STOCK_HOLD_TONNES {
        return Err(Error::HoldFull);
    }
    account.credits_tenths = account
        .credits_tenths
        .checked_sub(quote.ask_tenths)
        .ok_or(Error::InsufficientFunds)?;
    account.other_commodities[index] += 1;
    if id.unit() == CargoUnit::Tonne {
        account.used_tonnes += 1;
    }
    quote.stock -= 1;
    Ok(Event::BoughtCommodity {
        site,
        commodity: id,
        paid_tenths: quote.ask_tenths,
    })
}
pub(super) fn sell(
    next: &mut MarketData,
    account: &mut AccountProjection,
    site: MarketSite,
    id: CommodityId,
) -> Result<Event, Error> {
    let index = id.index().checked_sub(1).ok_or(Error::InvalidContent)?;
    let quote = &mut next.other_quotes[site.index()][index];
    if account.other_commodities[index] <= account.reserved_commodities[index] {
        return Err(Error::NoSellableCargo);
    }
    if quote.stock >= MAX_MARKET_QUANTITY {
        return Err(Error::QuantityLimit);
    }
    let credits = account.credits_tenths.saturating_add(quote.bid_tenths);
    let credited_tenths = credits - account.credits_tenths;
    account.credits_tenths = credits;
    account.other_commodities[index] -= 1;
    if id.unit() == CargoUnit::Tonne {
        account.used_tonnes -= 1;
    }
    quote.stock += 1;
    Ok(Event::SoldCommodity {
        site,
        commodity: id,
        bid_tenths: quote.bid_tenths,
        credited_tenths,
    })
}
fn read_text(bytes: &mut &'static [u8]) -> Result<&'static str, Error> {
    let (&len, rest) = bytes.split_first().ok_or(Error::InvalidContent)?;
    let end = usize::from(len);
    let text = rest.get(..end).ok_or(Error::InvalidContent)?;
    *bytes = rest.get(end..).ok_or(Error::InvalidContent)?;
    let text = core::str::from_utf8(text).map_err(|_| Error::InvalidContent)?;
    if text.is_empty() {
        return Err(Error::InvalidContent);
    }
    Ok(text)
}
fn take<const N: usize>(bytes: &mut &'static [u8]) -> Result<[u8; N], Error> {
    let value = bytes
        .get(..N)
        .ok_or(Error::InvalidContent)?
        .try_into()
        .map_err(|_| Error::InvalidContent)?;
    *bytes = bytes.get(N..).ok_or(Error::InvalidContent)?;
    Ok(value)
}
fn read_definition(
    bytes: &mut &'static [u8],
    id: CommodityId,
) -> Result<CommodityDefinition, Error> {
    let [raw_id, unit, legal] = take::<3>(bytes)?;
    if raw_id as usize != id.index()
        || unit
            != match id.unit() {
                CargoUnit::Tonne => 0,
                CargoUnit::Kilogram => 1,
                CargoUnit::Gram => 2,
            }
        || legal != id.departure_legal_weight()
    {
        return Err(Error::InvalidContent);
    }
    let name = read_text(bytes)?;
    let sector = read_text(bytes)?;
    let base_price_cr = f64::from_le_bytes(take(bytes)?);
    let target_stock = u64::from_le_bytes(take(bytes)?);
    let technology = u16::from_le_bytes(take(bytes)?);
    let volatility = f64::from_le_bytes(take(bytes)?);
    if !base_price_cr.is_finite()
        || base_price_cr <= 0.0
        || !volatility.is_finite()
        || volatility < 0.0
        || target_stock > MAX_MARKET_QUANTITY
    {
        return Err(Error::InvalidContent);
    }
    Ok(CommodityDefinition {
        id,
        name,
        sector,
        unit: id.unit(),
        departure_legal_weight: legal,
        base_price_cr,
        target_stock,
        technology,
        volatility,
    })
}
fn read_catalogue() -> Result<([CommodityDefinition; 17], [CommodityId; 17]), Error> {
    let mut bytes = CATALOGUE;
    if take::<10>(&mut bytes)? != *b"MCOM0001\x11\x00" {
        return Err(Error::InvalidContent);
    }
    let mut definitions = Vec::with_capacity(17);
    for id in CommodityId::ALL {
        definitions.push(read_definition(&mut bytes, id)?);
    }
    let raw_order = take::<17>(&mut bytes)?;
    let mut order = CommodityId::ALL;
    let mut seen = [false; 17];
    for (index, raw) in raw_order.into_iter().enumerate() {
        let id = CommodityId::from_index(usize::from(raw)).ok_or(Error::InvalidContent)?;
        if seen[id.index()] {
            return Err(Error::InvalidContent);
        }
        seen[id.index()] = true;
        order[index] = id;
    }
    if !bytes.is_empty() {
        return Err(Error::InvalidContent);
    }
    Ok((
        definitions.try_into().map_err(|_| Error::InvalidContent)?,
        order,
    ))
}
pub(super) fn read_market_seeds() -> Result<[[CommodityQuote; 16]; 2], Error> {
    read_catalogue()?;
    decode_markets(MARKETS).map_err(|_| Error::InvalidContent)
}
fn read_quote(
    r: &mut Reader<'_>,
    site: MarketSite,
    id: CommodityId,
) -> Result<CommodityQuote, SaveError> {
    let invalid = SaveError::ContentMismatch;
    if usize::from(r.u8()?) != id.index() {
        return Err(invalid);
    }
    let quote = CommodityQuote {
        site,
        commodity: id,
        ask_tenths: r.u32()?,
        bid_tenths: r.u32()?,
        stock: r.u64()?,
        target_stock: r.u64()?,
        epoch: r.u64()?,
    };
    if quote.bid_tenths > quote.ask_tenths
        || quote.stock > MAX_MARKET_QUANTITY
        || quote.target_stock > MAX_MARKET_QUANTITY
    {
        return Err(invalid);
    }
    Ok(quote)
}
fn decode_markets(bytes: &[u8]) -> Result<[[CommodityQuote; 16]; 2], SaveError> {
    let invalid = SaveError::ContentMismatch;
    let mut r = Reader::new(bytes);
    if r.fixed::<12>()? != *b"MMKT0002\x02\x00\x11\x00" {
        return Err(invalid);
    }
    let foods = read_seeds(SEEDS).map_err(|_| invalid)?;
    let mut result = Vec::with_capacity(2);
    for site in [MarketSite::Avenor, MarketSite::Renas] {
        if r.u16()? != site.system() {
            return Err(invalid);
        }
        let mut rows = Vec::with_capacity(16);
        for id in CommodityId::ALL {
            let quote = read_quote(&mut r, site, id)?;
            if id == CommodityId::Food {
                let food = foods[site.index()];
                if (food.ask_tenths, food.bid_tenths, food.stock)
                    != (quote.ask_tenths, quote.bid_tenths, quote.stock)
                {
                    return Err(invalid);
                }
            } else {
                rows.push(quote);
            }
        }
        result.push(rows.try_into().map_err(|_| invalid)?);
    }
    if !r.finished() {
        return Err(invalid);
    }
    result.try_into().map_err(|_| invalid)
}

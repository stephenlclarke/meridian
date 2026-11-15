//! Bounded ordinary Food commerce and paid stock-SH025 refuelling rules from immutable a11.
//! This prepares checked joint changes; only World can apply them to its sole actual account.
//! No snapshot, caller price, public fact constructor or historical event grants service access.

#![forbid(unsafe_code)]

const SEEDS: &[u8] = include_bytes!("../../../assets/legacy/g01-food.markets");
const STOCK_HOLD_TONNES: u16 = 92;
const STOCK_FUEL_TENTHS: u16 = 130;
const FUEL_PRICE_TENTHS: u32 = 2;
// Swift's supported 64-bit Int.max/17 quantity bound, not the immutable initial stock.
const MAX_MARKET_QUANTITY: u64 = i64::MAX as u64 / 17;

/// Descriptive choice. The World adapter must map its actual current main-orbital admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketSite {
    Avenor,
    Renas,
}
impl MarketSite {
    fn index(self) -> usize {
        match self {
            Self::Avenor => 0,
            Self::Renas => 1,
        }
    }
    pub fn system(self) -> u16 {
        match self {
            Self::Avenor => 7,
            Self::Renas => 185,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    BuyFood,
    SellFood,
    RefillFuel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidContent,
    InvalidAccount,
    WrongOwner,
    NotDocked,
    UnsupportedStation,
    Paused,
    StalePublication,
    InsufficientFunds,
    OutOfStock,
    HoldFull,
    NoSellableCargo,
    QuantityLimit,
    SequenceExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoodQuote {
    pub site: MarketSite,
    pub ask_tenths: u32,
    pub bid_tenths: u32,
    pub stock: u64,
}

/// Ephemeral projection of the sole account, never a second wallet or cargo owner.
/// World must derive used_tonnes from all actual ordinary, protected and G02-held cargo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccountProjection {
    pub(crate) owner: u64,
    pub(crate) hull: u64,
    pub(crate) revision: u64,
    pub(crate) credits_tenths: u32,
    pub(crate) fuel_tenths: u16,
    pub(crate) food_tonnes: u16,
    pub(crate) reserved_food_tonnes: u16,
    pub(crate) used_tonnes: u16,
}

/// Only a current World-owned initial berth/physical entry may supply this projection.
/// Its site must come from the bound galaxy0 system, not a UI selection or position test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ServiceFacts {
    pub(crate) owner: u64,
    pub(crate) hull: u64,
    pub(crate) site: MarketSite,
    pub(crate) station_entity: u64,
    pub(crate) visit: u64,
    pub(crate) dock_sequence: u64,
    pub(crate) epoch: u64,
    pub(crate) paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    BoughtFood {
        site: MarketSite,
        paid_tenths: u32,
    },
    SoldFood {
        site: MarketSite,
        bid_tenths: u32,
        credited_tenths: u32,
    },
    Refuelled {
        site: MarketSite,
        added_fuel_tenths: u16,
        paid_tenths: u32,
    },
}
impl Event {
    /// Actual cash spent by this committed ordinary service, excluding sale proceeds.
    pub fn paid_tenths(self) -> u32 {
        match self {
            Self::BoughtFood { paid_tenths, .. } | Self::Refuelled { paid_tenths, .. } => {
                paid_tenths
            }
            Self::SoldFood { .. } => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarketData {
    owner: u64,
    revision: u64,
    quotes: [FoodQuote; 2],
}

/// Only market stock is owned here. The actual account owns all credits, fuel and cargo.
#[derive(Debug)]
pub struct StationMarkets {
    data: MarketData,
}

/// Single-use prospective change. Dropping it changes neither stock nor the actual account.
#[derive(Debug)]
pub(crate) struct Prepared {
    before: MarketData,
    account: AccountProjection,
    facts: ServiceFacts,
    next: MarketData,
    next_account: AccountProjection,
    event: Event,
}

/// World must publish both parts under exclusive ownership, with no intervening await.
/// This is a checked candidate, not a saved receipt or permission to create a new account.
#[derive(Debug)]
pub(crate) struct Change {
    pub(crate) market: StationMarkets,
    pub(crate) account_after: AccountProjection,
    pub(crate) event: Event,
}

impl StationMarkets {
    /// Actual new-profile transition only. Never call on visit, load, depletion or reinstall.
    pub(crate) fn new_commander(owner: u64) -> Result<Self, Error> {
        if owner == 0 {
            return Err(Error::WrongOwner);
        }
        Ok(Self {
            data: MarketData {
                owner,
                revision: 0,
                quotes: read_seeds(SEEDS)?,
            },
        })
    }

    pub fn quote(&self, site: MarketSite) -> FoodQuote {
        self.data.quotes[site.index()]
    }

    pub(crate) fn prepare(
        &self,
        account: AccountProjection,
        facts: Option<ServiceFacts>,
        command: Command,
    ) -> Result<Prepared, Error> {
        validate_account(account)?;
        let facts = facts.ok_or(Error::NotDocked)?;
        if facts.paused {
            return Err(Error::Paused);
        }
        if self.data.owner != account.owner
            || facts.owner != account.owner
            || facts.hull != account.hull
        {
            return Err(Error::WrongOwner);
        }
        if facts.station_entity != 1 || facts.visit == 0 || facts.dock_sequence == 0 {
            return Err(Error::NotDocked);
        }
        let mut next = self.data;
        let mut next_account = account;
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        next_account.revision = account
            .revision
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        let quote = &mut next.quotes[facts.site.index()];
        let event = match command {
            Command::BuyFood => {
                if quote.stock == 0 {
                    return Err(Error::OutOfStock);
                }
                if account.used_tonnes >= STOCK_HOLD_TONNES {
                    return Err(Error::HoldFull);
                }
                next_account.credits_tenths = account
                    .credits_tenths
                    .checked_sub(quote.ask_tenths)
                    .ok_or(Error::InsufficientFunds)?;
                next_account.food_tonnes += 1;
                next_account.used_tonnes += 1;
                quote.stock -= 1;
                Event::BoughtFood {
                    site: facts.site,
                    paid_tenths: quote.ask_tenths,
                }
            }
            Command::SellFood => {
                if account.food_tonnes == account.reserved_food_tonnes {
                    return Err(Error::NoSellableCargo);
                }
                if quote.stock >= MAX_MARKET_QUANTITY {
                    return Err(Error::QuantityLimit);
                }
                // Ordinary market sale saturates at UInt32.max in the original Game kernel.
                // G02's once-only reward overflow refusal is a separate source rule.
                next_account.credits_tenths =
                    account.credits_tenths.saturating_add(quote.bid_tenths);
                next_account.food_tonnes -= 1;
                next_account.used_tonnes -= 1;
                quote.stock += 1;
                Event::SoldFood {
                    site: facts.site,
                    bid_tenths: quote.bid_tenths,
                    credited_tenths: next_account.credits_tenths - account.credits_tenths,
                }
            }
            Command::RefillFuel => {
                let missing = STOCK_FUEL_TENTHS - account.fuel_tenths;
                let price = u32::from(missing) * FUEL_PRICE_TENTHS;
                next_account.credits_tenths = account
                    .credits_tenths
                    .checked_sub(price)
                    .ok_or(Error::InsufficientFunds)?;
                next_account.fuel_tenths = STOCK_FUEL_TENTHS;
                Event::Refuelled {
                    site: facts.site,
                    added_fuel_tenths: missing,
                    paid_tenths: price,
                }
            }
        };
        Ok(Prepared {
            before: self.data,
            account,
            facts,
            next,
            next_account,
            event,
        })
    }
}

impl Prepared {
    /// Recheck the complete actual projection and current berth immediately before World publication.
    pub(crate) fn checked_change(
        self,
        market: &StationMarkets,
        account: AccountProjection,
        facts: Option<ServiceFacts>,
    ) -> Result<Change, Error> {
        if market.data != self.before || account != self.account || facts != Some(self.facts) {
            return Err(Error::StalePublication);
        }
        Ok(Change {
            market: StationMarkets { data: self.next },
            account_after: self.next_account,
            event: self.event,
        })
    }
}

fn validate_account(account: AccountProjection) -> Result<(), Error> {
    if account.owner == 0
        || account.hull == 0
        || account.owner == account.hull
        || account.fuel_tenths > STOCK_FUEL_TENTHS
        || account.reserved_food_tonnes > account.food_tonnes
        || account.food_tonnes > account.used_tonnes
        || account.used_tonnes > STOCK_HOLD_TONNES
    {
        return Err(Error::InvalidAccount);
    }
    Ok(())
}

fn read_seeds(bytes: &[u8]) -> Result<[FoodQuote; 2], Error> {
    if bytes.len() != 46 || bytes.get(..10) != Some(b"MMKT0001\x02\x00".as_slice()) {
        return Err(Error::InvalidContent);
    }
    let mut quotes = [FoodQuote {
        site: MarketSite::Avenor,
        ask_tenths: 0,
        bid_tenths: 0,
        stock: 0,
    }; 2];
    for (index, site) in [MarketSite::Avenor, MarketSite::Renas]
        .into_iter()
        .enumerate()
    {
        let row = &bytes[10 + index * 18..28 + index * 18];
        let system = u16::from_le_bytes([row[0], row[1]]);
        let ask = u32::from_le_bytes([row[2], row[3], row[4], row[5]]);
        let bid = u32::from_le_bytes([row[6], row[7], row[8], row[9]]);
        let stock = u64::from_le_bytes([
            row[10], row[11], row[12], row[13], row[14], row[15], row[16], row[17],
        ]);
        if system != site.system() || bid > ask || ask > 1_000_000 || stock > MAX_MARKET_QUANTITY {
            return Err(Error::InvalidContent);
        }
        quotes[index] = FoodQuote {
            site,
            ask_tenths: ask,
            bid_tenths: bid,
            stock,
        };
    }
    Ok(quotes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // Synthetic internal projections exercise domain arithmetic/transactions only.
    // They are not proof of actual World property, physical docking or service admission.
    fn account() -> AccountProjection {
        AccountProjection {
            owner: 101,
            hull: 102,
            revision: 0,
            credits_tenths: 7200,
            fuel_tenths: 96,
            food_tonnes: 0,
            reserved_food_tonnes: 0,
            used_tonnes: 8,
        }
    }
    fn facts(site: MarketSite) -> ServiceFacts {
        ServiceFacts {
            owner: 101,
            hull: 102,
            site,
            station_entity: 1,
            visit: 3,
            dock_sequence: 2,
            epoch: 4000,
            paused: false,
        }
    }
    fn change(
        market: &StationMarkets,
        account: AccountProjection,
        facts: ServiceFacts,
        command: Command,
    ) -> Change {
        market
            .prepare(account, Some(facts), command)
            .unwrap()
            .checked_change(market, account, Some(facts))
            .unwrap()
    }

    #[test]
    fn exact_original_food_seed_prices_stock_and_bound_content() {
        let market = StationMarkets::new_commander(101).unwrap();
        assert_eq!(
            market.quote(MarketSite::Avenor),
            FoodQuote {
                site: MarketSite::Avenor,
                ask_tenths: 226,
                bid_tenths: 208,
                stock: 128
            }
        );
        assert_eq!(
            market.quote(MarketSite::Renas),
            FoodQuote {
                site: MarketSite::Renas,
                ask_tenths: 216,
                bid_tenths: 196,
                stock: 112
            }
        );
        assert!(matches!(
            StationMarkets::new_commander(0),
            Err(Error::WrongOwner)
        ));
        assert_eq!(read_seeds(&SEEDS[..45]), Err(Error::InvalidContent));
        let mut bytes = SEEDS.to_vec();
        bytes[10] = 8;
        assert_eq!(read_seeds(&bytes), Err(Error::InvalidContent));
        let mut bytes = SEEDS.to_vec();
        bytes[2] = 0;
        assert_eq!(read_seeds(&bytes), Err(Error::InvalidContent));
    }

    #[test]
    fn ordinary_one_tonne_buy_then_sell_preserves_spread_stock_and_protected_hold() {
        for (site, ask, bid) in [
            (MarketSite::Avenor, 226, 208),
            (MarketSite::Renas, 216, 196),
        ] {
            let market = StationMarkets::new_commander(101).unwrap();
            let before = account();
            let berth = facts(site);
            let stock = market.quote(site).stock;
            let bought = change(&market, before, berth, Command::BuyFood);
            assert_eq!(
                bought.event,
                Event::BoughtFood {
                    site,
                    paid_tenths: ask
                }
            );
            assert_eq!(bought.account_after.credits_tenths, 7200 - ask);
            assert_eq!(
                (
                    bought.account_after.food_tonnes,
                    bought.account_after.used_tonnes
                ),
                (1, 9)
            );
            assert_eq!(bought.market.quote(site).stock, stock - 1);
            let sold = change(
                &bought.market,
                bought.account_after,
                berth,
                Command::SellFood,
            );
            assert_eq!(
                sold.event,
                Event::SoldFood {
                    site,
                    bid_tenths: bid,
                    credited_tenths: bid
                }
            );
            assert_eq!(sold.account_after.credits_tenths, 7200 - ask + bid);
            assert_eq!(
                (
                    sold.account_after.food_tonnes,
                    sold.account_after.used_tonnes
                ),
                (0, 8)
            );
            assert_eq!(sold.account_after.fuel_tenths, before.fuel_tenths);
            assert_eq!(sold.market.quote(site).stock, stock);
        }
    }

    #[test]
    fn station_stock_is_independent_and_retained_across_site_changes() {
        let market = StationMarkets::new_commander(101).unwrap();
        let bought = change(
            &market,
            account(),
            facts(MarketSite::Avenor),
            Command::BuyFood,
        );
        let sold = change(
            &bought.market,
            bought.account_after,
            facts(MarketSite::Renas),
            Command::SellFood,
        );
        assert_eq!(sold.market.quote(MarketSite::Avenor).stock, 127);
        assert_eq!(sold.market.quote(MarketSite::Renas).stock, 113);
        assert_eq!(sold.account_after.credits_tenths, 7200 - 226 + 196);
        assert_eq!(sold.account_after.food_tonnes, 0);
    }

    #[test]
    fn paid_full_refill_uses_missing_owned_fuel_and_exact_funds_without_free_grant() {
        let market = StationMarkets::new_commander(101).unwrap();
        let berth = facts(MarketSite::Renas);
        let before = account();
        let refilled = change(&market, before, berth, Command::RefillFuel);
        assert_eq!(
            refilled.event,
            Event::Refuelled {
                site: MarketSite::Renas,
                added_fuel_tenths: 34,
                paid_tenths: 68
            }
        );
        assert_eq!(
            (
                refilled.account_after.credits_tenths,
                refilled.account_after.fuel_tenths
            ),
            (7132, 130)
        );
        assert_eq!(refilled.market.data.quotes, market.data.quotes);
        assert_eq!(
            (
                refilled.account_after.food_tonnes,
                refilled.account_after.used_tonnes
            ),
            (0, 8)
        );
        let mut exact = before;
        exact.credits_tenths = 68;
        assert_eq!(
            change(&market, exact, berth, Command::RefillFuel)
                .account_after
                .credits_tenths,
            0
        );
        exact.credits_tenths = 67;
        assert!(matches!(
            market.prepare(exact, Some(berth), Command::RefillFuel),
            Err(Error::InsufficientFunds)
        ));
        let mut full = before;
        full.fuel_tenths = 130;
        full.credits_tenths = 0;
        assert_eq!(
            change(&market, full, berth, Command::RefillFuel).event,
            Event::Refuelled {
                site: MarketSite::Renas,
                added_fuel_tenths: 0,
                paid_tenths: 0
            }
        );
    }

    #[test]
    fn insufficient_money_stock_room_and_sellable_custody_refuse_without_mutation() {
        let mut market = StationMarkets::new_commander(101).unwrap();
        let berth = facts(MarketSite::Avenor);
        let before = market.data;
        let mut funds = account();
        funds.credits_tenths = 225;
        assert!(matches!(
            market.prepare(funds, Some(berth), Command::BuyFood),
            Err(Error::InsufficientFunds)
        ));
        let mut full = account();
        full.used_tonnes = 92;
        assert!(matches!(
            market.prepare(full, Some(berth), Command::BuyFood),
            Err(Error::HoldFull)
        ));
        assert!(matches!(
            market.prepare(account(), Some(berth), Command::SellFood),
            Err(Error::NoSellableCargo)
        ));
        let mut reserved = account();
        reserved.food_tonnes = 1;
        reserved.reserved_food_tonnes = 1;
        assert!(matches!(
            market.prepare(reserved, Some(berth), Command::SellFood),
            Err(Error::NoSellableCargo)
        ));
        assert_eq!(market.data, before);
        market.data.quotes[0].stock = 0;
        let depleted = market.data;
        assert!(matches!(
            market.prepare(account(), Some(berth), Command::BuyFood),
            Err(Error::OutOfStock)
        ));
        assert_eq!(market.data, depleted);
    }

    #[test]
    fn wrong_owner_missing_or_paused_berth_and_invalid_projection_cannot_prepare() {
        let market = StationMarkets::new_commander(101).unwrap();
        let berth = facts(MarketSite::Avenor);
        assert!(matches!(
            market.prepare(account(), None, Command::BuyFood),
            Err(Error::NotDocked)
        ));
        let mut paused = berth;
        paused.paused = true;
        assert!(matches!(
            market.prepare(account(), Some(paused), Command::RefillFuel),
            Err(Error::Paused)
        ));
        let mut foreign = berth;
        foreign.owner = 201;
        assert!(matches!(
            market.prepare(account(), Some(foreign), Command::BuyFood),
            Err(Error::WrongOwner)
        ));
        let mut foreign = berth;
        foreign.hull = 202;
        assert!(matches!(
            market.prepare(account(), Some(foreign), Command::BuyFood),
            Err(Error::WrongOwner)
        ));
        let mut fake = berth;
        fake.station_entity = 0;
        assert!(matches!(
            market.prepare(account(), Some(fake), Command::BuyFood),
            Err(Error::NotDocked)
        ));
        let mut invalid = account();
        invalid.fuel_tenths = 131;
        assert!(matches!(
            market.prepare(invalid, Some(berth), Command::RefillFuel),
            Err(Error::InvalidAccount)
        ));
        let mut invalid = account();
        invalid.reserved_food_tonnes = 1;
        assert!(matches!(
            market.prepare(invalid, Some(berth), Command::BuyFood),
            Err(Error::InvalidAccount)
        ));
    }

    #[test]
    fn dropped_plan_and_changed_account_stock_or_actual_berth_cannot_publish() {
        let market = StationMarkets::new_commander(101).unwrap();
        let before = account();
        let berth = facts(MarketSite::Avenor);
        {
            let _dropped = market
                .prepare(before, Some(berth), Command::BuyFood)
                .unwrap();
        }
        assert_eq!(market.quote(MarketSite::Avenor).stock, 128);
        assert_eq!(before.credits_tenths, 7200);
        let pending = market
            .prepare(before, Some(berth), Command::BuyFood)
            .unwrap();
        let mut changed_account = before;
        changed_account.fuel_tenths -= 1;
        assert!(matches!(
            pending.checked_change(&market, changed_account, Some(berth)),
            Err(Error::StalePublication)
        ));
        let pending = market
            .prepare(before, Some(berth), Command::BuyFood)
            .unwrap();
        let mut changed_berth = berth;
        changed_berth.visit += 1;
        assert!(matches!(
            pending.checked_change(&market, before, Some(changed_berth)),
            Err(Error::StalePublication)
        ));
        let pending = market
            .prepare(before, Some(berth), Command::BuyFood)
            .unwrap();
        let changed_market = change(&market, before, berth, Command::BuyFood).market;
        assert!(matches!(
            pending.checked_change(&changed_market, before, Some(berth)),
            Err(Error::StalePublication)
        ));
        assert_eq!(market.quote(MarketSite::Avenor).stock, 128);
    }

    #[test]
    fn ordinary_sale_caps_money_and_refuses_stock_or_sequence_overflow() {
        let mut market = StationMarkets::new_commander(101).unwrap();
        let berth = facts(MarketSite::Avenor);
        let mut rich = account();
        rich.food_tonnes = 1;
        rich.credits_tenths = u32::MAX - 100;
        let sold = change(&market, rich, berth, Command::SellFood);
        assert_eq!(sold.account_after.credits_tenths, u32::MAX);
        assert_eq!(
            sold.event,
            Event::SoldFood {
                site: MarketSite::Avenor,
                bid_tenths: 208,
                credited_tenths: 100
            }
        );
        market.data.quotes[0].stock = MAX_MARKET_QUANTITY;
        let before = market.data;
        assert!(matches!(
            market.prepare(rich, Some(berth), Command::SellFood),
            Err(Error::QuantityLimit)
        ));
        assert_eq!(market.data, before);
        rich.revision = u64::MAX;
        assert!(matches!(
            market.prepare(rich, Some(berth), Command::RefillFuel),
            Err(Error::SequenceExhausted)
        ));
        market.data.revision = u64::MAX;
        assert!(matches!(
            market.prepare(account(), Some(berth), Command::RefillFuel),
            Err(Error::SequenceExhausted)
        ));
    }
}

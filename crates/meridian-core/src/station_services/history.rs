//! Retained player service receipts, independent of the secret journal and final wallet.
use super::{CommodityId, Event, FUEL_PRICE_TENTHS, MarketSite, StationMarkets, record_accounting};
use crate::flight::{AccountEffect, AccountEvent};
use crate::save::{MAX_SAVE_BYTES, Reader, SaveError, Writer};
pub(super) const MAX_RECEIPTS: usize = MAX_SAVE_BYTES / 25;
#[derive(Debug, Clone, Copy)]
pub(super) struct Receipt {
    pub(super) revision: u64,
    pub(super) event: Event,
}
impl StationMarkets {
    pub(crate) fn cargo_mass_events(&self) -> Vec<(u64, i16)> {
        self.receipts
            .iter()
            .filter_map(|r| {
                let (commodity, delta) = match r.event {
                    Event::BoughtFood { .. } => (CommodityId::Food, 1),
                    Event::SoldFood { .. } => (CommodityId::Food, -1),
                    Event::BoughtCommodity { commodity, .. } => (commodity, 1),
                    Event::SoldCommodity { commodity, .. } => (commodity, -1),
                    Event::Refuelled { .. } => return None,
                };
                (commodity.unit() == super::CargoUnit::Tonne).then_some((r.revision, delta))
            })
            .collect()
    }
    #[cfg(test)]
    pub(crate) fn move_cargo_revision_for_test(&mut self, from: u64, to: u64) -> bool {
        let Some(receipt) = std::sync::Arc::make_mut(&mut self.receipts)
            .iter_mut()
            .find(|receipt| receipt.revision == from)
        else {
            return false;
        };
        receipt.revision = to;
        true
    }
    pub(crate) fn account_events(&self) -> Vec<AccountEvent> {
        self.receipts
            .iter()
            .map(|r| AccountEvent {
                revision: r.revision,
                effect: match r.event {
                    Event::BoughtFood { paid_tenths, .. }
                    | Event::BoughtCommodity { paid_tenths, .. } => {
                        AccountEffect::Debit(paid_tenths)
                    }
                    Event::SoldFood {
                        bid_tenths,
                        credited_tenths,
                        ..
                    }
                    | Event::SoldCommodity {
                        bid_tenths,
                        credited_tenths,
                        ..
                    } => AccountEffect::Sale {
                        bid: bid_tenths,
                        credited: credited_tenths,
                    },
                    Event::Refuelled {
                        added_fuel_tenths,
                        paid_tenths,
                        ..
                    } => AccountEffect::Refill {
                        units: added_fuel_tenths,
                        paid: paid_tenths,
                    },
                },
            })
            .collect()
    }
    pub(super) fn validate_receipts(&self) -> Result<(), SaveError> {
        let invalid = SaveError::InvalidState("service transaction prefix");
        if self.receipts.len() > MAX_RECEIPTS || self.receipts.len() as u64 != self.data.revision {
            return Err(invalid);
        }
        let mut replay = Self::new_commander(self.data.owner)
            .map_err(|_| invalid)?
            .data;
        let mut previous = 0;
        let mut held = [0_u64; 17];
        for receipt in self.receipts.iter() {
            if receipt.revision <= previous {
                return Err(invalid);
            }
            let (site, commodity, bought, amount) = match receipt.event {
                Event::BoughtFood { site, paid_tenths } => {
                    (site, CommodityId::Food, true, paid_tenths)
                }
                Event::SoldFood {
                    site, bid_tenths, ..
                } => (site, CommodityId::Food, false, bid_tenths),
                Event::BoughtCommodity {
                    site,
                    commodity,
                    paid_tenths,
                } => (site, commodity, true, paid_tenths),
                Event::SoldCommodity {
                    site,
                    commodity,
                    bid_tenths,
                    ..
                } => (site, commodity, false, bid_tenths),
                Event::Refuelled {
                    site,
                    added_fuel_tenths,
                    paid_tenths,
                } => {
                    if added_fuel_tenths > crate::navigation::MAX_FUEL_CAPACITY
                        || paid_tenths != u32::from(added_fuel_tenths) * FUEL_PRICE_TENTHS
                    {
                        return Err(invalid);
                    }
                    record_accounting(&mut replay, site.index(), receipt.event)
                        .map_err(|_| invalid)?;
                    previous = receipt.revision;
                    continue;
                }
            };
            if commodity == CommodityId::Food
                && matches!(
                    receipt.event,
                    Event::BoughtCommodity { .. } | Event::SoldCommodity { .. }
                )
            {
                return Err(invalid);
            }
            let held = &mut held[commodity.index()];
            *held = if bought {
                held.checked_add(1)
            } else {
                held.checked_sub(1)
            }
            .ok_or(invalid)?;
            let quote = self.commodity_quote(site, commodity);
            if amount
                != if bought {
                    quote.ask_tenths
                } else {
                    quote.bid_tenths
                }
            {
                return Err(invalid);
            }
            record_accounting(&mut replay, site.index(), receipt.event).map_err(|_| invalid)?;
            previous = receipt.revision;
        }
        if replay.bought != self.data.bought
            || replay.sold != self.data.sold
            || replay.other_bought != self.data.other_bought
            || replay.other_sold != self.data.other_sold
            || replay.refills != self.data.refills
            || replay.refilled_units != self.data.refilled_units
            || replay.paid_tenths != self.data.paid_tenths
            || replay.credited_tenths != self.data.credited_tenths
        {
            return Err(invalid);
        }
        Ok(())
    }
    pub(super) fn encode_receipts(&self, w: &mut Writer) {
        w.u32(self.receipts.len() as u32);
        for r in self.receipts.iter() {
            w.u64(r.revision);
            let (tag, site, commodity, amount, credited) = match r.event {
                Event::BoughtFood { site, paid_tenths } => {
                    (0, site, CommodityId::Food, paid_tenths, 0)
                }
                Event::SoldFood {
                    site,
                    bid_tenths,
                    credited_tenths,
                } => (1, site, CommodityId::Food, bid_tenths, credited_tenths),
                Event::BoughtCommodity {
                    site,
                    commodity,
                    paid_tenths,
                } => (2, site, commodity, paid_tenths, 0),
                Event::SoldCommodity {
                    site,
                    commodity,
                    bid_tenths,
                    credited_tenths,
                } => (3, site, commodity, bid_tenths, credited_tenths),
                Event::Refuelled {
                    site,
                    added_fuel_tenths,
                    paid_tenths,
                } => (
                    4,
                    site,
                    CommodityId::Food,
                    paid_tenths,
                    u32::from(added_fuel_tenths),
                ),
            };
            w.u8(tag);
            w.u8(site.index() as u8);
            w.u8(commodity.index() as u8);
            w.u32(amount);
            w.u32(credited);
        }
    }
    pub(super) fn decode_receipts(r: &mut Reader<'_>) -> Result<Vec<Receipt>, SaveError> {
        let count = r.u32()? as usize;
        if count > MAX_RECEIPTS {
            return Err(SaveError::TooLarge);
        }
        let mut receipts = Vec::new();
        for _ in 0..count {
            let revision = r.u64()?;
            let tag = r.u8()?;
            let site = match r.u8()? {
                0 => MarketSite::Avenor,
                1 => MarketSite::Renas,
                2 => MarketSite::Lareria,
                _ => return Err(SaveError::InvalidEncoding),
            };
            let commodity =
                CommodityId::from_index(usize::from(r.u8()?)).ok_or(SaveError::InvalidEncoding)?;
            let amount = r.u32()?;
            let credited = r.u32()?;
            let event = match tag {
                0 if commodity == CommodityId::Food && credited == 0 => Event::BoughtFood {
                    site,
                    paid_tenths: amount,
                },
                1 if commodity == CommodityId::Food => Event::SoldFood {
                    site,
                    bid_tenths: amount,
                    credited_tenths: credited,
                },
                2 if credited == 0 && commodity != CommodityId::Food => Event::BoughtCommodity {
                    site,
                    commodity,
                    paid_tenths: amount,
                },
                3 if commodity != CommodityId::Food => Event::SoldCommodity {
                    site,
                    commodity,
                    bid_tenths: amount,
                    credited_tenths: credited,
                },
                4 if commodity == CommodityId::Food => Event::Refuelled {
                    site,
                    added_fuel_tenths: u16::try_from(credited)
                        .map_err(|_| SaveError::InvalidEncoding)?,
                    paid_tenths: amount,
                },
                _ => return Err(SaveError::InvalidEncoding),
            };
            receipts.push(Receipt { revision, event });
        }
        Ok(receipts)
    }
}

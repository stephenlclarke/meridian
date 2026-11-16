//! One original paid, hull-bound cargo-expansion utility. World alone creates ownership.
use crate::save::{Reader, SaveError, Writer};
use crate::station_services::{self as services, MarketSite};
mod content;
pub use content::{Definition, StationOffer, definition};

pub const STANDARD_HOLD_TONNES: u16 = 92;
pub const EXPANDED_HOLD_TONNES: u16 = 127;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemId {
    owner: u64,
    serial: u8,
}
impl ItemId {
    pub fn owner_seed(self) -> u64 {
        self.owner
    }
    pub fn serial(self) -> u8 {
        self.serial
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    CargoExpansion,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    PurchaseCargoBay,
    FitCargoBay(ItemId),
    UnfitCargoBay(ItemId),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidContent,
    InvalidState,
    Unavailable,
    AlreadyOwned,
    NotOwned,
    WrongItem,
    AlreadyFitted,
    AlreadyStored,
    HoldFull,
    InsufficientFunds,
    SequenceExhausted,
    Service(services::Error),
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "equipment action refused: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedCargoBay {
    pub id: ItemId,
    pub bound_hull: u64,
    pub slot: Slot,
    pub fitted: bool,
    pub paid_tenths: u32,
    pub purchase_site: MarketSite,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    pub definition: Definition,
    pub owned: Option<OwnedCargoBay>,
    pub hold_capacity_tonnes: u16,
    pub used_tonnes: u16,
    pub equipment_units_used: u16,
    pub utility_mounts_used: u16,
    pub current_offer: Option<StationOffer>,
    pub service_available: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Purchased { item: ItemId, paid_tenths: u32 },
    Fitted { item: ItemId },
    Stored { item: ItemId },
}
impl Event {
    pub fn paid_tenths(self) -> u32 {
        match self {
            Self::Purchased { paid_tenths, .. } => paid_tenths,
            _ => 0,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Receipt {
    item: ItemId,
    owner: u64,
    hull: u64,
    transaction: u64,
    account_revision: u64,
    site: MarketSite,
    paid: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct State {
    revision: u64,
    purchase: Option<Receipt>,
    fitted: bool,
}
impl State {
    pub(crate) fn revision(self) -> u64 {
        self.revision
    }
    pub(crate) fn paid(self) -> u32 {
        self.purchase.map_or(0, |p| p.paid)
    }
    pub(crate) fn capacity(self) -> u16 {
        if self.fitted {
            EXPANDED_HOLD_TONNES
        } else {
            STANDARD_HOLD_TONNES
        }
    }
    pub(crate) fn historical_capacity(self) -> u16 {
        if self.purchase.is_some() {
            EXPANDED_HOLD_TONNES
        } else {
            STANDARD_HOLD_TONNES
        }
    }
    pub(crate) fn units_used(self) -> u16 {
        u16::from(self.fitted)
    }
    pub(crate) fn owned(self) -> Option<OwnedCargoBay> {
        self.purchase.map(|p| OwnedCargoBay {
            id: p.item,
            bound_hull: p.hull,
            slot: Slot::CargoExpansion,
            fitted: self.fitted,
            paid_tenths: p.paid,
            purchase_site: p.site,
        })
    }
    pub(crate) fn validate(
        self,
        owner: u64,
        hull: u64,
        account_revision: u64,
    ) -> Result<(), Error> {
        let Some(p) = self.purchase else {
            return if self.revision == 0 && !self.fitted {
                Ok(())
            } else {
                Err(Error::InvalidState)
            };
        };
        let declared = definition()?;
        if p.owner != owner
            || p.hull != hull
            || p.item != (ItemId { owner, serial: 1 })
            || p.transaction != 1
            || p.account_revision == 0
            || p.account_revision > account_revision
            || p.paid != declared.price_tenths
            || !declared.offer(p.site).purchase_available
            || self.revision == 0
            || self.fitted != (self.revision % 2 == 1)
            || u128::from(p.account_revision) + u128::from(self.revision - 1)
                > u128::from(account_revision)
            || self.units_used() > declared.equipment_units_capacity
            || self.units_used() > declared.utility_mounts_capacity
        {
            return Err(Error::InvalidState);
        }
        Ok(())
    }
    /// Return a candidate only. The real account publishes payment, item and fit together.
    pub(crate) fn prepare(
        self,
        owner: u64,
        hull: u64,
        account_revision: u64,
        used_tonnes: u16,
        site: MarketSite,
        command: Command,
    ) -> Result<(Self, Event), Error> {
        self.validate(owner, hull, account_revision)?;
        let declared = definition()?;
        let mut next = self;
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        let event = match command {
            Command::PurchaseCargoBay => {
                if self.purchase.is_some() {
                    return Err(Error::AlreadyOwned);
                }
                if !declared.offer(site).purchase_available {
                    return Err(Error::Unavailable);
                }
                let item = ItemId { owner, serial: 1 };
                next.purchase = Some(Receipt {
                    item,
                    owner,
                    hull,
                    transaction: 1,
                    account_revision: account_revision
                        .checked_add(1)
                        .ok_or(Error::SequenceExhausted)?,
                    site,
                    paid: declared.price_tenths,
                });
                next.fitted = true;
                Event::Purchased {
                    item,
                    paid_tenths: declared.price_tenths,
                }
            }
            Command::FitCargoBay(id) => {
                self.require_item(id)?;
                if self.fitted {
                    return Err(Error::AlreadyFitted);
                }
                next.fitted = true;
                Event::Fitted { item: id }
            }
            Command::UnfitCargoBay(id) => {
                self.require_item(id)?;
                if !self.fitted {
                    return Err(Error::AlreadyStored);
                }
                if used_tonnes > STANDARD_HOLD_TONNES {
                    return Err(Error::HoldFull);
                }
                next.fitted = false;
                Event::Stored { item: id }
            }
        };
        Ok((next, event))
    }
    fn require_item(self, id: ItemId) -> Result<(), Error> {
        let owned = self.purchase.ok_or(Error::NotOwned)?;
        if owned.item != id {
            return Err(Error::WrongItem);
        }
        Ok(())
    }
    pub(crate) fn encode(self, w: &mut Writer) {
        w.u64(self.revision);
        w.option(self.purchase.as_ref(), |w, p| {
            w.u64(p.item.owner);
            w.u8(p.item.serial);
            w.u64(p.owner);
            w.u64(p.hull);
            w.u64(p.transaction);
            w.u64(p.account_revision);
            w.u16(p.site.system());
            w.u32(p.paid);
        });
        w.bool(self.fitted);
    }
    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            revision: r.u64()?,
            purchase: r.option(|r| {
                Ok(Receipt {
                    item: ItemId {
                        owner: r.u64()?,
                        serial: r.u8()?,
                    },
                    owner: r.u64()?,
                    hull: r.u64()?,
                    transaction: r.u64()?,
                    account_revision: r.u64()?,
                    site: match r.u16()? {
                        7 => MarketSite::Avenor,
                        185 => MarketSite::Renas,
                        _ => return Err(SaveError::InvalidEncoding),
                    },
                    paid: r.u32()?,
                })
            })?,
            fitted: r.bool()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authentic_paid_state_keeps_hull_identity_and_usage_through_reversible_fit() {
        let (paid, event) = State::default()
            .prepare(
                101,
                102,
                0,
                0,
                MarketSite::Avenor,
                Command::PurchaseCargoBay,
            )
            .unwrap_or_else(|e| panic!("{e}"));
        let Event::Purchased { item, .. } = event else {
            panic!("expected purchase");
        };
        assert_eq!(paid.capacity(), 127);
        assert_eq!(paid.units_used(), 1);
        let (stored, _) = paid
            .prepare(
                101,
                102,
                1,
                92,
                MarketSite::Renas,
                Command::UnfitCargoBay(item),
            )
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(stored.capacity(), 92);
        assert_eq!(stored.historical_capacity(), 127);
        assert_eq!(stored.units_used(), 0);
        assert_eq!(stored.owned().map(|o| o.id), Some(item));
        assert_eq!(stored.validate(101, 999, 2), Err(Error::InvalidState));
        let (refitted, _) = stored
            .prepare(
                101,
                102,
                2,
                92,
                MarketSite::Avenor,
                Command::FitCargoBay(item),
            )
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(refitted.owned().map(|o| o.id), Some(item));
        assert_eq!(refitted.paid(), 28000);
        assert_eq!(refitted.validate(101, 102, 3), Ok(()));
        assert_eq!(refitted.validate(101, 102, 2), Err(Error::InvalidState));
    }
    #[test]
    fn foreign_item_fit_capacity_and_checked_revision_exhaustion_leave_source_unchanged() {
        let (mut paid, _) = State::default()
            .prepare(
                101,
                102,
                0,
                0,
                MarketSite::Avenor,
                Command::PurchaseCargoBay,
            )
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            paid.prepare(
                101,
                102,
                1,
                93,
                MarketSite::Avenor,
                Command::UnfitCargoBay(ItemId {
                    owner: 101,
                    serial: 1
                })
            ),
            Err(Error::HoldFull)
        );
        assert_eq!(
            paid.prepare(
                101,
                102,
                1,
                0,
                MarketSite::Avenor,
                Command::UnfitCargoBay(ItemId {
                    owner: 999,
                    serial: 1
                })
            ),
            Err(Error::WrongItem)
        );
        paid.revision = u64::MAX; // Internal arithmetic boundary, not a public authority fixture.
        let before = paid;
        assert_eq!(
            paid.prepare(
                101,
                102,
                u64::MAX,
                0,
                MarketSite::Avenor,
                Command::UnfitCargoBay(ItemId {
                    owner: 101,
                    serial: 1
                })
            ),
            Err(Error::SequenceExhausted)
        );
        assert_eq!(paid, before);
    }
}

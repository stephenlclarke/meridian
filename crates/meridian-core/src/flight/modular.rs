//! Paid modular laser custody. Effects live in the actual weapon/work owners, not menu rows.
use super::{FlightMode, World};
use crate::{
    freight::g02::CommanderAccount,
    navigation::DockContext,
    save::{Reader, SaveError, Writer},
};
use core::fmt;
pub const MINING_LASER_PRICE_TENTHS: u32 = 33_000;
pub const MINING_LASER_REQUIRED_TL: u8 = 6;
const INVALID: SaveError = SaveError::InvalidState("modular equipment ownership or fitting");
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleKind {
    PulseLaser,
    MiningLaser,
}
impl ModuleKind {
    pub fn family(self) -> &'static str {
        "laser"
    }
    pub fn node(self) -> &'static str {
        self.name()
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::PulseLaser => "Pulse Laser",
            Self::MiningLaser => "Mining Laser",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleSlot {
    FrontLaser,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleProvenance {
    StandardBirth,
    Paid {
        transaction: u32,
        dock: DockContext,
        tick: u64,
        account_revision: u64,
        paid_tenths: u32,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedModule {
    pub id: u64,
    pub kind: ModuleKind,
    pub slot: Option<ModuleSlot>,
    pub provenance: ModuleProvenance,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModularEquipmentSnapshot {
    pub items: Vec<OwnedModule>,
    pub fit_revision: u64,
    pub mining_offer: Option<ModuleOffer>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleOffer {
    pub kind: ModuleKind,
    pub price_tenths: u32,
    pub required_technology: u8,
    pub technology: u8,
    pub available: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleAction {
    PurchaseMiningLaser,
    Store { item: u64 },
    Install { item: u64, replacing: Option<u64> },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleError {
    NoCommander,
    StaleReview,
    Paused,
    BerthRequired,
    MissingItem,
    StaleOccupant,
    NoChange,
    Capacity,
    InsufficientCredits,
    SequenceExhausted,
    Save(SaveError),
}
impl fmt::Display for ModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Equipment refused: {self:?}")
    }
}
impl std::error::Error for ModuleError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleReview {
    action: ModuleAction,
    dock: DockContext,
    tick: u64,
    epoch: u64,
    revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Event {
    action: ModuleAction,
    item: u64,
    dock: DockContext,
    tick: u64,
    revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct State {
    events: Vec<Event>,
}
pub struct PreparedModule<'a> {
    world: &'a mut World,
    state: State,
    account: CommanderAccount,
    bytes: Vec<u8>,
    item: u64,
}
impl PreparedModule<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn item(&self) -> u64 {
        self.item
    }
    pub fn publish_with<E>(self, writer: impl FnOnce(&[u8]) -> Result<(), E>) -> Result<u64, E> {
        writer(&self.bytes)?;
        Ok(self.commit())
    }
    pub fn commit(self) -> u64 {
        self.world.commander_account = Some(self.account);
        self.world.modular = self.state;
        self.item
    }
}
/// An immutable readout, not a public work capability. Root mints MC1's opaque authority from
/// a fresh read of this actual fit and the same account revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FittedMiningTool {
    pub owner: u64,
    pub hull: u64,
    pub account_revision: u64,
    pub epoch: u64,
    pub item: u64,
    pub damage: u16,
    pub range: u16,
    pub energy_cost_hundredths: u16,
    pub heat_cost_fifths: u16,
    pub cooldown_ticks: u8,
}
#[derive(Default)]
pub(super) struct Ledger {
    pub(super) items: Vec<OwnedModule>,
    pub(super) paid: u32,
    pub(super) revision: u64,
}
impl Ledger {
    fn front(&self) -> Option<OwnedModule> {
        self.items
            .iter()
            .find(|i| i.slot == Some(ModuleSlot::FrontLaser))
            .copied()
    }
    fn apply(&mut self, event: Event, transaction: u32) -> Result<(), ModuleError> {
        match event.action {
            ModuleAction::PurchaseMiningLaser => {
                // Original per-node repeatable ownership bound, including stored examples.
                if self
                    .items
                    .iter()
                    .filter(|i| i.kind == ModuleKind::MiningLaser)
                    .count()
                    >= 5
                {
                    return Err(ModuleError::Capacity);
                }
                for i in &mut self.items {
                    i.slot = None;
                }
                self.items.push(OwnedModule {
                    id: event.item,
                    kind: ModuleKind::MiningLaser,
                    slot: Some(ModuleSlot::FrontLaser),
                    provenance: ModuleProvenance::Paid {
                        transaction,
                        dock: event.dock,
                        tick: event.tick,
                        account_revision: event.revision,
                        paid_tenths: MINING_LASER_PRICE_TENTHS,
                    },
                });
                self.paid = self
                    .paid
                    .checked_add(MINING_LASER_PRICE_TENTHS)
                    .ok_or(ModuleError::SequenceExhausted)?;
            }
            ModuleAction::Store { item } => {
                let i = self
                    .items
                    .iter_mut()
                    .find(|i| i.id == item)
                    .ok_or(ModuleError::MissingItem)?;
                if i.slot.is_none() {
                    return Err(ModuleError::NoChange);
                }
                i.slot = None;
            }
            ModuleAction::Install { item, replacing } => {
                let i = self
                    .items
                    .iter()
                    .position(|i| i.id == item)
                    .ok_or(ModuleError::MissingItem)?;
                if self.items[i].slot.is_some() {
                    return Err(ModuleError::NoChange);
                }
                if self.front().map(|i| i.id) != replacing {
                    return Err(ModuleError::StaleOccupant);
                }
                for i in &mut self.items {
                    i.slot = None;
                }
                self.items[i].slot = Some(ModuleSlot::FrontLaser);
            }
        }
        self.revision += 1;
        Ok(())
    }
}
impl State {
    /// Read-only historical actual item/fit proof for a retained work emission.
    pub(super) fn mining_was_fitted(&self, hit: &crate::mining::MiningStroke) -> bool {
        let mut ledger = Ledger {
            items: vec![OwnedModule {
                id: match hit.owner.owner.checked_add(7) {
                    Some(id) => id,
                    None => return false,
                },
                kind: ModuleKind::PulseLaser,
                slot: Some(ModuleSlot::FrontLaser),
                provenance: ModuleProvenance::StandardBirth,
            }],
            ..Ledger::default()
        };
        let mut transaction = 0_u32;
        for e in self.events.iter().filter(|e| e.tick <= hit.world_tick) {
            if e.dock.owner().seed() != hit.owner.owner
                || e.dock.ship().ordinal() != hit.owner.hull
                || e.dock.visit() >= hit.owner.visit
                || e.revision > hit.account_revision
            {
                return false;
            }
            if e.action == ModuleAction::PurchaseMiningLaser {
                transaction += 1;
            }
            if ledger.apply(*e, transaction).is_err() {
                return false;
            }
        }
        ledger.front().is_some_and(|i| {
            i.id == hit.item
                && i.kind == ModuleKind::MiningLaser
                && matches!(i.provenance, ModuleProvenance::Paid { .. })
        })
    }
    pub(super) fn paid_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.action == ModuleAction::PurchaseMiningLaser)
            .count()
    }
    pub(super) fn ledger(
        &self,
        world: &World,
        account: &CommanderAccount,
    ) -> Result<Ledger, SaveError> {
        let owner = world.combat_identity().ok_or(INVALID)?.owner;
        let mut ledger = Ledger {
            items: vec![OwnedModule {
                id: owner.checked_add(7).ok_or(INVALID)?,
                kind: ModuleKind::PulseLaser,
                slot: Some(ModuleSlot::FrontLaser),
                provenance: ModuleProvenance::StandardBirth,
            }],
            ..Ledger::default()
        };
        let mut transaction = 0_u32;
        let mut previous: Option<Event> = None;
        for e in &self.events {
            super::ammunition::validate_berth_event(
                world,
                account,
                e.dock,
                e.tick,
                e.revision,
                previous.map(|p| (p.dock.sequence(), p.tick, p.revision)),
            )?;
            if e.action == ModuleAction::PurchaseMiningLaser {
                transaction = transaction.checked_add(1).ok_or(INVALID)?;
                // Ammo owns the preceding 100000 paid identity positions. This domain never
                // aliases a born item or a finite missile receipt on the same owner.
                if transaction > 100000
                    || e.item
                        != owner
                            .checked_add(100010 + u64::from(transaction))
                            .ok_or(INVALID)?
                {
                    return Err(INVALID);
                }
            } else if match e.action {
                ModuleAction::Store { item } | ModuleAction::Install { item, .. } => item != e.item,
                _ => false,
            } {
                return Err(INVALID);
            }
            ledger.apply(*e, transaction).map_err(|_| INVALID)?;
            previous = Some(*e);
        }
        let pulse = account
            .starter_items()
            .ok_or(INVALID)?
            .iter()
            .find(|i| i.id() == owner + 7)
            .copied()
            .ok_or(INVALID)?;
        let fitted = ledger
            .front()
            .is_some_and(|i| i.kind == ModuleKind::PulseLaser);
        if pulse.fitted_hull() != fitted.then_some(world.combat_identity().ok_or(INVALID)?.hull) {
            return Err(INVALID);
        }
        if ledger.revision != account.modular_revision() {
            return Err(INVALID);
        }
        Ok(ledger)
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.u32(self.events.len() as u32);
        for e in &self.events {
            match e.action {
                ModuleAction::PurchaseMiningLaser => w.u8(0),
                ModuleAction::Store { .. } => w.u8(1),
                ModuleAction::Install { replacing, .. } => {
                    w.u8(2);
                    w.option(replacing.as_ref(), |w, i| w.u64(*i));
                }
            }
            w.u64(e.item);
            e.dock.encode_save(w);
            w.u64(e.tick);
            w.u64(e.revision);
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let count = r.u32()? as usize;
        if count > crate::save::MAX_SAVE_BYTES / 64 {
            return Err(SaveError::TooLarge);
        }
        let mut events = Vec::new();
        for _ in 0..count {
            let tag = r.u8()?;
            let replacing = if tag == 2 {
                r.option(|r| r.u64())?
            } else {
                None
            };
            let item = r.u64()?;
            let dock = DockContext::decode_save(r)?;
            let tick = r.u64()?;
            let revision = r.u64()?;
            let action = match tag {
                0 => ModuleAction::PurchaseMiningLaser,
                1 => ModuleAction::Store { item },
                2 => ModuleAction::Install { item, replacing },
                _ => return Err(SaveError::InvalidEncoding),
            };
            events.push(Event {
                action,
                item,
                dock,
                tick,
                revision,
            });
        }
        Ok(Self { events })
    }
}
impl World {
    pub fn modular_equipment_snapshot(&self) -> Option<ModularEquipmentSnapshot> {
        let account = self.commander_account.as_ref()?;
        let ledger = self.modular.ledger(self, account).ok()?;
        let mining_offer = self
            .current_dock_context()
            .and_then(|d| super::services::service_site(d.system()))
            .and_then(|site| crate::equipment::definition().ok().map(|d| d.offer(site)))
            .map(|offer| ModuleOffer {
                kind: ModuleKind::MiningLaser,
                price_tenths: MINING_LASER_PRICE_TENTHS,
                required_technology: MINING_LASER_REQUIRED_TL,
                technology: offer.technology,
                available: self
                    .review_module(ModuleAction::PurchaseMiningLaser, self.state.tick)
                    .is_ok(),
            });
        Some(ModularEquipmentSnapshot {
            items: ledger.items,
            fit_revision: ledger.revision,
            mining_offer,
        })
    }
    pub(super) fn modular_front(&self) -> Option<OwnedModule> {
        self.modular
            .ledger(self, self.commander_account.as_ref()?)
            .ok()?
            .front()
    }
    pub fn fitted_mining_tool(&self) -> Option<FittedMiningTool> {
        let account = self.commander_account.as_ref()?;
        let item = self.modular_front().filter(|i| {
            i.kind == ModuleKind::MiningLaser
                && matches!(i.provenance, ModuleProvenance::Paid { .. })
        })?;
        let owner = self.combat_identity()?;
        Some(FittedMiningTool {
            owner: owner.owner,
            hull: owner.hull,
            account_revision: account.revision(),
            epoch: self.freight_session.as_ref()?.epoch(),
            item: item.id,
            damage: 50,
            range: 6000,
            energy_cost_hundredths: 100,
            heat_cost_fifths: 50,
            cooldown_ticks: 18,
        })
    }
    /// The actual mining scene/cargo owner calls this only after sealed ray/source admission.
    /// A resource candidate confers no extraction, target, cash or fitting authority.
    pub(super) fn prepare_mining_resources(
        &self,
        item: u64,
        tick: u64,
    ) -> Result<super::weapons::PulseResources, super::FireError> {
        use super::FireError;
        if tick != self.state.tick {
            return Err(FireError::StaleCommand);
        }
        if self.is_paused() {
            return Err(FireError::Paused);
        }
        if self.state.mode != FlightMode::Flight {
            return Err(FireError::NotInFlight);
        }
        if self
            .fitted_mining_tool()
            .is_none_or(|tool| tool.item != item)
        {
            return Err(FireError::NoCompatibleMiningTool);
        }
        self.pulse.prepare_mining_discharge()
    }
    pub(crate) fn paid_fitted_mining_item(&self) -> Option<u64> {
        self.fitted_mining_tool().map(|tool| tool.item)
    }
    pub fn mining_resources_ready(&self, tick: u64) -> Result<(), super::FireError> {
        let item = self
            .paid_fitted_mining_item()
            .ok_or(super::FireError::NoCompatibleMiningTool)?;
        self.prepare_mining_resources(item, tick).map(|_| ())
    }
    pub fn review_module(
        &self,
        action: ModuleAction,
        tick: u64,
    ) -> Result<ModuleReview, ModuleError> {
        if tick != self.state.tick {
            return Err(ModuleError::StaleReview);
        }
        if self.is_paused() {
            return Err(ModuleError::Paused);
        }
        let account = self
            .commander_account
            .as_ref()
            .ok_or(ModuleError::NoCommander)?;
        let facts = self
            .current_service_facts(tick)
            .map_err(|_| ModuleError::BerthRequired)?;
        let dock = self
            .current_dock_context()
            .ok_or(ModuleError::BerthRequired)?;
        if self.state.mode != FlightMode::Docked
            || self.combat_snapshot().is_none_or(|s| s.player.destroyed)
        {
            return Err(ModuleError::BerthRequired);
        }
        if action == ModuleAction::PurchaseMiningLaser {
            if crate::equipment::definition()
                .map_err(|_| ModuleError::BerthRequired)?
                .offer(facts.site)
                .technology
                < MINING_LASER_REQUIRED_TL
            {
                return Err(ModuleError::BerthRequired);
            }
            if account.credits_tenths() < MINING_LASER_PRICE_TENTHS {
                return Err(ModuleError::InsufficientCredits);
            }
        }
        let mut ledger = self
            .modular
            .ledger(self, account)
            .map_err(ModuleError::Save)?;
        ledger.apply(
            Event {
                action,
                item: 0,
                dock,
                tick,
                revision: account.revision(),
            },
            0,
        )?;
        let ammo = self
            .ammunition
            .ledger(self, account)
            .map_err(ModuleError::Save)?;
        if ammo.items.iter().filter(|i| !i.consumed).count()
            + 4
            + usize::from(account.equipment_state().owned().is_some())
            + ledger.items.len()
            - 1
            > 256
        {
            return Err(ModuleError::Capacity);
        }
        Ok(ModuleReview {
            action,
            dock,
            tick,
            epoch: self
                .freight_session
                .as_ref()
                .ok_or(ModuleError::NoCommander)?
                .epoch(),
            revision: account.revision(),
        })
    }
    pub fn prepare_module(
        &mut self,
        review: ModuleReview,
        tick: u64,
    ) -> Result<PreparedModule<'_>, ModuleError> {
        if self.review_module(review.action, tick)? != review {
            return Err(ModuleError::StaleReview);
        }
        let current = self
            .commander_account
            .as_ref()
            .ok_or(ModuleError::NoCommander)?;
        let item = match review.action {
            ModuleAction::PurchaseMiningLaser => review
                .dock
                .owner()
                .seed()
                .checked_add(100011 + self.modular.paid_count() as u64)
                .ok_or(ModuleError::SequenceExhausted)?,
            ModuleAction::Store { item } | ModuleAction::Install { item, .. } => item,
        };
        let paid = if review.action == ModuleAction::PurchaseMiningLaser {
            MINING_LASER_PRICE_TENTHS
        } else {
            0
        };
        let pulse_fitted = matches!(review.action, ModuleAction::Install { item, .. } if item == review.dock.owner().seed() + 7);
        let account = current
            .prepare_module(paid, pulse_fitted)
            .map_err(|_| ModuleError::SequenceExhausted)?;
        let mut state = self.modular.clone();
        state.events.push(Event {
            action: review.action,
            item,
            dock: review.dock,
            tick,
            revision: account.revision(),
        });
        let bytes = self
            .save_bytes_with_modules(&account, &state)
            .map_err(ModuleError::Save)?;
        Ok(PreparedModule {
            world: self,
            state,
            account,
            bytes,
            item,
        })
    }
}

#[cfg(test)]
mod tests;

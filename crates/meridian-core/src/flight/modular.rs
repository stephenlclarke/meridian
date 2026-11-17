//! Paid modular equipment custody. Effects live in the actual weapon/work owners, not menu rows.
use super::{FlightMode, World};
use crate::{
    freight::g02::CommanderAccount,
    navigation::DockContext,
    save::{Reader, SaveError, Writer},
};
use core::fmt;
mod scoops;
pub use scoops::{FittedFuelScoops, FuelScoopsSnapshot};
pub const MINING_LASER_PRICE_TENTHS: u32 = 33_000;
pub const MINING_LASER_REQUIRED_TL: u8 = 6;
const INVALID: SaveError = SaveError::InvalidState("modular equipment ownership or fitting");
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleKind {
    PulseLaser,
    MiningLaser,
    BeamLaser,
    MilitaryLaser,
    NavalLaser,
    BasicEnergySystem,
    ExtraEnergyUnit,
    NavalEnergyUnit,
    EliteEnergyUnit,
    FuelScoops,
    BasicNavigation,
    IntermediateNavigation,
    AdvancedNavigation,
}
impl ModuleKind {
    pub fn family(self) -> &'static str {
        match self {
            Self::PulseLaser
            | Self::MiningLaser
            | Self::BeamLaser
            | Self::MilitaryLaser
            | Self::NavalLaser => "laser",
            Self::FuelScoops => "utility-fuelScoops",
            Self::BasicNavigation | Self::IntermediateNavigation | Self::AdvancedNavigation => {
                "navigation"
            }
            _ => "energy",
        }
    }
    pub fn node(self) -> &'static str {
        match self {
            Self::BasicNavigation => "Navigation: Basic",
            Self::IntermediateNavigation => "Navigation: Advanced",
            Self::AdvancedNavigation => "Navigation: Military",
            _ => self.name(),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::PulseLaser => "Pulse Laser",
            Self::MiningLaser => "Mining Laser",
            Self::FuelScoops => "Fuel Scoops",
            Self::BeamLaser => "Beam Laser",
            Self::MilitaryLaser => "Military Laser",
            Self::NavalLaser => "Naval Laser",
            Self::BasicEnergySystem => "Basic Energy System",
            Self::ExtraEnergyUnit => "Extra Energy Unit",
            Self::NavalEnergyUnit => "Naval Energy Unit",
            Self::EliteEnergyUnit => "Elite Energy Unit",
            Self::BasicNavigation => "Basic Navigation Computer",
            Self::IntermediateNavigation => "Intermediate Navigation Module",
            Self::AdvancedNavigation => "Advanced Navigation Module",
        }
    }
    pub fn laser_profile(self) -> Option<super::OrdinaryWeapon> {
        super::OrdinaryWeapon::from_module(self)
    }
    pub fn slot(self) -> ModuleSlot {
        match self {
            Self::PulseLaser
            | Self::MiningLaser
            | Self::BeamLaser
            | Self::MilitaryLaser
            | Self::NavalLaser => ModuleSlot::FrontLaser,
            Self::FuelScoops => ModuleSlot::UtilityFuelScoops,
            Self::BasicNavigation | Self::IntermediateNavigation | Self::AdvancedNavigation => {
                ModuleSlot::Navigation
            }
            _ => ModuleSlot::Energy,
        }
    }
    pub fn energy_recharge_multiplier(self) -> Option<u16> {
        match self {
            Self::BasicEnergySystem => Some(1),
            Self::ExtraEnergyUnit => Some(2),
            Self::NavalEnergyUnit => Some(3),
            Self::EliteEnergyUnit => Some(4),
            _ => None,
        }
    }
    pub fn navigation_reach_ly(self) -> Option<u8> {
        match self {
            Self::BasicNavigation => Some(7),
            Self::IntermediateNavigation => Some(14),
            Self::AdvancedNavigation => Some(21),
            _ => None,
        }
    }
    pub fn equipment_units(self) -> u16 {
        match self {
            Self::ExtraEnergyUnit => 2,
            Self::NavalEnergyUnit => 3,
            Self::EliteEnergyUnit => 4,
            _ => 0,
        }
    }
    fn shop(self) -> Option<(u32, u8)> {
        match self {
            Self::FuelScoops => Some((scoops::PRICE_TENTHS, scoops::REQUIRED_TL)),
            Self::PulseLaser => Some((14500, 3)),
            Self::MiningLaser => Some((MINING_LASER_PRICE_TENTHS, MINING_LASER_REQUIRED_TL)),
            Self::BeamLaser => Some((47000, 7)),
            Self::MilitaryLaser => Some((164000, 14)),
            Self::NavalLaser => None,
            Self::ExtraEnergyUnit => Some((72000, 9)),
            Self::NavalEnergyUnit => Some((196000, 17)),
            Self::EliteEnergyUnit => Some((135000, 14)),
            Self::BasicEnergySystem | Self::BasicNavigation => None,
            Self::IntermediateNavigation => Some((30_000, 8)),
            Self::AdvancedNavigation => Some((90_000, 11)),
        }
    }
    fn prerequisite(self) -> Option<Self> {
        match self {
            Self::ExtraEnergyUnit => Some(Self::BasicEnergySystem),
            Self::NavalEnergyUnit => Some(Self::ExtraEnergyUnit),
            Self::EliteEnergyUnit => Some(Self::NavalEnergyUnit),
            Self::IntermediateNavigation => Some(Self::BasicNavigation),
            Self::AdvancedNavigation => Some(Self::IntermediateNavigation),
            _ => None,
        }
    }
    fn maximum_owned(self) -> usize {
        match self {
            Self::PulseLaser | Self::BeamLaser | Self::MilitaryLaser | Self::NavalLaser => 4,
            Self::MiningLaser => 5,
            _ => 1,
        }
    }
    fn tag(self) -> u8 {
        match self {
            Self::PulseLaser => 0,
            Self::MiningLaser => 1,
            Self::BasicEnergySystem => 2,
            Self::ExtraEnergyUnit => 3,
            Self::NavalEnergyUnit => 4,
            Self::EliteEnergyUnit => 5,
            Self::BeamLaser => 6,
            Self::MilitaryLaser => 7,
            Self::NavalLaser => 8,
            Self::FuelScoops => 9,
            Self::BasicNavigation => 10,
            Self::IntermediateNavigation => 11,
            Self::AdvancedNavigation => 12,
        }
    }
    fn from_tag(tag: u8) -> Result<Self, SaveError> {
        match tag {
            0 => Ok(Self::PulseLaser),
            1 => Ok(Self::MiningLaser),
            2 => Ok(Self::BasicEnergySystem),
            3 => Ok(Self::ExtraEnergyUnit),
            4 => Ok(Self::NavalEnergyUnit),
            5 => Ok(Self::EliteEnergyUnit),
            6 => Ok(Self::BeamLaser),
            7 => Ok(Self::MilitaryLaser),
            8 => Ok(Self::NavalLaser),
            9 => Ok(Self::FuelScoops),
            10 => Ok(Self::BasicNavigation),
            11 => Ok(Self::IntermediateNavigation),
            12 => Ok(Self::AdvancedNavigation),
            _ => Err(SaveError::InvalidEncoding),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleSlot {
    FrontLaser,
    Energy,
    UtilityFuelScoops,
    Navigation,
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
    pub offers: Vec<ModuleOffer>,
    pub equipment_units_used: u16,
    pub equipment_units_capacity: u16,
    pub utility_mounts_used: u16,
    pub utility_mounts_capacity: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleOffer {
    pub kind: ModuleKind,
    pub price_tenths: u32,
    pub required_technology: u8,
    pub technology: u8,
    pub available: bool,
    pub slot: ModuleSlot,
    pub equipment_units: u16,
    pub prerequisite: Option<ModuleKind>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleAction {
    PurchaseMiningLaser,
    Purchase { kind: ModuleKind },
    PurchaseFuelScoops,
    Store { item: u64 },
    Install { item: u64, replacing: Option<u64> },
}
impl ModuleAction {
    fn purchase_kind(self) -> Option<ModuleKind> {
        match self {
            Self::PurchaseFuelScoops => Some(ModuleKind::FuelScoops),
            Self::PurchaseMiningLaser => Some(ModuleKind::MiningLaser),
            Self::Purchase { kind } => Some(kind),
            _ => None,
        }
    }
}
fn validate_purchase(kind: ModuleKind, technology: u8, credits: u32) -> Result<(), ModuleError> {
    let (price, required) = kind.shop().ok_or(ModuleError::NotForSale)?;
    if technology < required {
        return Err(if kind == ModuleKind::FuelScoops {
            ModuleError::BerthRequired
        } else {
            ModuleError::TechnologyUnavailable
        });
    }
    if credits < price {
        return Err(ModuleError::InsufficientCredits);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleError {
    NoCommander,
    StaleReview,
    Paused,
    BerthRequired,
    MissingItem,
    NotForSale,
    TechnologyUnavailable,
    MissingPrerequisite,
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
    // Dock visit minus dock sequence counts completed jumps before fittings at that berth.
    extended_jumps: u64,
    last_event_jump: u64,
}
impl Ledger {
    fn born(owner: u64) -> Result<Self, SaveError> {
        Ok(Self {
            items: vec![
                OwnedModule {
                    id: owner.checked_add(7).ok_or(INVALID)?,
                    kind: ModuleKind::PulseLaser,
                    slot: Some(ModuleSlot::FrontLaser),
                    provenance: ModuleProvenance::StandardBirth,
                },
                OwnedModule {
                    id: owner.checked_add(5).ok_or(INVALID)?,
                    kind: ModuleKind::BasicEnergySystem,
                    slot: Some(ModuleSlot::Energy),
                    provenance: ModuleProvenance::StandardBirth,
                },
                OwnedModule {
                    id: owner.checked_add(4).ok_or(INVALID)?,
                    kind: ModuleKind::BasicNavigation,
                    slot: Some(ModuleSlot::Navigation),
                    provenance: ModuleProvenance::StandardBirth,
                },
            ],
            ..Self::default()
        })
    }
    fn fitted(&self, slot: ModuleSlot) -> Option<OwnedModule> {
        self.items.iter().find(|i| i.slot == Some(slot)).copied()
    }
    fn front(&self) -> Option<OwnedModule> {
        self.fitted(ModuleSlot::FrontLaser)
    }
    pub(super) fn jump_reach_ly(&self) -> f64 {
        let native = crate::navigation::STOCK_JUMP_REACH_LY;
        self.fitted(ModuleSlot::Navigation)
            .and_then(|item| item.kind.navigation_reach_ly())
            .map_or(native, |reach| f64::from(reach).max(native))
    }
    fn record_navigation_interval(&mut self, jumps: u64) -> Result<(), SaveError> {
        let interval = jumps.checked_sub(self.last_event_jump).ok_or(INVALID)?;
        if self.jump_reach_ly() > crate::navigation::STOCK_JUMP_REACH_LY {
            self.extended_jumps += interval;
        }
        self.last_event_jump = jumps;
        Ok(())
    }
    pub(super) fn max_jump_fuel_spent(&self) -> u128 {
        let extended = u128::from(self.extended_jumps);
        let stock = u128::from(self.last_event_jump) - extended;
        stock * 90 + extended * u128::from(crate::navigation::FUEL_CAPACITY)
    }
    fn fitted_units(&self) -> u16 {
        self.items
            .iter()
            .filter(|i| i.slot.is_some())
            .map(|i| i.kind.equipment_units())
            .sum()
    }
    fn install(&mut self, index: usize, replacing: Option<u64>) -> Result<(), ModuleError> {
        let slot = self.items[index].kind.slot();
        if self.fitted(slot).map(|i| i.id) != replacing {
            return Err(ModuleError::StaleOccupant);
        }
        for item in &mut self.items {
            if item.slot == Some(slot) {
                item.slot = None;
            }
        }
        self.items[index].slot = Some(slot);
        Ok(())
    }
    fn apply(&mut self, event: Event, transaction: u32) -> Result<(), ModuleError> {
        if let Some(kind) = event.action.purchase_kind() {
            let price = kind.shop().ok_or(ModuleError::NotForSale)?.0;
            if self.items.iter().filter(|i| i.kind == kind).count() >= kind.maximum_owned() {
                return Err(ModuleError::Capacity);
            }
            if kind
                .prerequisite()
                .is_some_and(|required| !self.items.iter().any(|i| i.kind == required))
            {
                return Err(ModuleError::MissingPrerequisite);
            }
            let replacing = self.fitted(kind.slot()).map(|i| i.id);
            self.items.push(OwnedModule {
                id: event.item,
                kind,
                slot: None,
                provenance: ModuleProvenance::Paid {
                    transaction,
                    dock: event.dock,
                    tick: event.tick,
                    account_revision: event.revision,
                    paid_tenths: price,
                },
            });
            self.install(self.items.len() - 1, replacing)?;
            self.paid = self
                .paid
                .checked_add(price)
                .ok_or(ModuleError::SequenceExhausted)?;
        } else {
            match event.action {
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
                    let index = self
                        .items
                        .iter()
                        .position(|i| i.id == item)
                        .ok_or(ModuleError::MissingItem)?;
                    if self.items[index].slot.is_some() {
                        return Err(ModuleError::NoChange);
                    }
                    self.install(index, replacing)?;
                }
                ModuleAction::PurchaseMiningLaser
                | ModuleAction::PurchaseFuelScoops
                | ModuleAction::Purchase { .. } => {
                    return Err(ModuleError::NotForSale);
                }
            }
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(ModuleError::SequenceExhausted)?;
        Ok(())
    }
}

impl State {
    fn purchase_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.action.purchase_kind().is_some())
            .count()
    }

    /// Read-only historical actual item/fit proof for a retained work emission.
    pub(super) fn mining_was_fitted(&self, hit: &crate::mining::MiningStroke) -> bool {
        let Ok(mut ledger) = Ledger::born(hit.owner.owner) else {
            return false;
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
            if e.action.purchase_kind().is_some() {
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
            .filter(|e| e.action.purchase_kind().is_some())
            .count()
    }
    pub(super) fn ledger(
        &self,
        world: &World,
        account: &CommanderAccount,
    ) -> Result<Ledger, SaveError> {
        let owner = world.combat_identity().ok_or(INVALID)?.owner;
        let jumps = world.navigation.ok_or(INVALID)?.jump_sequence();
        let mut ledger = Ledger::born(owner)?;
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
            if let Some(kind) = e.action.purchase_kind() {
                let (_, required) = kind.shop().ok_or(INVALID)?;
                let site = super::services::service_site(e.dock.system()).ok_or(INVALID)?;
                if crate::equipment::definition()
                    .map_err(|_| INVALID)?
                    .offer(site)
                    .technology
                    < required
                {
                    return Err(INVALID);
                }
                transaction = transaction.checked_add(1).ok_or(INVALID)?;
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
            let prior_jumps = e.dock.visit() - e.dock.sequence();
            if prior_jumps > jumps {
                return Err(INVALID);
            }
            ledger.record_navigation_interval(prior_jumps)?;
            ledger.apply(*e, transaction).map_err(|_| INVALID)?;
            previous = Some(*e);
        }
        ledger.record_navigation_interval(jumps)?;
        for (offset, slot, component) in [
            (
                7,
                ModuleSlot::FrontLaser,
                super::StarterComponent::FrontPulseLaser,
            ),
            (5, ModuleSlot::Energy, super::StarterComponent::BasicEnergy),
            (
                4,
                ModuleSlot::Navigation,
                super::StarterComponent::BasicNavigation,
            ),
        ] {
            let born = account
                .starter_items()
                .ok_or(INVALID)?
                .iter()
                .find(|i| i.id() == owner + offset)
                .copied()
                .ok_or(INVALID)?;
            if born.owner() != owner || !born.is_standard_birth() || born.component() != component {
                return Err(INVALID);
            }
            let fitted = ledger.fitted(slot).is_some_and(|i| i.id == owner + offset);
            if born.fitted_hull() != fitted.then_some(world.combat_identity().ok_or(INVALID)?.hull)
            {
                return Err(INVALID);
            }
        }
        let budget = crate::equipment::definition().map_err(|_| INVALID)?;
        if ledger.fitted_units() + account.equipment_state().units_used()
            > budget.equipment_units_capacity
        {
            return Err(INVALID);
        }
        scoops::validate_fit(&ledger.items, account).map_err(|_| INVALID)?;
        if ledger.revision != account.modular_revision() {
            return Err(INVALID);
        }
        Ok(ledger)
    }
    fn ledger_at(&self, owner: u64, tick: u64, visit: u64) -> Option<Ledger> {
        let mut ledger = Ledger::born(owner).ok()?;
        let mut paid = 0;
        for e in self.events.iter().filter(|e| e.tick <= tick) {
            if e.dock.visit() >= visit {
                return None;
            }
            if e.action.purchase_kind().is_some() {
                paid += 1;
            }
            ledger.apply(*e, paid).ok()?;
        }
        Some(ledger)
    }
    fn item_at(&self, owner: u64, item: u64, tick: u64, visit: u64) -> Option<OwnedModule> {
        self.ledger_at(owner, tick, visit)?
            .items
            .into_iter()
            .find(|i| i.id == item)
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.u32(self.events.len() as u32);
        for e in &self.events {
            match e.action {
                ModuleAction::PurchaseMiningLaser => w.u8(0),
                ModuleAction::Purchase { kind } => {
                    w.u8(3);
                    w.u8(kind.tag());
                }
                ModuleAction::PurchaseFuelScoops => w.u8(16),
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
    pub(super) fn decode(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        if !matches!(version, 13 | 15 | 16 | crate::save::SAVE_VERSION) {
            return Err(SaveError::UnsupportedVersion);
        }
        let count = r.u32()? as usize;
        if count > crate::save::MAX_SAVE_BYTES / 64 {
            return Err(SaveError::TooLarge);
        }
        let mut events = Vec::new();
        for _ in 0..count {
            let tag = r.u8()?;
            let kind = if tag == 3 && version >= 16 {
                let kind = ModuleKind::from_tag(r.u8()?)?;
                if version < 17 && kind.navigation_reach_ly().is_some() {
                    return Err(SaveError::InvalidEncoding);
                }
                Some(kind)
            } else {
                None
            };
            let replacing = if tag == 2 {
                r.option(|r| r.u64())?
            } else {
                None
            };
            let item = r.u64()?;
            let dock = DockContext::decode_save(r)?;
            let tick = r.u64()?;
            let revision = r.u64()?;
            if version < 17
                && matches!(tag, 1 | 2)
                && dock.owner().seed().checked_add(4) == Some(item)
            {
                return Err(SaveError::InvalidEncoding);
            }
            let action = match tag {
                0 => ModuleAction::PurchaseMiningLaser,
                16 if version >= 16 => ModuleAction::PurchaseFuelScoops,
                1 => ModuleAction::Store { item },
                2 => ModuleAction::Install { item, replacing },
                3 if version >= 16 => ModuleAction::Purchase {
                    kind: kind.ok_or(SaveError::InvalidEncoding)?,
                },
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
        let offers = self.module_offers();
        let mining_offer = offers
            .iter()
            .find(|o| o.kind == ModuleKind::MiningLaser)
            .copied();
        let definition = crate::equipment::definition().ok()?;
        Some(ModularEquipmentSnapshot {
            equipment_units_used: ledger.fitted_units() + account.equipment_state().units_used(),
            equipment_units_capacity: definition.equipment_units_capacity,
            utility_mounts_used: account.equipment_state().units_used(),
            utility_mounts_capacity: definition.utility_mounts_capacity,
            items: ledger.items,
            fit_revision: ledger.revision,
            mining_offer,
            offers,
        })
    }
    pub fn module_offers(&self) -> Vec<ModuleOffer> {
        let Some(site) = self
            .current_dock_context()
            .and_then(|d| super::services::service_site(d.system()))
        else {
            return Vec::new();
        };
        let Ok(base) = crate::equipment::definition() else {
            return Vec::new();
        };
        let technology = base.offer(site).technology;
        [
            ModuleKind::PulseLaser,
            ModuleKind::MiningLaser,
            ModuleKind::BeamLaser,
            ModuleKind::MilitaryLaser,
            ModuleKind::ExtraEnergyUnit,
            ModuleKind::NavalEnergyUnit,
            ModuleKind::EliteEnergyUnit,
            ModuleKind::IntermediateNavigation,
            ModuleKind::AdvancedNavigation,
        ]
        .into_iter()
        .filter_map(|kind| {
            let (price_tenths, required_technology) = kind.shop()?;
            Some(ModuleOffer {
                kind,
                price_tenths,
                required_technology,
                technology,
                available: self
                    .review_module(ModuleAction::Purchase { kind }, self.state.tick)
                    .is_ok(),
                slot: kind.slot(),
                equipment_units: kind.equipment_units(),
                prerequisite: kind.prerequisite(),
            })
        })
        .collect()
    }
    pub(super) fn owned_front_laser_at(
        &self,
        item: u64,
        tick: u64,
        owner: super::CombatIdentity,
    ) -> Option<super::OrdinaryWeapon> {
        let current = self.combat_identity()?;
        if current.owner != owner.owner || current.hull != owner.hull || owner.visit > current.visit
        {
            return None;
        }
        let item = self.modular.item_at(owner.owner, item, tick, owner.visit)?;
        if item.slot != Some(ModuleSlot::FrontLaser) {
            return None;
        }
        super::OrdinaryWeapon::from_module(item.kind)
    }
    pub(super) fn energy_recharge_hundredths(&self) -> u16 {
        let multiplier = self
            .commander_account
            .as_ref()
            .and_then(|a| self.modular.ledger(self, a).ok())
            .and_then(|l| l.fitted(ModuleSlot::Energy))
            .and_then(|i| i.kind.energy_recharge_multiplier())
            .unwrap_or(1);
        super::weapons::ENERGY_RECHARGE_HUNDREDTHS * multiplier
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
        let changes_energy = action
            .purchase_kind()
            .is_some_and(|kind| kind.slot() == ModuleSlot::Energy)
            || match action {
                ModuleAction::Store { item } | ModuleAction::Install { item, .. } => self
                    .modular
                    .ledger(self, account)
                    .map_err(ModuleError::Save)?
                    .items
                    .iter()
                    .any(|i| i.id == item && i.kind.slot() == ModuleSlot::Energy),
                _ => false,
            };
        if changes_energy && !self.damage.enabled() {
            return Err(ModuleError::Save(SaveError::InvalidState(
                "energy fitting requires resource birth provenance",
            )));
        }
        if let Some(kind) = action.purchase_kind() {
            if matches!(
                kind,
                ModuleKind::BeamLaser | ModuleKind::MilitaryLaser | ModuleKind::NavalLaser
            ) && !self.damage.enabled()
            {
                return Err(ModuleError::Save(SaveError::InvalidState(
                    "fast lasers require fresh resource history",
                )));
            }
            let technology = crate::equipment::definition()
                .map_err(|_| ModuleError::BerthRequired)?
                .offer(facts.site)
                .technology;
            validate_purchase(kind, technology, account.credits_tenths())?;
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
        scoops::validate_fit(&ledger.items, account)?;
        let ammo = self
            .ammunition
            .ledger(self, account)
            .map_err(ModuleError::Save)?;
        if ammo.items.iter().filter(|i| !i.consumed).count()
            + 4
            + usize::from(account.equipment_state().owned().is_some())
            + ledger
                .items
                .iter()
                .filter(|item| matches!(item.provenance, ModuleProvenance::Paid { .. }))
                .count()
            > 256
        {
            return Err(ModuleError::Capacity);
        }
        let budget = crate::equipment::definition().map_err(|_| ModuleError::BerthRequired)?;
        if ledger.fitted_units() + account.equipment_state().units_used()
            > budget.equipment_units_capacity
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
        let item = if review.action.purchase_kind().is_some() {
            review
                .dock
                .owner()
                .seed()
                .checked_add(100011 + self.modular.purchase_count() as u64)
                .ok_or(ModuleError::SequenceExhausted)?
        } else {
            match review.action {
                ModuleAction::Store { item } | ModuleAction::Install { item, .. } => item,
                _ => return Err(ModuleError::NotForSale),
            }
        };
        let paid = review
            .action
            .purchase_kind()
            .map(|kind| {
                kind.shop()
                    .map(|offer| offer.0)
                    .ok_or(ModuleError::NotForSale)
            })
            .transpose()?
            .unwrap_or(0);
        let revision = current
            .revision()
            .checked_add(1)
            .ok_or(ModuleError::SequenceExhausted)?;
        let event = Event {
            action: review.action,
            item,
            dock: review.dock,
            tick,
            revision,
        };
        let mut ledger = self
            .modular
            .ledger(self, current)
            .map_err(ModuleError::Save)?;
        ledger.apply(event, (self.modular.purchase_count() + 1) as u32)?;
        let owner = review.dock.owner().seed();
        let pulse_fitted = ledger.front().is_some_and(|i| i.id == owner + 7);
        let energy_fitted = ledger
            .fitted(ModuleSlot::Energy)
            .is_some_and(|i| i.id == owner + 5);
        let navigation_fitted = ledger
            .fitted(ModuleSlot::Navigation)
            .is_some_and(|item| item.id == owner + 4);
        let account = current
            .prepare_module(paid, pulse_fitted, energy_fitted, navigation_fitted)
            .map_err(|_| ModuleError::SequenceExhausted)?;
        let mut state = self.modular.clone();
        state.events.push(event);
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
mod navigation_tests;
#[cfg(test)]
mod tests;

impl State {
    pub(super) fn energy_changes(&self, world: &World) -> Result<Vec<(u64, u16)>, SaveError> {
        let owner = world.combat_identity().ok_or(INVALID)?.owner;
        let mut ledger = Ledger::born(owner)?;
        let mut changes = Vec::new();
        let mut paid = 0;
        let mut before = 7;
        for event in &self.events {
            if event.action.purchase_kind().is_some() {
                paid += 1;
            }
            ledger.apply(*event, paid).map_err(|_| INVALID)?;
            let rate = ledger
                .fitted(ModuleSlot::Energy)
                .and_then(|i| i.kind.energy_recharge_multiplier())
                .unwrap_or(1)
                * 7;
            if rate != before {
                let clock = world
                    .damage
                    .berth_clock(event.dock.sequence())
                    .ok_or(INVALID)?;
                changes.push((clock, rate));
            }
            before = rate;
        }
        Ok(changes)
    }
    pub(super) fn front_at_physical(&self, world: &World, clock: u64) -> Option<OwnedModule> {
        let mut ledger = Ledger::born(world.combat_identity()?.owner).ok()?;
        let mut paid = 0;
        for event in &self.events {
            let at = world.damage.berth_clock(event.dock.sequence())?;
            if at > clock {
                break;
            }
            if event.action.purchase_kind().is_some() {
                paid += 1;
            }
            ledger.apply(*event, paid).ok()?;
        }
        ledger.front()
    }
}

impl World {
    fn historical_front_at_tick(&self, tick: u64) -> Option<OwnedModule> {
        let mut ledger = Ledger::born(self.combat_identity()?.owner).ok()?;
        let mut paid = 0;
        for event in self
            .modular
            .events
            .iter()
            .filter(|event| event.tick <= tick)
        {
            if event.action.purchase_kind().is_some() {
                paid += 1;
            }
            ledger.apply(*event, paid).ok()?;
        }
        ledger.front()
    }
    pub(super) fn front_weapon_at_tick(
        &self,
        item: u64,
        tick: u64,
    ) -> Option<super::OrdinaryWeapon> {
        let fitted = self.historical_front_at_tick(tick)?;
        if item != fitted.id {
            return None;
        }
        super::OrdinaryWeapon::from_module(fitted.kind)
    }
    pub(super) fn front_cadence_at_tick(&self, tick: u64) -> Option<u8> {
        super::OrdinaryWeapon::from_module(self.historical_front_at_tick(tick)?.kind)
            .map(super::OrdinaryWeapon::cooldown_ticks)
    }
}

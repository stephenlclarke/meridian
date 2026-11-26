//! Paid modular equipment custody. Effects live in the actual weapon/work owners, not menu rows.
use super::{FlightMode, ShieldGrade, World};
use crate::{
    equipment::TargetingTier,
    freight::g02::CommanderAccount,
    navigation::{DockContext, DriveGrade, NavigationFitChange, NavigationGrade},
    save::{Reader, SaveError, Writer},
};
use core::fmt;
mod cabins;
mod fuel_history;
mod history;
mod salvage;
pub(super) use cabins::add_cabin_mass;
pub use cabins::{
    CABIN_TONNES, CabinGrade, CabinStatus, PASSAGE_CLUE_ID, PASSAGE_REWARD_TENTHS, PassageStatus,
    PassageTerminalReceipt, PassengerAction, PassengerIdentity, PassengerJourneySnapshot,
};
mod scoops;
mod shields;
pub(super) use shields::ResourceFitChange;
mod work;
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
    SalvageCutter,
    Ecm,
    FuelStorage,
    BasicNavigation,
    IntermediateNavigation,
    AdvancedNavigation,
    Targeting(TargetingTier),
    MiningTargeting,
    SalvageTargeting,
    Drive(DriveGrade),
    Shield(ShieldGrade),
    Cabin(CabinGrade),
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
            Self::SalvageCutter => "utility-salvageCutter",
            Self::Ecm => "utility-ecm",
            Self::FuelStorage => "fuel-storage",
            Self::BasicNavigation | Self::IntermediateNavigation | Self::AdvancedNavigation => {
                "navigation"
            }
            Self::Targeting(_) => "targeting-core",
            Self::MiningTargeting => "targeting-mining",
            Self::SalvageTargeting => "targeting-salvage",
            Self::Drive(_) => "hyperspace-drive",
            Self::Shield(_) => "shields",
            Self::Cabin(_) => "passenger-cabins",
            _ => "energy",
        }
    }
    pub fn node(self) -> &'static str {
        match self {
            Self::BasicNavigation => "Navigation: Basic",
            Self::IntermediateNavigation => "Navigation: Advanced",
            Self::AdvancedNavigation => "Navigation: Military",
            Self::Targeting(tier) => match tier {
                TargetingTier::Basic => "Targeting: Basic",
                TargetingTier::Advanced => "Targeting: Advanced",
                TargetingTier::Docking => "Targeting: Docking",
                TargetingTier::Identification => "Targeting: Identification",
                TargetingTier::Pirate => "Targeting: Pirate",
                TargetingTier::Military => "Targeting: Military",
            },
            Self::MiningTargeting => "Targeting: Mining Module",
            Self::SalvageTargeting => "Targeting: Salvage Module",
            Self::Shield(grade) => grade.node(),
            _ => self.name(),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::PulseLaser => "Pulse Laser",
            Self::MiningLaser => "Mining Laser",
            Self::FuelScoops => "Fuel Scoops",
            Self::SalvageCutter => "Salvage Cutter",
            Self::Ecm => "E.C.M. System",
            Self::FuelStorage => "Fuel Storage Unit",
            Self::MiningTargeting => "Mining Targeting Module",
            Self::SalvageTargeting => "Salvage Targeting Module",
            Self::Drive(DriveGrade::Basic) => "Basic Hyperspace Drive",
            Self::Drive(DriveGrade::Intermediate) => "Intermediate Hyperspace Drive",
            Self::Drive(DriveGrade::Advanced) => "Advanced Hyperspace Drive",
            Self::Shield(grade) => grade.name(),
            Self::Cabin(grade) => grade.name(),
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
            Self::Targeting(tier) => match tier {
                TargetingTier::Basic => "Basic Targeting Module",
                TargetingTier::Advanced => "Advanced Targeting Module",
                TargetingTier::Docking => "Docking Targeting Module",
                TargetingTier::Identification => "Identification Targeting Module",
                TargetingTier::Pirate => "Pirate Targeting Module",
                TargetingTier::Military => "Military Targeting Module",
            },
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
            Self::SalvageCutter => ModuleSlot::UtilitySalvageCutter,
            Self::Ecm => ModuleSlot::UtilityEcm,
            Self::FuelStorage => ModuleSlot::FuelStorage(0),
            Self::BasicNavigation | Self::IntermediateNavigation | Self::AdvancedNavigation => {
                ModuleSlot::Navigation
            }
            Self::Targeting(_) => ModuleSlot::Targeting,
            Self::MiningTargeting => ModuleSlot::MiningTargeting,
            Self::SalvageTargeting => ModuleSlot::SalvageTargeting,
            Self::Drive(_) => ModuleSlot::Drive,
            Self::Shield(_) => ModuleSlot::Shield,
            Self::Cabin(_) => ModuleSlot::PassengerCabin,
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
    fn navigation_grade(self) -> Option<NavigationGrade> {
        match self {
            Self::BasicNavigation => Some(NavigationGrade::Basic),
            Self::IntermediateNavigation => Some(NavigationGrade::Intermediate),
            Self::AdvancedNavigation => Some(NavigationGrade::Advanced),
            _ => None,
        }
    }
    fn xyzzy_supported(self) -> bool {
        matches!(
            self,
            Self::NavalLaser
                | Self::EliteEnergyUnit
                | Self::MiningLaser
                | Self::AdvancedNavigation
                | Self::FuelScoops
                | Self::SalvageCutter
                | Self::Ecm
                | Self::FuelStorage
                | Self::Targeting(TargetingTier::Military)
                | Self::MiningTargeting
                | Self::SalvageTargeting
                | Self::Drive(DriveGrade::Advanced)
                | Self::Shield(_)
        )
    }
    pub fn navigation_reach_ly(self) -> Option<u8> {
        self.navigation_grade()
            .and_then(NavigationGrade::module_reach_ly)
    }
    pub fn equipment_units(self) -> u16 {
        match self {
            Self::ExtraEnergyUnit => 2,
            Self::NavalEnergyUnit => 3,
            Self::EliteEnergyUnit => 4,
            Self::Targeting(_) | Self::MiningTargeting | Self::SalvageTargeting => 1,
            Self::Drive(DriveGrade::Intermediate | DriveGrade::Advanced) => 1,
            Self::FuelStorage | Self::Ecm | Self::SalvageCutter => 1,
            Self::Shield(grade) => grade.equipment_units(),
            Self::Cabin(_) => 1,
            _ => 0,
        }
    }
    fn shop(self) -> Option<(u32, u8)> {
        match self {
            Self::FuelScoops => Some((scoops::PRICE_TENTHS, scoops::REQUIRED_TL)),
            Self::SalvageCutter => Some((25_000, 7)),
            Self::Ecm => Some((21_000, 5)),
            Self::FuelStorage => Some((20_000, 1)),
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
            Self::Targeting(tier) => Some((tier.price_tenths(), tier.required_technology())),
            // Frozen original shop, in credit tenths and displayed technology levels.
            Self::MiningTargeting => Some((20_000, 6)),
            Self::SalvageTargeting => Some((40_000, 8)),
            Self::Drive(DriveGrade::Basic) => None,
            Self::Drive(DriveGrade::Intermediate) => Some((50_000, 10)),
            Self::Drive(DriveGrade::Advanced) => Some((150_000, 13)),
            Self::Shield(grade) => grade.ordinary_offer(),
            Self::Cabin(grade) => Some((grade.price_tenths(), 1)),
        }
    }
    fn prerequisite(self) -> Option<Self> {
        match self {
            Self::ExtraEnergyUnit => Some(Self::BasicEnergySystem),
            Self::NavalEnergyUnit => Some(Self::ExtraEnergyUnit),
            Self::EliteEnergyUnit => Some(Self::NavalEnergyUnit),
            Self::IntermediateNavigation => Some(Self::BasicNavigation),
            Self::AdvancedNavigation => Some(Self::IntermediateNavigation),
            Self::Targeting(tier) => tier.predecessor().map(Self::Targeting),
            Self::MiningTargeting | Self::SalvageTargeting => {
                Some(Self::Targeting(TargetingTier::Basic))
            }
            Self::Drive(DriveGrade::Intermediate) => Some(Self::Drive(DriveGrade::Basic)),
            Self::Drive(DriveGrade::Advanced) => Some(Self::Drive(DriveGrade::Intermediate)),
            Self::Shield(grade) => grade.predecessor().map(Self::Shield),
            _ => None,
        }
    }
    fn maximum_owned(self) -> usize {
        match self {
            Self::PulseLaser | Self::BeamLaser | Self::MilitaryLaser | Self::NavalLaser => 4,
            Self::MiningLaser => 5,
            Self::FuelStorage => usize::from(crate::navigation::MAX_FUEL_STORAGE_UNITS),
            // The shared inventory limit is stricter than the legacy ownership cap.
            Self::Cabin(_) => 256,
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
            Self::Targeting(tier) => 12 + tier as u8,
            Self::MiningTargeting => 19,
            Self::SalvageTargeting => 20,
            Self::Drive(DriveGrade::Basic) => 21,
            Self::Drive(DriveGrade::Intermediate) => 22,
            Self::Drive(DriveGrade::Advanced) => 23,
            Self::Shield(grade) => 24 + grade as u8,
            Self::FuelStorage => 31,
            Self::Ecm => 32,
            Self::SalvageCutter => 33,
            Self::Cabin(grade) => 40 + grade as u8,
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
            13..=18 => Ok(Self::Targeting(TargetingTier::ALL[usize::from(tag - 13)])),
            19 => Ok(Self::MiningTargeting),
            20 => Ok(Self::SalvageTargeting),
            21 => Ok(Self::Drive(DriveGrade::Basic)),
            22 => Ok(Self::Drive(DriveGrade::Intermediate)),
            23 => Ok(Self::Drive(DriveGrade::Advanced)),
            24..=30 => Ok(Self::Shield(ShieldGrade::from_tag(tag - 24)?)),
            31 => Ok(Self::FuelStorage),
            32 => Ok(Self::Ecm),
            33 => Ok(Self::SalvageCutter),
            40..=45 => Ok(Self::Cabin(CabinGrade::ALL[usize::from(tag - 40)])),
            _ => Err(SaveError::InvalidEncoding),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleSlot {
    FrontLaser,
    Energy,
    UtilityFuelScoops,
    UtilitySalvageCutter,
    UtilityEcm,
    FuelStorage(u8),
    Navigation,
    Targeting,
    MiningTargeting,
    SalvageTargeting,
    Drive,
    Shield,
    PassengerCabin,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleProvenance {
    StandardBirth,
    XYZzy {
        grant: u32,
        dock: DockContext,
        tick: u64,
        account_revision: u64,
    },
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
    Purchase {
        kind: ModuleKind,
    },
    PurchaseFuelScoops,
    /// Reserved for the explicit XYZZY outfit; ordinary reviews refuse it.
    GrantStoredSalvageCutter,
    Store {
        item: u64,
    },
    Install {
        item: u64,
        replacing: Option<u64>,
    },
    Passenger(PassengerAction),
    #[doc(hidden)]
    GrantStoredMiningLaser,
    #[doc(hidden)]
    FlightLaserExchange {
        item: u64,
        replacing: u64,
        visit: u64,
        clock: u64,
        emission: u64,
        restoring: bool,
    },
}
impl ModuleAction {
    fn purchase_kind(self) -> Option<ModuleKind> {
        match self {
            Self::PurchaseFuelScoops => Some(ModuleKind::FuelScoops),
            Self::GrantStoredSalvageCutter => Some(ModuleKind::SalvageCutter),
            Self::PurchaseMiningLaser | Self::GrantStoredMiningLaser => {
                Some(ModuleKind::MiningLaser)
            }
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
    LowerTargetingGrade,
    DriveRequired,
    ShieldRequired,
    FuelCapacityOccupied,
    CabinOccupied,
    PassageConsent,
    PassageClosed,
    PassageHistory,
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
    xyzzy: bool,
    action: ModuleAction,
    item: u64,
    dock: DockContext,
    tick: u64,
    revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct State {
    events: Vec<Event>,
    cargo_fit_history: Vec<fuel_history::CargoFit>,
    history: history::History,
    salvage: salvage::State,
}
pub struct PreparedModule<'a> {
    world: &'a mut World,
    state: State,
    account: CommanderAccount,
    combat: super::combat::State,
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
        self.world.combat = self.combat;
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
    pub(super) navigation_changes: Vec<NavigationFitChange>,
    test_tank_items: Vec<u64>,
    passage: cabins::Passage,
    auto_return: Option<u64>,
}
impl Ledger {
    pub(super) fn passenger_totals(&self) -> (u32, u64) {
        (self.passage.reward(), self.passage.events)
    }
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
                OwnedModule {
                    id: owner.checked_add(6).ok_or(INVALID)?,
                    kind: ModuleKind::Drive(DriveGrade::Basic),
                    slot: Some(ModuleSlot::Drive),
                    provenance: ModuleProvenance::StandardBirth,
                },
                OwnedModule {
                    id: owner.checked_add(3).ok_or(INVALID)?,
                    kind: ModuleKind::Shield(ShieldGrade::Basic),
                    slot: Some(ModuleSlot::Shield),
                    provenance: ModuleProvenance::StandardBirth,
                },
            ],
            ..Self::default()
        })
    }
    pub(super) fn fitted(&self, slot: ModuleSlot) -> Option<OwnedModule> {
        self.items.iter().find(|i| i.slot == Some(slot)).copied()
    }
    fn front(&self) -> Option<OwnedModule> {
        self.fitted(ModuleSlot::FrontLaser)
    }
    pub(super) fn navigation_grade(&self) -> NavigationGrade {
        self.fitted(ModuleSlot::Navigation)
            .and_then(|item| item.kind.navigation_grade())
            .unwrap_or(NavigationGrade::Empty)
    }
    pub(super) fn jump_reach_ly(&self) -> f64 {
        self.navigation_grade().reach_ly()
    }
    pub(super) fn drive(&self) -> Option<(u64, DriveGrade)> {
        let item = self.fitted(ModuleSlot::Drive)?;
        match item.kind {
            ModuleKind::Drive(grade) => Some((item.id, grade)),
            _ => None,
        }
    }
    fn fitted_units(&self) -> u16 {
        self.items
            .iter()
            .filter(|i| i.slot.is_some())
            .map(|i| i.kind.equipment_units())
            .sum()
    }
    fn awake_cabin_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| cabins::fitted_cabin(item).is_some_and(CabinGrade::supports_awake))
            .count()
    }
    pub(super) fn fuel_storage_count(&self) -> u8 {
        self.items
            .iter()
            .filter(|item| matches!(item.slot, Some(ModuleSlot::FuelStorage(_))))
            .count() as u8
    }
    fn mark_test_tanks(&mut self) {
        self.test_tank_items = self
            .items
            .iter()
            .filter(|item| matches!(item.slot, Some(ModuleSlot::FuelStorage(_))))
            .map(|item| item.id)
            .collect();
    }
    pub(super) fn test_tank_count(&self) -> u16 {
        self.test_tank_items.len() as u16
    }
    fn budget_usage(
        &self,
        account: &CommanderAccount,
        emp_mounts: u16,
    ) -> Result<(u16, u16), ModuleError> {
        Ok((
            self.fitted_units() + account.equipment_state().units_used(),
            scoops::mounts_used(&self.items, account, emp_mounts)?,
        ))
    }
    fn validate_budgets(
        &self,
        account: &CommanderAccount,
        emp_mounts: u16,
        allowance: u16,
    ) -> Result<(), ModuleError> {
        let definition = crate::equipment::definition().map_err(|_| ModuleError::BerthRequired)?;
        let (units, mounts) = self.budget_usage(account, emp_mounts)?;
        if units.saturating_sub(allowance) > definition.equipment_units_capacity
            || mounts.saturating_sub(allowance) > definition.utility_mounts_capacity
        {
            return Err(ModuleError::Capacity);
        }
        Ok(())
    }
    fn install(&mut self, index: usize, replacing: Option<u64>) -> Result<(), ModuleError> {
        let slot = if self.items[index].kind == ModuleKind::FuelStorage {
            if replacing.is_some() {
                return Err(ModuleError::StaleOccupant);
            }
            (0..crate::navigation::MAX_FUEL_STORAGE_UNITS)
                .map(ModuleSlot::FuelStorage)
                .find(|&slot| self.fitted(slot).is_none())
                .ok_or(ModuleError::Capacity)?
        } else {
            self.items[index].kind.slot()
        };
        // A cabin is a separate physical module; fitting one never ejects another.
        if slot == ModuleSlot::PassengerCabin {
            if replacing.is_some() {
                return Err(ModuleError::StaleOccupant);
            }
            self.items[index].slot = Some(slot);
            return Ok(());
        }
        if matches!(
            slot,
            ModuleSlot::MiningTargeting | ModuleSlot::SalvageTargeting
        ) && self.fitted(ModuleSlot::Targeting).is_none()
        {
            return Err(ModuleError::MissingPrerequisite);
        }
        let fitted = self.fitted(slot);
        if fitted.map(|i| i.id) != replacing {
            return Err(ModuleError::StaleOccupant);
        }
        if let (
            ModuleKind::Targeting(next),
            Some(OwnedModule {
                kind: ModuleKind::Targeting(current),
                ..
            }),
        ) = (self.items[index].kind, fitted)
            && next < current
        {
            return Err(ModuleError::LowerTargetingGrade);
        }
        for item in &mut self.items {
            if item.slot == Some(slot) {
                item.slot = None;
            }
        }
        self.items[index].slot = Some(slot);
        Ok(())
    }
    fn replay(&mut self, event: Event, counters: &mut [u32; 2]) -> Result<(), ModuleError> {
        let index = usize::from(event.xyzzy);
        if event.action.purchase_kind().is_some() {
            counters[index] = counters[index]
                .checked_add(1)
                .ok_or(ModuleError::SequenceExhausted)?;
        }
        self.apply(event, counters[index])
    }
    fn prerequisites_satisfied(&self, kind: ModuleKind) -> bool {
        kind.prerequisite().is_none_or(|required| {
            self.items.iter().any(|item| {
                item.kind == required
                    && (!matches!(kind, ModuleKind::Drive(_))
                        || item.slot == Some(ModuleSlot::Drive))
            })
        })
    }
    fn apply(&mut self, event: Event, transaction: u32) -> Result<(), ModuleError> {
        if let ModuleAction::FlightLaserExchange {
            item,
            replacing,
            restoring,
            ..
        } = event.action
        {
            if event.xyzzy
                || event.item != item
                || self.front().is_none_or(|front| front.id != replacing)
            {
                return Err(ModuleError::StaleOccupant);
            }
            let index = self
                .items
                .iter()
                .position(|i| i.id == item && i.slot.is_none())
                .ok_or(ModuleError::MissingItem)?;
            let current = self.front().ok_or(ModuleError::MissingItem)?;
            let target = self.items[index];
            let combat = |kind| {
                kind != ModuleKind::MiningLaser
                    && super::OrdinaryWeapon::from_module(kind).is_some()
            };
            let adopting = !restoring
                && current.kind == ModuleKind::MiningLaser
                && combat(target.kind)
                && self.auto_return.is_none();
            if adopting {
                self.auto_return = Some(item);
            } else if !restoring
                && target.kind == ModuleKind::MiningLaser
                && combat(current.kind)
                && self.auto_return.is_none()
            {
                self.auto_return = Some(replacing);
            } else if restoring
                && current.kind == ModuleKind::MiningLaser
                && combat(target.kind)
                && self.auto_return == Some(item)
            {
                self.auto_return = None;
            } else {
                return Err(ModuleError::StaleOccupant);
            }
            if !adopting {
                self.install(index, Some(replacing))?;
            }
            self.revision = self
                .revision
                .checked_add(1)
                .ok_or(ModuleError::SequenceExhausted)?;
            return Ok(());
        }
        if matches!(
            event.action,
            ModuleAction::GrantStoredMiningLaser | ModuleAction::GrantStoredSalvageCutter
        ) && !event.xyzzy
        {
            return Err(ModuleError::NotForSale);
        }
        if let Some(kind) = event.action.purchase_kind() {
            let price = if event.xyzzy {
                0
            } else {
                kind.shop().ok_or(ModuleError::NotForSale)?.0
            };
            if self.items.iter().filter(|i| i.kind == kind).count() >= kind.maximum_owned() {
                return Err(ModuleError::Capacity);
            }
            if !event.xyzzy && !self.prerequisites_satisfied(kind) {
                return Err(ModuleError::MissingPrerequisite);
            }
            let replacing =
                if kind == ModuleKind::FuelStorage || kind.slot() == ModuleSlot::PassengerCabin {
                    None
                } else {
                    self.fitted(kind.slot()).map(|i| i.id)
                };
            self.items.push(OwnedModule {
                id: event.item,
                kind,
                slot: None,
                provenance: if event.xyzzy {
                    ModuleProvenance::XYZzy {
                        grant: transaction,
                        dock: event.dock,
                        tick: event.tick,
                        account_revision: event.revision,
                    }
                } else {
                    ModuleProvenance::Paid {
                        transaction,
                        dock: event.dock,
                        tick: event.tick,
                        account_revision: event.revision,
                        paid_tenths: price,
                    }
                },
            });
            if !matches!(
                event.action,
                ModuleAction::GrantStoredMiningLaser | ModuleAction::GrantStoredSalvageCutter
            ) {
                self.install(self.items.len() - 1, replacing)?;
                if kind.slot() == ModuleSlot::FrontLaser {
                    self.auto_return = None;
                }
            }
            self.paid = self
                .paid
                .checked_add(if event.xyzzy { 0 } else { price })
                .ok_or(ModuleError::SequenceExhausted)?;
        } else {
            match event.action {
                ModuleAction::Store { item } => {
                    if self.passage.occupied(item) {
                        return Err(ModuleError::CabinOccupied);
                    }
                    let i = self
                        .items
                        .iter_mut()
                        .find(|i| i.id == item)
                        .ok_or(ModuleError::MissingItem)?;
                    if i.slot.is_none() {
                        return Err(ModuleError::NoChange);
                    }
                    if i.slot == Some(ModuleSlot::Drive) {
                        return Err(ModuleError::DriveRequired);
                    }
                    if i.slot == Some(ModuleSlot::Shield) {
                        return Err(ModuleError::ShieldRequired);
                    }
                    if i.slot == Some(ModuleSlot::FrontLaser) {
                        self.auto_return = None;
                    }
                    i.slot = None;
                    self.test_tank_items.retain(|&id| id != item);
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
                    if self.items[index].kind.slot() == ModuleSlot::FrontLaser {
                        self.auto_return = None;
                    }
                }
                ModuleAction::PurchaseMiningLaser
                | ModuleAction::GrantStoredMiningLaser
                | ModuleAction::FlightLaserExchange { .. }
                | ModuleAction::PurchaseFuelScoops
                | ModuleAction::GrantStoredSalvageCutter
                | ModuleAction::Purchase { .. } => {
                    return Err(ModuleError::NotForSale);
                }
                ModuleAction::Passenger(action) => return self.apply_passage(event, action),
            }
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(ModuleError::SequenceExhausted)?;
        Ok(())
    }
}

fn validate_module_purchase(e: &Event, owner: u64, mut transaction: u32) -> Result<u32, SaveError> {
    if let Some(kind) = e.action.purchase_kind() {
        if !e.xyzzy {
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
        }
    } else if match e.action {
        ModuleAction::Store { item } | ModuleAction::Install { item, .. } => item != e.item,
        _ => false,
    } {
        return Err(INVALID);
    }
    Ok(transaction)
}

impl State {
    fn ledger_at_revision(&self, owner: u64, revision: u64) -> Result<Ledger, SaveError> {
        let mut ledger = Ledger::born(owner)?;
        let mut counters = [0; 2];
        for event in self
            .events
            .iter()
            .take_while(|event| event.revision <= revision)
        {
            ledger.replay(*event, &mut counters).map_err(|_| INVALID)?;
        }
        Ok(ledger)
    }
    pub(super) fn fuel_capacity_at_revision(
        &self,
        owner: u64,
        revision: u64,
    ) -> Result<u16, SaveError> {
        Ok(crate::navigation::fuel_capacity(
            self.ledger_at_revision(owner, revision)?
                .fuel_storage_count(),
        ))
    }
    pub(super) fn ecm_fitted_at_revision(
        &self,
        owner: u64,
        revision: u64,
    ) -> Result<bool, SaveError> {
        Ok(self
            .ledger_at_revision(owner, revision)?
            .fitted(ModuleSlot::UtilityEcm)
            .is_some())
    }
    pub(super) fn storage_count_possible_at_visit(
        &self,
        owner: u64,
        visit: u64,
        count: u8,
    ) -> Result<bool, SaveError> {
        let mut ledger = Ledger::born(owner)?;
        let mut counters = [0; 2];
        let mut possible = false;
        for event in self
            .events
            .iter()
            .take_while(|event| event.dock.visit() <= visit)
        {
            if event.dock.visit() == visit && ledger.fuel_storage_count() == count {
                possible = true;
            }
            ledger.replay(*event, &mut counters).map_err(|_| INVALID)?;
        }
        Ok(possible || ledger.fuel_storage_count() == count)
    }
    pub(super) fn account_events(&self, owner: u64) -> Result<Vec<super::AccountEvent>, SaveError> {
        let mut ledger = Ledger::born(owner)?;
        let mut counters = [0; 2];
        let mut events = Vec::new();
        for e in &self.events {
            let previous = ledger.fuel_storage_count();
            ledger.replay(*e, &mut counters).map_err(|_| INVALID)?;
            let count = ledger.fuel_storage_count();
            let paid = e.action.purchase_kind().map(|kind| {
                if e.xyzzy {
                    0
                } else {
                    kind.shop().map_or(0, |s| s.0)
                }
            });
            let effect = if previous != count {
                super::AccountEffect::TankFit {
                    count,
                    paid: paid.unwrap_or(0),
                }
            } else if let Some(paid) = paid {
                super::AccountEffect::Debit(paid)
            } else if let ModuleAction::Passenger(action) = e.action {
                match action {
                    PassengerAction::RequestReplies | PassengerAction::Board => {
                        super::AccountEffect::ObserveFuel(
                            cabins::receipt_fuel(e.item).ok_or(INVALID)?,
                        )
                    }
                    PassengerAction::Receive => super::AccountEffect::Reward(PASSAGE_REWARD_TENTHS),
                    PassengerAction::Cancel => super::AccountEffect::Debit(0),
                }
            } else if self.history.tracks_revision(e.revision)
                && matches!(
                    e.action,
                    ModuleAction::Store { .. } | ModuleAction::Install { .. }
                )
            {
                super::AccountEffect::Debit(0)
            } else if matches!(e.action, ModuleAction::FlightLaserExchange { .. }) {
                super::AccountEffect::Debit(0)
            } else {
                continue;
            };
            events.push(super::AccountEvent {
                revision: e.revision,
                effect,
            });
        }
        events.extend(self.cargo_account_events());
        Ok(events)
    }

    pub(super) fn passenger_occupancy(&self) -> (usize, usize) {
        let boarded = self
            .events
            .iter()
            .any(|e| e.action == ModuleAction::Passenger(PassengerAction::Board));
        let ended = self.events.iter().any(|e| {
            matches!(
                e.action,
                ModuleAction::Passenger(PassengerAction::Receive | PassengerAction::Cancel)
            )
        });
        (usize::from(boarded), usize::from(boarded && !ended))
    }

    pub(super) fn passenger_aboard_at(&self, revision: u64) -> bool {
        let mut aboard = false;
        for event in self
            .events
            .iter()
            .take_while(|event| event.revision <= revision)
        {
            match event.action {
                ModuleAction::Passenger(PassengerAction::Board) => aboard = true,
                ModuleAction::Passenger(PassengerAction::Receive | PassengerAction::Cancel) => {
                    aboard = false
                }
                _ => {}
            }
        }
        aboard
    }

    fn purchase_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| !e.xyzzy && e.action.purchase_kind().is_some())
            .count()
    }

    /// Read-only historical actual item/fit proof for a retained work emission.
    pub(super) fn mining_was_fitted(&self, hit: &crate::mining::MiningStroke) -> bool {
        let Ok(mut ledger) = Ledger::born(hit.owner.owner) else {
            return false;
        };
        let mut counters = [0; 2];
        for e in self
            .events
            .iter()
            .filter(|e| e.revision <= hit.account_revision)
        {
            if e.dock.owner().seed() != hit.owner.owner
                || e.dock.ship().ordinal() != hit.owner.hull
                || e.dock.visit() >= hit.owner.visit
                || e.tick > hit.world_tick
            {
                return false;
            }
            if ledger.replay(*e, &mut counters).is_err() {
                return false;
            }
        }
        ledger.front().is_some_and(|i| {
            i.id == hit.item
                && i.kind == ModuleKind::MiningLaser
                && matches!(
                    i.provenance,
                    ModuleProvenance::Paid { .. } | ModuleProvenance::XYZzy { .. }
                )
        })
    }
    fn grant_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.xyzzy && e.action.purchase_kind().is_some())
            .count()
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
        let mut ledger = Ledger::born(owner)?;
        let mut transaction = 0_u32;
        let mut grants = 0_u32;
        let mut previous: Option<Event> = None;
        let mut previous_exchange: Option<(u64, u64, u64)> = None;
        let budget = crate::equipment::definition().map_err(|_| INVALID)?;
        for e in &self.events {
            super::ammunition::validate_berth_event(
                world,
                account,
                e.dock,
                e.tick,
                e.revision,
                previous.map(|p| (p.dock.sequence(), p.tick, p.revision)),
            )?;
            if let ModuleAction::FlightLaserExchange {
                visit,
                clock,
                emission,
                restoring,
                ..
            } = e.action
            {
                let prior_dock = previous.map(|event| event.dock).ok_or(INVALID)?;
                let system = world.jump_journal.system_at_visit(visit, e.tick)?;
                let (jumps, arrival) = world.jump_journal.visit_origin(visit);
                let launch = visit
                    .checked_sub(1)
                    .and_then(|v| v.checked_sub(jumps))
                    .ok_or(INVALID)?;
                let berth = world.damage.berth_clock(launch).ok_or(INVALID)?;
                if e.dock != prior_dock
                    || visit <= e.dock.visit()
                    || visit > world.combat_identity().ok_or(INVALID)?.visit
                    || clock == 0
                    || clock > world.combat.physical_tick()
                    || clock > e.tick
                    || emission > world.combat.emissions
                    || (e.tick - clock)
                        > world
                            .state
                            .tick
                            .checked_sub(world.combat.physical_tick())
                            .ok_or(INVALID)?
                    || (!restoring
                        && (system != crate::navigation::SystemId::AVENOR
                            || ledger.fitted(ModuleSlot::MiningTargeting).is_none()
                            || ledger.fitted(ModuleSlot::Targeting).is_none()))
                    || arrival.map_or(clock <= berth, |arrival| clock < arrival)
                    || world
                        .damage
                        .berth_clock(launch.checked_add(1).ok_or(INVALID)?)
                        .is_some_and(|next| clock > next || (!restoring && clock == next))
                    || previous_exchange.is_some_and(
                        |(prior_visit, prior_clock, prior_emission)| {
                            visit < prior_visit || clock < prior_clock || emission < prior_emission
                        },
                    )
                {
                    return Err(INVALID);
                }
                previous_exchange = Some((visit, clock, emission));
            }
            let serial = if e.xyzzy {
                let kind = e
                    .action
                    .purchase_kind()
                    .filter(|kind| kind.xyzzy_supported())
                    .ok_or(INVALID)?;
                if kind == ModuleKind::MiningLaser
                    && e.action != ModuleAction::GrantStoredMiningLaser
                {
                    return Err(INVALID);
                }
                let authorized = match kind {
                    ModuleKind::Shield(grade) => world
                        .xyzzy
                        .authorizes_shield(grade, e.dock, e.tick, e.revision),
                    ModuleKind::FuelStorage => {
                        world.xyzzy.authorizes_max_tanks(e.dock, e.tick, e.revision)
                    }
                    ModuleKind::FuelScoops => {
                        world.xyzzy.authorizes_scoops(e.dock, e.tick, e.revision)
                    }
                    ModuleKind::Ecm => world.xyzzy.authorizes_ecm(e.dock, e.tick, e.revision),
                    _ => world.xyzzy.authorizes(e.dock, e.tick, e.revision),
                };
                if !authorized {
                    return Err(INVALID);
                }
                grants = grants.checked_add(1).ok_or(INVALID)?;
                if grants > 100000
                    || e.item
                        != owner
                            .checked_add(200010 + u64::from(grants))
                            .ok_or(INVALID)?
                {
                    return Err(INVALID);
                }
                grants
            } else {
                transaction
            };
            transaction = validate_module_purchase(e, owner, transaction)?;
            if let ModuleAction::Passenger(action) = e.action {
                world
                    .jump_journal
                    .validate_passenger_berth(e.dock, e.tick)?;
                if !self.history.owns_berth(e.dock) {
                    return Err(INVALID);
                }
                if matches!(
                    action,
                    PassengerAction::RequestReplies | PassengerAction::Board
                ) && cabins::receipt_epoch(e.item)
                    > world.freight_session.as_ref().ok_or(INVALID)?.epoch()
                {
                    return Err(INVALID);
                }
                if matches!(
                    action,
                    PassengerAction::RequestReplies | PassengerAction::Board
                ) {
                    let cargo = self.history.bay_fitted_at(e.revision).ok_or(INVALID)?;
                    let mounts = u16::from(ledger.fuel_storage_count())
                        + u16::from(ledger.fitted(ModuleSlot::UtilityFuelScoops).is_some())
                        + u16::from(ledger.fitted(ModuleSlot::UtilitySalvageCutter).is_some())
                        + u16::from(ledger.fitted(ModuleSlot::UtilityEcm).is_some())
                        + cargo
                        + world.emp.utility_mounts_at_revision(e.revision);
                    let ordinary = mounts <= budget.utility_mounts_capacity
                        && ledger.fitted_units() + cargo <= budget.equipment_units_capacity;
                    if !ordinary
                        && (action == PassengerAction::Board || ledger.awake_cabin_count() >= 2)
                    {
                        return Err(INVALID);
                    }
                }
            }
            let previous_grade = ledger.navigation_grade();
            let previous_drive = ledger.drive();
            let previous_tanks = ledger.fuel_storage_count();
            ledger
                .apply(*e, if e.xyzzy { serial } else { transaction })
                .map_err(|_| INVALID)?;
            let grade = ledger.navigation_grade();
            let drive = ledger.drive().ok_or(INVALID)?;
            let tanks = ledger.fuel_storage_count();
            if tanks > previous_tanks {
                if world.xyzzy.authorizes_max_tanks(e.dock, e.tick, e.revision) {
                    if !e.xyzzy && !matches!(e.action, ModuleAction::Install { .. }) {
                        return Err(INVALID);
                    }
                    ledger.mark_test_tanks();
                } else {
                    let budget = crate::equipment::definition().map_err(|_| INVALID)?;
                    if u16::from(tanks)
                        + u16::from(ledger.fitted(ModuleSlot::UtilityFuelScoops).is_some())
                        + u16::from(ledger.fitted(ModuleSlot::UtilitySalvageCutter).is_some())
                        + u16::from(ledger.fitted(ModuleSlot::UtilityEcm).is_some())
                        > budget.utility_mounts_capacity
                    {
                        return Err(INVALID);
                    }
                }
            }
            let allowance = if self.passenger_aboard_at(e.revision) {
                0
            } else {
                ledger.test_tank_count()
            };
            if ledger.fitted(ModuleSlot::UtilitySalvageCutter).is_some() {
                let cargo = self.known_cargo_mounts_at(account, e.revision);
                let mounts = u16::from(tanks)
                    + u16::from(ledger.fitted(ModuleSlot::UtilityFuelScoops).is_some())
                    + u16::from(ledger.fitted(ModuleSlot::UtilityEcm).is_some())
                    + 1
                    + cargo
                    + world.emp.utility_mounts_at_revision(e.revision);
                if mounts.saturating_sub(allowance) > budget.utility_mounts_capacity {
                    return Err(INVALID);
                }
            }
            if ledger.fitted_units().saturating_sub(allowance) > budget.equipment_units_capacity
                || ledger.cabin_tonnes() > crate::equipment::EXPANDED_HOLD_TONNES
            {
                return Err(INVALID);
            }
            if grade != previous_grade || Some(drive) != previous_drive || tanks != previous_tanks {
                ledger.navigation_changes.push(NavigationFitChange {
                    dock: e.dock,
                    tick: e.tick,
                    grade,
                    drive: drive.1,
                    fuel_storage_count: tanks,
                });
            }
            previous = Some(*e);
        }
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
            (6, ModuleSlot::Drive, super::StarterComponent::BasicDrive),
            (3, ModuleSlot::Shield, super::StarterComponent::BasicShield),
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
        if ledger.fuel_storage_count() != account.fuel_storage_count() {
            return Err(INVALID);
        }
        let allowance = if world.test_tank_custody_clear() && self.passenger_occupancy().1 == 0 {
            ledger.test_tank_count()
        } else {
            0
        };
        ledger
            .validate_budgets(account, world.emp.utility_mounts(), allowance)
            .map_err(|_| INVALID)?;
        if ledger.revision != account.modular_revision() {
            return Err(INVALID);
        }
        if self.cabin_tonnes().map_err(|_| INVALID)? != ledger.cabin_tonnes() {
            return Err(INVALID);
        }
        if ledger.cabin_tonnes() != 0 {
            let projection = account
                .service_projection(&world.freight_session.as_ref().ok_or(INVALID)?.state)
                .map_err(|_| INVALID)?;
            add_cabin_mass(projection, ledger.cabin_tonnes()).map_err(|_| INVALID)?;
        }
        Ok(ledger)
    }
    fn ledger_at(&self, owner: u64, tick: u64, visit: u64) -> Option<Ledger> {
        let mut ledger = Ledger::born(owner).ok()?;
        let mut transaction = 0;
        let mut grants = 0;
        for event in self.events.iter().filter(|e| e.tick <= tick) {
            if event.dock.visit() >= visit {
                return None;
            }
            if event.action.purchase_kind().is_some() {
                if event.xyzzy {
                    grants += 1;
                } else {
                    transaction += 1;
                }
            }
            ledger
                .apply(*event, if event.xyzzy { grants } else { transaction })
                .ok()?;
        }
        Some(ledger)
    }
    fn item_at(&self, owner: u64, item: u64, tick: u64, visit: u64) -> Option<OwnedModule> {
        self.ledger_at(owner, tick, visit)?
            .items
            .into_iter()
            .find(|i| i.id == item)
    }
    pub(super) fn scoop_item_at(&self, owner: u64, tick: u64, visit: u64) -> Option<OwnedModule> {
        self.ledger_at(owner, tick, visit)?
            .fitted(ModuleSlot::UtilityFuelScoops)
    }
    pub(in crate::flight) fn scoop_was_fitted_for_collection(
        &self,
        receipt: &crate::mining::Collection,
    ) -> bool {
        self.ledger_at_revision(receipt.owner.owner, receipt.revision.saturating_sub(1))
            .ok()
            .and_then(|ledger| ledger.fitted(ModuleSlot::UtilityFuelScoops))
            .is_some_and(|item| item.id == receipt.item)
            && self
                .scoop_item_at(receipt.owner.owner, receipt.world_tick, receipt.owner.visit)
                .is_some_and(|item| item.id == receipt.item)
    }
    pub(super) fn ecm_item_at(&self, owner: u64, tick: u64, visit: u64) -> Option<OwnedModule> {
        self.ledger_at(owner, tick, visit)?
            .fitted(ModuleSlot::UtilityEcm)
    }
    pub(super) fn drive_at_visit(&self, owner: u64, tick: u64, visit: u64) -> Option<DriveGrade> {
        self.ledger_at(owner, tick, visit)?
            .drive()
            .map(|(_, grade)| grade)
    }
    pub(super) fn validate_utility_test_record(
        &self,
        owner: u64,
        start: u64,
        end: u64,
        account: &CommanderAccount,
        emp: &super::emp::State,
        kind: ModuleKind,
    ) -> Result<(), SaveError> {
        let fit_at = |revision| -> Result<Ledger, SaveError> {
            let mut ledger = Ledger::born(owner)?;
            let mut counters = [0; 2];
            for event in self
                .events
                .iter()
                .take_while(|event| event.revision <= revision)
            {
                ledger.replay(*event, &mut counters).map_err(|_| INVALID)?;
            }
            Ok(ledger)
        };
        let slot = kind.slot();
        let before = fit_at(start)?.fitted(slot);
        let after_ledger = fit_at(end)?;
        let after = after_ledger.fitted(slot).ok_or(INVALID)?;
        if start == end {
            return if before == Some(after) {
                Ok(())
            } else {
                Err(INVALID)
            };
        }
        if before.is_some() || start.checked_add(1) != Some(end) {
            return Err(INVALID);
        }
        let event = self
            .events
            .iter()
            .find(|event| event.revision == end)
            .ok_or(INVALID)?;
        let budget = crate::equipment::definition().map_err(|_| INVALID)?;
        let cargo = self.known_cargo_mounts_at(account, end);
        let mounts = u16::from(after_ledger.fuel_storage_count())
            + u16::from(after_ledger.fitted(ModuleSlot::UtilityFuelScoops).is_some())
            + u16::from(
                after_ledger
                    .fitted(ModuleSlot::UtilitySalvageCutter)
                    .is_some(),
            )
            + u16::from(after_ledger.fitted(ModuleSlot::UtilityEcm).is_some())
            + cargo
            + emp.utility_mounts_at_revision(end);
        if event.item != after.id
            || mounts > budget.utility_mounts_capacity
            || after_ledger.fitted_units() + cargo > budget.equipment_units_capacity
        {
            return Err(INVALID);
        }
        match event.action {
            ModuleAction::Purchase { kind: purchased } if event.xyzzy && purchased == kind => {
                Ok(())
            }
            ModuleAction::Install {
                item,
                replacing: None,
            } if !event.xyzzy && item == after.id => Ok(()),
            _ => Err(INVALID),
        }
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        let flags = (u32::from(!self.cargo_fit_history.is_empty()) << 31)
            | (u32::from(self.history.active()) << 30)
            | (u32::from(!self.salvage.is_empty()) << 29);
        w.u32(self.events.len() as u32 | flags);
        for e in &self.events {
            match e.action {
                ModuleAction::PurchaseMiningLaser => w.u8(0),
                ModuleAction::GrantStoredMiningLaser => w.u8(19),
                ModuleAction::FlightLaserExchange {
                    replacing,
                    visit,
                    clock,
                    emission,
                    restoring,
                    ..
                } => {
                    w.u8(18);
                    for value in [replacing, visit, clock, emission] {
                        w.u64(value);
                    }
                    w.bool(restoring);
                }
                ModuleAction::Purchase { kind } => {
                    w.u8(3);
                    w.u8(kind.tag());
                }
                ModuleAction::PurchaseFuelScoops => w.u8(16),
                ModuleAction::GrantStoredSalvageCutter => w.u8(17),
                ModuleAction::Store { .. } => w.u8(1),
                ModuleAction::Install { replacing, .. } => {
                    w.u8(2);
                    w.option(replacing.as_ref(), |w, i| w.u64(*i));
                }
                ModuleAction::Passenger(action) => w.u8(match action {
                    PassengerAction::RequestReplies => 4,
                    PassengerAction::Board => 5,
                    PassengerAction::Receive => 6,
                    PassengerAction::Cancel => 7,
                }),
            }
            w.u64(e.item);
            e.dock.encode_save(w);
            w.u64(e.tick);
            w.u64(e.revision);
            w.bool(e.xyzzy);
        }
        if !self.cargo_fit_history.is_empty() {
            self.encode_cargo_history(w);
        }
        self.history.encode(w);
        if !self.salvage.is_empty() {
            self.salvage.encode(w);
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        if version != crate::save::SAVE_VERSION {
            return Err(SaveError::UnsupportedVersion);
        }
        let raw_count = r.u32()?;
        let count = (raw_count & 0x1fff_ffff) as usize;
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
            let exchange = if tag == 18 {
                Some((r.u64()?, r.u64()?, r.u64()?, r.u64()?, r.bool()?))
            } else {
                None
            };
            let item = r.u64()?;
            let dock = DockContext::decode_save(r)?;
            let tick = r.u64()?;
            let revision = r.u64()?;
            let xyzzy = r.bool()?;
            let action = match tag {
                0 => ModuleAction::PurchaseMiningLaser,
                19 => ModuleAction::GrantStoredMiningLaser,
                18 => {
                    let (replacing, visit, clock, emission, restoring) =
                        exchange.ok_or(SaveError::InvalidEncoding)?;
                    ModuleAction::FlightLaserExchange {
                        item,
                        replacing,
                        visit,
                        clock,
                        emission,
                        restoring,
                    }
                }
                16 if version >= 16 => ModuleAction::PurchaseFuelScoops,
                17 => ModuleAction::GrantStoredSalvageCutter,
                1 => ModuleAction::Store { item },
                2 => ModuleAction::Install { item, replacing },
                3 if version >= 16 => ModuleAction::Purchase {
                    kind: kind.ok_or(SaveError::InvalidEncoding)?,
                },
                4 => ModuleAction::Passenger(PassengerAction::RequestReplies),
                5 => ModuleAction::Passenger(PassengerAction::Board),
                6 => ModuleAction::Passenger(PassengerAction::Receive),
                7 => ModuleAction::Passenger(PassengerAction::Cancel),
                _ => return Err(SaveError::InvalidEncoding),
            };
            events.push(Event {
                xyzzy,
                action,
                item,
                dock,
                tick,
                revision,
            });
        }
        let cargo_fit_history = if raw_count & (1 << 31) != 0 {
            Self::decode_cargo_history(r)?
        } else {
            Vec::new()
        };
        let history = if raw_count & (1 << 30) != 0 {
            history::History::decode(r)?
        } else {
            history::History::default()
        };
        let salvage = if raw_count & (1 << 29) != 0 {
            salvage::State::decode(r)?
        } else {
            salvage::State::default()
        };
        if events
            .iter()
            .any(|event| history::requires_history(event.action))
            != history.active()
        {
            return Err(SaveError::InvalidEncoding);
        }
        Ok(Self {
            events,
            cargo_fit_history,
            history,
            salvage,
        })
    }
}
impl World {
    pub(super) fn test_tank_overfit(&self) -> bool {
        let Some(account) = self.commander_account.as_ref() else {
            return false;
        };
        self.modular.ledger(self, account).is_ok_and(|ledger| {
            ledger.test_tank_count() > 0
                && ledger
                    .validate_budgets(account, self.emp.utility_mounts(), 0)
                    .is_err()
        })
    }
    /// The actual fitted fictional inventory grade, or no installed module.
    pub fn targeting_tier(&self) -> Option<TargetingTier> {
        let account = self.commander_account.as_ref()?;
        let ledger = self.modular.ledger(self, account).ok()?;
        match ledger.fitted(ModuleSlot::Targeting)?.kind {
            ModuleKind::Targeting(tier) => Some(tier),
            _ => None,
        }
    }
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
            utility_mounts_used: scoops::mounts_used(
                &ledger.items,
                account,
                self.emp.utility_mounts(),
            )
            .ok()?,
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
            ModuleKind::FuelStorage,
            ModuleKind::SalvageCutter,
            ModuleKind::Ecm,
            ModuleKind::Drive(DriveGrade::Intermediate),
            ModuleKind::Drive(DriveGrade::Advanced),
            ModuleKind::Shield(ShieldGrade::BasicII),
            ModuleKind::Shield(ShieldGrade::Intermediate),
            ModuleKind::Shield(ShieldGrade::IntermediateII),
            ModuleKind::Shield(ShieldGrade::Advanced),
            ModuleKind::Targeting(TargetingTier::Basic),
            ModuleKind::Targeting(TargetingTier::Advanced),
            ModuleKind::Targeting(TargetingTier::Docking),
            ModuleKind::Targeting(TargetingTier::Identification),
            ModuleKind::Targeting(TargetingTier::Pirate),
            ModuleKind::Targeting(TargetingTier::Military),
            ModuleKind::MiningTargeting,
            ModuleKind::SalvageTargeting,
            ModuleKind::Cabin(CabinGrade::PackedStasis),
            ModuleKind::Cabin(CabinGrade::Economy),
            ModuleKind::Cabin(CabinGrade::Standard),
            ModuleKind::Cabin(CabinGrade::Luxury),
            ModuleKind::Cabin(CabinGrade::SingleStateroom),
            ModuleKind::Cabin(CabinGrade::DoubleStateroom),
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
                && matches!(
                    i.provenance,
                    ModuleProvenance::Paid { .. } | ModuleProvenance::XYZzy { .. }
                )
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
    fn owned_fitted_mining_item(&self) -> Option<u64> {
        self.fitted_mining_tool().map(|tool| tool.item)
    }
    pub fn mining_resources_ready(&self, tick: u64) -> Result<(), super::FireError> {
        let item = self
            .owned_fitted_mining_item()
            .ok_or(super::FireError::NoCompatibleMiningTool)?;
        self.prepare_mining_resources(item, tick).map(|_| ())
    }
    pub fn review_module(
        &self,
        action: ModuleAction,
        tick: u64,
    ) -> Result<ModuleReview, ModuleError> {
        if matches!(
            action,
            ModuleAction::GrantStoredSalvageCutter
                | ModuleAction::GrantStoredMiningLaser
                | ModuleAction::FlightLaserExchange { .. }
        ) {
            return Err(ModuleError::NotForSale);
        }
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
        let resource_slot = |slot| matches!(slot, ModuleSlot::Energy | ModuleSlot::Shield);
        let changes_resources = action
            .purchase_kind()
            .is_some_and(|kind| resource_slot(kind.slot()))
            || match action {
                ModuleAction::Store { item } | ModuleAction::Install { item, .. } => self
                    .modular
                    .ledger(self, account)
                    .map_err(ModuleError::Save)?
                    .items
                    .iter()
                    .any(|i| i.id == item && resource_slot(i.kind.slot())),
                _ => false,
            };
        if changes_resources && !self.damage.enabled() {
            return Err(ModuleError::Save(SaveError::InvalidState(
                "energy or shield fitting requires resource birth provenance",
            )));
        }
        if action.purchase_kind() == Some(ModuleKind::Ecm) && !self.damage.enabled() {
            return Err(ModuleError::Save(SaveError::InvalidState(
                "ECM fitting requires resource birth provenance",
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
        let was_outside_ordinary_budget = ledger
            .validate_budgets(account, self.emp.utility_mounts(), 0)
            .is_err();
        if history::requires_history(action) && !self.modular.history.active() {
            let mut history = history::History::default();
            history.activate(self, account, dock)?;
        }
        if matches!(action, ModuleAction::Passenger(PassengerAction::Board))
            && !self.original_service_has_capacity()
        {
            return Err(ModuleError::Capacity);
        }
        let item = match action {
            ModuleAction::Passenger(action) => cabins::receipt_data(self, action)?,
            _ => 0,
        };
        ledger.apply(
            Event {
                xyzzy: false,
                action,
                item,
                dock,
                tick,
                revision: account.revision(),
            },
            0,
        )?;
        let allowance = if was_outside_ordinary_budget
            && (matches!(action, ModuleAction::Store { .. })
                || (action == ModuleAction::Passenger(PassengerAction::RequestReplies)
                    && ledger.awake_cabin_count() < 2))
        {
            ledger.test_tank_count()
        } else {
            0
        };
        ledger.validate_budgets(account, self.emp.utility_mounts(), allowance)?;
        if account.fuel_tenths() > crate::navigation::fuel_capacity(ledger.fuel_storage_count()) {
            return Err(ModuleError::FuelCapacityOccupied);
        }
        let ammo = self
            .ammunition
            .ledger(self, account)
            .map_err(ModuleError::Save)?;
        if ammo.items.iter().filter(|i| !i.consumed).count()
            + 4
            + usize::from(account.equipment_state().owned().is_some())
            + usize::from(self.emp.owned().ok().flatten().is_some())
            + ledger
                .items
                .iter()
                .filter(|item| !matches!(item.provenance, ModuleProvenance::StandardBirth))
                .count()
            > 256
        {
            return Err(ModuleError::Capacity);
        }
        if ledger.cabin_tonnes() != 0 {
            let projection = account
                .service_projection(
                    &self
                        .freight_session
                        .as_ref()
                        .ok_or(ModuleError::NoCommander)?
                        .state,
                )
                .map_err(|_| ModuleError::Capacity)?;
            let projection = add_cabin_mass(projection, ledger.cabin_tonnes())
                .map_err(|_| ModuleError::Capacity)?;
            self.navigation_trade
                .reservations()
                .apply(projection)
                .map_err(|_| ModuleError::Capacity)?;
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
                ModuleAction::Passenger(action) => cabins::receipt_data(self, action)?,
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
            xyzzy: false,
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
        let drive_fitted = ledger.drive().is_some_and(|(id, _)| id == owner + 6);
        let shield_fitted = ledger
            .fitted(ModuleSlot::Shield)
            .is_some_and(|item| item.id == owner + 3);
        let shield_grade = ledger.shield_grade().ok_or(ModuleError::ShieldRequired)?;
        let account = if let ModuleAction::Passenger(action) = review.action {
            current.prepare_passage_action(action == PassengerAction::Receive)
        } else {
            current.prepare_module(
                paid,
                [
                    pulse_fitted,
                    energy_fitted,
                    navigation_fitted,
                    drive_fitted,
                    shield_fitted,
                ],
                ledger.fuel_storage_count(),
            )
        }
        .map_err(|_| ModuleError::SequenceExhausted)?;
        let mut state = self.modular.clone();
        if history::requires_history(review.action) {
            state.history.activate(self, current, review.dock)?;
        }
        state.events.push(event);
        let mut combat = self.combat;
        let (account, state, bytes) = if shield_grade != combat.shields().grade() {
            combat.refit_shields(shield_grade);
            self.shield_fit_save(account, state, combat)
                .map_err(ModuleError::Save)?
        } else {
            let bytes = self
                .save_bytes_with_modules(&account, &state)
                .map_err(ModuleError::Save)?;
            (account, state, bytes)
        };
        Ok(PreparedModule {
            world: self,
            state,
            account,
            combat,
            bytes,
            // Passage changes have a receipt, not an issued physical equipment item.
            item: if matches!(review.action, ModuleAction::Passenger(_)) {
                0
            } else {
                item
            },
        })
    }
}

#[cfg(test)]
mod cabin_tests;
#[cfg(test)]
mod drive_tests;
#[cfg(test)]
mod fuel_storage_tests;
#[cfg(test)]
mod navigation_tests;
#[cfg(test)]
mod shield_tests;
#[cfg(test)]
mod tests;

impl State {
    pub(super) fn front_at_physical(
        &self,
        world: &World,
        clock: u64,
        firing_emission: u64,
    ) -> Option<OwnedModule> {
        let mut ledger = Ledger::born(world.combat_identity()?.owner).ok()?;
        let mut counters = [0; 2];
        for event in &self.events {
            if let ModuleAction::FlightLaserExchange {
                clock: at,
                emission,
                ..
            } = event.action
            {
                if at > clock || (at == clock && emission >= firing_emission) {
                    break;
                }
            } else if world.damage.berth_clock(event.dock.sequence())? > clock {
                break;
            }
            ledger.replay(*event, &mut counters).ok()?;
        }
        ledger.front()
    }
}

impl World {
    fn historical_front_at_tick(&self, tick: u64, firing_emission: u64) -> Option<OwnedModule> {
        let mut ledger = Ledger::born(self.combat_identity()?.owner).ok()?;
        let mut counters = [0; 2];
        for event in self
            .modular
            .events
            .iter()
            .filter(|event| event.tick <= tick)
        {
            if matches!(event.action, ModuleAction::FlightLaserExchange { emission, .. } if event.tick == tick && emission >= firing_emission)
            {
                break;
            }
            ledger.replay(*event, &mut counters).ok()?;
        }
        ledger.front()
    }
    pub(super) fn front_weapon_at_tick(
        &self,
        item: u64,
        tick: u64,
        emission: u64,
    ) -> Option<super::OrdinaryWeapon> {
        let fitted = self.historical_front_at_tick(tick, emission)?;
        if item != fitted.id {
            return None;
        }
        super::OrdinaryWeapon::from_module(fitted.kind)
    }
    pub(super) fn front_cadence_at_tick(&self, tick: u64) -> Option<u8> {
        super::OrdinaryWeapon::from_module(self.historical_front_at_tick(tick, u64::MAX)?.kind)
            .map(super::OrdinaryWeapon::cooldown_ticks)
    }
}

impl World {
    /// Select an actual owned work tool in the original front laser mount. No purchase,
    /// new item, resource refill or extra mount is implied by selecting a rock.
    pub(in crate::flight) fn select_auto_mining_tool(
        &mut self,
    ) -> Result<(), crate::mining::MiningError> {
        use crate::mining::MiningError;
        if self.state.mode != super::FlightMode::Flight || self.is_paused() || !self.pulse.alive() {
            return Err(MiningError::NotInFlight);
        }
        if self.emp.input_blocked() {
            return Err(MiningError::Resources);
        }
        if self
            .selected_work_target()
            .is_none_or(|target| !matches!(target.id, crate::equipment::WorkTargetId::Asteroid(_)))
        {
            return Err(MiningError::NoTarget);
        }
        if self.flight_reception.occupancy().1 > 0
            || self.navigation_trade.occupancy().1 > 0
            || self.practice.is_some_and(|p| !p.reception_is_terminal())
            || self
                .mission_snapshot()
                .is_some_and(|s| s.freight.held_tonnes > 0)
        {
            return Err(MiningError::ProtectedActivity);
        }
        let account = self
            .commander_account
            .as_ref()
            .ok_or(MiningError::NoCommander)?;
        let ledger = self
            .modular
            .ledger(self, account)
            .map_err(|_| MiningError::InvalidState)?;
        let front = ledger.front().ok_or(MiningError::NoCompatibleFittedTool)?;
        if front.kind == ModuleKind::MiningLaser {
            if ledger.auto_return.is_some() {
                return Ok(());
            }
            let combat = ledger
                .items
                .iter()
                .filter(|i| {
                    i.slot.is_none()
                        && i.kind != ModuleKind::MiningLaser
                        && super::OrdinaryWeapon::from_module(i.kind).is_some()
                })
                .max_by_key(|i| {
                    (
                        super::OrdinaryWeapon::from_module(i.kind).map(|w| w.damage_hundredths()),
                        i.id,
                    )
                })
                .copied();
            return if let Some(combat) = combat {
                self.exchange_auto_mining_tool(ledger, front, combat, false)
            } else {
                Ok(())
            };
        }
        let mining = ledger
            .items
            .iter()
            .find(|i| {
                i.slot.is_none()
                    && i.kind == ModuleKind::MiningLaser
                    && matches!(
                        i.provenance,
                        ModuleProvenance::Paid { .. } | ModuleProvenance::XYZzy { .. }
                    )
            })
            .copied()
            .ok_or(MiningError::NoCompatibleFittedTool)?;
        self.exchange_auto_mining_tool(ledger, front, mining, false)
    }

    pub(in crate::flight) fn restore_auto_mining_tool(
        &mut self,
    ) -> Result<(), crate::mining::MiningError> {
        use crate::mining::MiningError;
        // No full ledger replay on ordinary flight before automatic mining is used.
        if !self
            .modular
            .events
            .iter()
            .rev()
            .find_map(|e| match e.action {
                ModuleAction::FlightLaserExchange { restoring, .. } => Some(!restoring),
                _ => None,
            })
            .unwrap_or(false)
        {
            return Ok(());
        }
        let account = self
            .commander_account
            .as_ref()
            .ok_or(MiningError::NoCommander)?;
        let ledger = self
            .modular
            .ledger(self, account)
            .map_err(|_| MiningError::InvalidState)?;
        let Some(item) = ledger.auto_return else {
            return Ok(());
        };
        if self.is_paused() || self.emp.input_blocked() || !self.pulse.alive() {
            return Err(MiningError::Resources);
        }
        let front = ledger.front().ok_or(MiningError::InvalidState)?;
        let combat = ledger
            .items
            .iter()
            .find(|i| i.id == item && i.slot.is_none())
            .copied()
            .ok_or(MiningError::InvalidState)?;
        self.exchange_auto_mining_tool(ledger, front, combat, true)
    }

    fn exchange_auto_mining_tool(
        &mut self,
        mut ledger: Ledger,
        front: OwnedModule,
        next: OwnedModule,
        restoring: bool,
    ) -> Result<(), crate::mining::MiningError> {
        use crate::mining::MiningError;
        let owner = self.combat_identity().ok_or(MiningError::NoCommander)?;
        let current = self
            .commander_account
            .as_ref()
            .ok_or(MiningError::NoCommander)?;
        let dock = self
            .modular
            .events
            .last()
            .ok_or(MiningError::InvalidState)?
            .dock;
        let event = Event {
            xyzzy: false,
            action: ModuleAction::FlightLaserExchange {
                item: next.id,
                replacing: front.id,
                visit: owner.visit,
                clock: self.combat.physical_tick(),
                emission: self.combat.emissions,
                restoring,
            },
            item: next.id,
            dock,
            tick: self.state.tick,
            revision: current
                .revision()
                .checked_add(1)
                .ok_or(MiningError::ProofCapacity)?,
        };
        ledger
            .apply(event, 0)
            .map_err(|_| MiningError::InvalidState)?;
        let allowance = if self.test_tank_custody_clear() {
            ledger.test_tank_count()
        } else {
            0
        };
        ledger
            .validate_budgets(current, self.emp.utility_mounts(), allowance)
            .map_err(|_| MiningError::InvalidState)?;
        let born = |slot, offset| {
            ledger
                .fitted(slot)
                .is_some_and(|item| item.id == owner.owner + offset)
        };
        let account = current
            .prepare_module(
                0,
                [
                    born(ModuleSlot::FrontLaser, 7),
                    born(ModuleSlot::Energy, 5),
                    born(ModuleSlot::Navigation, 4),
                    born(ModuleSlot::Drive, 6),
                    born(ModuleSlot::Shield, 3),
                ],
                ledger.fuel_storage_count(),
            )
            .map_err(|_| MiningError::InvalidState)?;
        let mut modules = self.modular.clone();
        modules.events.push(event);
        self.save_bytes_with_modules(&account, &modules)
            .map_err(|_| MiningError::ProofCapacity)?;
        self.modular = modules;
        self.commander_account = Some(account);
        Ok(())
    }

    pub(super) fn grant_xyzzy_module(
        &mut self,
        dock: DockContext,
        tick: u64,
        kind: ModuleKind,
    ) -> Result<(), SaveError> {
        if !kind.xyzzy_supported() {
            return Err(INVALID);
        }
        let current = self.commander_account.as_ref().ok_or(INVALID)?;
        let mut ledger = self.modular.ledger(self, current)?;
        let tank = kind == ModuleKind::FuelStorage;
        if tank && ledger.fuel_storage_count() == crate::navigation::MAX_FUEL_STORAGE_UNITS {
            return Ok(());
        }
        let existing = ledger
            .items
            .iter()
            .find(|item| item.kind == kind && (!tank || item.slot.is_none()))
            .copied();
        let cutter_stored = if kind == ModuleKind::SalvageCutter {
            let definition = crate::equipment::definition().map_err(|_| INVALID)?;
            let (units, mounts) = ledger
                .budget_usage(current, self.emp.utility_mounts())
                .map_err(|_| INVALID)?;
            units >= definition.equipment_units_capacity
                || mounts >= definition.utility_mounts_capacity
        } else {
            false
        };
        if (cutter_stored || kind == ModuleKind::MiningLaser) && existing.is_some() {
            return Ok(());
        }
        if !tank && existing.is_some_and(|item| item.slot == Some(kind.slot())) {
            return Ok(());
        }
        let grant = u32::try_from(self.modular.grant_count() + 1).map_err(|_| INVALID)?;
        if grant > 100000 {
            return Err(INVALID);
        }
        let (action, item, xyzzy) = if let Some(item) = existing {
            (
                ModuleAction::Install {
                    item: item.id,
                    replacing: if tank {
                        None
                    } else {
                        ledger.fitted(kind.slot()).map(|item| item.id)
                    },
                },
                item.id,
                false,
            )
        } else {
            (
                if cutter_stored {
                    ModuleAction::GrantStoredSalvageCutter
                } else if kind == ModuleKind::MiningLaser {
                    ModuleAction::GrantStoredMiningLaser
                } else {
                    ModuleAction::Purchase { kind }
                },
                dock.owner()
                    .seed()
                    .checked_add(200010 + u64::from(grant))
                    .ok_or(INVALID)?,
                true,
            )
        };
        let event = Event {
            xyzzy,
            action,
            item,
            dock,
            tick,
            revision: current.revision().checked_add(1).ok_or(INVALID)?,
        };
        ledger.apply(event, grant).map_err(|_| INVALID)?;
        let owner = dock.owner().seed();
        let born_fitted = |slot, offset| {
            ledger
                .fitted(slot)
                .is_some_and(|item| item.id == owner + offset)
        };
        let account = current
            .prepare_module(
                0,
                [
                    born_fitted(ModuleSlot::FrontLaser, 7),
                    born_fitted(ModuleSlot::Energy, 5),
                    born_fitted(ModuleSlot::Navigation, 4),
                    born_fitted(ModuleSlot::Drive, 6),
                    born_fitted(ModuleSlot::Shield, 3),
                ],
                ledger.fuel_storage_count(),
            )
            .map_err(|_| INVALID)?;
        if let ModuleKind::Shield(grade) = kind {
            if !self.damage.enabled() {
                return Err(INVALID);
            }
            self.combat.refit_shields(grade);
        }
        self.commander_account = Some(account);
        self.modular.events.push(event);
        Ok(())
    }
}

#[cfg(test)]
mod xyzzy_tests;

impl State {
    pub(super) fn emp_energy_changes(
        &self,
        world: &World,
    ) -> Result<Vec<(u64, Option<u8>)>, SaveError> {
        let owner = world.combat_identity().ok_or(INVALID)?.owner;
        let mut ledger = Ledger::born(owner)?;
        let mut counters = [0; 2];
        let mut changes = vec![(0, Some(0))];
        for event in &self.events {
            ledger.replay(*event, &mut counters).map_err(|_| INVALID)?;
            let stage = ledger
                .fitted(ModuleSlot::Energy)
                .and_then(|item| match item.kind {
                    ModuleKind::BasicEnergySystem => Some(0),
                    ModuleKind::ExtraEnergyUnit => Some(1),
                    ModuleKind::NavalEnergyUnit => Some(2),
                    ModuleKind::EliteEnergyUnit => Some(3),
                    _ => None,
                });
            if changes
                .last()
                .is_none_or(|(_, previous)| *previous != stage)
            {
                changes.push((
                    world
                        .damage
                        .berth_clock(event.dock.sequence())
                        .ok_or(INVALID)?,
                    stage,
                ));
            }
        }
        Ok(changes)
    }
}

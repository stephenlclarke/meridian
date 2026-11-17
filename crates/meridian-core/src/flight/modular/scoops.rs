//! Original paid Fuel Scoops utility. A fitted readout never grants physical ore custody.
use super::*;
pub(super) const PRICE_TENTHS: u32 = 19_000;
pub(super) const REQUIRED_TL: u8 = 4;
/// Descriptive current paid fit. No work/collection API accepts a caller-constructed readout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FittedFuelScoops {
    pub owner: u64,
    pub hull: u64,
    pub item: u64,
    pub account_revision: u64,
    pub fit_revision: u64,
    pub epoch: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuelScoopsSnapshot {
    pub family: &'static str,
    pub node: &'static str,
    pub compatible_hull: &'static str,
    pub slot: ModuleSlot,
    pub owned: Option<OwnedModule>,
    pub fitted: Option<FittedFuelScoops>,
    pub current_offer: Option<ModuleOffer>,
    /// Actual implemented cargo-expansion and scoop mounts; other utility families are absent.
    pub utility_mounts_used: u16,
    pub utility_mounts_capacity: u16,
}
fn validate_slots(
    definition: crate::equipment::Definition,
    items: &[OwnedModule],
    cargo_mounts: u16,
) -> Result<u16, ModuleError> {
    if definition.hull != "SH025"
        || items
            .iter()
            .any(|i| i.slot.is_some_and(|slot| slot != i.kind.slot()))
    {
        return Err(ModuleError::StaleOccupant);
    }
    let scoops = items
        .iter()
        .filter(|i| i.slot == Some(ModuleSlot::UtilityFuelScoops))
        .count();
    let used = cargo_mounts
        .checked_add(u16::try_from(scoops).map_err(|_| ModuleError::Capacity)?)
        .ok_or(ModuleError::Capacity)?;
    if used > definition.utility_mounts_capacity {
        return Err(ModuleError::Capacity);
    }
    Ok(used)
}
pub(super) fn validate_fit(
    items: &[OwnedModule],
    account: &CommanderAccount,
) -> Result<(), ModuleError> {
    validate_slots(
        crate::equipment::definition().map_err(|_| ModuleError::BerthRequired)?,
        items,
        account.equipment_state().units_used(),
    )
    .map(|_| ())
}
impl World {
    pub fn fuel_scoops_snapshot(&self) -> Option<FuelScoopsSnapshot> {
        let account = self.commander_account.as_ref()?;
        let ledger = self.modular.ledger(self, account).ok()?;
        let definition = crate::equipment::definition().ok()?;
        let utility_mounts_used = validate_slots(
            definition,
            &ledger.items,
            account.equipment_state().units_used(),
        )
        .ok()?;
        let owned = ledger
            .items
            .iter()
            .find(|i| i.kind == ModuleKind::FuelScoops)
            .copied();
        let owner = self.combat_identity()?;
        let epoch = self.freight_session.as_ref()?.epoch();
        let fitted = owned
            .filter(|i| {
                i.slot == Some(ModuleSlot::UtilityFuelScoops)
                    && matches!(i.provenance, ModuleProvenance::Paid { .. })
            })
            .map(|item| FittedFuelScoops {
                owner: owner.owner,
                hull: owner.hull,
                item: item.id,
                account_revision: account.revision(),
                fit_revision: ledger.revision,
                epoch,
            });
        let current_offer = self
            .current_dock_context()
            .and_then(|dock| super::super::services::service_site(dock.system()))
            .map(|site| ModuleOffer {
                kind: ModuleKind::FuelScoops,
                slot: ModuleKind::FuelScoops.slot(),
                equipment_units: ModuleKind::FuelScoops.equipment_units(),
                prerequisite: ModuleKind::FuelScoops.prerequisite(),
                price_tenths: PRICE_TENTHS,
                required_technology: REQUIRED_TL,
                technology: definition.offer(site).technology,
                available: self
                    .review_module(ModuleAction::PurchaseFuelScoops, self.state.tick)
                    .is_ok(),
            });
        Some(FuelScoopsSnapshot {
            family: ModuleKind::FuelScoops.family(),
            node: ModuleKind::FuelScoops.node(),
            compatible_hull: definition.hull,
            slot: ModuleSlot::UtilityFuelScoops,
            owned,
            fitted,
            current_offer,
            utility_mounts_used,
            utility_mounts_capacity: definition.utility_mounts_capacity,
        })
    }
    pub fn fitted_fuel_scoops(&self) -> Option<FittedFuelScoops> {
        self.fuel_scoops_snapshot()?.fitted
    }
}
#[cfg(test)]
mod tests;

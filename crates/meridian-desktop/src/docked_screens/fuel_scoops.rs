//! Typed utility presentation from MC1's actual paid Fuel Scoops snapshot.
use super::{EquipmentOperation, EquipmentRow, EquipmentSlot, OfferQuantity};
use meridian_core::{ModuleAction, ModuleKind, ModuleSlot, World};

pub(crate) const SLOT: &str = "utility-fuelScoops";

pub(crate) fn slot(world: &World) -> Result<EquipmentSlot, String> {
    let snapshot = world
        .fuel_scoops_snapshot()
        .ok_or("No Fuel Scoops inventory")?;
    Ok(EquipmentSlot {
        id: SLOT.into(),
        label: "Fuel Scoops utility".into(),
        fitted: if snapshot.fitted.is_some() {
            "Fuel Scoops"
        } else {
            "Empty"
        }
        .into(),
        available: true,
    })
}

fn row(world: &World) -> Result<EquipmentRow, String> {
    let snapshot = world
        .fuel_scoops_snapshot()
        .ok_or("No Fuel Scoops inventory")?;
    Ok(EquipmentRow {
        node: snapshot.node.into(),
        name: "Fuel Scoops".into(),
        group: "Utility".into(),
        price_tenths: None,
        quantity: OfferQuantity::NotTracked,
        instance: None,
        owned: 0,
        fitted: 0,
        compatibility: format!("{} | Utility mounts {}/{}", snapshot.compatible_hull, snapshot.utility_mounts_used, snapshot.utility_mounts_capacity),
        refusal: None,
        readout: vec!["A fitted utility module; does not replace the front laser".into(), "Collection still requires an eligible physical body; fitting does not grant cargo or sale income".into()],
        options: Vec::new(),
        can_buy: false,
        can_fit: false,
        can_store: false,
    })
}

pub(crate) fn rows(world: &World) -> Result<Vec<EquipmentRow>, String> {
    let snapshot = world
        .fuel_scoops_snapshot()
        .ok_or("No Fuel Scoops inventory")?;
    let tick = world.snapshot().tick;
    let mut result = Vec::new();
    if let Some(owned) = snapshot.owned {
        let mut r = row(world)?;
        r.instance = Some(owned.id);
        r.owned = 1;
        r.fitted = u32::from(snapshot.fitted.is_some());
        r.readout
            .push(crate::xyzzy::provenance(owned.provenance).into());
        r.readout.push(
            if r.fitted == 1 {
                "Owned and fitted"
            } else {
                "Owned in storage; refit without repurchase"
            }
            .into(),
        );
        r.can_store = world
            .review_module(ModuleAction::Store { item: owned.id }, tick)
            .is_ok();
        r.can_fit = world
            .review_module(
                ModuleAction::Install {
                    item: owned.id,
                    replacing: snapshot.fitted.map(|f| f.item),
                },
                tick,
            )
            .is_ok();
        result.push(r);
    }
    if let Some(offer) = snapshot.current_offer {
        let mut r = row(world)?;
        r.name = "Fuel Scoops (purchase)".into();
        r.price_tenths = Some(u64::from(offer.price_tenths));
        r.readout.push(format!(
            "Requires technology {} | Current {}",
            offer.required_technology, offer.technology
        ));
        r.readout
            .push("One owned Fuel Scoops module, including stored equipment".into());
        let admission = world.review_module(ModuleAction::PurchaseFuelScoops, tick);
        r.can_buy = admission.is_ok();
        r.refusal = admission.err().map(|e| e.to_string());
        result.push(r);
    }
    Ok(result)
}

pub(crate) fn action(
    world: &World,
    node: &str,
    instance: Option<u64>,
    operation: EquipmentOperation,
) -> Result<ModuleAction, String> {
    let snapshot = world
        .fuel_scoops_snapshot()
        .ok_or("No Fuel Scoops inventory")?;
    if node != snapshot.node || snapshot.slot != ModuleSlot::UtilityFuelScoops {
        return Err("Not the current Fuel Scoops utility".into());
    }
    let action = match operation {
        EquipmentOperation::Buy if instance.is_none() => {
            snapshot
                .current_offer
                .filter(|o| o.kind == ModuleKind::FuelScoops)
                .ok_or("No current Fuel Scoops offer")?;
            ModuleAction::PurchaseFuelScoops
        }
        EquipmentOperation::Store | EquipmentOperation::Fit => {
            let owned = snapshot
                .owned
                .filter(|i| Some(i.id) == instance && i.kind == ModuleKind::FuelScoops)
                .ok_or("Fuel Scoops ownership changed; review its identity")?;
            if operation == EquipmentOperation::Store {
                ModuleAction::Store { item: owned.id }
            } else {
                ModuleAction::Install {
                    item: owned.id,
                    replacing: snapshot.fitted.map(|f| f.item),
                }
            }
        }
        _ => return Err("This Fuel Scoops operation is unavailable".into()),
    };
    world
        .review_module(action, world.snapshot().tick)
        .map_err(|e| e.to_string())?;
    Ok(action)
}

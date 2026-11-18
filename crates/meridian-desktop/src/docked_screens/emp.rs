//! EMP rows describe current World offers; opaque row names never grant transaction authority.
use super::{EquipmentOperation, EquipmentRow, EquipmentSlot, OfferQuantity};
use meridian_core::{EmpAxis, EmpEquipmentAction, EmpProvenance, World};

pub(crate) const SLOT: &str = "utility-emp";
fn node(action: EmpEquipmentAction) -> &'static str {
    match action {
        EmpEquipmentAction::Upgrade {
            axis: EmpAxis::Power,
            ..
        } => "emp-power",
        EmpEquipmentAction::Upgrade {
            axis: EmpAxis::Capacity,
            ..
        } => "emp-capacity",
        EmpEquipmentAction::Upgrade {
            axis: EmpAxis::Recharge,
            ..
        } => "emp-recharge",
        _ => "emp-property",
    }
}
fn name(action: EmpEquipmentAction) -> &'static str {
    match action {
        EmpEquipmentAction::Upgrade {
            axis: EmpAxis::Power,
            ..
        } => "EMP power upgrade",
        EmpEquipmentAction::Upgrade {
            axis: EmpAxis::Capacity,
            ..
        } => "EMP bank capacity upgrade",
        EmpEquipmentAction::Upgrade {
            axis: EmpAxis::Recharge,
            ..
        } => "EMP bank recharge upgrade",
        _ => "EMP electrical utility",
    }
}
pub(crate) fn slot(world: &World) -> EquipmentSlot {
    EquipmentSlot {
        id: SLOT.into(),
        label: "EMP utility".into(),
        available: true,
        fitted: if world.owned_emp().is_some_and(|i| i.fitted) {
            "EMP"
        } else {
            "Empty"
        }
        .into(),
    }
}
pub(crate) fn rows(world: &World) -> Vec<EquipmentRow> {
    let owned = world.owned_emp();
    world
        .emp_equipment_offers()
        .into_iter()
        .map(|offer| {
            let permitted = offer.refusal.is_none();
            let purchase = matches!(
                offer.action,
                EmpEquipmentAction::Purchase | EmpEquipmentAction::Upgrade { .. }
            );
            let mut readout = vec![
                format!(
                    "Requires TL {} | Station TL {}",
                    offer.required_technology, offer.technology
                ),
                "Separate bank; eight units per pulse; hold U in flight for ten seconds".into(),
                "Recent-attacker self-defence only; unsupported contacts refuse without spending"
                    .into(),
            ];
            if let Some(item) = owned {
                readout.push(format!(
                    "Item {} | {} | Power/Capacity/Recharge {:?}",
                    item.item,
                    match item.provenance {
                        EmpProvenance::Paid => "Paid original receipt",
                        EmpProvenance::Xyzzy => "XYZZY grant; not paid",
                    },
                    item.grades
                ));
            }
            if let Some(state) = world.emp_snapshot() {
                readout.push(format!(
                    "Bank {:.2}/{:.2} | Main electrical {:.2} | Transfer loss {:.2}",
                    state.bank_energy, state.bank_capacity, state.main_energy, state.transfer_loss
                ));
            }
            EquipmentRow {
                node: node(offer.action).into(),
                name: name(offer.action).into(),
                group: "EMP utility".into(),
                price_tenths: purchase.then_some(u64::from(offer.price_tenths)),
                quantity: OfferQuantity::NotTracked,
                instance: if purchase {
                    None
                } else {
                    owned.map(|i| i.item)
                },
                owned: u32::from(owned.is_some()),
                fitted: u32::from(owned.is_some_and(|i| i.fitted)),
                compatibility:
                    "SH025 utility mount | Bank starts empty; no ECM or hull-damage alias".into(),
                refusal: offer.refusal.map(|e| e.to_string()),
                readout,
                options: Vec::new(),
                can_buy: purchase && permitted,
                can_fit: matches!(offer.action, EmpEquipmentAction::Fit { .. }) && permitted,
                can_store: matches!(offer.action, EmpEquipmentAction::Store { .. }) && permitted,
            }
        })
        .collect()
}
pub(crate) fn action(
    world: &World,
    selected: &str,
    instance: Option<u64>,
    operation: EquipmentOperation,
) -> Result<EmpEquipmentAction, String> {
    let expected = if operation == EquipmentOperation::Buy {
        None
    } else {
        world.owned_emp().map(|i| i.item)
    };
    if instance != expected {
        return Err("EMP ownership changed; review the current item".into());
    }
    let offer = world
        .emp_equipment_offers()
        .into_iter()
        .find(|o| {
            node(o.action) == selected
                && matches!(
                    (operation, o.action),
                    (
                        EquipmentOperation::Buy,
                        EmpEquipmentAction::Purchase | EmpEquipmentAction::Upgrade { .. }
                    ) | (EquipmentOperation::Fit, EmpEquipmentAction::Fit { .. })
                        | (EquipmentOperation::Store, EmpEquipmentAction::Store { .. })
                )
        })
        .ok_or("No current EMP offer for this action")?;
    if let Some(error) = offer.refusal {
        return Err(error.to_string());
    }
    Ok(offer.action)
}

/// One concise status row; detailed controls remain in contextual Help.
pub(crate) fn status(world: &World) -> Option<String> {
    let s = world.emp_snapshot()?;
    let phase = match s.charge_phase {
        Some(meridian_core::EmpChargePhase::Charging) => {
            format!("HOLD {:.1}/10S", s.charge_ticks as f64 / 60.0)
        }
        Some(meridian_core::EmpChargePhase::Released) if s.charge_release.is_none() => {
            "REFUSED; RELEASE U".into()
        }
        Some(meridian_core::EmpChargePhase::Released) => "PULSE RELEASED".into(),
        Some(meridian_core::EmpChargePhase::Cancelled) => "CANCELLED".into(),
        None => if s.item.fitted {
            "HOLD U: EMP"
        } else {
            "STORED"
        }
        .into(),
    };
    let phase = s
        .target
        .map_or_else(|| phase.clone(), |id| format!("{phase} T{}", id.serial()));
    Some(format!(
        "EMP {:.1}/{:.1} E{:.1} | {}{}{}{}",
        s.bank_energy,
        s.bank_capacity,
        s.main_energy,
        phase,
        if s.electrically_disabled {
            " | SELF OFF"
        } else {
            ""
        },
        if s.live_disabled_targets > 0 {
            format!(" | NPC OFF {}", s.live_disabled_targets)
        } else {
            String::new()
        },
        if s.travel_blocked { " | NO JUMP" } else { "" }
    ))
}

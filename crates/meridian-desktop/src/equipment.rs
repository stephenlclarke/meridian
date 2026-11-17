//! Equipment controls forward typed commands; only World owns paid items and fitting.
use crate::session::Action;
use meridian_core::{World, equipment};

pub fn command(world: &mut World, action: Action) -> Result<String, String> {
    let snapshot = world
        .equipment_snapshot()
        .ok_or("No commander equipment account")?;
    let command = match action {
        Action::PurchaseCargoBay => equipment::Command::PurchaseCargoBay,
        Action::FitCargoBay | Action::UnfitCargoBay => {
            let item = snapshot
                .owned
                .ok_or("Purchase a cargo bay before fitting or storing it")?
                .id;
            if matches!(action, Action::FitCargoBay) {
                equipment::Command::FitCargoBay(item)
            } else {
                equipment::Command::UnfitCargoBay(item)
            }
        }
        _ => return Err("Select an equipment action".into()),
    };
    let event = world
        .request_equipment(command, world.snapshot().tick)
        .map_err(|e| e.to_string())?;
    Ok(match event {
        equipment::Event::Purchased { paid_tenths, .. } => format!(
            "Large Cargo Bay purchased and fitted for {}.{} CR.",
            paid_tenths / 10,
            paid_tenths % 10
        ),
        equipment::Event::Fitted { .. } => "Owned cargo bay refitted. No payment.".into(),
        equipment::Event::Stored { .. } => {
            "Owned cargo bay stored. No refund; cargo remains aboard.".into()
        }
    })
}

pub fn append_panel(world: &World, lines: &mut Vec<String>) -> Result<(), String> {
    let equipment = world
        .equipment_snapshot()
        .ok_or("No commander equipment account")?;
    let credit = world.credits_tenths().ok_or("No commander wallet")?;
    lines.push("EQUIPMENT: LARGE CARGO BAY".into());
    lines.push(format!(
        "CREDITS {}.{} CR | HOLD {}/{}T",
        credit / 10,
        credit % 10,
        equipment.used_tonnes,
        equipment.hold_capacity_tonnes
    ));
    lines.push(format!(
        "PURCHASE {}.{} CR | ADDS 35T WHILE FITTED",
        equipment.definition.price_tenths / 10,
        equipment.definition.price_tenths % 10
    ));
    lines.push(match equipment.owned {
        Some(item) if item.fitted => "OWNED AND FITTED | U STORES THE SAME ITEM".into(),
        Some(_) => "OWNED AND STORED | Y REFITS THE SAME ITEM".into(),
        None => "NOT OWNED | K PURCHASES AND FITS ONE CARGO BAY".into(),
    });
    if let Some(item) = equipment.owned {
        lines.push(
            if item.xyzzy {
                "XYZZY GRANT | NO PAID PURCHASE"
            } else {
                "PAID CARGO BAY | ORIGINAL RECEIPT RETAINED"
            }
            .into(),
        );
    }
    lines.push(format!(
        "UTILITY MOUNTS {}/3 | EQUIPMENT UNITS {}/24",
        equipment.utility_mounts_used, equipment.equipment_units_used
    ));
    lines.push("STORING REQUIRES ALL CARGO TO FIT THE STANDARD HOLD".into());
    lines.push(match equipment.current_offer {
        Some(offer) if equipment.service_available && offer.purchase_available => {
            "PURCHASE AVAILABLE AT THIS BERTH".into()
        }
        _ => "RESUME AT AN ELIGIBLE STATION BERTH FOR EQUIPMENT SERVICE".into(),
    });
    lines.push("O CLOSE | M MARKET | I MISSION | F5 SAVE | F9 RELOAD".into());
    Ok(())
}

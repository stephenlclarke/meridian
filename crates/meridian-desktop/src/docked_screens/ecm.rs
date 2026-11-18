//! Manual ECM property and field readouts. A presented row is never transaction authority.
use super::{EquipmentOperation, EquipmentRow, EquipmentSlot};
use meridian_core::{EcmError, ModuleAction, ModuleKind, ModuleSlot, World};

pub(crate) const SLOT: &str = "utility-ecm";

pub(crate) fn slot(world: &World) -> Result<EquipmentSlot, String> {
    let snapshot = world.ecm_snapshot().ok_or("No ECM equipment inventory")?;
    Ok(EquipmentSlot {
        id: SLOT.into(),
        label: "ECM utility".into(),
        available: true,
        fitted: if snapshot.fitted_item.is_some() {
            "E.C.M. System"
        } else {
            "Empty"
        }
        .into(),
    })
}
pub(crate) fn describe(world: &World, rows: &mut [EquipmentRow]) -> Result<(), String> {
    let equipment = world
        .modular_equipment_snapshot()
        .ok_or("No utility inventory")?;
    for row in rows {
        row.compatibility = format!(
            "SH025 utility mounts {}/{} | One owned ECM system",
            equipment.utility_mounts_used, equipment.utility_mounts_capacity
        );
        row.readout.extend([
            "N in flight: manual field costs 22 shared energy; no separate bank".into(),
            "Standard missiles in the current scene; field lasts 0.53 seconds".into(),
            "Continue keeps the cooldown and past effects; live field permission ends".into(),
            "Manual control only; automatic ECM is unavailable".into(),
        ]);
    }
    Ok(())
}
pub(crate) fn action(
    world: &World,
    node: &str,
    instance: Option<u64>,
    operation: EquipmentOperation,
) -> Result<ModuleAction, String> {
    if node != ModuleKind::EcmSystem.node() {
        return Err("Select the current E.C.M. System".into());
    }
    let modules = world
        .modular_equipment_snapshot()
        .ok_or("No ECM property inventory")?;
    let action = match operation {
        EquipmentOperation::Buy if instance.is_none() => ModuleAction::Purchase {
            kind: ModuleKind::EcmSystem,
        },
        EquipmentOperation::Fit | EquipmentOperation::Store => {
            let item = modules
                .items
                .iter()
                .find(|item| item.kind == ModuleKind::EcmSystem && Some(item.id) == instance)
                .ok_or("ECM ownership changed; select the current item")?;
            if operation == EquipmentOperation::Store {
                ModuleAction::Store { item: item.id }
            } else {
                ModuleAction::Install {
                    item: item.id,
                    replacing: modules
                        .items
                        .iter()
                        .find(|item| item.slot == Some(ModuleSlot::UtilityEcm))
                        .map(|item| item.id),
                }
            }
        }
        _ => return Err("This ECM operation is unavailable".into()),
    };
    world
        .review_module(action, world.snapshot().tick)
        .map_err(|error| error.to_string())?;
    Ok(action)
}
pub(crate) fn refusal(error: EcmError) -> String {
    match error {
        EcmError::NoCommander => "Start or continue a commander before using ECM".into(),
        EcmError::StaleCommand => "ECM command expired; try again".into(),
        EcmError::Paused => "Resume flight before activating ECM".into(),
        EcmError::NotInFlight => "ECM requires a living ship in flight".into(),
        EcmError::NoFittedHardware => "Fit an owned E.C.M. System before activating it".into(),
        EcmError::ElectricalDisabled => "ECM unavailable while electrical power is disabled".into(),
        EcmError::FieldActive => "ECM is cooling down; wait for the field timer to expire".into(),
        EcmError::InsufficientEnergy => "ECM needs more than 22 energy".into(),
        EcmError::Capacity => "ECM history is full; activation refused".into(),
        EcmError::Save(error) => format!("ECM could not be prepared: {error}"),
    }
}
pub(crate) fn status(world: &World) -> Option<String> {
    let modules = world.modular_equipment_snapshot()?;
    let item = modules
        .items
        .iter()
        .find(|item| item.kind == ModuleKind::EcmSystem)?;
    let snapshot = world.ecm_snapshot()?;
    let phase = if item.slot.is_none() {
        "STORED".into()
    } else if snapshot.remaining_ticks > 0 {
        format!(
            "{} {:.2}S",
            if snapshot.field.is_some() {
                "FIELD"
            } else {
                "COOLDOWN"
            },
            f64::from(snapshot.remaining_ticks) / 60.0
        )
    } else if world
        .emp_snapshot()
        .is_some_and(|emp| emp.electrically_disabled)
    {
        "POWER DISABLED".into()
    } else if world.flight_resources_snapshot()?.energy_hundredths <= 2200 {
        "LOW ENERGY".into()
    } else {
        "N ACTIVATE".into()
    };
    Some(format!(
        "ECM {phase} | COST 22 ENERGY | ENDED {}",
        snapshot.terminated_missiles
    ))
}

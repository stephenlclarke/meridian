//! Actual row adapters, selected-slot authorities and atomic profile publication; authored unrun.
use super::*;
use crate::profile::tests::{TestDirectory, open};
use meridian_core::{ModuleProvenance, XyzzyAction};
use std::collections::HashSet;

fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("expected actual row/item"))
}
fn start() -> (TestDirectory, Profile) {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(27101)));
    (directory, profile)
}
fn panel(profile: &Profile, slot: &str) -> ui::EquipmentPanel {
    ok(equipment_panel(&profile.world, slot, ""))
}
fn intent(profile: &Profile, item: u64, slot: &str, operation: ui::EquipmentOperation) -> Action {
    let row = some(
        panel(profile, slot)
            .rows
            .into_iter()
            .find(|row| row.instance == Some(item)),
    );
    ok(equipment_action(
        &profile.world,
        &row.node,
        row.instance,
        slot,
        operation,
    ))
}

#[test]
fn all_four_real_mount_rows_have_unique_slot_item_offer_identities_and_foreign_nodes_refuse() {
    let (_directory, profile) = start();
    let mut ids = HashSet::new();
    let before = ok(profile.world.save_bytes());
    for (facing, slot, label) in LASER_SLOTS {
        let panel = panel(&profile, slot);
        assert_eq!(panel.selected_slot, slot);
        assert_eq!(some(panel.slots.iter().find(|s| s.id == slot)).label, label);
        let owned = some(panel.rows.iter().find(|r| r.instance.is_some()));
        assert_eq!(owned.fitted, u32::from(facing == LaserFacing::Front));
        assert_eq!(owned.can_store, facing == LaserFacing::Front);
        assert_eq!(owned.can_fit, facing != LaserFacing::Front);
        for row in &panel.rows {
            assert!(ids.insert((slot.to_string(), row.node.clone(), row.instance)));
            assert!(row.node != ModuleKind::BasicEnergySystem.node());
        }
        let offer = some(panel.rows.iter().find(|r| r.price_tenths == Some(14500)));
        assert!(!offer.can_buy);
        assert!(some(offer.refusal.as_ref()).contains("InsufficientCredits"));
        for wrong in ["energy", "unknown-slot"] {
            assert!(
                equipment_action(
                    &profile.world,
                    &offer.node,
                    None,
                    wrong,
                    ui::EquipmentOperation::Buy
                )
                .is_err()
            );
        }
        if facing != LaserFacing::Front {
            assert!(
                equipment_action(
                    &profile.world,
                    ModuleKind::PulseLaser.node(),
                    None,
                    slot,
                    ui::EquipmentOperation::Buy
                )
                .is_err()
            );
        }
    }
    assert_eq!(ok(profile.world.save_bytes()), before);
}

#[test]
fn selected_occupied_mount_replaces_that_actual_item_and_moves_the_born_item_back_to_front_cold() {
    let (directory, mut profile) = start();
    // Explicit testing-menu funding/outfit, not earned/native acceptance. Purchases below
    // still use the actual ordinary paid transaction, wallet debit and durable publisher.
    ok(profile.xyzzy(XyzzyAction::Enter));
    let born = some(
        some(profile.world.modular_equipment_snapshot())
            .items
            .into_iter()
            .find(|item| item.kind == ModuleKind::PulseLaser),
    )
    .id;
    let node = module_node(ModuleKind::PulseLaser, ModuleSlot::RearLaser);
    let buy = ok(equipment_action(
        &profile.world,
        &node,
        None,
        "rear-laser",
        ui::EquipmentOperation::Buy,
    ));
    let cash = some(profile.world.credits_tenths());
    ok(profile.command(buy));
    assert_eq!(profile.world.credits_tenths(), Some(cash - 14500));
    let paid = some(profile.world.laser_snapshot(LaserFacing::Rear)).fitted_item_id;
    assert!(some(profile.world.modular_equipment_snapshot()).items.into_iter()
        .any(|item| item.id == paid && matches!(item.provenance, ModuleProvenance::Paid { .. })));
    let count = some(profile.world.modular_equipment_snapshot()).items.len();
    let command = intent(&profile, born, "rear-laser", ui::EquipmentOperation::Fit);
    assert!(
        matches!(command, Action::Module(ModuleAction::InstallLaser { item, facing: LaserFacing::Rear, replacing: Some(id) }) if item == born && id == paid)
    );
    ok(profile.command(command));
    let rear = panel(&profile, "rear-laser");
    assert_eq!(
        some(rear.rows.iter().find(|r| r.instance == Some(born))).fitted,
        1
    );
    assert_eq!(
        some(rear.rows.iter().find(|r| r.instance == Some(paid))).fitted,
        0
    );
    let front = panel(&profile, "front-laser");
    assert_eq!(
        some(front.rows.iter().find(|r| r.instance == Some(born))).fitted,
        0
    );
    assert!(!some(front.rows.iter().find(|r| r.instance == Some(born))).can_store);
    let command = intent(&profile, born, "front-laser", ui::EquipmentOperation::Fit);
    assert!(
        matches!(command, Action::Module(ModuleAction::InstallLaser { item, facing: LaserFacing::Front, .. }) if item == born)
    );
    ok(profile.command(command));
    assert_eq!(
        some(profile.world.front_laser_snapshot()).fitted_item_id,
        born
    );
    assert!(profile.world.laser_snapshot(LaserFacing::Rear).is_none());
    assert_eq!(
        some(profile.world.modular_equipment_snapshot()).items.len(),
        count
    );
    assert_eq!(profile.world.credits_tenths(), Some(cash - 14500));
    let disk = ok(std::fs::read(directory.0.join("commander.meridian")));
    assert_eq!(disk, ok(profile.world.save_bytes()));
    let expected_modules = profile.world.modular_equipment_snapshot();
    drop(profile); // Release the exclusive Store lease before fresh Continue.
    let mut cold = open(&directory.0);
    ok(cold.continue_saved());
    assert_eq!(
        cold.world.modular_equipment_snapshot(),
        expected_modules
    );
}

#[test]
fn laser_fit_and_paid_purchase_writer_failures_retain_world_wallet_item_and_prior_file() {
    let (directory, mut profile) = start();
    ok(profile.xyzzy(XyzzyAction::Enter)); // Labelled secret setup only.
    let born = some(
        some(profile.world.modular_equipment_snapshot())
            .items
            .into_iter()
            .find(|item| item.kind == ModuleKind::PulseLaser),
    )
    .id;
    ok(profile.save());
    let slot = directory.0.join("commander.meridian");
    let retained = directory.0.join("retained-before-laser-change");
    let before = ok(std::fs::read(&slot));
    ok(std::fs::rename(&slot, &retained));
    ok(std::fs::create_dir(&slot));
    let bank = profile.world.credits_tenths();
    let items = profile.world.modular_equipment_snapshot();
    let fit = intent(&profile, born, "left-laser", ui::EquipmentOperation::Fit);
    let buy = ok(equipment_action(
        &profile.world,
        &module_node(ModuleKind::PulseLaser, ModuleSlot::RightLaser),
        None,
        "right-laser",
        ui::EquipmentOperation::Buy,
    ));
    for command in [fit, buy] {
        let error = some(profile.command(command).err());
        assert!(error.contains("could not be saved"));
        assert_eq!(ok(profile.world.save_bytes()), before);
        assert_eq!(profile.world.credits_tenths(), bank);
        assert_eq!(profile.world.modular_equipment_snapshot(), items);
        assert_eq!(ok(std::fs::read(&retained)), before);
        assert!(slot.is_dir());
    }
}

#[test]
fn review_captures_selected_mount_and_a_changed_occupant_cannot_publish_the_old_fit() {
    let (_directory, mut profile) = start();
    let item = some(profile.world.front_laser_snapshot()).fitted_item_id;
    let mut menu = ok(Docked::new(&profile.world));
    ok(menu.open(&profile.world, ui::Screen::Equipment));
    ok(menu.request(
        &mut profile,
        ui::Request::InspectEquipmentSlot("left-laser".into()),
    ));
    let node = module_node(ModuleKind::PulseLaser, ModuleSlot::LeftLaser);
    let review = ui::Request::ReviewEquipment {
        node,
        instance: Some(item),
        slot: "left-laser".into(),
        operation: ui::EquipmentOperation::Fit,
        option: None,
    };
    ok(menu.request(&mut profile, review));
    let token = some(menu.pending.as_ref()).view.token;
    // Another genuine published action changes the captured modular revision/occupant.
    let command = intent(&profile, item, "right-laser", ui::EquipmentOperation::Fit);
    ok(profile.command(command));
    let before = ok(profile.world.save_bytes());
    ok(menu.request(&mut profile, ui::Request::ConfirmReview(token)));
    assert!(menu.feedback.contains("Values changed"));
    assert!(menu.pending.is_none());
    assert_eq!(ok(profile.world.save_bytes()), before);
    assert!(profile.world.laser_snapshot(LaserFacing::Left).is_none());
    assert_eq!(
        some(profile.world.laser_snapshot(LaserFacing::Right)).fitted_item_id,
        item
    );
}

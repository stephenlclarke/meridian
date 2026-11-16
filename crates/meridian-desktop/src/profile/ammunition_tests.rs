//! Durable paid-equipment actions use the same real Profile/Store as the desktop.
use super::tests::{TestDirectory, action, open};
use super::*;
use meridian_core::AmmunitionAction;
use std::fs;

fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn inventory(profile: &Profile) -> meridian_core::AmmunitionSnapshot {
    profile
        .world
        .ammunition_snapshot()
        .unwrap_or_else(|| panic!("missing ammunition"))
}
#[test]
fn paid_round_store_refit_and_cold_continue_retain_item_and_wallet() {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(81001)));
    action(&mut profile, Action::Ammunition(AmmunitionAction::Purchase));
    let bought = inventory(&profile);
    assert_eq!(profile.world.credits_tenths(), Some(5400));
    assert_eq!(
        profile
            .world
            .missile_rounds()
            .unwrap_or_else(|| panic!("rounds"))
            .iter()
            .flatten()
            .count(),
        4
    );
    let item = bought
        .items
        .last()
        .unwrap_or_else(|| panic!("paid item"))
        .item;
    action(
        &mut profile,
        Action::Ammunition(AmmunitionAction::Store { item }),
    );
    assert_eq!(
        inventory(&profile)
            .items
            .last()
            .unwrap_or_else(|| panic!("item"))
            .pylon,
        None
    );
    action(
        &mut profile,
        Action::Ammunition(AmmunitionAction::Refit { item, pylon: 8 }),
    );
    let before = ok(profile.world.save_bytes());
    assert_eq!(inventory(&profile).purchases.len(), 1);
    ok(profile.prepare_quit());
    drop(profile);
    let mut continued = open(&directory.0);
    ok(continued.continue_saved());
    assert_eq!(ok(continued.world.save_bytes()), before);
    assert_eq!(continued.world.credits_tenths(), Some(5400));
    assert_eq!(
        inventory(&continued)
            .items
            .last()
            .unwrap_or_else(|| panic!("item"))
            .pylon,
        Some(8)
    );
    assert_eq!(
        inventory(&continued)
            .items
            .last()
            .unwrap_or_else(|| panic!("item"))
            .item,
        item
    );
}
#[test]
fn failed_equipment_publication_preserves_world_and_previous_save_for_each_action() {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(82001)));
    action(&mut profile, Action::Ammunition(AmmunitionAction::Purchase));
    let item = inventory(&profile)
        .items
        .last()
        .unwrap_or_else(|| panic!("item"))
        .item;
    for command in [
        AmmunitionAction::Purchase,
        AmmunitionAction::Store { item },
        AmmunitionAction::Refit { item, pylon: 8 },
    ] {
        // Establish the real precondition through the successful durable command path.
        if matches!(command, AmmunitionAction::Refit { .. }) {
            action(
                &mut profile,
                Action::Ammunition(AmmunitionAction::Store { item }),
            );
        }
        let before = ok(profile.world.save_bytes());
        let slot = directory.0.join("commander.meridian");
        let backup = directory.0.join("retained-owned-fixture-save");
        let saved = ok(fs::read(&slot));
        ok(fs::rename(&slot, &backup));
        ok(fs::create_dir(&slot));
        assert!(profile.command(Action::Ammunition(command)).is_err());
        assert_eq!(ok(profile.world.save_bytes()), before);
        assert_eq!(ok(fs::read(&backup)), saved);
        ok(fs::remove_dir(&slot));
        ok(fs::rename(&backup, &slot));
        assert_eq!(ok(fs::read(&slot)), saved);
    }
}

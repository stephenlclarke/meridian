//! Durable paid-equipment actions use the same real Profile/Store as the desktop.
use super::tests::{TestDirectory, action, open};
use super::{Action, Profile};
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
    let before_inventory = inventory(&profile);
    let before_flight = profile.world.snapshot();
    let before_equipment = profile.world.equipment_snapshot();
    let before_services = profile.world.service_snapshot();
    let before_combat = profile.world.combat_snapshot();
    let before_pulse = profile.world.pulse_laser_snapshot();
    let before_mission = profile.world.mission_snapshot();
    let before_practice = profile.world.practice_snapshot();
    let before_flight_lesson = profile.world.flight_reception_snapshot();
    let before_trade_lesson = profile.world.navigation_trade_snapshot();
    let before_celestial = profile.world.celestial_snapshot();
    let before_rounds = profile.world.missile_rounds();
    let before_projectiles = profile.world.missile_projectiles().collect::<Vec<_>>();
    let old_review = ok(profile
        .world
        .review_ammunition(AmmunitionAction::Store { item }, before_flight.tick));
    assert_eq!(before_inventory.purchases.len(), 1);
    ok(profile.prepare_quit());
    drop(profile);
    let mut continued = open(&directory.0);
    ok(continued.continue_saved());
    // Continue intentionally rotates UI review authority. It must preserve the saved file
    // and every gameplay domain, while rejecting a pre-Continue fitting review.
    assert_eq!(ok(fs::read(directory.0.join("commander.meridian"))), before);
    assert_eq!(inventory(&continued), before_inventory);
    assert_eq!(continued.world.snapshot(), before_flight);
    assert_eq!(continued.world.equipment_snapshot(), before_equipment);
    assert_eq!(continued.world.service_snapshot(), before_services);
    assert_eq!(continued.world.combat_snapshot(), before_combat);
    assert_eq!(continued.world.pulse_laser_snapshot(), before_pulse);
    assert_eq!(continued.world.mission_snapshot(), before_mission);
    assert_eq!(continued.world.practice_snapshot(), before_practice);
    assert_eq!(
        continued.world.flight_reception_snapshot(),
        before_flight_lesson
    );
    assert_eq!(
        continued.world.navigation_trade_snapshot(),
        before_trade_lesson
    );
    assert_eq!(continued.world.celestial_snapshot(), before_celestial);
    assert_eq!(continued.world.missile_rounds(), before_rounds);
    assert_eq!(
        continued.world.missile_projectiles().collect::<Vec<_>>(),
        before_projectiles
    );
    assert!(matches!(
        continued
            .world
            .prepare_ammunition(old_review, before_flight.tick),
        Err(meridian_core::AmmunitionError::StaleReview)
    ));
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

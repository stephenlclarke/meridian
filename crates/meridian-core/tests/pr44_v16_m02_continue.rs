//! Byte-exact V16 save produced by the isolated PR #44 M02 diagnostic profile.
use meridian_core::{ModuleKind, ModuleSlot, World, save::SAVE_VERSION};

const V16: &[u8] = include_bytes!("fixtures/pr44-v16-m02-purchases.bin");

#[test]
fn pr44_v16_keeps_paid_equipment_and_original_navigation_across_v17_continue() {
    assert_eq!(u16::from_le_bytes([V16[8], V16[9]]), 16);
    let world = World::from_save_bytes(V16).unwrap_or_else(|error| panic!("{error}"));
    let modules = world
        .modular_equipment_snapshot()
        .unwrap_or_else(|| panic!("M02 active equipment"));
    assert!(
        modules
            .items
            .iter()
            .any(|item| item.kind == ModuleKind::MiningLaser
                && item.slot == Some(ModuleSlot::FrontLaser))
    );
    assert!(
        modules
            .items
            .iter()
            .any(|item| item.kind == ModuleKind::FuelScoops
                && item.slot == Some(ModuleSlot::UtilityFuelScoops))
    );
    assert!(
        modules
            .items
            .iter()
            .any(|item| item.kind == ModuleKind::BasicNavigation
                && item.slot == Some(ModuleSlot::Navigation))
    );
    assert_eq!(world.jump_reach_ly(), Some(9.0));
    let current = world.save_bytes().unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(u16::from_le_bytes([current[8], current[9]]), SAVE_VERSION);
    assert_eq!(SAVE_VERSION, 17);
    let continued = World::from_save_bytes(&current).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(continued.snapshot(), world.snapshot());
    assert_eq!(continued.modular_equipment_snapshot(), Some(modules));
    assert_eq!(continued.credits_tenths(), world.credits_tenths());
    assert_eq!(continued.ammunition_snapshot(), world.ammunition_snapshot());
}

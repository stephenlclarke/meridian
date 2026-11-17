//! Retained historical fixture and explicit current-only refusal boundary.
use meridian_core::{
    World,
    save::{SAVE_VERSION, SaveError},
};
const HISTORICAL: &[u8] = include_bytes!("fixtures/pr44-v16-m02-purchases.bin");

#[test]
fn literal_v16_refuses_without_replacing_current_world() {
    assert_eq!(u16::from_le_bytes([HISTORICAL[8], HISTORICAL[9]]), 16);
    assert!(matches!(
        World::from_save_bytes(HISTORICAL),
        Err(SaveError::UnsupportedVersion)
    ));
    let mut world = World::new_commander(15900 + 16).unwrap_or_else(|e| panic!("{e}"));
    let before = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(u16::from_le_bytes([before[8], before[9]]), SAVE_VERSION);
    assert_eq!(SAVE_VERSION, 17);
    assert_eq!(
        world.restore_bytes(HISTORICAL),
        Err(SaveError::UnsupportedVersion)
    );
    assert_eq!(world.save_bytes().unwrap_or_else(|e| panic!("{e}")), before);
    let cold = World::from_save_bytes(&before).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(cold.snapshot(), world.snapshot());
    assert_eq!(cold.credits_tenths(), world.credits_tenths());
    assert_eq!(
        cold.modular_equipment_snapshot(),
        world.modular_equipment_snapshot()
    );
}

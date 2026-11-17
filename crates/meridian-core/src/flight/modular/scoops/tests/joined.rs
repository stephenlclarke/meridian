//! Root-only World loader integration cases. Require the pinned shared caller patch; UNRUN.
use super::*;
fn continued(w: &World) -> World {
    let loaded = ok(World::from_save_bytes(&ok(w.save_bytes())));
    assert_eq!(loaded.credits_tenths(), w.credits_tenths());
    assert_eq!(loaded.service_snapshot(), w.service_snapshot());
    assert_eq!(
        loaded.flight_resources_snapshot(),
        w.flight_resources_snapshot()
    );
    assert_eq!(
        loaded.modular_equipment_snapshot(),
        w.modular_equipment_snapshot()
    );
    let old = some(w.fitted_fuel_scoops());
    let new = some(loaded.fitted_fuel_scoops());
    assert_eq!(
        (
            new.owner,
            new.hull,
            new.item,
            new.account_revision,
            new.fit_revision
        ),
        (
            old.owner,
            old.hull,
            old.item,
            old.account_revision,
            old.fit_revision
        )
    );
    assert_ne!(new.epoch, old.epoch);
    loaded
}
#[test]
fn root_join_cold_continue_preserves_paid_same_item_and_typed_store_refit_without_resource_reset() {
    let mut w = funded();
    let laser = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    let item = apply(&mut w, ModuleAction::PurchaseFuelScoops);
    let mut w = continued(&w);
    let cash = w.credits_tenths();
    let pools = w.flight_resources_snapshot();
    let stale = ok(w.review_module(ModuleAction::Store { item }, w.snapshot().tick));
    let bytes = ok(w.save_bytes());
    ok(w.restore_bytes(&bytes));
    let tick = w.snapshot().tick;
    assert!(matches!(
        w.prepare_module(stale, tick),
        Err(ModuleError::StaleReview)
    ));
    apply(&mut w, ModuleAction::Store { item });
    assert!(w.fitted_fuel_scoops().is_none());
    let mut w = ok(World::from_save_bytes(&ok(w.save_bytes())));
    assert!(w.fitted_fuel_scoops().is_none());
    apply(
        &mut w,
        ModuleAction::Install {
            item,
            replacing: None,
        },
    );
    assert_eq!(some(w.fitted_mining_tool()).item, laser);
    assert_eq!(some(w.fitted_fuel_scoops()).item, item);
    assert_eq!(w.credits_tenths(), cash);
    assert_eq!(w.flight_resources_snapshot(), pools);
    let _again = continued(&w);
}
#[test]
fn root_join_keeps_real_old_fifteen_bytes_accepted_and_rejects_new_utility_relabelling_atomically()
{
    let mut w = funded();
    let old = ok(crate::flight::persistence::v15_test_fixture(&w));
    let older = ok(World::from_save_bytes(&old));
    assert_eq!(older.credits_tenths(), w.credits_tenths());
    assert_eq!(
        older.modular_equipment_snapshot(),
        w.modular_equipment_snapshot()
    );
    assert!(older.fitted_fuel_scoops().is_none());
    let item = apply(&mut w, ModuleAction::PurchaseFuelScoops);
    let mut loaded = continued(&w);
    // Negative codec fixture only: no gear, pose, cargo or success is supplied to gameplay.
    let forged_old = ok(crate::flight::persistence::v15_test_fixture(&w));
    assert!(World::from_save_bytes(&forged_old).is_err());
    let before = ok(loaded.save_bytes());
    assert!(loaded.restore_bytes(&forged_old).is_err());
    assert_eq!(ok(loaded.save_bytes()), before);
    assert_eq!(some(loaded.fitted_fuel_scoops()).item, item);
}

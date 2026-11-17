//! Public paid equipment plus labelled decoder/policy boundaries. All UNRUN.
use super::*;
use crate::flight::practice::tests::launch;
use crate::save::{Reader, Writer};
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("{e:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("missing actual equipment"))
}
fn funded() -> World {
    crate::flight::equipment::tests::earned_world(12005)
}
fn apply(w: &mut World, action: ModuleAction) -> u64 {
    let t = w.snapshot().tick;
    let review = ok(w.review_module(action, t));
    ok(w.prepare_module(review, t)).commit()
}
fn encoded(state: &State) -> Vec<u8> {
    let mut w = Writer::new();
    state.encode(&mut w);
    w.bytes
}
fn decoded(state: &State, version: u16) -> State {
    let b = encoded(state);
    let mut r = Reader::new(&b);
    let result = ok(State::decode_version(&mut r, version));
    assert!(r.finished());
    assert_eq!(&result, state);
    result
}
#[test]
fn original_terms_admit_exact_tl_four_and_1900cr_boundaries() {
    // Component offer admission only, not fabricated funded/low-TL World facts.
    assert_eq!(
        ModuleAction::PurchaseFuelScoops.purchase(),
        Some((ModuleKind::FuelScoops, 19000, 4))
    );
    assert_eq!(
        validate_purchase(ModuleAction::PurchaseFuelScoops, 3, 19000),
        Err(ModuleError::BerthRequired)
    );
    assert_eq!(
        validate_purchase(ModuleAction::PurchaseFuelScoops, 4, 18999),
        Err(ModuleError::InsufficientCredits)
    );
    ok(validate_purchase(
        ModuleAction::PurchaseFuelScoops,
        4,
        19000,
    ));
    assert_eq!(
        (
            ModuleKind::FuelScoops.family(),
            ModuleKind::FuelScoops.node(),
            ModuleKind::FuelScoops.slot()
        ),
        (
            "utility-fuelScoops",
            "Fuel Scoops",
            ModuleSlot::UtilityFuelScoops
        )
    );
}
#[test]
fn fresh_sh025_support_does_not_grant_recommended_kit_or_a_free_scoop() {
    let w = ok(World::new_commander(12005));
    let view = some(w.fuel_scoops_snapshot());
    assert_eq!(
        (
            view.compatible_hull,
            view.utility_mounts_capacity,
            view.utility_mounts_used
        ),
        ("SH025", 3, 0)
    );
    assert!(view.owned.is_none() && view.fitted.is_none());
    let offer = some(view.current_offer);
    assert_eq!(
        (
            offer.price_tenths,
            offer.required_technology,
            offer.available
        ),
        (19000, 4, false)
    );
    let before = ok(w.save_bytes());
    assert_eq!(
        w.review_module(ModuleAction::PurchaseFuelScoops, 0),
        Err(ModuleError::InsufficientCredits)
    );
    assert_eq!(w.credits_tenths(), Some(7200));
    assert_eq!(ok(w.save_bytes()), before);
}
#[test]
fn actual_paid_purchase_preserves_born_pulse_shared_resources_cargo_and_emissions() {
    let mut w = funded();
    let front = w.pulse_laser_snapshot();
    let pools = w.flight_resources_snapshot();
    let combat = w.combat_snapshot();
    let cargo = w.service_snapshot();
    let cash = some(w.credits_tenths());
    let revision = ok(w.raw_service_projection()).revision;
    let journal = {
        let mut b = Writer::new();
        w.damage.encode(&mut b);
        b.bytes
    };
    let berth = some(w.current_dock_context());
    let item = apply(&mut w, ModuleAction::PurchaseFuelScoops);
    let view = some(w.fuel_scoops_snapshot());
    let owned = some(view.owned);
    let fit = some(view.fitted);
    assert_eq!(
        (owned.id, owned.kind, owned.slot),
        (
            item,
            ModuleKind::FuelScoops,
            Some(ModuleSlot::UtilityFuelScoops)
        )
    );
    assert!(
        matches!(owned.provenance, ModuleProvenance::Paid { transaction: 1, dock, paid_tenths: 19000, .. } if dock == berth)
    );
    assert_eq!(some(w.credits_tenths()), cash - 19000);
    assert_eq!(
        (fit.item, fit.account_revision, fit.fit_revision),
        (item, revision + 1, 1)
    );
    assert_eq!(view.utility_mounts_used, 1);
    assert!(!some(view.current_offer).available);
    assert_eq!(w.pulse_laser_snapshot(), front);
    assert_eq!(w.flight_resources_snapshot(), pools);
    assert_eq!(w.combat_snapshot(), combat);
    assert_eq!(w.service_snapshot(), cargo);
    let mut retained = Writer::new();
    w.damage.encode(&mut retained);
    assert_eq!(retained.bytes, journal);
    // Actual module cold-codec boundary. Full World Continue requires Root's pinned loader join.
    let ledger = ok(decoded(&w.modular, 16).ledger(&w, some(w.commander_account.as_ref())));
    assert_eq!(ledger.items, some(w.modular_equipment_snapshot()).items);
    apply(&mut w, ModuleAction::Store { item });
    let before = ok(w.save_bytes());
    let tick = w.snapshot().tick;
    assert_eq!(
        w.review_module(ModuleAction::PurchaseFuelScoops, tick),
        Err(ModuleError::Capacity)
    );
    assert_eq!(ok(w.save_bytes()), before);
    apply(
        &mut w,
        ModuleAction::Install {
            item,
            replacing: None,
        },
    );
    assert_eq!(some(w.fitted_fuel_scoops()).item, item);
    assert_eq!(w.pulse_laser_snapshot(), front);
    let laser = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    assert_eq!(some(w.fitted_fuel_scoops()).item, item);
    assert_eq!(some(w.fitted_mining_tool()).item, laser);
    assert_eq!(w.flight_resources_snapshot(), pools);
}
#[test]
fn typed_store_and_refit_preserve_actual_cargo_bay_and_mining_laser() {
    let mut w = funded();
    let t = w.snapshot().tick;
    ok(w.request_equipment(crate::equipment::Command::PurchaseCargoBay, t));
    let bay = w.equipment_snapshot();
    let laser = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    let item = apply(&mut w, ModuleAction::PurchaseFuelScoops);
    let cash = w.credits_tenths();
    let pools = w.flight_resources_snapshot();
    assert_eq!(some(w.fuel_scoops_snapshot()).utility_mounts_used, 2);
    apply(&mut w, ModuleAction::Store { item });
    assert!(w.fitted_fuel_scoops().is_none());
    let before = ok(w.save_bytes());
    let t = w.snapshot().tick;
    assert_eq!(
        w.review_module(
            ModuleAction::Install {
                item,
                replacing: Some(laser)
            },
            t
        ),
        Err(ModuleError::StaleOccupant)
    );
    assert_eq!(ok(w.save_bytes()), before);
    apply(
        &mut w,
        ModuleAction::Install {
            item,
            replacing: None,
        },
    );
    assert_eq!(some(w.fitted_fuel_scoops()).item, item);
    assert_eq!(some(w.fitted_mining_tool()).item, laser);
    assert_eq!(w.credits_tenths(), cash);
    assert_eq!(w.equipment_snapshot(), bay);
    assert_eq!(w.flight_resources_snapshot(), pools);
    assert_eq!(
        ok(decoded(&w.modular, 16).ledger(&w, some(w.commander_account.as_ref()))).paid,
        52000
    );
    assert_eq!(w.modular.paid_count(), 2);
}
#[test]
fn native_mount_limit_foreign_hull_and_wrong_typed_slot_refuse_at_fit_boundary() {
    // Negative component data only: no live hull, mount capacity or ownership is injected.
    let w = ok(World::new_commander(12005));
    let declaration = ok(crate::equipment::definition());
    let items = some(w.modular_equipment_snapshot()).items;
    assert_eq!(validate_slots(declaration, &items, 3), Ok(3));
    assert_eq!(
        validate_slots(declaration, &items, 4),
        Err(ModuleError::Capacity)
    );
    assert_eq!(
        validate_slots(
            crate::equipment::Definition {
                hull: "SH028",
                ..declaration
            },
            &items,
            0
        ),
        Err(ModuleError::StaleOccupant)
    );
    let mut wrong = items;
    wrong[0].slot = Some(ModuleSlot::UtilityFuelScoops);
    assert_eq!(
        validate_slots(declaration, &wrong, 0),
        Err(ModuleError::StaleOccupant)
    );
}
#[test]
fn refused_atomic_writer_discards_real_candidate_payment_item_and_fit_together() {
    let mut w = funded();
    let before = ok(w.save_bytes());
    let cash = some(w.credits_tenths());
    let t = w.snapshot().tick;
    let review = ok(w.review_module(ModuleAction::PurchaseFuelScoops, t));
    let prepared = ok(w.prepare_module(review, t));
    assert_eq!(prepared.account.credits_tenths(), cash - 19000);
    assert_eq!(decoded(&prepared.state, 16).events.len(), 1);
    assert_eq!(
        prepared.publish_with(|candidate| {
            assert!(!candidate.is_empty());
            Err::<(), _>("atomic write refused")
        }),
        Err("atomic write refused")
    );
    assert_eq!(ok(w.save_bytes()), before);
    assert!(w.fitted_fuel_scoops().is_none());
    assert!(some(w.fuel_scoops_snapshot()).owned.is_none());
}
#[test]
fn stale_pause_foreign_pre_continue_and_changed_account_reviews_never_publish() {
    let mut w = funded();
    let t = w.snapshot().tick;
    let review = ok(w.review_module(ModuleAction::PurchaseFuelScoops, t));
    let mut other = ok(World::new_commander(12006));
    let other_before = ok(other.save_bytes());
    let ot = other.snapshot().tick;
    assert!(other.prepare_module(review, ot).is_err());
    assert_eq!(ok(other.save_bytes()), other_before);
    let before = ok(w.save_bytes());
    assert_eq!(
        w.review_module(ModuleAction::PurchaseFuelScoops, t - 1),
        Err(ModuleError::StaleReview)
    );
    ok(w.request_pause(true, t));
    let paused = ok(w.save_bytes());
    assert!(matches!(
        w.prepare_module(review, t),
        Err(ModuleError::Paused)
    ));
    assert_eq!(ok(w.save_bytes()), paused);
    ok(w.request_pause(false, t));
    // Genuine old-tag World restore, before any scoop is owned; no new-format caller is assumed.
    ok(w.restore_bytes(&before));
    assert!(matches!(
        w.prepare_module(review, t),
        Err(ModuleError::StaleReview)
    ));
    let item = apply(&mut w, ModuleAction::PurchaseFuelScoops);
    let stale = ok(w.review_module(ModuleAction::Store { item }, t));
    let ammo = ok(w.review_ammunition(crate::AmmunitionAction::Purchase, t));
    ok(w.prepare_ammunition(ammo, t)).commit();
    let before = ok(w.save_bytes());
    assert!(matches!(
        w.prepare_module(stale, t),
        Err(ModuleError::StaleReview)
    ));
    assert_eq!(ok(w.save_bytes()), before);
    launch(&mut w);
    assert_eq!(
        w.review_module(ModuleAction::Store { item }, w.snapshot().tick),
        Err(ModuleError::BerthRequired)
    );
}
#[test]
fn actual_decoder_preserves_old_bytes_and_refuses_new_utility_in_thirteen_and_fifteen() {
    let mut w = funded();
    apply(&mut w, ModuleAction::PurchaseMiningLaser);
    let old = encoded(&w.modular);
    for version in [13, 15, 16] {
        assert_eq!(encoded(&decoded(&w.modular, version)), old);
    }
    let mut r = Reader::new(&old);
    assert_eq!(ok(State::decode(&mut r)), w.modular);
    assert!(r.finished());
    apply(&mut w, ModuleAction::PurchaseFuelScoops);
    let current = encoded(&w.modular);
    for version in [13, 15] {
        assert!(State::decode_version(&mut Reader::new(&current), version).is_err());
    }
    for version in [12, 14, 17, u16::MAX] {
        assert_eq!(
            State::decode_version(&mut Reader::new(&current), version),
            Err(SaveError::UnsupportedVersion)
        );
    }
    assert!(State::decode(&mut Reader::new(&current)).is_err());
    decoded(&w.modular, 16);
    assert!(State::decode_version(&mut Reader::new(&current[..current.len() - 1]), 16).is_err());
    let mut corrupt = current;
    corrupt[4] = 250;
    assert!(State::decode_version(&mut Reader::new(&corrupt), 16).is_err());
}
#[test]
fn genuine_fitted_scoops_still_refuse_missing_eligible_body_without_cargo_or_cash_grant() {
    let mut w = funded();
    apply(&mut w, ModuleAction::PurchaseFuelScoops);
    launch(&mut w);
    let before = ok(w.save_bytes());
    let cargo = w.service_snapshot();
    let cash = w.credits_tenths();
    assert_eq!(
        w.request_collect_mined_ore(w.snapshot().tick),
        Err(crate::mining::MiningError::NoEligibleOreBody)
    );
    assert_eq!(ok(w.save_bytes()), before);
    assert_eq!(w.service_snapshot(), cargo);
    assert_eq!(w.credits_tenths(), cash);
}
#[test]
fn altered_paid_identity_or_foreign_receipt_is_rejected_by_real_save_validation() {
    // Negative receipt corruption only; successful paid custody uses public commands.
    let mut w = funded();
    apply(&mut w, ModuleAction::PurchaseFuelScoops);
    let original = w.modular.clone();
    let before = ok(w.save_bytes());
    w.modular.events[0].item += 1;
    assert!(w.save_bytes().is_err());
    w.modular = original.clone();
    w.modular.events[0].dock = some(ok(World::new_commander(12006)).current_dock_context());
    assert!(w.save_bytes().is_err());
    w.modular = original;
    assert_eq!(ok(w.save_bytes()), before);
}

#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::equipment::tests::earned_world;
use crate::flight::practice::tests::{launch, step};
use crate::{ModuleAction, ModuleError, ModuleKind, ModuleProvenance, XyzzyAction, XyzzyError};

const BUY: ModuleAction = ModuleAction::Purchase {
    kind: ModuleKind::Ecm,
};

fn module(world: &mut World, action: ModuleAction) -> u64 {
    let tick = world.snapshot().tick;
    let review = world.review_module(action, tick).unwrap();
    let prepared = world.prepare_module(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    let item = prepared.commit();
    assert_eq!(world.save_bytes().unwrap(), bytes);
    assert_eq!(
        World::from_save_bytes(&bytes).unwrap().fitted_ecm(),
        world.fitted_ecm()
    );
    item
}
fn secret(world: &mut World, action: XyzzyAction) {
    let tick = world.snapshot().tick;
    let review = world.review_xyzzy(action, tick).unwrap();
    let prepared = world.prepare_xyzzy(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    prepared.commit();
    assert_eq!(world.save_bytes().unwrap(), bytes);
    assert_eq!(
        World::from_save_bytes(&bytes).unwrap().xyzzy_snapshot(),
        world.xyzzy_snapshot()
    );
}

#[test]
fn paid_unique_stock_offer_rollback_store_refit_and_shared_mount_budget() {
    let mut poor = World::new_commander(18501).unwrap();
    let offer = poor
        .module_offers()
        .into_iter()
        .find(|offer| offer.kind == ModuleKind::Ecm)
        .unwrap();
    assert_eq!(
        (
            offer.price_tenths,
            offer.required_technology,
            offer.equipment_units
        ),
        (21_000, 5, 1)
    );
    assert_eq!(
        poor.review_module(BUY, 0),
        Err(ModuleError::InsufficientCredits)
    );
    assert_eq!(poor.request_ecm(0), Err(EcmError::FlightRequired));
    launch(&mut poor);
    assert_eq!(
        poor.request_ecm(poor.snapshot().tick),
        Err(EcmError::MissingFit)
    );

    let mut world = earned_world(18502);
    let cash = world.credits_tenths().unwrap();
    let tick = world.snapshot().tick;
    let before = world.save_bytes().unwrap();
    let review = world.review_module(BUY, tick).unwrap();
    assert_eq!(
        world
            .prepare_module(review, tick)
            .unwrap()
            .publish_with(|_| Err::<(), _>("disk")),
        Err("disk")
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    let item = module(&mut world, BUY);
    assert_eq!(world.credits_tenths(), Some(cash - 21_000));
    assert!(matches!(
        world.fitted_ecm().unwrap().provenance,
        ModuleProvenance::Paid {
            paid_tenths: 21_000,
            ..
        }
    ));
    assert!(world.review_module(BUY, tick).is_err());
    module(&mut world, ModuleAction::Store { item });
    assert!(world.fitted_ecm().is_none());
    assert!(world.review_module(BUY, tick).is_err());
    module(
        &mut world,
        ModuleAction::Install {
            item,
            replacing: None,
        },
    );
    let fitted = world.modular_equipment_snapshot().unwrap();
    assert_eq!(
        (fitted.utility_mounts_used, fitted.equipment_units_used),
        (1, 1)
    );
    assert_eq!(world.fitted_ecm().unwrap().id, item);
    assert_eq!(world.credits_tenths(), Some(cash - 21_000));
}

#[test]
fn manual_field_exact_cost_same_clock_32_ticks_and_cold_resume() {
    let mut world = World::new_commander(18503).unwrap();
    secret(&mut world, XyzzyAction::Enter);
    let scoop = world.fitted_fuel_scoops().unwrap();
    module(&mut world, ModuleAction::Store { item: scoop.item });
    secret(&mut world, XyzzyAction::FitEcm);
    launch(&mut world);
    let tick = world.snapshot().tick;
    let before = world.flight_resources_snapshot().unwrap();
    let activation = world.request_ecm(tick).unwrap();
    assert_eq!(world.snapshot().tick, tick);
    assert_eq!(world.combat.physical_tick(), activation.physical_tick);
    let after = world.flight_resources_snapshot().unwrap();
    assert_eq!(after.energy_hundredths, before.energy_hundredths - 2200);
    assert_eq!(after.heat_fifths, before.heat_fifths);
    assert_eq!(after.cooldown_ticks, before.cooldown_ticks);
    assert_eq!(world.ecm_snapshot().unwrap().remaining_ticks, 32);
    let bytes = world.save_bytes().unwrap();
    assert_eq!(world.request_ecm(tick), Err(EcmError::FieldActive));
    assert_eq!(world.request_ecm(tick + 1), Err(EcmError::StaleCommand));
    assert_eq!(world.save_bytes().unwrap(), bytes);
    world = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(world.ecm_snapshot().unwrap().active, Some(activation));
    step(&mut world, 31);
    assert_eq!(world.ecm_snapshot().unwrap().remaining_ticks, 1);
    assert_eq!(
        world.request_ecm(world.snapshot().tick),
        Err(EcmError::FieldActive)
    );
    step(&mut world, 1);
    assert_eq!(world.ecm_snapshot().unwrap().remaining_ticks, 0);
    let next = world.request_ecm(world.snapshot().tick).unwrap();
    assert_eq!(next.sequence, activation.sequence + 1);
    assert_eq!(
        World::from_save_bytes(&world.save_bytes().unwrap())
            .unwrap()
            .ecm_snapshot(),
        world.ecm_snapshot()
    );
}

#[test]
fn guarded_automatic_intent_and_dedicated_fit_never_grant_unrelated_supply() {
    let mut world = World::new_commander(18504).unwrap();
    for action in [XyzzyAction::FitEcm, XyzzyAction::ToggleAutoEcm] {
        assert_eq!(world.review_xyzzy(action, 0), Err(XyzzyError::Inactive));
    }
    assert!(!world.ecm_snapshot().unwrap().xyzzy_automatic_requested);
    secret(&mut world, XyzzyAction::Enter);
    assert!(world.ecm_snapshot().unwrap().xyzzy_automatic_requested);
    assert_eq!(
        world.review_xyzzy(XyzzyAction::FitEcm, 0),
        Err(XyzzyError::UtilityFitConflict)
    );
    let cash = world.credits_tenths();
    let navigation = world.snapshot().navigation;
    let modules = world.modular_equipment_snapshot();
    secret(&mut world, XyzzyAction::ToggleAutoEcm);
    assert!(!world.ecm_snapshot().unwrap().xyzzy_automatic_requested);
    assert_eq!(world.modular_equipment_snapshot(), modules);
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(world.snapshot().navigation, navigation);
    let scoop = world.fitted_fuel_scoops().unwrap();
    module(&mut world, ModuleAction::Store { item: scoop.item });
    secret(&mut world, XyzzyAction::FitEcm);
    let item = world.fitted_ecm().unwrap();
    assert!(matches!(item.provenance, ModuleProvenance::XYZzy { .. }));
    assert_eq!(
        world.review_xyzzy(XyzzyAction::MaximumFuelStorage, 0),
        Err(XyzzyError::UtilityFitConflict)
    );
    secret(&mut world, XyzzyAction::FitEcm);
    assert_eq!(world.fitted_ecm(), Some(item));
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(world.snapshot().navigation, navigation);
    secret(&mut world, XyzzyAction::ToggleAutoEcm);
    assert!(world.ecm_snapshot().unwrap().xyzzy_automatic_requested);
    launch(&mut world);
    step(&mut world, 32);
    assert!(world.ecm_snapshot().unwrap().last_activation.is_none()); // No incoming producer.
}

#[test]
fn resource_threshold_is_strict_and_retained_history_cannot_be_removed_or_forged() {
    for energy in [2100_u16, 2200, 2201, 25600] {
        let mut writer = Writer::new();
        writer.u16(energy);
        writer.u16(117);
        writer.u8(9);
        let pulse = PulseResources::decode_save(&mut Reader::new(&writer.bytes)).unwrap();
        match pulse.prepare_ecm_discharge() {
            Ok(result) => {
                let mut output = Writer::new();
                result.encode_save(&mut output);
                let mut reader = Reader::new(&output.bytes);
                assert_eq!(
                    (
                        reader.u16().unwrap(),
                        reader.u16().unwrap(),
                        reader.u8().unwrap()
                    ),
                    (energy - 2200, 117, 9)
                );
            }
            Err(_) => assert!(energy <= 2200),
        }
    }
    let mut world = World::new_commander(18505).unwrap();
    secret(&mut world, XyzzyAction::Enter);
    let scoop = world.fitted_fuel_scoops().unwrap();
    module(&mut world, ModuleAction::Store { item: scoop.item });
    secret(&mut world, XyzzyAction::FitEcm);
    launch(&mut world);
    world.request_ecm(world.snapshot().tick).unwrap();
    world.request_pause(true, world.snapshot().tick).unwrap();
    assert_eq!(
        world.request_ecm(world.snapshot().tick),
        Err(EcmError::Paused)
    );
    world.request_pause(false, world.snapshot().tick).unwrap();
    let bytes = world.save_bytes().unwrap();
    // Strip only the tagged extension, keep the independent header marker, then re-checksum.
    let from = bytes.windows(8).position(|tag| tag == b"MECM0001").unwrap();
    let mut stripped = bytes[..from].to_vec();
    stripped.extend(crate::save::checksum(&stripped).to_le_bytes());
    assert!(World::from_save_bytes(&stripped).is_err());
    Arc::make_mut(&mut world.ecm.activations)[0].item += 1;
    let forged = world.unchecked_test_save().unwrap();
    assert!(World::from_save_bytes(&forged).is_err());
}

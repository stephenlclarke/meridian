#![allow(clippy::unwrap_used)]

use super::*;
use crate::FlightInput;
use crate::flight::equipment::tests::{earned_world, journey};
use crate::flight::practice::tests::launch;
use crate::navigation::{NavigationError, SystemId};

fn fitted(world: &World) -> OwnedModule {
    world
        .modular_equipment_snapshot()
        .unwrap()
        .items
        .into_iter()
        .find(|item| item.slot == Some(ModuleSlot::Drive))
        .unwrap()
}

fn apply(world: &mut World, action: ModuleAction) -> u64 {
    let tick = world.snapshot().tick;
    let review = world.review_module(action, tick).unwrap();
    let prepared = world.prepare_module(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    let restored = World::from_save_bytes(&bytes).unwrap();
    let item = prepared.commit();
    assert_eq!(fitted(world), fitted(&restored));
    assert_eq!(world.save_bytes().unwrap(), bytes);
    item
}

fn buy(grade: DriveGrade) -> ModuleAction {
    ModuleAction::Purchase {
        kind: ModuleKind::Drive(grade),
    }
}

#[test]
fn original_drive_prices_technology_prerequisites_and_foundation() {
    let world = World::new_commander(17001).unwrap();
    let basic = fitted(&world);
    assert_eq!(
        (basic.id, basic.kind, basic.provenance),
        (
            17007,
            ModuleKind::Drive(DriveGrade::Basic),
            ModuleProvenance::StandardBirth
        )
    );
    assert_eq!(
        world.review_module(ModuleAction::Store { item: basic.id }, 0),
        Err(ModuleError::DriveRequired)
    );
    for (grade, lower, price, technology, spool, distance) in [
        (
            DriveGrade::Intermediate,
            DriveGrade::Basic,
            50_000,
            10,
            300,
            480.0,
        ),
        (
            DriveGrade::Advanced,
            DriveGrade::Intermediate,
            150_000,
            13,
            60,
            360.0,
        ),
    ] {
        let kind = ModuleKind::Drive(grade);
        let offer = world
            .module_offers()
            .into_iter()
            .find(|offer| offer.kind == kind)
            .unwrap();
        assert_eq!(
            (
                offer.price_tenths,
                offer.required_technology,
                offer.prerequisite,
                offer.equipment_units
            ),
            (price, technology, Some(ModuleKind::Drive(lower)), 1)
        );
        assert_eq!(
            (grade.spool_ticks(), grade.mass_lock_distance()),
            (spool, distance)
        );
        assert_eq!(ModuleKind::from_tag(kind.tag()).unwrap(), kind);
        assert_eq!(
            validate_purchase(kind, technology - 1, price),
            Err(ModuleError::TechnologyUnavailable)
        );
        assert_eq!(
            validate_purchase(kind, technology, price - 1),
            Err(ModuleError::InsufficientCredits)
        );
        assert_eq!(validate_purchase(kind, technology, price), Ok(()));
    }
    assert_eq!(
        world.review_module(buy(DriveGrade::Basic), 0),
        Err(ModuleError::NotForSale)
    );
    assert_eq!(
        world.review_module(buy(DriveGrade::Intermediate), 0),
        Err(ModuleError::InsufficientCredits)
    );
}

#[test]
fn earned_intermediate_purchase_is_atomic_replaces_the_same_foundation_and_survives_real_travel() {
    let mut world = earned_world(17011);
    let cash = world.credits_tenths().unwrap();
    let basic = fitted(&world);
    let tick = world.snapshot().tick;
    let before = world.save_bytes().unwrap();
    let review = world
        .review_module(buy(DriveGrade::Intermediate), tick)
        .unwrap();
    assert_eq!(
        world
            .prepare_module(review, tick)
            .unwrap()
            .publish_with(|_| Err::<(), _>("disk")),
        Err("disk")
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    let paid = apply(&mut world, buy(DriveGrade::Intermediate));
    assert_eq!(world.credits_tenths(), Some(cash - 50_000));
    assert_eq!(fitted(&world).id, paid);
    assert!(matches!(
        fitted(&world).provenance,
        ModuleProvenance::Paid {
            paid_tenths: 50_000,
            ..
        }
    ));
    assert_eq!(
        world
            .modular_equipment_snapshot()
            .unwrap()
            .equipment_units_used,
        1
    );
    assert_eq!(
        world.review_module(ModuleAction::Store { item: paid }, tick),
        Err(ModuleError::DriveRequired)
    );
    apply(
        &mut world,
        ModuleAction::Install {
            item: basic.id,
            replacing: Some(paid),
        },
    );
    assert_eq!(fitted(&world), basic);
    apply(
        &mut world,
        ModuleAction::Install {
            item: paid,
            replacing: Some(basic.id),
        },
    );
    assert_eq!(world.credits_tenths(), Some(cash - 50_000));
    assert_eq!(
        world.review_module(buy(DriveGrade::Intermediate), tick),
        Err(ModuleError::InsufficientCredits)
    );
    world.request_refuel(tick).unwrap();
    launch(&mut world);
    let opening = world.snapshot().navigation.unwrap();
    world
        .request_jump(SystemId::AVENOR, world.snapshot().tick)
        .unwrap();
    assert_eq!(
        world
            .snapshot()
            .navigation
            .unwrap()
            .travel
            .unwrap()
            .remaining_ticks,
        300
    );
    for _ in 0..123 {
        world.step(FlightInput::default()).unwrap();
    }
    let mut world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(
        world
            .snapshot()
            .navigation
            .unwrap()
            .travel
            .unwrap()
            .remaining_ticks,
        177
    );
    assert_eq!(fitted(&world).id, paid);
    for _ in 0..176 {
        world.step(FlightInput::default()).unwrap();
    }
    assert_eq!(world.snapshot().navigation.unwrap().system, opening.system);
    world.step(FlightInput::default()).unwrap();
    let arrival = world.snapshot().navigation.unwrap();
    assert_eq!(
        (arrival.system, arrival.fuel_units, arrival.travel),
        (SystemId::AVENOR, opening.fuel_units - 34, None)
    );
    let saved = world.save_bytes().unwrap();
    assert_eq!(
        World::from_save_bytes(&saved)
            .unwrap()
            .snapshot()
            .navigation,
        Some(arrival)
    );
}

#[test]
fn paid_advanced_requires_fitted_predecessor_and_xyzz_refits_not_regrants_it() {
    // This checked-in historical test save has explicit XYZZY cash but predates drive grants.
    // It proves real paid debits separately from the earned Intermediate journey above.
    let mut world = World::from_save_bytes(include_bytes!(
        "../../../tests/fixtures/xyzzy-paid-base-v17.bin"
    ))
    .unwrap();
    if world.snapshot().navigation.unwrap().system != SystemId::RENAS {
        journey(&mut world, SystemId::RENAS);
    }
    let tick = world.snapshot().tick;
    let basic = fitted(&world);
    assert_eq!(basic.kind, ModuleKind::Drive(DriveGrade::Basic));
    assert_eq!(
        world.review_module(buy(DriveGrade::Advanced), tick),
        Err(ModuleError::MissingPrerequisite)
    );
    let cash = world.credits_tenths().unwrap();
    let intermediate = apply(&mut world, buy(DriveGrade::Intermediate));
    assert_eq!(
        world.review_module(buy(DriveGrade::Intermediate), tick),
        Err(ModuleError::Capacity)
    );
    apply(
        &mut world,
        ModuleAction::Install {
            item: basic.id,
            replacing: Some(intermediate),
        },
    );
    assert_eq!(
        world.review_module(buy(DriveGrade::Advanced), tick),
        Err(ModuleError::MissingPrerequisite)
    );
    apply(
        &mut world,
        ModuleAction::Install {
            item: intermediate,
            replacing: Some(basic.id),
        },
    );
    let advanced = apply(&mut world, buy(DriveGrade::Advanced));
    assert_eq!(world.credits_tenths(), Some(cash - 200_000));
    let paid = fitted(&world);
    assert_eq!(paid.id, advanced);
    apply(
        &mut world,
        ModuleAction::Install {
            item: intermediate,
            replacing: Some(advanced),
        },
    );
    let review = world
        .review_xyzzy(super::super::XyzzyAction::MaximumSupportedHardware, tick)
        .unwrap();
    world.prepare_xyzzy(review, tick).unwrap().commit();
    assert_eq!(fitted(&world), paid);
    assert_eq!(world.credits_tenths(), Some(cash - 200_000));
    launch(&mut world);
    assert_eq!(
        world.review_module(buy(DriveGrade::Basic), world.snapshot().tick),
        Err(ModuleError::BerthRequired)
    );
    world
        .request_jump(SystemId::AVENOR, world.snapshot().tick)
        .unwrap();
    assert_eq!(
        world
            .snapshot()
            .navigation
            .unwrap()
            .travel
            .unwrap()
            .remaining_ticks,
        60
    );
    for _ in 0..59 {
        world.step(FlightInput::default()).unwrap();
    }
    let pending = world.save_bytes().unwrap();
    let mut world = World::from_save_bytes(&pending).unwrap();
    assert_eq!(
        world.request_jump(SystemId::AVENOR, world.snapshot().tick),
        Err(NavigationError::AlreadyTravelling)
    );
    world.step(FlightInput::default()).unwrap();
    assert_eq!(
        world.snapshot().navigation.unwrap().system,
        SystemId::AVENOR
    );
    assert!(World::from_save_bytes(&world.save_bytes().unwrap()).is_ok());
}

#[test]
fn explicit_secret_outfit_grants_advanced_once_and_retains_the_born_basic() {
    let mut world = World::new_commander(17021).unwrap();
    let basic = fitted(&world);
    let tick = world.snapshot().tick;
    let action = super::super::XyzzyAction::Enter;
    let review = world.review_xyzzy(action, tick).unwrap();
    world.prepare_xyzzy(review, tick).unwrap().commit();
    let granted = fitted(&world);
    assert_eq!(granted.kind, ModuleKind::Drive(DriveGrade::Advanced));
    assert!(matches!(granted.provenance, ModuleProvenance::XYZzy { .. }));
    let inventory = world.modular_equipment_snapshot().unwrap();
    assert!(
        inventory
            .items
            .iter()
            .any(|item| item.id == basic.id && item.slot.is_none())
    );
    assert!(
        !inventory
            .items
            .iter()
            .any(|item| item.kind == ModuleKind::Drive(DriveGrade::Intermediate))
    );
    let review = world.review_xyzzy(action, tick).unwrap();
    world.prepare_xyzzy(review, tick).unwrap().commit();
    assert_eq!(fitted(&world), granted);
    assert_eq!(
        world.modular_equipment_snapshot().unwrap().items,
        inventory.items
    );
    assert_eq!(
        fitted(&World::from_save_bytes(&world.save_bytes().unwrap()).unwrap()),
        granted
    );
}

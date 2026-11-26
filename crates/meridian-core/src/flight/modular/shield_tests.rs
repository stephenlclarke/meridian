#![allow(clippy::unwrap_used)]

use super::*;
use crate::flight::equipment::tests::{earned_world, journey};
use crate::flight::missiles::tests::{fly_axis, stop};
use crate::flight::practice::tests::launch;
use crate::navigation::SystemId;
use crate::station_services::Command;
use crate::{FlightInput, ShieldGrade, XyzzyAction, XyzzyError};

fn fitted(world: &World) -> OwnedModule {
    world
        .modular_equipment_snapshot()
        .unwrap()
        .items
        .into_iter()
        .find(|item| item.slot == Some(ModuleSlot::Shield))
        .unwrap()
}

fn buy(grade: ShieldGrade) -> ModuleAction {
    ModuleAction::Purchase {
        kind: ModuleKind::Shield(grade),
    }
}

fn apply(world: &mut World, action: ModuleAction) -> u64 {
    let tick = world.snapshot().tick;
    let review = world.review_module(action, tick).unwrap();
    let prepared = world.prepare_module(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    let restored = World::from_save_bytes(&bytes).unwrap();
    let id = prepared.commit();
    assert_eq!(world.save_bytes().unwrap(), bytes);
    assert_eq!(fitted(world), fitted(&restored));
    assert_eq!(world.combat_snapshot(), restored.combat_snapshot());
    id
}

fn secret(world: &mut World, action: XyzzyAction) {
    let tick = world.snapshot().tick;
    let review = world.review_xyzzy(action, tick).unwrap();
    world.prepare_xyzzy(review, tick).unwrap().commit();
}

fn banks(world: &World, grade: ShieldGrade) {
    let player = world.combat_snapshot().unwrap().player;
    assert_eq!(player.shield_grade, grade);
    assert_eq!(player.shield_capacity_hundredths, grade.capacity());
    assert_eq!(
        [
            player.front_shield_hundredths,
            player.rear_shield_hundredths
        ],
        [grade.capacity(); 2]
    );
}

#[test]
fn actual_earned_paid_shields_preserve_ownership_budget_and_cold_refits() {
    let mut world = earned_world(18001);
    let mut cash = world.credits_tenths().unwrap();
    let original = fitted(&world);
    assert_eq!(
        (original.id, original.provenance),
        (18004, ModuleProvenance::StandardBirth)
    );
    assert_eq!(
        world.review_module(
            ModuleAction::Store { item: original.id },
            world.snapshot().tick
        ),
        Err(ModuleError::ShieldRequired)
    );
    banks(&world, ShieldGrade::Basic);
    let tick = world.snapshot().tick;
    let before = world.save_bytes().unwrap();
    let review = world
        .review_module(buy(ShieldGrade::BasicII), tick)
        .unwrap();
    assert_eq!(
        world
            .prepare_module(review, tick)
            .unwrap()
            .publish_with(|_| Err::<(), _>("disk")),
        Err("disk")
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    assert_eq!(
        world.review_module(buy(ShieldGrade::Intermediate), tick),
        Err(ModuleError::MissingPrerequisite)
    );
    let mut previous = original.id;
    for (grade, debit) in [
        (ShieldGrade::BasicII, 10_875),
        (ShieldGrade::Intermediate, 24_750),
        (ShieldGrade::IntermediateII, 35_250),
    ] {
        let id = apply(&mut world, buy(grade));
        cash -= debit;
        assert_eq!(world.credits_tenths(), Some(cash));
        assert_eq!(fitted(&world).id, id);
        assert!(
            matches!(fitted(&world).provenance, ModuleProvenance::Paid { paid_tenths, .. } if paid_tenths == debit)
        );
        let inventory = world.modular_equipment_snapshot().unwrap();
        assert_eq!(inventory.equipment_units_used, 2);
        assert!(
            inventory
                .items
                .iter()
                .any(|item| item.id == previous && item.slot.is_none())
        );
        banks(&world, grade);
        previous = id;
    }
    apply(
        &mut world,
        ModuleAction::Install {
            item: original.id,
            replacing: Some(previous),
        },
    );
    banks(&world, ShieldGrade::Basic);
    apply(
        &mut world,
        ModuleAction::Install {
            item: previous,
            replacing: Some(original.id),
        },
    );
    banks(&world, ShieldGrade::IntermediateII);
    assert_eq!(world.credits_tenths(), Some(cash));
}

#[test]
fn paid_advanced_requires_tl14_and_preserves_paid_identity_in_explicit_test_outfit() {
    // Real checked-in old XYZZY-funded base, predating shield grants. This is a
    // labelled disposable paid-path fixture, not ordinary-earned Advanced evidence.
    let mut world = World::from_save_bytes(include_bytes!(
        "../../../tests/fixtures/xyzzy-paid-base-v17.bin"
    ))
    .unwrap();
    let mut cash = world.credits_tenths().unwrap();
    let tick = world.snapshot().tick;
    for grade in [
        ShieldGrade::BasicII,
        ShieldGrade::Intermediate,
        ShieldGrade::IntermediateII,
    ] {
        apply(&mut world, buy(grade));
        cash -= grade.ordinary_offer().unwrap().0;
    }
    assert_eq!(world.credits_tenths(), Some(cash));
    if world.snapshot().navigation.unwrap().system != SystemId::LARERIA {
        assert_eq!(
            world.review_module(buy(ShieldGrade::Advanced), tick),
            Err(ModuleError::TechnologyUnavailable)
        );
        journey(&mut world, SystemId::LARERIA);
    }
    let advanced = apply(&mut world, buy(ShieldGrade::Advanced));
    cash -= 123_000;
    assert_eq!(world.credits_tenths(), Some(cash));
    banks(&world, ShieldGrade::Advanced);
    for grade in [ShieldGrade::AdvancedII, ShieldGrade::Elite] {
        assert_eq!(
            world.review_module(buy(grade), world.snapshot().tick),
            Err(ModuleError::NotForSale)
        );
    }
    let paid = fitted(&world);
    secret(&mut world, XyzzyAction::MaximumSupportedHardware);
    banks(&world, ShieldGrade::Elite);
    let before = world.credits_tenths();
    secret(&mut world, XyzzyAction::FitShield(ShieldGrade::Advanced));
    assert_eq!(fitted(&world), paid);
    assert_eq!(fitted(&world).id, advanced);
    assert_eq!(world.credits_tenths(), before);
    banks(&world, ShieldGrade::Advanced);
}

#[test]
fn each_explicit_shield_test_grade_is_durable_not_paid_and_maximum_remains_elite() {
    let mut world = World::new_commander(18011).unwrap();
    assert_eq!(
        world.review_xyzzy(XyzzyAction::FitShield(ShieldGrade::AdvancedII), 0),
        Err(XyzzyError::Inactive)
    );
    secret(&mut world, XyzzyAction::Enter);
    banks(&world, ShieldGrade::Elite);
    let elite = fitted(&world);
    let cash = world.credits_tenths();
    let navigation = world.snapshot().navigation;
    let ammunition = world.ammunition_snapshot();
    for grade in ShieldGrade::ALL {
        secret(&mut world, XyzzyAction::FitShield(grade));
        banks(&world, grade);
        if grade == ShieldGrade::Basic {
            assert_eq!(fitted(&world).provenance, ModuleProvenance::StandardBirth);
        } else {
            assert!(matches!(
                fitted(&world).provenance,
                ModuleProvenance::XYZzy { .. }
            ));
        }
        assert_eq!(world.credits_tenths(), cash);
        assert_eq!(world.snapshot().navigation, navigation);
        assert_eq!(world.ammunition_snapshot(), ammunition);
        let bytes = world.save_bytes().unwrap();
        let restored = World::from_save_bytes(&bytes).unwrap();
        assert_eq!(restored.combat_snapshot(), world.combat_snapshot());
        assert_eq!(fitted(&restored), fitted(&world));
    }
    assert_eq!(fitted(&world), elite);
    let inventory = world.modular_equipment_snapshot().unwrap().items;
    secret(&mut world, XyzzyAction::FitShield(ShieldGrade::Basic));
    secret(&mut world, XyzzyAction::MaximumSupportedHardware);
    assert_eq!(fitted(&world), elite);
    assert_eq!(world.modular_equipment_snapshot().unwrap().items, inventory);
}

#[test]
fn raised_shields_take_actual_native_incoming_hit_and_forged_grade_cannot_restore() {
    let mut world = World::new_commander(12001).unwrap();
    secret(&mut world, XyzzyAction::Enter);
    secret(&mut world, XyzzyAction::FitShield(ShieldGrade::AdvancedII));
    world
        .request_station_service(Command::BuyFood, world.snapshot().tick)
        .unwrap();
    launch(&mut world);
    stop(&mut world);
    fly_axis(&mut world, 0, -600.0, 10.0);
    let mut contacted = false;
    for _ in 0..24000 {
        world.step(FlightInput::default()).unwrap();
        if world
            .traffic_snapshot()
            .is_some_and(|scene| scene.last_player_hit.is_some())
        {
            contacted = true;
            break;
        }
    }
    assert!(contacted, "no actual incoming NPC Pulse");
    let player = world.combat_snapshot().unwrap().player;
    let mut remaining = [
        player.front_shield_hundredths,
        player.rear_shield_hundredths,
    ];
    remaining.sort();
    assert_eq!(remaining, [205_500, 207_000]);
    assert_eq!(player.energy_hundredths, 38_000);
    let before = world.save_bytes().unwrap();
    let mut forged = World::from_save_bytes(&before).unwrap();
    assert_eq!(forged.combat_snapshot(), world.combat_snapshot());
    forged.combat.refit_shields(ShieldGrade::Elite);
    assert!(forged.save_bytes().is_err());
    assert!(
        world
            .restore_bytes(&forged.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(world.save_bytes().unwrap(), before);
}

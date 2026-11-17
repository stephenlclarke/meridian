#![allow(clippy::unwrap_used)]

use super::*;
use crate::FlightInput;
use crate::flight::equipment::tests::{earned_world, journey};
use crate::flight::practice::tests::launch;
use crate::navigation::{BASIC_SPOOL_TICKS, NavigationError, SystemId};

const ENLIRIN: SystemId = SystemId {
    galaxy: 0,
    system: 162,
};

fn transact(world: &mut World, action: ModuleAction) -> u64 {
    let tick = world.snapshot().tick;
    let review = world.review_module(action, tick).unwrap();
    let prepared = world.prepare_module(review, tick).unwrap();
    let candidate = World::from_save_bytes(prepared.bytes()).unwrap();
    let id = prepared.commit();
    assert_eq!(
        candidate.modular_equipment_snapshot(),
        world.modular_equipment_snapshot()
    );
    assert_eq!(candidate.credits_tenths(), world.credits_tenths());
    assert_eq!(candidate.jump_reach_ly(), world.jump_reach_ly());
    id
}

#[test]
fn original_navigation_offers_gate_real_fits_and_preserve_stock_jump_reach() {
    let world = World::new_commander(15800).unwrap();
    let offers = world.module_offers();
    for (kind, price_tenths, technology, prerequisite) in [
        (
            ModuleKind::IntermediateNavigation,
            30_000,
            8,
            ModuleKind::BasicNavigation,
        ),
        (
            ModuleKind::AdvancedNavigation,
            90_000,
            11,
            ModuleKind::IntermediateNavigation,
        ),
    ] {
        let offer = offers.iter().find(|offer| offer.kind == kind).unwrap();
        assert_eq!(
            (
                offer.price_tenths,
                offer.required_technology,
                offer.prerequisite
            ),
            (price_tenths, technology, Some(prerequisite)),
        );
        assert_eq!(offer.slot, ModuleSlot::Navigation);
        assert!(!offer.available);
        assert_eq!(
            validate_purchase(kind, technology - 1, price_tenths),
            Err(ModuleError::TechnologyUnavailable)
        );
        assert_eq!(validate_purchase(kind, technology, price_tenths), Ok(()));
    }
    assert!(
        offers
            .iter()
            .all(|offer| offer.kind != ModuleKind::BasicNavigation)
    );
    assert_eq!(world.jump_reach_ly(), Some(9.0));
    let mut stock = world;
    launch(&mut stock);
    let before = stock.save_bytes().unwrap();
    assert_eq!(
        stock.request_jump(ENLIRIN, stock.snapshot().tick),
        Err(NavigationError::OutOfRange)
    );
    assert_eq!(stock.save_bytes().unwrap(), before);
    assert!(
        stock
            .request_jump(SystemId::RENAS, stock.snapshot().tick)
            .is_ok()
    );
}

#[test]
fn paid_intermediate_fit_extends_real_travel_and_cold_pending_save_charges_once() {
    let mut world = earned_world(15801);
    world.request_refuel(world.snapshot().tick).unwrap();
    journey(&mut world, SystemId::AVENOR);
    let cash = world.credits_tenths().unwrap();
    let ammo = world.ammunition_snapshot();
    let resources = world.flight_resources_snapshot();
    let basic = world
        .modular_equipment_snapshot()
        .unwrap()
        .items
        .into_iter()
        .find(|item| item.kind == ModuleKind::BasicNavigation)
        .unwrap();
    assert_eq!(basic.slot, Some(ModuleSlot::Navigation));
    assert_eq!(basic.id, 15805);
    let buy = ModuleAction::Purchase {
        kind: ModuleKind::IntermediateNavigation,
    };
    let item = transact(&mut world, buy);
    assert_eq!(world.credits_tenths(), Some(cash - 30_000));
    assert_eq!(world.ammunition_snapshot(), ammo);
    assert_eq!(world.flight_resources_snapshot(), resources);
    assert_eq!(world.jump_reach_ly(), Some(14.0));
    let fit = world.modular_equipment_snapshot().unwrap();
    assert!(
        fit.items
            .iter()
            .any(|old| old.id == basic.id && old.slot.is_none())
    );
    let paid = fit.items.iter().find(|owned| owned.id == item).unwrap();
    assert_eq!(paid.slot, Some(ModuleSlot::Navigation));
    assert!(matches!(
        paid.provenance,
        ModuleProvenance::Paid {
            paid_tenths: 30_000,
            ..
        }
    ));
    assert!(matches!(
        world.review_module(buy, world.snapshot().tick),
        Err(ModuleError::Capacity)
    ));

    transact(&mut world, ModuleAction::Store { item });
    assert_eq!(world.jump_reach_ly(), Some(9.0));
    transact(
        &mut world,
        ModuleAction::Install {
            item: basic.id,
            replacing: None,
        },
    );
    assert_eq!(world.jump_reach_ly(), Some(9.0));
    let before = world.save_bytes().unwrap();
    assert!(matches!(
        world.review_module(
            ModuleAction::Install {
                item,
                replacing: None
            },
            world.snapshot().tick
        ),
        Err(ModuleError::StaleOccupant),
    ));
    assert_eq!(world.save_bytes().unwrap(), before);
    transact(
        &mut world,
        ModuleAction::Install {
            item,
            replacing: Some(basic.id),
        },
    );
    assert_eq!(world.jump_reach_ly(), Some(14.0));

    let mut underfueled = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    launch(&mut underfueled);
    assert_eq!(underfueled.snapshot().navigation.unwrap().fuel_units, 96);
    let before = underfueled.save_bytes().unwrap();
    assert_eq!(
        underfueled.request_jump(ENLIRIN, underfueled.snapshot().tick),
        Err(NavigationError::InsufficientFuel)
    );
    assert_eq!(underfueled.save_bytes().unwrap(), before);

    world.request_refuel(world.snapshot().tick).unwrap();
    launch(&mut world);
    world.request_jump(ENLIRIN, world.snapshot().tick).unwrap();
    world.step(FlightInput::default()).unwrap();
    let mut world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let before = world.snapshot().navigation.unwrap();
    assert_eq!(before.fuel_units, 130);
    let travel = before.travel.unwrap();
    assert_eq!(
        (travel.destination, travel.remaining_ticks),
        (ENLIRIN, BASIC_SPOOL_TICKS - 1)
    );
    for _ in 0..travel.remaining_ticks {
        world.step(FlightInput::default()).unwrap();
    }
    let after = world.snapshot().navigation.unwrap();
    assert_eq!(
        (after.system, after.fuel_units, after.travel),
        (ENLIRIN, 32, None)
    );
    assert!(world.current_dock_context().is_none());
    crate::flight::persistence::assert_resealed_enlirin_fuel_refusal(&mut world);
    let mut continued = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    continued.step(FlightInput::default()).unwrap();
    assert_eq!(continued.snapshot().navigation, Some(after));
}

#[test]
fn advanced_navigation_requires_the_owned_intermediate_grade() {
    let world = World::new_commander(15802).unwrap();
    let dock = world.current_dock_context().unwrap();
    let mut ledger = Ledger::born(dock.owner().seed()).unwrap();
    let purchase = |kind, item, revision| Event {
        action: ModuleAction::Purchase { kind },
        item,
        dock,
        tick: 0,
        revision,
    };
    let advanced = purchase(ModuleKind::AdvancedNavigation, 200002, 2);
    assert_eq!(
        ledger.apply(advanced, 2),
        Err(ModuleError::MissingPrerequisite)
    );
    ledger
        .apply(purchase(ModuleKind::IntermediateNavigation, 200001, 1), 1)
        .unwrap();
    ledger.apply(advanced, 2).unwrap();
    assert_eq!(ledger.jump_reach_ly(), 21.0);
    assert_eq!(ledger.paid, 120_000);
    assert_eq!(ledger.fitted_units(), 0);
}

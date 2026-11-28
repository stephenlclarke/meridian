#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::equipment::tests::{earned_world, journey};
use crate::freight::g02::{Choice, Command, Event as FreightEvent};
use crate::navigation::{FUEL_CAPACITY, MAX_FUEL_CAPACITY, SystemId};
use crate::{XyzzyAction, XyzzyError};

const BUY: ModuleAction = ModuleAction::Purchase {
    kind: ModuleKind::FuelStorage,
};

fn tank_items(world: &World) -> Vec<OwnedModule> {
    world
        .modular_equipment_snapshot()
        .unwrap()
        .items
        .into_iter()
        .filter(|item| item.kind == ModuleKind::FuelStorage)
        .collect()
}
fn fuel(world: &World) -> (u16, u16) {
    let nav = world.snapshot().navigation.unwrap();
    (nav.fuel_units, nav.fuel_capacity_units)
}
fn transact(world: &mut World, action: ModuleAction) -> u64 {
    let tick = world.snapshot().tick;
    let review = world.review_module(action, tick).unwrap();
    let prepared = world.prepare_module(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    let result = prepared.commit();
    assert_eq!(world.save_bytes().unwrap(), bytes);
    let cold = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(tank_items(world), tank_items(&cold));
    assert_eq!(fuel(world), fuel(&cold));
    result
}
fn secret(world: &mut World, action: XyzzyAction) {
    let tick = world.snapshot().tick;
    let review = world.review_xyzzy(action, tick).unwrap();
    let prepared = world.prepare_xyzzy(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    prepared.commit();
    assert_eq!(world.save_bytes().unwrap(), bytes);
    let cold = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(tank_items(world), tank_items(&cold));
    assert_eq!(fuel(world), fuel(&cold));
}
fn accept_review(world: &mut World) -> crate::freight::g02::Review {
    let tick = world.snapshot().tick;
    let npc = world.station_contacts()[0];
    world.request_contact(npc, tick).unwrap();
    match world
        .request_freight(Command::ReviewOffer { actor: npc.actor() }, tick)
        .unwrap()
    {
        FreightEvent::Reviewed(review) => review,
        other => panic!("unexpected offer {other:?}"),
    }
}

#[test]
fn earned_four_distinct_paid_tanks_only_three_fit_and_refuel_is_separate() {
    let poor = World::new_commander(18150).unwrap();
    assert_eq!(
        poor.review_module(BUY, 0),
        Err(ModuleError::InsufficientCredits)
    );
    let mut world = earned_world(18151);
    let opening_fuel = fuel(&world).0;
    let opening_cash = world.credits_tenths().unwrap();
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
    let first = transact(&mut world, BUY);
    let second = transact(&mut world, BUY);
    let third = transact(&mut world, BUY);
    assert_eq!(fuel(&world), (opening_fuel, 340));
    assert_eq!(world.credits_tenths(), Some(opening_cash - 60_000));
    assert_eq!(world.review_module(BUY, tick), Err(ModuleError::Capacity));
    let fit = world.modular_equipment_snapshot().unwrap();
    assert_eq!(
        (
            fit.utility_mounts_used,
            fit.utility_mounts_capacity,
            fit.equipment_units_used
        ),
        (3, 3, 3)
    );
    transact(&mut world, ModuleAction::Store { item: first });
    let fourth = transact(&mut world, BUY);
    assert_eq!(world.credits_tenths(), Some(opening_cash - 80_000));
    let items = tank_items(&world);
    assert_eq!(
        items.iter().map(|item| item.id).collect::<Vec<_>>(),
        [first, second, third, fourth]
    );
    for (index, item) in items.iter().enumerate() {
        assert!(
            matches!(item.provenance, ModuleProvenance::Paid { transaction, paid_tenths: 20_000, .. } if transaction == index as u32 + 1)
        );
    }
    let cash = world.credits_tenths().unwrap();
    let debit = u32::from(340 - opening_fuel) * 2;
    assert_eq!(world.request_refuel(tick), Ok(debit));
    assert_eq!(world.credits_tenths(), Some(cash - debit));
    assert_eq!(fuel(&world), (340, 340));
    assert_eq!(
        world.review_module(ModuleAction::Store { item: second }, tick),
        Err(ModuleError::FuelCapacityOccupied)
    );
    assert_eq!(
        world.review_module(
            ModuleAction::Install {
                item: first,
                replacing: None
            },
            tick
        ),
        Err(ModuleError::Capacity)
    );
    let mut cold = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let distance = crate::navigation::OriginalNavigation::bundled()
        .unwrap()
        .distance_ly(SystemId::RENAS, SystemId::LARERIA)
        .unwrap();
    let jump_fuel = crate::navigation::stock_jump_fuel_cost(distance).unwrap();
    journey(&mut cold, SystemId::LARERIA);
    assert_eq!(fuel(&cold), (340 - jump_fuel, 340));
    assert!(fuel(&cold).0 <= 270);
    transact(&mut cold, ModuleAction::Store { item: second });
    assert_eq!(fuel(&cold), (340 - jump_fuel, 270));
    transact(
        &mut cold,
        ModuleAction::Install {
            item: first,
            replacing: None,
        },
    );
    assert_eq!(fuel(&cold), (340 - jump_fuel, 340));
    assert_eq!(cold.credits_tenths(), Some(cash - debit));
}

#[test]
fn medusa_stores_paid_tanks_at_base_capacity_and_refuses_one_tenth_over() {
    let mut world = earned_world(18157);
    world.request_refuel(world.snapshot().tick).unwrap();
    let ids = [transact(&mut world, BUY), transact(&mut world, BUY)];
    let paid = tank_items(&world);
    assert_eq!(fuel(&world), (130, 270));
    let verify_exchange = |world: &mut World| {
        secret(world, XyzzyAction::ActivateMedusa);
        let first = world.xyzzy_medusa_issue();
        let cash = world.credits_tenths();
        secret(world, XyzzyAction::ActivateMedusa);
        let continued = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
        assert_eq!(fuel(&continued), (130, 130));
        assert_eq!(continued.xyzzy_medusa_issue(), first);
        assert_eq!(continued.credits_tenths(), cash);
        assert!(continued.xyzzy_medusa_active());
        let stored = tank_items(&continued);
        assert_eq!(stored.len(), ids.len());
        for (index, item) in stored.iter().enumerate() {
            assert_eq!(item.id, ids[index]);
            assert_eq!(item.provenance, paid[index].provenance);
            assert!(matches!(item.provenance, ModuleProvenance::Paid { .. }));
            assert!(item.slot.is_none());
        }
    };
    let mut exact = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    verify_exchange(&mut exact);
    world.request_refuel(world.snapshot().tick).unwrap();
    journey(&mut world, SystemId::LARERIA);
    crate::flight::practice::tests::launch(&mut world);
    let vemar = SystemId {
        galaxy: 0,
        system: 101,
    };
    for destination in [vemar, SystemId::LARERIA] {
        world
            .request_jump(destination, world.snapshot().tick)
            .unwrap();
        let remaining = world
            .snapshot()
            .navigation
            .unwrap()
            .travel
            .unwrap()
            .remaining_ticks;
        for _ in 0..remaining {
            world.step(crate::FlightInput::default()).unwrap();
        }
    }
    crate::flight::missiles::tests::dock(&mut world);
    assert_eq!(fuel(&world), (131, 270));
    let before = world.save_bytes().unwrap();
    let err = world
        .review_xyzzy(XyzzyAction::ActivateMedusa, world.snapshot().tick)
        .unwrap_err();
    assert_eq!(
        err,
        XyzzyError::MedusaFuelOverCapacity {
            carried: 131,
            capacity: 130
        }
    );
    for quantity in ["13.1 LY", "13.0 LY", "0.1 LY"] {
        assert!(err.to_string().contains(quantity));
    }
    assert_eq!(world.save_bytes().unwrap(), before);
    assert_eq!(tank_items(&world), paid);
    assert!(!world.xyzzy_medusa_active());
    journey(&mut world, SystemId::RENAS);
    assert_eq!(fuel(&world), (58, 270));
    verify_exchange(&mut world);
}

#[test]
fn explicit_max_preserves_paid_identity_grants_only_missing_tanks_fills_then_repeats() {
    let mut world = earned_world(18152);
    let paid = transact(&mut world, BUY);
    let old = tank_items(&world)[0];
    secret(&mut world, XyzzyAction::MaximumFuelStorage);
    let fitted = tank_items(&world);
    assert_eq!(fitted.len(), 4);
    assert_eq!(fitted[0], old);
    assert_eq!(fitted[0].id, paid);
    assert!(
        fitted[1..]
            .iter()
            .all(|item| matches!(item.provenance, ModuleProvenance::XYZzy { .. }))
    );
    assert_eq!(fuel(&world), (MAX_FUEL_CAPACITY, MAX_FUEL_CAPACITY));
    assert!(world.test_tank_overfit());
    let snapshot = world.modular_equipment_snapshot().unwrap();
    assert_eq!(
        (
            snapshot.utility_mounts_used,
            snapshot.utility_mounts_capacity
        ),
        (7, 3)
    );
    assert_eq!(
        world.credits_tenths(),
        Some(super::super::STARTING_CREDITS_TENTHS)
    );
    let tick = world.snapshot().tick;
    assert_eq!(world.review_module(BUY, tick), Err(ModuleError::Capacity));
    assert_eq!(
        world.review_module(ModuleAction::Store { item: paid }, tick),
        Err(ModuleError::FuelCapacityOccupied)
    );
    secret(&mut world, XyzzyAction::MaximumFuelStorage);
    assert_eq!(tank_items(&world), fitted);
    let mut world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    secret(&mut world, XyzzyAction::ToggleUnlimitedFuel);
    journey(&mut world, SystemId::LARERIA);
    assert!(fuel(&world).0 <= 340);
    transact(&mut world, ModuleAction::Store { item: paid });
    assert_eq!(fuel(&world).1, 340);
    let tick = world.snapshot().tick;
    assert_eq!(
        world.review_module(
            ModuleAction::Install {
                item: paid,
                replacing: None
            },
            tick
        ),
        Err(ModuleError::Capacity)
    );
    secret(&mut world, XyzzyAction::MaximumFuelStorage);
    assert_eq!(tank_items(&world), fitted);
    assert_eq!(fuel(&world), (410, 410));
}

#[test]
fn active_real_freight_blocks_test_max_and_new_custody_cannot_use_overfit() {
    let mut world = World::new_commander(18153).unwrap();
    let review = accept_review(&mut world);
    let tick = world.snapshot().tick;
    let job = match world
        .request_freight(
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            tick,
        )
        .unwrap()
    {
        FreightEvent::Accepted { job, .. } => job,
        event => panic!("unexpected accept {event:?}"),
    };
    assert_eq!(
        world.review_xyzzy(XyzzyAction::MaximumFuelStorage, tick),
        Err(XyzzyError::ActiveContractCustody)
    );
    let mut world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(world.active_freight_job(), Some(job));
    assert_eq!(
        world.review_xyzzy(XyzzyAction::MaximumFuelStorage, tick),
        Err(XyzzyError::ActiveContractCustody)
    );
    let npc = world.station_contacts()[0];
    world.request_contact(npc, tick).unwrap();
    let review = match world
        .request_freight(
            Command::ReviewAbandonment {
                actor: npc.actor(),
                job,
            },
            tick,
        )
        .unwrap()
    {
        FreightEvent::Reviewed(review) => review,
        event => panic!("unexpected abandon review {event:?}"),
    };
    world
        .request_freight(
            Command::Confirm {
                review,
                choice: Choice::Abandon,
            },
            tick,
        )
        .unwrap();
    assert_eq!(world.active_freight_job(), None);
    secret(&mut world, XyzzyAction::MaximumFuelStorage);
    assert_eq!(fuel(&world), (410, 410));
    let new = accept_review(&mut world);
    assert_eq!(
        world.request_freight(
            Command::Confirm {
                review: new,
                choice: Choice::Accept
            },
            tick
        ),
        Err(crate::MissionError::TestFit)
    );
    assert_eq!(world.active_freight_job(), None);
    let cold = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert!(cold.test_tank_overfit());
}

#[test]
fn packed_account_fuel_rejects_foreign_count_bits_and_preserves_stock_bytes() {
    use crate::navigation::{fuel_capacity, pack_fuel, unpack_fuel};
    assert_eq!(pack_fuel(FUEL_CAPACITY, 0), FUEL_CAPACITY);
    for count in 0..=4 {
        let capacity = fuel_capacity(count);
        for units in [0, 1, capacity] {
            assert_eq!(unpack_fuel(pack_fuel(units, count)), Ok((units, count)));
        }
        assert!(unpack_fuel(pack_fuel(capacity + 1, count)).is_err());
    }
    assert!(unpack_fuel(5 << 9).is_err());
    assert!(unpack_fuel(1 << 15).is_err());
}

#[test]
fn ordinary_paid_tank_then_limited_outfit_keeps_tank_and_real_mount_budget() {
    let mut world = earned_world(18154);
    transact(&mut world, BUY);
    let paid = tank_items(&world);
    let old_fuel = fuel(&world).0;
    assert!(old_fuel <= 200);
    for action in [
        XyzzyAction::Enter,
        XyzzyAction::MaximumSupportedHardware,
        XyzzyAction::RestoreStarter,
    ] {
        secret(&mut world, action);
        assert_eq!(tank_items(&world), paid);
        assert_eq!(fuel(&world), (200, 200));
        let snapshot = world.modular_equipment_snapshot().unwrap();
        assert_eq!(
            (
                snapshot.utility_mounts_used,
                snapshot.utility_mounts_capacity
            ),
            (3, 3)
        );
        assert!(!world.test_tank_overfit());
        assert!(world.fitted_fuel_scoops().is_some());
        assert!(world.owned_emp().is_some_and(|item| item.fitted));
        assert!(world.equipment_snapshot().unwrap().owned.is_none());
        assert_eq!(
            world.credits_tenths(),
            Some(super::super::STARTING_CREDITS_TENTHS)
        );
    }
}

#[test]
fn a_closed_freight_issue_cannot_be_relabelled_into_an_earlier_hidden_test_overfit() {
    let mut world = World::new_commander(18155).unwrap();
    secret(&mut world, XyzzyAction::MaximumFuelStorage);
    secret(&mut world, XyzzyAction::ToggleUnlimitedFuel);
    for destination in [
        SystemId::LARERIA,
        SystemId::AVENOR,
        SystemId::LARERIA,
        SystemId::AVENOR,
    ] {
        journey(&mut world, destination);
    }
    let tanks = tank_items(&world);
    for item in &tanks[..3] {
        transact(&mut world, ModuleAction::Store { item: item.id });
    }
    assert_eq!(fuel(&world).1, 200);
    assert!(world.test_tank_overfit()); // Cargo, scoop, EMP and remaining tank = 4/3.
    let specialist = world
        .modular_equipment_snapshot()
        .unwrap()
        .items
        .into_iter()
        .find(|item| item.kind == ModuleKind::SalvageTargeting)
        .unwrap();
    transact(
        &mut world,
        ModuleAction::Store {
            item: specialist.id,
        },
    );
    let hidden_revision = world.commander_account.as_ref().unwrap().revision();
    let cargo = world.equipment_snapshot().unwrap().owned.unwrap();
    world
        .request_equipment(
            crate::equipment::Command::UnfitCargoBay(cargo.id),
            world.snapshot().tick,
        )
        .unwrap();
    assert!(!world.test_tank_overfit());
    world.request_refuel(world.snapshot().tick).unwrap();
    let tick = world.snapshot().tick;
    let review = accept_review(&mut world);
    let job = match world
        .request_freight(
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            tick,
        )
        .unwrap()
    {
        FreightEvent::Accepted { job, .. } => job,
        event => panic!("unexpected accept {event:?}"),
    };
    let review = match world
        .request_freight(
            Command::ReviewAbandonment {
                actor: world.station_contacts()[0].actor(),
                job,
            },
            tick,
        )
        .unwrap()
    {
        FreightEvent::Reviewed(review) => review,
        event => panic!("unexpected abandon {event:?}"),
    };
    world
        .request_freight(
            Command::Confirm {
                review,
                choice: Choice::Abandon,
            },
            tick,
        )
        .unwrap();
    assert!(!world.test_tank_overfit());
    let bytes = world.save_bytes().unwrap();
    let mut invalid = World::from_save_bytes(&bytes).unwrap();
    invalid
        .freight_session
        .as_mut()
        .unwrap()
        .state
        .forge_issue_revision_for_test(hidden_revision);
    assert!(
        invalid
            .modular
            .validate_test_custody_history(
                &invalid,
                invalid.commander_account.as_ref().unwrap(),
                &invalid.emp,
                &invalid.navigation_trade
            )
            .is_err()
    );
    assert!(invalid.save_bytes().is_err());
    assert!(
        world
            .restore_bytes(&invalid.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(world.save_bytes().unwrap(), bytes);
    let mut erased = World::from_save_bytes(&bytes).unwrap();
    erased
        .freight_session
        .as_mut()
        .unwrap()
        .state
        .clear_last_custody_markers_for_test();
    assert!(
        erased
            .modular
            .validate_test_custody_history(
                &erased,
                erased.commander_account.as_ref().unwrap(),
                &erased.emp,
                &erased.navigation_trade
            )
            .is_err()
    );
    assert!(erased.save_bytes().is_err());
    assert!(
        world
            .restore_bytes(&erased.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(world.save_bytes().unwrap(), bytes);

    for action in [
        crate::NavigationTradeAction::Accept,
        crate::NavigationTradeAction::Cancel,
    ] {
        let tick = world.snapshot().tick;
        let review = world.review_navigation_trade(action, tick).unwrap();
        world
            .prepare_navigation_trade(review, tick)
            .unwrap()
            .commit();
    }
    let bytes = world.save_bytes().unwrap();
    let mut erased = World::from_save_bytes(&bytes).unwrap();
    erased
        .navigation_trade
        .clear_last_custody_markers_for_test();
    assert!(
        erased
            .modular
            .validate_test_custody_history(
                &erased,
                erased.commander_account.as_ref().unwrap(),
                &erased.emp,
                &erased.navigation_trade
            )
            .is_err()
    );
    assert!(erased.save_bytes().is_err());
    assert!(
        world
            .restore_bytes(&erased.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(world.save_bytes().unwrap(), bytes);
}

#[test]
fn proved_completed_pre_max_legacy_freight_without_issue_marker_remains_readable() {
    let mut world = World::new_commander(18156).unwrap();
    let review = accept_review(&mut world);
    let tick = world.snapshot().tick;
    let job = match world
        .request_freight(
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            tick,
        )
        .unwrap()
    {
        FreightEvent::Accepted { job, .. } => job,
        event => panic!("unexpected accept {event:?}"),
    };
    journey(&mut world, SystemId::RENAS);
    let tick = world.snapshot().tick;
    let npc = world.station_contacts()[0];
    world.request_contact(npc, tick).unwrap();
    let review = match world
        .request_freight(
            Command::ReviewReceiving {
                actor: npc.actor(),
                job,
            },
            tick,
        )
        .unwrap()
    {
        FreightEvent::Reviewed(review) => review,
        event => panic!("unexpected receiving review {event:?}"),
    };
    world
        .request_freight(
            Command::Confirm {
                review,
                choice: Choice::ReceiveAll,
            },
            tick,
        )
        .unwrap();
    world
        .freight_session
        .as_mut()
        .unwrap()
        .state
        .clear_last_custody_markers_for_test();
    let before = world.mission_snapshot().unwrap().freight;
    let mut old = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    secret(&mut old, XyzzyAction::MaximumFuelStorage);
    assert_eq!(old.mission_snapshot().unwrap().freight, before);
}

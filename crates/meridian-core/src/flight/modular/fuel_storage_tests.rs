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

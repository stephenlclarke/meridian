//! Ordinary paid auto-work acceptance through the public World command boundary.
use meridian_core::{
    FlightInput, FlightMode, ModuleAction, ModuleKind, ModuleProvenance, TargetFilter,
    TargetIdentity, TargetingViewport, World,
    equipment::{TargetingTier, WorkTargetId},
    navigation::SystemId,
    station_services::{Command, CommodityId},
};

fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}

fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("required public World value"))
}

fn dock_at(world: &mut World, destination: SystemId) {
    ok(world.request_launch(world.snapshot().tick));
    ok(world.step(FlightInput::default()));
    ok(world.request_jump(destination, world.snapshot().tick));
    let remaining = some(some(world.snapshot().navigation).travel).remaining_ticks;
    for _ in 0..remaining {
        ok(world.step(FlightInput::default()));
    }
    for _ in 0..9000 {
        let snapshot = world.snapshot();
        if snapshot.mode == FlightMode::Docked {
            assert_eq!(some(snapshot.navigation).system, destination);
            return;
        }
        let speed = if snapshot.position[2] > 2500.0 {
            34.3
        } else {
            4.0
        };
        ok(world.step(FlightInput {
            throttle: ((speed - snapshot.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
            ..FlightInput::default()
        }));
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    panic!("native docking did not complete");
}

fn trade(world: &mut World, command: Command, quantity: usize) {
    for _ in 0..quantity {
        ok(world.request_station_service(command, world.snapshot().tick));
    }
}

fn refuel_for_return(world: &mut World) {
    if some(world.snapshot().navigation).fuel_units < 68 {
        ok(world.request_refuel(world.snapshot().tick));
    }
}

fn market_funded_commander() -> World {
    // The same finite public market and journey checkpoints as the existing equipment tests.
    // No test-only state, credit grant, XYZZY action or fabricated item is used.
    let mut world = ok(World::new_commander(16844));
    for (quantity, expected_cash) in [(29, 11434), (47, 18296), (75, 29042)] {
        if some(world.snapshot().navigation).system != SystemId::AVENOR {
            refuel_for_return(&mut world);
            dock_at(&mut world, SystemId::AVENOR);
        }
        trade(
            &mut world,
            Command::BuyCommodity(CommodityId::Minerals),
            quantity,
        );
        dock_at(&mut world, SystemId::RENAS);
        trade(
            &mut world,
            Command::SellCommodity(CommodityId::Minerals),
            quantity,
        );
        assert_eq!(world.credits_tenths(), Some(expected_cash));
    }
    trade(&mut world, Command::BuyCommodity(CommodityId::Alloys), 48);
    dock_at(&mut world, SystemId::AVENOR);
    trade(&mut world, Command::SellCommodity(CommodityId::Alloys), 48);
    assert_eq!(world.credits_tenths(), Some(33650));
    assert_eq!(ok(world.request_refuel(world.snapshot().tick)), 204);
    trade(
        &mut world,
        Command::BuyCommodity(CommodityId::GemStones),
        37,
    );
    dock_at(&mut world, SystemId::RENAS);
    trade(
        &mut world,
        Command::SellCommodity(CommodityId::GemStones),
        37,
    );
    assert_eq!(world.credits_tenths(), Some(49541));
    trade(&mut world, Command::BuyCommodity(CommodityId::Alloys), 83);
    dock_at(&mut world, SystemId::AVENOR);
    trade(&mut world, Command::SellCommodity(CommodityId::Alloys), 83);
    assert_eq!(world.credits_tenths(), Some(57509));
    trade(
        &mut world,
        Command::BuyCommodity(CommodityId::GemStones),
        64,
    );
    dock_at(&mut world, SystemId::RENAS);
    trade(
        &mut world,
        Command::SellCommodity(CommodityId::GemStones),
        64,
    );
    assert_eq!(world.credits_tenths(), Some(85349));
    refuel_for_return(&mut world);
    dock_at(&mut world, SystemId::AVENOR);
    world
}

fn buy_fitted(world: &mut World, kind: ModuleKind, action: ModuleAction, price: u32) {
    let cash = some(world.credits_tenths());
    let offer = some(
        world
            .module_offers()
            .into_iter()
            .find(|offer| offer.kind == kind),
    );
    assert_eq!(offer.price_tenths, price);
    assert!(offer.available);
    let tick = world.snapshot().tick;
    let review = ok(world.review_module(action, tick));
    let item = ok(world.prepare_module(review, tick)).commit();
    assert_eq!(world.credits_tenths(), Some(cash - price));
    let equipment = some(world.modular_equipment_snapshot());
    let owned = some(equipment.items.iter().find(|entry| entry.id == item));
    assert_eq!(owned.kind, kind);
    assert_eq!(owned.slot, Some(offer.slot));
    assert!(matches!(owned.provenance, ModuleProvenance::Paid { .. }));
}

fn select_rock(world: &mut World, target: WorkTargetId) {
    // Model the public viewport contract with the real body's ID. This tests World
    // admission, not renderer clipping or physical keyboard delivery.
    let viewport = TargetingViewport {
        size: [1280, 720],
        front: Vec::new(),
    };
    assert!(ok(world.cycle_targeting_with_work(
        world.snapshot().tick,
        viewport,
        &[target],
    )));
    assert_eq!(
        some(world.targeting_snapshot()).identity,
        TargetIdentity::Work(target)
    );
    assert!(world.automatic_work_status().is_some());
}

#[test]
fn earned_mining_select_once_delivers_one_real_ore_unit_and_retains_it_on_continue() {
    let mut world = market_funded_commander();
    let basic = ModuleKind::Targeting(TargetingTier::Basic);
    buy_fitted(
        &mut world,
        basic,
        ModuleAction::Purchase { kind: basic },
        8000,
    );
    buy_fitted(
        &mut world,
        ModuleKind::MiningTargeting,
        ModuleAction::Purchase {
            kind: ModuleKind::MiningTargeting,
        },
        20000,
    );
    buy_fitted(
        &mut world,
        ModuleKind::MiningLaser,
        ModuleAction::PurchaseMiningLaser,
        33000,
    );
    buy_fitted(
        &mut world,
        ModuleKind::FuelScoops,
        ModuleAction::PurchaseFuelScoops,
        19000,
    );
    let paid_cash = world.credits_tenths();
    let paid_equipment = some(world.modular_equipment_snapshot());

    ok(world.request_launch(world.snapshot().tick));
    ok(world.step(FlightInput::default()));
    assert_eq!(ok(world.cycle_target_filter()), TargetFilter::Mining);
    let rock = some(world.mining_belt_snapshot()).bodies[0];
    assert_eq!(rock.initial_units, 1);
    assert_eq!(rock.remaining_units, 1);
    let target = WorkTargetId::Asteroid(rock.id);
    let identity = TargetIdentity::Work(target);
    let held = some(world.commodity_quantity(rock.commodity));

    // A manual override ends the original lock before it can work.
    select_rock(&mut world, target);
    ok(world.step(FlightInput {
        yaw: 0.5,
        ..FlightInput::default()
    }));
    assert!(world.automatic_work_status().is_none());
    assert!(world.targeting_snapshot().is_none());
    assert_eq!(some(world.mining_belt_snapshot()).bodies[0], rock);
    assert_eq!(world.commodity_quantity(rock.commodity), Some(held));

    // One selection starts extraction and physical intake. No manual fire, scoop,
    // second target selection, test-only movement or direct resource mutation follows.
    select_rock(&mut world, target);
    let mut complete = false;
    for _ in 0..12000 {
        ok(world.step(FlightInput::default()));
        if world.commodity_quantity(rock.commodity) == Some(held + 1) {
            complete = true;
            break;
        }
        assert_eq!(
            world.targeting_snapshot().map(|snapshot| snapshot.identity),
            Some(identity),
            "the original rock lock must survive depletion and ore approach"
        );
        assert_eq!(world.commodity_quantity(rock.commodity), Some(held));
        assert_eq!(world.credits_tenths(), paid_cash);
    }
    assert!(
        complete,
        "automatic intake did not complete: {:?}",
        world.automatic_work_status()
    );
    let belt = some(world.mining_belt_snapshot());
    assert_eq!(belt.bodies[0].remaining_units, 0);
    assert_eq!(belt.bodies[0].extracted_units, 1);
    assert!(
        belt.pending_ore
            .iter()
            .all(|ore| ore.id.asteroid() != rock.id)
    );
    assert_eq!(world.credits_tenths(), paid_cash);
    let retained_equipment = some(world.modular_equipment_snapshot());
    assert_eq!(retained_equipment.items, paid_equipment.items);
    assert_eq!(retained_equipment.fit_revision, paid_equipment.fit_revision);

    let saved = ok(world.save_bytes());
    let mut cold = ok(World::from_save_bytes(&saved));
    assert_eq!(cold.commodity_quantity(rock.commodity), Some(held + 1));
    assert_eq!(cold.mining_belt_snapshot(), Some(belt));
    assert_eq!(cold.credits_tenths(), paid_cash);
    assert_eq!(cold.modular_equipment_snapshot(), Some(retained_equipment));
    assert!(cold.automatic_work_status().is_none());
    for _ in 0..120 {
        ok(cold.step(FlightInput::default()));
    }
    assert_eq!(cold.commodity_quantity(rock.commodity), Some(held + 1));
    assert_eq!(cold.credits_tenths(), paid_cash);
}

//! Ordinary paid auto-work acceptance through the public World command boundary.
use meridian_core::{
    FlightInput, FlightMode, ModuleAction, ModuleKind, ModuleProvenance, ModuleSlot, TargetFilter,
    TargetIdentity, TargetingViewport, World,
    docking::Vec3,
    equipment::{TargetingTier, WorkTargetId},
    mining::{ASTEROID_FACES, AsteroidSnapshot, OreSnapshot},
    navigation::SystemId,
    station_services::{Command, CommodityId},
};
#[path = "automatic_work_public/salvage.rs"]
mod salvage;

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

fn market_funded(mut world: World) -> World {
    // The same finite public market and journey checkpoints as the existing equipment tests.
    // No test-only state, credit grant, XYZZY action or fabricated item is used.
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
    let offer = some(if kind == ModuleKind::FuelScoops {
        world
            .fuel_scoops_snapshot()
            .and_then(|scoops| scoops.current_offer)
    } else {
        world
            .module_offers()
            .into_iter()
            .find(|offer| offer.kind == kind)
    });
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

fn launch_mining(world: &mut World) -> AsteroidSnapshot {
    ok(world.request_launch(world.snapshot().tick));
    ok(world.step(FlightInput::default()));
    assert_eq!(ok(world.cycle_target_filter()), TargetFilter::Mining);
    let rock = some(world.mining_belt_snapshot()).bodies[0];
    assert_eq!(rock.initial_units, 1);
    assert_eq!(rock.remaining_units, 1);
    rock
}

fn verify_fragment(world: &World, rock: AsteroidSnapshot, ore: OreSnapshot) {
    assert_eq!(ore.id.asteroid(), rock.id);
    assert_eq!(ore.id.fragment(), 0);
    assert_eq!(ore.commodity, rock.commodity);
    assert_eq!(ore.quantity, 1);
    assert!(ore.released_tick <= world.snapshot().tick);
    assert_eq!(ore.velocity, Vec3::new(0.0, 2.0, 3.0));
    assert_eq!(world.mining_collection_target(rock.id), Some(ore));
    let p = ore.position;
    // The same approved Meridian six-vertex fragment consumed by the ordinary renderer.
    // This verifies the projected body, not actual screen rendering.
    assert_eq!(
        ore.vertices(),
        [
            Vec3::new(p.x + 7.0, p.y + 1.0, p.z - 2.0),
            Vec3::new(p.x - 5.0, p.y - 1.0, p.z + 2.0),
            Vec3::new(p.x + 1.0, p.y + 10.0, p.z + 1.0),
            Vec3::new(p.x - 2.0, p.y - 6.0, p.z - 1.0),
            Vec3::new(p.x + 2.0, p.y + 3.0, p.z + 6.0),
            Vec3::new(p.x - 1.0, p.y, p.z - 8.0),
        ]
    );
    assert_eq!(
        ASTEROID_FACES,
        [
            [0, 2, 4],
            [2, 1, 4],
            [1, 3, 4],
            [3, 0, 4],
            [2, 0, 5],
            [1, 2, 5],
            [3, 1, 5],
            [0, 3, 5]
        ]
    );
}

fn refuse_missing_scoops(mut world: World) {
    let cash = world.credits_tenths();
    let equipment = some(world.modular_equipment_snapshot());
    let scoop = some(
        equipment
            .items
            .into_iter()
            .find(|item| item.kind == ModuleKind::FuelScoops),
    );
    let tick = world.snapshot().tick;
    let reviewed = ok(world.review_module(ModuleAction::Store { item: scoop.id }, tick));
    ok(world.prepare_module(reviewed, tick)).commit();
    assert_eq!(world.credits_tenths(), cash);
    let stored = some(
        some(world.modular_equipment_snapshot())
            .items
            .into_iter()
            .find(|item| item.id == scoop.id),
    );
    assert_eq!(stored.slot, None);
    assert_eq!(stored.provenance, scoop.provenance);

    let rock = launch_mining(&mut world);
    let held = world.commodity_quantity(rock.commodity);
    select_rock(&mut world, WorkTargetId::Asteroid(rock.id));
    let mut refused = false;
    for _ in 0..12000 {
        ok(world.step(FlightInput::default()));
        assert_eq!(world.commodity_quantity(rock.commodity), held);
        assert_eq!(world.credits_tenths(), cash);
        assert_eq!(some(world.mining_belt_snapshot()).bodies[0], rock);
        if world.automatic_work_status() == Some("Fit fuel scoops") {
            refused = true;
            break;
        }
    }
    assert!(
        refused,
        "missing-scoops refusal unavailable: {:?}",
        world.automatic_work_status()
    );
    assert!(world.mining_collection_target(rock.id).is_none());
    let cold = ok(World::from_save_bytes(&ok(world.save_bytes())));
    assert_eq!(cold.commodity_quantity(rock.commodity), held);
    assert_eq!(cold.credits_tenths(), cash);
    assert_eq!(some(cold.mining_belt_snapshot()).bodies[0], rock);
    assert!(cold.mining_collection_target(rock.id).is_none());
}

#[test]
fn earned_mining_select_once_delivers_one_real_ore_unit_and_retains_it_on_continue() {
    let mut world = market_funded(ok(World::new_commander(16844)));
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
    // The paid Mining Laser remains owned, but automatic selection must replace and
    // later restore the real born combat laser rather than requiring a manual pre-fit.
    let equipment = some(world.modular_equipment_snapshot());
    let mining = *some(
        equipment
            .items
            .iter()
            .find(|item| item.kind == ModuleKind::MiningLaser),
    );
    let pulse = *some(
        equipment
            .items
            .iter()
            .find(|item| item.kind == ModuleKind::PulseLaser),
    );
    assert_eq!(mining.slot, Some(ModuleSlot::FrontLaser));
    assert_eq!(pulse.slot, None);
    let tick = world.snapshot().tick;
    let review = ok(world.review_module(
        ModuleAction::Install {
            item: pulse.id,
            replacing: Some(mining.id),
        },
        tick,
    ));
    assert_eq!(ok(world.prepare_module(review, tick)).commit(), pulse.id);
    let paid_cash = world.credits_tenths();
    let paid_equipment = some(world.modular_equipment_snapshot());
    assert_eq!(some(world.pulse_laser_snapshot()).fitted_item_id, pulse.id);
    assert!(world.fitted_mining_tool().is_none());

    // A separately continued copy of the normally earned profile exercises refusal.
    // It never mutates the paid positive journey below.
    refuse_missing_scoops(ok(World::from_save_bytes(&ok(world.save_bytes()))));

    let rock = launch_mining(&mut world);
    let target = WorkTargetId::Asteroid(rock.id);
    let identity = TargetIdentity::Work(target);
    let held = some(world.commodity_quantity(rock.commodity));

    // A manual override ends the original lock before it can work.
    select_rock(&mut world, target);
    ok(world.step(FlightInput::default()));
    assert_eq!(some(world.fitted_mining_tool()).item, mining.id);
    ok(world.step(FlightInput {
        yaw: 0.5,
        ..FlightInput::default()
    }));
    assert!(world.automatic_work_status().is_none());
    assert!(world.targeting_snapshot().is_none());
    assert_eq!(some(world.mining_belt_snapshot()).bodies[0], rock);
    assert_eq!(world.commodity_quantity(rock.commodity), Some(held));
    assert_eq!(some(world.pulse_laser_snapshot()).fitted_item_id, pulse.id);
    assert!(world.fitted_mining_tool().is_none());

    // One selection starts extraction and physical intake. No manual fire, scoop,
    // second target selection, test-only movement or direct resource mutation follows.
    select_rock(&mut world, target);
    let mut complete = false;
    let mut witnessed_ore = None;
    let mut witnessed_mining_tool = false;
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
        if let Some(tool) = world.fitted_mining_tool() {
            assert_eq!(tool.item, mining.id);
            witnessed_mining_tool = true;
        }
        let belt = some(world.mining_belt_snapshot());
        if let Some(ore) = belt
            .pending_ore
            .iter()
            .find(|ore| ore.id.asteroid() == rock.id)
            .copied()
        {
            assert_eq!(belt.bodies[0].remaining_units, 0);
            assert_eq!(belt.bodies[0].extracted_units, 1);
            assert_eq!(
                belt.pending_ore
                    .iter()
                    .filter(|other| other.id.asteroid() == rock.id)
                    .count(),
                1
            );
            verify_fragment(&world, rock, ore);
            if let Some((id, released_tick, released_physical_tick)) = witnessed_ore {
                assert_eq!(
                    (ore.id, ore.released_tick, ore.released_physical_tick),
                    (id, released_tick, released_physical_tick)
                );
            } else {
                witnessed_ore = Some((ore.id, ore.released_tick, ore.released_physical_tick));
            }
        }
    }
    assert!(
        complete,
        "automatic intake did not complete: {:?}",
        world.automatic_work_status()
    );
    assert!(
        witnessed_ore.is_some(),
        "delivery must follow a real released fragment"
    );
    assert!(witnessed_mining_tool);
    let belt = some(world.mining_belt_snapshot());
    assert_eq!(belt.bodies[0].remaining_units, 0);
    assert_eq!(belt.bodies[0].extracted_units, 1);
    assert!(
        belt.pending_ore
            .iter()
            .all(|ore| ore.id.asteroid() != rock.id)
    );
    assert!(world.mining_collection_target(rock.id).is_none());
    assert_eq!(world.credits_tenths(), paid_cash);
    let retained_equipment = some(world.modular_equipment_snapshot());
    assert_eq!(retained_equipment.items, paid_equipment.items);
    assert!(retained_equipment.fit_revision > paid_equipment.fit_revision);
    assert_eq!(some(world.pulse_laser_snapshot()).fitted_item_id, pulse.id);
    assert!(world.fitted_mining_tool().is_none());

    let saved = ok(world.save_bytes());
    let mut cold = ok(World::from_save_bytes(&saved));
    assert_eq!(cold.commodity_quantity(rock.commodity), Some(held + 1));
    assert_eq!(cold.mining_belt_snapshot(), Some(belt));
    assert!(cold.mining_collection_target(rock.id).is_none());
    assert_eq!(cold.credits_tenths(), paid_cash);
    assert_eq!(cold.modular_equipment_snapshot(), Some(retained_equipment));
    assert!(cold.automatic_work_status().is_none());
    for _ in 0..120 {
        ok(cold.step(FlightInput::default()));
    }
    assert_eq!(cold.commodity_quantity(rock.commodity), Some(held + 1));
    assert_eq!(cold.credits_tenths(), paid_cash);
}

#![allow(clippy::unwrap_used)]
use super::*;
use crate::equipment::TargetingTier;
use crate::flight::equipment::tests::earned_world;
use crate::flight::practice::tests::launch;
use crate::{ModuleAction, ModuleError, ModuleKind, ModuleProvenance, XyzzyAction};

fn fitted(w: &World, slot: ModuleSlot) -> u64 {
    w.modular_equipment_snapshot()
        .unwrap()
        .items
        .iter()
        .find(|item| item.slot == Some(slot))
        .unwrap()
        .id
}

fn change(w: &mut World, action: ModuleAction) -> u64 {
    let tick = w.snapshot().tick;
    let review = w.review_module(action, tick).unwrap();
    let prepared = w.prepare_module(review, tick).unwrap();
    let loaded = World::from_save_bytes(prepared.bytes()).unwrap();
    let id = prepared.commit();
    assert_eq!(
        loaded.modular_equipment_snapshot(),
        w.modular_equipment_snapshot()
    );
    assert_eq!(loaded.work_targeting_fit(), w.work_targeting_fit());
    id
}

#[test]
fn ordinary_work_modules_require_owned_basic_and_a_fitted_core_and_keep_independent_slots() {
    let mut world = earned_world(16841);
    let basic = ModuleKind::Targeting(TargetingTier::Basic);
    let mining = ModuleKind::MiningTargeting;
    let salvage = ModuleKind::SalvageTargeting;
    let purchase = |kind| ModuleAction::Purchase { kind };
    for (kind, slot, price, technology) in [
        (mining, ModuleSlot::MiningTargeting, 20_000, 6),
        (salvage, ModuleSlot::SalvageTargeting, 40_000, 8),
    ] {
        let offer = world
            .module_offers()
            .into_iter()
            .find(|o| o.kind == kind)
            .unwrap();
        assert_eq!(
            (offer.slot, offer.price_tenths, offer.required_technology),
            (slot, price, technology)
        );
        assert_eq!(offer.prerequisite, Some(basic));
        assert!(!offer.available);
        assert_eq!(
            world.review_module(purchase(kind), world.snapshot().tick),
            Err(ModuleError::MissingPrerequisite)
        );
    }
    let credits = world.credits_tenths().unwrap();
    let core = change(&mut world, purchase(basic));
    let mining_item = change(&mut world, purchase(mining));
    change(&mut world, ModuleAction::Store { item: core });
    assert_eq!(world.work_targeting_fit(), WorkTargetingFit::default());
    assert_eq!(
        world.review_module(purchase(salvage), world.snapshot().tick),
        Err(ModuleError::MissingPrerequisite)
    );
    change(&mut world, ModuleAction::Store { item: mining_item });
    assert_eq!(
        world.review_module(
            ModuleAction::Install {
                item: mining_item,
                replacing: None
            },
            world.snapshot().tick
        ),
        Err(ModuleError::MissingPrerequisite)
    );
    change(
        &mut world,
        ModuleAction::Install {
            item: core,
            replacing: None,
        },
    );
    change(
        &mut world,
        ModuleAction::Install {
            item: mining_item,
            replacing: None,
        },
    );
    let salvage_item = change(&mut world, purchase(salvage));
    assert_eq!(world.credits_tenths(), Some(credits - 68_000));
    assert_eq!(fitted(&world, ModuleSlot::MiningTargeting), mining_item);
    assert_eq!(fitted(&world, ModuleSlot::SalvageTargeting), salvage_item);
    assert_eq!(
        world.work_targeting_fit(),
        WorkTargetingFit {
            mining: true,
            salvage: true
        }
    );
    change(&mut world, ModuleAction::Store { item: mining_item });
    assert_eq!(
        world.work_targeting_fit(),
        WorkTargetingFit {
            mining: false,
            salvage: true
        }
    );
}

#[test]
fn explicit_xyzzy_grants_independent_fits_and_only_real_same_system_work_targets() {
    let mut world = World::new_commander(16842).unwrap();
    assert_eq!(world.work_targeting_fit(), WorkTargetingFit::default());
    assert!(world.work_targets().is_empty());
    let grant = |world: &mut World| {
        let tick = world.snapshot().tick;
        let review = world.review_xyzzy(XyzzyAction::Enter, tick).unwrap();
        world.prepare_xyzzy(review, tick).unwrap().commit();
    };
    grant(&mut world);
    let items = world.modular_equipment_snapshot().unwrap().items;
    for (kind, slot) in [
        (ModuleKind::MiningTargeting, ModuleSlot::MiningTargeting),
        (ModuleKind::SalvageTargeting, ModuleSlot::SalvageTargeting),
    ] {
        let item = items.iter().find(|item| item.kind == kind).unwrap();
        assert_eq!(item.slot, Some(slot));
        assert!(matches!(item.provenance, ModuleProvenance::XYZzy { .. }));
    }
    grant(&mut world);
    assert_eq!(items, world.modular_equipment_snapshot().unwrap().items);
    let mut world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(
        world.work_targeting_fit(),
        WorkTargetingFit {
            mining: true,
            salvage: true
        }
    );
    assert!(world.work_targets().is_empty());
    launch(&mut world);
    let belt = world.mining_belt_snapshot().unwrap();
    let before = world.save_bytes().unwrap();
    assert_eq!(
        world.work_targets(),
        belt.bodies
            .into_iter()
            .map(|rock| WorkTarget {
                id: WorkTargetId::Asteroid(rock.id),
                role: WorkTargetRole::RockHold,
                position: rock.position,
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    world
        .request_jump(SystemId::RENAS, world.snapshot().tick)
        .unwrap();
    assert!(world.work_targets().is_empty());
}

#[test]
fn fixed_rock_guidance_brakes_to_a_clear_hold_using_ordinary_flight_steps() {
    let mut world = World::new_commander(16843).unwrap();
    launch(&mut world);
    let rock = world.mining_belt_snapshot().unwrap().bodies[0];
    // Place the isolated test craft on a clear exterior ray to a real belt body.
    world.state.position = [
        rock.position.x as f32,
        rock.position.y as f32,
        rock.position.z as f32 - 250.0,
    ];
    world.state.orientation = [0.0, 0.0, 0.0, 1.0];
    world.state.speed = 0.0;
    let target = WorkTarget {
        id: WorkTargetId::Asteroid(rock.id),
        role: WorkTargetRole::RockHold,
        position: rock.position,
    };
    let cash = world.credits_tenths();
    let starting_range = rock.position.z - f64::from(world.state.position[2]);
    let input = crate::FlightInput::default();
    assert!(world.work_hold_input(target, input).throttle > 0.0);
    for _ in 0..600 {
        world.step(world.work_hold_input(target, input)).unwrap();
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    let ending_range = rock.position.z - f64::from(world.state.position[2]);
    let player_radius = world
        .station_data
        .as_ref()
        .unwrap()
        .player_clearance_radius();
    assert!(ending_range < starting_range);
    assert!(ending_range > rock.bounding_radius() + player_radius);
    assert!(world.state.speed < 0.1);
    assert_eq!(world.credits_tenths(), cash);
    assert!(world.mining_belt_snapshot().unwrap().pending_ore.is_empty());
}

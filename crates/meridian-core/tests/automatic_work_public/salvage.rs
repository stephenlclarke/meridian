//! Real ordinary cargo settlement and recovery with paid fittings.
use super::*;
use meridian_core::{
    AmmunitionAction, MAX_SPEED, MissileContact, MissileOrigin, ProjectileSnapshot, Snapshot,
    traffic::{NpcId, NpcPhase, NpcSnapshot, Population, WreckDamageSource, WreckManifest},
};

fn finance_salvage() -> World {
    let mut world = ok(World::new_commander(12005));
    // Existing ordinary practice consent holds the finite traffic actors during the real
    // funding journey. The original public wreck scenario uses this same protection.
    let review = ok(world.review_practice(world.snapshot().tick));
    ok(world.prepare_practice_confirmation(review, world.snapshot().tick)).commit();
    let mut world = market_funded(world);

    // Extend ordinary funding with the actual legal Gold market, not an invented grant.
    let cash = some(world.credits_tenths());
    let ask = some(world.commodity_quote(CommodityId::Gold));
    assert!(ask.stock >= 16);
    assert_eq!(CommodityId::Gold.departure_legal_weight(), 0);
    trade(&mut world, Command::BuyCommodity(CommodityId::Gold), 16);
    assert_eq!(world.commodity_quantity(CommodityId::Gold), Some(16));
    assert_eq!(world.credits_tenths(), Some(cash - 16 * ask.ask_tenths));
    refuel_for_return(&mut world);
    dock_at(&mut world, SystemId::RENAS);
    let bid = some(world.commodity_quote(CommodityId::Gold));
    assert!(bid.bid_tenths > ask.ask_tenths);
    let before_sale = some(world.credits_tenths());
    trade(&mut world, Command::SellCommodity(CommodityId::Gold), 16);
    assert_eq!(
        world.credits_tenths(),
        Some(before_sale + 16 * bid.bid_tenths)
    );
    assert_eq!(world.commodity_quantity(CommodityId::Gold), Some(0));

    let kinds = [
        (ModuleKind::Targeting(TargetingTier::Basic), 8000),
        (ModuleKind::SalvageTargeting, 40000),
        (ModuleKind::SalvageCutter, 25000),
        (ModuleKind::FuelScoops, 19000),
    ];
    assert!(some(world.credits_tenths()) > 99200);
    for (kind, price) in kinds {
        let action = if kind == ModuleKind::FuelScoops {
            ModuleAction::PurchaseFuelScoops
        } else {
            ModuleAction::Purchase { kind }
        };
        buy_fitted(&mut world, kind, action, price);
    }
    // Four paid rounds plus the three finite born rounds are the existing source test.
    for _ in 0..4 {
        let cash = some(world.credits_tenths());
        let ammunition = some(world.ammunition_snapshot());
        assert!(ammunition.purchase_available);
        let tick = world.snapshot().tick;
        let review = ok(world.review_ammunition(AmmunitionAction::Purchase, tick));
        let item = ok(world.prepare_ammunition(review, tick)).commit();
        assert_eq!(world.credits_tenths(), Some(cash - ammunition.price_tenths));
        let loaded = some(world.ammunition_snapshot());
        assert!(
            loaded
                .purchases
                .iter()
                .any(|purchase| purchase.item == item)
        );
    }
    refuel_for_return(&mut world);
    dock_at(&mut world, SystemId::AVENOR);
    let tick = world.snapshot().tick;
    let closure = ok(world.review_practice_closure(tick));
    ok(world.prepare_practice_closure(closure, tick)).commit();
    world
}

fn subtract(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn position(snapshot: Snapshot) -> Vec3 {
    let [x, y, z] = snapshot.position.map(f64::from);
    Vec3::new(x, y, z)
}

fn forward(snapshot: Snapshot) -> Vec3 {
    // Public snapshot quaternion, local forward is +Z.
    let [x, y, z, w] = snapshot.orientation;
    Vec3::new(
        f64::from(2.0 * (x * z + w * y)),
        f64::from(2.0 * (y * z - w * x)),
        f64::from(1.0 - 2.0 * (x * x + y * y)),
    )
}

fn stop(world: &mut World) {
    for _ in 0..300 {
        if world.snapshot().speed == 0.0 {
            return;
        }
        ok(world.step(FlightInput {
            brake: true,
            ..FlightInput::default()
        }));
    }
    panic!("public brake did not stop");
}

fn launch_clear(world: &mut World) {
    ok(world.request_launch(world.snapshot().tick));
    ok(world.step(FlightInput::default()));
    stop(world);
    let snapshot = world.snapshot();
    let direction = forward(snapshot);
    let intended = (-600.0 - f64::from(snapshot.position[0])).atan2(0.0);
    let yaw = (intended - direction.x.atan2(direction.z)) as f32;
    let ticks = (yaw.abs() / (0.65 / 60.0)).ceil() as u16;
    for _ in 0..ticks {
        ok(world.step(FlightInput {
            yaw: yaw / (0.65 / 60.0) / f32::from(ticks),
            ..FlightInput::default()
        }));
    }
    for _ in 0..30000 {
        let snapshot = world.snapshot();
        if snapshot.position[0] <= -600.0 {
            stop(world);
            return;
        }
        ok(world.step(FlightInput {
            throttle: ((10.0 - snapshot.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
            ..FlightInput::default()
        }));
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    panic!("public flight did not reach the ordinary encounter's clear approach");
}

fn trader(world: &World) -> Option<NpcSnapshot> {
    some(world.traffic_snapshot())
        .actors
        .into_iter()
        .find(|actor| actor.population == Population::Trader)
}

fn pursuit(world: &World, source: NpcSnapshot) -> FlightInput {
    let snapshot = world.snapshot();
    let delta = subtract(source.pose.position, position(snapshot));
    let distance = delta.dot(delta).sqrt();
    let lead = (distance / (2.0 * f64::from(MAX_SPEED) * 18.0)).min(1.0);
    let velocity = source.speed_units * 18.0 * lead;
    let heading = source.pose.orientation.z;
    let goal = Vec3::new(
        source.pose.position.x + heading.x * velocity,
        source.pose.position.y + heading.y * velocity,
        source.pose.position.z + heading.z * velocity,
    );
    let aim = subtract(goal, position(snapshot));
    let direction = forward(snapshot);
    let error = aim.x.atan2(aim.z) - direction.x.atan2(direction.z);
    let error = error.sin().atan2(error.cos());
    let speed = if error.abs() > 0.3 {
        0.0
    } else {
        (source.speed_units + (distance - 600.0) / 36.0).clamp(0.0, f64::from(MAX_SPEED)) as f32
    };
    FlightInput {
        yaw: (error / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32,
        throttle: ((speed - snapshot.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
        ..FlightInput::default()
    }
}

fn lock_and_fire(world: &mut World, source: NpcId) -> MissileOrigin {
    for _ in 0..12000 {
        let actor = some(trader(world));
        assert_eq!(actor.id, source);
        assert!(!matches!(
            actor.phase,
            NpcPhase::Destroyed | NpcPhase::Complete
        ));
        let snapshot = world.snapshot();
        let delta = subtract(actor.pose.position, position(snapshot));
        let distance = delta.dot(delta).sqrt();
        if distance > 0.0
            && distance < 900.0
            && delta.dot(forward(snapshot)) / distance > 0.995
            && snapshot.speed >= actor.speed_units as f32
        {
            ok(world.request_npc_missile_lock(Some(actor.id), snapshot.tick));
            ok(world.request_missile_arm(true, snapshot.tick));
            return ok(world.request_fire_missile(snapshot.tick));
        }
        ok(world.step(pursuit(world, actor)));
    }
    panic!("real ordinary source did not enter native lock range");
}

fn finish_projectile(world: &mut World, origin: MissileOrigin) -> ProjectileSnapshot {
    for _ in 0..500 {
        let projectile = some(
            world
                .missile_projectiles()
                .find(|p| p.origin.projectile == origin.projectile),
        );
        if projectile.ended.is_some() {
            return projectile;
        }
        ok(world.step(pursuit(world, some(trader(world)))));
    }
    panic!("actual finite projectile did not end");
}

fn terminal_source(world: &mut World) -> WreckManifest {
    launch_clear(world);
    for _ in 0..1000 {
        if trader(world).is_some() {
            break;
        }
        ok(world.step(FlightInput::default()));
    }
    let source = some(trader(world));
    let (commodity, quantity) = some(source.cargo);
    assert_eq!(commodity.departure_legal_weight(), 0);
    assert!((1..=12).contains(&quantity));
    assert!(!world.auto_salvage_eligible(source.id));
    assert!(world.wreck_manifest(source.id).is_err());
    let before = world.commodity_quantity(commodity);
    let cash = world.credits_tenths();
    for _ in 0..7 {
        let origin = lock_and_fire(world, source.id);
        let impact = finish_projectile(world, origin);
        assert_eq!(impact.contact, Some(MissileContact::Npc(source.id)));
        if some(trader(world)).phase == NpcPhase::Destroyed {
            let manifest = ok(world.wreck_manifest(source.id));
            assert_eq!(
                (manifest.source, manifest.commodity, manifest.quantity),
                (source.id, commodity, quantity)
            );
            assert_eq!(
                manifest.cause,
                WreckDamageSource::Missile(origin.projectile)
            );
            assert_eq!(manifest.destroyed_at_tick, impact.last_tick);
            assert_eq!(manifest.impact_point, impact.position);
            assert_eq!(world.commodity_quantity(commodity), before);
            assert_eq!(world.credits_tenths(), cash);
            assert!(world.auto_salvage_eligible(source.id));
            return manifest;
        }
        assert!(!world.auto_salvage_eligible(source.id));
        while some(world.missile_snapshot()).cooldown_ticks > 0 {
            ok(world.step(FlightInput::default()));
        }
    }
    panic!("finite game ordnance did not settle the actual ordinary source");
}

fn salvage_filter(world: &mut World) {
    assert_eq!(ok(world.cycle_target_filter()), TargetFilter::Mining);
    assert_eq!(ok(world.cycle_target_filter()), TargetFilter::Salvage);
}

fn select_wreck(world: &mut World, target: WorkTargetId) -> bool {
    ok(world.cycle_targeting_with_work(
        world.snapshot().tick,
        TargetingViewport {
            size: [1280, 720],
            front: Vec::new(),
        },
        &[target],
    ))
}

fn reject_altered_first_recovery_pose(
    bytes: &[u8],
    manifest: WreckManifest,
    receipts: u8,
    first: Snapshot,
) {
    // Match the exact new claim and first receipt identity rather than the current flight
    // snapshot elsewhere in the save. The checksum is deliberately repaired so the
    // semantic decoder, not corruption detection, must refuse the altered receipt.
    let source = manifest.source;
    let system = source.system();
    let mut id = Vec::new();
    id.extend(source.profile().to_le_bytes());
    id.push(system.galaxy);
    id.extend(system.system.to_le_bytes());
    id.push(source.serial());
    let mut prefix = vec![0x81];
    prefix.extend(&id);
    prefix.extend(manifest.destroyed_at_tick.to_le_bytes());
    prefix.push(manifest.commodity.index() as u8);
    prefix.push(manifest.quantity);
    prefix.push(receipts);
    prefix.extend(&id);
    prefix.push(0); // first source ordinal
    let matches = bytes
        .windows(prefix.len())
        .enumerate()
        .filter_map(|(start, candidate)| (candidate == prefix.as_slice()).then_some(start))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);
    let method = matches[0] + prefix.len() + 6 * core::mem::size_of::<u64>();
    assert_eq!(bytes[method], 1); // explicit direct cutter-to-scoop feed
    let pose = method + 1;
    let orientation = pose + 3 * core::mem::size_of::<f32>();
    let speed = orientation + 4 * core::mem::size_of::<f32>();
    for (index, value) in first
        .position
        .into_iter()
        .chain(first.orientation)
        .chain([first.speed])
        .enumerate()
    {
        let offset = pose + index * core::mem::size_of::<f32>();
        assert_eq!(&bytes[offset..offset + 4], &value.to_le_bytes());
    }
    let seal = |mut forged: Vec<u8>| {
        let end = forged.len() - 8;
        let digest = forged[..end]
            .iter()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
            });
        forged[end..].copy_from_slice(&digest.to_le_bytes());
        assert!(matches!(
            World::from_save_bytes(&forged),
            Err(meridian_core::save::SaveError::InvalidState(
                "external salvage custody"
            ))
        ));
    };
    let mut fast = bytes.to_vec();
    fast[speed..speed + 4].copy_from_slice(&0.05_f32.to_le_bytes());
    seal(fast);
    let [x, y, z, w] = first.orientation;
    let mut facing_away = bytes.to_vec();
    for (index, turned) in [-z, w, x, -y].into_iter().enumerate() {
        let offset = orientation + index * 4;
        facing_away[offset..offset + 4].copy_from_slice(&turned.to_le_bytes());
    }
    seal(facing_away);
}

#[test]
fn earned_external_salvage_recovers_only_the_actual_legal_manifest_once() {
    check_paid_wreck_recovery(finance_salvage());
}

#[test]
fn stored_paid_cutter_is_selected_for_recovery_and_returned_on_the_final_unit() {
    let mut world = finance_salvage();
    let cutter = some(world.fitted_salvage_cutter());
    let tick = world.snapshot().tick;
    let store = ok(world.review_module(ModuleAction::Store { item: cutter.id }, tick));
    ok(world.prepare_module(store, tick)).commit();
    assert!(world.fitted_salvage_cutter().is_none());
    check_paid_wreck_recovery(world);
}

fn check_paid_wreck_recovery(mut world: World) {
    let manifest = terminal_source(&mut world);
    let held = some(world.commodity_quantity(manifest.commodity));
    let cash = world.credits_tenths();
    // Preserve any law consequence of the source encounter; recovery is not an amnesty.
    let law = world.legal_value();
    let fitted = some(world.modular_equipment_snapshot()).items;
    let target = WorkTargetId::Wreck(manifest.source);
    let tick = world.snapshot().tick;
    let viewport = [1280, 720];
    assert!(ok(world.cycle_salvage_contact(tick, viewport)));
    assert!(ok(world.cycle_salvage_contact(tick, viewport)));
    assert_eq!(world.commodity_quantity(manifest.commodity), Some(held));
    assert_eq!(
        some(world.targeting_snapshot()).identity,
        TargetIdentity::Work(target)
    );
    assert!(world.automatic_work_status().is_none());
    let mut recovered = 0u64;
    let mut previous_recovery = None;
    let mut first_recovery_pose = None;
    for _ in 0..12000 {
        ok(world.step(FlightInput::default()));
        assert_eq!(ok(world.wreck_manifest(manifest.source)), manifest);
        assert_eq!(world.credits_tenths(), cash);
        assert_eq!(world.legal_value(), law);
        let quantity = some(world.commodity_quantity(manifest.commodity));
        if quantity != held + recovered {
            assert_eq!(quantity, held + recovered + 1);
            recovered += 1;
            if recovered < u64::from(manifest.quantity) {
                assert!(world.fitted_salvage_cutter().is_some());
            }
            let tick = world.snapshot().tick;
            if let Some(previous) = previous_recovery {
                assert!(
                    tick - previous >= 60,
                    "each finite recovery needs its own accepted interval"
                );
            }
            previous_recovery = Some(tick);
            let bytes = ok(world.save_bytes());
            if recovered == 1 {
                first_recovery_pose = Some(world.snapshot());
                reject_altered_first_recovery_pose(&bytes, manifest, 1, some(first_recovery_pose));
            } else if recovered == 2 {
                // First receipt is now historical, so rejection must not rely only on
                // equality with the current saved pose.
                reject_altered_first_recovery_pose(&bytes, manifest, 2, some(first_recovery_pose));
            }
            let continued = ok(World::from_save_bytes(&bytes));
            assert_eq!(
                continued.commodity_quantity(manifest.commodity),
                Some(quantity)
            );
            assert_eq!(ok(continued.wreck_manifest(manifest.source)), manifest);
            assert_eq!(continued.credits_tenths(), cash);
            assert_eq!(continued.legal_value(), law);
        }
        assert!(recovered <= u64::from(manifest.quantity));
        if recovered == u64::from(manifest.quantity) {
            break;
        }
        assert_eq!(
            some(world.targeting_snapshot()).identity,
            TargetIdentity::Work(target)
        );
        assert!(world.auto_salvage_eligible(manifest.source));
    }
    assert_eq!(
        recovered,
        u64::from(manifest.quantity),
        "salvage result: {:?}",
        world.automatic_work_status()
    );
    assert!(!world.auto_salvage_eligible(manifest.source));
    assert_eq!(some(world.modular_equipment_snapshot()).items, fitted);
    let mut cold = ok(World::from_save_bytes(&ok(world.save_bytes())));
    assert_eq!(
        cold.commodity_quantity(manifest.commodity),
        Some(held + recovered)
    );
    assert!(!cold.auto_salvage_eligible(manifest.source));
    assert_eq!(ok(cold.wreck_manifest(manifest.source)), manifest);
    assert_eq!(cold.credits_tenths(), cash);
    assert_eq!(cold.legal_value(), law);
    assert_eq!(some(cold.modular_equipment_snapshot()).items, fitted);
    assert!(cold.automatic_work_status().is_none());
    // It may remain visible as a terminal actor. Re-selecting it must never mint its cargo.
    salvage_filter(&mut cold);
    let _ = select_wreck(&mut cold, target);
    for _ in 0..120 {
        ok(cold.step(FlightInput::default()));
    }
    assert_eq!(
        cold.commodity_quantity(manifest.commodity),
        Some(held + recovered)
    );
    assert!(!cold.auto_salvage_eligible(manifest.source));
    let tick = cold.snapshot().tick;
    assert!(ok(cold.cycle_scanner_target(tick, viewport)));
    let selected = cold.targeting_snapshot();
    assert!(!ok(cold.cycle_salvage_contact(tick, viewport)));
    assert_eq!(cold.targeting_snapshot(), selected);
}

#[test]
fn manual_scanner_recovers_finite_wreck_cargo_without_targeting_modules() {
    // Replay flight inputs from a separate assisted pilot flying the same encounter.
    // Only the physical hardware on the manual pilot's own ship can produce a receipt.
    let mut guide = finance_salvage();
    let mut pilot = ok(World::from_save_bytes(&ok(guide.save_bytes())));
    for kind in [
        ModuleKind::SalvageTargeting,
        ModuleKind::Targeting(TargetingTier::Basic),
    ] {
        let items = some(pilot.modular_equipment_snapshot()).items;
        let item = some(items.iter().find(|item| item.kind == kind)).id;
        let tick = pilot.snapshot().tick;
        let review = ok(pilot.review_module(ModuleAction::Store { item }, tick));
        ok(pilot.prepare_module(review, tick)).commit();
    }
    assert_eq!(pilot.targeting_tier(), None);
    let manifest = terminal_source(&mut pilot);
    assert_eq!(terminal_source(&mut guide), manifest);
    let target = WorkTargetId::Wreck(manifest.source);
    salvage_filter(&mut guide);
    assert!(select_wreck(&mut guide, target));
    for _ in pilot.scanner_targets() {
        ok(pilot.cycle_scanner_target(pilot.snapshot().tick, [1280, 720]));
        if pilot.selected_work_target().is_some_and(|w| w.id == target) {
            break;
        }
    }
    let selected = pilot.targeting_snapshot();
    assert!(!ok(
        pilot.cycle_salvage_contact(pilot.snapshot().tick, [1280, 720])
    ));
    assert_eq!(pilot.targeting_snapshot(), selected);
    assert!(pilot.pilot_operates_selected_tool());
    assert_eq!(pilot.request_manual_wreck_recovery(), "Hold for recovery");
    let initial = some(pilot.commodity_quantity(manifest.commodity));
    for _ in 0..12000 {
        let input = guide.targeting_flight_input(FlightInput::default());
        ok(guide.step(FlightInput::default()));
        ok(pilot.step(input));
        assert_eq!(pilot.commodity_quantity(manifest.commodity), Some(initial));
        if some(guide.commodity_quantity(manifest.commodity)) > initial {
            break;
        }
    }
    assert_eq!(pilot.request_manual_wreck_recovery(), "Salvage recovered");
    assert_eq!(pilot.request_manual_wreck_recovery(), "Recovering salvage");
    let continued = ok(World::from_save_bytes(&ok(pilot.save_bytes())));
    assert_eq!(
        continued.commodity_quantity(manifest.commodity),
        Some(initial + 1)
    );
    assert_eq!(continued.targeting_tier(), None);
}

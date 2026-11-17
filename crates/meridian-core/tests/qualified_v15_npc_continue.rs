//! Compatibility witness from a byte-exact isolated native qualification checkpoint.
use meridian_core::navigation::SystemId;
use meridian_core::save::{MAX_SAVE_BYTES, SAVE_VERSION};
use meridian_core::station_services::CommodityId;
use meridian_core::{
    FireError, FlightMode, MissileContact, MissileEnd, ModuleKind, ModuleProvenance, ModuleSlot,
    OrdinaryWeapon, World,
};

const V15: &[u8] = include_bytes!("fixtures/qualified-v15-npc-flight.bin");
const TICK: u64 = 9012;
const PULSE: u64 = 10_948_529_457_934_505_010;

fn ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("qualified v15 continuation refused: {error:?}"))
}

fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("qualified v15 state is missing"))
}

fn encode_current(world: &World) -> Vec<u8> {
    let bytes = ok(world.save_bytes());
    assert!(bytes.len() <= MAX_SAVE_BYTES);
    // Root selects the combined writer. Never infer it from this old input or draft v16.
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), SAVE_VERSION);
    assert_ne!(SAVE_VERSION, 15);
    bytes
}

fn assert_one_continue_apart(before: &[u8], after: &[u8]) {
    // In the current encoding of this fixed fixture, these two freight counters
    // precede the rest of the session record. Firing leaves the preceding fields'
    // encoded lengths unchanged. Continue advances both and reseals the checksum.
    const EPOCH: usize = 1701 + b"navigation-computer/route-journal-v1/a11fe169;".len();
    const REVIEW_SEQUENCE: usize = EPOCH + 8;
    const SESSION_BODY: usize = REVIEW_SEQUENCE + 8;
    assert_eq!(before.len(), after.len());
    assert_eq!(&before[..EPOCH], &after[..EPOCH]);
    for offset in [EPOCH, REVIEW_SEQUENCE] {
        let previous = u64::from_le_bytes(ok(before[offset..offset + 8].try_into()));
        let current = u64::from_le_bytes(ok(after[offset..offset + 8].try_into()));
        assert_eq!(Some(current), previous.checked_add(1));
    }
    let checksum = before.len() - 8;
    assert_eq!(
        &before[SESSION_BODY..checksum],
        &after[SESSION_BODY..checksum]
    );
}

fn assert_same_projection(opening: &World, continued: &World) {
    assert_eq!(continued.snapshot(), opening.snapshot());
    assert_eq!(continued.mission_snapshot(), opening.mission_snapshot());
    assert_eq!(continued.combat_snapshot(), opening.combat_snapshot());
    assert_eq!(continued.traffic_snapshot(), opening.traffic_snapshot());
    assert_eq!(
        continued.ammunition_snapshot(),
        opening.ammunition_snapshot()
    );
    assert_eq!(continued.missile_snapshot(), opening.missile_snapshot());
    assert_eq!(
        continued.modular_equipment_snapshot(),
        opening.modular_equipment_snapshot()
    );
    assert_eq!(
        continued.front_laser_snapshot(),
        opening.front_laser_snapshot()
    );
    assert_eq!(
        continued.flight_resources_snapshot(),
        opening.flight_resources_snapshot()
    );
    assert_eq!(continued.credits_tenths(), opening.credits_tenths());
    assert_eq!(
        continued.commodity_quantity(CommodityId::Food),
        opening.commodity_quantity(CommodityId::Food)
    );
}

fn assert_native_checkpoint(world: &World) {
    let flight = world.snapshot();
    assert_eq!((flight.mode, flight.tick), (FlightMode::Flight, TICK));
    let nav = some(flight.navigation);
    assert_eq!(
        (nav.system, nav.visit, nav.fuel_units),
        (SystemId::RENAS, 3, 96)
    );
    assert!(nav.travel.is_none());
    assert!(some(world.mission_snapshot()).paused);
    assert_eq!(world.credits_tenths(), Some(3374));
    assert_eq!(world.commodity_quantity(CommodityId::Food), Some(1));

    let condition = some(world.combat_snapshot()).player;
    assert_eq!(
        (
            condition.front_shield_hundredths,
            condition.rear_shield_hundredths
        ),
        (1026, 30000)
    );
    assert_eq!(condition.energy_hundredths, 38000);
    assert!(!condition.destroyed);
    let fitted = some(world.front_laser_snapshot());
    assert_eq!(
        (fitted.fitted_item_id, fitted.weapon),
        (PULSE, OrdinaryWeapon::FrontPulseLaser)
    );
    let resources = some(world.flight_resources_snapshot());
    assert_eq!(
        (
            resources.energy_hundredths,
            resources.heat_fifths,
            resources.cooldown_ticks
        ),
        (38000, 0, 0)
    );
    assert_eq!(resources.energy_recharge_hundredths_per_tick, 7);
    assert!(
        some(world.modular_equipment_snapshot())
            .items
            .iter()
            .any(|item| item.kind == ModuleKind::BasicEnergySystem
                && item.id == PULSE - 2
                && item.slot == Some(ModuleSlot::Energy)
                && item.provenance == ModuleProvenance::StandardBirth)
    );

    let traffic = some(world.traffic_snapshot());
    let incoming = some(traffic.last_player_hit);
    let pirate = some(
        traffic
            .actors
            .iter()
            .find(|actor| actor.id == incoming.attacker),
    );
    assert_eq!(pirate.hull_id, "SH025");
    assert_eq!(
        (
            pirate.front_shield_hundredths,
            pirate.rear_shield_hundredths,
            pirate.energy_hundredths
        ),
        (402, 30000, 36255)
    );
    assert_eq!(
        (incoming.world_tick, incoming.physical_tick, incoming.visit),
        (TICK, 2222, 3)
    );
    assert_eq!(
        (incoming.attacker.system(), incoming.attacker.serial()),
        (SystemId::RENAS, 2)
    );
    assert!(incoming.front);
    let projectiles: Vec<_> = world.missile_projectiles().collect();
    assert_eq!(projectiles.len(), 2);
    let ammo = some(world.ammunition_snapshot());
    assert_eq!(ammo.purchases.len(), 2);
    assert_eq!(ammo.items.iter().filter(|item| !item.consumed).count(), 3);
    for (projectile, (emission, launched, settled, age, purchase)) in projectiles
        .iter()
        .zip([(1, 8759, 8830, 71, 2), (2, 8845, 8879, 34, 1)])
    {
        assert_eq!(
            (
                projectile.origin.emission,
                projectile.origin.launched_tick,
                projectile.last_tick,
                projectile.age_ticks
            ),
            (emission, launched, settled, age)
        );
        assert_eq!(projectile.ended, Some(MissileEnd::Impact));
        assert_eq!(
            projectile.contact,
            Some(MissileContact::Npc(incoming.attacker))
        );
        assert!(
            ammo.items
                .iter()
                .any(|item| item.item == projectile.origin.item
                    && item.purchase == Some(purchase)
                    && item.consumed)
        );
    }
}

#[test]
fn qualified_airborne_v15_retains_paid_npc_impacts_and_admits_next_selected_discharge() {
    assert_eq!(V15.len(), 13952);
    assert_eq!(&V15[..10], b"MERSAVE1\x0f\x00");
    assert_eq!(
        &V15[V15.len() - 8..],
        &0x3937_00b1_c1d2_7e34_u64.to_le_bytes()
    );
    let mut legacy = ok(World::from_save_bytes(V15));
    assert_native_checkpoint(&legacy);

    let before = encode_current(&legacy);
    // Exercise the new writer before any post-load command as well as after firing.
    let mut upgraded = ok(World::from_save_bytes(&before));
    assert_same_projection(&legacy, &upgraded);
    assert_one_continue_apart(&before, &encode_current(&upgraded));

    let retained_projectiles: Vec<_> = legacy.missile_projectiles().collect();
    let retained_incoming = some(legacy.traffic_snapshot()).last_player_hit;
    let retained_ammunition = legacy.ammunition_snapshot();
    let mut shot_from_legacy = None;
    for world in [&mut legacy, &mut upgraded] {
        let paused = encode_current(world);
        assert_eq!(world.request_fire_front_laser(TICK), Err(FireError::Paused));
        assert_eq!(encode_current(world), paused);
        ok(world.request_pause(false, TICK));
        let shot = ok(world.request_fire_front_laser(TICK));
        assert_eq!((shot.tick, shot.fitted_item_id), (TICK, PULSE));
        if let Some(first) = shot_from_legacy {
            assert_eq!(shot, first);
        } else {
            shot_from_legacy = Some(shot);
        }
        let resources = some(world.flight_resources_snapshot());
        assert_eq!(
            (
                resources.energy_hundredths,
                resources.heat_fifths,
                resources.cooldown_ticks
            ),
            (37900, 50, 18)
        );
        assert_eq!(
            world.missile_projectiles().collect::<Vec<_>>(),
            retained_projectiles
        );
        assert_eq!(
            some(world.traffic_snapshot()).last_player_hit,
            retained_incoming
        );
        assert_eq!(world.ammunition_snapshot(), retained_ammunition);
        assert_eq!(world.credits_tenths(), Some(3374));

        let after = encode_current(world);
        assert_eq!(
            world.request_fire_front_laser(TICK),
            Err(FireError::CoolingDown)
        );
        assert_eq!(encode_current(world), after);
        let cold = ok(World::from_save_bytes(&after));
        assert_same_projection(world, &cold);
        assert_one_continue_apart(&after, &encode_current(&cold));
    }
    assert_same_projection(&legacy, &upgraded);
    assert_one_continue_apart(&encode_current(&legacy), &encode_current(&upgraded));
}

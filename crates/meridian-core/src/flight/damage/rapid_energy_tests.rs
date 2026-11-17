//! Public paid histories and negative-only corruption. Authored UNRUN; no wallet/pose grants.
use super::{Effect, Kind, State, replay};
use crate::flight::equipment::tests::earned_world;
use crate::flight::practice::tests::{aim_at, launch, step};
use crate::{
    FireError, ModuleAction, ModuleError, ModuleKind, ModuleProvenance, ModuleSlot, OrdinaryWeapon,
    PulseContact, World,
};
use std::sync::Arc;
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("missing actual state"))
}
fn apply(world: &mut World, action: ModuleAction) -> u64 {
    let tick = world.snapshot().tick;
    let review = ok(world.review_module(action, tick));
    let prepared = ok(world.prepare_module(review, tick));
    let bytes = prepared.bytes().to_vec();
    let candidate = ok(World::from_save_bytes(&bytes));
    let item = prepared.commit();
    assert_eq!(
        world.modular_equipment_snapshot(),
        candidate.modular_equipment_snapshot()
    );
    assert_eq!(
        world.flight_resources_snapshot(),
        candidate.flight_resources_snapshot()
    );
    assert_eq!(ok(world.save_bytes()), bytes);
    item
}
fn cold(world: &World) -> World {
    let bytes = ok(world.save_bytes());
    assert_eq!(
        u16::from_le_bytes([bytes[8], bytes[9]]),
        crate::save::SAVE_VERSION
    );
    let loaded = ok(World::from_save_bytes(&bytes));
    assert_eq!(
        loaded.flight_resources_snapshot(),
        world.flight_resources_snapshot()
    );
    assert_eq!(
        loaded.modular_equipment_snapshot(),
        world.modular_equipment_snapshot()
    );
    assert_eq!(loaded.combat_snapshot(), world.combat_snapshot());
    assert_eq!(loaded.traffic_snapshot(), world.traffic_snapshot());
    assert_eq!(loaded.credits_tenths(), world.credits_tenths());
    assert_eq!(
        loaded.missile_projectiles().collect::<Vec<_>>(),
        world.missile_projectiles().collect::<Vec<_>>()
    );
    loaded
}
#[test]
fn actual_paid_beam_burst_retains_ready_energy_deficit_exact_heat_and_cold_state() {
    let mut world = earned_world(14101);
    assert_eq!(world.credits_tenths(), Some(85349));
    let item = apply(
        &mut world,
        ModuleAction::Purchase {
            kind: ModuleKind::BeamLaser,
        },
    );
    assert_eq!(world.credits_tenths(), Some(38349));
    let fit = some(world.modular_equipment_snapshot());
    let beam = some(fit.items.iter().find(|module| module.id == item));
    assert_eq!(beam.slot, Some(ModuleSlot::FrontLaser));
    assert!(matches!(
        beam.provenance,
        ModuleProvenance::Paid {
            paid_tenths: 47000,
            transaction: 1,
            ..
        }
    ));
    assert_eq!(
        some(world.front_laser_snapshot()).weapon,
        OrdinaryWeapon::FrontBeamLaser
    );
    launch(&mut world);
    crate::flight::missiles::tests::stop(&mut world);
    aim_at(&mut world, crate::docking::Vec3::new(0.0, 20000.0, 20000.0));
    for (index, expected) in [(37900, 50, 6), (37842, 94, 6), (37784, 138, 6)]
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            step(&mut world, 6);
        }
        let shot = ok(world.request_fire_front_laser(world.snapshot().tick));
        assert_eq!(shot.fitted_item_id, item);
        assert_eq!(shot.contact, PulseContact::Clear);
        let resources = some(world.flight_resources_snapshot());
        assert_eq!(
            (
                resources.energy_hundredths,
                resources.heat_fifths,
                resources.cooldown_ticks
            ),
            expected
        );
        world = cold(&world);
    }
    step(&mut world, 6);
    let resources = some(world.flight_resources_snapshot());
    assert_eq!(
        (
            resources.energy_hundredths,
            resources.heat_fifths,
            resources.cooldown_ticks
        ),
        (37826, 132, 0)
    );
    let before = ok(world.save_bytes());
    assert_eq!(
        world.request_fire_front_pulse(world.snapshot().tick),
        Err(FireError::NoFittedFrontPulseLaser)
    );
    assert_eq!(ok(world.save_bytes()), before);
    cold(&world);
    let mut fake_full = before.clone();
    let pool_start = crate::flight::persistence::pulse_save_start(&world, &fake_full);
    fake_full[pool_start..pool_start + 2].copy_from_slice(&38000_u16.to_le_bytes());
    let end = fake_full.len() - 8;
    let digest = crate::save::checksum(&fake_full[..end]);
    fake_full[end..].copy_from_slice(&digest.to_le_bytes());
    let mut still_live = cold(&world);
    assert!(still_live.restore_bytes(&fake_full).is_err());
    assert_eq!(ok(still_live.save_bytes()), before);
    // Negative fixture only: replacing the real Beam spend with Pulse must not create
    // a valid alternate history even after both cooldowns could have elapsed.
    let mut bad = ok(World::from_save_bytes(&before));
    let last = some(Arc::make_mut(&mut bad.damage.events).last_mut());
    assert_eq!(last.kind, Kind::Laser(OrdinaryWeapon::FrontBeamLaser));
    last.kind = Kind::Pulse;
    assert!(bad.save_bytes().is_err());
    assert_eq!(ok(world.save_bytes()), before);
    for shot in 4..=28 {
        ok(world.request_fire_front_laser(world.snapshot().tick));
        if shot < 28 {
            step(&mut world, 6);
        }
    }
    let resources = some(world.flight_resources_snapshot());
    assert_eq!(
        (
            resources.energy_hundredths,
            resources.heat_fifths,
            resources.cooldown_ticks
        ),
        (36334, 1238, 6)
    );
    step(&mut world, 6);
    let before_heat_refusal = ok(world.save_bytes());
    assert_eq!(
        world.request_fire_front_laser(world.snapshot().tick),
        Err(FireError::Overheated)
    );
    assert_eq!(ok(world.save_bytes()), before_heat_refusal);
    step(&mut world, 33);
    assert_eq!(some(world.flight_resources_snapshot()).heat_fifths, 1199);
    ok(world.request_fire_front_laser(world.snapshot().tick));
    let resources = some(world.flight_resources_snapshot());
    assert_eq!(
        (
            resources.energy_hundredths,
            resources.heat_fifths,
            resources.cooldown_ticks
        ),
        (36507, 1249, 6)
    );
    cold(&world);
}
#[test]
fn paid_extra_energy_preserves_born_identity_and_changes_real_pulse_recharge_only() {
    let mut world = earned_world(14201);
    let item = apply(
        &mut world,
        ModuleAction::Purchase {
            kind: ModuleKind::ExtraEnergyUnit,
        },
    );
    assert_eq!(world.credits_tenths(), Some(13349));
    let fit = some(world.modular_equipment_snapshot());
    let basic = some(
        fit.items
            .iter()
            .find(|module| module.kind == ModuleKind::BasicEnergySystem),
    );
    assert_eq!(
        (basic.id, basic.slot, basic.provenance),
        (14206, None, ModuleProvenance::StandardBirth)
    );
    assert_eq!(fit.equipment_units_used, 2);
    assert_eq!(fit.equipment_units_capacity, 24);
    assert_eq!(
        some(world.flight_resources_snapshot()).energy_recharge_hundredths_per_tick,
        14
    );
    let mut stored = cold(&world);
    apply(&mut stored, ModuleAction::Store { item });
    apply(
        &mut stored,
        ModuleAction::Install {
            item: basic.id,
            replacing: None,
        },
    );
    assert_eq!(stored.credits_tenths(), Some(13349));
    assert_eq!(
        some(stored.flight_resources_snapshot()).energy_recharge_hundredths_per_tick,
        7
    );
    assert_eq!(
        some(stored.modular_equipment_snapshot()).equipment_units_used,
        0
    );
    cold(&stored);
    launch(&mut world);
    ok(world.request_fire_front_pulse(world.snapshot().tick));
    for elapsed in 0..=8_u16 {
        if elapsed > 0 {
            step(&mut world, 1);
        }
        let resources = some(world.flight_resources_snapshot());
        assert_eq!(resources.energy_capacity_hundredths, 38000);
        assert_eq!(
            resources.energy_hundredths,
            (37900 + 14 * elapsed).min(38000)
        );
        assert_eq!(resources.heat_fifths, 50 - elapsed);
        assert_eq!(resources.cooldown_ticks, 18 - elapsed as u8);
        world = cold(&world);
    }
}
#[test]
fn actual_failed_module_publication_keeps_wallet_fit_resources_and_history_unchanged() {
    let mut world = earned_world(14301);
    let before = ok(world.save_bytes());
    let tick = world.snapshot().tick;
    let review = ok(world.review_module(
        ModuleAction::Purchase {
            kind: ModuleKind::ExtraEnergyUnit,
        },
        tick,
    ));
    let prepared = ok(world.prepare_module(review, tick));
    assert_eq!(
        prepared.publish_with(|_| Err("fixture disk failure")),
        Err("fixture disk failure")
    );
    assert_eq!(ok(world.save_bytes()), before);
    cold(&world);
}
#[test]
fn actual_port_does_not_fabricate_military_technology_or_an_ordinary_naval_offer() {
    let world = ok(World::new_commander(14401));
    let offers = world.module_offers();
    let beam = some(
        offers
            .iter()
            .find(|offer| offer.kind == ModuleKind::BeamLaser),
    );
    assert_eq!((beam.price_tenths, beam.required_technology), (47000, 7));
    let military = some(
        offers
            .iter()
            .find(|offer| offer.kind == ModuleKind::MilitaryLaser),
    );
    assert_eq!(
        (military.price_tenths, military.required_technology),
        (164000, 14)
    );
    assert!(military.technology < 14);
    assert!(!military.available);
    assert_eq!(
        world.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::MilitaryLaser
            },
            0
        ),
        Err(ModuleError::TechnologyUnavailable)
    );
    assert!(
        offers
            .iter()
            .all(|offer| offer.kind != ModuleKind::NavalLaser)
    );
    assert_eq!(
        world.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::NavalLaser
            },
            0
        ),
        Err(ModuleError::NotForSale)
    );
    let before = ok(world.save_bytes());
    assert_eq!(
        world.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::BasicEnergySystem
            },
            0
        ),
        Err(ModuleError::NotForSale)
    );
    assert_eq!(ok(world.save_bytes()), before);
}
#[test]
fn original_laser_and_energy_profile_values_do_not_confer_owned_hardware() {
    for (kind, weapon, damage, cadence) in [
        (
            ModuleKind::PulseLaser,
            OrdinaryWeapon::FrontPulseLaser,
            1500,
            18,
        ),
        (
            ModuleKind::MiningLaser,
            OrdinaryWeapon::FrontMiningLaser,
            5000,
            18,
        ),
        (
            ModuleKind::BeamLaser,
            OrdinaryWeapon::FrontBeamLaser,
            1500,
            6,
        ),
        (
            ModuleKind::MilitaryLaser,
            OrdinaryWeapon::FrontMilitaryLaser,
            2300,
            6,
        ),
        (
            ModuleKind::NavalLaser,
            OrdinaryWeapon::FrontNavalLaser,
            6400,
            6,
        ),
    ] {
        assert_eq!(OrdinaryWeapon::from_module(kind), Some(weapon));
        assert_eq!(weapon.damage_hundredths(), damage);
        assert_eq!(weapon.cooldown_ticks(), cadence);
    }
    for (kind, multiplier, units) in [
        (ModuleKind::BasicEnergySystem, 1, 0),
        (ModuleKind::ExtraEnergyUnit, 2, 2),
        (ModuleKind::NavalEnergyUnit, 3, 3),
        (ModuleKind::EliteEnergyUnit, 4, 4),
    ] {
        assert_eq!(kind.energy_recharge_multiplier(), Some(multiplier));
        assert_eq!(kind.equipment_units(), units);
    }
    let world = ok(World::new_commander(14501));
    let fit = some(world.modular_equipment_snapshot());
    assert_eq!(fit.items.len(), 2);
    assert!(fit.items.iter().all(|module| matches!(
        module.kind,
        ModuleKind::PulseLaser | ModuleKind::BasicEnergySystem
    )));
    assert_eq!(world.damage, State::fresh());
}

#[test]
fn selected_energy_rate_and_rapid_spend_replay_after_actual_damage_phases_in_one_pool() {
    // Pure resource reducer vectors, not an assertion of Naval acquisition or an emitted hit.
    let events = [
        (0, 0, 1, Effect::Recharge(14)),
        (
            1,
            1,
            1,
            Effect::Hit {
                front: true,
                damage: 16000,
            },
        ),
        (
            1,
            1,
            2,
            Effect::Hit {
                front: true,
                damage: 16000,
            },
        ),
        (
            1,
            2,
            0,
            Effect::Hit {
                front: true,
                damage: 1500,
            },
        ),
        (1, 3, 0, Effect::Laser(OrdinaryWeapon::FrontNavalLaser)),
    ];
    let pools = ok(replay(&events, 7));
    let mut writer = crate::save::Writer::new();
    pools.pulse.encode_save(&mut writer);
    assert_eq!(
        writer.bytes,
        [
            34484_u16.to_le_bytes().as_slice(),
            44_u16.to_le_bytes().as_slice(),
            &[0]
        ]
        .concat()
    );
    assert_eq!(pools.shields, [18, 30000]);
}
#[test]
fn extending_selected_laser_classes_keeps_nine_byte_resource_events() {
    let state = ok(State::fresh().laser(1, OrdinaryWeapon::FrontBeamLaser));
    let mut writer = crate::save::Writer::new();
    state.encode(&mut writer);
    assert_eq!(writer.bytes.len(), 3 + 9);
    let mut reader = crate::save::Reader::new(&writer.bytes);
    assert_eq!(
        ok(State::decode(&mut reader, crate::save::SAVE_VERSION)),
        state
    );
    assert!(reader.finished());
    let mut reader = crate::save::Reader::new(&writer.bytes);
    assert!(State::decode(&mut reader, 15).is_err());
}

#[test]
fn genuinely_paid_beam_hits_a_native_actor_and_same_damage_pulse_alias_cannot_restore() {
    use crate::flight::equipment::tests::{earned_from, journey};
    use crate::flight::missiles::tests::{fly_axis, stop};
    use crate::flight::practice::tests::consent;
    use crate::navigation::SystemId;
    use crate::traffic::{NpcPhase, Population};
    use crate::{FlightInput, FlightMode};
    let mut world = ok(World::new_commander(12005));
    // Actual consent protects traffic during the unchanged finite-market funding route.
    // Close it at the real Avenor berth before any combat; no actor clock or pose is installed.
    consent(&mut world);
    let mut world = earned_from(world);
    if some(world.snapshot().navigation).fuel_units < 68 {
        ok(world.request_refuel(world.snapshot().tick));
    }
    journey(&mut world, SystemId::AVENOR);
    assert_eq!(world.snapshot().mode, FlightMode::Docked);
    let tick = world.snapshot().tick;
    let closure = ok(world.review_practice_closure(tick));
    ok(world.prepare_practice_closure(closure, tick)).commit();
    let opening_cash = some(world.credits_tenths());
    let item = apply(
        &mut world,
        ModuleAction::Purchase {
            kind: ModuleKind::BeamLaser,
        },
    );
    assert_eq!(world.credits_tenths(), Some(opening_cash - 47000));
    launch(&mut world);
    stop(&mut world);
    fly_axis(&mut world, 0, -600.0, 10.0);
    let mut hit = None;
    for _ in 0..1500 {
        let scene = some(world.traffic_snapshot());
        if let Some(target) = scene.actors.iter().find(|actor| {
            actor.population == Population::Trader && actor.phase != NpcPhase::Complete
        }) {
            let state = world.snapshot();
            let pose = some(crate::flight::flight_pose(
                state.position,
                state.orientation,
            ));
            let delta = crate::docking::Vec3::new(
                target.pose.position.x - pose.position.x,
                0.0,
                target.pose.position.z - pose.position.z,
            );
            let desired = delta.x.atan2(delta.z);
            let yaw = pose.orientation.z.x.atan2(pose.orientation.z.z);
            let error = (desired - yaw).sin().atan2((desired - yaw).cos());
            if matches!(
                world.pulse_scene(pose),
                Ok((_, PulseContact::Npc { .. }, _))
            ) {
                let resources = some(world.flight_resources_snapshot());
                let shot = ok(world.request_fire_front_laser(state.tick));
                let PulseContact::Npc { id, .. } = shot.contact else {
                    panic!("lost actual contact");
                };
                assert_eq!(shot.fitted_item_id, item);
                assert_eq!(
                    some(world.flight_resources_snapshot()).energy_hundredths,
                    resources.energy_hundredths - 100
                );
                let beam = some(some(world.traffic_snapshot()).last_beam);
                assert_eq!(beam.weapon, OrdinaryWeapon::FrontBeamLaser);
                assert_eq!(beam.victim, id);
                hit = Some(id);
                break;
            }
            ok(world.step(FlightInput {
                yaw: (error / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32,
                ..FlightInput::default()
            }));
        } else {
            ok(world.step(FlightInput::default()));
        }
    }
    assert!(
        hit.is_some(),
        "public paid Beam never reached the genuine native trader"
    );
    let mut live = cold(&world);
    let before = ok(live.save_bytes());
    let mut bad = ok(World::from_save_bytes(&before));
    bad.traffic
        .corrupt_beams(|beams| some(beams.last_mut()).weapon = OrdinaryWeapon::FrontPulseLaser);
    // Pulse and Beam deal the same1500 damage. Resource sums alone cannot detect this
    // class alias; the historical fitted paid item must independently reject it.
    assert!(bad.save_bytes().is_err());
    let poisoned = ok(bad.unchecked_test_save());
    assert!(live.restore_bytes(&poisoned).is_err());
    assert_eq!(ok(live.save_bytes()), before);
}

#[test]
fn paid_beam_interception_fast_gunnery_and_later_station_refit_keep_real_custody() {
    use crate::flight::equipment::tests::journey;
    use crate::flight::missiles::tests::{dock, stop};
    use crate::flight::practice::tests::consent;
    use crate::navigation::SystemId;
    let mut world = earned_world(15101);
    if some(world.snapshot().navigation).fuel_units < 68 {
        ok(world.request_refuel(world.snapshot().tick));
    }
    journey(&mut world, SystemId::AVENOR);
    let beam = apply(
        &mut world,
        ModuleAction::Purchase {
            kind: ModuleKind::BeamLaser,
        },
    );
    let cash = world.credits_tenths();
    consent(&mut world);
    launch(&mut world);
    stop(&mut world);
    let beacons = some(some(world.practice_snapshot()).beacons);
    let tick = world.snapshot().tick;
    ok(world.request_missile_lock(Some(beacons[1].id), tick));
    ok(world.request_missile_arm(true, tick));
    let origin = ok(world.request_fire_missile(tick));
    let shot = ok(world.request_fire_front_laser(tick));
    assert!(matches!(shot.contact,PulseContact::Projectile {id,..} if id==origin.projectile));
    assert_eq!(shot.fitted_item_id, beam);
    assert_eq!(some(world.missile_rounds()).iter().flatten().count(), 2);
    world = cold(&world);
    step(&mut world, 6);
    assert!(matches!(
        ok(world.request_fire_front_laser(world.snapshot().tick)).contact,
        PulseContact::PracticeBeacon {
            first_hit: true,
            ..
        }
    ));
    aim_at(&mut world, beacons[0].pose.position);
    assert!(
        matches!(ok(world.request_fire_front_laser(world.snapshot().tick)).contact,PulseContact::PracticeBeacon {id,first_hit:true,..} if id==beacons[0].id)
    );
    let left_tick = world.snapshot().tick;
    aim_at(&mut world, beacons[2].pose.position);
    let interval = world.snapshot().tick - left_tick;
    assert!(
        (6..18).contains(&interval),
        "this public turn must exercise the actual fast cadence"
    );
    assert!(
        matches!(ok(world.request_fire_front_laser(world.snapshot().tick)).contact,PulseContact::PracticeBeacon {id,first_hit:true,..} if id==beacons[2].id)
    );
    assert_eq!(some(world.practice_snapshot()).first_hits, 3);
    world = cold(&world);
    step(&mut world, 179);
    assert!(!some(world.practice_reception_snapshot()).ready);
    step(&mut world, 1);
    assert!(some(world.practice_reception_snapshot()).ready);
    dock(&mut world);
    let tick = world.snapshot().tick;
    let review = ok(world.review_practice_reception(tick));
    ok(world.prepare_practice_reception(review, tick)).commit();
    assert_eq!(world.credits_tenths(), cash);
    world = cold(&world);
    launch(&mut world);
    stop(&mut world);
    aim_at(&mut world, crate::docking::Vec3::new(70.0, 0.0, 136.32));
    assert!(matches!(
        ok(world.request_fire_front_laser(world.snapshot().tick)).contact,
        PulseContact::StationBlocked { .. }
    ));
    let receipt = some(some(world.combat_snapshot()).last_ordinary_impact);
    assert_eq!(
        (receipt.item, receipt.weapon),
        (beam, OrdinaryWeapon::FrontBeamLaser)
    );
    world = cold(&world);
    dock(&mut world);
    apply(&mut world, ModuleAction::Store { item: beam });
    let pulse = some(world.modular_equipment_snapshot())
        .items
        .into_iter()
        .find(|item| {
            item.kind == ModuleKind::PulseLaser
                && item.provenance == ModuleProvenance::StandardBirth
        })
        .unwrap_or_else(|| panic!("same born Pulse"));
    apply(
        &mut world,
        ModuleAction::Install {
            item: pulse.id,
            replacing: None,
        },
    );
    assert_eq!(some(world.pulse_laser_snapshot()).fitted_item_id, pulse.id);
    assert_eq!(
        some(world.combat_snapshot()).last_ordinary_impact,
        Some(receipt)
    );
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(some(world.missile_rounds()).iter().flatten().count(), 2);
    cold(&world);
}

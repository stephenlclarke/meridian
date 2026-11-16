//! Public positive shots use genuine paid inventory. Reducer/resource fixtures are labelled below.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::missiles::tests::{dock, stop};
use crate::flight::practice::tests::{aim_at, launch, step};
use crate::{ModuleAction, OrdinaryWeapon};

fn buy(w: &mut World) -> u64 {
    let tick = w.snapshot().tick;
    let review = w
        .review_module(ModuleAction::PurchaseMiningLaser, tick)
        .unwrap();
    w.prepare_module(review, tick).unwrap().commit()
}
fn mining(seed: u64) -> (World, u64) {
    let mut w = crate::flight::equipment::tests::earned_world(seed);
    let item = buy(&mut w);
    assert_eq!(w.credits_tenths(), Some(52_349));
    (w, item)
}
fn cold(w: &World) -> World {
    let bytes = w.save_bytes().unwrap();
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 16);
    let loaded = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(loaded.front_laser_snapshot(), w.front_laser_snapshot());
    assert_eq!(
        loaded.flight_resources_snapshot(),
        w.flight_resources_snapshot()
    );
    assert_eq!(loaded.combat_snapshot(), w.combat_snapshot());
    assert_eq!(
        loaded.modular_equipment_snapshot(),
        w.modular_equipment_snapshot()
    );
    assert_eq!(loaded.credits_tenths(), w.credits_tenths());
    loaded
}
fn station(w: &mut World) {
    stop(w);
    aim_at(w, Vec3::new(70.0, 0.0, 136.32));
}

#[test]
fn genuine_paid_mining_front_shot_uses_same_pool_cadence_and_work_cooldown_without_a_pulse_alias() {
    let (mut w, item) = mining(13101);
    let snapshot = w.front_laser_snapshot().unwrap();
    assert_eq!(snapshot.fitted_item_id, item);
    assert_eq!(snapshot.weapon, OrdinaryWeapon::FrontMiningLaser);
    assert_eq!(snapshot.damage_hundredths, 5000);
    assert_eq!(
        (snapshot.range, snapshot.cooldown_ticks_per_shot),
        (6000, 18)
    );
    assert!(w.pulse_laser_snapshot().is_none());
    assert_eq!(w.fitted_mining_tool().unwrap().item, item);
    launch(&mut w);
    let tick = w.snapshot().tick;
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.request_fire_front_pulse(tick),
        Err(FireError::NoFittedFrontPulseLaser)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let shot = w.request_fire_front_laser(tick).unwrap();
    assert_eq!(shot.fitted_item_id, item);
    assert_eq!(shot.tick, tick);
    let resources = w.flight_resources_snapshot().unwrap();
    assert_eq!(
        (
            resources.energy_hundredths,
            resources.heat_fifths,
            resources.cooldown_ticks
        ),
        (37_900, 50, 18)
    );
    assert_eq!(
        w.prepare_mining_resources(item, tick),
        Err(FireError::CoolingDown)
    );
    let fired = w.save_bytes().unwrap();
    assert_eq!(
        w.request_fire_front_laser(tick),
        Err(FireError::CoolingDown)
    );
    assert_eq!(w.save_bytes().unwrap(), fired);
    cold(&w);
    step(&mut w, 17);
    assert_eq!(w.flight_resources_snapshot().unwrap().cooldown_ticks, 1);
    assert_eq!(
        w.request_fire_front_laser(w.snapshot().tick),
        Err(FireError::CoolingDown)
    );
    cold(&w);
    step(&mut w, 1);
    assert!(w.prepare_mining_resources(item, w.snapshot().tick).is_ok());
    assert_eq!(
        w.request_fire_front_laser(w.snapshot().tick)
            .unwrap()
            .fitted_item_id,
        item
    );
    assert_eq!(w.credits_tenths(), Some(52_349));
    cold(&w);
}

#[test]
fn empty_front_slot_refuses_generic_fire_and_continue_retains_paid_stored_custody() {
    let (mut w, item) = mining(13201);
    let tick = w.snapshot().tick;
    let r = w.review_module(ModuleAction::Store { item }, tick).unwrap();
    w.prepare_module(r, tick).unwrap().commit();
    assert!(w.front_laser_snapshot().is_none());
    assert!(w.fitted_mining_tool().is_none());
    let mut w = cold(&w);
    launch(&mut w);
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.request_fire_front_laser(w.snapshot().tick),
        Err(FireError::NoFittedFrontLaser)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    assert_eq!(w.credits_tenths(), Some(52_349));
}

#[test]
fn mining_station_receipt_binds_kind_and_paid_item_and_survives_later_born_pulse_refit() {
    let (mut w, item) = mining(13301);
    launch(&mut w);
    station(&mut w);
    let shields = w.combat_snapshot().unwrap().player;
    let shot = w.request_fire_front_laser(w.snapshot().tick).unwrap();
    assert!(matches!(shot.contact, PulseContact::StationBlocked { .. }));
    let hit = w.combat_snapshot().unwrap().last_ordinary_impact.unwrap();
    assert_eq!(
        (hit.item, hit.weapon, hit.target),
        (
            item,
            OrdinaryWeapon::FrontMiningLaser,
            OrdinaryTarget::MainStation
        )
    );
    assert_eq!(w.legal_value(), Some(64));
    assert_eq!(
        w.combat_snapshot().unwrap().player.front_shield_hundredths,
        shields.front_shield_hundredths
    );
    let valid = w.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let before = live.save_bytes().unwrap();
    w.combat.last.as_mut().unwrap().weapon = OrdinaryWeapon::FrontPulseLaser;
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
    w = World::from_save_bytes(&valid).unwrap();
    w.combat.last.as_mut().unwrap().item = 13308;
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
    w = World::from_save_bytes(&valid).unwrap();
    step(&mut w, 180);
    dock(&mut w);
    let tick = w.snapshot().tick;
    let r = w
        .review_module(
            ModuleAction::Install {
                item: 13308,
                replacing: Some(item),
            },
            tick,
        )
        .unwrap();
    w.prepare_module(r, tick).unwrap().commit();
    assert_eq!(
        w.front_laser_snapshot().unwrap().weapon,
        OrdinaryWeapon::FrontPulseLaser
    );
    assert_eq!(w.combat_snapshot().unwrap().last_ordinary_impact, Some(hit));
    cold(&w);
}

#[test]
fn generic_and_pulse_only_commands_emit_identical_actual_born_pulse_results() {
    let mut pulse = World::new_commander(13401).unwrap();
    launch(&mut pulse);
    let mut generic = World::from_save_bytes(&pulse.save_bytes().unwrap()).unwrap();
    assert_eq!(
        pulse
            .request_fire_front_pulse(pulse.snapshot().tick)
            .unwrap(),
        generic
            .request_fire_front_laser(generic.snapshot().tick)
            .unwrap()
    );
    assert_eq!(
        pulse.flight_resources_snapshot(),
        generic.flight_resources_snapshot()
    );
    assert_eq!(pulse.combat_snapshot(), generic.combat_snapshot());
    assert_eq!(
        generic.front_laser_snapshot().unwrap().damage_hundredths,
        1500
    );
    cold(&pulse);
    cold(&generic);
}

#[test]
fn real_mining_prepared_contact_refuses_after_continue_epoch_without_second_debit() {
    let (mut w, _) = mining(13501);
    launch(&mut w);
    station(&mut w);
    let bytes = w.save_bytes().unwrap();
    let mut other = World::from_save_bytes(&bytes).unwrap();
    let actual = other
        .request_fire_front_laser(other.snapshot().tick)
        .unwrap();
    let prepared = w.prepare_pulse_impact(actual).unwrap();
    w.restore_bytes(&bytes).unwrap();
    let before = w.save_bytes().unwrap();
    assert_eq!(prepared.commit(&mut w), Err(FireError::StaleCommand));
    assert_eq!(w.save_bytes().unwrap(), before);
    assert_eq!(w.credits_tenths(), Some(52_349));
}

#[test]
fn source_mining_combat_heat_guard_stays_distinct_from_selected_rock_work_guard() {
    // Resource unit fixture only. This does not claim a public physical mining/extraction action.
    let mut w = crate::save::Writer::new();
    w.u16(38_000);
    w.u16(1151);
    w.u8(0);
    let mut r = crate::save::Reader::new(&w.bytes);
    let pool = super::super::weapons::PulseResources::decode_save(&mut r).unwrap();
    assert!(pool.prepare_discharge().is_ok());
    assert_eq!(pool.prepare_mining_discharge(), Err(FireError::Overheated));
}

#[test]
fn mining_damage_reducer_applies_fifty_to_selected_shield_then_shared_energy() {
    // Internal reducer fixture only. There is no public NPC laser victim producer in this increment.
    let mut state = State::default();
    let mut pool = super::super::weapons::PulseResources::default();
    for n in 1..=7 {
        state.resolve_impact(
            &mut pool,
            OrdinaryTarget::Player { front: false },
            OrdinaryWeapon::FrontMiningLaser,
        );
        assert_eq!(state.rear, 30_000_u16.saturating_sub(n * 5000));
        assert_eq!(state.front, 30_000);
    }
    let mut encoded = crate::save::Writer::new();
    pool.encode_save(&mut encoded);
    assert_eq!(&encoded.bytes[..2], &33_000_u16.to_le_bytes());
    assert!(state.unbound_player_impact);
}

#[test]
fn generic_paid_mining_fire_refuses_docked_stale_and_paused_commands_without_mutation() {
    let (mut w, _) = mining(13701);
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.request_fire_front_laser(w.snapshot().tick),
        Err(FireError::NotInFlight)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    launch(&mut w);
    let tick = w.snapshot().tick;
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.request_fire_front_laser(tick - 1),
        Err(FireError::StaleCommand)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let resources = w.flight_resources_snapshot();
    let inventory = w.modular_equipment_snapshot();
    w.request_pause(true, tick).unwrap();
    let paused = w.save_bytes().unwrap();
    assert_eq!(w.request_fire_front_laser(tick), Err(FireError::Paused));
    assert_eq!(w.save_bytes().unwrap(), paused);
    w.request_pause(false, tick).unwrap();
    // Pause/Continue deliberately advances the session epoch, not the pool or custody.
    assert_eq!(w.flight_resources_snapshot(), resources);
    assert_eq!(w.modular_equipment_snapshot(), inventory);
    assert_eq!(w.credits_tenths(), Some(52_349));
    assert!(w.request_fire_front_laser(tick).is_ok());
    cold(&w);
}

#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::equipment::tests::{earned_world, journey};
use crate::flight::missiles::tests::{fire, stop};
use crate::flight::practice::tests::{consent, launch, step};
use crate::navigation::SystemId;
use crate::{ModuleAction, ModuleKind, ModuleProvenance};

fn ready_paid() -> World {
    let mut world = earned_world(12005);
    let tick = world.snapshot().tick;
    let review = world
        .review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::Ecm,
            },
            tick,
        )
        .unwrap();
    world.prepare_module(review, tick).unwrap().commit();
    assert!(matches!(
        world.fitted_ecm().unwrap().provenance,
        ModuleProvenance::Paid {
            paid_tenths: 21_000,
            ..
        }
    ));
    if world.snapshot().navigation.unwrap().fuel_units < 68 {
        world.request_refuel(world.snapshot().tick).unwrap();
    }
    journey(&mut world, SystemId::AVENOR);
    consent(&mut world);
    launch(&mut world);
    stop(&mut world);
    world
}
fn fire_round(world: &mut World) {
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    fire(world, target);
}
fn projectile(world: &World, index: usize) -> ProjectileSnapshot {
    world.missile_projectiles().nth(index).unwrap()
}
fn cold(world: &World) -> World {
    World::from_save_bytes(&world.save_bytes().unwrap()).unwrap()
}
fn refused(world: &World) {
    let bytes = world.unchecked_test_save().unwrap();
    assert!(World::from_save_bytes(&bytes).is_err());
}

#[test]
fn ordinary_paid_field_ends_actual_live_and_newborn_rounds_without_impact_or_refund() {
    let mut world = ready_paid();
    fire_round(&mut world);
    step(&mut world, 1);
    let before = projectile(&world, 0);
    assert!(before.ended.is_none() && before.age_ticks > 0);
    let cash = world.credits_tenths();
    let cargo = world.service_snapshot();
    let rounds = world.missile_rounds();
    let resources = world.flight_resources_snapshot().unwrap();
    let blast_cursor = world.combat.emissions;
    let tick = world.snapshot().tick;
    let bytes = world.save_bytes().unwrap();
    let prepared = world.prepare_ecm(tick).unwrap();
    assert_eq!(prepared.terminated(), 1);
    assert_eq!(prepared.publish_with(|_| Err::<(), _>("disk")), Err("disk"));
    assert_eq!(world.save_bytes().unwrap(), bytes);
    let prepared = world.prepare_ecm(tick).unwrap();
    let exact = prepared.bytes().to_vec();
    drop(prepared);
    assert_eq!(world.save_bytes().unwrap(), bytes);
    let activation = world.request_ecm(tick).unwrap();
    assert_eq!(world.save_bytes().unwrap(), exact);
    let ended = projectile(&world, 0);
    assert_eq!(ended.ended, Some(MissileEnd::Ecm));
    assert_eq!(ended.origin, before.origin);
    assert_eq!(
        (ended.position, ended.age_ticks, ended.propulsion_travel),
        (before.position, before.age_ticks, before.propulsion_travel)
    );
    assert!(ended.contact.is_none() && ended.intercepting_emission.is_none());
    assert_eq!(
        world.ecm_termination(ended.origin.projectile),
        Some(EcmTermination {
            projectile: ended.origin.projectile,
            activation_sequence: activation.sequence,
            world_tick: tick,
            physical_tick: activation.physical_tick,
            after_emission: blast_cursor
        })
    );
    let after = world.flight_resources_snapshot().unwrap();
    assert_eq!(after.energy_hundredths, resources.energy_hundredths - 2200);
    assert_eq!(
        (after.heat_fifths, after.cooldown_ticks),
        (resources.heat_fifths, resources.cooldown_ticks)
    );
    assert_eq!(world.combat.emissions, blast_cursor);
    assert_eq!(world.missile_rounds(), rounds);
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(world.service_snapshot(), cargo);
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 0);
    let restored = cold(&world);
    assert_eq!(projectile(&restored, 0), ended);
    assert_eq!(restored.ecm_snapshot().unwrap().active, None);
    assert_eq!(restored.ecm_snapshot().unwrap().remaining_ticks, 32);
    assert_eq!(restored.ecm_snapshot().unwrap().terminated_projectiles, 1);
    let mut bad = cold(&world);
    Arc::make_mut(&mut bad.ecm.field.terminations).clear();
    refused(&bad);
    let mut bad = cold(&world);
    Arc::make_mut(&mut bad.ecm.field.terminations)[0].after_emission += 1;
    refused(&bad);

    // The existing launcher cooldown is independent and longer than the field.
    step(&mut world, 60);
    let mut restored_next = cold(&world);
    let field = world.request_ecm(world.snapshot().tick).unwrap();
    fire_round(&mut world);
    let newborn = projectile(&world, 1);
    assert_eq!(newborn.ended, Some(MissileEnd::Ecm));
    assert_eq!(newborn.age_ticks, 0);
    assert_eq!(
        world
            .ecm_termination(newborn.origin.projectile)
            .unwrap()
            .activation_sequence,
        field.sequence
    );
    assert_eq!(
        world
            .ecm_termination(newborn.origin.projectile)
            .unwrap()
            .after_emission,
        newborn.origin.emission
    );
    assert_eq!(world.ecm_snapshot().unwrap().terminated_projectiles, 2);
    assert_eq!(projectile(&cold(&world), 1), newborn);

    // Same actual start, but a real cold Continue consumes live effect authority.
    let field = restored_next
        .request_ecm(restored_next.snapshot().tick)
        .unwrap();
    restored_next = cold(&restored_next);
    assert_eq!(restored_next.ecm_snapshot().unwrap().remaining_ticks, 32);
    assert_eq!(restored_next.ecm_snapshot().unwrap().active, None);
    assert_eq!(
        restored_next.request_ecm(restored_next.snapshot().tick),
        Err(EcmError::FieldActive)
    );
    fire_round(&mut restored_next);
    let survivor = projectile(&restored_next, 1);
    assert!(survivor.ended.is_none());
    assert!(
        restored_next
            .ecm_termination(survivor.origin.projectile)
            .is_none()
    );
    assert_eq!(
        restored_next.ecm_snapshot().unwrap().last_activation,
        Some(field)
    );
    let mut bad = cold(&restored_next);
    Arc::make_mut(&mut bad.ecm.field.fields)
        .last_mut()
        .unwrap()
        .closed = None;
    refused(&bad);
    let mut bad = cold(&restored_next);
    Arc::make_mut(&mut bad.ecm.field.fields)
        .last_mut()
        .unwrap()
        .closed
        .as_mut()
        .unwrap()
        .after_emission += 1;
    refused(&bad);
    step(&mut restored_next, 32);
    let fresh = restored_next
        .request_ecm(restored_next.snapshot().tick)
        .unwrap();
    assert_eq!(fresh.sequence, field.sequence + 1);
    assert_eq!(
        restored_next
            .ecm_termination(survivor.origin.projectile)
            .unwrap()
            .activation_sequence,
        fresh.sequence
    );
    assert_eq!(restored_next.practice_snapshot().unwrap().first_hits, 0);
    assert_eq!(restored_next.credits_tenths(), cash);
    assert_eq!(
        projectile(&cold(&restored_next), 1).ended,
        Some(MissileEnd::Ecm)
    );

    // Ordinary game discharges retain their actual ordering at the same physical
    // clock, with a single shared energy balance in either order.
    step(&mut restored_next, 32);
    let tick = restored_next.snapshot().tick;
    let energy = restored_next
        .flight_resources_snapshot()
        .unwrap()
        .energy_hundredths;
    restored_next.request_fire_front_laser(tick).unwrap();
    let emission = restored_next.combat.emissions;
    let field = restored_next.request_ecm(tick).unwrap();
    assert_eq!(
        restored_next
            .ecm
            .field
            .field(field.sequence)
            .unwrap()
            .after_emission,
        emission
    );
    assert_eq!(
        restored_next
            .flight_resources_snapshot()
            .unwrap()
            .energy_hundredths,
        energy - 2300
    );
    step(&mut restored_next, 32);
    let tick = restored_next.snapshot().tick;
    let energy = restored_next
        .flight_resources_snapshot()
        .unwrap()
        .energy_hundredths;
    let emission = restored_next.combat.emissions;
    let field = restored_next.request_ecm(tick).unwrap();
    restored_next.request_fire_front_laser(tick).unwrap();
    assert_eq!(
        restored_next
            .ecm
            .field
            .field(field.sequence)
            .unwrap()
            .after_emission,
        emission
    );
    assert_eq!(restored_next.combat.emissions, emission + 1);
    assert_eq!(
        restored_next
            .flight_resources_snapshot()
            .unwrap()
            .energy_hundredths,
        energy - 2300
    );
    assert_eq!(
        cold(&restored_next).ecm_snapshot().unwrap().remaining_ticks,
        32
    );
}

#[test]
fn legacy_v2_timer_shape_continues_without_retroactive_field_or_new_termination() {
    let mut world = ready_paid();
    let activation = world.request_ecm(world.snapshot().tick).unwrap();
    // The exact former v2 codec had these activation/local records only and
    // did not produce any projectile terminal event. No synthetic game missile.
    world.ecm.field = History::legacy(world.ecm.activations().len());
    let bytes = world.save_bytes().unwrap();
    assert!(bytes.windows(8).any(|tag| tag == b"MECM0002"));
    let decoded = World::decode_saved_world(&bytes).unwrap();
    assert_eq!(decoded.save_bytes().unwrap(), bytes);
    let mut loaded = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(loaded.ecm_snapshot().unwrap().active, None);
    assert_eq!(loaded.ecm_snapshot().unwrap().remaining_ticks, 32);
    assert_eq!(
        loaded.ecm_snapshot().unwrap().last_activation,
        Some(activation)
    );
    fire_round(&mut loaded);
    let survivor = projectile(&loaded, 0);
    assert!(survivor.ended.is_none());
    step(&mut loaded, 32);
    let fresh = loaded.request_ecm(loaded.snapshot().tick).unwrap();
    assert_eq!(fresh.sequence, activation.sequence + 1);
    assert!(
        loaded
            .save_bytes()
            .unwrap()
            .windows(8)
            .any(|tag| tag == b"MECM0003")
    );
    assert_eq!(
        loaded
            .ecm_termination(survivor.origin.projectile)
            .unwrap()
            .activation_sequence,
        fresh.sequence
    );
    assert_eq!(projectile(&cold(&loaded), 0).ended, Some(MissileEnd::Ecm));
}

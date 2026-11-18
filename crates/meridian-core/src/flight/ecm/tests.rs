//! Source-authored tests, UNRUN. Positive state comes from genuine public transactions/flight.
#![allow(clippy::unwrap_used)]
use super::{ECM_ENERGY_COST_HUNDREDTHS, ECM_FIELD_TICKS, EcmError, MissileEnd, State, World};
use crate::flight::equipment::tests::{earned_world, journey};
use crate::flight::missiles::tests::{fire, stop};
use crate::flight::practice::tests::{consent, launch, step};
use crate::flight::weapons::PulseResources;
use crate::navigation::SystemId;
use crate::save::{Reader, Writer};
use crate::{ModuleAction, ModuleKind, ModuleProvenance, ModuleSlot, ProjectileSnapshot};
use std::sync::Arc;

fn apply(world: &mut World, action: ModuleAction) -> u64 {
    let tick = world.snapshot().tick;
    let review = world.review_module(action, tick).unwrap();
    world.prepare_module(review, tick).unwrap().commit()
}
fn paid() -> World {
    let mut world = earned_world(12005);
    apply(
        &mut world,
        ModuleAction::Purchase {
            kind: ModuleKind::EcmSystem,
        },
    );
    world
}
fn ready() -> World {
    let mut world = paid();
    // Practice consent requires its actual Avenor main berth, reached by paid public travel.
    if world.snapshot().navigation.unwrap().fuel_units < 68 {
        world.request_refuel(world.snapshot().tick).unwrap();
    }
    journey(&mut world, SystemId::AVENOR);
    consent(&mut world);
    launch(&mut world);
    stop(&mut world);
    world
}
fn missile(world: &World, ordinal: usize) -> ProjectileSnapshot {
    world.missile_snapshot().unwrap().projectiles[ordinal].unwrap()
}
fn launch_missile(world: &mut World) {
    let target = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    fire(world, target);
}
fn same(a: &World, b: &World) {
    assert_eq!(a.snapshot(), b.snapshot());
    let a_ecm = a.ecm_snapshot().unwrap();
    let b_ecm = b.ecm_snapshot().unwrap();
    assert_eq!(
        (
            a_ecm.fitted_item,
            a_ecm.remaining_ticks,
            a_ecm.activations,
            a_ecm.terminated_missiles
        ),
        (
            b_ecm.fitted_item,
            b_ecm.remaining_ticks,
            b_ecm.activations,
            b_ecm.terminated_missiles
        ),
    );
    assert!(
        b_ecm.field.is_none(),
        "Continue must not restore live field permission"
    );
    assert_eq!(a.missile_snapshot(), b.missile_snapshot());
    assert_eq!(a.flight_resources_snapshot(), b.flight_resources_snapshot());
    assert_eq!(a.combat_snapshot(), b.combat_snapshot());
    assert_eq!(a.emp_snapshot(), b.emp_snapshot());
    assert_eq!(a.practice_snapshot(), b.practice_snapshot());
    assert_eq!(
        a.modular_equipment_snapshot(),
        b.modular_equipment_snapshot()
    );
    assert_eq!(a.service_snapshot(), b.service_snapshot());
    assert_eq!(a.credits_tenths(), b.credits_tenths());
    assert_eq!(a.legal_value(), b.legal_value());
}
fn cold(world: &World) -> World {
    let loaded = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    same(world, &loaded);
    loaded
}
fn refused(world: &mut World, tick: u64, error: EcmError) {
    let before = world.save_bytes().unwrap();
    assert_eq!(world.request_ecm(tick), Err(error));
    assert_eq!(world.save_bytes().unwrap(), before);
}
fn corrupt(world: &World, change: impl FnOnce(&mut World)) {
    // Decoder-only negative fixtures; no altered state is admitted as a positive scenario.
    let mut forged = cold(world);
    change(&mut forged);
    let bytes = forged.unchecked_test_save().unwrap();
    let mut target = cold(world);
    let before = target.save_bytes().unwrap();
    assert!(World::from_save_bytes(&bytes).is_err());
    assert!(target.restore_bytes(&bytes).is_err());
    assert_eq!(target.save_bytes().unwrap(), before);
}
#[test]
fn paid_original_hardware_debits_2100cr_and_uses_one_actual_utility_mount() {
    let mut world = earned_world(12005);
    let cash = world.credits_tenths().unwrap();
    let pools = world.flight_resources_snapshot();
    let cargo = world.service_snapshot();
    let front = world.front_laser_snapshot();
    let offer = world
        .module_offers()
        .into_iter()
        .find(|o| o.kind == ModuleKind::EcmSystem)
        .unwrap();
    assert_eq!(
        (
            offer.price_tenths,
            offer.required_technology,
            offer.available
        ),
        (21000, 5, true)
    );
    assert_eq!(
        (ModuleKind::EcmSystem.family(), ModuleKind::EcmSystem.node()),
        ("utility-ecm", "E.C.M. System")
    );
    let item = apply(
        &mut world,
        ModuleAction::Purchase {
            kind: ModuleKind::EcmSystem,
        },
    );
    let snapshot = world.modular_equipment_snapshot().unwrap();
    let hardware = snapshot.items.iter().find(|i| i.id == item).unwrap();
    assert_eq!(hardware.slot, Some(ModuleSlot::UtilityEcm));
    assert!(matches!(
        hardware.provenance,
        ModuleProvenance::Paid {
            paid_tenths: 21000,
            ..
        }
    ));
    assert_eq!(snapshot.utility_mounts_used, 1);
    assert_eq!(world.credits_tenths(), Some(cash - 21000));
    assert_eq!(world.flight_resources_snapshot(), pools);
    assert_eq!(world.service_snapshot(), cargo);
    assert_eq!(world.front_laser_snapshot(), front);
    assert_eq!(world.ecm_snapshot().unwrap().fitted_item, Some(item));
    cold(&world);
    apply(&mut world, ModuleAction::Store { item });
    assert!(world.ecm_snapshot().unwrap().fitted_item.is_none());
    assert_eq!(
        world
            .modular_equipment_snapshot()
            .unwrap()
            .utility_mounts_used,
        0
    );
    apply(
        &mut world,
        ModuleAction::Install {
            item,
            replacing: None,
        },
    );
    assert_eq!(world.ecm_snapshot().unwrap().fitted_item, Some(item));
    assert_eq!(world.credits_tenths(), Some(cash - 21000));
    cold(&world);
}
#[test]
fn refused_equipment_writer_cannot_publish_paid_cash_or_fit() {
    let mut world = earned_world(12005);
    let before = world.save_bytes().unwrap();
    let tick = world.snapshot().tick;
    let review = world
        .review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::EcmSystem,
            },
            tick,
        )
        .unwrap();
    let prepared = world.prepare_module(review, tick).unwrap();
    let staged = World::from_save_bytes(prepared.bytes()).unwrap();
    assert!(staged.ecm_snapshot().unwrap().fitted_item.is_some());
    assert_eq!(
        prepared.publish_with(|_| Err::<(), _>("writer refused")),
        Err("writer refused")
    );
    assert_eq!(world.save_bytes().unwrap(), before);
}
#[test]
fn fresh_commander_has_no_free_hardware_or_affordable_purchase() {
    let world = World::new_commander(12005).unwrap();
    assert!(world.ecm_snapshot().unwrap().fitted_item.is_none());
    assert!(
        !world
            .module_offers()
            .into_iter()
            .find(|o| o.kind == ModuleKind::EcmSystem)
            .unwrap()
            .available
    );
    assert!(
        world
            .review_module(
                ModuleAction::Purchase {
                    kind: ModuleKind::EcmSystem
                },
                0
            )
            .is_err()
    );
}
#[test]
fn real_owned_missile_is_terminated_once_without_impact_reward_or_round_refund() {
    let mut world = ready();
    launch_missile(&mut world);
    step(&mut world, 1);
    let before = missile(&world, 0);
    assert!(before.ended.is_none() && before.age_ticks > 0);
    let cash = world.credits_tenths();
    let cargo = world.service_snapshot();
    let combat = world.combat;
    let rounds = world.missile_snapshot().unwrap().rounds;
    let resources = world.flight_resources_snapshot().unwrap();
    let origin = world.request_ecm(world.snapshot().tick).unwrap();
    let ended = missile(&world, 0);
    assert_eq!(ended.ended, Some(MissileEnd::Ecm));
    assert_eq!(ended.ecm.unwrap().origin, origin);
    assert_eq!(ended.origin, before.origin);
    assert_eq!(
        (ended.position, ended.age_ticks, ended.propulsion_travel),
        (before.position, before.age_ticks, before.propulsion_travel)
    );
    assert!(ended.contact.is_none() && ended.intercepting_emission.is_none());
    let after = world.flight_resources_snapshot().unwrap();
    assert_eq!(after.energy_hundredths, resources.energy_hundredths - 2200);
    assert_eq!(
        (after.heat_fifths, after.cooldown_ticks),
        (resources.heat_fifths, resources.cooldown_ticks)
    );
    assert_eq!(world.combat, combat);
    assert_eq!(world.missile_snapshot().unwrap().rounds, rounds);
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(world.service_snapshot(), cargo);
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 0);
    assert_eq!(world.ecm_snapshot().unwrap().terminated_missiles, 1);
    cold(&world);
    step(&mut world, 40);
    assert_eq!(missile(&world, 0), ended);
    assert_eq!(world.practice_snapshot().unwrap().first_hits, 0);
    cold(&world);
}
#[test]
fn active_field_catches_a_real_newborn_and_freezes_during_pause_and_continue() {
    let mut world = ready();
    let field = world.request_ecm(world.snapshot().tick).unwrap();
    launch_missile(&mut world);
    assert_eq!(missile(&world, 0).ecm.unwrap().origin, field);
    assert_eq!(missile(&world, 0).age_ticks, 0);
    assert_eq!(world.ecm_snapshot().unwrap().remaining_ticks, 32);
    let tick = world.snapshot().tick;
    refused(&mut world, tick, EcmError::FieldActive);
    world.request_pause(true, tick).unwrap();
    refused(&mut world, tick, EcmError::Paused);
    let mut loaded = cold(&world);
    for _ in 0..40 {
        assert_eq!(
            world.step(crate::FlightInput::default()),
            Err(crate::StepError::Paused)
        );
        assert_eq!(
            loaded.step(crate::FlightInput::default()),
            Err(crate::StepError::Paused)
        );
    }
    same(&world, &loaded);
    world.request_pause(false, tick).unwrap();
    loaded.request_pause(false, tick).unwrap();
    for remaining in (0..32).rev() {
        step(&mut world, 1);
        step(&mut loaded, 1);
        same(&world, &loaded);
        assert_eq!(world.ecm_snapshot().unwrap().remaining_ticks, remaining);
    }
    assert!(world.ecm_snapshot().unwrap().field.is_none());
    cold(&world);
    // The actual missile cooldown is independent of the shorter ECM field.
    step(&mut world, 28);
    launch_missile(&mut world);
    assert!(missile(&world, 1).ended.is_none());
    let second = world.request_ecm(world.snapshot().tick).unwrap();
    assert_eq!(second.activation, 2);
    assert_eq!(missile(&world, 1).ecm.unwrap().origin, second);
    cold(&world);
}
#[test]
fn dock_stale_and_missing_hardware_refuse_without_any_saved_delta() {
    let mut world = paid();
    let tick = world.snapshot().tick;
    refused(&mut world, tick, EcmError::NotInFlight);
    refused(&mut world, tick - 1, EcmError::StaleCommand);
    let item = world.ecm_snapshot().unwrap().fitted_item.unwrap();
    apply(&mut world, ModuleAction::Store { item });
    launch(&mut world);
    let tick = world.snapshot().tick;
    refused(&mut world, tick, EcmError::NoFittedHardware);
    assert_eq!(World::new().request_ecm(0), Err(EcmError::NoCommander));
}
#[test]
fn energy_must_exceed_22_and_discharge_keeps_laser_heat_and_cooldown() {
    // Narrow resource boundary; these are not claimed as positive flight histories.
    for energy in [0_u16, 2199, 2200, 2201, 38000] {
        let mut writer = Writer::new();
        writer.u16(energy);
        writer.u16(99);
        writer.u8(7);
        let resources = PulseResources::decode_save(&mut Reader::new(&writer.bytes)).unwrap();
        let result = resources.prepare_ecm_discharge();
        if energy <= ECM_ENERGY_COST_HUNDREDTHS {
            assert!(result.is_err());
        } else {
            let mut encoded = Writer::new();
            result.unwrap().encode_save(&mut encoded);
            let mut reader = Reader::new(&encoded.bytes);
            assert_eq!(
                (
                    reader.u16().unwrap(),
                    reader.u16().unwrap(),
                    reader.u8().unwrap()
                ),
                (energy - 2200, 99, 7)
            );
        }
    }
}
#[test]
fn retained_field_codec_and_exact_replay_reject_invented_ownership_and_timing() {
    let mut world = ready();
    launch_missile(&mut world);
    world.request_ecm(world.snapshot().tick).unwrap();
    let mut writer = Writer::new();
    world.ecm.encode(&mut writer);
    let mut reader = Reader::new(&writer.bytes);
    assert_eq!(State::decode(&mut reader).unwrap(), world.ecm);
    assert!(reader.finished());
    corrupt(&world, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0].origin.owner.owner += 1
    });
    corrupt(&world, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0].origin.owner.hull += 1
    });
    corrupt(&world, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0].origin.owner.visit += 1
    });
    corrupt(&world, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0].origin.activation += 1
    });
    corrupt(&world, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0].item += 1
    });
    corrupt(&world, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0].world_tick += 1
    });
    corrupt(&world, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0].physical_tick += 1
    });
    corrupt(&world, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0].after_emission += 1
    });
    corrupt(&world, |w| w.ecm = State::default());
}
#[test]
fn missile_effect_proof_cannot_be_erased_retargeted_or_turned_into_a_hit() {
    let mut world = ready();
    launch_missile(&mut world);
    world.request_ecm(world.snapshot().tick).unwrap();
    corrupt(&world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().ecm = None
    });
    corrupt(&world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().ended = None
    });
    corrupt(&world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().ended = Some(MissileEnd::Impact)
    });
    corrupt(&world, |w| {
        w.missiles.projectiles[0]
            .as_mut()
            .unwrap()
            .ecm
            .as_mut()
            .unwrap()
            .after_emission += 1
    });
    corrupt(&world, |w| {
        w.missiles.projectiles[0]
            .as_mut()
            .unwrap()
            .ecm
            .as_mut()
            .unwrap()
            .origin
            .activation += 1
    });
    corrupt(&world, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().position.x += 1.0
    });
    corrupt(&world, |w| w.pulse = PulseResources::default());
}
#[test]
fn quiet_field_replay_does_not_need_an_invented_incoming_missile() {
    let mut world = ready();
    assert!(
        world
            .missile_snapshot()
            .unwrap()
            .projectiles
            .iter()
            .all(Option::is_none)
    );
    world.request_ecm(world.snapshot().tick).unwrap();
    assert_eq!(world.ecm_snapshot().unwrap().terminated_missiles, 0);
    let mut loaded = cold(&world);
    for _ in 0..ECM_FIELD_TICKS + 10 {
        step(&mut world, 1);
        step(&mut loaded, 1);
        same(&world, &loaded);
    }
    assert_eq!(world.ecm_snapshot().unwrap().activations, 1);
    cold(&world);
}

#[test]
fn repeated_real_manual_fields_eventually_refuse_low_energy_without_a_free_recharge() {
    let mut world = ready();
    for _ in 0..30 {
        if world.flight_resources_snapshot().unwrap().energy_hundredths <= 2200 {
            let tick = world.snapshot().tick;
            refused(&mut world, tick, EcmError::InsufficientEnergy);
            cold(&world);
            return;
        }
        world.request_ecm(world.snapshot().tick).unwrap();
        step(&mut world, 32);
    }
    panic!("genuine repeated ECM spends did not exhaust the electrical pool");
}
#[test]
fn failed_whole_save_validation_rolls_back_field_and_missile_effects_together() {
    let mut world = ready();
    launch_missile(&mut world);
    // Deliberately invalid resource history tests rollback, never a positive admitted state.
    world.pulse.damage_energy(1);
    let before = world.unchecked_test_save().unwrap();
    assert!(matches!(
        world.request_ecm(world.snapshot().tick),
        Err(EcmError::Save(_))
    ));
    assert_eq!(world.unchecked_test_save().unwrap(), before);
    assert!(world.ecm.activations.is_empty());
    assert!(missile(&world, 0).ended.is_none());
}

#[test]
fn both_same_tick_laser_command_orders_replay_the_exact_shared_spend() {
    for ecm_first in [false, true] {
        let mut world = ready();
        let tick = world.snapshot().tick;
        if ecm_first {
            world.request_ecm(tick).unwrap();
            world.request_fire_front_pulse(tick).unwrap();
        } else {
            world.request_fire_front_pulse(tick).unwrap();
            world.request_ecm(tick).unwrap();
        }
        assert_eq!(
            world.ecm.activations[0].after_emission,
            u64::from(!ecm_first)
        );
        assert_eq!(world.combat.emissions, 1);
        cold(&world);
        step(&mut world, 32);
        cold(&world);
    }
}

#[test]
fn cold_timer_never_authorizes_new_missile_destruction_or_a_free_reactivation() {
    let mut world = ready();
    world.request_ecm(world.snapshot().tick).unwrap();
    assert!(world.ecm_snapshot().unwrap().field.is_some());
    let mut loaded = cold(&world);
    let snapshot = loaded.ecm_snapshot().unwrap();
    assert_eq!(
        (
            snapshot.remaining_ticks,
            snapshot.activations,
            snapshot.terminated_missiles
        ),
        (32, 1, 0)
    );
    let tick = loaded.snapshot().tick;
    refused(&mut loaded, tick, EcmError::FieldActive);
    launch_missile(&mut loaded);
    assert!(missile(&loaded, 0).ended.is_none());
    assert!(missile(&loaded, 0).ecm.is_none());
    assert_eq!(loaded.combat.emissions, 1);
    cold(&loaded);
    corrupt(&loaded, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0].closed = None
    });
    corrupt(&loaded, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0]
            .closed
            .as_mut()
            .unwrap()
            .after_emission += 1
    });
    corrupt(&loaded, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0]
            .closed
            .as_mut()
            .unwrap()
            .world_tick += 1
    });
    corrupt(&loaded, |w| {
        Arc::make_mut(&mut w.ecm.activations)[0]
            .closed
            .as_mut()
            .unwrap()
            .physical_tick += 1
    });
    step(&mut loaded, 31);
    let tick = loaded.snapshot().tick;
    refused(&mut loaded, tick, EcmError::FieldActive);
    step(&mut loaded, 1);
    assert_eq!(loaded.ecm_snapshot().unwrap().remaining_ticks, 0);
    let energy = loaded
        .flight_resources_snapshot()
        .unwrap()
        .energy_hundredths;
    let fresh = loaded.request_ecm(loaded.snapshot().tick).unwrap();
    assert_eq!(fresh.activation, 2);
    assert_eq!(missile(&loaded, 0).ecm.unwrap().origin, fresh);
    assert_eq!(
        loaded
            .flight_resources_snapshot()
            .unwrap()
            .energy_hundredths,
        energy - 2200
    );
    assert_eq!(loaded.combat.emissions, 1);
    cold(&loaded);
}
#[test]
fn all_actual_utility_items_share_the_original_three_mounts() {
    let mut world = paid();
    let item = world.ecm_snapshot().unwrap().fitted_item.unwrap();
    world
        .request_equipment(
            crate::equipment::Command::PurchaseCargoBay,
            world.snapshot().tick,
        )
        .unwrap();
    apply(&mut world, ModuleAction::PurchaseFuelScoops);
    assert_eq!(
        world
            .modular_equipment_snapshot()
            .unwrap()
            .utility_mounts_used,
        3
    );
    assert_eq!(world.fuel_scoops_snapshot().unwrap().utility_mounts_used, 3);
    let cash = world.credits_tenths();
    apply(&mut world, ModuleAction::Store { item });
    assert_eq!(
        world
            .modular_equipment_snapshot()
            .unwrap()
            .utility_mounts_used,
        2
    );
    let before = world.save_bytes().unwrap();
    assert!(
        world
            .review_module(
                ModuleAction::Purchase {
                    kind: ModuleKind::EcmSystem
                },
                world.snapshot().tick
            )
            .is_err()
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    apply(
        &mut world,
        ModuleAction::Install {
            item,
            replacing: None,
        },
    );
    assert_eq!(
        world
            .modular_equipment_snapshot()
            .unwrap()
            .utility_mounts_used,
        3
    );
    assert_eq!(world.credits_tenths(), cash);
    cold(&world);
}

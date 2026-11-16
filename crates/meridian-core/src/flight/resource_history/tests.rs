#![allow(clippy::unwrap_used)]
use super::*;
use crate::docking::Vec3;
use crate::flight::missiles::tests::{dock, fire, stop};
use crate::flight::practice::tests::{aim_at, consent, launch, step};
use crate::{FireError, ModuleAction, ModuleKind, PulseContact, XyzzyAction};
fn module(w: &mut World, action: ModuleAction) -> u64 {
    let t = w.snapshot().tick;
    let r = w.review_module(action, t).unwrap();
    w.prepare_module(r, t).unwrap().commit()
}
fn beam(seed: u64) -> (World, u64) {
    let mut w = crate::flight::equipment::tests::earned_world(seed);
    let item = module(
        &mut w,
        ModuleAction::Purchase {
            kind: ModuleKind::BeamLaser,
        },
    );
    assert_eq!(w.credits_tenths(), Some(38_349));
    (w, item)
}
fn shot(w: &mut World) -> crate::PulseShot {
    w.request_fire_front_laser(w.snapshot().tick).unwrap()
}
fn cold(w: &World) -> World {
    let b = w.save_bytes().unwrap();
    assert_eq!(u16::from_le_bytes([b[8], b[9]]), 17);
    let l = World::from_save_bytes(&b).unwrap();
    assert_eq!(l.flight_resources_snapshot(), w.flight_resources_snapshot());
    assert_eq!(l.front_laser_snapshot(), w.front_laser_snapshot());
    assert_eq!(l.combat_snapshot(), w.combat_snapshot());
    assert_eq!(l.resource_history, w.resource_history);
    assert_eq!(l.credits_tenths(), w.credits_tenths());
    l
}
fn secret(w: &mut World) {
    let t = w.snapshot().tick;
    let r = w.review_xyzzy(XyzzyAction::Enter, t).unwrap();
    w.prepare_xyzzy(r, t).unwrap().commit();
}
fn pool(w: &World) -> (u16, u16, u8) {
    let s = w.flight_resources_snapshot().unwrap();
    (s.energy_hundredths, s.heat_fifths, s.cooldown_ticks)
}
fn refuses(w: &World, live: &mut World, before: &[u8]) {
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
}
fn forged_pool(energy: u16, heat: u16, cooldown: u8) -> super::super::weapons::PulseResources {
    let mut w = Writer::new();
    w.u16(energy);
    w.u16(heat);
    w.u8(cooldown);
    super::super::weapons::PulseResources::decode_save(&mut Reader::new(&w.bytes)).unwrap()
}
#[test]
fn genuine_paid_beam_replays_six_tick_deficits_instead_of_assuming_full_at_ready() {
    let (mut w, item) = beam(16101);
    let s = w.front_laser_snapshot().unwrap();
    assert_eq!((s.damage_hundredths, s.cooldown_ticks_per_shot), (1500, 6));
    assert_eq!(s.fitted_item_id, item);
    launch(&mut w);
    assert_eq!(shot(&mut w).fitted_item_id, item);
    assert_eq!(pool(&w), (37900, 50, 6));
    cold(&w);
    step(&mut w, 5);
    assert_eq!(pool(&w), (37935, 45, 1));
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.request_fire_front_laser(w.snapshot().tick),
        Err(FireError::CoolingDown)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    step(&mut w, 1);
    assert_eq!(pool(&w), (37942, 44, 0));
    cold(&w);
    shot(&mut w);
    assert_eq!(pool(&w), (37842, 94, 6));
    cold(&w);
    step(&mut w, 6);
    assert_eq!(pool(&w), (37884, 88, 0));
    shot(&mut w);
    assert_eq!(pool(&w), (37784, 138, 6));
    step(&mut w, 6);
    assert_eq!(pool(&w), (37826, 132, 0));
    cold(&w);
    assert_eq!(w.credits_tenths(), Some(38349));
}
#[test]
fn actual_fast_heat_gate_refuses_after_twenty_eight_shots_and_exact_cooling_restores_admission() {
    let (mut w, _) = beam(16201);
    launch(&mut w);
    for i in 0..28 {
        if i > 0 {
            step(&mut w, 6);
        }
        shot(&mut w);
    }
    assert_eq!(pool(&w), (36334, 1238, 6));
    step(&mut w, 6);
    assert_eq!(pool(&w), (36376, 1232, 0));
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.request_fire_front_laser(w.snapshot().tick),
        Err(FireError::Overheated)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    cold(&w);
    step(&mut w, 33);
    assert_eq!(pool(&w), (36607, 1199, 0));
    shot(&mut w);
    assert_eq!(pool(&w), (36507, 1249, 6));
    cold(&w);
}
#[test]
fn full_energy_or_wrong_cooldown_cannot_replace_an_actual_multi_shot_deficit() {
    let (mut w, _) = beam(16301);
    launch(&mut w);
    shot(&mut w);
    step(&mut w, 6);
    shot(&mut w);
    step(&mut w, 6);
    let valid = w.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let before = live.save_bytes().unwrap();
    w.pulse = forged_pool(38000, 88, 0);
    refuses(&w, &mut live, &before);
    w = World::from_save_bytes(&valid).unwrap();
    w.pulse = forged_pool(37884, 88, 6);
    refuses(&w, &mut live, &before);
}
#[test]
fn deleted_duplicated_wrong_class_and_physical_tick_events_refuse_coherently() {
    let (mut w, _) = beam(16401);
    launch(&mut w);
    shot(&mut w);
    step(&mut w, 6);
    shot(&mut w);
    let valid = w.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let before = live.save_bytes().unwrap();
    w.resource_history.as_mut().unwrap().events.pop();
    refuses(&w, &mut live, &before);
    w = World::from_save_bytes(&valid).unwrap();
    let last = *w.resource_history.as_ref().unwrap().events.last().unwrap();
    w.resource_history.as_mut().unwrap().events.push(last);
    refuses(&w, &mut live, &before);
    for physical in [false, true] {
        w = World::from_save_bytes(&valid).unwrap();
        if let Some(Event::Shot(s)) = w.resource_history.as_mut().unwrap().events.last_mut() {
            if physical {
                s.physical += 1;
            } else {
                s.weapon = OrdinaryWeapon::FrontPulseLaser;
            }
        }
        refuses(&w, &mut live, &before);
    }
}
#[test]
fn actual_beam_station_receipt_and_later_born_refit_keep_typed_custody_and_resource_history() {
    let (mut w, item) = beam(16501);
    launch(&mut w);
    stop(&mut w);
    aim_at(&mut w, Vec3::new(70.0, 0.0, 136.32));
    let s = shot(&mut w);
    assert!(matches!(s.contact, PulseContact::StationBlocked { .. }));
    let hit = w.combat_snapshot().unwrap().last_ordinary_impact.unwrap();
    assert_eq!(
        (hit.item, hit.weapon),
        (item, OrdinaryWeapon::FrontBeamLaser)
    );
    assert_eq!(w.legal_value(), Some(64));
    cold(&w);
    step(&mut w, 6);
    shot(&mut w);
    step(&mut w, 180);
    dock(&mut w);
    module(
        &mut w,
        ModuleAction::Install {
            item: 16508,
            replacing: Some(item),
        },
    );
    assert_eq!(
        w.front_laser_snapshot().unwrap().weapon,
        OrdinaryWeapon::FrontPulseLaser
    );
    cold(&w);
    launch(&mut w);
    shot(&mut w);
    assert_eq!(pool(&w).2, 18);
    cold(&w);
}
#[test]
fn explicit_xyzzy_naval_front_has_real_sixty_four_damage_six_tick_effect_and_no_ordinary_offer() {
    let mut w = World::new_commander(16601).unwrap();
    secret(&mut w);
    let s = w.front_laser_snapshot().unwrap();
    assert_eq!(
        (s.weapon, s.damage_hundredths, s.cooldown_ticks_per_shot),
        (OrdinaryWeapon::FrontNavalLaser, 6400, 6)
    );
    assert_eq!(
        w.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::NavalLaser
            },
            w.snapshot().tick
        ),
        Err(crate::ModuleError::NotForSale)
    );
    assert_eq!(
        w.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::MilitaryLaser
            },
            w.snapshot().tick
        ),
        Err(crate::ModuleError::TechnologyUnavailable)
    );
    let elite = w
        .modular_equipment_snapshot()
        .unwrap()
        .items
        .iter()
        .find(|i| i.kind == ModuleKind::EliteEnergyUnit)
        .unwrap()
        .id;
    module(&mut w, ModuleAction::Store { item: elite });
    assert_eq!(
        w.flight_resources_snapshot()
            .unwrap()
            .energy_recharge_hundredths_per_tick,
        7
    );
    launch(&mut w);
    shot(&mut w);
    step(&mut w, 6);
    shot(&mut w);
    assert_eq!(pool(&w), (37842, 94, 6));
    cold(&w);
}
#[test]
fn actual_energy_fitting_across_docks_replays_historical_basic_and_extra_rates_without_refill_alias()
 {
    let mut w = World::new_commander(16701).unwrap();
    secret(&mut w);
    let elite = w
        .modular_equipment_snapshot()
        .unwrap()
        .items
        .iter()
        .find(|i| i.kind == ModuleKind::EliteEnergyUnit)
        .unwrap()
        .id;
    module(&mut w, ModuleAction::Store { item: elite });
    let item = module(
        &mut w,
        ModuleAction::Purchase {
            kind: ModuleKind::BeamLaser,
        },
    );
    launch(&mut w);
    shot(&mut w);
    step(&mut w, 6);
    shot(&mut w);
    step(&mut w, 180);
    dock(&mut w);
    module(
        &mut w,
        ModuleAction::Purchase {
            kind: ModuleKind::ExtraEnergyUnit,
        },
    );
    launch(&mut w);
    assert_eq!(shot(&mut w).fitted_item_id, item);
    step(&mut w, 6);
    assert_eq!(pool(&w), (37984, 44, 0));
    shot(&mut w);
    assert_eq!(pool(&w), (37884, 94, 6));
    cold(&w);
}
#[test]
fn scalar_energy_boundary_is_only_a_discharge_guard_and_never_a_saved_resource_proof() {
    // Pure resource unit fixture. No public World, inventory grant or replay acceptance is claimed.
    for weapon in [
        OrdinaryWeapon::FrontBeamLaser,
        OrdinaryWeapon::FrontMilitaryLaser,
        OrdinaryWeapon::FrontNavalLaser,
    ] {
        assert_eq!(
            forged_pool(100, 0, 0).prepare_laser_discharge(weapon),
            Err(FireError::InsufficientEnergy)
        );
        assert!(
            forged_pool(101, 0, 0)
                .prepare_laser_discharge(weapon)
                .is_ok()
        );
        assert_eq!(
            forged_pool(38000, 1200, 0).prepare_laser_discharge(weapon),
            Err(FireError::Overheated)
        );
    }
}
#[test]
fn actual_beam_gunnery_receipt_uses_its_real_cadence_and_quiet_reception_after_cold_continue() {
    let mut w = World::new_commander(16801).unwrap();
    secret(&mut w);
    let _ = module(
        &mut w,
        ModuleAction::Purchase {
            kind: ModuleKind::BeamLaser,
        },
    );
    consent(&mut w);
    launch(&mut w);
    assert!(matches!(
        shot(&mut w).contact,
        PulseContact::PracticeBeacon {
            first_hit: true,
            ..
        }
    ));
    step(&mut w, 6);
    cold(&w);
    stop(&mut w);
    let beacons = w.practice_snapshot().unwrap().beacons.unwrap();
    for index in [0, 2] {
        aim_at(&mut w, beacons[index].pose.position);
        shot(&mut w);
        step(&mut w, 6);
        cold(&w);
    }
    assert_eq!(w.practice_snapshot().unwrap().first_hits, 3);
    step(&mut w, 180);
    assert!(w.practice_reception_snapshot().unwrap().ready);
    cold(&w);
}
#[test]
fn full_history_save_cap_refuses_a_real_next_discharge_with_complete_rollback() {
    let (mut w, _) = beam(16901);
    launch(&mut w);
    // Real public discharges and cooling, no copied or fabricated history prefix.
    for count in 0..2000 {
        let before = w.save_bytes().unwrap();
        let resources = w.flight_resources_snapshot();
        let combat = w.combat_snapshot();
        let history = w.resource_history.clone();
        match w.request_fire_front_laser(w.snapshot().tick) {
            Ok(beam) => {
                assert_eq!(beam.contact, PulseContact::Clear);
                step(&mut w, 50);
            }
            Err(FireError::SaveRefused) => {
                // A Clear shot adds exactly one45-byte record and no optional contact receipt.
                assert!(count > 500);
                assert!(before.len() + 45 > crate::save::MAX_SAVE_BYTES);
                assert_eq!(w.save_bytes().unwrap(), before);
                assert_eq!(w.flight_resources_snapshot(), resources);
                assert_eq!(w.combat_snapshot(), combat);
                assert_eq!(w.resource_history, history);
                cold(&w);
                return;
            }
            other => panic!("unexpected genuine resource admission: {other:?}"),
        }
    }
    panic!("64KiB complete-save bound did not refuse a growing history");
}

#[test]
fn actual_paid_beam_and_secret_naval_interceptions_retain_origin_and_later_fitted_custody() {
    for (seed, kind, weapon) in [
        (17001, ModuleKind::BeamLaser, OrdinaryWeapon::FrontBeamLaser),
        (
            17002,
            ModuleKind::NavalLaser,
            OrdinaryWeapon::FrontNavalLaser,
        ),
    ] {
        let mut w = World::new_commander(seed).unwrap();
        secret(&mut w);
        let item = if kind == ModuleKind::BeamLaser {
            module(&mut w, ModuleAction::Purchase { kind })
        } else {
            w.front_laser_snapshot().unwrap().fitted_item_id
        };
        consent(&mut w);
        launch(&mut w);
        stop(&mut w);
        let target = w.practice_snapshot().unwrap().beacons.unwrap()[1].id;
        let origin = fire(&mut w, target);
        let beam = shot(&mut w);
        assert_eq!(beam.fitted_item_id, item);
        assert!(matches!(beam.contact,PulseContact::Projectile {id,..} if id==origin.projectile));
        assert_eq!(w.front_laser_snapshot().unwrap().weapon, weapon);
        let valid = w.save_bytes().unwrap();
        let mut live = World::from_save_bytes(&valid).unwrap();
        let before = live.save_bytes().unwrap();
        if let Some(Event::Shot(s)) = w.resource_history.as_mut().unwrap().events.last_mut() {
            s.weapon = OrdinaryWeapon::FrontPulseLaser;
        }
        refuses(&w, &mut live, &before);
        w = World::from_save_bytes(&valid).unwrap();
        dock(&mut w);
        module(
            &mut w,
            ModuleAction::Install {
                item: seed + 7,
                replacing: Some(item),
            },
        );
        assert!(
            w.modular_equipment_snapshot()
                .unwrap()
                .items
                .iter()
                .any(|i| i.id == item && i.slot.is_none())
        );
        assert_eq!(w.front_laser_snapshot().unwrap().fitted_item_id, seed + 7);
        assert!(w.missile_projectiles().any(|p| p.origin == origin));
        cold(&w);
    }
}

#[test]
fn missing_launch_arrival_dock_and_work_counter_events_are_not_replaceable_by_pool_scalars() {
    let (mut w, _) = beam(17101);
    launch(&mut w);
    shot(&mut w);
    step(&mut w, 180);
    dock(&mut w);
    let valid = w.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let before = live.save_bytes().unwrap();
    for select in [0, 1, 2] {
        w = World::from_save_bytes(&valid).unwrap();
        let h = w.resource_history.as_mut().unwrap();
        let i = h
            .events
            .iter()
            .position(|e| {
                if select == 0 {
                    matches!(e, Event::Launch { .. })
                } else if select == 1 {
                    matches!(e, Event::Dock(_))
                } else {
                    matches!(e, Event::Arrival { .. })
                }
            })
            .unwrap();
        h.events.remove(i);
        refuses(&w, &mut live, &before);
    }
    w = World::from_save_bytes(&valid).unwrap();
    let h = w.resource_history.as_mut().unwrap();
    let s = h.shots().last().unwrap();
    h.events.push(Event::Work {
        shot: s,
        asteroid: 0,
    });
    refuses(&w, &mut live, &before);
}

#[test]
fn elapsed_replay_matches_repeated_original_fixed_tick_advance_for_pool_and_shields() {
    // Pure arithmetic/reducer fixtures, not arbitrary opening balances accepted by a World save.
    for (energy, heat, cooldown) in [(38000, 0, 0), (37900, 50, 6), (100, 1199, 18), (0, 1238, 0)] {
        for rate in [7, 14, 21, 28] {
            for n in [0, 1, 5, 6, 18, 255, 10000] {
                for docked in [false, true] {
                    if n == 0 && docked {
                        continue;
                    }
                    let mut fixed = forged_pool(energy, heat, cooldown);
                    let mut elapsed = fixed;
                    let mut fixed_shields = super::super::combat::State::default();
                    let mut elapsed_shields = fixed_shields;
                    for i in 0..n {
                        let next = if docked && i == n - 1 {
                            FlightMode::Docked
                        } else {
                            FlightMode::Flight
                        };
                        fixed.advance(FlightMode::Flight, next, rate);
                        fixed_shields.advance(FlightMode::Flight, next);
                    }
                    elapsed.replay_advance(n, rate, docked);
                    elapsed_shields.replay_advance(n, docked);
                    assert_eq!(elapsed, fixed);
                    assert_eq!(
                        elapsed_shields.shield_resources(),
                        fixed_shields.shield_resources()
                    );
                }
            }
        }
    }
}

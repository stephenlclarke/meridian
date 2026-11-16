#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::equipment::tests::journey;
use crate::flight::missiles::tests::{dock, fire, stop};
use crate::flight::practice::tests::{consent, launch};
use crate::navigation::SystemId;
use crate::{AmmunitionAction, ModuleAction, ModuleKind, ModuleProvenance, ModuleSlot};

fn apply(w: &mut World, action: XyzzyAction) {
    let tick = w.snapshot().tick;
    let review = w.review_xyzzy(action, tick).unwrap();
    let prepared = w.prepare_xyzzy(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    let expected = World::from_save_bytes(&bytes).unwrap();
    let snapshot = prepared.snapshot();
    prepared.commit();
    assert_eq!(w.xyzzy_snapshot(), snapshot);
    assert_eq!(w.xyzzy_snapshot(), expected.xyzzy_snapshot());
    assert_eq!(w.save_bytes().unwrap(), bytes);
}
fn cold(w: &World) -> World {
    let bytes = w.save_bytes().unwrap();
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 17);
    let loaded = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(loaded.xyzzy_snapshot(), w.xyzzy_snapshot());
    assert_eq!(loaded.snapshot(), w.snapshot());
    assert_eq!(
        loaded.modular_equipment_snapshot(),
        w.modular_equipment_snapshot()
    );
    assert_eq!(loaded.ammunition_snapshot(), w.ammunition_snapshot());
    assert_eq!(loaded.equipment_snapshot(), w.equipment_snapshot());
    assert_eq!(loaded.combat_snapshot(), w.combat_snapshot());
    loaded
}
fn module(w: &mut World, action: ModuleAction) -> u64 {
    let tick = w.snapshot().tick;
    let review = w.review_module(action, tick).unwrap();
    w.prepare_module(review, tick).unwrap().commit()
}
fn poison_refuses(w: &World, live: &mut World, before: &[u8]) {
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
}

#[test]
fn explicit_activation_sets_clarke_once_and_grants_only_actual_supported_outfit_with_secret_provenance()
 {
    let mut w = World::new_commander(14101).unwrap();
    apply(&mut w, XyzzyAction::Enter);
    let s = w.xyzzy_snapshot().unwrap();
    assert!(s.active && s.unlimited_fuel && s.cash_granted_once);
    assert!(!s.full_legacy_outfit_available);
    assert_eq!((s.commander_name, s.cash_tenths), ("CLARKE", 10_000_000));
    assert!(s.label.contains("XYZZY"));
    assert_eq!(
        w.flight_resources_snapshot()
            .unwrap()
            .energy_recharge_hundredths_per_tick,
        28
    );
    let modules = w.modular_equipment_snapshot().unwrap();
    let elite = modules
        .items
        .iter()
        .find(|i| i.kind == ModuleKind::EliteEnergyUnit)
        .unwrap();
    assert_eq!(elite.slot, Some(ModuleSlot::Energy));
    assert!(matches!(
        elite.provenance,
        ModuleProvenance::XYZzy { grant: 1, .. }
    ));
    assert!(!modules.items.iter().any(|i| i.kind == ModuleKind::ExtraEnergyUnit || i.kind == ModuleKind::NavalEnergyUnit));
    let naval = w.front_laser_snapshot().unwrap();
    assert_eq!(naval.weapon, crate::OrdinaryWeapon::FrontNavalLaser);
    assert_eq!(
        (naval.damage_hundredths, naval.cooldown_ticks_per_shot),
        (6400, 6)
    );
    assert!(modules.items.iter().any(|i| i.id == 14108
        && i.slot.is_none()
        && i.provenance == ModuleProvenance::StandardBirth));
    let ammo = w.ammunition_snapshot().unwrap();
    assert_eq!(ammo.items.iter().filter(|i| i.pylon.is_some()).count(), 9);
    assert_eq!(
        ammo.items.iter().filter(|i| i.xyzzy && !i.consumed).count(),
        6
    );
    assert!(ammo.purchases.is_empty());
    let bay = w.equipment_snapshot().unwrap().owned.unwrap();
    assert!(bay.xyzzy && bay.fitted);
    assert_eq!(bay.paid_tenths, 0);
    assert_eq!(w.equipment_snapshot().unwrap().hold_capacity_tonnes, 127);
    cold(&w);
}

#[test]
fn normal_readouts_and_normal_continue_never_activate_grant_or_rename() {
    let w = World::new_commander(14201).unwrap();
    let before = w.save_bytes().unwrap();
    for _ in 0..3 {
        assert_eq!(w.commander_name(), Some("Rowan Calder"));
        assert!(!w.xyzzy_snapshot().unwrap().active);
        assert!(!w.xyzzy_snapshot().unwrap().unlimited_fuel);
        assert_eq!(w.ammunition_snapshot().unwrap().items.len(), 3);
        assert_eq!(w.modular_equipment_snapshot().unwrap().items.len(), 2);
        assert!(w.equipment_snapshot().unwrap().owned.is_none());
    }
    assert_eq!(w.save_bytes().unwrap(), before);
    let w = cold(&w);
    assert_eq!(w.credits_tenths(), Some(7200));
    assert_eq!(
        w.review_xyzzy(XyzzyAction::MaximumSupportedHardware, 0),
        Err(XyzzyError::Inactive)
    );
}

#[test]
fn secret_writer_failure_discards_cash_name_fuel_and_all_grants_together() {
    let mut w = World::new_commander(14301).unwrap();
    let before = w.save_bytes().unwrap();
    let r = w.review_xyzzy(XyzzyAction::Enter, 0).unwrap();
    let p = w.prepare_xyzzy(r, 0).unwrap();
    assert_eq!(
        World::from_save_bytes(p.bytes()).unwrap().credits_tenths(),
        Some(10_000_000)
    );
    assert_eq!(
        p.publish_with(|_| Err::<(), _>("atomic writer refused")),
        Err("atomic writer refused")
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    assert_eq!(w.commander_name(), Some("Rowan Calder"));
}

#[test]
fn paid_purchase_after_secret_still_debits_and_reopening_or_maximum_never_repeats_cash_or_items() {
    let mut w = World::new_commander(14401).unwrap();
    apply(&mut w, XyzzyAction::Enter);
    let paid = module(
        &mut w,
        ModuleAction::Purchase {
            kind: ModuleKind::PulseLaser,
        },
    );
    assert_eq!(w.credits_tenths(), Some(9_985_500));
    assert!(matches!(
        w.modular_equipment_snapshot()
            .unwrap()
            .items
            .iter()
            .find(|i| i.id == paid)
            .unwrap()
            .provenance,
        ModuleProvenance::Paid {
            paid_tenths: 14500,
            transaction: 1,
            ..
        }
    ));
    let modules = w.modular_equipment_snapshot().unwrap().items;
    let ammo = w.ammunition_snapshot().unwrap().items;
    let bay = w.equipment_snapshot().unwrap().owned;
    for action in [
        XyzzyAction::Enter,
        XyzzyAction::MaximumSupportedHardware,
        XyzzyAction::RestoreStarter,
    ] {
        apply(&mut w, action);
        assert_eq!(w.credits_tenths(), Some(9_985_500));
        let mut expected = modules.clone();
        for item in &mut expected {
            if item.kind == ModuleKind::PulseLaser {
                item.slot = None;
            }
            if item.kind == ModuleKind::NavalLaser {
                item.slot = Some(ModuleSlot::FrontLaser);
            }
        }
        assert_eq!(w.modular_equipment_snapshot().unwrap().items, expected);
        assert_eq!(w.ammunition_snapshot().unwrap().items, ammo);
        assert_eq!(w.equipment_snapshot().unwrap().owned, bay);
        assert_eq!(
            w.front_laser_snapshot().unwrap().weapon,
            crate::OrdinaryWeapon::FrontNavalLaser
        );
        assert!(
            w.modular_equipment_snapshot()
                .unwrap()
                .items
                .iter()
                .any(|i| i.id == paid && i.slot.is_none())
        );
        cold(&w);
    }
}

#[test]
fn activation_after_real_earned_paid_items_preserves_payment_history_identity_cargo_and_missions() {
    let mut w = crate::flight::equipment::tests::earned_world(14501);
    w.request_equipment(
        crate::equipment::Command::PurchaseCargoBay,
        w.snapshot().tick,
    )
    .unwrap();
    let paid = module(
        &mut w,
        ModuleAction::Purchase {
            kind: ModuleKind::PulseLaser,
        },
    );
    assert_eq!(w.credits_tenths(), Some(42_849));
    let bay = w.equipment_snapshot().unwrap().owned;
    let paid_item = *w
        .modular_equipment_snapshot()
        .unwrap()
        .items
        .iter()
        .find(|i| i.id == paid)
        .unwrap();
    let mission = w.mission_snapshot().unwrap().freight;
    let commodities = w.service_snapshot();
    apply(&mut w, XyzzyAction::Enter);
    assert_eq!(w.credits_tenths(), Some(10_000_000));
    assert_eq!(w.equipment_snapshot().unwrap().owned, bay);
    assert!(!bay.unwrap().xyzzy);
    assert_eq!(
        *w.modular_equipment_snapshot()
            .unwrap()
            .items
            .iter()
            .find(|i| i.id == paid)
            .unwrap(),
        crate::OwnedModule {
            slot: None,
            ..paid_item
        }
    );
    assert_eq!(w.mission_snapshot().unwrap().freight, mission);
    // Readout balances change by the explicit cash/fuel grant; commodity custody remains empty.
    assert_eq!(
        w.service_snapshot().unwrap().used_tonnes,
        commodities.unwrap().used_tonnes
    );
    assert_eq!(w.xyzzy.records[0].cash_before_reset, Some(42_849));
    cold(&w);
}

#[test]
fn actual_unlimited_fuel_travel_toggle_and_refill_reconcile_normal_navigation_debits_without_paid_fuel()
 {
    let mut w = World::new_commander(14601).unwrap();
    apply(&mut w, XyzzyAction::Enter);
    journey(&mut w, SystemId::RENAS);
    assert_eq!(w.snapshot().navigation.unwrap().fuel_units, 130);
    assert_eq!(w.navigation.unwrap().fuel_spent(), 34);
    assert_eq!(w.credits_tenths(), Some(10_000_000));
    cold(&w);
    apply(&mut w, XyzzyAction::ToggleUnlimitedFuel);
    journey(&mut w, SystemId::AVENOR);
    assert_eq!(w.snapshot().navigation.unwrap().fuel_units, 96);
    assert_eq!(w.navigation.unwrap().fuel_spent(), 68);
    cold(&w);
    apply(&mut w, XyzzyAction::ToggleUnlimitedFuel);
    assert_eq!(w.snapshot().navigation.unwrap().fuel_units, 130);
    journey(&mut w, SystemId::RENAS);
    assert_eq!(w.snapshot().navigation.unwrap().fuel_units, 130);
    assert_eq!(w.navigation.unwrap().fuel_spent(), 102);
    assert_eq!(w.credits_tenths(), Some(10_000_000));
    cold(&w);
}

#[test]
fn refill_reuses_stored_born_missile_and_retains_consumed_secret_item_and_real_projectile_origin() {
    let mut w = World::new_commander(14701).unwrap();
    let r = w
        .review_ammunition(AmmunitionAction::Store { item: 14711 }, 0)
        .unwrap();
    w.prepare_ammunition(r, 0).unwrap().commit();
    apply(&mut w, XyzzyAction::Enter);
    assert_eq!(
        w.ammunition_snapshot()
            .unwrap()
            .items
            .iter()
            .filter(|i| i.xyzzy)
            .count(),
        6
    );
    consent(&mut w);
    launch(&mut w);
    stop(&mut w);
    let target = w.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let origin = fire(&mut w, target);
    assert!(
        w.ammunition_snapshot()
            .unwrap()
            .items
            .iter()
            .any(|i| i.item == origin.item && i.xyzzy && i.consumed)
    );
    assert_eq!(
        w.ammunition_snapshot()
            .unwrap()
            .items
            .iter()
            .filter(|i| i.pylon.is_some())
            .count(),
        8
    );
    cold(&w);
    dock(&mut w);
    apply(&mut w, XyzzyAction::MaximumSupportedHardware);
    let ammo = w.ammunition_snapshot().unwrap();
    assert_eq!(ammo.items.iter().filter(|i| i.pylon.is_some()).count(), 9);
    assert_eq!(ammo.items.iter().filter(|i| i.xyzzy).count(), 7);
    assert!(
        ammo.items
            .iter()
            .any(|i| i.item == origin.item && i.xyzzy && i.consumed)
    );
    assert!(ammo.purchases.is_empty());
    assert!(w.missile_projectiles().any(|p| p.origin == origin));
    assert_eq!(w.credits_tenths(), Some(10_000_000));
    cold(&w);
}

#[test]
fn coherent_activation_cash_and_fuel_journal_poison_refuses_without_replacing_live_world() {
    let mut w = World::new_commander(14801).unwrap();
    apply(&mut w, XyzzyAction::Enter);
    let valid = w.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let before = live.save_bytes().unwrap();
    w.xyzzy.records[0].cash_before_reset = Some(7201);
    poison_refuses(&w, &mut live, &before);
    w = World::from_save_bytes(&valid).unwrap();
    w.xyzzy.records[0].fuel_added = 1;
    poison_refuses(&w, &mut live, &before);
    w = World::from_save_bytes(&valid).unwrap();
    w.xyzzy.records[0].end_revision = w.xyzzy.records[0].revision;
    poison_refuses(&w, &mut live, &before);
    w = World::from_save_bytes(&valid).unwrap();
    w.xyzzy = State::default();
    poison_refuses(&w, &mut live, &before);
}

#[test]
fn second_cash_reset_record_and_foreign_activation_berth_cannot_be_imported() {
    let mut w = World::new_commander(14901).unwrap();
    apply(&mut w, XyzzyAction::Enter);
    apply(&mut w, XyzzyAction::Enter);
    let valid = w.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let before = live.save_bytes().unwrap();
    w.xyzzy.records[1].cash_before_reset = Some(10_000_000);
    poison_refuses(&w, &mut live, &before);
    w = World::from_save_bytes(&valid).unwrap();
    w.xyzzy.records[0].dock = World::new_commander(14902)
        .unwrap()
        .current_dock_context()
        .unwrap();
    poison_refuses(&w, &mut live, &before);
}

#[test]
fn secret_review_refuses_pause_flight_stale_foreign_and_restore_epoch_without_mutation() {
    let mut w = World::new_commander(15001).unwrap();
    let review = w.review_xyzzy(XyzzyAction::Enter, 0).unwrap();
    let mut other = World::new_commander(15002).unwrap();
    let before = other.save_bytes().unwrap();
    assert!(matches!(
        other.prepare_xyzzy(review, 0),
        Err(XyzzyError::StaleReview)
    ));
    assert_eq!(other.save_bytes().unwrap(), before);
    let bytes = w.save_bytes().unwrap();
    w.restore_bytes(&bytes).unwrap();
    assert!(matches!(
        w.prepare_xyzzy(review, 0),
        Err(XyzzyError::StaleReview)
    ));
    w.request_pause(true, 0).unwrap();
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.review_xyzzy(XyzzyAction::Enter, 0),
        Err(XyzzyError::Paused)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    w.request_pause(false, 0).unwrap();
    launch(&mut w);
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.review_xyzzy(XyzzyAction::Enter, w.snapshot().tick),
        Err(XyzzyError::BerthRequired)
    );
    assert_eq!(
        w.review_xyzzy(XyzzyAction::Enter, 0),
        Err(XyzzyError::StaleReview)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
}

#[test]
fn secret_identity_overflow_refuses_atomically_instead_of_aliasing_paid_or_born_items() {
    let mut w = World::new_commander(u64::MAX - 10).unwrap();
    let before = w.save_bytes().unwrap();
    let review = w.review_xyzzy(XyzzyAction::Enter, 0).unwrap();
    assert!(matches!(
        w.prepare_xyzzy(review, 0),
        Err(XyzzyError::Save(_))
    ));
    assert_eq!(w.save_bytes().unwrap(), before);
}

#[test]
fn active_earned_fuel_practice_is_preserved_and_unlimited_supply_cannot_be_earned_practice_evidence()
 {
    let mut w = World::new_commander(15101).unwrap();
    let r = w
        .review_navigation_trade(crate::NavigationTradeAction::Accept, 0)
        .unwrap();
    w.prepare_navigation_trade(r, 0).unwrap().commit();
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.review_xyzzy(XyzzyAction::Enter, 0),
        Err(XyzzyError::ActiveFuelPractice)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let mut secret = World::new_commander(15102).unwrap();
    apply(&mut secret, XyzzyAction::Enter);
    assert_eq!(
        secret.review_navigation_trade(crate::NavigationTradeAction::Accept, 0),
        Err(crate::NavigationTradeError::Fuel)
    );
    apply(&mut secret, XyzzyAction::ToggleUnlimitedFuel);
    assert!(
        secret
            .review_navigation_trade(crate::NavigationTradeAction::Accept, 0)
            .is_ok()
    );
    cold(&secret);
}

#[test]
fn real_paid_lower_energy_then_maximum_refits_same_secret_elite_without_refund_or_duplicate() {
    let mut w = World::new_commander(15201).unwrap();
    apply(&mut w, XyzzyAction::Enter);
    let elite = w
        .modular_equipment_snapshot()
        .unwrap()
        .items
        .iter()
        .find(|i| i.kind == ModuleKind::EliteEnergyUnit)
        .unwrap()
        .id;
    let paid = module(
        &mut w,
        ModuleAction::Purchase {
            kind: ModuleKind::ExtraEnergyUnit,
        },
    );
    assert_eq!(w.credits_tenths(), Some(9_928_000));
    assert_eq!(
        w.flight_resources_snapshot()
            .unwrap()
            .energy_recharge_hundredths_per_tick,
        14
    );
    apply(&mut w, XyzzyAction::MaximumSupportedHardware);
    assert_eq!(
        w.flight_resources_snapshot()
            .unwrap()
            .energy_recharge_hundredths_per_tick,
        28
    );
    let items = w.modular_equipment_snapshot().unwrap().items;
    assert_eq!(
        items
            .iter()
            .filter(|i| i.kind == ModuleKind::EliteEnergyUnit)
            .count(),
        1
    );
    assert_eq!(
        items.iter().find(|i| i.id == elite).unwrap().slot,
        Some(ModuleSlot::Energy)
    );
    assert_eq!(items.iter().find(|i| i.id == paid).unwrap().slot, None);
    assert!(matches!(
        items.iter().find(|i| i.id == paid).unwrap().provenance,
        ModuleProvenance::Paid {
            paid_tenths: 72000,
            ..
        }
    ));
    assert_eq!(w.credits_tenths(), Some(9_928_000));
    cold(&w);
}

#[test]
fn actual_eight_unit_freight_custody_survives_activation_and_still_pays_only_once_after_real_receiving()
 {
    use crate::freight::g02::{Choice, Command, Event};
    let mut w = World::new_commander(15501).unwrap();
    let source = w.station_contacts()[0];
    w.request_contact(source, 0).unwrap();
    let Event::Reviewed(review) = w
        .request_freight(
            Command::ReviewOffer {
                actor: source.actor(),
            },
            0,
        )
        .unwrap()
    else {
        panic!("actual offer missing");
    };
    let Event::Accepted { job, units } = w
        .request_freight(
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            0,
        )
        .unwrap()
    else {
        panic!("actual eight-unit custody missing");
    };
    assert_eq!(units.len(), 8);
    let held = w.mission_snapshot().unwrap().freight;
    let cargo = w.service_snapshot().unwrap().used_tonnes;
    assert_eq!(cargo, 8);
    apply(&mut w, XyzzyAction::Enter);
    assert_eq!(w.mission_snapshot().unwrap().freight, held);
    assert_eq!(w.service_snapshot().unwrap().used_tonnes, cargo);
    assert_eq!(w.active_freight_job(), Some(job));
    let mut w = cold(&w);
    journey(&mut w, SystemId::RENAS);
    let crew = w.station_contacts()[0];
    let tick = w.snapshot().tick;
    w.request_contact(crew, tick).unwrap();
    let Event::Reviewed(review) = w
        .request_freight(
            Command::ReviewReceiving {
                actor: crew.actor(),
                job,
            },
            tick,
        )
        .unwrap()
    else {
        panic!("actual receiving review missing");
    };
    let Event::Received {
        remaining,
        completion,
        ..
    } = w
        .request_freight(
            Command::Confirm {
                review,
                choice: Choice::ReceiveAll,
            },
            tick,
        )
        .unwrap()
    else {
        panic!("actual receiving missing");
    };
    assert_eq!(remaining, 0);
    assert!(completion.is_some());
    assert_eq!(w.credits_tenths(), Some(10_007_300));
    assert_eq!(w.service_snapshot().unwrap().used_tonnes, 0);
    let mut w = cold(&w);
    let before = w.save_bytes().unwrap();
    assert!(
        w.request_freight(
            Command::Confirm {
                review,
                choice: Choice::ReceiveAll
            },
            w.snapshot().tick
        )
        .is_err()
    );
    assert_eq!(w.credits_tenths(), Some(10_007_300));
    assert_eq!(w.save_bytes().unwrap(), before);
}

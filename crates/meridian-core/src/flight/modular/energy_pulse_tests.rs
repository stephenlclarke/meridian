//! Genuine public purchases/travel/fire create positive World evidence; no wallet or tool grants.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::PulseContact;
use crate::flight::missiles::tests::{dock, stop};
use crate::flight::practice::tests::{aim_at, launch, step};
use crate::navigation::SystemId;
use crate::station_services::{Command as Trade, CommodityId};

fn funded(seed: u64) -> World {
    crate::flight::equipment::tests::earned_world(seed)
}
fn apply(w: &mut World, action: ModuleAction) -> u64 {
    let tick = w.snapshot().tick;
    let review = w.review_module(action, tick).unwrap();
    let prepared = w.prepare_module(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    let staged = World::from_save_bytes(&bytes).unwrap();
    let inventory = staged.modular_equipment_snapshot();
    let resources = staged.flight_resources_snapshot();
    let item = prepared.commit();
    assert_eq!(w.modular_equipment_snapshot(), inventory);
    assert_eq!(w.flight_resources_snapshot(), resources);
    assert_eq!(w.save_bytes().unwrap(), bytes);
    item
}
fn cold(w: &World) -> World {
    let bytes = w.save_bytes().unwrap();
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 16);
    let loaded = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(
        loaded.modular_equipment_snapshot(),
        w.modular_equipment_snapshot()
    );
    assert_eq!(
        loaded.flight_resources_snapshot(),
        w.flight_resources_snapshot()
    );
    assert_eq!(loaded.combat_snapshot(), w.combat_snapshot());
    assert_eq!(loaded.credits_tenths(), w.credits_tenths());
    loaded
}
fn purchase(w: &mut World, kind: ModuleKind) -> u64 {
    apply(w, ModuleAction::Purchase { kind })
}
fn extra_round_trip(w: &mut World) {
    assert_eq!(w.credits_tenths(), Some(85_349));
    w.request_refuel(w.snapshot().tick).unwrap();
    crate::flight::equipment::tests::journey(w, SystemId::AVENOR);
    for _ in 0..10 {
        w.request_station_service(
            Trade::BuyCommodity(CommodityId::Minerals),
            w.snapshot().tick,
        )
        .unwrap();
    }
    crate::flight::equipment::tests::journey(w, SystemId::RENAS);
    for _ in 0..10 {
        w.request_station_service(
            Trade::SellCommodity(CommodityId::Minerals),
            w.snapshot().tick,
        )
        .unwrap();
    }
    // Original finite mineral ask241/bid387 and one102-unit fuel refill cost204.
    assert_eq!(w.credits_tenths(), Some(86_605));
}
#[test]
fn extra_energy_purchase_preserves_born_item_and_capacity_with_real_7200cr_debit_and_two_units() {
    let mut w = funded(11101);
    let before = w.flight_resources_snapshot().unwrap();
    let item = purchase(&mut w, ModuleKind::ExtraEnergyUnit);
    assert_eq!(w.credits_tenths(), Some(13_349));
    let s = w.modular_equipment_snapshot().unwrap();
    assert_eq!(
        s.items
            .iter()
            .filter(|i| i.kind == ModuleKind::BasicEnergySystem)
            .count(),
        1
    );
    let basic = s
        .items
        .iter()
        .find(|i| i.kind == ModuleKind::BasicEnergySystem)
        .unwrap();
    assert_eq!(
        (basic.id, basic.slot, basic.provenance),
        (11106, None, ModuleProvenance::StandardBirth)
    );
    let paid = s.items.iter().find(|i| i.id == item).unwrap();
    assert_eq!(paid.slot, Some(ModuleSlot::Energy));
    assert!(matches!(
        paid.provenance,
        ModuleProvenance::Paid {
            paid_tenths: 72_000,
            transaction: 1,
            ..
        }
    ));
    assert_eq!(
        (
            s.equipment_units_used,
            s.equipment_units_capacity,
            s.utility_mounts_used
        ),
        (2, 24, 0)
    );
    let after = w.flight_resources_snapshot().unwrap();
    assert_eq!(after.energy_hundredths, before.energy_hundredths);
    assert_eq!(after.heat_fifths, before.heat_fifths);
    assert_eq!(after.cooldown_ticks, before.cooldown_ticks);
    assert_eq!(after.energy_capacity_hundredths, 38_000);
    assert_eq!(after.energy_recharge_hundredths_per_tick, 14);
    assert_eq!(w.pulse_laser_snapshot().unwrap().fitted_item_id, 11108);
    cold(&w);
}
#[test]
fn real_pulse_deficit_recharges_at_four_point_two_vs_eight_point_four_per_second_and_cold_continues()
 {
    let mut basic = funded(11201);
    let mut extra = funded(11202);
    purchase(&mut extra, ModuleKind::ExtraEnergyUnit);
    launch(&mut basic);
    launch(&mut extra);
    basic
        .request_fire_front_pulse(basic.snapshot().tick)
        .unwrap();
    extra
        .request_fire_front_pulse(extra.snapshot().tick)
        .unwrap();
    for n in 1..=8 {
        step(&mut basic, 1);
        step(&mut extra, 1);
        let a = basic.flight_resources_snapshot().unwrap();
        let b = extra.flight_resources_snapshot().unwrap();
        assert_eq!(a.energy_hundredths, (37_900 + 7 * n).min(38_000));
        assert_eq!(b.energy_hundredths, (37_900 + 14 * n).min(38_000));
        assert_eq!(a.heat_fifths, b.heat_fifths);
        assert_eq!(a.cooldown_ticks, b.cooldown_ticks);
        if n == 1 || n == 7 || n == 8 {
            cold(&basic);
            cold(&extra);
        }
    }
    assert_eq!(
        extra.flight_resources_snapshot().unwrap().energy_hundredths,
        38_000
    );
    assert_eq!(
        basic.flight_resources_snapshot().unwrap().energy_hundredths,
        37_956
    );
}
#[test]
fn storing_and_refitting_energy_and_laser_touch_only_their_own_slots_and_never_refund() {
    let mut w = funded(11301);
    extra_round_trip(&mut w);
    let energy = purchase(&mut w, ModuleKind::ExtraEnergyUnit);
    let pulse = purchase(&mut w, ModuleKind::PulseLaser);
    assert_eq!(w.credits_tenths(), Some(105));
    assert_eq!(w.pulse_laser_snapshot().unwrap().fitted_item_id, pulse);
    assert_eq!(
        w.flight_resources_snapshot()
            .unwrap()
            .energy_recharge_hundredths_per_tick,
        14
    );
    apply(&mut w, ModuleAction::Store { item: energy });
    assert_eq!(
        w.flight_resources_snapshot()
            .unwrap()
            .energy_recharge_hundredths_per_tick,
        7
    );
    assert_eq!(w.pulse_laser_snapshot().unwrap().fitted_item_id, pulse);
    apply(
        &mut w,
        ModuleAction::Install {
            item: 11306,
            replacing: None,
        },
    );
    apply(
        &mut w,
        ModuleAction::Install {
            item: energy,
            replacing: Some(11306),
        },
    );
    apply(
        &mut w,
        ModuleAction::Install {
            item: 11308,
            replacing: Some(pulse),
        },
    );
    assert_eq!(
        w.flight_resources_snapshot()
            .unwrap()
            .energy_recharge_hundredths_per_tick,
        14
    );
    assert_eq!(w.pulse_laser_snapshot().unwrap().fitted_item_id, 11308);
    assert_eq!(w.credits_tenths(), Some(105));
    assert_eq!(w.modular_equipment_snapshot().unwrap().items.len(), 4);
    cold(&w);
}
#[test]
fn paid_pulse_fires_from_its_own_identity_and_its_station_impact_remains_valid_after_later_refit() {
    let mut w = funded(11401);
    let item = purchase(&mut w, ModuleKind::PulseLaser);
    assert_eq!(w.credits_tenths(), Some(70_849));
    launch(&mut w);
    stop(&mut w);
    aim_at(&mut w, crate::docking::Vec3::new(70.0, 0.0, 136.32));
    let shot = w.request_fire_front_pulse(w.snapshot().tick).unwrap();
    assert_eq!(shot.fitted_item_id, item);
    assert!(matches!(shot.contact, PulseContact::StationBlocked { .. }));
    let impact = w.combat_snapshot().unwrap().last_ordinary_impact.unwrap();
    assert_eq!(impact.item, item);
    cold(&w);
    for _ in 0..180 {
        step(&mut w, 1);
    }
    dock(&mut w);
    apply(
        &mut w,
        ModuleAction::Install {
            item: 11408,
            replacing: Some(item),
        },
    );
    assert_eq!(
        w.combat_snapshot().unwrap().last_ordinary_impact.unwrap(),
        impact
    );
    cold(&w);
}
#[test]
fn failed_energy_writer_and_real_cash_or_technology_refusal_preserve_previous_fit_and_bytes() {
    let mut w = funded(11501);
    let before = w.save_bytes().unwrap();
    let tick = w.snapshot().tick;
    let review = w
        .review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::ExtraEnergyUnit,
            },
            tick,
        )
        .unwrap();
    let staged = w.prepare_module(review, tick).unwrap();
    assert_eq!(
        World::from_save_bytes(staged.bytes())
            .unwrap()
            .flight_resources_snapshot()
            .unwrap()
            .energy_recharge_hundredths_per_tick,
        14
    );
    assert_eq!(
        staged.publish_with(|_| Err::<(), _>("writer refused")),
        Err("writer refused")
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    assert_eq!(
        w.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::NavalEnergyUnit
            },
            tick
        ),
        Err(ModuleError::TechnologyUnavailable)
    );
    assert_eq!(
        w.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::EliteEnergyUnit
            },
            tick
        ),
        Err(ModuleError::TechnologyUnavailable)
    );
    assert_eq!(
        w.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::BasicEnergySystem
            },
            tick
        ),
        Err(ModuleError::NotForSale)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let mut new = World::new_commander(11502).unwrap();
    let bytes = new.save_bytes().unwrap();
    assert_eq!(
        new.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::ExtraEnergyUnit
            },
            0
        ),
        Err(ModuleError::InsufficientCredits)
    );
    assert_eq!(
        new.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::PulseLaser
            },
            0
        ),
        Err(ModuleError::InsufficientCredits)
    );
    new.restore_bytes(&bytes).unwrap();
    assert_eq!(new.credits_tenths(), Some(7200));
}
#[test]
fn all_stock_energy_definitions_retain_original_price_tech_cost_and_prerequisite_graph() {
    // Definition/policy unit fixture. High-tech grades are not claimed acquired in the two-site World.
    let world = World::new_commander(11601).unwrap();
    let dock = world.current_dock_context().unwrap();
    let mut ledger = Ledger::born(11601).unwrap();
    let naval = Event {
        xyzzy: false,
        action: ModuleAction::Purchase {
            kind: ModuleKind::NavalEnergyUnit,
        },
        item: 111612,
        dock,
        tick: 0,
        revision: 1,
    };
    assert_eq!(
        ledger.apply(naval, 1),
        Err(ModuleError::MissingPrerequisite)
    );
    for (n, kind, price, tl, rate, units) in [
        (1, ModuleKind::ExtraEnergyUnit, 72000, 9, 2, 2),
        (2, ModuleKind::NavalEnergyUnit, 196000, 17, 3, 3),
        (3, ModuleKind::EliteEnergyUnit, 135000, 14, 4, 4),
    ] {
        assert_eq!(kind.shop(), Some((price, tl)));
        assert_eq!(kind.energy_recharge_multiplier(), Some(rate));
        assert_eq!(kind.equipment_units(), units);
        ledger
            .apply(
                Event {
                    xyzzy: false,
                    action: ModuleAction::Purchase { kind },
                    item: 111611 + u64::from(n),
                    dock,
                    tick: 0,
                    revision: u64::from(n),
                },
                n,
            )
            .unwrap();
        assert_eq!(ledger.fitted(ModuleSlot::Energy).unwrap().kind, kind);
        assert_eq!(ledger.fitted_units(), units);
    }
    let before = ledger.items.clone();
    assert_eq!(
        ledger.apply(
            Event {
                xyzzy: false,
                action: ModuleAction::Purchase {
                    kind: ModuleKind::ExtraEnergyUnit
                },
                item: 111615,
                dock,
                tick: 0,
                revision: 4
            },
            4
        ),
        Err(ModuleError::Capacity)
    );
    assert_eq!(ledger.items, before);
    assert_eq!(ledger.paid, 403_000);
    assert_eq!(ledger.items.len(), 5);
    assert_eq!(
        ledger
            .items
            .iter()
            .filter(|i| i.slot == Some(ModuleSlot::Energy))
            .count(),
        1
    );
}
#[test]
fn stored_energy_keeps_its_single_paid_identity_without_refund() {
    let mut w = funded(11701);
    let item = purchase(&mut w, ModuleKind::ExtraEnergyUnit);
    apply(&mut w, ModuleAction::Store { item });
    let before = w.save_bytes().unwrap();
    let tick = w.snapshot().tick;
    // Cash is checked first; storage cannot turn a paid item into a refund or free replacement.
    assert_eq!(
        w.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::ExtraEnergyUnit
            },
            tick
        ),
        Err(ModuleError::InsufficientCredits)
    );
    apply(
        &mut w,
        ModuleAction::Install {
            item,
            replacing: None,
        },
    );
    assert_eq!(
        w.modular_equipment_snapshot()
            .unwrap()
            .items
            .iter()
            .filter(|i| i.kind == ModuleKind::ExtraEnergyUnit)
            .count(),
        1
    );
    assert_eq!(w.credits_tenths(), Some(13_349));
    assert_ne!(w.save_bytes().unwrap(), before);
    cold(&w);
}
#[test]
fn forged_energy_or_paid_pulse_identity_cannot_rebind_a_prior_save_or_invent_earlier_fit() {
    let mut w = funded(11801);
    let item = purchase(&mut w, ModuleKind::PulseLaser);
    launch(&mut w);
    stop(&mut w);
    aim_at(&mut w, crate::docking::Vec3::new(70.0, 0.0, 136.32));
    w.request_fire_front_pulse(w.snapshot().tick).unwrap();
    let bytes = w.save_bytes().unwrap();
    let mut live = cold(&w);
    let before = live.save_bytes().unwrap();
    assert!(!w.owned_front_pulse_at(11808, w.snapshot().tick, w.combat_identity().unwrap()));
    assert!(w.owned_front_pulse_at(item, w.snapshot().tick, w.combat_identity().unwrap()));
    w.modular.events[0].item = 11808;
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
    let mut future = World::from_save_bytes(&bytes).unwrap();
    future.modular.events[0].tick = u64::MAX;
    assert!(future.save_bytes().is_err());
    assert!(
        live.restore_bytes(&future.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
}
#[test]
fn real_energy_cold_refusal_detects_relabelled_stored_grade_without_a_corresponding_paid_receipt() {
    let mut w = funded(11901);
    purchase(&mut w, ModuleKind::ExtraEnergyUnit);
    let mut live = cold(&w);
    let before = live.save_bytes().unwrap();
    w.modular.events[0].action = ModuleAction::Purchase {
        kind: ModuleKind::NavalEnergyUnit,
    };
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
}

#[test]
fn three_genuine_paid_pulse_spares_reach_original_four_owned_limit_even_after_storage() {
    let mut w = funded(12201);
    let mut ids = Vec::new();
    for _ in 0..3 {
        ids.push(purchase(&mut w, ModuleKind::PulseLaser));
    }
    assert_eq!(w.credits_tenths(), Some(41_849));
    let last = *ids.last().unwrap();
    apply(&mut w, ModuleAction::Store { item: last });
    let before = w.save_bytes().unwrap();
    let tick = w.snapshot().tick;
    assert_eq!(
        w.review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::PulseLaser
            },
            tick
        ),
        Err(ModuleError::Capacity)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let inventory = w.modular_equipment_snapshot().unwrap();
    assert_eq!(
        inventory
            .items
            .iter()
            .filter(|i| i.kind == ModuleKind::PulseLaser)
            .count(),
        4
    );
    assert_eq!(
        inventory
            .items
            .iter()
            .filter(|i| matches!(i.provenance, ModuleProvenance::Paid { .. }))
            .count(),
        3
    );
    assert!(ids.iter().all(|id| *id != 12208));
    cold(&w);
}

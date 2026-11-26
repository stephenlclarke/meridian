//! Wallet/fit positives use real public transactions. Resource preparation is a unit boundary,
//! not a claim that the separate mining scene has been joined or executed.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::practice::tests::launch;
use crate::save::Writer;

fn funded(seed: u64) -> World {
    crate::flight::equipment::tests::earned_world(seed)
}
fn snap(w: &World) -> ModularEquipmentSnapshot {
    w.modular_equipment_snapshot().unwrap()
}
fn apply(w: &mut World, action: ModuleAction) -> u64 {
    let tick = w.snapshot().tick;
    let review = w.review_module(action, tick).unwrap();
    let prepared = w.prepare_module(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    let staged = World::from_save_bytes(&bytes).unwrap();
    let expected = snap(&staged);
    let item = prepared.item();
    assert_eq!(prepared.commit(), item);
    assert_eq!(snap(w), expected);
    assert_eq!(w.save_bytes().unwrap(), bytes);
    item
}
fn cold(w: &World) -> World {
    let bytes = w.save_bytes().unwrap();
    assert_eq!(
        u16::from_le_bytes([bytes[8], bytes[9]]),
        crate::save::SAVE_VERSION
    );
    let loaded = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(snap(&loaded), snap(w));
    assert_eq!(loaded.credits_tenths(), w.credits_tenths());
    assert_eq!(
        loaded.flight_resources_snapshot(),
        w.flight_resources_snapshot()
    );
    assert_eq!(loaded.combat_snapshot(), w.combat_snapshot());
    assert_eq!(loaded.ammunition_snapshot(), w.ammunition_snapshot());
    loaded
}
#[test]
fn targeting_inventory_has_six_original_paid_grades() {
    let mut w = funded(15901);
    let kinds = TargetingTier::ALL.map(ModuleKind::Targeting);
    let prices = [8_000, 20_000, 40_000, 60_000, 100_000, 120_000];
    let technologies = [4, 5, 7, 9, 10, 11];
    let dock = w.current_dock_context().unwrap();
    let mut ledger = Ledger::born(15901).unwrap();
    for (i, kind) in kinds.into_iter().enumerate() {
        let offer = w
            .module_offers()
            .into_iter()
            .find(|o| o.kind == kind)
            .unwrap();
        assert_eq!(
            (
                offer.price_tenths,
                offer.required_technology,
                offer.slot,
                offer.equipment_units
            ),
            (prices[i], technologies[i], ModuleSlot::Targeting, 1)
        );
        assert_eq!(
            offer.prerequisite,
            i.checked_sub(1).map(|previous| kinds[previous])
        );
        assert_eq!(
            validate_purchase(kind, technologies[i] - 1, prices[i]),
            Err(ModuleError::TechnologyUnavailable)
        );
        assert_eq!(
            validate_purchase(kind, technologies[i], prices[i] - 1),
            Err(ModuleError::InsufficientCredits)
        );
        assert_eq!(ModuleKind::from_tag(kind.tag()).unwrap(), kind);
        let id = 200_000 + i as u64;
        ledger
            .apply(
                Event {
                    xyzzy: false,
                    action: ModuleAction::Purchase { kind },
                    item: id,
                    dock,
                    tick: 0,
                    revision: i as u64,
                },
                i as u32 + 1,
            )
            .unwrap();
        assert_eq!(ledger.fitted(ModuleSlot::Targeting).unwrap().id, id);
    }
    let cash = w.credits_tenths().unwrap();
    let buy = |kind| ModuleAction::Purchase { kind };
    assert_eq!(w.targeting_tier(), None);
    assert!(matches!(
        w.review_module(buy(kinds[1]), w.snapshot().tick),
        Err(ModuleError::MissingPrerequisite)
    ));
    let basic = apply(&mut w, buy(kinds[0]));
    let advanced = apply(&mut w, buy(kinds[1]));
    assert_eq!(w.credits_tenths(), Some(cash - prices[0] - prices[1]));
    let before = w.save_bytes().unwrap();
    assert!(matches!(
        w.review_module(
            ModuleAction::Install {
                item: basic,
                replacing: Some(advanced)
            },
            w.snapshot().tick
        ),
        Err(ModuleError::LowerTargetingGrade)
    ));
    assert_eq!(w.save_bytes().unwrap(), before);
    apply(&mut w, ModuleAction::Store { item: advanced });
    assert_eq!(w.targeting_tier(), None);
    apply(
        &mut w,
        ModuleAction::Install {
            item: basic,
            replacing: None,
        },
    );
    assert_eq!(cold(&w).targeting_tier(), Some(TargetingTier::Basic));
}
#[test]
fn xyzzy_targeting_inventory_refits_same_maximum_item_without_another_grant() {
    let mut w = World::new_commander(15902).unwrap();
    let review = w.review_xyzzy(crate::XyzzyAction::Enter, 0).unwrap();
    w.prepare_xyzzy(review, 0).unwrap().commit();
    let maximum = snap(&w)
        .items
        .into_iter()
        .find(|item| item.kind == ModuleKind::Targeting(TargetingTier::Military))
        .unwrap();
    assert!(matches!(maximum.provenance, ModuleProvenance::XYZzy { .. }));
    assert_eq!(w.targeting_tier(), Some(TargetingTier::Military));
    apply(&mut w, ModuleAction::Store { item: maximum.id });
    apply(
        &mut w,
        ModuleAction::Purchase {
            kind: ModuleKind::Targeting(TargetingTier::Basic),
        },
    );
    let cash = w.credits_tenths();
    let tick = w.snapshot().tick;
    let review = w
        .review_xyzzy(crate::XyzzyAction::MaximumSupportedHardware, tick)
        .unwrap();
    w.prepare_xyzzy(review, tick).unwrap().commit();
    let restored = cold(&w);
    assert_eq!(restored.credits_tenths(), cash);
    assert_eq!(restored.targeting_tier(), Some(TargetingTier::Military));
    assert_eq!(
        snap(&restored)
            .items
            .into_iter()
            .filter(|item| item.kind == maximum.kind)
            .collect::<Vec<_>>(),
        vec![maximum]
    );
}
#[test]
fn paid_mining_laser_uses_original_3300cr_tl6_and_stores_the_same_born_pulse() {
    let mut w = funded(9101);
    let cash = w.credits_tenths().unwrap();
    let resources = w.flight_resources_snapshot();
    let ammo = w.ammunition_snapshot();
    let pulse = snap(&w).items[0];
    let berth = w.current_dock_context().unwrap();
    assert_eq!(pulse.id, 9108);
    assert_eq!(pulse.provenance, ModuleProvenance::StandardBirth);
    let offer = snap(&w).mining_offer.unwrap();
    assert_eq!(offer.price_tenths, 33_000);
    assert_eq!(offer.required_technology, 6);
    assert!(offer.available);
    let item = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    assert_eq!(item, 109112);
    assert_eq!(w.credits_tenths(), Some(cash - 33_000));
    let after = snap(&w);
    assert_eq!(after.items.len(), 5);
    assert_eq!(after.fit_revision, 1);
    assert_eq!(
        after.items[1],
        OwnedModule {
            id: 9106,
            kind: ModuleKind::BasicEnergySystem,
            slot: Some(ModuleSlot::Energy),
            provenance: ModuleProvenance::StandardBirth,
        }
    );
    assert_eq!(
        after.items[0],
        OwnedModule {
            slot: None,
            ..pulse
        }
    );
    assert_eq!(after.items[4].slot, Some(ModuleSlot::FrontLaser));
    assert_eq!(after.items[4].kind, ModuleKind::MiningLaser);
    assert!(
        matches!(after.items[4].provenance, ModuleProvenance::Paid { transaction: 1, dock, paid_tenths: 33_000, .. } if dock == berth)
    );
    assert_eq!(w.flight_resources_snapshot(), resources);
    assert_eq!(w.ammunition_snapshot(), ammo);
    assert!(w.pulse_laser_snapshot().is_none());
    assert_eq!(
        w.combat_snapshot().unwrap().player.energy_hundredths,
        38_000
    );
    let tool = w.fitted_mining_tool().unwrap();
    assert_eq!(
        (tool.item, tool.damage, tool.range, tool.cooldown_ticks),
        (item, 50, 6000, 18)
    );
    cold(&w);
}
#[test]
fn ordinary_new_commander_cannot_afford_a_free_mining_tool() {
    let w = World::new_commander(9201).unwrap();
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.review_module(ModuleAction::PurchaseMiningLaser, 0),
        Err(ModuleError::InsufficientCredits)
    );
    assert!(w.fitted_mining_tool().is_none());
    assert_eq!(w.credits_tenths(), Some(7200));
    assert_eq!(w.save_bytes().unwrap(), before);
}
#[test]
fn failed_writer_drops_paid_fit_and_displaced_pulse_together() {
    let mut w = funded(9301);
    let before = w.save_bytes().unwrap();
    let tick = w.snapshot().tick;
    let review = w
        .review_module(ModuleAction::PurchaseMiningLaser, tick)
        .unwrap();
    let prepared = w.prepare_module(review, tick).unwrap();
    let staged = World::from_save_bytes(prepared.bytes()).unwrap();
    assert!(staged.fitted_mining_tool().is_some());
    assert!(staged.pulse_laser_snapshot().is_none());
    assert_eq!(
        prepared.publish_with(|bytes| {
            assert!(!bytes.is_empty());
            Err::<(), _>("atomic write refused")
        }),
        Err("atomic write refused")
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    assert!(w.pulse_laser_snapshot().is_some());
}
#[test]
fn removing_and_reinstalling_exact_items_never_refunds_duplicates_or_resets_resources() {
    let mut w = funded(9401);
    let pulse = snap(&w).items[0];
    let item = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    let cash = w.credits_tenths();
    let resources = w.flight_resources_snapshot();
    let tick = w.snapshot().tick;
    assert_eq!(
        w.review_module(
            ModuleAction::Install {
                item: pulse.id,
                replacing: None
            },
            tick
        ),
        Err(ModuleError::StaleOccupant)
    );
    apply(&mut w, ModuleAction::Store { item });
    assert!(w.fitted_mining_tool().is_none());
    assert!(w.pulse_laser_snapshot().is_none());
    assert!(!w.combat_snapshot().unwrap().player.destroyed);
    apply(
        &mut w,
        ModuleAction::Install {
            item: pulse.id,
            replacing: None,
        },
    );
    assert_eq!(w.pulse_laser_snapshot().unwrap().fitted_item_id, pulse.id);
    apply(
        &mut w,
        ModuleAction::Install {
            item,
            replacing: Some(pulse.id),
        },
    );
    assert_eq!(w.fitted_mining_tool().unwrap().item, item);
    assert_eq!(w.credits_tenths(), cash);
    assert_eq!(w.flight_resources_snapshot(), resources);
    assert_eq!(snap(&w).items.len(), 5);
    assert_eq!(
        snap(&w).items[0].provenance,
        ModuleProvenance::StandardBirth
    );
    assert!(matches!(
        snap(&w).items[4].provenance,
        ModuleProvenance::Paid {
            transaction: 1,
            paid_tenths: 33_000,
            ..
        }
    ));
    cold(&w);
}
#[test]
fn stale_foreign_paused_and_pre_continue_reviews_never_publish() {
    let mut w = funded(9501);
    let mut other = funded(9502);
    let tick = w.snapshot().tick;
    let review = w
        .review_module(ModuleAction::PurchaseMiningLaser, tick)
        .unwrap();
    let before = other.save_bytes().unwrap();
    assert!(other.prepare_module(review, other.snapshot().tick).is_err());
    assert_eq!(other.save_bytes().unwrap(), before);
    w.request_pause(true, tick).unwrap();
    assert!(matches!(
        w.prepare_module(review, tick),
        Err(ModuleError::Paused)
    ));
    w.request_pause(false, tick).unwrap();
    let bytes = w.save_bytes().unwrap();
    w.restore_bytes(&bytes).unwrap();
    assert!(matches!(
        w.prepare_module(review, tick),
        Err(ModuleError::StaleReview)
    ));
    let review = w
        .review_module(ModuleAction::PurchaseMiningLaser, tick)
        .unwrap();
    apply(&mut w, ModuleAction::PurchaseMiningLaser);
    assert!(matches!(
        w.prepare_module(review, tick),
        Err(ModuleError::StaleReview)
    ));
    launch(&mut w);
    assert_eq!(
        w.review_module(ModuleAction::Store { item: 9508 }, w.snapshot().tick),
        Err(ModuleError::BerthRequired)
    );
}
#[test]
fn actual_mining_binding_changes_on_store_refit_and_continue_without_pulse_alias() {
    let mut w = funded(9601);
    assert!(w.fitted_mining_tool().is_none());
    let item = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    let first = w.fitted_mining_tool().unwrap();
    assert_eq!(first.owner, 9601);
    assert_eq!(first.item, item);
    apply(&mut w, ModuleAction::Store { item });
    assert!(w.fitted_mining_tool().is_none());
    apply(
        &mut w,
        ModuleAction::Install {
            item,
            replacing: None,
        },
    );
    let refitted = w.fitted_mining_tool().unwrap();
    assert_ne!(refitted.account_revision, first.account_revision);
    let bytes = w.save_bytes().unwrap();
    w.restore_bytes(&bytes).unwrap();
    let continued = w.fitted_mining_tool().unwrap();
    assert_eq!(continued.item, first.item);
    assert_ne!(continued.epoch, refitted.epoch);
    assert_eq!(continued.account_revision, refitted.account_revision);
}
#[test]
fn mining_resource_preflight_uses_same_pool_and_real_fit_without_committing_a_fake_shot() {
    let mut w = funded(9701);
    let item = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    launch(&mut w);
    let tick = w.snapshot().tick;
    let before = w.save_bytes().unwrap();
    let resources = w.flight_resources_snapshot().unwrap();
    assert_eq!(
        w.request_fire_front_pulse(tick),
        Err(super::super::FireError::NoFittedFrontPulseLaser)
    );
    assert!(w.prepare_mining_resources(item + 1, tick).is_err());
    let candidate = w.prepare_mining_resources(item, tick).unwrap();
    let mut writer = Writer::new();
    candidate.encode_save(&mut writer);
    assert_eq!(
        writer.bytes,
        [
            37_900_u16.to_le_bytes().as_slice(),
            50_u16.to_le_bytes().as_slice(),
            &[18]
        ]
        .concat()
    );
    assert_eq!(resources.energy_hundredths, 38_000);
    assert_eq!(resources.heat_fifths, 0);
    assert_eq!(w.save_bytes().unwrap(), before);
    w.request_pause(true, tick).unwrap();
    assert!(w.prepare_mining_resources(item, tick).is_err());
    w.request_pause(false, tick).unwrap();
    cold(&w);
}
#[test]
fn source_canonical_repeatable_mining_node_refuses_sixth_owned_example() {
    // Pure ownership-policy fixture, not a funded World purchase or gameplay acceptance.
    let w = World::new_commander(9801).unwrap();
    let dock = w.current_dock_context().unwrap();
    let mut ledger = Ledger::default();
    for transaction in 1..=5 {
        ledger
            .apply(
                Event {
                    xyzzy: false,
                    action: ModuleAction::PurchaseMiningLaser,
                    item: 110000 + u64::from(transaction),
                    dock,
                    tick: 0,
                    revision: u64::from(transaction),
                },
                transaction,
            )
            .unwrap();
    }
    assert_eq!(ledger.items.len(), 5);
    assert_eq!(ledger.paid, 165_000);
    assert_eq!(
        ledger.apply(
            Event {
                xyzzy: false,
                action: ModuleAction::PurchaseMiningLaser,
                item: 110006,
                dock,
                tick: 0,
                revision: 6
            },
            6
        ),
        Err(ModuleError::Capacity)
    );
}
fn reject(mut w: World, change: impl FnOnce(&mut World)) {
    let mut live = cold(&w);
    let before = live.save_bytes().unwrap();
    change(&mut w);
    assert!(w.save_bytes().is_err());
    let bytes = w.unchecked_test_save().unwrap();
    assert!(live.restore_bytes(&bytes).is_err());
    assert_eq!(live.save_bytes().unwrap(), before);
}
#[test]
fn coherent_checksum_cannot_erase_reassign_or_duplicate_paid_module_receipts() {
    let mut w = funded(9901);
    apply(&mut w, ModuleAction::PurchaseMiningLaser);
    let saved = w.save_bytes().unwrap();
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        w.modular.events.clear()
    });
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        w.modular.events[0].item += 1
    });
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        let e = w.modular.events[0];
        w.modular.events.push(e);
    });
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        w.modular.events[0].revision = u64::MAX
    });
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        w.modular.events[0].dock = World::new_commander(9902)
            .unwrap()
            .current_dock_context()
            .unwrap()
    });
}

#[test]
fn actual_cargo_bay_and_stored_born_pulse_reconcile_one_wallet_and_real_capacity() {
    let mut w = funded(10001);
    let tick = w.snapshot().tick;
    w.request_equipment(crate::equipment::Command::PurchaseCargoBay, tick)
        .unwrap();
    let cash = w.credits_tenths().unwrap();
    let cargo = w.equipment_snapshot();
    let item = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    assert_eq!(w.credits_tenths(), Some(cash - 33_000));
    assert_eq!(w.equipment_snapshot(), cargo);
    assert_eq!(w.service_snapshot().unwrap().hold_capacity_tonnes, 127);
    assert_eq!(w.fitted_mining_tool().unwrap().item, item);
    cold(&w);
}
#[test]
fn paid_module_fitting_preserves_three_real_occupied_lesson_slots_and_invalidates_old_review() {
    let mut w = funded(10101);
    if w.snapshot().navigation.unwrap().fuel_units < 34 {
        w.request_refuel(w.snapshot().tick).unwrap();
    }
    crate::flight::equipment::tests::journey(&mut w, crate::navigation::SystemId::AVENOR);
    let tick = w.snapshot().tick;
    let t01 = w
        .review_flight_reception(crate::FlightReceptionAction::Accept, tick)
        .unwrap();
    let t04 = w
        .review_navigation_trade(crate::NavigationTradeAction::Accept, tick)
        .unwrap();
    let practice = w.review_practice(tick).unwrap();
    let item = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    assert!(w.prepare_flight_reception(t01, tick).is_err());
    assert!(w.prepare_navigation_trade(t04, tick).is_err());
    assert!(w.prepare_practice_confirmation(practice, tick).is_err());
    apply(
        &mut w,
        ModuleAction::Install {
            item: 10108,
            replacing: Some(item),
        },
    );
    let t01 = w
        .review_flight_reception(crate::FlightReceptionAction::Accept, tick)
        .unwrap();
    w.prepare_flight_reception(t01, tick).unwrap().commit();
    let t04 = w
        .review_navigation_trade(crate::NavigationTradeAction::Accept, tick)
        .unwrap();
    w.prepare_navigation_trade(t04, tick).unwrap().commit();
    crate::flight::practice::tests::consent(&mut w);
    assert_eq!(w.original_service_occupancy(), (3, 3));
    apply(
        &mut w,
        ModuleAction::Install {
            item,
            replacing: Some(10108),
        },
    );
    assert_eq!(w.original_service_occupancy(), (3, 3));
    cold(&w);
}
#[test]
fn same_born_pulse_can_fire_after_genuine_store_and_refit_without_new_weapon_identity() {
    let mut w = funded(10201);
    let item = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    apply(
        &mut w,
        ModuleAction::Install {
            item: 10208,
            replacing: Some(item),
        },
    );
    let cash = w.credits_tenths();
    launch(&mut w);
    let shot = w.request_fire_front_pulse(w.snapshot().tick).unwrap();
    assert_eq!(shot.fitted_item_id, 10208);
    assert_eq!(w.credits_tenths(), cash);
    let resources = w.flight_resources_snapshot().unwrap();
    assert_eq!(resources.energy_hundredths, 37_900);
    assert_eq!(resources.heat_fifths, 50);
    assert_eq!(resources.cooldown_ticks, 18);
    cold(&w);
}
#[test]
fn bounded_real_module_history_refuses_next_action_without_touching_wallet_fit_or_save() {
    let mut w = funded(10301);
    let item = apply(&mut w, ModuleAction::PurchaseMiningLaser);
    for n in 0..crate::save::MAX_SAVE_BYTES {
        let action = if n % 2 == 0 {
            ModuleAction::Store { item }
        } else {
            ModuleAction::Install {
                item,
                replacing: None,
            }
        };
        let tick = w.snapshot().tick;
        let review = w.review_module(action, tick).unwrap();
        let before = w.save_bytes().unwrap();
        match w.prepare_module(review, tick).map(|p| p.commit()) {
            Ok(_) => {}
            Err(ModuleError::Save(SaveError::TooLarge)) => {
                assert_eq!(w.save_bytes().unwrap(), before);
                cold(&w);
                return;
            }
            Err(error) => panic!("unexpected refusal: {error:?}"),
        }
    }
    panic!("module history escaped the complete-save byte bound");
}

#[test]
fn repeatable_beam_and_military_policy_counts_stored_examples_toward_the_four_item_limit() {
    // Pure ledger policy fixture, not a funded purchase or technology admission.
    let world = World::new_commander(14901).unwrap();
    let dock = world.current_dock_context().unwrap();
    for kind in [ModuleKind::BeamLaser, ModuleKind::MilitaryLaser] {
        let mut ledger = Ledger::default();
        for transaction in 1..=4 {
            ledger
                .apply(
                    Event {
                        xyzzy: false,
                        action: ModuleAction::Purchase { kind },
                        item: u64::from(transaction),
                        dock,
                        tick: 0,
                        revision: u64::from(transaction),
                    },
                    transaction,
                )
                .unwrap();
            if transaction == 1 {
                ledger
                    .apply(
                        Event {
                            xyzzy: false,
                            action: ModuleAction::Store { item: 1 },
                            item: 1,
                            dock,
                            tick: 0,
                            revision: 1,
                        },
                        transaction,
                    )
                    .unwrap();
            }
        }
        assert_eq!(ledger.items.len(), 4);
        assert_eq!(ledger.items[0].slot, None);
        assert_eq!(
            ledger.apply(
                Event {
                    xyzzy: false,
                    action: ModuleAction::Purchase { kind },
                    item: 5,
                    dock,
                    tick: 0,
                    revision: 5
                },
                5
            ),
            Err(ModuleError::Capacity)
        );
        assert_eq!(ledger.items.len(), 4);
    }
}

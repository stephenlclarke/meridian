//! Positive fixtures use ordinary public commands. Private writes only create invalid saves.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::FlightInput;
use crate::flight::missiles::tests::{dock, fire, stop};
use crate::flight::practice::tests::{consent, launch, step};
fn apply(w: &mut World, action: AmmunitionAction) -> u64 {
    let tick = w.snapshot().tick;
    let review = w.review_ammunition(action, tick).unwrap();
    let prepared = w.prepare_ammunition(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    let item = prepared.item();
    let candidate = World::from_save_bytes(&bytes).unwrap();
    let inventory = candidate.ammunition_snapshot();
    assert_eq!(prepared.commit(), item);
    assert_eq!(w.ammunition_snapshot(), inventory);
    assert_eq!(w.save_bytes().unwrap(), bytes);
    item
}
fn cold(w: &World) -> World {
    let bytes = w.save_bytes().unwrap();
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 13);
    let other = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(other.ammunition_snapshot(), w.ammunition_snapshot());
    assert_eq!(
        other.missile_projectiles().collect::<Vec<_>>(),
        w.missile_projectiles().collect::<Vec<_>>()
    );
    assert_eq!(other.credits_tenths(), w.credits_tenths());
    assert_eq!(other.pulse_laser_snapshot(), w.pulse_laser_snapshot());
    assert_eq!(other.combat_snapshot(), w.combat_snapshot());
    other
}
#[test]
fn actual_paid_round_uses_first_free_of_nine_without_six_free_rounds() {
    let mut w = World::new_commander(7101).unwrap();
    let before = w.ammunition_snapshot().unwrap();
    assert_eq!(before.capacity, 9);
    assert_eq!(before.items.len(), 3);
    assert_eq!(before.price_tenths, 1800);
    assert_eq!(before.required_technology, 2);
    let pulse = w.pulse_laser_snapshot();
    let combat = w.combat_snapshot();
    let equipment = w.equipment_snapshot();
    let services = w.service_snapshot();
    let item = apply(&mut w, AmmunitionAction::Purchase);
    assert_eq!(item, 7112);
    assert_eq!(w.credits_tenths(), Some(5400));
    let after = w.ammunition_snapshot().unwrap();
    assert_eq!(after.fit_revision, before.fit_revision + 1);
    assert_eq!(after.items[..3], before.items);
    assert_eq!(
        after.items[3],
        AmmunitionItem {
            item,
            pylon: Some(3),
            purchase: Some(1),
            consumed: false
        }
    );
    assert_eq!(w.pulse_laser_snapshot(), pulse);
    assert_eq!(w.combat_snapshot(), combat);
    assert_eq!(w.equipment_snapshot(), equipment);
    assert_eq!(w.service_snapshot(), services);
    cold(&w);
}
#[test]
fn failed_publication_drops_complete_paid_candidate_without_spend_or_fit_change() {
    let mut w = World::new_commander(7201).unwrap();
    let before = w.save_bytes().unwrap();
    let review = w.review_ammunition(AmmunitionAction::Purchase, 0).unwrap();
    let prepared = w.prepare_ammunition(review, 0).unwrap();
    let staged = World::from_save_bytes(prepared.bytes()).unwrap();
    assert_eq!(staged.credits_tenths(), Some(5400));
    let failed = prepared.publish_with(|bytes| {
        assert_eq!(
            World::from_save_bytes(bytes).unwrap().credits_tenths(),
            Some(5400)
        );
        Err::<(), _>("injected atomic writer failure")
    });
    assert_eq!(failed, Err("injected atomic writer failure"));
    assert_eq!(w.save_bytes().unwrap(), before);
    assert_eq!(apply(&mut w, AmmunitionAction::Purchase), 7212);
}
#[test]
fn stale_foreign_pause_continue_and_flight_reviews_refuse_atomically() {
    let mut w = World::new_commander(7301).unwrap();
    let review = w.review_ammunition(AmmunitionAction::Purchase, 0).unwrap();
    let mut other = World::new_commander(7302).unwrap();
    let before = other.save_bytes().unwrap();
    assert!(matches!(
        other.prepare_ammunition(review, 0),
        Err(AmmunitionError::StaleReview)
    ));
    assert_eq!(other.save_bytes().unwrap(), before);
    w.request_pause(true, 0).unwrap();
    assert!(matches!(
        w.prepare_ammunition(review, 0),
        Err(AmmunitionError::Paused)
    ));
    w.request_pause(false, 0).unwrap();
    let saved = w.save_bytes().unwrap();
    w.restore_bytes(&saved).unwrap();
    assert!(matches!(
        w.prepare_ammunition(review, 0),
        Err(AmmunitionError::StaleReview)
    ));
    let review = w.review_ammunition(AmmunitionAction::Purchase, 0).unwrap();
    apply(&mut w, AmmunitionAction::Purchase);
    assert!(matches!(
        w.prepare_ammunition(review, 0),
        Err(AmmunitionError::StaleReview)
    ));
    w.step(FlightInput::default()).unwrap();
    assert!(matches!(
        w.prepare_ammunition(review, 0),
        Err(AmmunitionError::StaleReview)
    ));
    launch(&mut w);
    assert_eq!(
        w.review_ammunition(AmmunitionAction::Purchase, w.snapshot().tick),
        Err(AmmunitionError::BerthRequired)
    );
}
#[test]
fn same_paid_and_born_id_store_refit_preserves_cash_and_acquisition() {
    let mut w = World::new_commander(7401).unwrap();
    let item = apply(&mut w, AmmunitionAction::Purchase);
    let paid = w.credits_tenths();
    apply(&mut w, AmmunitionAction::Store { item });
    let held = w.ammunition_snapshot().unwrap().items[3];
    assert_eq!(held.purchase, Some(1));
    assert_eq!(held.pylon, None);
    apply(&mut w, AmmunitionAction::Refit { item, pylon: 8 });
    assert_eq!(w.ammunition_snapshot().unwrap().items[3].pylon, Some(8));
    assert_eq!(w.credits_tenths(), paid);
    let born = 7409;
    apply(&mut w, AmmunitionAction::Store { item: born });
    apply(
        &mut w,
        AmmunitionAction::Refit {
            item: born,
            pylon: 7,
        },
    );
    assert_eq!(w.ammunition_snapshot().unwrap().items.len(), 4);
    let bytes = w.save_bytes().unwrap();
    assert_eq!(
        w.review_ammunition(AmmunitionAction::Refit { item, pylon: 7 }, 0),
        Err(AmmunitionError::Occupied)
    );
    assert_eq!(
        w.review_ammunition(AmmunitionAction::Store { item: 999 }, 0),
        Err(AmmunitionError::MissingItem)
    );
    assert_eq!(w.save_bytes().unwrap(), bytes);
    cold(&w);
}
#[test]
fn actual_wallet_refuses_fifth_purchase_without_grant_or_refund() {
    let mut w = World::new_commander(7501).unwrap();
    for _ in 0..4 {
        apply(&mut w, AmmunitionAction::Purchase);
    }
    assert_eq!(w.credits_tenths(), Some(0));
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.review_ammunition(AmmunitionAction::Purchase, 0),
        Err(AmmunitionError::InsufficientCredits)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    cold(&w);
}
#[test]
fn actual_nine_positions_refuse_tenth_round_with_earned_wallet() {
    let mut w = crate::flight::equipment::tests::earned_world(7601);
    for _ in 0..6 {
        apply(&mut w, AmmunitionAction::Purchase);
    }
    let tick = w.snapshot().tick;
    let before = w.save_bytes().unwrap();
    assert_eq!(w.missile_rounds().unwrap().iter().flatten().count(), 9);
    assert_eq!(
        w.review_ammunition(AmmunitionAction::Purchase, tick),
        Err(AmmunitionError::Full)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    cold(&w);
}
#[test]
fn five_actual_discharges_keep_every_origin_beyond_old_three_shot_archive() {
    let mut w = World::new_commander(7701).unwrap();
    apply(&mut w, AmmunitionAction::Purchase);
    apply(&mut w, AmmunitionAction::Purchase);
    consent(&mut w);
    launch(&mut w);
    stop(&mut w);
    let target = w.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let mut origins = Vec::new();
    for expected in [7713, 7712, 7711, 7710, 7709] {
        let o = fire(&mut w, target);
        assert_eq!(o.item, expected);
        origins.push(o);
        for _ in 0..100 {
            step(&mut w, 1);
        }
        let loaded = cold(&w);
        assert_eq!(
            loaded
                .missile_projectiles()
                .map(|p| p.origin)
                .collect::<Vec<_>>(),
            origins
        );
        assert_eq!(loaded.credits_tenths(), Some(3600));
    }
    assert!(w.missile_rounds().unwrap().iter().all(Option::is_none));
    assert_eq!(w.missile_projectiles().count(), 5);
    let p = w.ammunition_snapshot().unwrap();
    assert!(p.items.iter().all(|i| i.consumed));
    assert_eq!(p.items.iter().filter(|i| i.purchase.is_some()).count(), 2);
}
#[test]
fn real_paid_replacement_after_three_spent_starters_does_not_resurrect_or_reset_archive() {
    let mut w = World::new_commander(7801).unwrap();
    consent(&mut w);
    launch(&mut w);
    stop(&mut w);
    let target = w.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let mut old = Vec::new();
    for _ in 0..3 {
        old.push(fire(&mut w, target));
        for _ in 0..100 {
            step(&mut w, 1);
        }
    }
    dock(&mut w);
    let cash = w.credits_tenths().unwrap();
    let item = apply(&mut w, AmmunitionAction::Purchase);
    assert_eq!(item, 7812);
    assert_eq!(w.credits_tenths(), Some(cash - 1800));
    for spent in &old {
        let tick = w.snapshot().tick;
        assert_eq!(
            w.review_ammunition(
                AmmunitionAction::Refit {
                    item: spent.item,
                    pylon: 1
                },
                tick
            ),
            Err(AmmunitionError::MissingItem)
        );
    }
    let review = w.review_practice_closure(w.snapshot().tick).unwrap();
    w.prepare_practice_closure(review, w.snapshot().tick)
        .unwrap()
        .commit();
    consent(&mut w);
    launch(&mut w);
    stop(&mut w);
    let target = w.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let new = fire(&mut w, target);
    assert_eq!(new.item, item);
    assert_eq!(new.projectile.entity(), 8);
    let all = w
        .missile_projectiles()
        .map(|p| p.origin)
        .collect::<Vec<_>>();
    assert_eq!(&all[..3], &old);
    assert_eq!(all[3], new);
    for _ in 0..100 {
        step(&mut w, 1);
    }
    cold(&w);
    dock(&mut w);
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.review_ammunition(
            AmmunitionAction::Refit { item, pylon: 0 },
            w.snapshot().tick
        ),
        Err(AmmunitionError::MissingItem)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let replacement = apply(&mut w, AmmunitionAction::Purchase);
    assert_ne!(replacement, item);
    assert_eq!(
        w.missile_projectiles()
            .map(|p| p.origin)
            .collect::<Vec<_>>(),
        all
    );
}
#[test]
fn moved_born_item_fires_from_actual_highest_position_without_new_paid_origin() {
    let mut w = World::new_commander(7901).unwrap();
    apply(
        &mut w,
        AmmunitionAction::Refit {
            item: 7909,
            pylon: 8,
        },
    );
    consent(&mut w);
    launch(&mut w);
    stop(&mut w);
    let target = w.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    assert_eq!(fire(&mut w, target).item, 7909);
    assert_eq!(w.credits_tenths(), Some(7200));
    cold(&w);
}
#[test]
fn paid_purchase_invalidates_existing_real_fit_reviews_without_consuming_service_slots() {
    let mut w = World::new_commander(8001).unwrap();
    let review = w.review_practice(0).unwrap();
    let before = w.original_service_occupancy();
    let t01 = w
        .review_flight_reception(crate::FlightReceptionAction::Accept, 0)
        .unwrap();
    apply(&mut w, AmmunitionAction::Purchase);
    assert!(w.prepare_practice_confirmation(review, 0).is_err());
    assert!(w.prepare_flight_reception(t01, 0).is_err());
    assert_eq!(w.original_service_occupancy(), before);
    cold(&w);
}
fn reject(mut w: World, change: impl FnOnce(&mut World)) {
    let mut live = cold(&w);
    let before = live.save_bytes().unwrap();
    change(&mut w);
    assert!(w.save_bytes().is_err());
    let poisoned = w.unchecked_test_save().unwrap();
    assert!(live.restore_bytes(&poisoned).is_err());
    assert_eq!(live.save_bytes().unwrap(), before);
}
#[test]
fn coherent_checksum_cannot_erase_duplicate_reprice_or_reassign_paid_custody() {
    let mut w = World::new_commander(8101).unwrap();
    apply(&mut w, AmmunitionAction::Purchase);
    let saved = w.save_bytes().unwrap();
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        w.ammunition.events.clear()
    });
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        w.ammunition.events[0].item += 1
    });
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        let e = w.ammunition.events[0];
        w.ammunition.events.push(e);
    });
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        w.ammunition.events[0].revision += 1
    });
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        w.ammunition.events[0].tick = u64::MAX
    });
    reject(World::from_save_bytes(&saved).unwrap(), |w| {
        w.ammunition.events[0].dock = World::new_commander(8102)
            .unwrap()
            .current_dock_context()
            .unwrap();
    });
}

#[test]
fn all_three_actual_training_slots_remain_occupied_during_paid_fitting() {
    let mut w = World::new_commander(8201).unwrap();
    let review = w
        .review_navigation_trade(crate::NavigationTradeAction::Accept, 0)
        .unwrap();
    w.prepare_navigation_trade(review, 0).unwrap().commit();
    let review = w
        .review_flight_reception(crate::FlightReceptionAction::Accept, 0)
        .unwrap();
    w.prepare_flight_reception(review, 0).unwrap().commit();
    consent(&mut w);
    assert_eq!(w.original_service_occupancy(), (3, 3));
    let cash = w.credits_tenths().unwrap();
    let item = apply(&mut w, AmmunitionAction::Purchase);
    apply(&mut w, AmmunitionAction::Store { item });
    apply(&mut w, AmmunitionAction::Refit { item, pylon: 8 });
    assert_eq!(w.original_service_occupancy(), (3, 3));
    assert_eq!(w.credits_tenths(), Some(cash - 1800));
    cold(&w);
}

#[test]
fn actual_complete_save_byte_bound_refuses_next_free_fitting_action_without_mutation() {
    let mut w = World::new_commander(8301).unwrap();
    let item = apply(&mut w, AmmunitionAction::Purchase);
    for n in 0..crate::save::MAX_SAVE_BYTES {
        let action = if n % 2 == 0 {
            AmmunitionAction::Store { item }
        } else {
            AmmunitionAction::Refit { item, pylon: 3 }
        };
        let tick = w.snapshot().tick;
        let review = w.review_ammunition(action, tick).unwrap();
        let before = w.save_bytes().unwrap();
        let result = w.prepare_ammunition(review, tick).map(|p| p.commit());
        match result {
            Ok(_) => {}
            Err(AmmunitionError::Save(SaveError::TooLarge)) => {
                assert_eq!(w.save_bytes().unwrap(), before);
                assert!(before.len() <= 65_536);
                cold(&w);
                return;
            }
            Err(error) => panic!("unexpected fitting refusal: {error:?}"),
        }
    }
    panic!("fitting history escaped the existing complete-save byte cap");
}

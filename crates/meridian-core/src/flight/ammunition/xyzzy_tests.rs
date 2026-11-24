#![allow(clippy::unwrap_used)]
use super::World;
#[test]
fn actual_secret_missile_cannot_be_relabelled_as_a_paid_purchase_or_paid_identity() {
    let mut w = World::new_commander(15401).unwrap();
    let review = w.review_xyzzy(crate::XyzzyAction::Enter, 0).unwrap();
    w.prepare_xyzzy(review, 0).unwrap().commit();
    let valid = w.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let before = live.save_bytes().unwrap();
    w.ammunition.events[0].xyzzy = false;
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
    w = World::from_save_bytes(&valid).unwrap();
    w.ammunition.events[0].item = 15412;
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
}

#[test]
fn xyzzy_fills_every_pylon_and_preserves_a_stored_paid_round_on_reload() {
    use crate::{AmmunitionAction, XyzzyAction};
    let mut w = World::new_commander(15402).unwrap();
    let buy = w.review_ammunition(AmmunitionAction::Purchase, 0).unwrap();
    let id = w.prepare_ammunition(buy, 0).unwrap().commit();
    let store = w
        .review_ammunition(AmmunitionAction::Store { item: id }, 0)
        .unwrap();
    w.prepare_ammunition(store, 0).unwrap().commit();
    let paid_history = w.ammunition_snapshot().unwrap().purchases;
    assert_eq!(paid_history[0].price_tenths, super::PRICE_TENTHS);
    for action in [XyzzyAction::Enter, XyzzyAction::MaximumSupportedHardware] {
        let review = w.review_xyzzy(action, 0).unwrap();
        w.prepare_xyzzy(review, 0).unwrap().commit();
        w = World::from_save_bytes(&w.save_bytes().unwrap()).unwrap();
        let inventory = w.ammunition_snapshot().unwrap();
        assert_eq!(inventory.purchases, paid_history);
        assert_eq!(inventory.items.len(), super::PYLONS);
        for pylon in 0..inventory.capacity {
            assert_eq!(
                inventory
                    .items
                    .iter()
                    .filter(|i| i.pylon == Some(pylon) && !i.consumed)
                    .count(),
                1
            );
        }
        assert!(
            inventory
                .items
                .iter()
                .any(|i| i.item == id && !i.xyzzy && i.purchase == Some(1))
        );
        assert_eq!(
            w.credits_tenths(),
            Some(crate::flight::xyzzy::STARTING_CREDITS_TENTHS)
        );
    }
}

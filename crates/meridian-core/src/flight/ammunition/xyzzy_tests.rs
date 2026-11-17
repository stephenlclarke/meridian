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

//! Coherent negative grant-relabel fixtures; positives use the actual explicit secret command.
#![allow(clippy::unwrap_used)]
use super::{ModuleAction, ModuleKind, World};
fn active(seed: u64) -> World {
    let mut w = World::new_commander(seed).unwrap();
    let review = w.review_xyzzy(crate::XyzzyAction::Enter, 0).unwrap();
    w.prepare_xyzzy(review, 0).unwrap().commit();
    w
}
fn refuses(w: &World, live: &mut World, before: &[u8]) {
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
}
#[test]
fn secret_elite_cannot_be_relabelled_as_paid_or_as_an_unsupported_secret_weapon() {
    let mut w = active(15301);
    let valid = w.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let before = live.save_bytes().unwrap();
    w.modular.events[0].xyzzy = false;
    refuses(&w, &mut live, &before);
    w = World::from_save_bytes(&valid).unwrap();
    w.modular.events[0].action = ModuleAction::Purchase {
        kind: ModuleKind::PulseLaser,
    };
    refuses(&w, &mut live, &before);
}

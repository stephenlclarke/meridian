//! A real paid Mining Laser, not a modified born fixture, creates the interception receipt.
use super::*;
use crate::{ModuleAction, ModuleKind};

#[test]
fn paid_mining_interception_binds_actual_fitted_item_at_shot_and_survives_later_born_refit() {
    let mut w = crate::flight::equipment::tests::earned_world(13601);
    w.request_refuel(w.snapshot().tick).unwrap();
    crate::flight::equipment::tests::journey(&mut w, crate::navigation::SystemId::AVENOR);
    let tick = w.snapshot().tick;
    let review = w
        .review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::MiningLaser,
            },
            tick,
        )
        .unwrap();
    let paid = w.prepare_module(review, tick).unwrap().commit();
    assert_ne!(paid, 13608);
    assert_eq!(w.credits_tenths(), Some(52_145));
    consent(&mut w);
    launch(&mut w);
    stop(&mut w);
    let target = w.practice_snapshot().unwrap().beacons.unwrap()[1].id;
    let origin = fire(&mut w, target);
    let shot = w.request_fire_front_laser(w.snapshot().tick).unwrap();
    assert_eq!(shot.fitted_item_id, paid);
    assert!(matches!(shot.contact,PulseContact::Projectile{id,..} if id==origin.projectile));
    assert_eq!(
        w.missiles.beams[0].as_ref().unwrap().shot.fitted_item_id,
        paid
    );
    let bytes = w.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&bytes).unwrap();
    let before = live.save_bytes().unwrap();
    w.missiles.beams[0].as_mut().unwrap().shot.fitted_item_id = 13608;
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
    w = World::from_save_bytes(&bytes).unwrap();
    dock(&mut w);
    let tick = w.snapshot().tick;
    let review = w
        .review_module(
            ModuleAction::Install {
                item: 13608,
                replacing: Some(paid),
            },
            tick,
        )
        .unwrap();
    w.prepare_module(review, tick).unwrap().commit();
    assert_eq!(w.pulse_laser_snapshot().unwrap().fitted_item_id, 13608);
    let loaded = World::from_save_bytes(&w.save_bytes().unwrap()).unwrap();
    assert_eq!(
        loaded.missiles.beams[0]
            .as_ref()
            .unwrap()
            .shot
            .fitted_item_id,
        paid
    );
    assert_eq!(loaded.missile_projectiles().next().unwrap().origin, origin);
    assert_eq!(loaded.credits_tenths(), Some(52_145));
}

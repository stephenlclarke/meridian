//! Root-only actual MC1 composition tests, ALL UNRUN. Include as a child of resource_history.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::flight::missiles::tests::{dock, stop};
use crate::flight::practice::tests::{aim_at, launch, step};
use crate::{FlightInput, ModuleAction, ModuleKind};
fn fixture(seed: u64) -> World {
    let mut w = crate::flight::equipment::tests::earned_world(seed);
    w.request_refuel(w.snapshot().tick).unwrap();
    crate::flight::equipment::tests::journey(&mut w, crate::navigation::SystemId::AVENOR);
    let t = w.snapshot().tick;
    let r = w
        .review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::BeamLaser,
            },
            t,
        )
        .unwrap();
    let beam = w.prepare_module(r, t).unwrap().commit();
    launch(&mut w);
    w.request_fire_front_laser(w.snapshot().tick).unwrap();
    step(&mut w, 6);
    w.request_fire_front_laser(w.snapshot().tick).unwrap();
    step(&mut w, 180);
    dock(&mut w);
    let t = w.snapshot().tick;
    let r = w
        .review_module(ModuleAction::PurchaseMiningLaser, t)
        .unwrap();
    w.prepare_module(r, t).unwrap().commit();
    assert!(
        w.modular_equipment_snapshot()
            .unwrap()
            .items
            .iter()
            .any(|i| i.id == beam && i.slot.is_none())
    );
    launch(&mut w);
    stop(&mut w);
    let target = w.mining_belt_snapshot().unwrap().bodies[0].position;
    aim_at(&mut w, target);
    for _ in 0..1800 {
        let p = w.snapshot().position;
        let d = (f64::from(p[0]) - target.x).hypot(f64::from(p[2]) - target.z);
        if d < 1900.0 {
            stop(&mut w);
            return w;
        }
        w.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        })
        .unwrap();
        assert!(w.snapshot().motion_refusal.is_none());
    }
    panic!("actual belt approach did not complete");
}
#[test]
fn actual_fast_burst_then_paid_mining_extraction_replays_one_shared_pool_and_depletion() {
    let mut w = fixture(18101);
    let authority = w.mining_tool_authority().unwrap();
    let before = w.flight_resources_snapshot().unwrap();
    let shot = w.request_mining(authority, w.snapshot().tick).unwrap();
    let after = w.flight_resources_snapshot().unwrap();
    assert_eq!(before.energy_hundredths - after.energy_hundredths, 100);
    assert_eq!(after.heat_fifths - before.heat_fifths, 50);
    assert_eq!(after.cooldown_ticks, 18);
    assert_eq!(
        w.actual_resource_work_counts()
            .unwrap()
            .into_iter()
            .sum::<u64>(),
        1
    );
    let bytes = w.save_bytes().unwrap();
    let loaded = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(
        loaded.flight_resources_snapshot(),
        w.flight_resources_snapshot()
    );
    assert_eq!(
        loaded.mining_belt_snapshot().unwrap().bodies,
        w.mining_belt_snapshot().unwrap().bodies
    );
    assert_eq!(
        loaded.commodity_quantity(shot.commodity),
        w.commodity_quantity(shot.commodity)
    );
}
#[test]
fn removing_actual_extraction_event_cannot_hide_its_cargo_depletion_and_energy_debit() {
    let mut w = fixture(18102);
    let authority = w.mining_tool_authority().unwrap();
    w.request_mining(authority, w.snapshot().tick).unwrap();
    let bytes = w.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&bytes).unwrap();
    let before = live.save_bytes().unwrap();
    let h = w.resource_history.as_mut().unwrap();
    let index = h
        .events
        .iter()
        .position(|e| matches!(e, Event::Work { .. }))
        .unwrap();
    h.events.remove(index);
    assert!(w.save_bytes().is_err());
    assert!(
        live.restore_bytes(&w.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), before);
}

//! Actual public-command positives; direct mutation is restricted to labelled corrupt saves.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::AmmunitionAction;
use crate::flight::missiles::tests::{fly_axis, stop};
use crate::flight::practice::tests::{aim_at, consent, launch, step};
use crate::traffic::{NpcPhase, Population};

fn ready(paid: usize) -> World {
    let mut w = World::new_commander(12005).unwrap();
    for _ in 0..paid {
        let tick = w.snapshot().tick;
        let review = w
            .review_ammunition(AmmunitionAction::Purchase, tick)
            .unwrap();
        w.prepare_ammunition(review, tick).unwrap().commit();
    }
    launch(&mut w);
    stop(&mut w);
    fly_axis(&mut w, 0, -600.0, 10.0);
    for _ in 0..1000 {
        if w.traffic_snapshot()
            .unwrap()
            .actors
            .iter()
            .any(|a| a.population == Population::Trader)
        {
            return w;
        }
        step(&mut w, 1);
    }
    panic!("real finite-stock traffic birth did not occur");
}
fn trader(w: &World) -> crate::traffic::NpcSnapshot {
    w.traffic_snapshot()
        .unwrap()
        .actors
        .into_iter()
        .find(|a| a.population == Population::Trader)
        .unwrap()
}
fn fire_npc(w: &mut World) -> MissileOrigin {
    // Follow the genuine moving actor through public controls; no position, aim ray,
    // stock or NPC state is installed. Fire within the finite homing catch-up range.
    for _ in 0..12000 {
        let actor = trader(w);
        assert!(!matches!(
            actor.phase,
            NpcPhase::Destroyed | NpcPhase::Complete
        ));
        let p = w.snapshot();
        let pose = crate::flight::flight_pose(p.position, p.orientation).unwrap();
        let delta = sub(actor.pose.position, pose.position);
        let yaw = pose.orientation.z.x.atan2(pose.orientation.z.z);
        let desired = delta.x.atan2(delta.z);
        let error = (desired - yaw).sin().atan2((desired - yaw).cos());
        if length(delta) < 900.0 && unit(delta).unwrap().dot(pose.orientation.z) > 0.8 {
            w.request_npc_missile_lock(Some(actor.id), p.tick).unwrap();
            w.request_missile_arm(true, p.tick).unwrap();
            return w.request_fire_missile(p.tick).unwrap();
        }
        w.step(crate::FlightInput {
            yaw: (error / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32,
            throttle: if error.abs() < 0.3 { 1.0 } else { -1.0 },
            ..crate::FlightInput::default()
        })
        .unwrap();
    }
    panic!("ordinary pursuit did not admit finite homing range");
}
fn cold(w: &World) -> World {
    let bytes = w.save_bytes().unwrap();
    assert!(bytes.len() <= crate::save::MAX_SAVE_BYTES);
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 15);
    let loaded = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(loaded.traffic_snapshot(), w.traffic_snapshot());
    assert_eq!(
        loaded.missile_projectiles().collect::<Vec<_>>(),
        w.missile_projectiles().collect::<Vec<_>>()
    );
    assert_eq!(loaded.ammunition_snapshot(), w.ammunition_snapshot());
    assert_eq!(loaded.credits_tenths(), w.credits_tenths());
    loaded
}
fn finish_npc(w: &mut World, id: ProjectileId) -> ProjectileSnapshot {
    for _ in 0..500 {
        let p = w
            .missile_projectiles()
            .find(|p| p.origin.projectile == id)
            .unwrap();
        if p.ended.is_some() {
            return p;
        }
        step(w, 1);
    }
    panic!("finite actual projectile did not end");
}
fn impacted() -> World {
    let mut w = ready(1);
    let origin = fire_npc(&mut w);
    let ended = finish_npc(&mut w, origin.projectile);
    assert_eq!(
        ended.ended,
        Some(MissileEnd::Impact),
        "public homing missile failed to reach native NPC"
    );
    assert_eq!(ended.contact, Some(MissileContact::Npc(trader(&w).id)));
    w
}
#[test]
fn genuine_paid_missile_locks_actual_npc_and_retains_native_contact_damage_cold() {
    let mut w = ready(1);
    let before = w.ammunition_snapshot().unwrap();
    let cash = w.credits_tenths();
    let manifest = trader(&w).cargo;
    let origin = fire_npc(&mut w);
    assert_eq!(w.missile_snapshot().unwrap().lock, None);
    assert_eq!(w.missile_snapshot().unwrap().npc_lock, Some(trader(&w).id));
    let item = before.items.iter().find(|i| i.item == origin.item).unwrap();
    assert!(
        item.purchase.is_some(),
        "highest pylon must spend the actually paid round"
    );
    let mut loaded = cold(&w);
    let a = finish_npc(&mut w, origin.projectile);
    let b = finish_npc(&mut loaded, origin.projectile);
    assert_eq!(a, b);
    assert_eq!(a.contact, Some(MissileContact::Npc(trader(&w).id)));
    let hit = w.traffic.missile_hits().last().unwrap();
    assert_eq!(hit.origin, origin);
    assert_eq!(hit.point, a.position);
    let npc = trader(&w);
    assert!(
        npc.front_shield_hundredths < 30000
            || npc.rear_shield_hundredths < 30000
            || npc.energy_hundredths < 38000
    );
    assert_eq!(npc.cargo, manifest);
    assert_eq!(w.credits_tenths(), cash);
    assert!(
        w.ammunition_snapshot()
            .unwrap()
            .items
            .iter()
            .find(|i| i.item == origin.item)
            .unwrap()
            .consumed
    );
    cold(&w);
}
#[test]
fn actual_finite_seven_rounds_can_destroy_npc_without_award_loot_refill_or_respawn() {
    let mut w = ready(4);
    assert_eq!(w.credits_tenths(), Some(0));
    let cargo = trader(&w).cargo.unwrap();
    let id = trader(&w).id;
    let mut fired = 0;
    while fired < 7 && trader(&w).phase != NpcPhase::Destroyed {
        let origin = fire_npc(&mut w);
        let ended = finish_npc(&mut w, origin.projectile);
        assert_eq!(ended.contact, Some(MissileContact::Npc(id)));
        fired += 1;
        while w.missile_snapshot().unwrap().cooldown_ticks > 0 {
            step(&mut w, 1);
        }
        w = cold(&w);
    }
    assert_eq!(trader(&w).phase, NpcPhase::Destroyed);
    assert_eq!(trader(&w).energy_hundredths, 0);
    assert_eq!(trader(&w).cargo, None, "destroyed cargo is not usable loot");
    let retained = w.traffic.retained_manifest(id).unwrap();
    assert_eq!((retained.0, retained.1), cargo);
    assert!(!retained.2, "destroyed manifest cannot earn a berth sale");
    assert_eq!(w.credits_tenths(), Some(0));
    assert!(w.mission_snapshot().unwrap().freight.held_tonnes == 0);
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.request_npc_missile_lock(Some(id), w.snapshot().tick),
        Err(MissileError::NoLiveLock)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let mut loaded = cold(&w);
    for _ in 0..1000 {
        step(&mut loaded, 1);
    }
    assert_eq!(trader(&loaded).id, id);
    assert_eq!(trader(&loaded).phase, NpcPhase::Destroyed);
    assert_eq!(trader(&loaded).cargo, None);
    let retained = loaded.traffic.retained_manifest(id).unwrap();
    assert_eq!((retained.0, retained.1), cargo);
    assert!(!retained.2);
    assert_eq!(loaded.ammunition_snapshot(), w.ammunition_snapshot());
}
#[test]
fn stale_paused_foreign_and_behind_npc_commands_preserve_actual_save_and_ammunition() {
    let mut w = ready(0);
    let id = trader(&w).id;
    let foreign = trader(&ready_foreign()).id;
    let tick = w.snapshot().tick;
    let bytes = w.save_bytes().unwrap();
    assert_eq!(
        w.request_npc_missile_lock(Some(id), tick - 1),
        Err(MissileError::StaleCommand)
    );
    assert_eq!(
        w.request_npc_missile_lock(Some(foreign), tick),
        Err(MissileError::NoLiveLock)
    );
    assert_eq!(w.save_bytes().unwrap(), bytes);
    w.request_pause(true, tick).unwrap();
    let paused = w.save_bytes().unwrap();
    assert_eq!(
        w.request_npc_missile_lock(Some(id), tick),
        Err(MissileError::Paused)
    );
    assert_eq!(w.save_bytes().unwrap(), paused);
    w.request_pause(false, tick).unwrap();
    let p = w.snapshot().position;
    let t = trader(&w).pose.position;
    aim_at(
        &mut w,
        Vec3::new(
            2.0 * f64::from(p[0]) - t.x,
            0.0,
            2.0 * f64::from(p[2]) - t.z,
        ),
    );
    let tick = w.snapshot().tick;
    w.request_npc_missile_lock(Some(id), tick).unwrap();
    w.request_missile_arm(true, tick).unwrap();
    let bytes = w.save_bytes().unwrap();
    assert_eq!(
        w.request_fire_missile(tick),
        Err(MissileError::TargetBehind)
    );
    assert_eq!(w.save_bytes().unwrap(), bytes);
}
fn ready_foreign() -> World {
    let mut w = World::new_commander(12006).unwrap();
    launch(&mut w);
    stop(&mut w);
    fly_axis(&mut w, 0, -600.0, 10.0);
    for _ in 0..1000 {
        if !w.traffic_snapshot().unwrap().actors.is_empty() {
            return w;
        }
        step(&mut w, 1);
    }
    panic!("foreign actual birth missing");
}
#[test]
fn active_practice_keeps_npc_missile_admission_protected_without_proxy_beacons() {
    let mut w = World::new_commander(12005).unwrap();
    consent(&mut w);
    launch(&mut w);
    let id = trader(&ready(0)).id;
    let tick = w.snapshot().tick;
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.request_npc_missile_lock(Some(id), tick),
        Err(MissileError::ProtectedTraffic)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
}
fn refuses(valid: &World, corrupt: impl FnOnce(&mut World)) {
    let bytes = valid.save_bytes().unwrap();
    let mut bad = World::from_save_bytes(&bytes).unwrap();
    corrupt(&mut bad);
    let bad = bad.unchecked_test_save().unwrap();
    let mut live = World::from_save_bytes(&bytes).unwrap();
    let unchanged = live.save_bytes().unwrap();
    assert!(live.restore_bytes(&bad).is_err());
    assert_eq!(live.save_bytes().unwrap(), unchanged);
}
#[test]
fn removing_duplicating_or_retargeting_real_npc_missile_damage_refuses_before_restore() {
    let w = impacted();
    refuses(&w, |w| w.traffic.corrupt_missile_hits(|h| h.clear()));
    refuses(&w, |w| w.traffic.corrupt_missile_hits(|h| h.push(h[0])));
    refuses(&w, |w| {
        w.traffic.corrupt_missile_hits(|h| h[0].front = !h[0].front)
    });
    refuses(&w, |w| {
        w.traffic.corrupt_missile_hits(|h| h[0].origin.item += 1)
    });
    refuses(&w, |w| {
        w.traffic
            .corrupt_missile_hits(|h| h[0].victim_pose.position.x += 1.0)
    });
}
#[test]
fn coherent_scene_and_projectile_endpoint_poison_cannot_remove_actual_damage_receipt() {
    let w = impacted();
    refuses(&w, |w| {
        let p = w.missiles.projectiles[0].as_mut().unwrap();
        p.contact = Some(MissileContact::Station);
    });
    refuses(&w, |w| {
        w.missiles.motions[0].as_mut().unwrap().npc_scene = None
    });
    refuses(&w, |w| {
        w.missiles.npc_launches[0]
            .as_mut()
            .unwrap()
            .scene
            .bodies
            .clear()
    });
    refuses(&w, |w| {
        w.missiles.npc_launches[0].as_mut().unwrap().dock_sequence += 1
    });
    refuses(&w, |w| {
        w.missiles.projectiles[0].as_mut().unwrap().position.x += 1.0
    });
    refuses(&w, |w| {
        w.missiles.motions[0].as_mut().unwrap().player_alive = false
    });
}
#[test]
fn unsupported_draft14_and_public13_absence_keep_distinct_format_boundaries() {
    let w = World::new_commander(12005).unwrap();
    let bytes = crate::flight::persistence::v13_test_fixture(&w).unwrap();
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 13);
    let loaded = World::from_save_bytes(&bytes).unwrap();
    assert!(loaded.traffic_snapshot().is_none());
    let mut bad = w.save_bytes().unwrap();
    bad[8..10].copy_from_slice(&14u16.to_le_bytes());
    let n = bad.len() - 8;
    let checksum = crate::save::checksum(&bad[..n]);
    bad[n..].copy_from_slice(&checksum.to_le_bytes());
    assert_eq!(
        World::from_save_bytes(&bad).unwrap_err(),
        SaveError::UnsupportedVersion
    );
}

#[test]
fn full_combined_proof_budget_refuses_another_native_contact_before_mutating_traffic() {
    // Capacity-negative component fixture: duplicate an actual receipt only to fill the
    // collection. This is deliberately not a loadable or accepted gameplay history.
    let mut w = impacted();
    let origin = w.traffic.missile_hits()[0].origin;
    w.traffic.corrupt_missile_hits(|hits| {
        let h = hits[0];
        hits.resize(crate::traffic::MAX_TRAFFIC_SHOTS, h);
    });
    let actor = trader(&w);
    let scene = w.traffic.missile_scene(
        origin.owner.system,
        w.snapshot().tick,
        SceneStage::Step(origin.projectile.ordinal()),
    );
    let position = add(actor.pose.position, Vec3::new(0.0, 0.0, -500.0));
    let ray = pose(position, Vec3::new(0.0, 0.0, 1.0)).unwrap();
    let data = w.station_data.as_ref().unwrap();
    let contact = scene
        .contact(data, position, ray.orientation.z, 1000.0)
        .unwrap();
    let before = w.traffic.clone();
    assert_eq!(
        w.traffic
            .prepare_missile_contact(data, &scene, origin, ray, contact.distance, contact.id),
        Err(crate::traffic::TrafficError::ReceiptCapacity)
    );
    assert_eq!(w.traffic, before);
}

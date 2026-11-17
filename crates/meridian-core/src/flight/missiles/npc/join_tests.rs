//! Combined public histories, not synthetic journal padding or substituted contact receipts.
use super::*;
use crate::FlightInput;
use crate::flight::missiles::tests::{dock, fly_axis, stop};
use crate::flight::practice::tests::{aim_at, consent, launch, step};
use crate::station_services::Command;
use crate::traffic::{NpcPhase, Population};
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("{e:?}"))
}
fn same_cold(world: &World) -> World {
    let loaded = ok(World::from_save_bytes(&ok(world.save_bytes())));
    assert_eq!(loaded.traffic_snapshot(), world.traffic_snapshot());
    assert_eq!(loaded.combat_snapshot(), world.combat_snapshot());
    assert_eq!(
        loaded.flight_resources_snapshot(),
        world.flight_resources_snapshot()
    );
    assert_eq!(loaded.missile_snapshot(), world.missile_snapshot());
    assert_eq!(loaded.credits_tenths(), world.credits_tenths());
    assert_eq!(
        loaded.missile_projectiles().collect::<Vec<_>>(),
        world.missile_projectiles().collect::<Vec<_>>()
    );
    loaded
}
fn finish_projectile(world: &mut World, id: ProjectileId) -> ProjectileSnapshot {
    // All five genuinely owned rounds participate; the legacy snapshot projects only three.
    // Keep the same finite 500-step bound as the original fixture helper.
    for _ in 0..500 {
        let projectile = world
            .missile_projectiles()
            .find(|p| p.origin.projectile == id)
            .unwrap_or_else(|| panic!("missing actual projectile"));
        if projectile.ended.is_some() {
            return projectile;
        }
        step(world, 1);
    }
    panic!("finite actual projectile did not end");
}
fn fire_pirate(world: &mut World) -> MissileOrigin {
    for _ in 0..1200 {
        let target = world
            .traffic_snapshot()
            .unwrap_or_else(|| panic!("traffic"))
            .actors
            .into_iter()
            .find(|a| a.population == Population::Pirate)
            .unwrap_or_else(|| panic!("actual pirate"));
        assert!(!matches!(
            target.phase,
            NpcPhase::Destroyed | NpcPhase::Complete
        ));
        let state = world.snapshot();
        let pose = super::super::super::flight_pose(state.position, state.orientation)
            .unwrap_or_else(|| panic!("player pose"));
        let delta = sub(target.pose.position, pose.position);
        let desired = delta.x.atan2(delta.z);
        let yaw = pose.orientation.z.x.atan2(pose.orientation.z.z);
        let error = (desired - yaw).sin().atan2((desired - yaw).cos());
        if length(delta) < 5500.0
            && unit(delta).is_some_and(|d| d.dot(pose.orientation.z) > 0.8)
            && world
                .missile_snapshot()
                .is_some_and(|m| m.cooldown_ticks == 0)
        {
            ok(world.request_npc_missile_lock(Some(target.id), state.tick));
            ok(world.request_missile_arm(true, state.tick));
            return ok(world.request_fire_missile(state.tick));
        }
        ok(world.step(FlightInput {
            yaw: (error / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32,
            brake: true,
            ..FlightInput::default()
        }));
    }
    panic!("actual pirate could not be faced and fired on through public controls");
}
#[test]
fn mixed_real_npc_pulse_owned_missiles_player_pulse_and_berth_replay_cold_once() {
    let mut world = ok(World::new_commander(12001));
    for _ in 0..2 {
        let tick = world.snapshot().tick;
        let review = ok(world.review_ammunition(crate::AmmunitionAction::Purchase, tick));
        ok(world.prepare_ammunition(review, tick)).commit();
    }
    ok(world.request_station_service(Command::BuyFood, world.snapshot().tick));
    launch(&mut world);
    stop(&mut world);
    fly_axis(&mut world, 0, -600.0, 10.0);
    let mut contacted = false;
    for _ in 0..24000 {
        ok(world.step(FlightInput::default()));
        if world
            .traffic_snapshot()
            .is_some_and(|s| s.last_player_hit.is_some())
        {
            contacted = true;
            break;
        }
    }
    assert!(contacted, "no actual incoming NPC Pulse");
    let cash = world.credits_tenths();
    assert_eq!(cash, Some(3374));
    ok(world.request_fire_front_pulse(world.snapshot().tick));
    let mut saved = same_cold(&world);
    for _ in 0..5 {
        let origin = fire_pirate(&mut saved);
        let end = finish_projectile(&mut saved, origin.projectile);
        assert!(
            matches!(end.contact, Some(MissileContact::Npc(_))),
            "actual projectile did not reach native pirate"
        );
        saved = same_cold(&saved);
        if saved.traffic_snapshot().is_some_and(|s| {
            s.actors
                .iter()
                .any(|a| a.population == Population::Pirate && a.phase == NpcPhase::Destroyed)
        }) {
            break;
        }
    }
    assert!(saved.traffic_snapshot().is_some_and(|s| {
        s.actors
            .iter()
            .any(|a| a.population == Population::Pirate && a.phase == NpcPhase::Destroyed)
    }));
    assert_eq!(saved.credits_tenths(), cash);
    assert_eq!(
        saved.commodity_quantity(crate::station_services::CommodityId::Food),
        Some(1)
    );
    assert!(!saved.traffic.missile_hits().is_empty());
    assert!(!saved.traffic.player_hits().is_empty());
    assert_eq!(
        saved
            .combat_snapshot()
            .unwrap_or_else(|| panic!("combat"))
            .last_ordinary_impact,
        None
    );
    // Combat began 600 units beside the launch lane. Translate onto the aperture
    // centreline in clear exterior space before the final inward half-turn.
    // Low public lateral speed bounds braking overshoot without installing a pose.
    fly_axis(&mut saved, 0, 0.0, 1.0);
    assert!(saved.snapshot().position[0].abs() < 2.0);
    dock(&mut saved);
    assert_eq!(saved.snapshot().mode, FlightMode::Docked);
    let after = same_cold(&saved);
    let player = after
        .combat_snapshot()
        .unwrap_or_else(|| panic!("combat"))
        .player;
    assert_eq!(player.front_shield_hundredths, 30000);
    assert_eq!(player.rear_shield_hundredths, 30000);
    assert_eq!(player.energy_hundredths, 38000);
    assert_eq!(after.credits_tenths(), cash);
}
#[test]
fn genuinely_earned_near_full_journal_refuses_next_growing_missile_scene_atomically() {
    let mut world = ok(World::new_commander(12005));
    consent(&mut world);
    launch(&mut world);
    stop(&mut world);
    aim_at(&mut world, Vec3::new(0.0, 0.0, 20000.0));
    for _ in 0..(crate::save::MAX_SAVE_BYTES / 9) {
        let bytes = ok(world.save_bytes());
        if bytes.len() > crate::save::MAX_SAVE_BYTES - 2048 {
            // This is a cold continuation of an entirely public-command history.
            let mut trial = ok(World::from_save_bytes(&bytes));
            let beacon = trial
                .practice_snapshot()
                .and_then(|p| p.beacons)
                .unwrap_or_else(|| panic!("beacons"))[0];
            aim_at(&mut trial, beacon.pose.position);
            let tick = trial.snapshot().tick;
            ok(trial.request_missile_lock(Some(beacon.id), tick));
            ok(trial.request_missile_arm(true, tick));
            let origin = ok(trial.request_fire_missile(tick));
            let before = ok(trial.save_bytes());
            let result = trial.step(FlightInput::default());
            if matches!(result, Err(StepError::TickOverflow)) {
                assert_eq!(ok(trial.save_bytes()), before);
                assert_eq!(trial.snapshot().tick, tick);
                assert!(
                    trial
                        .missile_projectiles()
                        .any(|p| p.origin == origin && p.age_ticks == 0)
                );
                same_cold(&trial);
                return;
            }
            ok(result);
        }
        let tick = world.snapshot().tick;
        ok(world.request_fire_front_pulse(tick));
        step(&mut world, 50);
    }
    panic!(
        "valid public journal never reached a launch-admitted, motion-refused whole-save boundary"
    );
}

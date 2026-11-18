//! Real host commands select World-owned contacts; a scanner dot grants no lock authority.
use super::{Action, Session};
use meridian_core::{FlightInput, World};
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("{e:?}"))
}
fn contacts() -> World {
    let mut world = ok(World::new_commander(12001));
    ok(world.request_launch(world.snapshot().tick));
    for _ in 0..1600 {
        ok(world.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        }));
        if world
            .traffic_snapshot()
            .is_some_and(|s| s.actors.len() == 2)
        {
            return world;
        }
    }
    panic!("public flight did not produce the actual two ship contacts");
}
#[test]
fn ship_lock_cycle_arm_safe_and_public_cold_state_use_actual_contact_ids() {
    let mut world = contacts();
    let mut session = Session::default();
    let ids: Vec<_> = world
        .traffic_snapshot()
        .unwrap_or_else(|| panic!("traffic"))
        .actors
        .into_iter()
        .map(|a| a.id)
        .collect();
    let rounds = world.missile_rounds();
    let credits = world.credits_tenths();
    let notice = ok(session.command(&mut world, Action::CycleMissileLock));
    assert_eq!(
        notice,
        format!("Missile locked on ship {}", ids[0].serial())
    );
    assert!(session.lines(&world, true, Some(&notice)).contains(&notice));
    assert_eq!(
        world
            .missile_snapshot()
            .unwrap_or_else(|| panic!("missiles"))
            .npc_lock,
        Some(ids[0])
    );
    ok(session.command(&mut world, Action::MissionDetails));
    assert!(
        session
            .lines(&world, true, None)
            .iter()
            .any(|s| s.contains("LOCK SHIP 1"))
    );
    assert!(
        ok(crate::hud::cockpit::Cockpit::capture(
            &world,
            world.snapshot()
        ))
        .locked
    );
    ok(session.command(&mut world, Action::ToggleMissileArm));
    let continued = ok(World::from_save_bytes(&ok(world.save_bytes())));
    let missile = continued
        .missile_snapshot()
        .unwrap_or_else(|| panic!("missiles"));
    assert!(missile.armed);
    assert_eq!(missile.npc_lock, Some(ids[0]));
    assert!(missile.lock.is_none());
    ok(session.command(&mut world, Action::CycleMissileLock));
    assert_eq!(
        world
            .missile_snapshot()
            .unwrap_or_else(|| panic!("missiles"))
            .npc_lock,
        Some(ids[1])
    );
    ok(session.command(&mut world, Action::ClearMissile));
    let cleared = world
        .missile_snapshot()
        .unwrap_or_else(|| panic!("missiles"));
    assert!(cleared.npc_lock.is_none() && !cleared.armed);
    assert!(
        !ok(crate::hud::cockpit::Cockpit::capture(
            &world,
            world.snapshot()
        ))
        .locked
    );
    assert_eq!(world.missile_rounds(), rounds);
    assert_eq!(world.credits_tenths(), credits);
}
#[test]
fn paused_host_lock_attempt_preserves_actual_world_and_owned_rounds() {
    let mut world = contacts();
    let mut session = Session::default();
    let tick = world.snapshot().tick;
    ok(world.request_pause(true, tick));
    let before = ok(world.save_bytes());
    assert!(
        session
            .command(&mut world, Action::CycleMissileLock)
            .is_err()
    );
    assert_eq!(ok(world.save_bytes()), before);
}
#[test]
fn no_live_ship_contact_creates_no_lock_or_beacon_alias() {
    let mut world = ok(World::new_commander(12005));
    let mut session = Session::default();
    let before = ok(world.save_bytes());
    assert!(
        session
            .command(&mut world, Action::CycleMissileLock)
            .is_err()
    );
    let state = world
        .missile_snapshot()
        .unwrap_or_else(|| panic!("missiles"));
    assert!(state.lock.is_none() && state.npc_lock.is_none());
    assert_eq!(ok(world.save_bytes()), before);
}

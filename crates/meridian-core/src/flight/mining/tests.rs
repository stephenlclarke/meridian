//! Real World command paths. Internal negative fixtures never grant a paid mining tool.
use super::*;
use crate::flight::missiles::tests::stop;
use crate::flight::practice::tests::{aim_at, launch};
use crate::{FlightInput, MotionRefusal, PulseContact};

pub(super) fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("unexpected refusal: {error:?}"),
    }
}
pub(super) fn some<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => panic!("missing actual World projection"),
    }
}
pub(super) fn approach(world: &mut World) {
    launch(world);
    stop(world);
    let target = some(world.mining_belt_snapshot()).bodies[0].position;
    aim_at(world, target);
    for _ in 0..1800 {
        let p = world.snapshot().position;
        let range = (f64::from(p[0]) - target.x).hypot(f64::from(p[2]) - target.z);
        if range < 1900.0 {
            stop(world);
            assert!(some(world.mining_belt_snapshot()).front_contact.is_some());
            return;
        }
        ok(world.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        }));
        assert!(world.snapshot().motion_refusal.is_none());
    }
    panic!("ordinary flight never approached the authoritative mining destination");
}
fn bad_tool(world: &World) -> MiningToolAuthority {
    let p = ok(world.raw_service_projection());
    MiningToolAuthority {
        owner: p.owner + 1,
        hull: p.hull,
        account_revision: p.revision,
        item_id: 0,
        epoch: some(world.freight_session.as_ref()).epoch(),
        scope: std::sync::Arc::downgrade(&world.mining_scope),
        current_item: |_| None,
    }
}

#[test]
fn actual_navigation_provider_and_cold_continue_retain_identical_body_identity_mesh_and_stock() {
    let mut world = ok(World::new_commander(1101));
    let destination = some(world.mining_destination(SystemId::AVENOR));
    assert_eq!(destination.position, crate::mining::BELT_CENTRE);
    assert!(world.mining_destination(SystemId::RENAS).is_none());
    let bodies = some(world.mining_belt_snapshot()).bodies;
    for _ in 0..10 {
        ok(world.step(FlightInput::default()));
    }
    let bytes = ok(world.save_bytes());
    assert_eq!(
        u16::from_le_bytes([bytes[8], bytes[9]]),
        crate::save::SAVE_VERSION
    );
    let cold = ok(World::from_save_bytes(&bytes));
    assert_eq!(some(cold.mining_belt_snapshot()).bodies, bodies);
    assert_eq!(some(cold.mining_destination(SystemId::AVENOR)), destination);
    assert_eq!(cold.credits_tenths(), world.credits_tenths());
    assert_eq!(cold.service_snapshot(), world.service_snapshot());
    assert!(World::new().mining_belt_snapshot().is_none());
}

#[test]
fn real_approach_ray_and_pulse_hit_the_shared_body_without_claiming_ore_or_money() {
    let mut world = ok(World::new_commander(1102));
    approach(&mut world);
    let before = some(world.mining_belt_snapshot());
    let contact = some(before.front_contact);
    assert!(contact.distance <= MINING_RANGE);
    assert_eq!(contact.asteroid, before.bodies[0].id);
    let cargo = world.service_snapshot();
    let cash = world.credits_tenths();
    let tick = world.snapshot().tick;
    let shot = ok(world.request_fire_front_pulse(tick));
    assert!(
        matches!(shot.contact,PulseContact::AsteroidBlocked{id,distance} if id==contact.asteroid && distance==contact.distance)
    );
    assert_eq!(some(world.mining_belt_snapshot()).bodies, before.bodies);
    assert_eq!(world.service_snapshot(), cargo);
    assert_eq!(world.credits_tenths(), cash);
    let cold = ok(World::from_save_bytes(&ok(world.save_bytes())));
    assert_eq!(some(cold.mining_belt_snapshot()).bodies, before.bodies);
}

#[test]
fn real_flight_stops_at_asteroid_hull_contact_and_cannot_tunnel_through_render_geometry() {
    let mut world = ok(World::new_commander(1103));
    approach(&mut world);
    let mut touched = false;
    for _ in 0..600 {
        let before = world.snapshot();
        ok(world.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        }));
        if world.snapshot().motion_refusal == Some(MotionRefusal::AsteroidContact) {
            assert_eq!(world.snapshot().position, before.position);
            assert_eq!(world.snapshot().speed, 0.0);
            touched = true;
            break;
        }
    }
    assert!(touched);
    let pose = some(flight_pose(world.state.position, world.state.orientation));
    assert!(world.mining_motion_clear(pose, pose, 0.0));
    assert!(World::from_save_bytes(&ok(world.save_bytes())).is_ok());
}

#[test]
fn stale_paused_docked_foreign_and_missing_item_work_refuse_without_any_state_or_cargo_effect() {
    let mut world = ok(World::new_commander(1104));
    let before = ok(world.save_bytes());
    let tool = bad_tool(&world);
    assert_eq!(world.request_mining(tool, 0), Err(MiningError::NotInFlight));
    assert_eq!(ok(world.save_bytes()), before);
    launch(&mut world);
    let tick = world.snapshot().tick;
    let before = ok(world.save_bytes());
    let tool = bad_tool(&world);
    assert_eq!(
        world.request_mining(tool, tick - 1),
        Err(MiningError::StaleCommand)
    );
    let tool = bad_tool(&world);
    assert_eq!(
        world.request_mining(tool, tick),
        Err(MiningError::NoCompatibleFittedTool)
    );
    assert_eq!(ok(world.save_bytes()), before);
    ok(world.request_pause(true, tick));
    let paused = ok(world.save_bytes());
    let tool = bad_tool(&world);
    assert_eq!(world.request_mining(tool, tick), Err(MiningError::Paused));
    assert_eq!(ok(world.save_bytes()), paused);
}

#[test]
fn depletion_without_actual_account_yield_or_ship_discharge_is_rejected_on_save_and_restore() {
    let mut world = ok(World::new_commander(1105));
    let pristine = ok(world.save_bytes());
    let id = some(world.mining_belt_snapshot()).bodies[0].id;
    world.mining = ok(world.mining.prepare_extract(id)).0; // Deliberately inconsistent negative fixture.
    assert_eq!(
        world.save_bytes(),
        Err(crate::save::SaveError::InvalidState(
            "mining yield without ship discharge"
        ))
    );
    let corrupt = ok(world.unchecked_test_save());
    let mut live = ok(World::from_save_bytes(&pristine));
    let before = ok(live.save_bytes());
    assert_eq!(
        live.restore_bytes(&corrupt),
        Err(crate::save::SaveError::InvalidState(
            "mining yield without ship discharge"
        ))
    );
    assert_eq!(ok(live.save_bytes()), before);
    let mut corrupt = pristine.clone();
    let index = corrupt.len() - 8 - 24;
    corrupt[index] = 255;
    let digest = crate::save::checksum(&corrupt[..corrupt.len() - 8]);
    let end = corrupt.len();
    corrupt[end - 8..].copy_from_slice(&digest.to_le_bytes());
    assert_eq!(
        live.restore_bytes(&corrupt),
        Err(crate::save::SaveError::InvalidState(
            "mining body identity/depletion conservation"
        ))
    );
    assert_eq!(ok(live.save_bytes()), before);
}

#[test]
fn independent_continues_of_one_file_cannot_share_a_numeric_tool_capability() {
    let mut source = ok(World::new_commander(1106));
    launch(&mut source);
    let saved = ok(source.save_bytes());
    let old = ok(World::from_save_bytes(&saved));
    let mut current = ok(World::from_save_bytes(&saved));
    let projection = ok(old.raw_service_projection());
    assert_eq!(projection, ok(current.raw_service_projection()));
    assert_eq!(
        some(old.freight_session.as_ref()).epoch(),
        some(current.freight_session.as_ref()).epoch()
    );
    // Internal negative fixture only. This must refuse before any fitted-item lookup, even if
    // all saved numeric IDs and both post-Continue numeric epochs are identical.
    let stale = MiningToolAuthority {
        owner: projection.owner,
        hull: projection.hull,
        account_revision: projection.revision,
        item_id: projection.owner + 7,
        epoch: some(old.freight_session.as_ref()).epoch(),
        scope: std::sync::Arc::downgrade(&old.mining_scope),
        current_item: |_| panic!("foreign live scope reached fitted-item lookup"),
    };
    let before = ok(current.save_bytes());
    let tick = current.snapshot().tick;
    assert_eq!(
        current.request_mining(stale, tick),
        Err(MiningError::NoCompatibleFittedTool)
    );
    assert_eq!(ok(current.save_bytes()), before);
}

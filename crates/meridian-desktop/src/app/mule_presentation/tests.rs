use super::current_placement;
use crate::{mesh, mule_render::Placement};
use meridian_core::{
    FlightInput, FlightMode, FlightStart, World,
    freight::g02::{Choice, Command, Event},
    navigation::{BASIC_SPOOL_TICKS, DockOrigin, SystemId},
};

fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}

fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("expected actual World publication"))
}

fn start_flight(world: &mut World) {
    ok(world.request_launch(world.snapshot().tick));
    ok(world.step(FlightInput::default()));
    assert_eq!(world.snapshot().mode, FlightMode::Flight);
}

fn dock_at_renas(world: &mut World) {
    start_flight(world);
    ok(world.request_jump(SystemId::RENAS, world.snapshot().tick));
    for _ in 0..BASIC_SPOOL_TICKS {
        ok(world.step(FlightInput::default()));
    }
    for _ in 0..9000 {
        let snapshot = world.snapshot();
        if snapshot.mode == FlightMode::Docked {
            assert_eq!(some(snapshot.navigation).system, SystemId::RENAS);
            assert_eq!(
                some(world.current_dock_context()).origin(),
                DockOrigin::PhysicalEntry
            );
            return;
        }
        let speed = if snapshot.position[2] > 2500.0 {
            34.3
        } else {
            4.0
        };
        ok(world.step(FlightInput {
            throttle: ((speed - snapshot.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
            ..FlightInput::default()
        }));
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    panic!("public flight did not dock at Renas");
}

#[test]
fn the_paid_freight_sortie_adds_mule_to_the_existing_camera_and_flight_mesh() {
    let mut world = ok(World::new_commander(16903));
    assert!(current_placement(&world, world.snapshot(), None).is_none());
    let issuer = world.station_contacts()[0];
    ok(world.request_contact(issuer, world.snapshot().tick));
    let Event::Reviewed(review) = ok(world.request_freight(
        Command::ReviewOffer {
            actor: issuer.actor(),
        },
        world.snapshot().tick,
    )) else {
        panic!("actual G02 offer")
    };
    let Event::Accepted { job, units } = ok(world.request_freight(
        Command::Confirm {
            review,
            choice: Choice::Accept,
        },
        world.snapshot().tick,
    )) else {
        panic!("actual G02 load")
    };
    assert_eq!(units.len(), 8);
    dock_at_renas(&mut world);

    let receiver = world.station_contacts()[0];
    ok(world.request_contact(receiver, world.snapshot().tick));
    let Event::Reviewed(review) = ok(world.request_freight(
        Command::ReviewReceiving {
            actor: receiver.actor(),
            job,
        },
        world.snapshot().tick,
    )) else {
        panic!("actual G02 receiving review")
    };
    let cash = world.credits_tenths();
    assert!(matches!(
        ok(world.request_freight(
            Command::Confirm {
                review,
                choice: Choice::ReceiveAll
            },
            world.snapshot().tick,
        )),
        Event::Received {
            remaining: 0,
            completion: Some(_),
            clue_awarded: true,
            ..
        }
    ));
    assert_eq!(world.credits_tenths(), cash.map(|value| value + 7300));
    assert_eq!(some(world.mission_snapshot()).freight.held_tonnes, 0);
    assert!(world.mule_actor_snapshot().is_none());
    start_flight(&mut world);
    let snapshot = world.snapshot();
    let frame = some(world.mule_actor_snapshot());
    let placement = some(current_placement(&world, snapshot, Some(&frame)));
    assert!(matches!(placement, Placement::Ordinary { .. }));
    assert!(placement.internal_observer().is_none());
    assert!(world.mule_observer_pose(&frame).is_none());
    assert_eq!(frame.actor().hull_id, "SH001");
    assert_eq!(frame.crew_aboard(), Some(2));

    let ship = ok(mesh::load_ship());
    let station_vertices = ok(mesh::load_station());
    let station = mesh::StationRender {
        vertices: &station_vertices,
        pose: some(snapshot.station),
        port: some(world.station_port()),
        docked: false,
    };
    let camera = ok(mesh::scene_camera_pose(&snapshot, station));
    let scene = ok(placement.for_camera(|| mesh::scene_camera_pose(&snapshot, station)));
    assert_eq!(scene.observer, camera);
    assert_ne!(scene.observer.position.z as f32, snapshot.position[2]);
    assert_eq!(scene.actor.id, frame.actor().id);
    assert!(std::ptr::eq(scene.envelope, frame.presentation().as_ref()));

    let mut ordinary = Vec::new();
    ok(mesh::frame_vertices(
        &snapshot,
        &ship,
        Some(station),
        &mut ordinary,
    ));
    let mut with_mule = Vec::new();
    ok(mesh::frame_vertices_for_observer(
        &snapshot,
        &ship,
        Some(station),
        placement.internal_observer(),
        &mut with_mule,
    ));
    assert_eq!(ordinary.len(), ship.len() + station_vertices.len());
    if let Some(traffic) = world.traffic_snapshot() {
        ok(mesh::traffic::append(
            &snapshot,
            station,
            &ship,
            &traffic,
            &mut ordinary,
        ));
        ok(mesh::traffic::append(
            &snapshot,
            station,
            &ship,
            &traffic,
            &mut with_mule,
        ));
    }
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&ordinary),
        bytemuck::cast_slice::<_, u8>(&with_mule)
    );
    let cockpit = ok(crate::hud::cockpit::Cockpit::capture(&world, snapshot));
    assert_eq!(cockpit.flight, snapshot);
    let mut wrong_tick = snapshot;
    wrong_tick.tick += 1;
    assert!(current_placement(&world, wrong_tick, Some(&frame)).is_none());

    ok(world.step(FlightInput::default()));
    assert!(current_placement(&world, world.snapshot(), Some(&frame)).is_none());
    assert!(current_placement(&world, world.snapshot(), None).is_none());
    let current = some(world.mule_actor_snapshot());
    assert!(current_placement(&world, world.snapshot(), Some(&current)).is_some());
    let foreign = ok(World::new_commander(16904));
    assert!(current_placement(&foreign, world.snapshot(), Some(&current)).is_none());
}

#[test]
fn isolated_frame_keeps_its_world_owned_observer_and_rejects_stale_publications() {
    let mut world = ok(World::mule_validation_scenario(
        917,
        FlightStart {
            position: [0.0, 0.0, 16000.0],
            orientation: [0.0, 1.0, 0.0, 0.0],
            speed: 0.0,
        },
    ));
    let snapshot = world.snapshot();
    let frame = some(world.mule_actor_snapshot());
    let observer = some(world.mule_observer_pose(&frame));
    let placement = some(current_placement(&world, snapshot, Some(&frame)));
    assert_eq!(placement.internal_observer(), Some(observer));
    let visual = ok(placement.for_camera(|| panic!("isolated scene must use the World observer")));
    assert_eq!(visual.observer, observer);
    let ship = ok(mesh::load_ship());
    let station_vertices = ok(mesh::load_station());
    let station = mesh::StationRender {
        vertices: &station_vertices,
        pose: some(snapshot.station),
        port: some(world.station_port()),
        docked: false,
    };
    let mut frame_vertices = Vec::new();
    ok(mesh::frame_vertices_for_observer(
        &snapshot,
        &ship,
        Some(station),
        placement.internal_observer(),
        &mut frame_vertices,
    ));
    assert_eq!(frame_vertices.len(), station_vertices.len());
    ok(world.step(FlightInput::default()));
    assert!(current_placement(&world, world.snapshot(), Some(&frame)).is_none());
}

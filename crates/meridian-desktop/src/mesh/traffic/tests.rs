//! Geometry fixtures are descriptive projections, never World admission or damage evidence.
use super::{append, contains_pulse, vertex_capacity};
use crate::mesh::{StationRender, Vertex, apply_flight_view, camera_for, load_ship, load_station};
use meridian_core::docking::{Basis3, RigidPose, Vec3};
use meridian_core::traffic::{
    BeamSource, MAX_TRAFFIC_ACTORS, NpcPhase, Population, TrafficBeam, TrafficSnapshot,
};
use meridian_core::{
    FlightInput, FlightMode, FlightView, PulseContact, PulseShot, Snapshot, World,
};
use std::sync::OnceLock;

fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|error| panic!("{error:?}"))
}

// Real public launch and movement supply identities; no synthetic NpcId or spawn setter.
fn admitted() -> &'static (Snapshot, TrafficSnapshot) {
    static SCENE: OnceLock<(Snapshot, TrafficSnapshot)> = OnceLock::new();
    SCENE.get_or_init(|| {
        let mut world = ok(World::new_commander(12001));
        ok(world.request_launch(world.snapshot().tick));
        for _ in 0..1600 {
            ok(world.step(FlightInput {
                throttle: 1.0,
                ..FlightInput::default()
            }));
            let scene = world
                .traffic_snapshot()
                .unwrap_or_else(|| panic!("traffic"));
            if scene.actors.len() == 2 {
                assert_eq!(world.snapshot().mode, FlightMode::Flight);
                assert!(
                    scene
                        .actors
                        .iter()
                        .any(|a| a.population == Population::Trader)
                );
                assert!(
                    scene
                        .actors
                        .iter()
                        .any(|a| a.population == Population::Pirate)
                );
                return (world.snapshot(), scene);
            }
        }
        panic!("public flight never admitted the two actual SH025 bodies");
    })
}

fn station<'a>(
    snapshot: &Snapshot,
    port: &'a meridian_core::docking::DockingPort,
) -> StationRender<'a> {
    StationRender {
        vertices: &[],
        pose: snapshot.station.unwrap_or_else(|| panic!("station")),
        port,
        docked: false,
    }
}

fn ports() -> meridian_core::World {
    ok(World::at_original_station())
}

fn same(a: &[Vertex], b: &[Vertex]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.position, b.position);
        assert_eq!(a.color, b.color);
    }
}

fn close(a: f32, b: f64) {
    assert!((f64::from(a) - b).abs() < 0.001, "{a} != {b}");
}

#[test]
fn actual_world_poses_render_every_source_triangle_and_color_without_mutating_projection() {
    let (snapshot, scene) = admitted();
    let unchanged = scene.clone();
    let ship = ok(load_ship());
    let world = ports();
    let station = station(
        snapshot,
        world.station_port().unwrap_or_else(|| panic!("port")),
    );
    let camera = ok(camera_for(snapshot, station));
    let mut scene = scene.clone();
    scene.last_beam = None;
    let mut output = Vec::new();
    ok(append(snapshot, station, &ship, &scene, &mut output));
    assert_eq!(output.len(), ship.len() * 2);
    for (actor, body) in scene.actors.iter().zip(output.chunks_exact(ship.len())) {
        assert_eq!(actor.hull_id, "SH025");
        for (source, actual) in ship.iter().zip(body) {
            let point = actor.pose.orientation.apply(Vec3::new(
                source.position[0].into(),
                source.position[1].into(),
                source.position[2].into(),
            ));
            let delta = Vec3::new(
                point.x + actor.pose.position.x - camera.pose.position.x,
                point.y + actor.pose.position.y - camera.pose.position.y,
                point.z + actor.pose.position.z - camera.pose.position.z,
            );
            for (actual, axis) in actual.position.into_iter().zip([
                camera.pose.orientation.x,
                camera.pose.orientation.y,
                camera.pose.orientation.z,
            ]) {
                close(
                    actual,
                    axis.x * delta.x + axis.y * delta.y + axis.z * delta.z,
                );
            }
            assert_eq!(source.color, actual.color);
        }
    }
    assert_eq!(admitted().1, unchanged);
}

#[test]
fn rotated_translated_pose_uses_full_native_basis_not_only_yaw_or_origin() {
    let (snapshot, scene) = admitted();
    let mut scene = scene.clone();
    scene.actors.truncate(1);
    scene.last_beam = None;
    scene.actors[0].pose = RigidPose {
        position: Vec3::new(123.0, -87.0, 2100.0),
        orientation: Basis3 {
            x: Vec3::new(0.0, 1.0, 0.0),
            y: Vec3::new(0.0, 0.0, 1.0),
            z: Vec3::new(1.0, 0.0, 0.0),
        },
    };
    let ship = ok(load_ship());
    let world = ports();
    let station = station(
        snapshot,
        world.station_port().unwrap_or_else(|| panic!("port")),
    );
    let camera = ok(camera_for(snapshot, station));
    let mut output = Vec::new();
    ok(append(snapshot, station, &ship, &scene, &mut output));
    for (v, actual) in ship.iter().zip(output) {
        let expected = camera.pose.orientation.inverse_coordinates(Vec3::new(
            123.0 + f64::from(v.position[2]) - camera.pose.position.x,
            -87.0 + f64::from(v.position[0]) - camera.pose.position.y,
            2100.0 + f64::from(v.position[1]) - camera.pose.position.z,
        ));
        close(actual.position[0], expected.x);
        close(actual.position[1], expected.y);
        close(actual.position[2], expected.z);
    }
}

#[test]
fn all_airborne_phases_and_destroyed_solids_remain_complete_only_is_hidden() {
    let (snapshot, scene) = admitted();
    let mut scene = scene.clone();
    scene.actors.truncate(1);
    scene.last_beam = None;
    let ship = ok(load_ship());
    let world = ports();
    let station = station(
        snapshot,
        world.station_port().unwrap_or_else(|| panic!("port")),
    );
    let mut retained = Vec::new();
    scene.actors[0].phase = NpcPhase::Destroyed;
    ok(append(snapshot, station, &ship, &scene, &mut retained));
    for phase in [
        NpcPhase::Departing,
        NpcPhase::Outbound,
        NpcPhase::ReturnAlign,
        NpcPhase::Inbound,
        NpcPhase::Approach,
        NpcPhase::FinalAlign,
        NpcPhase::Hunting,
        NpcPhase::Retreating,
        NpcPhase::Destroyed,
        NpcPhase::Complete,
    ] {
        scene.actors[0].phase = phase;
        let mut output = Vec::new();
        ok(append(snapshot, station, &ship, &scene, &mut output));
        if phase == NpcPhase::Complete {
            assert!(output.is_empty());
        } else {
            same(&output, &retained);
        }
    }
}

#[test]
fn each_flight_view_rotates_full_hull_and_preserves_forward_depth_for_its_cardinal_body() {
    let (snapshot, scene) = admitted();
    let mut snapshot = *snapshot;
    // Camera-only projection fixture; no World pose or flight state is changed.
    snapshot.position = [100.0, 200.0, 5000.0];
    snapshot.orientation = [0.0, 0.0, 0.0, 1.0];
    let world = ports();
    let station = station(
        &snapshot,
        world.station_port().unwrap_or_else(|| panic!("port")),
    );
    let camera = ok(camera_for(&snapshot, station));
    let ship = ok(load_ship());
    let mut scene = scene.clone();
    scene.actors.truncate(1);
    scene.last_beam = None;
    for (view, offset) in [
        (FlightView::Front, [0.0, 0.0, 1500.0]),
        (FlightView::Rear, [0.0, 0.0, -1500.0]),
        (FlightView::Left, [-1500.0, 0.0, 0.0]),
        (FlightView::Right, [1500.0, 0.0, 0.0]),
    ] {
        scene.actors[0].pose.position = Vec3::new(
            camera.pose.position.x + offset[0],
            camera.pose.position.y + offset[1],
            camera.pose.position.z + offset[2],
        );
        let mut output = Vec::new();
        ok(append(&snapshot, station, &ship, &scene, &mut output));
        apply_flight_view(view, &mut output);
        assert_eq!(output.len(), ship.len());
        for v in output {
            assert!(v.position[2] > 1.0 && v.position[2] < 20000.0);
            // Same near=1/far=20000 homogeneous projection as flight.wgsl.
            let depth = (v.position[2] * 20000.0 / 19999.0 - 20000.0 / 19999.0) / v.position[2];
            assert!((0.0..1.0).contains(&depth));
        }
    }
}

fn beam_fixture() -> (Snapshot, TrafficSnapshot) {
    let (snapshot, scene) = admitted();
    let mut scene = scene.clone();
    // Explicit retained-segment fixture with real descriptive IDs, not a simulated hit.
    scene.last_beam = Some(TrafficBeam {
        source: BeamSource::Npc(scene.actors[1].id),
        victim: scene.actors[0].id,
        world_tick: snapshot.tick,
        local_tick: 1,
        origin: Vec3::new(30.0, 20.0, 1200.0),
        direction: Vec3::new(0.0, 0.0, 1.0),
        point: Vec3::new(130.0, 35.0, 1700.0),
        victim_pose: scene.actors[0].pose,
        front: true,
    });
    for actor in &mut scene.actors {
        actor.phase = NpcPhase::Complete;
    }
    (*snapshot, scene)
}

#[test]
fn retained_beam_ends_at_recorded_point_even_after_victim_pose_moves() {
    let (snapshot, mut scene) = beam_fixture();
    let ship = ok(load_ship());
    let world = ports();
    let station = station(
        &snapshot,
        world.station_port().unwrap_or_else(|| panic!("port")),
    );
    let camera = ok(camera_for(&snapshot, station));
    let beam = scene.last_beam.unwrap_or_else(|| panic!("beam"));
    let mut output = Vec::new();
    ok(append(&snapshot, station, &ship, &scene, &mut output));
    assert_eq!(output.len(), crate::mesh::PULSE_VERTEX_COUNT);
    for (indices, endpoint) in [([0, 1], beam.origin), ([2, 5], beam.point)] {
        let v = camera.pose.orientation.inverse_coordinates(Vec3::new(
            endpoint.x - camera.pose.position.x,
            endpoint.y - camera.pose.position.y,
            endpoint.z - camera.pose.position.z,
        ));
        for axis in 0..3 {
            close(
                (output[indices[0]].position[axis] + output[indices[1]].position[axis]) / 2.0,
                [v.x, v.y, v.z][axis],
            );
        }
    }
    scene.actors[0].pose.position = Vec3::new(9000.0, -8000.0, 7000.0);
    let mut moved = Vec::new();
    ok(append(&snapshot, station, &ship, &scene, &mut moved));
    same(&output, &moved);
    for view in [
        FlightView::Front,
        FlightView::Rear,
        FlightView::Left,
        FlightView::Right,
    ] {
        let mut viewed = output.clone();
        apply_flight_view(view, &mut viewed);
        assert_eq!(viewed.len(), output.len());
        assert!(viewed.iter().flat_map(|v| v.position).all(f32::is_finite));
        for indices in [[0, 1], [2, 5]] {
            let center: [f32; 3] = std::array::from_fn(|axis| {
                (output[indices[0]].position[axis] + output[indices[1]].position[axis]) / 2.0
            });
            let [x, y, z] = center;
            let expected = match view {
                FlightView::Front => [x, y, z],
                FlightView::Rear => [-x, y, -z],
                FlightView::Left => [z, y, -x],
                FlightView::Right => [-z, y, x],
            };
            for (axis, expected) in expected.into_iter().enumerate() {
                close(
                    (viewed[indices[0]].position[axis] + viewed[indices[1]].position[axis]) / 2.0,
                    expected.into(),
                );
            }
        }
    }
}

#[test]
fn beam_tick_lifetime_and_player_pulse_identity_never_replay_future_or_old_receipts() {
    let (mut snapshot, mut scene) = beam_fixture();
    let beam = scene.last_beam.as_mut().unwrap_or_else(|| panic!("beam"));
    beam.source = BeamSource::Player {
        owner: 12001,
        hull: 12002,
        item: 12004,
        emission: 1,
    };
    let pulse = PulseShot {
        tick: beam.world_tick,
        fitted_item_id: 12004,
        origin: beam.origin,
        direction: beam.direction,
        end: beam.point,
        contact: PulseContact::Clear,
    };
    assert!(contains_pulse(&scene, snapshot.tick, &pulse));
    assert!(!contains_pulse(
        &scene,
        snapshot.tick,
        &PulseShot {
            fitted_item_id: 99,
            ..pulse
        }
    ));
    assert!(!contains_pulse(
        &scene,
        snapshot.tick,
        &PulseShot {
            end: Vec3::new(0.0, 0.0, 0.0),
            ..pulse
        }
    ));
    let ship = ok(load_ship());
    let world = ports();
    for (offset, count) in [(-1, 0), (0, 12), (4, 12), (5, 0)] {
        snapshot.tick = pulse
            .tick
            .checked_add_signed(offset)
            .unwrap_or_else(|| panic!("tick"));
        let station = station(
            &snapshot,
            world.station_port().unwrap_or_else(|| panic!("port")),
        );
        let mut output = Vec::new();
        ok(append(&snapshot, station, &ship, &scene, &mut output));
        assert_eq!(output.len(), count);
        assert_eq!(contains_pulse(&scene, snapshot.tick, &pulse), count != 0);
    }
}

#[test]
fn unsupported_populations_and_hulls_refuse_without_a_proxy_or_partial_append() {
    let (snapshot, scene) = admitted();
    let ship = ok(load_ship());
    let world = ports();
    let station = station(
        snapshot,
        world.station_port().unwrap_or_else(|| panic!("port")),
    );
    let sentinel = Vertex {
        position: [1.0, 2.0, 3.0],
        color: [0.1, 0.2, 0.3],
    };
    for population in [
        Population::Police,
        Population::Miner,
        Population::BountyHunter,
        Population::Alien,
        Population::AiConstruct,
    ] {
        let mut scene = scene.clone();
        scene.actors[0].population = population;
        let mut output = vec![sentinel];
        assert!(append(snapshot, station, &ship, &scene, &mut output).is_err());
        same(&output, &[sentinel]);
    }
    let mut scene = scene.clone();
    scene.actors[0].hull_id = "SH031";
    let mut output = vec![sentinel];
    assert!(append(snapshot, station, &ship, &scene, &mut output).is_err());
    same(&output, &[sentinel]);
}

#[test]
fn nonfinite_body_or_retained_segment_rolls_back_the_entire_traffic_append() {
    let (snapshot, scene) = admitted();
    let ship = ok(load_ship());
    let world = ports();
    let station = station(
        snapshot,
        world.station_port().unwrap_or_else(|| panic!("port")),
    );
    let sentinel = Vertex {
        position: [1.0, 2.0, 3.0],
        color: [0.0; 3],
    };
    let mut invalid = scene.clone();
    invalid.last_beam = None;
    invalid.actors[1].pose.position.x = f64::NAN;
    let mut output = vec![sentinel];
    assert!(append(snapshot, station, &ship, &invalid, &mut output).is_err());
    same(&output, &[sentinel]);
    let (_, mut invalid) = beam_fixture();
    invalid
        .last_beam
        .as_mut()
        .unwrap_or_else(|| panic!("beam"))
        .point
        .z = f64::INFINITY;
    assert!(append(snapshot, station, &ship, &invalid, &mut output).is_err());
    same(&output, &[sentinel]);
}

#[test]
fn capacity_is_bounded_by_real_hull_size_and_refuses_excess_descriptive_bodies() {
    let ship = ok(load_ship());
    assert_eq!(
        vertex_capacity(ship.len()),
        Some(MAX_TRAFFIC_ACTORS * ship.len() + 12)
    );
    assert_eq!(vertex_capacity(usize::MAX), None);
    let (snapshot, scene) = admitted();
    let mut invalid = scene.clone();
    invalid
        .actors
        .resize(MAX_TRAFFIC_ACTORS + 1, scene.actors[0]);
    let world = ports();
    let station = station(
        snapshot,
        world.station_port().unwrap_or_else(|| panic!("port")),
    );
    let mut output = Vec::new();
    assert!(append(snapshot, station, &ship, &invalid, &mut output).is_err());
    assert!(output.is_empty());
    // Actual station asset is unchanged; NPC rendering does not replace it with a fixture.
    assert!(!ok(load_station()).is_empty());
}

// This contact comes from paid cargo and public flight, never a constructed NpcPlayerHit.
fn actual_incoming() -> &'static (Snapshot, TrafficSnapshot) {
    static SCENE: OnceLock<(Snapshot, TrafficSnapshot)> = OnceLock::new();
    SCENE.get_or_init(|| {
        let mut world = ok(World::new_commander(12001));
        ok(world.request_station_service(meridian_core::station_services::Command::BuyFood, 0));
        ok(world.request_launch(world.snapshot().tick));
        ok(world.step(FlightInput::default()));
        for _ in 0..300 {
            if world.snapshot().speed == 0.0 {
                break;
            }
            ok(world.step(FlightInput {
                brake: true,
                ..FlightInput::default()
            }));
        }
        assert_eq!(world.snapshot().speed, 0.0);
        let angle = -core::f32::consts::FRAC_PI_2;
        let ticks = (angle.abs() / (0.65 / 60.0)).ceil() as u16;
        for _ in 0..ticks {
            ok(world.step(FlightInput {
                yaw: angle / (0.65 / 60.0) / f32::from(ticks),
                ..FlightInput::default()
            }));
        }
        for _ in 0..3000 {
            if world.snapshot().position[0] <= -600.0 {
                break;
            }
            let speed = world.snapshot().speed;
            ok(world.step(FlightInput {
                throttle: ((10.0 - speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..FlightInput::default()
            }));
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        assert!(world.snapshot().position[0] <= -600.0);
        for _ in 0..300 {
            if world.snapshot().speed == 0.0 {
                break;
            }
            ok(world.step(FlightInput {
                brake: true,
                ..FlightInput::default()
            }));
        }
        assert_eq!(world.snapshot().speed, 0.0);
        for _ in 0..24000 {
            ok(world.step(FlightInput::default()));
            let scene = world
                .traffic_snapshot()
                .unwrap_or_else(|| panic!("traffic"));
            if let Some(hit) = &scene.last_player_hit {
                assert_eq!(hit.world_tick, world.snapshot().tick);
                assert_eq!(world.credits_tenths(), Some(6974));
                return (world.snapshot(), scene);
            }
        }
        panic!("public cargo flight did not receive a native pirate contact");
    })
}

#[test]
fn actual_incoming_player_hit_uses_retained_segment_and_expires_without_inventing_npc_victim() {
    let (snapshot, original) = actual_incoming();
    let hit = original
        .last_player_hit
        .as_ref()
        .unwrap_or_else(|| panic!("hit"));
    assert_eq!(hit.attacker.serial(), 2);
    assert_eq!(hit.player_hull, 12002);
    let mut scene = original.clone();
    scene.last_beam = None;
    for actor in &mut scene.actors {
        actor.phase = NpcPhase::Complete;
    }
    let world = ports();
    let station = station(
        snapshot,
        world.station_port().unwrap_or_else(|| panic!("port")),
    );
    let camera = ok(camera_for(snapshot, station));
    let ship = ok(load_ship());
    let mut output = Vec::new();
    ok(append(snapshot, station, &ship, &scene, &mut output));
    assert_eq!(output.len(), crate::mesh::PULSE_VERTEX_COUNT);
    for (indices, endpoint) in [([0, 1], hit.source_pose.position), ([2, 5], hit.point)] {
        let expected = camera.pose.orientation.inverse_coordinates(Vec3::new(
            endpoint.x - camera.pose.position.x,
            endpoint.y - camera.pose.position.y,
            endpoint.z - camera.pose.position.z,
        ));
        for axis in 0..3 {
            close(
                (output[indices[0]].position[axis] + output[indices[1]].position[axis]) / 2.0,
                [expected.x, expected.y, expected.z][axis],
            );
        }
    }
    // Moving current poses cannot retarget an already admitted discharge.
    for actor in &mut scene.actors {
        actor.pose.position.x += 4000.0;
    }
    let mut moved = Vec::new();
    ok(append(snapshot, station, &ship, &scene, &mut moved));
    same(&output, &moved);
    let mut later = *snapshot;
    later.tick = hit.world_tick + 5;
    let mut expired = Vec::new();
    ok(append(&later, station, &ship, &scene, &mut expired));
    assert!(expired.is_empty());
    assert_eq!(original, &actual_incoming().1);
}

#[test]
fn incoming_segment_tie_keeps_separate_player_pulse_and_retains_one_traffic_segment() {
    let (snapshot, original) = actual_incoming();
    let mut scene = original.clone();
    let hit = scene
        .last_player_hit
        .as_ref()
        .unwrap_or_else(|| panic!("hit"))
        .clone();
    let actor = scene
        .actors
        .iter()
        .find(|a| a.id != hit.attacker)
        .unwrap_or_else(|| panic!("actual other actor"));
    scene.last_beam = Some(TrafficBeam {
        source: BeamSource::Player {
            owner: 12001,
            hull: 12002,
            item: 12003,
            emission: 1,
        },
        victim: actor.id,
        world_tick: hit.world_tick,
        local_tick: hit.local_tick,
        origin: hit.player_pose.position,
        direction: hit.player_pose.orientation.z,
        point: actor.pose.position,
        victim_pose: actor.pose,
        front: true,
    });
    let beam = scene.last_beam.unwrap_or_else(|| panic!("beam"));
    // This is a labelled presentation-only duplicate fixture, not a World damage grant.
    let pulse = PulseShot {
        tick: beam.world_tick,
        fitted_item_id: 12003,
        origin: beam.origin,
        direction: beam.direction,
        end: beam.point,
        contact: PulseContact::Npc {
            id: beam.victim,
            distance: 1.0,
        },
    };
    assert!(!contains_pulse(&scene, snapshot.tick, &pulse));
    scene.last_player_hit = None;
    assert!(contains_pulse(&scene, snapshot.tick, &pulse));
}

#[test]
fn malformed_incoming_player_segment_refuses_without_partial_traffic_vertices() {
    let (snapshot, original) = actual_incoming();
    let ship = ok(load_ship());
    let world = ports();
    let station = station(
        snapshot,
        world.station_port().unwrap_or_else(|| panic!("port")),
    );
    for bad_visit in [false, true] {
        let mut scene = original.clone();
        let hit = scene
            .last_player_hit
            .as_mut()
            .unwrap_or_else(|| panic!("hit"));
        if bad_visit {
            hit.visit += 1;
        } else {
            hit.point.x = f64::NAN;
        }
        let sentinel = Vertex {
            position: [1.0, 2.0, 3.0],
            color: [0.2, 0.3, 0.4],
        };
        let mut output = vec![sentinel];
        assert!(append(snapshot, station, &ship, &scene, &mut output).is_err());
        assert_eq!(output.len(), 1);
        same(&output, &[sentinel]);
    }
}

use meridian_core::docking::{Basis3, DockingPort, RigidPose, StationPose, Vec3};
use meridian_core::{PRACTICE_HALF_SIDE, PracticeBeaconSnapshot, PulseShot, Snapshot};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

pub fn load_ship() -> Result<Vec<Vertex>, String> {
    decode_mesh(include_bytes!("../../../assets/legacy/sh025.mesh"))
}

pub fn load_station() -> Result<Vec<Vertex>, String> {
    decode_mesh(include_bytes!("../../../assets/legacy/sh028.mesh"))
}

/// Presentation borrows content and the authoritative station frame; it grants no admission.
#[derive(Clone, Copy)]
pub struct StationRender<'a> {
    pub vertices: &'a [Vertex],
    pub pose: StationPose,
    pub port: &'a DockingPort,
    pub docked: bool,
}

struct Camera {
    pose: RigidPose,
    draw_ship: bool,
}

fn point(values: [f32; 3]) -> Vec3 {
    Vec3::new(values[0].into(), values[1].into(), values[2].into())
}

fn player_pose(snapshot: &Snapshot) -> RigidPose {
    let [x, y, z, w] = snapshot.orientation.map(f64::from);
    RigidPose {
        position: point(snapshot.position),
        orientation: Basis3 {
            x: Vec3::new(
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y + w * z),
                2.0 * (x * z - w * y),
            ),
            y: Vec3::new(
                2.0 * (x * y - w * z),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z + w * x),
            ),
            z: Vec3::new(
                2.0 * (x * z + w * y),
                2.0 * (y * z - w * x),
                1.0 - 2.0 * (x * x + y * y),
            ),
        },
    }
}

fn difference(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn camera_for(snapshot: &Snapshot, station: StationRender<'_>) -> Result<Camera, String> {
    let entrance = station
        .pose
        .entrance_frame(station.port)
        .map_err(|e| e.to_string())?;
    let player = player_pose(snapshot);
    if station.docked {
        // A docked exterior observer is presentation only; World retains the parked pose.
        let outward = entrance.outward();
        let forward = Vec3::new(-outward.x, -outward.y, -outward.z);
        let right = cross(entrance.up(), forward);
        return Ok(Camera {
            pose: RigidPose {
                position: entrance.approach_point(600.0).map_err(|e| e.to_string())?,
                orientation: Basis3 {
                    x: right,
                    y: cross(forward, right),
                    z: forward,
                },
            },
            draw_ship: false,
        });
    }
    let distance = difference(player.position, entrance.position());
    // Use the pilot's pose near the door so the chase camera cannot sit inside a wall.
    // This threshold is a view convention, not a docking or collision condition.
    if distance.dot(distance) < 500.0 * 500.0 {
        return Ok(Camera {
            pose: player,
            draw_ship: false,
        });
    }
    let offset = player.orientation.apply(Vec3::new(0.0, 45.0, -220.0));
    Ok(Camera {
        pose: RigidPose {
            position: Vec3::new(
                player.position.x + offset.x,
                player.position.y + offset.y,
                player.position.z + offset.z,
            ),
            orientation: player.orientation,
        },
        draw_ship: true,
    })
}

fn append_mesh(
    vertices: &[Vertex],
    model: RigidPose,
    camera: &Camera,
    output: &mut Vec<Vertex>,
) -> Result<(), String> {
    for vertex in vertices {
        let local = model.orientation.apply(point(vertex.position));
        let world = Vec3::new(
            local.x + model.position.x,
            local.y + model.position.y,
            local.z + model.position.z,
        );
        let view = camera
            .pose
            .orientation
            .inverse_coordinates(difference(world, camera.pose.position));
        let position = [view.x as f32, view.y as f32, view.z as f32];
        if position.iter().any(|v| !v.is_finite()) {
            return Err("Scene transform produced a non-finite vertex".into());
        }
        output.push(Vertex {
            position,
            color: vertex.color,
        });
    }
    Ok(())
}

// Three closed cubes, with two triangles per face.
pub const PRACTICE_VERTEX_COUNT: usize = 3 * 6 * 6;

pub fn append_practice_beacons(
    snapshot: &Snapshot,
    station: StationRender<'_>,
    beacons: &[PracticeBeaconSnapshot; 3],
    output: &mut Vec<Vertex>,
) -> Result<(), String> {
    let camera = camera_for(snapshot, station)?;
    let side = PRACTICE_HALF_SIDE as f32;
    let corners = [
        [-side, -side, -side],
        [side, -side, -side],
        [side, side, -side],
        [-side, side, -side],
        [-side, -side, side],
        [side, -side, side],
        [side, side, side],
        [-side, side, side],
    ];
    let faces = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [3, 7, 6, 2],
        [0, 4, 7, 3],
        [1, 2, 6, 5],
    ];
    for beacon in beacons {
        let color = if beacon.flashing {
            [1.0, 1.0, 1.0]
        } else {
            [0.0, 1.0, 1.0]
        };
        let vertices: [Vertex; 36] = std::array::from_fn(|index| Vertex {
            position: corners[faces[index / 6][[0, 1, 2, 0, 2, 3][index % 6]]],
            color,
        });
        append_mesh(&vertices, beacon.pose, &camera, output)?;
    }
    Ok(())
}

// Three finite projectiles, each original eight-face mesh triangulated to ten triangles.
pub const MISSILE_VERTEX_COUNT: usize = 3 * 10 * 3;

pub fn append_missiles(
    snapshot: &Snapshot,
    station: StationRender<'_>,
    missiles: &meridian_core::MissileSnapshot,
    output: &mut Vec<Vertex>,
) -> Result<(), String> {
    let camera = camera_for(snapshot, station)?;
    let mut vertices = [Vertex {
        position: [0.0; 3],
        color: [0.8; 3],
    }; 30];
    let mut count = 0;
    for face in meridian_core::MISSILE_FACES {
        for corner in 1..face.len() - 1 {
            for index in [face[0], face[corner], face[corner + 1]] {
                let point = meridian_core::MISSILE_VERTICES[index];
                vertices[count].position = [point.x as f32, point.y as f32, point.z as f32];
                count += 1;
            }
        }
    }
    for projectile in missiles
        .projectiles
        .iter()
        .flatten()
        .filter(|p| p.ended.is_none())
    {
        append_mesh(
            &vertices,
            RigidPose {
                position: projectile.position,
                orientation: projectile.origin.orientation,
            },
            &camera,
            output,
        )?;
    }
    Ok(())
}

pub const PULSE_VERTEX_COUNT: usize = 12;

pub fn append_pulse(
    snapshot: &Snapshot,
    station: StationRender<'_>,
    shot: &PulseShot,
    output: &mut Vec<Vertex>,
) -> Result<(), String> {
    let camera = camera_for(snapshot, station)?;
    let view = |world| {
        camera
            .pose
            .orientation
            .inverse_coordinates(difference(world, camera.pose.position))
    };
    let start = view(shot.origin);
    let end = view(shot.end);
    let extent = difference(end, start);
    let length = extent.dot(extent).sqrt();
    if !length.is_finite() {
        return Err("Pulse transform produced a non-finite segment".into());
    }
    if length == 0.0 {
        return Ok(());
    }
    let vertex = |center: Vec3, x: f64, y: f64| Vertex {
        position: [
            (center.x + x) as f32,
            (center.y + y) as f32,
            center.z as f32,
        ],
        color: [1.0, 0.24, 0.09],
    };
    let ribbon = [
        vertex(start, -1.0, 0.0),
        vertex(start, 1.0, 0.0),
        vertex(end, 1.0, 0.0),
        vertex(end, -1.0, 0.0),
    ];
    // An end-on beam projects to a line. Put a depth-tested flash on its
    // centerline near the emitter, inside even a short obstructed segment.
    let mut along = (12.0 / length).min(0.5);
    // Match the flight projection's near plane for a short visible segment.
    let near = 1.0;
    if start.z + along * extent.z <= near && start.z <= near && end.z > near {
        let visible_start = (near - start.z) / extent.z;
        along = (1.0 + visible_start) * 0.5;
    }
    let center = Vec3::new(
        start.x + along * extent.x,
        start.y + along * extent.y,
        start.z + along * extent.z,
    );
    let flash = [
        vertex(center, -0.35, -0.35),
        vertex(center, 0.35, -0.35),
        vertex(center, 0.35, 0.35),
        vertex(center, -0.35, 0.35),
    ];
    if ribbon
        .iter()
        .chain(&flash)
        .flat_map(|v| v.position)
        .any(|v| !v.is_finite())
    {
        return Err("Pulse transform produced a non-finite vertex".into());
    }
    for quad in [ribbon, flash] {
        output.extend([quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]);
    }
    Ok(())
}

fn decode_mesh(bytes: &[u8]) -> Result<Vec<Vertex>, String> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(72) {
        return Err("Imported ship must contain complete 24-byte triangle vertices".into());
    }
    bytes
        .as_chunks::<24>()
        .0
        .iter()
        .map(|vertex| {
            let mut values = [0.0; 6];
            for (value, chunk) in values.iter_mut().zip(vertex.as_chunks::<4>().0.iter()) {
                *value = f32::from_le_bytes(*chunk);
            }
            if values.iter().any(|value| !value.is_finite()) {
                return Err("Imported ship has non-finite vertex data".into());
            }
            Ok(Vertex {
                position: [values[0], values[1], values[2]],
                color: [values[3], values[4], values[5]],
            })
        })
        .collect()
}

fn rotate_inverse(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let u = [-q[0], -q[1], -q[2]];
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let t = cross(u, v).map(|x| 2.0 * x);
    let c = cross(u, t);
    std::array::from_fn(|i| v[i] + q[3] * t[i] + c[i])
}

pub fn frame_vertices(
    snapshot: &Snapshot,
    ship: &[Vertex],
    station: Option<StationRender<'_>>,
    output: &mut Vec<Vertex>,
) -> Result<(), String> {
    output.clear();
    if let Some(station) = station {
        let camera = camera_for(snapshot, station)?;
        if camera.draw_ship {
            append_mesh(ship, player_pose(snapshot), &camera, output)?;
        }
        append_mesh(
            station.vertices,
            station.pose.model_pose().map_err(|e| e.to_string())?,
            &camera,
            output,
        )?;
        return Ok(());
    }
    let eye_offset = [0.0, -45.0, 220.0];
    output.extend(ship.iter().map(|vertex| Vertex {
        position: std::array::from_fn(|i| vertex.position[i] + eye_offset[i]),
        color: vertex.color,
    }));
    // Navigation markers are prototype fixtures, not station collision/admission geometry.
    for z in [0.0, 1200.0, 2400.0] {
        for segment in 0..32 {
            let angle = segment as f32 * std::f32::consts::TAU / 32.0;
            let next = (segment + 1) as f32 * std::f32::consts::TAU / 32.0;
            let p = |a: f32, radius: f32| [radius * a.cos(), radius * a.sin(), z];
            let quad = [
                p(angle, 150.0),
                p(next, 150.0),
                p(next, 155.0),
                p(angle, 155.0),
            ];
            for index in [0, 1, 2, 0, 2, 3] {
                let relative = std::array::from_fn(|i| quad[index][i] - snapshot.position[i]);
                let local = rotate_inverse(snapshot.orientation, relative);
                output.push(Vertex {
                    position: std::array::from_fn(|i| local[i] + eye_offset[i]),
                    color: [0.08, 0.65, 0.7],
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn actual_missile_renders_original_mesh_then_removes_ended_body() {
        let mut world = meridian_core::World::new_commander(992).unwrap();
        let review = world.review_practice(0).unwrap();
        world
            .prepare_practice_confirmation(review, 0)
            .unwrap()
            .commit();
        world.request_launch(0).unwrap();
        world.step(meridian_core::FlightInput::default()).unwrap();
        let target = world.practice_snapshot().unwrap().beacons.unwrap()[1].id;
        let tick = world.snapshot().tick;
        world.request_missile_lock(Some(target), tick).unwrap();
        world.request_missile_arm(true, tick).unwrap();
        world.request_fire_missile(tick).unwrap();
        let vertices = load_station().unwrap();
        let render = |world: &meridian_core::World| {
            let snapshot = world.snapshot();
            let station = StationRender {
                vertices: &vertices,
                pose: snapshot.station.unwrap(),
                port: world.station_port().unwrap(),
                docked: false,
            };
            let mut frame = Vec::new();
            append_missiles(
                &snapshot,
                station,
                &world.missile_snapshot().unwrap(),
                &mut frame,
            )
            .unwrap();
            frame
        };
        let frame = render(&world);
        assert_eq!(frame.len(), 30);
        assert!(
            frame
                .iter()
                .all(|v| v.position.iter().all(|x| x.is_finite()))
        );
        for _ in 0..1500 {
            if world
                .missile_snapshot()
                .unwrap()
                .projectiles
                .iter()
                .flatten()
                .all(|p| p.ended.is_some())
            {
                break;
            }
            world.step(meridian_core::FlightInput::default()).unwrap();
        }
        assert!(render(&world).is_empty());
    }

    use super::*;

    fn reference_port() -> DockingPort {
        DockingPort::new(
            Vec3::new(0.0, 0.0, 136.32),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            53.52,
            vec![
                Vec3::new(-62.32, -50.8, 0.0),
                Vec3::new(62.32, -50.8, 0.0),
                Vec3::new(62.32, 50.8, 0.0),
                Vec3::new(-62.32, 50.8, 0.0),
            ],
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    fn reference_station_pose() -> StationPose {
        StationPose {
            pose: RigidPose {
                position: Vec3::new(0.0, 0.0, 0.0),
                orientation: Basis3::IDENTITY,
            },
            axial_rotation: 0.0,
            rotation_rate: 0.0,
        }
    }

    #[test]
    fn malformed_or_nonfinite_mesh_is_refused() {
        assert!(decode_mesh(&[]).is_err());
        assert!(decode_mesh(&[0; 73]).is_err());
        let mut bytes = [0; 72];
        bytes[..4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(decode_mesh(&bytes).is_err());
    }

    #[test]
    fn imported_reference_is_complete_and_finite() {
        let ship = load_ship().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(ship.len(), 3484 * 3);
        assert!(
            ship.iter()
                .flat_map(|v| v.color)
                .all(|c| (0.0..=1.0).contains(&c))
        );
    }

    #[test]
    fn inverse_camera_rotation_has_the_expected_handedness() {
        let half = std::f32::consts::FRAC_PI_4;
        let actual = rotate_inverse([0.0, half.sin(), 0.0, half.cos()], [1.0, 0.0, 0.0]);
        assert!(actual[0].abs() < 0.00001);
        assert!((actual[2] - 1.0).abs() < 0.00001);
    }

    #[test]
    fn scene_buffers_are_reused_and_snapshots_are_not_mutated() {
        let snapshot = meridian_core::World::new().snapshot();
        let original = snapshot;
        let ship = load_ship().unwrap_or_else(|error| panic!("{error}"));
        let count = ship.len() + 3 * 32 * 6;
        let mut output = Vec::with_capacity(count);
        let capacity = output.capacity();
        for _ in 0..10 {
            frame_vertices(&snapshot, &ship, None, &mut output).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(output.len(), count);
            assert_eq!(output.capacity(), capacity);
            assert_eq!(snapshot, original);
        }
    }

    #[test]
    fn station_display_transform_preserves_roll_before_basis_and_translation() {
        let model = StationPose {
            pose: RigidPose {
                position: Vec3::new(10.0, 20.0, 30.0),
                orientation: Basis3 {
                    x: Vec3::new(0.0, 0.0, 1.0),
                    y: Vec3::new(0.0, 1.0, 0.0),
                    z: Vec3::new(-1.0, 0.0, 0.0),
                },
            },
            axial_rotation: std::f64::consts::FRAC_PI_2,
            rotation_rate: 0.0,
        };
        let camera = Camera {
            pose: RigidPose {
                position: Vec3::new(0.0, 0.0, 0.0),
                orientation: Basis3::IDENTITY,
            },
            draw_ship: true,
        };
        let vertex = Vertex {
            position: [1.0, 2.0, 3.0],
            color: [0.2, 0.3, 0.4],
        };
        let mut frame = Vec::new();
        append_mesh(
            &[vertex],
            model.model_pose().unwrap_or_else(|e| panic!("{e}")),
            &camera,
            &mut frame,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(frame[0].position, [7.0, 21.0, 28.0]);
        assert_eq!(frame[0].color, vertex.color);
    }

    #[test]
    fn docked_observer_faces_real_port_without_changing_world_pose() {
        let snapshot = meridian_core::World::new().snapshot();
        let before = snapshot;
        let port = reference_port();
        let station = StationRender {
            vertices: &[],
            pose: reference_station_pose(),
            port: &port,
            docked: true,
        };
        let camera = camera_for(&snapshot, station).unwrap_or_else(|e| panic!("{e}"));
        assert!(!camera.draw_ship);
        assert_eq!(camera.pose.orientation.z, Vec3::new(0.0, 0.0, -1.0));
        assert!((camera.pose.position.z - 736.32).abs() < 1.0e-10);
        let vertex = Vertex {
            position: [0.0, 0.0, 136.32],
            color: [0.1, 0.2, 0.3],
        };
        let mut frame = Vec::new();
        append_mesh(
            &[vertex],
            station.pose.model_pose().unwrap_or_else(|e| panic!("{e}")),
            &camera,
            &mut frame,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(frame[0].position[0].abs() < 1.0e-5);
        assert!(frame[0].position[1].abs() < 1.0e-5);
        assert!((frame[0].position[2] - 600.0).abs() < 1.0e-4);
        assert_eq!(snapshot, before);
    }

    #[test]
    fn centered_pilot_pulse_has_visible_projected_area_before_clear_and_blocked_endpoints() {
        use meridian_core::PulseContact;

        let mut snapshot = meridian_core::World::new().snapshot();
        snapshot.position = [0.0, 0.0, 300.0];
        let port = reference_port();
        let station = StationRender {
            vertices: &[],
            pose: reference_station_pose(),
            port: &port,
            docked: false,
        };
        assert!(
            !camera_for(&snapshot, station)
                .unwrap_or_else(|e| panic!("{e}"))
                .draw_ship
        );
        let origin = point(snapshot.position);
        for (distance, contact) in [
            (6000.0, PulseContact::Clear),
            (136.32, PulseContact::StationBlocked { distance: 136.32 }),
            (1.25, PulseContact::StationBlocked { distance: 1.25 }),
        ] {
            let shot = PulseShot {
                tick: snapshot.tick,
                fitted_item_id: 1,
                origin,
                direction: Vec3::new(0.0, 0.0, 1.0),
                end: Vec3::new(origin.x, origin.y, origin.z + distance),
                contact,
            };
            let mut output = Vec::with_capacity(PULSE_VERTEX_COUNT);
            append_pulse(&snapshot, station, &shot, &mut output)
                .unwrap_or_else(|error| panic!("{error}"));
            // Match flight.wgsl's perspective division at aspect 1.
            let has_visible_triangle = output.as_chunks::<3>().0.iter().any(|vertices| {
                let [a, b, c] = vertices.map(|vertex| vertex.position);
                if [a, b, c]
                    .iter()
                    .any(|&[_, _, z]| z <= 1.0 || f64::from(z) >= distance)
                {
                    return false;
                }
                let project = |[x, y, z]: [f32; 3]| [x * 1.6 / z, y * 1.6 / z];
                let [[ax, ay], [bx, by], [cx, cy]] = [a, b, c].map(project);
                ((bx - ax) * (cy - ay) - (by - ay) * (cx - ax)).abs() > 0.0001
            });
            assert!(has_visible_triangle, "{contact:?}");
            assert_eq!(output.len(), PULSE_VERTEX_COUNT);
            assert!((f64::from(output[2].position[2]) - distance).abs() < 0.001);
        }
    }

    #[test]
    fn close_station_view_uses_pilot_pose_and_has_no_fixture_rings() {
        let mut snapshot = meridian_core::World::new().snapshot();
        snapshot.position = [0.0, 0.0, 300.0];
        let port = reference_port();
        let vertices = [Vertex {
            position: [1.0, 2.0, 136.32],
            color: [0.2, 0.4, 0.6],
        }; 3];
        let station = StationRender {
            vertices: &vertices,
            pose: reference_station_pose(),
            port: &port,
            docked: false,
        };
        let camera = camera_for(&snapshot, station).unwrap_or_else(|e| panic!("{e}"));
        assert!(!camera.draw_ship);
        assert_eq!(camera.pose.position, Vec3::new(0.0, 0.0, 300.0));
        let ship = load_ship().unwrap_or_else(|e| panic!("{e}"));
        let mut frame = Vec::with_capacity(vertices.len());
        let capacity = frame.capacity();
        for _ in 0..10 {
            frame_vertices(&snapshot, &ship, Some(station), &mut frame)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(frame.len(), 3);
            assert_eq!(frame.capacity(), capacity);
            assert_eq!(frame[0].position, [1.0, 2.0, 136.32_f32 - 300.0]);
        }
    }
}

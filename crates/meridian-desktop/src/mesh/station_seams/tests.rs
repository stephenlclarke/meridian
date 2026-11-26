use super::super::{StationRender, Vertex, decode_mesh, frame_vertices, load_station};
use super::append;
use meridian_core::World;

const ORIGINAL: &[u8] = include_bytes!("../../../../../assets/legacy/sh028.mesh");

fn ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("{error:?}"))
}

fn docked_display(vertices: &[Vertex]) -> Vec<Vertex> {
    let world = ok(World::new_commander(528));
    let snapshot = world.snapshot();
    let station = StationRender {
        vertices,
        pose: snapshot
            .station
            .unwrap_or_else(|| panic!("original station")),
        port: world
            .station_port()
            .unwrap_or_else(|| panic!("original port")),
        docked: true,
        player_hull: meridian_core::PlayerHullKind::Peregrine,
    };
    let mut output = Vec::new();
    ok(frame_vertices(&snapshot, &[], Some(station), &mut output));
    assert_eq!(world.snapshot(), snapshot);
    output
}

fn covers(vertices: &[Vertex], sample: [f64; 2]) -> bool {
    fn edge(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
        (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
    }
    vertices.as_chunks::<3>().0.iter().any(|triangle| {
        // Docked station triangles are inside the shader's [1, 20,000] depth interval.
        assert!(
            triangle
                .iter()
                .all(|v| (1.0..20_000.0).contains(&v.position[2]))
        );
        let [a, b, c] = triangle.map(|v| {
            let [x, y, z] = v.position.map(f64::from);
            [x / z, y / z]
        });
        let edges = [edge(a, b, sample), edge(b, c, sample), edge(c, a, sample)];
        edge(a, b, c).abs() > 1e-12
            && (edges.iter().all(|&e| e >= 0.0) || edges.iter().all(|&e| e <= 0.0))
    })
}

#[test]
fn real_dock_camera_closes_both_sides_of_both_crosspieces_and_keeps_the_port_open() {
    let original = ok(decode_mesh(ORIGINAL));
    let before = docked_display(&original);
    let after = docked_display(&ok(load_station()));
    let seams = &after[before.len()..];
    // Perspective ray slopes from the screenshot's dark wedges, registered to
    // the original rim silhouette at the real 600 m dock observer.
    for horizontal in [-0.031, 0.031] {
        for vertical in [-0.108, 0.108] {
            let wedge = [horizontal, vertical];
            assert!(!covers(&before, wedge), "original wedge at {wedge:?}");
            assert!(covers(seams, wedge), "new local join at {wedge:?}");
            assert!(covers(&before, [horizontal, vertical.signum() * 0.100]));
        }
    }
    // Check the optical axis and the interior at both ends of each crosspiece.
    for horizontal in [-0.08, 0.0, 0.08] {
        for vertical in [-0.060, 0.0, 0.060] {
            let opening = [horizontal, vertical];
            assert!(!covers(&before, opening));
            assert!(!covers(&after, opening), "port stays open at {opening:?}");
        }
    }
}

#[test]
fn source_rim_endpoints_and_rail_material_are_retained_outside_the_native_port() {
    let source = ok(decode_mesh(ORIGINAL));
    let mut repaired = source.clone();
    ok(append(&mut repaired));
    let (unchanged, seams) = repaired.split_at(source.len());
    assert_eq!(bytemuck::cast_slice::<Vertex, u8>(unchanged), ORIGINAL);
    assert_eq!(seams.len(), 48);
    let inner = 13.90888_f32;
    let outer = 40.77872_f32;
    for (half, y, color) in [
        (&seams[..24], 64.4_f32, source[29388].color),
        (&seams[24..], -64.4_f32, source[29357].color),
    ] {
        assert!(half.as_chunks::<3>().0.iter().all(|triangle| {
            triangle.iter().all(|v| {
                let [x, vy, z] = v.position;
                (inner..=outer).contains(&x.abs())
                    && vy.abs() > 50.8
                    && vy.signum() == y.signum()
                    && v.color == color
                    && (z == -80.0 || (z == 82.8 && vy == y))
            })
        }));
        for (x, height) in [
            (inner, 91.37152_f32),
            (27.57976, 89.01976),
            (outer, 85.14472),
        ] {
            for x in [-x, x] {
                assert!(
                    half.iter()
                        .any(|v| v.position == [x, height.copysign(y), -80.0])
                );
            }
        }
    }
    let mut incomplete = Vec::new();
    assert!(append(&mut incomplete).is_err());
    assert!(incomplete.is_empty());
}

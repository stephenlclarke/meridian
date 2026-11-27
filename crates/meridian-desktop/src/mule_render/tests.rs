//! Authored tests, UNRUN in the source lane. No fixture grants a World actor.
use super::*;
use meridian_core::docking::Basis3;
pub(super) fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn basis() -> Basis3 {
    Basis3 {
        x: Vec3::new(1.0, 0.0, 0.0),
        y: Vec3::new(0.0, 1.0, 0.0),
        z: Vec3::new(0.0, 0.0, 1.0),
    }
}
fn pose(x: f64, y: f64, z: f64) -> RigidPose {
    RigidPose {
        position: Vec3::new(x, y, z),
        orientation: basis(),
    }
}
fn blank_pieces() -> [Piece; MULE_PIECES] {
    [Piece {
        range: [0; 4],
        low: [0.0; 4],
        high: [0.0; 4],
    }; MULE_PIECES]
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
fn decode_point(bytes: &[u8]) -> Vec3 {
    let values: [f64; 3] = core::array::from_fn(|i| {
        f64::from_le_bytes(
            bytes[i * 8..i * 8 + 8]
                .try_into()
                .unwrap_or_else(|_| panic!("point")),
        ) * 40000.0
            / 3411.0
    });
    Vec3::new(values[0], values[1], values[2])
}
#[test]
fn every_detailed_face_keeps_original_index_order_scale_and_finite_uv_materials() {
    let vertices = ok(visual());
    assert_eq!(vertices.len(), 7236 * 3);
    assert_eq!(std::mem::size_of::<VisualVertex>(), 56);
    let source = include_bytes!("../../../../assets/fleet/sh001/all-source.vertices");
    let triangles = include_bytes!("../../../../assets/fleet/sh001/source-triangles.bin");
    let mut faces_per_layer = [0; 3];
    let mut emissive = 0;
    for (face, actual) in triangles[..7236 * 12]
        .as_chunks::<12>()
        .0
        .iter()
        .zip(vertices.as_chunks::<3>().0)
    {
        let layer = actual[0].texture;
        faces_per_layer[layer as usize] += 1;
        for (index, v) in face.as_chunks::<4>().0.iter().zip(actual) {
            assert_eq!(v.texture, layer);
            let index = u32::from_le_bytes(
                index
                    .as_slice()
                    .try_into()
                    .unwrap_or_else(|_| panic!("index")),
            ) as usize;
            let p = decode_point(&source[16 + index * 24..16 + (index + 1) * 24]);
            for (actual, expected) in v.position.iter().zip([p.x, p.y, p.z]) {
                assert!((f64::from(*actual) - expected).abs() <= 0.000004);
            }
            assert!(v.uv.iter().all(|v| v.is_finite()));
            assert_eq!(v.color[3], 1.0);
            emissive += usize::from(v.emissive != 0);
            let n = v.normal.map(f64::from);
            assert!((n.iter().map(|a| a * a).sum::<f64>() - 1.0).abs() < 0.000001);
        }
    }
    // The texture catalogue includes the strike map, but no detailed SH001 face references it.
    // The independent source-field fixture checks each individual face, including its layer.
    assert_eq!(faces_per_layer, [452, 0, 6784]);
    assert!(emissive > 0);
}
#[test]
fn all_boundaries_certify_source_support_and_ten_micrometre_tightness() {
    let source = ok(MuleHullSource::bundled());
    let envelope = ok(source.presentation(24000, 24000, 24000));
    let (triangles, pieces) = ok(supports(&envelope));
    assert!(!triangles.is_empty());
    assert_eq!(envelope.revision, MULE_PRESENTATION_REVISION);
    let mut stale = envelope.clone();
    stale.revision = 1;
    assert!(supports(&stale).is_err());
    assert_eq!(
        source
            .pieces()
            .iter()
            .map(|piece| piece.source_points().len())
            .sum::<usize>(),
        MAX_POINTS
    );
    assert!(triangles.len() <= 2 * MAX_POINTS - 4 * MULE_PIECES);
    for (index, piece) in source.pieces().iter().enumerate() {
        let hull = ok(rounded::build(piece.source_points()));
        assert!((0.0..=0.00001).contains(&hull.error_metres));
        assert_eq!(pieces[index].range[1] as usize, hull.faces.len());
        assert_eq!(pieces[index].low[3], 0.02f32);
        for x in -2..=2 {
            for y in -2..=2 {
                for z in -2..=2 {
                    if x == 0 && y == 0 && z == 0 {
                        continue;
                    }
                    let n = Vec3::new(f64::from(x), f64::from(y), f64::from(z));
                    let norm = n.dot(n).sqrt();
                    let original = piece
                        .source_points()
                        .iter()
                        .map(|p| p.dot(n))
                        .fold(f64::NEG_INFINITY, f64::max);
                    let boundary = hull
                        .faces
                        .iter()
                        .flat_map(|f| f.0)
                        .map(|i| piece.source_points()[i].dot(n))
                        .fold(f64::NEG_INFINITY, f64::max);
                    assert!(boundary <= original);
                    assert!(original - boundary <= norm * hull.error_metres + 1e-10);
                    assert!((ok(piece.support_bound(n)) - (original + 0.02 * norm)).abs() < 1e-9);
                }
            }
        }
    }
}
#[test]
fn cube_boundary_retains_corners_and_rejects_flat_or_invalid_source() {
    let mut points = Vec::new();
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                points.push(Vec3::new(x, y, z));
            }
        }
    }
    points.push(Vec3::new(0.0, 0.0, 0.0));
    points.push(points[0]);
    let hull = ok(rounded::build(&points));
    assert_eq!(hull.faces.len(), 12);
    assert!(hull.error_metres < 1e-8);
    for f in hull.faces {
        assert!(f.0.iter().all(|i| *i < 8));
    }
    assert!(rounded::build(&[Vec3::new(0.0, 0.0, 0.0); 4]).is_err());
    assert!(rounded::build(&[Vec3::new(f64::NAN, 0.0, 0.0); 4]).is_err());
    assert!(rounded::build(&[Vec3::new(100.0, 0.0, 0.0); 4]).is_err());
    assert!(rounded::build(&[Vec3::new(0.0, 0.0, 0.0); 5345]).is_err());
}
#[test]
fn zero_banks_hide_shimmer_without_retracting_any_contact_piece() {
    let source = ok(MuleHullSource::bundled());
    let full = ok(source.presentation(24000, 24000, 24000));
    let depleted = ok(source.presentation(0, 0, 24000));
    assert!(std::sync::Arc::ptr_eq(&full.pieces, &depleted.pieces));
    for point in [Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -1.0)] {
        assert_eq!(depleted.shimmer_alpha(point, 1.0, 0.0), 0.0);
        assert!((full.shimmer_alpha(point, 1.0, 0.0) - 0.018).abs() < 1e-8);
        assert_eq!(full.shimmer_alpha(point, 64.0, 0.0), 0.0);
        assert_eq!(full.shimmer_alpha(point, 10.0, 1.0), 0.0);
        assert!((full.shimmer_alpha(point, 48.0, 0.5) - 0.018 * 0.25 * 0.0625).abs() < 1e-8);
    }
    let split = ok(source.presentation(0, 12000, 24000));
    assert_eq!(split.shimmer_alpha(Vec3::new(0.0, 0.0, 0.0), 1.0, 0.0), 0.0);
    assert!((split.shimmer_alpha(Vec3::new(0.0, 0.0, -1.0), 1.0, 0.0) - 0.009).abs() < 1e-8);
}
#[test]
fn camera_views_share_the_actual_pose_and_keep_the_actor_unchanged() {
    let source = ok(MuleHullSource::bundled());
    let envelope = ok(source.presentation(24000, 12000, 24000));
    let model = pose(10.0, 20.0, 30.0);
    let before = model;
    for (flight, expected) in [
        (FlightView::Front, [10.0, 20.0, 30.0]),
        (FlightView::Rear, [-10.0, 20.0, -30.0]),
        (FlightView::Left, [30.0, 20.0, -10.0]),
        (FlightView::Right, [-30.0, 20.0, 10.0]),
    ] {
        let u = ok(frame_uniforms(
            model,
            &envelope,
            View {
                camera: pose(0.0, 0.0, 0.0),
                flight,
                projection: [1.5, 1.6, 1.0, 20000.0],
                tick: 123,
            },
            true,
            blank_pieces(),
        ));
        assert_eq!(&u.origin[..3], &expected);
        assert_eq!(&u.camera[..3], &[-10.0, -20.0, -30.0]);
        assert_eq!(&u.banks[..3], &[1.0, 0.5, 0.018]);
    }
    assert_eq!(model, before);
    let mut invalid = model;
    invalid.orientation.x = Vec3::new(1.1, 0.0, 0.0);
    assert!(
        frame_uniforms(
            invalid,
            &envelope,
            View {
                camera: pose(0.0, 0.0, 0.0),
                flight: FlightView::Front,
                projection: [1.5, 1.6, 1.0, 20000.0],
                tick: 0
            },
            true,
            blank_pieces()
        )
        .is_err()
    );
}
#[test]
fn linear_light_mips_keep_opacity_and_do_not_average_srgb_as_linear() {
    assert_eq!(mip(&[255; 16], 2), vec![255; 4]);
    let checker = [
        0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
    ];
    assert_eq!(mip(&checker, 2), vec![188, 188, 188, 255]);
    for map in TEXTURES {
        assert_eq!(map.len(), 1024 * 1024 * 4);
        assert!(map.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    }
}
#[test]
fn encoded_facets_have_finite_outward_normals_after_f32_conversion() {
    let source = ok(MuleHullSource::bundled());
    let (triangles, pieces) = ok(supports(&ok(source.presentation(24000, 24000, 24000))));
    for piece in pieces {
        for t in &triangles[piece.range[0] as usize..(piece.range[0] + piece.range[1]) as usize] {
            let v = |p: [f32; 4]| Vec3::new(p[0].into(), p[1].into(), p[2].into());
            let normal = cross(subtract(v(t.b), v(t.a)), subtract(v(t.c), v(t.a)));
            assert!(normal.dot(normal) > 1e-16);
        }
    }
}
#[test]
fn actual_wgsl_parses_and_validates_all_four_render_entries() {
    // Owning lane only: validates actual source shader, not a rewritten test shader.
    let module = ok(wgpu::naga::front::wgsl::parse_str(include_str!(
        "mule.wgsl"
    )));
    ok(wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::empty(),
    )
    .validate(&module));
    // Compare the parsed shader uniform layout with the Rust upload, including the last piece.
    use wgpu::naga::{ArraySize, TypeInner};
    let (_, shader_inputs) = module
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some("Inputs"))
        .unwrap_or_else(|| panic!("missing shader Inputs"));
    let TypeInner::Struct { members, span } = &shader_inputs.inner else {
        panic!("shader Inputs is not a struct");
    };
    assert_eq!(*span as usize, std::mem::size_of::<Uniforms>());
    let shader_pieces = members
        .iter()
        .find(|member| member.name.as_deref() == Some("pieces"))
        .unwrap_or_else(|| panic!("missing shader pieces"));
    assert_eq!(
        shader_pieces.offset as usize,
        std::mem::offset_of!(Uniforms, pieces)
    );
    let TypeInner::Array {
        size: ArraySize::Constant(count),
        stride,
        ..
    } = &module.types[shader_pieces.ty].inner
    else {
        panic!("shader pieces is not a fixed array");
    };
    assert_eq!(count.get() as usize, MULE_PIECES);
    assert_eq!(*stride as usize, std::mem::size_of::<Piece>());
    assert_eq!(module.entry_points.len(), 4);
    for name in [
        "solid_vertex",
        "solid_fragment",
        "union_vertex",
        "union_fragment",
    ] {
        assert!(module.entry_points.iter().any(|e| e.name == name));
    }
}

#[test]
fn actual_camera_f32_quaternion_residual_is_kept_without_world_pose_repair() {
    let source = ok(MuleHullSource::bundled());
    let envelope = ok(source.presentation(24000, 24000, 24000));
    let mut camera = pose(0.0, 0.0, 0.0);
    // This is the actual mesh adapter's matrix construction from a rounded f32 quaternion.
    let y = f64::from(0.38268343_f32);
    let w = f64::from(0.9238795_f32);
    camera.orientation = Basis3 {
        x: Vec3::new(1.0 - 2.0 * y * y, 0.0, -2.0 * w * y),
        y: Vec3::new(0.0, 1.0, 0.0),
        z: Vec3::new(2.0 * w * y, 0.0, 1.0 - 2.0 * y * y),
    };
    let before = camera;
    assert!(valid_pose(camera, 1e-5));
    assert!(!valid_pose(camera, 1e-10));
    let u = ok(frame_uniforms(
        pose(10.0, 0.0, 20.0),
        &envelope,
        View {
            camera,
            flight: FlightView::Front,
            projection: [1.0, 1.6, 1.0, 20000.0],
            tick: 0,
        },
        true,
        blank_pieces(),
    ));
    let expected = camera
        .orientation
        .inverse_coordinates(Vec3::new(10.0, 0.0, 20.0));
    assert_eq!(
        &u.origin[..3],
        &[expected.x as f32, expected.y as f32, expected.z as f32]
    );
    assert_eq!(camera, before);
}

#[test]
fn actual_world_sh025_contact_cannot_be_substituted_by_the_mule_adapter() {
    let mut world = ok(meridian_core::World::new_commander(12001));
    ok(world.request_launch(world.snapshot().tick));
    for _ in 0..1600 {
        ok(world.step(meridian_core::FlightInput {
            throttle: 1.0,
            ..Default::default()
        }));
        if world
            .traffic_snapshot()
            .is_some_and(|s| s.actors.len() == 2)
        {
            break;
        }
    }
    let traffic = world
        .traffic_snapshot()
        .unwrap_or_else(|| panic!("actual contacts"));
    assert_eq!(traffic.actors.len(), 2);
    let before = ok(world.save_bytes());
    let source = ok(MuleHullSource::bundled());
    let envelope = ok(source.presentation(24000, 24000, 24000));
    for actor in &traffic.actors {
        assert_eq!(actor.hull_id, "SH025");
        assert!(
            uniforms(
                Scene {
                    actor,
                    envelope: &envelope,
                    observer: pose(0.0, 0.0, 0.0),
                },
                View {
                    camera: pose(0.0, 0.0, 0.0),
                    flight: FlightView::Front,
                    projection: [1.0, 1.6, 1.0, 20000.0],
                    tick: world.snapshot().tick
                },
                true,
                blank_pieces()
            )
            .is_err()
        );
    }
    assert_eq!(world.traffic_snapshot(), Some(traffic));
    assert_eq!(ok(world.save_bytes()), before);
}

#[test]
fn controlled_observer_reframes_real_station_and_scene_without_an_extra_player_hull() {
    let world = ok(meridian_core::World::new_commander(12001));
    let snapshot = world.snapshot();
    let station_pose = snapshot.station.unwrap_or_else(|| panic!("actual station"));
    let port = world
        .station_port()
        .unwrap_or_else(|| panic!("actual port"));
    let vertices = ok(crate::mesh::load_station());
    let station = crate::mesh::StationRender {
        vertices: &vertices,
        pose: station_pose,
        port,
        docked: true,
        player_hull: meridian_core::PlayerHullKind::Peregrine,
    };
    let observer = pose(20.0, 40.0, -100.0);
    let mut output = Vec::new();
    ok(crate::mesh::station_from_observer(
        station,
        observer,
        &mut output,
    ));
    assert_eq!(output.len(), vertices.len());
    let model = ok(station_pose.model_pose());
    let p = vertices[0].position.map(f64::from);
    let local = model.orientation.apply(Vec3::new(p[0], p[1], p[2]));
    let expected = observer.orientation.inverse_coordinates(Vec3::new(
        local.x + model.position.x - observer.position.x,
        local.y + model.position.y - observer.position.y,
        local.z + model.position.z - observer.position.z,
    ));
    assert_eq!(
        output[0].position,
        [expected.x as f32, expected.y as f32, expected.z as f32]
    );
    let mut primitives = [crate::mesh::Vertex {
        position: [1.0, 2.0, 3.0],
        color: [0.3, 0.4, 0.5],
    }];
    ok(crate::mesh::reframe_observer(
        &mut primitives,
        pose(10.0, 20.0, 30.0),
        pose(4.0, 5.0, 6.0),
    ));
    assert_eq!(primitives[0].position, [7.0, 17.0, 27.0]);
    assert_eq!(primitives[0].color, [0.3, 0.4, 0.5]);
    assert_eq!(world.snapshot(), snapshot);
}

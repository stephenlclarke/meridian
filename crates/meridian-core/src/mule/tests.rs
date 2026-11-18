use super::*;
use crate::docking::GeometryError;
use crate::station::OriginalStationData;
fn pose(x: f64, y: f64, z: f64) -> RigidPose {
    RigidPose {
        position: Vec3::new(x, y, z),
        orientation: Basis3::IDENTITY,
    }
}
fn piece() -> Result<SmoothPiece, EnvelopeError> {
    SmoothPiece::new(
        vec![
            Vec3::new(-2.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, -3.0),
            Vec3::new(0.0, 0.0, 3.0),
        ],
        0.02,
    )
}
fn port(low: Vec3, high: Vec3) -> Result<DockingPort, GeometryError> {
    DockingPort::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 0.0),
        100.0,
        vec![
            Vec3::new(low.x, low.y, 0.0),
            Vec3::new(high.x, low.y, 0.0),
            Vec3::new(high.x, high.y, 0.0),
            Vec3::new(low.x, high.y, 0.0),
        ],
    )
}
fn station() -> StationPose {
    StationPose {
        pose: pose(0.0, 0.0, 0.0),
        axial_rotation: 0.0,
        rotation_rate: 0.0,
    }
}

#[test]
fn mule_authenticates_source_and_encloses_all_components_and_triangle_interiors()
-> Result<(), EnvelopeError> {
    let source = MuleHullSource::bundled()?;
    for (index, group) in MEMBERSHIP.iter().copied().enumerate() {
        let p = source_vertex(VERTICES, index)?;
        assert!(
            source.pieces[usize::from(group)]
                .source_points()
                .contains(&p)
        );
        assert!(source.pieces[usize::from(group)].contains(p));
    }
    let triangles = include_bytes!("../../../../assets/fleet/sh001/source-triangles.bin");
    assert_eq!(triangles.len(), 12 * (7236 + 1772));
    for record in triangles.chunks_exact(12) {
        let mut indices = [0usize; 3];
        for (axis, index) in indices.iter_mut().enumerate() {
            *index = u32::from_le_bytes(
                record[4 * axis..4 * (axis + 1)]
                    .try_into()
                    .map_err(|_| EnvelopeError::InvalidContent)?,
            ) as usize;
        }
        let group = MEMBERSHIP[indices[0]];
        assert!(indices.iter().all(|i| MEMBERSHIP[*i] == group));
        let a = source_vertex(VERTICES, indices[0])?;
        let b = source_vertex(VERTICES, indices[1])?;
        let c = source_vertex(VERTICES, indices[2])?;
        for w in [
            [1.0 / 3.0; 3],
            [0.5, 0.5, 0.0],
            [0.0, 0.25, 0.75],
            [0.25, 0.0, 0.75],
        ] {
            assert!(
                source.pieces[usize::from(group)]
                    .contains(add(add(scale(a, w[0]), scale(b, w[1])), scale(c, w[2])))
            );
        }
    }
    let mut changed = VERTICES.to_vec();
    changed[24] ^= 1;
    assert_eq!(
        MuleHullSource::from_bytes(&changed, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut changed = MEMBERSHIP.to_vec();
    changed[0] = 15;
    assert_eq!(
        MuleHullSource::from_bytes(VERTICES, &changed),
        Err(EnvelopeError::ChangedSource)
    );
    Ok(())
}

#[test]
fn mule_directional_gap_is_two_centimetres_and_engineering_is_preserved()
-> Result<(), EnvelopeError> {
    let source = MuleHullSource::bundled()?;
    assert_eq!(source.engineering(), MULE_ENGINEERING);
    assert_eq!(source.engineering().dry_mass_kilograms, 62470);
    assert_eq!(
        (
            source.engineering().scale_numerator,
            source.engineering().scale_denominator
        ),
        (40000, 3411)
    );
    assert_eq!(source.pieces.len(), 16);
    for (index, piece) in source.pieces.iter().enumerate() {
        assert_eq!(piece.rounding_metres(), 0.02);
        for x in [-1, 0, 1] {
            for y in [-1, 0, 1] {
                for z in [-1, 0, 1] {
                    if x == 0 && y == 0 && z == 0 {
                        continue;
                    }
                    let d = Vec3::new(f64::from(x), f64::from(y), f64::from(z));
                    let norm = d.dot(d).sqrt();
                    let raw = piece
                        .source_points()
                        .iter()
                        .map(|p| p.dot(d))
                        .fold(f64::NEG_INFINITY, f64::max);
                    let gap = (piece.support_bound(d)? - raw) / norm;
                    assert!((0.02 - 1e-12..=0.0200001).contains(&gap));
                }
            }
        }
        let (low, high) = piece.source_bounds();
        eprintln!(
            "MULE_ENVELOPE_PARAMETERS {{\"piece\":{index},\"kind\":\"source-convex-support-plus-ball\",\"points\":{},\"roundingMetres\":0.02,\"sourceLow\":[{},{},{}],\"sourceHigh\":[{},{},{}],\"allDirectionsSupportGapMetres\":0.02}}",
            piece.source_points().len(),
            low.x,
            low.y,
            low.z,
            high.x,
            high.y,
            high.z
        );
    }
    Ok(())
}

#[test]
fn rounded_piece_keeps_real_contacts_without_box_corner_false_blocks() -> Result<(), EnvelopeError>
{
    let shape = piece()?;
    assert!((shape.support_bound(Vec3::new(1.0, 0.0, 0.0))? - 2.02).abs() < 1e-9);
    let corner = [
        Vec3::new(1.8, 0.9, -0.1),
        Vec3::new(1.8, 0.9, 0.1),
        Vec3::new(1.9, 0.95, 0.0),
    ];
    assert!(shape.sweep_clears_triangle(pose(0.0, 0.0, 0.0), pose(0.0, 0.0, 0.0), 0.0, corner)?);
    let tangent = [
        Vec3::new(2.02, -5.0, -5.0),
        Vec3::new(2.02, 5.0, -5.0),
        Vec3::new(2.02, 0.0, 5.0),
    ];
    assert!(!shape.sweep_clears_triangle(
        pose(0.0, 0.0, 0.0),
        pose(0.0, 0.0, 0.0),
        0.0,
        tangent
    )?);
    assert!(!shape.sweep_clears_triangle(
        pose(0.0, 0.0, 0.0),
        pose(0.0, 0.0, 0.0),
        0.0,
        tangent.map(|p| Vec3::new(p.x - 0.01, p.y, p.z))
    )?);
    Ok(())
}

#[test]
fn rounded_piece_sweeps_translation_and_rotation_without_tunnelling() -> Result<(), EnvelopeError> {
    let shape = piece()?;
    let wall = [
        Vec3::new(0.0, -10.0, -10.0),
        Vec3::new(0.0, 10.0, -10.0),
        Vec3::new(0.0, 0.0, 10.0),
    ];
    assert!(shape.sweep_clears_triangle(pose(-5.0, 0.0, 0.0), pose(-5.0, 0.0, 0.0), 0.0, wall)?);
    assert!(shape.sweep_clears_triangle(pose(5.0, 0.0, 0.0), pose(5.0, 0.0, 0.0), 0.0, wall)?);
    assert!(!shape.sweep_clears_triangle(pose(-5.0, 0.0, 0.0), pose(5.0, 0.0, 0.0), 0.0, wall)?);
    let arm = SmoothPiece::new(
        vec![
            Vec3::new(3.5, -0.25, -0.25),
            Vec3::new(4.5, 0.25, 0.25),
            Vec3::new(4.5, -0.25, 0.25),
            Vec3::new(3.5, 0.25, -0.25),
        ],
        0.02,
    )?;
    let end = RigidPose {
        position: Vec3::new(0.0, 0.0, 0.0),
        orientation: Basis3 {
            x: Vec3::new(-1.0, 0.0, 0.0),
            y: Vec3::new(0.0, -1.0, 0.0),
            z: Vec3::new(0.0, 0.0, 1.0),
        },
    };
    let middle = [
        Vec3::new(-0.2, 4.0, -1.0),
        Vec3::new(0.2, 4.0, -1.0),
        Vec3::new(0.0, 4.0, 1.0),
    ];
    assert!(!arm.sweep_clears_triangle(pose(0.0, 0.0, 0.0), end, core::f64::consts::PI, middle)?);
    assert_eq!(
        arm.sweep_clears_triangle(pose(0.0, 0.0, 0.0), end, 0.0, middle),
        Err(EnvelopeError::UnderstatedRotation)
    );
    assert_eq!(
        shape.sweep_clears_triangle(pose(f64::NAN, 0.0, 0.0), end, 0.0, wall),
        Err(EnvelopeError::InvalidGeometry)
    );
    Ok(())
}

#[test]
fn mule_actual_inter_pannier_gap_stays_clear_in_the_compound_surface() -> Result<(), EnvelopeError>
{
    let source = MuleHullSource::bundled()?;
    let gap = scale(Vec3::new(-0.527, -0.08, -0.518), SCALE);
    assert!(!source.contains_local(gap));
    let obstacle = [
        add(gap, Vec3::new(0.0, -0.02, -0.02)),
        add(gap, Vec3::new(0.0, 0.02, -0.02)),
        add(gap, Vec3::new(0.0, 0.0, 0.02)),
    ];
    assert!(source.sweep_clears_triangles(
        pose(0.0, 0.0, 0.0),
        pose(0.0, 0.0, 0.0),
        0.0,
        &[obstacle]
    )?);
    Ok(())
}

#[test]
fn mule_analytic_port_fit_is_strict_rotated_and_covers_all_outer_protrusions()
-> Result<(), Box<dyn std::error::Error>> {
    let source = MuleHullSource::bundled()?;
    let bound = |d: Vec3| -> Result<f64, EnvelopeError> {
        source.pieces.iter().try_fold(f64::NEG_INFINITY, |m, p| {
            p.support_bound(d).map(|v| m.max(v))
        })
    };
    let low = Vec3::new(
        -bound(Vec3::new(-1.0, 0.0, 0.0))?,
        -bound(Vec3::new(0.0, -1.0, 0.0))?,
        0.0,
    );
    let high = Vec3::new(
        bound(Vec3::new(1.0, 0.0, 0.0))?,
        bound(Vec3::new(0.0, 1.0, 0.0))?,
        0.0,
    );
    let aperture = port(
        sub(low, Vec3::new(0.05, 0.05, 0.0)),
        add(high, Vec3::new(0.05, 0.05, 0.0)),
    )?;
    assert!(source.fits_port(&aperture, pose(0.0, 0.0, 0.0), station(), 0.0)?);
    let clipped = port(
        sub(low, Vec3::new(0.05, 0.05, 0.0)),
        Vec3::new(high.x - 0.001, high.y + 0.05, 0.0),
    )?;
    assert!(!source.fits_port(&clipped, pose(0.0, 0.0, 0.0), station(), 0.0)?);
    let station = StationPose {
        pose: pose(31.0, -7.0, 5.0),
        axial_rotation: core::f64::consts::FRAC_PI_2,
        rotation_rate: 0.2,
    };
    let player = RigidPose {
        position: station.pose.position,
        orientation: Basis3 {
            x: Vec3::new(0.0, 1.0, 0.0),
            y: Vec3::new(-1.0, 0.0, 0.0),
            z: Vec3::new(0.0, 0.0, 1.0),
        },
    };
    assert!(source.fits_port(&aperture, player, station, 0.0)?);
    assert!(!source.fits_port(&aperture, pose(1000.0, 0.0, 0.0), station, 0.0)?);
    assert_eq!(
        source.fits_port(&aperture, player, station, -0.1),
        Err(EnvelopeError::InvalidGeometry)
    );
    Ok(())
}

#[test]
fn mule_shield_depletion_hides_the_subtle_shimmer_without_retracting_collision()
-> Result<(), EnvelopeError> {
    let source = MuleHullSource::bundled()?;
    let charged = source.presentation(24000, 12000, 24000)?;
    let depleted = source.presentation(0, 0, 24000)?;
    assert!(Arc::ptr_eq(&charged.pieces, &depleted.pieces));
    assert_eq!(charged.pieces, source.pieces);
    assert_eq!(
        charged.shimmer_alpha(Vec3::new(0.0, 0.0, 1.0), 10.0, 1.0),
        0.0
    );
    assert_eq!(
        charged.shimmer_alpha(Vec3::new(0.0, 0.0, 1.0), 65.0, 0.0),
        0.0
    );
    assert_eq!(
        charged.shimmer_alpha(Vec3::new(0.0, 0.0, 1.0), 10.0, 0.0),
        0.018
    );
    assert_eq!(
        charged.shimmer_alpha(Vec3::new(0.0, 0.0, -1.0), 10.0, 0.0),
        0.009
    );
    assert_eq!(
        depleted.shimmer_alpha(Vec3::new(0.0, 0.0, 1.0), 10.0, 0.0),
        0.0
    );
    assert_eq!(
        charged.shimmer_alpha(Vec3::new(0.0, 0.0, 1.0), f32::NAN, 0.0),
        0.0
    );
    assert_eq!(
        source.presentation(24001, 0, 24000),
        Err(EnvelopeError::InvalidShield)
    );
    assert_eq!(
        source.presentation(0, 0, 0),
        Err(EnvelopeError::InvalidShield)
    );
    Ok(())
}

#[test]
fn mule_launch_uses_its_surface_without_relabelling_sh025_or_granting_arrival()
-> Result<(), Box<dyn std::error::Error>> {
    let source = MuleHullSource::bundled()?;
    let data = OriginalStationData::bundled()?;
    let launch = data.mule_launch_start(&source)?;
    let before = crate::station::flight_pose(launch.position, launch.orientation)
        .ok_or(EnvelopeError::InvalidGeometry)?;
    let after = RigidPose {
        position: add(before.position, Vec3::new(0.0, 0.0, 10.0)),
        orientation: before.orientation,
    };
    assert!(data.mule_motion_clear(&source, before, after, 0.0));
    assert!(!data.mule_entry_clear(&source, before, after, 144.0, 0.0, 0.0));
    assert_eq!(
        crate::station::HULL_SOURCE_SHA256,
        "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883"
    );
    Ok(())
}

#[test]
fn rounded_ray_contacts_require_an_inside_witness_and_leave_tangency_uncertain()
-> Result<(), EnvelopeError> {
    let shape = piece()?;
    let hit = shape.ray_contact(
        pose(0.0, 0.0, 0.0),
        Vec3::new(-5.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        10.0,
    )?;
    let EnvelopeRayContact::Surface { distance, point } = hit else {
        return Err(EnvelopeError::InvalidGeometry);
    };
    assert!((distance - 2.98).abs() < 1e-4);
    assert!((point.x + 2.02).abs() < 1e-4);
    assert!(shape.contains(point));
    assert_eq!(
        shape.ray_contact(
            pose(0.0, 0.0, 0.0),
            Vec3::new(1.8, 0.9, -10.0),
            Vec3::new(0.0, 0.0, 1.0),
            20.0
        )?,
        EnvelopeRayContact::Clear
    );
    assert!(matches!(
        shape.ray_contact(
            pose(0.0, 0.0, 0.0),
            Vec3::new(2.02, 0.0, -10.0),
            Vec3::new(0.0, 0.0, 1.0),
            20.0
        )?,
        EnvelopeRayContact::Uncertain { .. }
    ));
    assert_eq!(
        shape.ray_contact(
            pose(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            20.0
        ),
        Err(EnvelopeError::InvalidGeometry)
    );
    Ok(())
}

#[test]
fn rounded_support_invalid_geometry_never_grants_clearance_or_a_contact()
-> Result<(), EnvelopeError> {
    assert_eq!(
        SmoothPiece::new(Vec::new(), 0.02),
        Err(EnvelopeError::InvalidContent)
    );
    assert_eq!(
        SmoothPiece::new(vec![Vec3::new(f64::INFINITY, 0.0, 0.0)], 0.02),
        Err(EnvelopeError::InvalidContent)
    );
    assert_eq!(
        SmoothPiece::new(vec![Vec3::new(0.0, 0.0, 0.0)], 0.03),
        Err(EnvelopeError::InvalidContent)
    );
    let shape = piece()?;
    assert!(!shape.contains(Vec3::new(f64::NAN, 0.0, 0.0)));
    assert_eq!(
        shape.support_bound(Vec3::new(f64::NAN, 0.0, 0.0)),
        Err(EnvelopeError::InvalidGeometry)
    );
    let triangle = [Vec3::new(100.0, 0.0, 0.0); 3];
    assert_eq!(
        shape.sweep_clears_triangle(
            pose(0.0, 0.0, 0.0),
            pose(0.0, 0.0, 0.0),
            f64::INFINITY,
            triangle
        ),
        Err(EnvelopeError::InvalidGeometry)
    );
    let mut stretched = pose(0.0, 0.0, 0.0);
    stretched.orientation.x.x = 1.00001;
    assert_eq!(
        shape.sweep_clears_triangle(pose(0.0, 0.0, 0.0), stretched, 0.0, triangle),
        Err(EnvelopeError::InvalidGeometry)
    );
    let source = MuleHullSource::bundled()?;
    assert_eq!(
        source.sweep_clears_triangles(pose(0.0, 0.0, 0.0), pose(0.0, 0.0, 0.0), 0.0, &[]),
        Err(EnvelopeError::InvalidGeometry)
    );
    Ok(())
}

#[test]
fn rounded_sweep_accounts_for_near_orthogonal_trace_three_endpoint() -> Result<(), EnvelopeError> {
    let shape = SmoothPiece::new(vec![Vec3::new(20.0, 0.0, 0.0)], 0.02)?;
    let start = pose(0.0, 0.0, 0.0);
    let mut end = start;
    end.orientation = Basis3 {
        x: Vec3::new(1.0, 0.000005, 0.0),
        y: Vec3::new(-0.000005, 1.0, 0.0),
        z: Vec3::new(0.0, 0.0, 1.0),
    };
    // Independent witness: the entire triangle lies inside the end source ball.
    // The previous trace/acos check saw angle zero and its broadphase cleared it.
    let triangle = [
        Vec3::new(19.9999, 0.02007, -0.0001),
        Vec3::new(20.0001, 0.02007, -0.0001),
        Vec3::new(20.0, 0.02007, 0.0001),
    ];
    let center = Vec3::new(20.0, 0.0001, 0.0);
    for vertex in triangle {
        let delta = Vec3::new(
            vertex.x - center.x,
            vertex.y - center.y,
            vertex.z - center.z,
        );
        assert!(delta.dot(delta) < 0.02 * 0.02);
    }
    assert!(geometry::valid_pose(end));
    assert_eq!(
        shape.sweep_clears_triangle(start, end, 0.0, triangle),
        Err(EnvelopeError::UnderstatedRotation)
    );
    assert!(!shape.sweep_clears_triangle(start, end, 0.000005, triangle)?);

    // Proper small rotations still admit clearance when their travel is supplied.
    let (sine, cosine) = 0.000005_f64.sin_cos();
    end.orientation.x = Vec3::new(cosine, sine, 0.0);
    end.orientation.y = Vec3::new(-sine, cosine, 0.0);
    assert!(!shape.sweep_clears_triangle(start, end, 0.000005, triangle)?);
    let clear_triangle = triangle.map(|p| Vec3::new(p.x, 1.0, p.z));
    assert!(shape.sweep_clears_triangle(start, end, 0.000005, clear_triangle)?);
    let angle = std::f64::consts::PI - 0.00001;
    let (sine, cosine) = angle.sin_cos();
    end.orientation.x = Vec3::new(cosine, sine, 0.0);
    end.orientation.y = Vec3::new(-sine, cosine, 0.0);
    let distant_triangle = triangle.map(|p| Vec3::new(p.x, 100.0, p.z));
    assert!(shape.sweep_clears_triangle(start, end, angle, distant_triangle)?);
    Ok(())
}

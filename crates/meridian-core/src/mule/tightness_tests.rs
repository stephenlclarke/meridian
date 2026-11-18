//! Sampled detailed-surface gaps, not a maximum-distance/Hausdorff certificate.
use super::geometry::scale;
use super::*;

// Original detailed creator-component outer well-rim starts; copied from COMPONENT-PIECES.
const WELL_RIMS: &[(usize, usize)] = &[
    (0, 120),
    (0, 156),
    (0, 192),
    (0, 228),
    (0, 264),
    (0, 300),
    (0, 336),
    (0, 372),
    (0, 408),
    (0, 444),
    (0, 480),
    (0, 516),
    (2, 696),
    (2, 732),
    (2, 768),
    (2, 804),
    (2, 840),
    (2, 876),
    (2, 912),
    (2, 948),
    (2, 984),
    (2, 1020),
    (2, 1056),
    (2, 1092),
    (4, 1272),
    (4, 1308),
    (4, 1344),
    (4, 1380),
    (4, 1416),
    (4, 1452),
    (4, 1488),
    (4, 1524),
    (4, 1560),
    (4, 1596),
    (4, 1632),
    (4, 1668),
    (6, 1848),
    (6, 1884),
    (6, 1920),
    (6, 1956),
    (6, 1992),
    (6, 2028),
    (6, 2064),
    (6, 2100),
    (6, 2136),
    (6, 2172),
    (6, 2208),
    (6, 2244),
    (8, 2424),
    (8, 2460),
    (8, 2496),
    (8, 2532),
    (8, 2568),
    (8, 2604),
    (8, 2640),
    (8, 2676),
    (8, 2712),
    (8, 2748),
    (8, 2784),
    (8, 2820),
    (10, 3000),
    (10, 3036),
    (10, 3072),
    (10, 3108),
    (10, 3144),
    (10, 3180),
    (10, 3216),
    (10, 3252),
    (10, 3288),
    (10, 3324),
    (10, 3360),
    (10, 3396),
    (12, 3576),
    (12, 3612),
    (12, 3648),
    (12, 3684),
    (12, 3720),
    (12, 3756),
    (12, 3792),
    (12, 3828),
    (12, 3864),
    (12, 3900),
    (12, 3936),
    (12, 3972),
];

fn segment_distance_squared(p: Vec3, a: Vec3, b: Vec3) -> f64 {
    let edge = sub(b, a);
    let squared = edge.dot(edge);
    if squared <= f64::MIN_POSITIVE {
        return sub(p, a).dot(sub(p, a));
    }
    let t = (sub(p, a).dot(edge) / squared).clamp(0.0, 1.0);
    let delta = sub(p, add(a, scale(edge, t)));
    delta.dot(delta)
}
fn triangle_distance_squared(p: Vec3, triangle: [Vec3; 3]) -> f64 {
    let [a, b, c] = triangle;
    let u = sub(b, a);
    let v = sub(c, a);
    let q = sub(p, a);
    let uu = u.dot(u);
    let vv = v.dot(v);
    let uv = u.dot(v);
    let determinant = uu * vv - uv * uv;
    let edge_distance = segment_distance_squared(p, a, b)
        .min(segment_distance_squared(p, b, c))
        .min(segment_distance_squared(p, c, a));
    if determinant <= 1e-20 * (1.0 + uu * vv) {
        return edge_distance;
    }
    let s = (q.dot(u) * vv - q.dot(v) * uv) / determinant;
    let t = (q.dot(v) * uu - q.dot(u) * uv) / determinant;
    if s >= 0.0 && t >= 0.0 && s + t <= 1.0 {
        let delta = sub(q, add(scale(u, s), scale(v, t)));
        edge_distance.min(delta.dot(delta))
    } else {
        edge_distance
    }
}
fn detailed_triangles() -> Result<Vec<[Vec3; 3]>, EnvelopeError> {
    include_bytes!("../../../../assets/fleet/sh001/source-triangles.bin")[..7236 * 12]
        .as_chunks::<12>()
        .0
        .iter()
        .map(|record| {
            let mut points = [Vec3::new(0.0, 0.0, 0.0); 3];
            for (axis, p) in points.iter_mut().enumerate() {
                let index = u32::from_le_bytes(
                    record[axis * 4..axis * 4 + 4]
                        .try_into()
                        .map_err(|_| EnvelopeError::InvalidContent)?,
                ) as usize;
                *p = source_vertex(VERTICES, index)?;
            }
            Ok(points)
        })
        .collect()
}

#[test]
fn mule_sampled_visible_envelope_excess_reports_actual_detailed_surface_gaps()
-> Result<(), EnvelopeError> {
    // Independent analytic face/edge/vertex/degenerate metric checks.
    let face = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    assert_eq!(
        triangle_distance_squared(Vec3::new(0.5, 0.5, 3.0), face),
        9.0
    );
    assert_eq!(
        triangle_distance_squared(Vec3::new(1.0, -2.0, 0.0), face),
        4.0
    );
    assert_eq!(
        triangle_distance_squared(Vec3::new(-1.0, -1.0, 0.0), face),
        2.0
    );
    assert_eq!(
        triangle_distance_squared(Vec3::new(0.0, 0.0, 3.0), [face[0]; 3]),
        9.0
    );
    let source = MuleHullSource::bundled()?;
    let triangles = detailed_triangles()?;
    let pose = RigidPose {
        position: Vec3::new(0.0, 0.0, 0.0),
        orientation: Basis3::IDENTITY,
    };
    for (index, piece) in source.pieces.iter().enumerate() {
        let mut directions = Vec::new();
        for x in [-1, 0, 1] {
            for y in [-1, 0, 1] {
                for z in [-1, 0, 1] {
                    if x != 0 || y != 0 || z != 0 {
                        directions.push(Vec3::new(f64::from(x), f64::from(y), f64::from(z)));
                    }
                }
            }
        }
        for &(group, first) in WELL_RIMS {
            if group == index {
                let mut center = Vec3::new(0.0, 0.0, 0.0);
                for vertex in first..first + 4 {
                    center = add(center, scale(source_vertex(VERTICES, vertex)?, 0.25));
                }
                directions.push(sub(center, piece.center()));
            }
        }
        let mut samples = 0;
        let mut uncertain = 0;
        let mut internal = 0;
        let mut clear = 0;
        let mut largest = 0.0_f64;
        let mut largest_point = Vec3::new(0.0, 0.0, 0.0);
        let mut nearest_face = 0;
        let range = 8.0 * piece.origin_radius();
        for d in directions {
            let norm = d.dot(d).sqrt();
            assert!(norm > 0.0);
            let d = scale(d, 1.0 / norm);
            let origin = add(piece.center(), scale(d, 0.5 * range));
            match piece.ray_contact(pose, origin, scale(d, -1.0), range)? {
                EnvelopeRayContact::Surface { point, .. } => {
                    if source
                        .pieces
                        .iter()
                        .enumerate()
                        .any(|(other, p)| other != index && p.contains(point))
                    {
                        internal += 1;
                        continue;
                    }
                    samples += 1;
                    let mut distance = f64::INFINITY;
                    let mut closest_face = 0;
                    for (face_index, triangle) in triangles.iter().enumerate() {
                        let candidate = triangle_distance_squared(point, *triangle);
                        if candidate < distance {
                            distance = candidate;
                            closest_face = face_index;
                        }
                    }
                    assert!(distance.is_finite() && distance >= 0.0);
                    if samples == 1 || distance.sqrt() > largest {
                        largest = distance.sqrt();
                        largest_point = point;
                        nearest_face = closest_face;
                    }
                }
                EnvelopeRayContact::Uncertain { .. } => uncertain += 1,
                EnvelopeRayContact::Clear => clear += 1,
            }
        }
        assert!(
            samples > 0,
            "piece {index} has no confirmed visible tightness samples"
        );
        let (low, high) = piece.source_bounds();
        eprintln!(
            "MULE_DETAILED_GAP_SAMPLES {{\"piece\":{index},\"confirmedVisibleSamples\":{samples},\"uncertain\":{uncertain},\"internalSkipped\":{internal},\"clearRays\":{clear},\"largestSampledDistanceMetres\":{largest},\"exceedsTwoCentimetres\":{},\"samplePoint\":[{},{},{}],\"nearestDetailedFace\":{nearest_face},\"sourceLow\":[{},{},{}],\"sourceHigh\":[{},{},{}],\"envelopeAxialPaddingMetres\":0.02,\"certifiedMaximumOrAcceptance\":false}}",
            largest > 0.020001,
            largest_point.x,
            largest_point.y,
            largest_point.z,
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
fn mule_preserves_named_pannier_cockpit_and_engine_source_gaps() -> Result<(), EnvelopeError> {
    let source = MuleHullSource::bundled()?;
    let pose = RigidPose {
        position: Vec3::new(0.0, 0.0, 0.0),
        orientation: Basis3::IDENTITY,
    };
    // Source coordinates: four inter-pannier slots, the forward cockpit/body
    // opening and the low rear space between the separately assigned engines.
    let gaps = [
        ("negative-rear-pannier", [-0.527, -0.08, -0.518]),
        ("positive-rear-pannier", [0.527, -0.08, -0.518]),
        ("negative-front-pannier", [-0.527, -0.08, 0.518]),
        ("positive-front-pannier", [0.527, -0.08, 0.518]),
        ("forward-cockpit-body", [0.0, 0.48, 1.148]),
        ("between-rear-engines", [0.0, 0.0, -1.69]),
    ];
    for (name, point) in gaps {
        let center = scale(Vec3::new(point[0], point[1], point[2]), SCALE);
        assert!(
            !source.contains_local(center),
            "source gap {name} was filled"
        );
        let triangle = [
            add(center, Vec3::new(-0.01, -0.01, 0.0)),
            add(center, Vec3::new(0.01, -0.01, 0.0)),
            add(center, Vec3::new(0.0, 0.01, 0.0)),
        ];
        assert!(
            source.sweep_clears_triangles(pose, pose, 0.0, &[triangle])?,
            "source gap {name} blocks a small physical triangle"
        );
        eprintln!(
            "MULE_NAMED_GAP {{\"name\":\"{name}\",\"sourcePoint\":[{},{},{}],\"triangleSideExtentMetres\":0.02,\"clear\":true}}",
            point[0], point[1], point[2]
        );
    }
    Ok(())
}

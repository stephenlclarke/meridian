//! Sampled detailed-surface gaps, not a maximum-distance/Hausdorff certificate.
use super::geometry::{add, scale, sub};
use super::{
    EnvelopeError, EnvelopeRayContact, MuleHullSource, SCALE, SmoothPiece, VERTICES, source_vertex,
};
use crate::docking::{Basis3, RigidPose, Vec3};

// Original detailed creator-component outer well-rim starts; copied from COMPONENT-PIECES.
const FORMER_BODY_BRIDGE: Vec3 =
    Vec3::new(-1.9652283480888286, 1.5538569641363296, 1.9641025785197854);

const WELL_RIMS: &[(usize, usize)] = &[
    (16, 120),
    (17, 156),
    (18, 192),
    (19, 228),
    (20, 264),
    (21, 300),
    (22, 336),
    (23, 372),
    (24, 408),
    (25, 444),
    (26, 480),
    (27, 516),
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

fn largest_detailed_gap(points: &[Vec3], triangles: &[[Vec3; 3]]) -> Option<(f64, Vec3, usize)> {
    let mut largest: Option<(f64, Vec3, usize)> = None;
    for point in points {
        let mut distance = f64::INFINITY;
        let mut closest_face = 0;
        for (face_index, triangle) in triangles.iter().enumerate() {
            let candidate = triangle_distance_squared(*point, *triangle);
            if candidate < distance {
                distance = candidate;
                closest_face = face_index;
            }
        }
        assert!(distance.is_finite() && distance >= 0.0);
        let distance = distance.sqrt();
        if largest.is_none_or(|(old, _, _)| distance > old) {
            largest = Some((distance, *point, closest_face));
        }
    }
    largest
}

// A convex combination of maximizing source points is an inside witness. Adding
// almost the ball radius along its support normal also locates it within a tiny
// explicit support-plane error of the boundary. These are measurement fixtures,
// never a ray/effect capability or a change to uncertain gameplay ray results.
#[derive(Clone, Copy)]
struct BoundaryWitness {
    point: Vec3,
    boundary_error: f64,
}
impl BoundaryWitness {
    fn excludes(self, other: &SmoothPiece) -> bool {
        other.excludes_neighbourhood(self.point, self.boundary_error)
    }
}

fn support_boundary_samples(piece: &SmoothPiece, directions: &[Vec3]) -> Vec<BoundaryWitness> {
    let mut samples = Vec::new();
    let guard = 2048.0 * f64::EPSILON * (1.0 + piece.origin_radius());
    for direction in directions {
        let n = scale(*direction, 1.0 / direction.dot(*direction).sqrt());
        let support = piece
            .source_points()
            .iter()
            .map(|p| p.dot(n))
            .fold(f64::NEG_INFINITY, f64::max);
        let mut face = Vec::new();
        for point in piece.source_points() {
            if support - point.dot(n) <= guard && !face.contains(point) {
                face.push(*point);
            }
        }
        assert!(!face.is_empty());
        // Four spread source vertices keep the normal qualification bounded.
        let chosen: Vec<_> = if face.len() <= 4 {
            face
        } else {
            (0..4).map(|i| face[i * (face.len() - 1) / 3]).collect()
        };
        let mut witness = |q: Vec3| {
            let point = add(q, scale(n, piece.rounding_metres() - 4.0 * guard));
            let displacement = sub(point, q);
            assert!(displacement.dot(displacement) <= piece.rounding_metres().powi(2));
            let deficit = support + piece.rounding_metres() - point.dot(n);
            assert!((-guard..=16.0 * guard).contains(&deficit));
            // The outward unit ray from this inside witness must cross the
            // member boundary by the support plane. Allow extra error for
            // normalization, projection, and the measured witness deficit.
            samples.push(BoundaryWitness {
                point,
                boundary_error: 32.0 * guard,
            });
        };
        for (i, a) in chosen.iter().enumerate() {
            witness(*a);
            for b in chosen.iter().skip(i + 1) {
                for step in 1..8 {
                    let t = f64::from(step) / 8.0;
                    witness(add(scale(*a, 1.0 - t), scale(*b, t)));
                }
            }
        }
    }
    samples
}

fn sampling_directions(index: usize, piece: &SmoothPiece) -> Result<Vec<Vec3>, EnvelopeError> {
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
    Ok(directions)
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
    let mut visible_piece_count = 0;
    // Keep the historical distance and report whether any current piece still covers it.
    let observed = FORMER_BODY_BRIDGE;
    let observed_gap = triangles
        .iter()
        .map(|t| triangle_distance_squared(observed, *t))
        .fold(f64::INFINITY, f64::min)
        .sqrt();
    assert!((observed_gap - 0.6596408669309312).abs() < 1e-9);
    eprintln!(
        "MULE_KNOWN_TIGHTNESS_BLOCKER {{\"observedDistanceMetres\":{observed_gap},\"pointStillEnclosed\":{},\"maximumOrAcceptance\":false}}",
        source.contains_local(observed)
    );
    for (index, piece) in source.pieces.iter().enumerate() {
        let directions = sampling_directions(index, piece)?;
        let boundary_samples = support_boundary_samples(piece, &directions);
        let certified_boundary_samples = boundary_samples.len();
        assert!(
            certified_boundary_samples > 0,
            "piece {index} has no independently certified boundary samples"
        );
        let mut ray_samples = 0;
        let mut ray_not_point_covered = 0;
        let mut support_unresolved = 0;
        let mut support_samples = 0;
        let mut visible_points = Vec::new();
        let mut uncertain = 0;
        let mut internal = 0;
        let mut clear = 0;
        let range = 8.0 * piece.origin_radius();
        for d in directions {
            let norm = d.dot(d).sqrt();
            assert!(norm > 0.0);
            let d = scale(d, 1.0 / norm);
            let origin = add(piece.center(), scale(d, 0.5 * range));
            match piece.ray_contact(pose, origin, scale(d, -1.0), range)? {
                EnvelopeRayContact::Surface { point, .. } => {
                    ray_samples += 1;
                    if source
                        .pieces
                        .iter()
                        .enumerate()
                        .any(|(other, p)| other != index && p.contains(point))
                    {
                        internal += 1;
                        continue;
                    }
                    ray_not_point_covered += 1;
                }
                EnvelopeRayContact::Uncertain { .. } => uncertain += 1,
                EnvelopeRayContact::Clear => clear += 1,
            }
        }
        for witness in boundary_samples {
            if source
                .pieces
                .iter()
                .enumerate()
                .any(|(other, piece)| other != index && !witness.excludes(piece))
            {
                support_unresolved += 1;
                continue;
            }
            support_samples += 1;
            visible_points.push(witness.point);
        }
        let samples = visible_points.len();
        let largest = largest_detailed_gap(&visible_points, &triangles);
        assert_eq!(largest.is_some(), samples > 0);
        // A union may conceal a member completely. Do not fabricate a visible
        // sample or call this coverage complete when all candidates were excluded.
        let visible_coverage_established = samples > 0;
        visible_piece_count += usize::from(visible_coverage_established);
        let (low, high) = piece.source_bounds();
        let distance_json = largest.map_or_else(|| "null".to_string(), |(d, _, _)| d.to_string());
        let point_json = largest.map_or_else(
            || "null".to_string(),
            |(_, p, _)| format!("[{},{},{}]", p.x, p.y, p.z),
        );
        let face_json = largest.map_or_else(|| "null".to_string(), |(_, _, f)| f.to_string());
        let excess_json = largest.map_or_else(
            || "null".to_string(),
            |(d, _, _)| (d > 0.020001).to_string(),
        );
        eprintln!(
            "MULE_DETAILED_GAP_SAMPLES {{\"piece\":{index},\"certifiedPieceBoundarySamples\":{certified_boundary_samples},\"confirmedVisibleSamples\":{samples},\"visibleCoverageEstablished\":{visible_coverage_established},\"rayInsideWitnesses\":{ray_samples},\"rayInsideWitnessesNotPointCovered\":{ray_not_point_covered},\"supportExposureUnresolved\":{support_unresolved},\"confirmedSupportWitnessSamples\":{support_samples},\"uncertain\":{uncertain},\"internalSkipped\":{internal},\"clearRays\":{clear},\"largestSampledDistanceMetres\":{distance_json},\"exceedsTwoCentimetres\":{excess_json},\"samplePoint\":{point_json},\"nearestDetailedFace\":{face_json},\"sourceLow\":[{},{},{}],\"sourceHigh\":[{},{},{}],\"envelopeAxialPaddingMetres\":0.02,\"certifiedMaximumOrAcceptance\":false}}",
            low.x, low.y, low.z, high.x, high.y, high.z
        );
    }
    assert!(
        visible_piece_count > 0,
        "no actual visible union surface was measured"
    );
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

#[test]
fn compound_gap_sampling_does_not_invent_visibility_for_an_internal_member()
-> Result<(), EnvelopeError> {
    let inner = SmoothPiece::new(vec![Vec3::new(0.0, 0.0, 0.0)], 0.02)?;
    let outer = SmoothPiece::new(
        vec![
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
        0.02,
    )?;
    let directions = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    ];
    let inner_samples = support_boundary_samples(&inner, &directions);
    assert!(!inner_samples.is_empty());
    // Independent analytic containment in the unrounded unit octahedron:
    // |x|+|y|+|z|<1. A valid compound can have no exposed inner boundary.
    for witness in &inner_samples {
        let p = witness.point;
        assert!(p.x.abs() + p.y.abs() + p.z.abs() < 1.0);
        assert!(outer.contains(p));
    }
    let inner_visible = inner_samples
        .iter()
        .filter(|witness| witness.excludes(&outer))
        .count();
    assert_eq!(inner_visible, 0);
    let outer_samples = support_boundary_samples(&outer, &directions);
    assert!(!outer_samples.is_empty());
    let outer_visible = outer_samples
        .iter()
        .filter(|witness| witness.excludes(&inner))
        .count();
    assert_eq!(outer_visible, outer_samples.len());
    assert!(outer_visible > 0);
    let triangle = [[
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]];
    assert_eq!(largest_detailed_gap(&[], &triangle), None);
    assert!(
        largest_detailed_gap(
            &outer_samples
                .iter()
                .map(|witness| witness.point)
                .collect::<Vec<_>>(),
            &triangle
        )
        .is_some()
    );
    // This proves the fixture's scope, not coverage/hiddenness of any actual Mule
    // member. Real zero-visible records remain explicit unresolved acceptance data.
    Ok(())
}

#[test]
fn mule_pressure_body_well_partition_clears_the_recorded_convex_bridge() -> Result<(), EnvelopeError>
{
    let source = MuleHullSource::bundled()?;
    let point = FORMER_BODY_BRIDGE;
    let triangle = [
        add(point, Vec3::new(-0.01, -0.01, 0.0)),
        add(point, Vec3::new(0.01, -0.01, 0.0)),
        add(point, Vec3::new(0.0, 0.01, 0.0)),
    ];
    let unchanged_pose = RigidPose {
        position: Vec3::new(0.0, 0.0, 0.0),
        orientation: Basis3::IDENTITY,
    };
    let body_pieces = source
        .pieces()
        .iter()
        .enumerate()
        .filter(|(index, _)| *index == 0 || *index >= 16);
    for (index, piece) in body_pieces {
        let separation_axis = if index == 0 {
            // The original body skin lies inward of the recorded low-left point.
            Vec3::new(-1.0, -1.0, 0.0)
        } else if index >= 32 {
            Vec3::new(-1.0, 0.0, 0.0)
        } else if piece.source_bounds().1.z < point.z {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            Vec3::new(0.0, 0.0, -1.0)
        };
        let support = piece.support_bound(separation_axis)?;
        for vertex in triangle {
            assert!(
                vertex.dot(separation_axis) > support,
                "body piece {index} bridges the measured opening"
            );
        }
        assert!(
            !piece.contains(point),
            "body piece {index} still contains the measured point"
        );
        assert!(
            piece.sweep_clears_triangle(unchanged_pose, unchanged_pose, 0.0, triangle)?,
            "body piece {index} blocks the physical triangle at the old bridge"
        );
    }
    Ok(())
}

#[test]
fn near_boundary_measurement_requires_clearance_across_seam_error() -> Result<(), EnvelopeError> {
    let radius = 0.02;
    let origin = Vec3::new(0.0, 0.0, 0.0);
    let normal = Vec3::new(1.0, 0.0, 0.0);
    let member = SmoothPiece::new(vec![origin], radius)?;
    let guard = 2048.0 * f64::EPSILON * (1.0 + radius);
    let other_center = 2.0 * radius - 2.0 * guard;
    let neighbour = SmoothPiece::new(vec![Vec3::new(other_center, 0.0, 0.0)], radius)?;
    let witness = support_boundary_samples(&member, &[normal])[0];
    assert!(other_center - witness.point.x > radius);
    assert!(other_center - radius < radius);
    assert!(radius - witness.point.x < witness.boundary_error);
    assert!(
        !neighbour.contains(witness.point),
        "point-only check would count the sample"
    );
    assert!(
        !witness.excludes(&neighbour),
        "the boundary neighbourhood reaches the other piece"
    );
    let separate = SmoothPiece::new(vec![Vec3::new(0.05, 0.0, 0.0)], radius)?;
    assert!(witness.excludes(&separate));
    Ok(())
}

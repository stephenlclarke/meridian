use super::{
    ARCHIVED_NATIVE_VERTICES, CANDIDATE_NATIVE_VERTICES, DETAILED_VERTICES, MEMBERSHIP,
    SHIKRA_PIECES, ShikraHullSource, VERTICES, decode_point,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh094/source-triangles.bin");

#[test]
fn shikra_all_three_meshes_share_complete_local_supports() -> Result<(), EnvelopeError> {
    let hull = ShikraHullSource::bundled()?;
    let points = VERTICES[20..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), SHIKRA_PIECES);
    assert_eq!(
        points.len(),
        DETAILED_VERTICES + ARCHIVED_NATIVE_VERTICES + CANDIDATE_NATIVE_VERTICES
    );
    assert_eq!(TRIANGLES.len(), (5396 + 1404 + 512) * 12);
    for (point, &slot) in points.iter().zip(MEMBERSHIP) {
        let piece = &hull.pieces()[usize::from(slot)];
        assert!(piece.source_points().contains(&decode_point(point)));
        assert_eq!(piece.rounding_metres(), 0.02);
    }
    for face in TRIANGLES.as_chunks::<12>().0 {
        let [a, b, c] = face.as_chunks::<4>().0 else {
            return Err(EnvelopeError::InvalidContent);
        };
        let indices = [*a, *b, *c].map(|i| u32::from_le_bytes(i) as usize);
        let [a, b, c] = indices.map(|i| decode_point(&points[i]));
        let center = Vec3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        let slot = MEMBERSHIP[indices[0]];
        assert!(indices.iter().all(|&i| MEMBERSHIP[i] == slot));
        assert!(hull.pieces()[usize::from(slot)].contains(center));
    }
    let mut rewritten = VERTICES.to_vec();
    rewritten[20] ^= 1;
    assert_eq!(
        ShikraHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        ShikraHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        ShikraHullSource::from_bytes(VERTICES, &shifted),
        Err(EnvelopeError::ChangedSource)
    );
    Ok(())
}

fn translated(z: f64) -> RigidPose {
    RigidPose {
        position: Vec3::new(0.0, 0.0, z),
        orientation: Basis3::IDENTITY,
    }
}

#[test]
fn shikra_aft_and_exterior_routes_clear() -> Result<(), EnvelopeError> {
    let hull = ShikraHullSource::bundled()?;
    let start = translated(0.0);
    for (x, y, z, travel) in [
        (0.0, 7.6, -17.0, -33.0),
        (0.0, -7.6, -17.0, -33.0),
        (0.0, 3.5, -17.0, -6.7),
        (-13.7, 0.0, -17.0, -33.0),
        (13.7, 0.0, -17.0, -33.0),
        (-6.2, 0.0, -6.0, -21.5),
        (6.2, 0.0, -6.0, -21.5),
    ] {
        let triangle = [
            Vec3::new(x - 0.03, y, z),
            Vec3::new(x + 0.03, y, z),
            Vec3::new(x, y + 0.06, z),
        ];
        assert!(hull.sweep_clears_triangle(start, translated(travel), 0.0, triangle)?);
    }
    for index in [0, 2, 300, 3240, 4024, 4028] {
        let point = decode_point(&VERTICES[20..].as_chunks::<24>().0[index]);
        let triangle = [
            point,
            Vec3::new(point.x + 0.001, point.y, point.z),
            Vec3::new(point.x, point.y + 0.001, point.z),
        ];
        assert!(!hull.sweep_clears_triangle(start, start, 0.0, triangle)?);
    }
    Ok(())
}

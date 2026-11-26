use super::{
    DETAILED_VERTICES, KESTREL_PIECES, KestrelHullSource, MEMBERSHIP, NATIVE_VERTICES, VERTICES,
    decode_point,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh145/source-triangles.bin");

#[test]
fn kestrel_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = KestrelHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), KESTREL_PIECES);
    assert_eq!(points.len(), DETAILED_VERTICES + NATIVE_VERTICES);
    assert_eq!(TRIANGLES.len(), (2316 + 128) * 12);
    for (index, slot) in MEMBERSHIP.iter().copied().enumerate() {
        let point = decode_point(&points[index]);
        let piece = &hull.pieces()[usize::from(slot)];
        assert!(piece.source_points().contains(&point));
        assert_eq!(piece.rounding_metres(), 0.02);
    }
    for face in TRIANGLES.as_chunks::<12>().0 {
        let [a, b, c] = face.as_chunks::<4>().0 else {
            return Err(EnvelopeError::InvalidContent);
        };
        let indices = [*a, *b, *c].map(|i| u32::from_le_bytes(i) as usize);
        let slot = MEMBERSHIP[indices[0]];
        assert!(indices.iter().all(|&i| MEMBERSHIP[i] == slot));
        let [a, b, c] = indices.map(|i| decode_point(&points[i]));
        let center = Vec3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        assert!(hull.pieces()[usize::from(slot)].contains(center));
    }
    let mut rewritten = VERTICES.to_vec();
    rewritten[16] ^= 1;
    assert_eq!(
        KestrelHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let other = include_bytes!("../../../../assets/fleet/sh134/source.vertices");
    assert_eq!(
        KestrelHullSource::from_bytes(other, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        KestrelHullSource::from_bytes(VERTICES, &shifted),
        Err(EnvelopeError::ChangedSource)
    );
    Ok(())
}

fn translated(position: Vec3) -> RigidPose {
    RigidPose {
        position,
        orientation: Basis3::IDENTITY,
    }
}

#[test]
fn kestrel_routes_clear() -> Result<(), EnvelopeError> {
    let hull = KestrelHullSource::bundled()?;
    let start = translated(Vec3::new(0.0, 0.0, 0.0));
    for (x, y, z, travel) in [
        (0.0, 6.8, -18.0, -33.0),
        (0.0, -4.6, -18.0, -33.0),
        (0.0, -1.0, -18.0, -5.1),
        (-11.5, 0.0, -18.0, -33.0),
        (11.5, 0.0, -18.0, -33.0),
        (-7.0, 3.4, -18.0, -33.0),
        (7.0, 3.4, -18.0, -33.0),
    ] {
        let triangle = [
            Vec3::new(x - 0.03, y, z),
            Vec3::new(x + 0.03, y, z),
            Vec3::new(x, y + 0.06, z),
        ];
        assert!(hull.sweep_clears_triangle(
            start,
            translated(Vec3::new(0.0, 0.0, travel)),
            0.0,
            triangle,
        )?);
    }
    for index in [0, 8, 224, DETAILED_VERTICES, DETAILED_VERTICES + 8] {
        let point = decode_point(&VERTICES[16..].as_chunks::<24>().0[index]);
        let triangle = [
            point,
            Vec3::new(point.x + 0.001, point.y, point.z),
            Vec3::new(point.x, point.y + 0.001, point.z),
        ];
        assert!(!hull.sweep_clears_triangle(start, start, 0.0, triangle)?);
    }
    Ok(())
}

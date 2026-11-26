use super::{
    DETAILED_VERTICES, HOG_PIECES, HogHullSource, MEMBERSHIP, NATIVE_VERTICES, VERTICES,
    decode_point,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh076/source-triangles.bin");

#[test]
fn hog_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = HogHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), HOG_PIECES);
    assert_eq!(points.len(), DETAILED_VERTICES + NATIVE_VERTICES);
    assert_eq!(TRIANGLES.len(), (868 + 448) * 12);
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
        HogHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        HogHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        HogHullSource::from_bytes(VERTICES, &shifted),
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
fn hog_interdeck_and_exterior_routes_clear_and_solid_sources_contact() -> Result<(), EnvelopeError>
{
    let hull = HogHullSource::bundled()?;
    let scale = 55_000.0 / 1_347.0;
    for (x, y, near, far) in [
        (0.0, -0.225, -1.235, 1.25),
        (0.0, 0.277, -1.235, 1.25),
        (0.0, 0.975, -1.575, 1.31),
        (0.0, -0.97, -1.575, 1.31),
        (-0.99, 0.0, -1.575, 1.31),
        (0.99, 0.0, -1.575, 1.31),
        (0.0, 0.785, 1.102, 1.35),
    ] {
        let (x, y, near, z) = (x * scale, y * scale, near * scale, far * scale);
        let travel = z - near;
        let triangle = [
            Vec3::new(x - 0.035, y, z),
            Vec3::new(x + 0.035, y, z),
            Vec3::new(x, y + 0.07, z),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(travel), 0.0, triangle)?);
    }
    for index in [0, 8, 16, 40, 168, DETAILED_VERTICES, DETAILED_VERTICES + 40] {
        let vertex = decode_point(&VERTICES[16..].as_chunks::<24>().0[index]);
        let triangle = [
            vertex,
            Vec3::new(vertex.x + 0.001, vertex.y, vertex.z),
            Vec3::new(vertex.x, vertex.y + 0.001, vertex.z),
        ];
        assert!(!hull.sweep_clears_triangle(translated(0.0), translated(0.0), 0.0, triangle)?);
    }
    Ok(())
}

use super::{
    DETAILED_VERTICES, MEMBERSHIP, NATIVE_VERTICES, VERTICES, VULTURE_RAIDER_PIECES,
    VultureRaiderHullSource, decode_point,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh065/source-triangles.bin");

#[test]
fn vulture_raider_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = VultureRaiderHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), VULTURE_RAIDER_PIECES);
    assert_eq!(points.len(), DETAILED_VERTICES + NATIVE_VERTICES);
    assert_eq!(TRIANGLES.len(), (4120 + 368) * 12);
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
        VultureRaiderHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        VultureRaiderHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        VultureRaiderHullSource::from_bytes(VERTICES, &shifted),
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
fn vulture_raider_dorsal_collar_and_rail_passages_clear_and_solid_sources_contact()
-> Result<(), EnvelopeError> {
    let hull = VultureRaiderHullSource::bundled()?;
    for (x, y, z, travel) in [
        (0.0, 8.8, 18.5, 38.5),
        (-9.75, 3.75, 18.6, 12.4),
        (-12.7, -4.6, 6.75, 26.75),
        (-8.56, -2.65, 3.8, 18.8),
        (9.75, 3.75, 18.6, 12.4),
        (12.7, -4.6, 6.75, 26.75),
        (8.56, -2.65, 3.8, 18.8),
    ] {
        let triangle = [
            Vec3::new(x - 0.035, y, z),
            Vec3::new(x + 0.035, y, z),
            Vec3::new(x, y + 0.07, z),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(travel), 0.0, triangle)?);
    }
    for index in [0, 226, 388] {
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

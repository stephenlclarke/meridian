use super::{
    DETAILED_VERTICES, HOBBY_FIGHTER_PIECES, HobbyFighterHullSource, MEMBERSHIP, NATIVE_VERTICES,
    VERTICES, decode_point,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh054/source-triangles.bin");

#[test]
fn hobby_fighter_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = HobbyFighterHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), HOBBY_FIGHTER_PIECES);
    assert_eq!(points.len(), DETAILED_VERTICES + NATIVE_VERTICES);
    assert_eq!(TRIANGLES.len(), (4620 + 192) * 12);
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
        HobbyFighterHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        HobbyFighterHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        HobbyFighterHullSource::from_bytes(VERTICES, &shifted),
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
fn hobby_fighter_wing_and_pod_passages_clear_and_solid_sources_contact() -> Result<(), EnvelopeError>
{
    let hull = HobbyFighterHullSource::bundled()?;
    for (x, y, z, travel) in [
        (0.0, -5.65, 13.0, 26.8),
        (-7.0, -1.05, 6.0, 12.0),
        (-7.0, 1.0, 6.0, 12.0),
        (-3.0, -0.55, -11.3, 2.7),
        (7.0, -1.05, 6.0, 12.0),
        (7.0, 1.0, 6.0, 12.0),
        (3.0, -0.55, -11.3, 2.7),
    ] {
        let triangle = [
            Vec3::new(x - 0.035, y, z),
            Vec3::new(x + 0.035, y, z),
            Vec3::new(x, y + 0.07, z),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(travel), 0.0, triangle)?);
    }
    for index in [0, 482, 964] {
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

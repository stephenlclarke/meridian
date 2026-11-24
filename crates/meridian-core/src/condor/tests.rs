use super::{
    CONDOR_PIECES, CondorHullSource, DETAILED_VERTICES, MEMBERSHIP, NATIVE_VERTICES,
    NO_SECONDARY_PIECE, VERTICES, decode_point,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh055/source-triangles.bin");

#[test]
fn condor_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = CondorHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), CONDOR_PIECES);
    assert_eq!(points.len(), DETAILED_VERTICES + NATIVE_VERTICES);
    assert_eq!(TRIANGLES.len(), (4504 + 244) * 12);
    let memberships = MEMBERSHIP.as_chunks::<2>().0;
    assert_eq!(memberships.len(), points.len());
    assert_eq!(
        memberships
            .iter()
            .filter(|pair| pair[1] != NO_SECONDARY_PIECE)
            .count(),
        56
    );
    for (point, pair) in points.iter().zip(memberships) {
        let point = decode_point(point);
        for slot in pair
            .iter()
            .copied()
            .filter(|&slot| slot != NO_SECONDARY_PIECE)
        {
            let piece = &hull.pieces()[usize::from(slot)];
            assert!(piece.source_points().contains(&point));
            assert_eq!(piece.rounding_metres(), 0.02);
        }
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
        assert!(memberships[indices[0]].iter().copied().any(|slot| {
            slot != NO_SECONDARY_PIECE
                && indices.iter().all(|&i| memberships[i].contains(&slot))
                && hull.pieces()[usize::from(slot)].contains(center)
        }));
    }
    let artwork_membership =
        include_bytes!("../../../../assets/fleet/sh055/source-piece-membership.bin");
    assert_eq!(
        CondorHullSource::from_bytes(VERTICES, artwork_membership),
        Err(EnvelopeError::ChangedSource)
    );
    let mut rewritten = VERTICES.to_vec();
    rewritten[16] ^= 1;
    assert_eq!(
        CondorHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        CondorHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        CondorHullSource::from_bytes(VERTICES, &shifted),
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
fn condor_dorsal_wing_and_pod_passages_clear_and_solid_sources_contact() -> Result<(), EnvelopeError>
{
    let hull = CondorHullSource::bundled()?;
    for (x, y, z, travel) in [
        (0.0, 4.95, 17.0, 33.5),
        (-9.0, -2.95, 17.0, 12.25),
        (-9.0, 1.0, 17.0, 12.25),
        (-13.35, 0.38, 2.0, 18.75),
        (9.0, -2.95, 17.0, 12.25),
        (9.0, 1.0, 17.0, 12.25),
        (13.35, 0.38, 2.0, 18.75),
    ] {
        let triangle = [
            Vec3::new(x - 0.035, y, z),
            Vec3::new(x + 0.035, y, z),
            Vec3::new(x, y + 0.07, z),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(travel), 0.0, triangle)?);
    }
    for index in [0, 8, 202] {
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

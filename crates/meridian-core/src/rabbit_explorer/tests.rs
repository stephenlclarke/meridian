use super::{
    DETAILED_VERTICES, MEMBERSHIP, NATIVE_VERTICES, NO_SECONDARY_PIECE, RABBIT_EXPLORER_PIECES,
    RabbitExplorerHullSource, VERTICES, decode_point,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh066/source-triangles.bin");

#[test]
fn rabbit_explorer_original_detailed_and_candidate_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = RabbitExplorerHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), RABBIT_EXPLORER_PIECES);
    assert_eq!(points.len(), DETAILED_VERTICES + NATIVE_VERTICES);
    assert_eq!(TRIANGLES.len(), (2648 + 480) * 12);
    let memberships = MEMBERSHIP.as_chunks::<2>().0;
    assert_eq!(memberships.len(), points.len());
    assert_eq!(
        memberships
            .iter()
            .filter(|pair| pair[1] != NO_SECONDARY_PIECE)
            .count(),
        272
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
        include_bytes!("../../../../assets/fleet/sh066/source-piece-membership.bin");
    assert_eq!(
        RabbitExplorerHullSource::from_bytes(VERTICES, artwork_membership),
        Err(EnvelopeError::ChangedSource)
    );
    let mut rewritten = VERTICES.to_vec();
    rewritten[16] ^= 1;
    assert_eq!(
        RabbitExplorerHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        RabbitExplorerHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        RabbitExplorerHullSource::from_bytes(VERTICES, &shifted),
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
fn rabbit_explorer_open_bow_and_inner_arcs_clear_and_endcaps_contact() -> Result<(), EnvelopeError>
{
    let hull = RabbitExplorerHullSource::bundled()?;
    for (x, y, near, far) in [
        (0.0, -0.7, 12.05, 17.9),
        (0.0, 8.1, -16.6, 16.6),
        (0.0, -3.25, -16.6, 16.6),
        (-5.5, -1.0, -3.0, 7.0),
        (5.5, -1.0, -3.0, 7.0),
        (-14.2, 0.0, -16.6, 16.6),
        (14.2, 0.0, -16.6, 16.6),
    ] {
        let triangle = [
            Vec3::new(x - 0.035, y, far),
            Vec3::new(x + 0.035, y, far),
            Vec3::new(x, y + 0.07, far),
        ];
        assert!(hull.sweep_clears_triangle(
            translated(0.0),
            translated(far - near),
            0.0,
            triangle
        )?);
    }
    let points = VERTICES[16..].as_chunks::<24>().0;
    for index in [0, 528, DETAILED_VERTICES, DETAILED_VERTICES + 48] {
        let point = decode_point(&points[index]);
        let triangle = [
            point,
            Vec3::new(point.x + 0.001, point.y, point.z),
            Vec3::new(point.x, point.y + 0.001, point.z),
        ];
        assert!(!hull.sweep_clears_triangle(translated(0.0), translated(0.0), 0.0, triangle)?);
    }
    Ok(())
}

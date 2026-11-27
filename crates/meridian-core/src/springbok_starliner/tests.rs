use super::{
    ARCHIVED_NATIVE_VERTICES, CANDIDATE_NATIVE_VERTICES, DETAILED_VERTICES, MEMBERSHIP,
    NO_SECONDARY_PIECE, SPRINGBOK_STARLINER_PIECES, SpringbokStarlinerHullSource, VERTICES,
    decode_point,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh088/source-triangles.bin");

#[test]
fn springbok_starliner_all_three_meshes_share_complete_local_supports() -> Result<(), EnvelopeError>
{
    let hull = SpringbokStarlinerHullSource::bundled()?;
    let points = VERTICES[20..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), SPRINGBOK_STARLINER_PIECES);
    assert_eq!(
        points.len(),
        DETAILED_VERTICES + ARCHIVED_NATIVE_VERTICES + CANDIDATE_NATIVE_VERTICES
    );
    assert_eq!(TRIANGLES.len(), (2672 + 912 + 504) * 12);
    let memberships = MEMBERSHIP.as_chunks::<2>().0;
    assert_eq!(memberships.len(), points.len());
    assert_eq!(
        memberships
            .iter()
            .filter(|pair| pair[1] != NO_SECONDARY_PIECE)
            .count(),
        264
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
    let mut rewritten = VERTICES.to_vec();
    rewritten[20] ^= 1;
    assert_eq!(
        SpringbokStarlinerHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        SpringbokStarlinerHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        SpringbokStarlinerHullSource::from_bytes(VERTICES, &shifted),
        Err(EnvelopeError::ChangedSource)
    );
    Ok(())
}

fn translated(y: f64, z: f64) -> RigidPose {
    RigidPose {
        position: Vec3::new(0.0, y, z),
        orientation: Basis3::IDENTITY,
    }
}

#[test]
fn springbok_starliner_apertures_and_bow_gap_clear() -> Result<(), EnvelopeError> {
    let hull = SpringbokStarlinerHullSource::bundled()?;
    let start = translated(0.0, 0.0);
    for (x, z) in [
        (0.0, 29.5),
        (-12.0, -4.0),
        (-12.0, 0.0),
        (-12.0, 4.0),
        (12.0, -4.0),
        (12.0, 0.0),
        (12.0, 4.0),
    ] {
        let triangle = [
            Vec3::new(x - 0.035, -5.0, z),
            Vec3::new(x + 0.035, -5.0, z),
            Vec3::new(x, -5.0, z + 0.07),
        ];
        assert!(hull.sweep_clears_triangle(start, translated(-14.0, 0.0), 0.0, triangle)?);
    }
    for index in [0, 2, 300, 576, 1460, 1948, 1952] {
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

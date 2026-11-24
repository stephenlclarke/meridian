use super::{
    DETAILED_VERTICES, DZO_PIECES, DzoHullSource, MEMBERSHIP, NATIVE_VERTICES, NO_SECONDARY_PIECE,
    VERTICES, decode_point,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh074/source-triangles.bin");

#[test]
fn dzo_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = DzoHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), DZO_PIECES);
    assert_eq!(points.len(), DETAILED_VERTICES + NATIVE_VERTICES);
    assert_eq!(TRIANGLES.len(), (9228 + 416) * 12);
    let memberships = MEMBERSHIP.as_chunks::<2>().0;
    assert_eq!(memberships.len(), points.len());
    assert_eq!(
        memberships
            .iter()
            .filter(|pair| pair[1] != NO_SECONDARY_PIECE)
            .count(),
        512
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
        include_bytes!("../../../../assets/fleet/sh074/source-piece-membership.bin");
    assert_eq!(
        DzoHullSource::from_bytes(VERTICES, artwork_membership),
        Err(EnvelopeError::ChangedSource)
    );
    let mut rewritten = VERTICES.to_vec();
    rewritten[16] ^= 1;
    assert_eq!(
        DzoHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        DzoHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        DzoHullSource::from_bytes(VERTICES, &shifted),
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
fn dzo_central_and_exterior_routes_clear_and_float_centres_contact() -> Result<(), EnvelopeError> {
    let hull = DzoHullSource::bundled()?;
    let near = -26.1;
    let far = 23.0;
    for (x, y) in [
        (0.0, -1.0),
        (0.0, 14.4),
        (0.0, -9.72),
        (-13.95, -1.0),
        (-6.0, 9.32),
        (13.95, -1.0),
        (6.0, 9.32),
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
    for x in [-7.025, 7.025] {
        for z in [-12.989, 12.011] {
            let y = -0.96;
            let triangle = [
                Vec3::new(x, y, z),
                Vec3::new(x + 0.001, y, z),
                Vec3::new(x, y + 0.001, z),
            ];
            assert!(!hull.sweep_clears_triangle(
                translated(0.0),
                translated(0.0),
                0.0,
                triangle
            )?);
        }
    }
    Ok(())
}

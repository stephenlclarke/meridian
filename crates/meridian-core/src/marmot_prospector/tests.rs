use super::{
    DETAILED_VERTICES, MARMOT_PROSPECTOR_PIECES, MEMBERSHIP, MarmotProspectorHullSource,
    NATIVE_VERTICES, NO_SECONDARY_PIECE, VERTICES, decode_point,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh130/source-triangles.bin");

#[test]
fn marmot_prospector_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = MarmotProspectorHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), MARMOT_PROSPECTOR_PIECES);
    assert_eq!(points.len(), DETAILED_VERTICES + NATIVE_VERTICES);
    assert_eq!(TRIANGLES.len(), (8484 + 448) * 12);
    let memberships = MEMBERSHIP.as_chunks::<2>().0;
    assert_eq!(memberships.len(), points.len());
    assert_eq!(
        memberships
            .iter()
            .filter(|pair| pair[1] != NO_SECONDARY_PIECE)
            .count(),
        384
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
    rewritten[16] ^= 1;
    assert_eq!(
        MarmotProspectorHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh129/source.vertices");
    assert_eq!(
        MarmotProspectorHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        MarmotProspectorHullSource::from_bytes(VERTICES, &shifted),
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
fn marmot_prospector_routes_clear() -> Result<(), EnvelopeError> {
    let hull = MarmotProspectorHullSource::bundled()?;
    let start = translated(0.0, 0.0);
    for (x, y, z, travel) in [
        (0.0, 11.0, -32.0, -62.0),
        (0.0, -10.0, -32.0, -62.0),
        (-15.0, 0.0, -32.0, -62.0),
        (15.0, 0.0, -32.0, -62.0),
        (0.0, -4.8, -32.0, -8.0),
        (-4.0, -2.0, 30.0, 15.1),
        (4.0, -2.0, 30.0, 15.1),
    ] {
        let triangle = [
            Vec3::new(x - 0.03, y, z),
            Vec3::new(x + 0.03, y, z),
            Vec3::new(x, y + 0.06, z),
        ];
        assert!(hull.sweep_clears_triangle(start, translated(0.0, travel), 0.0, triangle)?);
    }
    for (collar, axis) in [-8.0, 0.0, 8.0].into_iter().enumerate() {
        let triangle = [
            Vec3::new(axis - 0.01, -2.0, 17.0),
            Vec3::new(axis + 0.01, -2.0, 17.0),
            Vec3::new(axis, -1.98, 17.0),
        ];
        for piece in &hull.pieces()[collar * 8..(collar + 1) * 8] {
            assert!(piece.sweep_clears_triangle(start, start, 0.0, triangle)?);
        }
    }
    for index in [0, 20, 100, DETAILED_VERTICES, DETAILED_VERTICES + 8] {
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

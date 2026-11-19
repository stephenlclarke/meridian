use super::{INDICES, SableLinerHullSource, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const FACES: &[u8] = include_bytes!("../../../../assets/fleet/sh012/source-triangles.bin");

#[test]
fn sable_liner_all_original_faces_keep_a_complete_piece_at_ring_seams() -> Result<(), EnvelopeError>
{
    let hull = SableLinerHullSource::bundled()?;
    assert_eq!(hull.pieces().len(), 47);
    let positions: Vec<_> = VERTICES[16..]
        .as_chunks::<24>()
        .0
        .iter()
        .map(decode_point)
        .collect();
    assert_eq!((positions.len(), FACES.len() / 12), (2440, 4724));
    for face in FACES.as_chunks::<12>().0 {
        let [a, b, c] = face.as_chunks::<4>().0 else {
            return Err(EnvelopeError::InvalidContent);
        };
        let triangle = [*a, *b, *c].map(|i| positions[u32::from_le_bytes(i) as usize]);
        let enclosing = hull
            .pieces()
            .iter()
            .find(|piece| triangle.iter().all(|p| piece.source_points().contains(p)))
            .ok_or(EnvelopeError::InvalidContent)?;
        assert_eq!(enclosing.rounding_metres(), 0.02);
        let [a, b, c] = triangle;
        assert!(enclosing.contains(Vec3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        )));
    }
    let mut changed_vertices = VERTICES.to_vec();
    changed_vertices[16] ^= 1;
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    for invalid in [changed_vertices.as_slice(), explorer.as_slice()] {
        assert_eq!(
            SableLinerHullSource::from_bytes(invalid, INDICES),
            Err(EnvelopeError::ChangedSource)
        );
    }
    let mut changed = INDICES.to_vec();
    changed[20] ^= 1;
    assert_eq!(
        SableLinerHullSource::from_bytes(VERTICES, &changed),
        Err(EnvelopeError::ChangedSource),
    );
    Ok(())
}

#[test]
fn sable_liner_crescent_opening_clears_but_original_rim_blocks() -> Result<(), EnvelopeError> {
    let hull = SableLinerHullSource::bundled()?;
    let pose = RigidPose {
        position: Vec3::new(0.0, 0.0, 0.0),
        orientation: Basis3::IDENTITY,
    };
    let opening = [
        Vec3::new(20.0, 0.85, -0.15),
        Vec3::new(20.0, 1.15, -0.15),
        Vec3::new(20.0, 1.0, 0.15),
    ];
    let end = RigidPose {
        position: Vec3::new(12.0, 0.0, 0.0),
        ..pose
    };
    assert!(hull.sweep_clears_triangle(pose, end, 0.0, opening)?);

    // Independent calibrated detailed rim vertex 194 and a 1 mm contact probe.
    let rim = [
        Vec3::new(16.490_633_1, 1.0, 6.913_684_3),
        Vec3::new(16.490_633_1, 1.001, 6.913_684_3),
        Vec3::new(16.490_633_1, 1.0, 6.914_684_3),
    ];
    assert!(!hull.sweep_clears_triangle(pose, pose, 0.0, rim)?);
    Ok(())
}

fn inward(x: f64, z: f64) -> RigidPose {
    RigidPose {
        position: Vec3::new(x, 0.0, z),
        orientation: Basis3 {
            x: Vec3::new(-1.0, 0.0, 0.0),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(0.0, 0.0, -1.0),
        },
    }
}

#[test]
fn sable_liner_original_station_aligned_clearance_and_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = SableLinerHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.sable_liner_motion_clear(&hull, inward(0.0, 137.32), inward(0.0, 135.32), 0.0));
    let before = inward(69.4, 161.82);
    let after = inward(69.4, 159.82);
    assert!(station.sable_liner_motion_clear(&hull, before, before, 0.0));
    // The original native crescent vertex meets the strict interior of source face 482.
    assert!(!station.sable_liner_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

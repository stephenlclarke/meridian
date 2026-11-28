use super::{MEMBERSHIP, StagLinerHullSource, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh035/source-triangles.bin");

#[test]
fn stag_liner_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = StagLinerHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 26);
    assert_eq!(points.len(), 1936);
    assert_eq!(TRIANGLES.len(), (3492 + 216) * 12);
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
        StagLinerHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        StagLinerHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        StagLinerHullSource::from_bytes(VERTICES, &shifted),
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
fn stag_liner_tower_and_gallery_passages_clear_and_solid_sources_contact()
-> Result<(), EnvelopeError> {
    let hull = StagLinerHullSource::bundled()?;
    for sign in [-1.0, 1.0] {
        for (x, bottom, top, z, travel) in [
            (sign * 3.8, 5.8, 5.9, 16.0, 11.0),
            (sign * 4.3, 13.8, 13.9, 26.0, 21.2),
            (sign * 11.65, 6.4, 6.5, 27.0, 22.0),
        ] {
            let triangle = [
                Vec3::new(x - 0.08, bottom, z),
                Vec3::new(x + 0.08, bottom, z),
                Vec3::new(x, top, z),
            ];
            assert!(hull.sweep_clears_triangle(
                translated(0.0),
                translated(travel),
                0.0,
                triangle
            )?);
        }
    }
    let aisle = [
        Vec3::new(-0.08, 14.1, 26.0),
        Vec3::new(0.08, 14.1, 26.0),
        Vec3::new(0.0, 14.2, 26.0),
    ];
    assert!(hull.sweep_clears_triangle(translated(0.0), translated(32.5), 0.0, aisle)?);
    for index in [0, 590, 998] {
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
fn stag_liner_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = StagLinerHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.stag_liner_motion_clear(&hull, inward(0.0, 137.32), inward(0.0, 135.32), 0.0));
    let before = inward(71.82000000000001, 170.32);
    let after = inward(71.82000000000001, 168.32);
    assert!(station.stag_liner_motion_clear(&hull, before, before, 0.0));
    assert!(!station.stag_liner_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

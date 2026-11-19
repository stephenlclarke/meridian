use super::{BuffaloMk2HullSource, MEMBERSHIP, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh014/source-triangles.bin");

#[test]
fn buffalo_mk2_corrected_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = BuffaloMk2HullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 57);
    assert_eq!(points.len(), 2336);
    assert_eq!(TRIANGLES.len(), (3460 + 500) * 12);
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
        BuffaloMk2HullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        BuffaloMk2HullSource::from_bytes(VERTICES, &shifted),
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
fn buffalo_mk2_preserves_both_interdrum_sweeps_but_blocks_an_original_drum_contact()
-> Result<(), EnvelopeError> {
    let hull = BuffaloMk2HullSource::bundled()?;
    for [x0, x1, x2] in [[-15.72, -15.52, -15.62], [15.52, 15.72, 15.62]] {
        let passage = [
            Vec3::new(x0, -19.35, 52.5),
            Vec3::new(x1, -19.35, 52.5),
            Vec3::new(x2, -19.15, 52.5),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(26.0), 0.0, passage)?);
    }
    let vertex = decode_point(&VERTICES[16..].as_chunks::<24>().0[0]);
    let rim = [
        vertex,
        Vec3::new(vertex.x + 0.001, vertex.y, vertex.z),
        Vec3::new(vertex.x, vertex.y + 0.001, vertex.z),
    ];
    assert!(!hull.sweep_clears_triangle(translated(0.0), translated(0.0), 0.0, rim)?);
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
fn buffalo_mk2_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = BuffaloMk2HullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.buffalo_mk2_motion_clear(&hull, inward(0.0, 137.32), inward(0.0, 135.32), 0.0));
    let before = inward(42.31548913043478, 192.32);
    let after = inward(42.31548913043478, 190.32);
    assert!(station.buffalo_mk2_motion_clear(&hull, before, before, 0.0));
    assert!(!station.buffalo_mk2_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

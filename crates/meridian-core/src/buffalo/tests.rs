use super::{BuffaloHullSource, MEMBERSHIP, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh013/source-triangles.bin");

#[test]
fn buffalo_corrected_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = BuffaloHullSource::bundled()?;
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
        BuffaloHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        BuffaloHullSource::from_bytes(VERTICES, &shifted),
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
fn buffalo_preserves_both_interdrum_sweeps_but_blocks_an_original_drum_contact()
-> Result<(), EnvelopeError> {
    let hull = BuffaloHullSource::bundled()?;
    for [x0, x1, x2] in [[-10.9, -10.7, -10.8], [10.7, 10.9, 10.8]] {
        let passage = [
            Vec3::new(x0, -13.8, 36.2),
            Vec3::new(x1, -13.8, 36.2),
            Vec3::new(x2, -13.6, 36.2),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(18.0), 0.0, passage)?);
    }
    let rim = [
        Vec3::new(-14.5483, -4.75, -38.0),
        Vec3::new(-14.5473, -4.75, -38.0),
        Vec3::new(-14.5483, -4.749, -38.0),
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
fn buffalo_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = BuffaloHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.buffalo_motion_clear(&hull, inward(0.0, 137.32), inward(0.0, 135.32), 0.0));
    let before = inward(50.5045, 175.32);
    let after = inward(50.5045, 173.32);
    assert!(station.buffalo_motion_clear(&hull, before, before, 0.0));
    assert!(!station.buffalo_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

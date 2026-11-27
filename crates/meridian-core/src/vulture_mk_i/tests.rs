use super::{MEMBERSHIP, VERTICES, VultureMkIHullSource, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh037/source-triangles.bin");

#[test]
fn vulture_mk_i_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = VultureMkIHullSource::bundled()?;
    let points = VERTICES[20..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 8);
    assert_eq!(points.len(), 7216);
    assert_eq!(TRIANGLES.len(), (8272 + 3016 + 512) * 12);
    for (index, slot) in MEMBERSHIP.iter().copied().enumerate() {
        let point = decode_point(&points[index]);
        let piece = &hull.pieces()[usize::from(slot)];
        assert!(piece.contains(point));
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
    rewritten[20] ^= 1;
    assert_eq!(
        VultureMkIHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        VultureMkIHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        VultureMkIHullSource::from_bytes(VERTICES, &shifted),
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
fn vulture_mk_i_shoulder_drive_and_fore_passages_clear_and_solid_sources_contact()
-> Result<(), EnvelopeError> {
    let hull = VultureMkIHullSource::bundled()?;
    for sign in [-1.0, 1.0] {
        for (x, y, z, travel) in [
            (sign * 7.0, -1.75, 5.9, 15.5),
            (sign * 7.0, 0.4, -10.8, 6.2),
            (sign * 8.1, -0.6, 14.8, 8.5),
        ] {
            let triangle = [
                Vec3::new(x - 0.035, y, z),
                Vec3::new(x + 0.035, y, z),
                Vec3::new(x, y + 0.07, z),
            ];
            assert!(hull.sweep_clears_triangle(
                translated(0.0),
                translated(travel),
                0.0,
                triangle
            )?);
        }
    }
    for index in [0, 220, 1476] {
        let vertex = decode_point(&VERTICES[20..].as_chunks::<24>().0[index]);
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
fn vulture_mk_i_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = VultureMkIHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.vulture_mk_i_motion_clear(
        &hull,
        inward(0.0, 137.32),
        inward(0.0, 135.32),
        0.0
    ));
    let before = inward(71.56039279869067, 151.55895253682488);
    let after = inward(71.56039279869067, 149.55895253682488);
    assert!(station.vulture_mk_i_motion_clear(&hull, before, before, 0.0));
    assert!(!station.vulture_mk_i_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

use super::{MEMBERSHIP, PantherExplorerHullSource, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::{EnvelopeError, MULE_CLEARANCE_METRES, SmoothPiece};

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh032/source-triangles.bin");

#[test]
fn panther_explorer_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = PantherExplorerHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 36);
    assert_eq!(points.len(), 3168);
    assert_eq!(TRIANGLES.len(), (5596 + 352) * 12);
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
        PantherExplorerHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        PantherExplorerHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        PantherExplorerHullSource::from_bytes(VERTICES, &shifted),
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
fn panther_explorer_sled_and_arch_passages_clear_and_solid_sources_contact()
-> Result<(), EnvelopeError> {
    let hull = PantherExplorerHullSource::bundled()?;
    for sign in [-1.0, 1.0] {
        for (x, bottom, top, z, travel) in [
            (sign * 5.25, -2.9, -2.8, 16.0, 26.0),
            (sign * 7.25, 0.35, 0.45, 16.0, 37.0),
            (sign * 5.25, -4.28, -4.18, -8.5, 12.5),
            (sign * 5.7, 0.44, 0.54, 7.0, 16.0),
            (sign * 5.7, 1.26, 1.38, 7.0, 2.0),
            (sign * 6.2, 1.99, 2.16, -0.9, 1.2),
            (sign * 5.7, 1.26, 1.38, -8.0, 2.0),
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
    for index in [0, 918, 1654, 1860] {
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

#[test]
fn panther_explorer_merging_adjacent_arch_supports_blocks_clearance() -> Result<(), EnvelopeError> {
    let hull = PantherExplorerHullSource::bundled()?;
    for (first_slot, sign) in [(2, -1.0), (9, 1.0)] {
        for (span, x, bottom, top, z, travel) in [
            (0..4, sign * 5.7, 0.44, 0.54, 7.0, 16.0),
            (0..2, sign * 5.7, 1.26, 1.38, 7.0, 2.0),
            (1..3, sign * 6.2, 1.99, 2.16, -0.9, 1.2),
            (2..4, sign * 5.7, 1.26, 1.38, -8.0, 2.0),
        ] {
            let merged_points = hull.pieces()[first_slot + span.start..first_slot + span.end]
                .iter()
                .flat_map(|piece| piece.source_points().iter().copied())
                .collect();
            let merged = SmoothPiece::new(merged_points, MULE_CLEARANCE_METRES)?;
            let triangle = [
                Vec3::new(x - 0.08, bottom, z),
                Vec3::new(x + 0.08, bottom, z),
                Vec3::new(x, top, z),
            ];
            assert!(!merged.sweep_clears_triangle(
                translated(0.0),
                translated(travel),
                0.0,
                triangle
            )?);
            assert!(hull.sweep_clears_triangle(
                translated(0.0),
                translated(travel),
                0.0,
                triangle
            )?);
        }
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
fn panther_explorer_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = PantherExplorerHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.panther_explorer_motion_clear(
        &hull,
        inward(0.0, 137.32),
        inward(0.0, 135.32),
        0.0
    ));
    let before = inward(70.28, 154.32);
    let after = inward(70.28, 152.32);
    assert!(station.panther_explorer_motion_clear(&hull, before, before, 0.0));
    assert!(!station.panther_explorer_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

use super::{IbexHullSource, MEMBERSHIP, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh027/source-triangles.bin");

#[test]
fn ibex_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = IbexHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 25);
    assert_eq!(points.len(), 1724);
    assert_eq!(TRIANGLES.len(), (3108 + 184) * 12);
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
        IbexHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        IbexHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        IbexHullSource::from_bytes(VERTICES, &shifted),
        Err(EnvelopeError::ChangedSource)
    );
    Ok(())
}

fn translated(x: f64, z: f64) -> RigidPose {
    RigidPose {
        position: Vec3::new(x, 0.0, z),
        orientation: Basis3::IDENTITY,
    }
}

#[test]
fn ibex_trench_and_both_pipe_channels_clear_but_original_keels_contact() -> Result<(), EnvelopeError>
{
    let hull = IbexHullSource::bundled()?;
    let longitudinal = [
        Vec3::new(-0.15, -3.2, 16.4),
        Vec3::new(0.15, -3.2, 16.4),
        Vec3::new(0.0, -2.98, 16.4),
    ];
    assert!(hull.sweep_clears_triangle(
        translated(0.0, 0.0),
        translated(0.0, 33.0),
        0.0,
        longitudinal
    )?);
    let lateral = [
        Vec3::new(1.35, -3.5, 1.2),
        Vec3::new(1.35, -3.25, 1.2),
        Vec3::new(1.35, -3.38, 1.48),
    ];
    assert!(hull.sweep_clears_triangle(
        translated(0.0, 0.0),
        translated(2.7, 0.0),
        0.0,
        lateral
    )?);
    for x in [-2.17, 2.17] {
        let y = 6.42;
        let pipe = [
            Vec3::new(x - 0.03, y - 0.02, -1.25),
            Vec3::new(x + 0.03, y - 0.02, -1.25),
            Vec3::new(x, y + 0.02, -1.25),
        ];
        assert!(hull.sweep_clears_triangle(
            translated(0.0, 0.0),
            translated(0.0, 4.0),
            0.0,
            pipe
        )?);
    }
    for index in [32, 440] {
        let vertex = decode_point(&VERTICES[16..].as_chunks::<24>().0[index]);
        let triangle = [
            vertex,
            Vec3::new(vertex.x + 0.001, vertex.y, vertex.z),
            Vec3::new(vertex.x, vertex.y + 0.001, vertex.z),
        ];
        assert!(!hull.sweep_clears_triangle(
            translated(0.0, 0.0),
            translated(0.0, 0.0),
            0.0,
            triangle
        )?);
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
fn ibex_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = IbexHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.ibex_motion_clear(&hull, inward(0.0, 137.32), inward(0.0, 135.32), 0.0));
    let before = inward(63.784000000000006, 153.32);
    let after = inward(63.784000000000006, 151.32);
    assert!(station.ibex_motion_clear(&hull, before, before, 0.0));
    assert!(!station.ibex_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

use super::{MEMBERSHIP, OnagerHullSource, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh018/source-triangles.bin");

#[test]
fn onager_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = OnagerHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 26);
    assert_eq!(points.len(), 2132);
    assert_eq!(TRIANGLES.len(), (3864 + 236) * 12);
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
        OnagerHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        OnagerHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        OnagerHullSource::from_bytes(VERTICES, &shifted),
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
fn onager_keeps_pod_gaps_underframe_and_both_recessed_pipe_runs_clear() -> Result<(), EnvelopeError>
{
    let hull = OnagerHullSource::bundled()?;
    for x in [-8.0, 8.0] {
        let triangle = [
            Vec3::new(x - 0.1, -2.1, 2.35),
            Vec3::new(x + 0.1, -2.1, 2.35),
            Vec3::new(x, -1.9, 2.35),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(4.7), 0.0, triangle)?);
    }
    let underframe = [
        Vec3::new(-0.1, -4.83, 10.0),
        Vec3::new(0.1, -4.83, 10.0),
        Vec3::new(0.0, -4.73, 10.0),
    ];
    assert!(hull.sweep_clears_triangle(translated(0.0), translated(20.0), 0.0, underframe)?);
    for x in [-3.51, 3.51] {
        let triangle = [
            Vec3::new(x - 0.04, 2.67, -0.9),
            Vec3::new(x + 0.04, 2.67, -0.9),
            Vec3::new(x, 2.72, -0.9),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(2.3), 0.0, triangle)?);
    }
    for index in [234, 828] {
        let vertex = decode_point(&VERTICES[16..].as_chunks::<24>().0[index]);
        let triangle = [
            vertex,
            Vec3::new(vertex.x, vertex.y + 0.001, vertex.z),
            Vec3::new(vertex.x, vertex.y, vertex.z + 0.001),
        ];
        assert!(!hull.sweep_clears_triangle(translated(0.0), translated(0.0), 0.0, triangle)?);
    }
    Ok(())
}

#[test]
fn onager_original_hatch_retains_local_rounded_stand_off() -> Result<(), EnvelopeError> {
    let hull = OnagerHullSource::bundled()?;
    let strip = |x| {
        [
            Vec3::new(x, -0.9, 1.0),
            Vec3::new(x, -0.7, 1.0),
            Vec3::new(x, -0.8, 0.95),
        ]
    };
    assert!(hull.sweep_clears_triangle(translated(0.0), translated(2.0), 0.0, strip(6.526))?);
    assert!(!hull.sweep_clears_triangle(translated(0.0), translated(2.0), 0.0, strip(6.519))?);
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
fn onager_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = OnagerHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.onager_motion_clear(&hull, inward(0.0, 137.32), inward(0.0, 135.32), 0.0));
    let before = inward(68.3, 151.82);
    let after = inward(68.3, 149.82);
    assert!(station.onager_motion_clear(&hull, before, before, 0.0));
    assert!(!station.onager_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

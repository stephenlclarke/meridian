use super::{KiteInterceptorHullSource, MEMBERSHIP, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh031/source-triangles.bin");

#[test]
fn kite_interceptor_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = KiteInterceptorHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 25);
    assert_eq!(points.len(), 1708);
    assert_eq!(TRIANGLES.len(), (3108 + 160) * 12);
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
        KiteInterceptorHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        KiteInterceptorHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        KiteInterceptorHullSource::from_bytes(VERTICES, &shifted),
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
fn kite_interceptor_sweeps_clear_the_bow_and_blade_channels_and_contact_solid_sources()
-> Result<(), EnvelopeError> {
    let hull = KiteInterceptorHullSource::bundled()?;
    for sign in [-1.0, 1.0] {
        for (x, bottom, top, z, travel) in [
            (sign * 3.65, 0.4, 0.5, 20.0, 18.5),
            (sign * 1.8, 4.55, 4.65, 1.0, 16.0),
            (sign * 4.3, 4.25, 4.35, 1.0, 16.0),
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
    for index in [0, 194, 602] {
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
fn kite_interceptor_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = KiteInterceptorHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.kite_interceptor_motion_clear(
        &hull,
        inward(0.0, 137.32),
        inward(0.0, 135.32),
        0.0
    ));
    let before = inward(69.994, 156.82);
    let after = inward(69.994, 154.82);
    assert!(station.kite_interceptor_motion_clear(&hull, before, before, 0.0));
    assert!(!station.kite_interceptor_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

use super::{CrowPrototypeHullSource, MEMBERSHIP, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh020/source-triangles.bin");

#[test]
fn crow_prototype_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = CrowPrototypeHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 68);
    assert_eq!(points.len(), 2592);
    assert_eq!(TRIANGLES.len(), (3472 + 952) * 12);
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
        CrowPrototypeHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        CrowPrototypeHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        CrowPrototypeHullSource::from_bytes(VERTICES, &shifted),
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
fn crow_prototype_separate_arms_keep_axial_and_lateral_fork_routes_clear()
-> Result<(), EnvelopeError> {
    let hull = CrowPrototypeHullSource::bundled()?;
    let axial = [
        Vec3::new(-0.15, -0.89, 15.1),
        Vec3::new(0.15, -0.89, 15.1),
        Vec3::new(0.0, -0.69, 15.1),
    ];
    assert!(hull.sweep_clears_triangle(translated(0.0, 0.0), translated(0.0, 9.6), 0.0, axial)?);
    let lateral = [
        Vec3::new(1.17, -0.94, 11.0),
        Vec3::new(1.17, -0.64, 11.0),
        Vec3::new(1.17, -0.79, 11.25),
    ];
    assert!(hull.sweep_clears_triangle(
        translated(0.0, 0.0),
        translated(2.34, 0.0),
        0.0,
        lateral
    )?);
    for [a, b, center] in [[-1.05, -0.85, -0.95], [0.85, 1.05, 0.95]] {
        let wall = [
            Vec3::new(a, -0.91, 14.85),
            Vec3::new(b, -0.91, 14.85),
            Vec3::new(center, -0.71, 14.85),
        ];
        assert!(hull.sweep_clears_triangle(
            translated(0.0, 0.0),
            translated(0.0, 9.1),
            0.0,
            wall
        )?);
    }
    for index in [112, 680] {
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
fn crow_prototype_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = CrowPrototypeHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.crow_prototype_motion_clear(
        &hull,
        inward(0.0, 137.32),
        inward(0.0, 135.32),
        0.0
    ));
    let before = inward(64.14930911962101, 153.82611922621396);
    let after = inward(64.14930911962101, 151.82611922621396);
    assert!(station.crow_prototype_motion_clear(&hull, before, before, 0.0));
    assert!(!station.crow_prototype_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

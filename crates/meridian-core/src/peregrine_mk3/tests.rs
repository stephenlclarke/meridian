use super::{MEMBERSHIP, PeregrineMk3HullSource, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh023/source-triangles.bin");

#[test]
fn peregrine_mk3_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = PeregrineMk3HullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 69);
    assert_eq!(points.len(), 2382);
    assert_eq!(TRIANGLES.len(), (3484 + 512) * 12);
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
        PeregrineMk3HullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        PeregrineMk3HullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        PeregrineMk3HullSource::from_bytes(VERTICES, &shifted),
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
fn peregrine_mk3_bow_gap_and_shoulder_sweeps_clear_but_original_solids_contact()
-> Result<(), EnvelopeError> {
    let hull = PeregrineMk3HullSource::bundled()?;
    let axial = [
        Vec3::new(-0.15, -1.5, 18.5),
        Vec3::new(0.15, -1.5, 18.5),
        Vec3::new(0.0, -1.3, 18.5),
    ];
    assert!(hull.sweep_clears_triangle(translated(0.0, 0.0), translated(0.0, 10.0), 0.0, axial)?);
    let lateral = [
        Vec3::new(2.35, -1.55, 12.0),
        Vec3::new(2.35, -1.25, 12.0),
        Vec3::new(2.35, -1.4, 12.25),
    ];
    assert!(hull.sweep_clears_triangle(
        translated(0.0, 0.0),
        translated(4.7, 0.0),
        0.0,
        lateral
    )?);
    for [a, b, center] in [[-2.35, -2.15, -2.25], [2.15, 2.35, 2.25]] {
        let shoulder = [
            Vec3::new(a, -1.55, 17.5),
            Vec3::new(b, -1.55, 17.5),
            Vec3::new(center, -1.35, 17.5),
        ];
        assert!(hull.sweep_clears_triangle(
            translated(0.0, 0.0),
            translated(0.0, 8.75),
            0.0,
            shoulder
        )?);
    }
    for index in [112, 680, 1728] {
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
fn peregrine_mk3_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = PeregrineMk3HullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.peregrine_mk3_motion_clear(
        &hull,
        inward(0.0, 137.32),
        inward(0.0, 135.32),
        0.0
    ));
    let before = inward(58.414207377866404, 157.08911371149708);
    let after = inward(58.414207377866404, 155.08911371149708);
    assert!(station.peregrine_mk3_motion_clear(&hull, before, before, 0.0));
    assert!(!station.peregrine_mk3_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

use super::{
    EnvelopeError, GOSHAWK_PIECES, GoshawkHullSource, MEMBERSHIP, VERTICES, source_vertex,
};
use crate::docking::{Basis3, RigidPose, Vec3};

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh008/source-triangles.bin");

#[test]
fn goshawk_encloses_both_source_triangle_sets_without_redistributing_vertices()
-> Result<(), EnvelopeError> {
    let source = GoshawkHullSource::bundled()?;
    assert_eq!(source.pieces().len(), GOSHAWK_PIECES);
    assert_eq!(
        source
            .pieces()
            .iter()
            .map(|p| p.source_points().len())
            .sum::<usize>(),
        2608
    );
    assert_eq!(TRIANGLES.len(), (3484 + 964) * 12);
    for (index, group) in MEMBERSHIP.iter().copied().enumerate() {
        let point = source_vertex(VERTICES, index)?;
        let piece = &source.pieces()[usize::from(group)];
        assert!(piece.source_points().contains(&point));
        assert!(piece.contains(point));
        assert_eq!(piece.rounding_metres(), 0.02);
    }
    for face in TRIANGLES.as_chunks::<12>().0 {
        let [a, b, c] = face.as_chunks::<4>().0 else {
            return Err(EnvelopeError::InvalidContent);
        };
        let ids = [*a, *b, *c].map(|part| u32::from_le_bytes(part) as usize);
        let group = MEMBERSHIP[ids[0]];
        assert!(ids.iter().all(|i| MEMBERSHIP[*i] == group));
        let [a, b, c] = [
            source_vertex(VERTICES, ids[0])?,
            source_vertex(VERTICES, ids[1])?,
            source_vertex(VERTICES, ids[2])?,
        ];
        let center = Vec3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        assert!(source.pieces()[usize::from(group)].contains(center));
    }
    let mut edited = MEMBERSHIP.to_vec();
    edited[0] ^= 1;
    assert_eq!(
        GoshawkHullSource::from_bytes(VERTICES, &edited),
        Err(EnvelopeError::ChangedSource)
    );
    let mut coordinates = VERTICES.to_vec();
    coordinates[24] ^= 1;
    assert_eq!(
        GoshawkHullSource::from_bytes(&coordinates, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    Ok(())
}

#[test]
fn goshawk_forebody_channel_stays_clear_while_a_source_drive_arm_blocks_contact()
-> Result<(), EnvelopeError> {
    let source = GoshawkHullSource::bundled()?;
    let start = RigidPose {
        position: Vec3::new(0.0, 0.0, 0.0),
        orientation: Basis3::IDENTITY,
    };
    let end = RigidPose {
        position: Vec3::new(0.0, 0.0, 10.0),
        ..start
    };
    let channel = [
        Vec3::new(-0.15, -1.65, 14.0),
        Vec3::new(0.15, -1.65, 14.0),
        Vec3::new(0.0, -1.35, 14.0),
    ];
    assert!(source.sweep_clears_triangles(start, end, 0.0, &[channel])?);
    let arm = source_vertex(VERTICES, 8)?;
    let contact = [
        arm,
        Vec3::new(arm.x + 0.003, arm.y, arm.z),
        Vec3::new(arm.x, arm.y + 0.003, arm.z),
    ];
    assert!(!source.sweep_clears_triangles(start, start, 0.0, &[contact])?);
    Ok(())
}

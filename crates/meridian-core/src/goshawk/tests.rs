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

#[test]
fn goshawk_smoothed_service_wells_block_the_four_original_approaches() -> Result<(), EnvelopeError>
{
    let source = GoshawkHullSource::bundled()?;
    // Calibrated mouth centers and exterior stations from the independent swept-clearance
    // certificate. These fixed coordinates do not read the runtime vertices or partition.
    let cases = [
        (
            "detailed body well 0",
            Vec3::new(2.321_523_564, 3.487_682_746, -9.963_112_909),
            Vec3::new(30.195_440_190, 32.176_418_708, -9.963_112_909),
        ),
        (
            "native body well 0",
            Vec3::new(2.302_049_164, 3.474_782_475, -8.396_213_846),
            Vec3::new(30.196_679_596, 32.143_376_835, -8.396_213_846),
        ),
        (
            "detailed left arm well 3",
            Vec3::new(-10.086_669_591, 0.703_563_159, 4.316_363_456),
            Vec3::new(-38.111_228_612, 29.245_204_394, 4.316_363_456),
        ),
        (
            "native left arm well 1",
            Vec3::new(-10.070_866_394, 0.692_846_161, 5.085_030_921),
            Vec3::new(-38.115_986_634, 29.214_269_500, 5.085_030_921),
        ),
    ];
    let at_mouth = RigidPose {
        position: Vec3::new(0.0, 0.0, 0.0),
        orientation: Basis3::IDENTITY,
    };
    for (well, mouth, outside) in cases {
        // The triangle fits within a 1 mm radius, inside even the narrowest open trial route.
        let probe = [[
            Vec3::new(mouth.x - 0.001, mouth.y, mouth.z),
            Vec3::new(mouth.x + 0.001, mouth.y, mouth.z),
            Vec3::new(mouth.x, mouth.y + 0.001, mouth.z),
        ]];
        // Move the hull past a fixed probe so the relative sweep follows the original approach.
        let at_exterior = RigidPose {
            position: Vec3::new(
                mouth.x - outside.x,
                mouth.y - outside.y,
                mouth.z - outside.z,
            ),
            ..at_mouth
        };
        assert!(
            source.sweep_clears_triangles(at_exterior, at_exterior, 0.0, &probe)?,
            "{well} must start clear outside the hull"
        );
        assert!(
            !source.sweep_clears_triangles(at_mouth, at_mouth, 0.0, &probe)?,
            "{well} must be covered by the rounded envelope"
        );
        assert!(
            !source.sweep_clears_triangles(at_exterior, at_mouth, 0.0, &probe)?,
            "{well} must block the continuous approach"
        );
    }
    Ok(())
}

fn inward_at(x: f64, z: f64) -> RigidPose {
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
fn goshawk_original_sh028_accepts_aligned_entry_but_rejects_bad_speed_roll_and_aperture()
-> Result<(), Box<dyn std::error::Error>> {
    let source = GoshawkHullSource::bundled()?;
    let station = crate::OriginalStationData::bundled()?;
    let z = station.port().center().z;
    let before = inward_at(0.0, z + 0.5);
    let after = inward_at(0.0, z - 0.5);
    assert!(station.goshawk_motion_clear(&source, before, after, 0.0));
    assert!(station.goshawk_entry_clear(&source, before, after, 72.0, 0.0, 0.0));
    assert!(!station.goshawk_entry_clear(&source, before, after, 144.0, 0.0, 0.0));
    assert!(!station.goshawk_entry_clear(&source, before, after, 72.0, 0.1, 0.0));
    let off_aperture_before = inward_at(100.0, z + 0.5);
    let off_aperture_after = inward_at(100.0, z - 0.5);
    assert!(station.goshawk_motion_clear(&source, off_aperture_before, off_aperture_after, 0.0));
    assert!(!station.goshawk_entry_clear(
        &source,
        off_aperture_before,
        off_aperture_after,
        72.0,
        0.0,
        0.0,
    ));
    Ok(())
}

#[test]
fn goshawk_original_sh028_blocks_the_lip_and_launches_from_its_own_rear_support()
-> Result<(), Box<dyn std::error::Error>> {
    let source = GoshawkHullSource::bundled()?;
    let station = crate::OriginalStationData::bundled()?;
    let rail = source_vertex(VERTICES, 6)?;
    let port_z = station.port().center().z;
    // Inward yaw negates local X and Z. The source rail passes through face
    // 482's strict interior while the body centre remains outside the port.
    let centre_x = 69.4 + rail.x;
    let contact_centre_z = port_z + rail.z;
    let before = inward_at(centre_x, contact_centre_z + 1.0);
    let penetrating = inward_at(centre_x, contact_centre_z - 1.0);
    assert!(station.depth(penetrating) > 0.0);
    assert!(station.goshawk_motion_clear(&source, before, before, 0.0));
    assert!(!station.goshawk_motion_clear(&source, before, penetrating, 0.0));

    let launch = station.goshawk_launch_start(&source)?;
    // Independent source pin: station max Z 136.32 m, Goshawk aft
    // vertex 1920 at -17.038276800032676 m, plus 0.02 m and 1,000 m.
    // The least outward f32 is 1153.3782958984375 m.
    assert_eq!(launch.position, [0.0, 0.0, f32::from_bits(0x4490_2c1b)]);
    let pose = crate::station::flight_pose(launch.position, launch.orientation)
        .ok_or(EnvelopeError::InvalidGeometry)?;
    let outward = RigidPose {
        position: Vec3::new(pose.position.x, pose.position.y, pose.position.z + 2.4),
        ..pose
    };
    assert!(station.goshawk_motion_clear(&source, pose, outward, 0.0));
    Ok(())
}

use super::*;
use crate::docking::Basis3;
use crate::goshawk::source_vertex;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh010/source-triangles.bin");

fn pose(x: f64, z: f64, inward: bool) -> RigidPose {
    RigidPose {
        position: Vec3::new(x, 0.0, z),
        orientation: if inward {
            Basis3 {
                x: Vec3::new(-1.0, 0.0, 0.0),
                y: Vec3::new(0.0, 1.0, 0.0),
                z: Vec3::new(0.0, 0.0, -1.0),
            }
        } else {
            Basis3::IDENTITY
        },
    }
}

#[test]
fn sh010_encloses_every_original_triangle_and_refuses_other_or_changed_sources()
-> Result<(), EnvelopeError> {
    let source = GoshawkCustomHullSource::bundled()?;
    assert_eq!(source.pieces().len(), 73);
    assert_eq!(source.engineering(), CUSTOM_ENGINEERING);
    assert_eq!(source.engineering().dry_mass_kilograms, 32411);
    assert!(source.maximum_origin_radius() > 17.0);
    assert_eq!(
        TRIANGLES.len(),
        (CUSTOM_DETAILED_FACES + CUSTOM_NATIVE_FACES) * 12
    );
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
        let indices = [*a, *b, *c].map(|word| u32::from_le_bytes(word) as usize);
        let group = MEMBERSHIP[indices[0]];
        assert!(indices.iter().all(|&index| MEMBERSHIP[index] == group));
        let [a, b, c] = [
            source_vertex(VERTICES, indices[0])?,
            source_vertex(VERTICES, indices[1])?,
            source_vertex(VERTICES, indices[2])?,
        ];
        let centre = Vec3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        assert!(source.pieces()[usize::from(group)].contains(centre));
    }
    // All four mount structures retain their own supports in both original source sets.
    for (piece, start_detail, start_native) in [
        (69, 2072, 536),
        (70, 2080, 544),
        (71, 2088, 552),
        (72, 2096, 560),
    ] {
        assert_eq!(source.pieces()[piece].source_points().len(), 16);
        for offset in 0..8 {
            let d = source_vertex(VERTICES, start_detail + offset)?;
            let n = source_vertex(VERTICES, CUSTOM_DETAILED_VERTICES + start_native + offset)?;
            assert_eq!(d, n);
            assert!(source.pieces()[piece].source_points().contains(&d));
        }
    }
    let mut edited = MEMBERSHIP.to_vec();
    edited[0] ^= 1;
    assert_eq!(
        GoshawkCustomHullSource::from_bytes(VERTICES, &edited),
        Err(EnvelopeError::ChangedSource)
    );
    let mut coordinates = VERTICES.to_vec();
    coordinates[24] ^= 1;
    assert_eq!(
        GoshawkCustomHullSource::from_bytes(&coordinates, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        GoshawkCustomHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    assert_eq!(
        calibrated_pieces::<CUSTOM_PIECES>(
            VERTICES,
            MEMBERSHIP,
            b"MSH008V1",
            CUSTOM_DETAILED_VERTICES,
            CUSTOM_NATIVE_VERTICES
        ),
        Err(EnvelopeError::InvalidContent)
    );
    edited[0] = u8::MAX;
    assert_eq!(
        calibrated_pieces::<CUSTOM_PIECES>(
            VERTICES,
            &edited,
            b"MSH010V1",
            CUSTOM_DETAILED_VERTICES,
            CUSTOM_NATIVE_VERTICES
        ),
        Err(EnvelopeError::InvalidContent)
    );
    Ok(())
}

#[test]
fn sh010_preserves_the_forward_channel_and_detects_mount_contact() -> Result<(), EnvelopeError> {
    let source = GoshawkCustomHullSource::bundled()?;
    let start = pose(0.0, 0.0, false);
    let end = pose(0.0, 10.0, false);
    // Independently certified in integration/sh010-goshawk-custom/FIT-WITNESS.json.
    let channel = [
        Vec3::new(-0.15, -1.65, 14.0),
        Vec3::new(0.15, -1.65, 14.0),
        Vec3::new(0.0, -1.35, 14.0),
    ];
    assert!(source.sweep_clears_triangles(start, end, 0.0, &[channel])?);
    for index in [8, 2080, 2096] {
        let source_point = source_vertex(VERTICES, index)?;
        let contact = [
            source_point,
            Vec3::new(source_point.x + 0.003, source_point.y, source_point.z),
            Vec3::new(source_point.x, source_point.y + 0.003, source_point.z),
        ];
        assert!(!source.sweep_clears_triangles(start, start, 0.0, &[contact])?);
    }
    assert_eq!(
        source.sweep_clears_triangles(start, end, 0.0, &[]),
        Err(EnvelopeError::InvalidGeometry)
    );
    Ok(())
}

#[test]
fn sh010_station_uses_original_fit_speed_attitude_and_swept_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let source = GoshawkCustomHullSource::bundled()?;
    let station = crate::OriginalStationData::bundled()?;
    let z = station.port().center().z;
    let before = pose(0.0, z + 0.5, true);
    let after = pose(0.0, z - 0.5, true);
    assert!(source.fits_port(station.port(), before, station.station_pose(), 0.0)?);
    assert_eq!(
        source.fits_port(station.port(), before, station.station_pose(), f64::NAN),
        Err(EnvelopeError::InvalidGeometry)
    );
    assert!(station.goshawk_custom_motion_clear(&source, before, after, 0.0));
    assert!(station.goshawk_custom_entry_clear(&source, before, after, 72.0, 0.0, 0.0));
    assert!(!station.goshawk_custom_entry_clear(&source, before, after, 144.0, 0.0, 0.0));
    assert!(!station.goshawk_custom_entry_clear(&source, before, after, 72.0, 0.1, 0.0));
    assert!(!station.goshawk_custom_entry_clear(
        &source,
        before,
        pose(0.0, z - 0.5, false),
        72.0,
        0.0,
        0.0
    ));
    let off_before = pose(100.0, z + 0.5, true);
    let off_after = pose(100.0, z - 0.5, true);
    assert!(!source.fits_port(station.port(), off_before, station.station_pose(), 0.0)?);
    assert!(station.goshawk_custom_motion_clear(&source, off_before, off_after, 0.0));
    assert!(!station.goshawk_custom_entry_clear(&source, off_before, off_after, 72.0, 0.0, 0.0));

    // Actual SH010 detailed rail vertex 6 crosses the strict interior of SH028 face 482
    // while the hull origin remains outside the port. Its separate source witness pins both.
    let rail = source_vertex(VERTICES, 6)?;
    let x = 69.4 + rail.x;
    let contact_z = z + rail.z;
    let opening = pose(x, contact_z + 1.0, true);
    let penetration = pose(x, contact_z - 1.0, true);
    assert!(station.depth(penetration) > 0.0);
    assert!(station.goshawk_custom_motion_clear(&source, opening, opening, 0.0));
    assert!(!station.goshawk_custom_motion_clear(&source, opening, penetration, 0.0));

    let launch = station.goshawk_custom_launch_start(&source)?;
    assert_eq!(launch.position, [0.0, 0.0, f32::from_bits(0x4490_2b9e)]);
    let start = crate::station::flight_pose(launch.position, launch.orientation)
        .ok_or(EnvelopeError::InvalidGeometry)?;
    let end = RigidPose {
        position: Vec3::new(start.position.x, start.position.y, start.position.z + 2.4),
        ..start
    };
    assert!(station.goshawk_custom_motion_clear(&source, start, end, 0.0));
    Ok(())
}

use super::*;
use crate::docking::Basis3;
use crate::goshawk::source_vertex;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh021/source-triangles.bin");

// These independent IEEE-754 words are derived from the immutable original physical
// JSON and the direct rational scaling of detailed vertex 96, not from ZEBRA_ENGINEERING.
// The Python source tests bind every word back to those inputs.
const ORIGINAL_MASS_KG: u32 = 60108;
const ORIGINAL_COM_BITS: [u64; 3] = [
    0xbfc5_3f2f_ff3e_9deb,
    0xbff9_5ab9_cb25_6dd0,
    0xc00d_c963_d918_32cb,
];
const ORIGINAL_INERTIA_BITS: [u64; 9] = [
    0x415c_5ca5_60d3_234e,
    0xc0c5_0cd5_4919_768d,
    0x40d9_4a2a_69eb_87e0,
    0xc0c5_0cd5_4919_768d,
    0x4162_b6e3_e28c_fdb5,
    0x40f8_ad68_1682_33ac,
    0x40d9_4a2a_69eb_87e0,
    0x40f8_ad68_1682_33ac,
    0x414a_3c0d_4274_a754,
];
const ORIGINAL_SCALE: [u32; 2] = [40, 3];
const ORIGINAL_SCALE_BITS: u64 = 0x402a_aaaa_aaaa_aaab;
const ORIGINAL_VERTEX96_BITS: [u64; 3] = [
    0xc015_c28f_5c28_f5c3,
    0xc005_5555_5555_5555,
    0x4034_0000_0000_0000,
];

#[test]
fn sh021_compiled_engineering_and_calibrated_source_match_independent_bits()
-> Result<(), EnvelopeError> {
    let source = ZebraHullSource::bundled()?;
    let engineering = source.engineering();
    assert_eq!(engineering.dry_mass_kilograms, ORIGINAL_MASS_KG);
    assert_eq!(
        engineering.center_of_mass_metres.map(f64::to_bits),
        ORIGINAL_COM_BITS
    );
    assert_eq!(
        engineering.inertia_about_center_kg_m2.map(f64::to_bits),
        ORIGINAL_INERTIA_BITS
    );
    assert_eq!(
        [engineering.scale_numerator, engineering.scale_denominator],
        ORIGINAL_SCALE
    );
    assert_eq!(
        (f64::from(engineering.scale_numerator) / f64::from(engineering.scale_denominator))
            .to_bits(),
        ORIGINAL_SCALE_BITS
    );
    let point = source_vertex(VERTICES, 96)?;
    assert_eq!(
        [point.x, point.y, point.z].map(f64::to_bits),
        ORIGINAL_VERTEX96_BITS
    );
    Ok(())
}

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
fn sh021_encloses_every_original_triangle_and_refuses_other_or_changed_sources()
-> Result<(), EnvelopeError> {
    let source = ZebraHullSource::bundled()?;
    assert_eq!(source.pieces().len(), 48);
    assert_eq!(source.engineering(), ZEBRA_ENGINEERING);
    assert_eq!(source.engineering().dry_mass_kilograms, 60108);
    assert!(source.maximum_origin_radius() > 22.0);
    assert_eq!(
        TRIANGLES.len(),
        (ZEBRA_DETAILED_FACES + ZEBRA_NATIVE_FACES) * 12
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
    // The original two freight keels and two cross-bridges remain independent.
    for (index, piece) in [(0, 0), (568, 15), (1136, 30), (1144, 31)] {
        assert_eq!(MEMBERSHIP[index], piece);
    }
    assert_eq!(ZEBRA_NATIVE_FACES, 744);
    assert_eq!(ZEBRA_NATIVE_EXACT_FACE_LIMIT, 512);
    let mut edited = MEMBERSHIP.to_vec();
    edited[0] ^= 1;
    assert_eq!(
        ZebraHullSource::from_bytes(VERTICES, &edited),
        Err(EnvelopeError::ChangedSource)
    );
    let mut coordinates = VERTICES.to_vec();
    coordinates[24] ^= 1;
    assert_eq!(
        ZebraHullSource::from_bytes(&coordinates, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        ZebraHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let custom = include_bytes!("../../../../assets/fleet/sh010/source.vertices");
    assert_eq!(
        ZebraHullSource::from_bytes(custom, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut wrong_sibling_identity = VERTICES.to_vec();
    wrong_sibling_identity[..8].copy_from_slice(b"MSH020V1");
    assert_eq!(
        ZebraHullSource::from_bytes(&wrong_sibling_identity, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    assert_eq!(
        calibrated_pieces::<ZEBRA_PIECES>(
            VERTICES,
            MEMBERSHIP,
            b"MSH008V1",
            ZEBRA_DETAILED_VERTICES,
            ZEBRA_NATIVE_VERTICES
        ),
        Err(EnvelopeError::InvalidContent)
    );
    edited[0] = u8::MAX;
    assert_eq!(
        calibrated_pieces::<ZEBRA_PIECES>(
            VERTICES,
            &edited,
            b"MSH021V1",
            ZEBRA_DETAILED_VERTICES,
            ZEBRA_NATIVE_VERTICES
        ),
        Err(EnvelopeError::InvalidContent)
    );
    Ok(())
}

#[test]
fn sh021_preserves_full_freight_trench_clearance_and_detects_source_contact()
-> Result<(), EnvelopeError> {
    let source = ZebraHullSource::bundled()?;
    let start = pose(0.0, 0.0, false);
    let end = pose(0.0, 40.0, false);
    // Independently certified across the full original 40m freight trench.
    let channel = [
        Vec3::new(-0.15, -2.1, 20.0),
        Vec3::new(0.15, -2.1, 20.0),
        Vec3::new(0.0, -1.8, 20.0),
    ];
    assert!(source.sweep_clears_triangles(start, end, 0.0, &[channel])?);
    for index in [0, 568, 1136, 1144, 1152, 1280] {
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
fn sh021_station_uses_original_fit_speed_attitude_and_swept_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ZebraHullSource::bundled()?;
    let station = crate::OriginalStationData::bundled()?;
    let z = station.port().center().z;
    let before = pose(0.0, z + 0.5, true);
    let after = pose(0.0, z - 0.5, true);
    assert!(source.fits_port(station.port(), before, station.station_pose(), 0.0)?);
    assert_eq!(
        source.fits_port(station.port(), before, station.station_pose(), f64::NAN),
        Err(EnvelopeError::InvalidGeometry)
    );
    assert!(station.zebra_motion_clear(&source, before, after, 0.0));
    assert!(station.zebra_entry_clear(&source, before, after, 72.0, 0.0, 0.0));
    assert!(!station.zebra_entry_clear(&source, before, after, 144.0, 0.0, 0.0));
    assert!(!station.zebra_entry_clear(&source, before, after, 72.0, 0.1, 0.0));
    assert!(!station.zebra_entry_clear(&source, before, pose(0.0, z - 0.5, false), 72.0, 0.0, 0.0));
    let off_before = pose(100.0, z + 0.5, true);
    let off_after = pose(100.0, z - 0.5, true);
    assert!(!source.fits_port(station.port(), off_before, station.station_pose(), 0.0)?);
    assert!(station.zebra_motion_clear(&source, off_before, off_after, 0.0));
    assert!(!station.zebra_entry_clear(&source, off_before, off_after, 72.0, 0.0, 0.0));

    // Actual SH021 detailed forebody vertex 96 crosses the strict interior of SH028 face 482
    // while the hull origin remains outside the port. Its separate source witness pins both.
    let rail = source_vertex(VERTICES, 96)?;
    let x = 69.4 + rail.x;
    let contact_z = z + rail.z;
    let opening = pose(x, contact_z + 1.0, true);
    let penetration = pose(x, contact_z - 1.0, true);
    assert!(station.depth(penetration) > 0.0);
    assert!(station.zebra_motion_clear(&source, opening, opening, 0.0));
    assert!(!station.zebra_motion_clear(&source, opening, penetration, 0.0));

    let launch = station.zebra_launch_start(&source)?;
    assert_eq!(launch.position, [0.0, 0.0, f32::from_bits(0x4490_8ae2)]);
    let start = crate::station::flight_pose(launch.position, launch.orientation)
        .ok_or(EnvelopeError::InvalidGeometry)?;
    let end = RigidPose {
        position: Vec3::new(start.position.x, start.position.y, start.position.z + 2.4),
        ..start
    };
    assert!(station.zebra_motion_clear(&source, start, end, 0.0));
    Ok(())
}

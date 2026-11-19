use super::*;
use crate::docking::Basis3;
use crate::goshawk::source_vertex;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh017/source-triangles.bin");

// These independent IEEE-754 words are derived from the immutable original physical
// JSON and the direct rational scaling of detailed vertex 1776, not from PROSPECTOR_ENGINEERING.
// The Python source tests bind every word back to those inputs.
const ORIGINAL_MASS_KG: u32 = 116421;
const ORIGINAL_COM_BITS: [u64; 3] = [
    0xbf1f_5b40_fade_4261,
    0xbffb_b48a_ddcd_6689,
    0xc004_3f0c_a1e0_14f1,
];
const ORIGINAL_INERTIA_BITS: [u64; 9] = [
    0x4172_9a0d_b022_276c,
    0x4067_d1f2_84cd_1f53,
    0xc07c_9967_e4aa_6ffd,
    0x4067_d1f2_84cd_1f53,
    0x417a_1d70_a7ba_0f1a,
    0x4135_5c0d_8b54_8fda,
    0xc07c_9967_e4aa_6ffd,
    0x4135_5c0d_8b54_8fda,
    0x4168_d5a1_5e87_7da4,
];
const ORIGINAL_SCALE: [u32; 2] = [920_000, 56_367];
const ORIGINAL_SCALE_BITS: u64 = 0x4030_5254_cfa0_7181;
const ORIGINAL_VERTEX1776_BITS: [u64; 3] = [
    0xc01f_58e7_305a_e7ed,
    0xc00a_1d54_7f67_1c02,
    0x4036_4842_0483_b0ab,
];

#[test]
fn sh017_compiled_engineering_and_calibrated_source_match_independent_bits()
-> Result<(), EnvelopeError> {
    let source = MoleProspectorHullSource::bundled()?;
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
    let point = source_vertex(VERTICES, 1776)?;
    assert_eq!(
        [point.x, point.y, point.z].map(f64::to_bits),
        ORIGINAL_VERTEX1776_BITS
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
fn sh017_encloses_every_original_triangle_and_refuses_other_or_changed_sources()
-> Result<(), EnvelopeError> {
    let source = MoleProspectorHullSource::bundled()?;
    assert_eq!(source.pieces().len(), 83);
    assert_eq!(source.engineering(), PROSPECTOR_ENGINEERING);
    assert_eq!(source.engineering().dry_mass_kilograms, 116421);
    assert!(source.maximum_origin_radius() > 28.0);
    assert_eq!(
        TRIANGLES.len(),
        (PROSPECTOR_DETAILED_FACES + PROSPECTOR_NATIVE_FACES) * 12
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
    // Both boring heads and all eight source-only collars own separate supports.
    assert_eq!(MEMBERSHIP[1776], 52);
    assert_eq!(MEMBERSHIP[4112], 58);
    for (index, piece) in [
        (1784, 53),
        (2360, 54),
        (2936, 55),
        (3512, 56),
        (4120, 59),
        (4696, 60),
        (5272, 61),
        (5848, 62),
    ] {
        assert_eq!(MEMBERSHIP[index], piece);
    }
    let mut edited = MEMBERSHIP.to_vec();
    edited[0] ^= 1;
    assert_eq!(
        MoleProspectorHullSource::from_bytes(VERTICES, &edited),
        Err(EnvelopeError::ChangedSource)
    );
    let mut coordinates = VERTICES.to_vec();
    coordinates[24] ^= 1;
    assert_eq!(
        MoleProspectorHullSource::from_bytes(&coordinates, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        MoleProspectorHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let custom = include_bytes!("../../../../assets/fleet/sh010/source.vertices");
    assert_eq!(
        MoleProspectorHullSource::from_bytes(custom, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut wrong_mole_identity = VERTICES.to_vec();
    wrong_mole_identity[..8].copy_from_slice(b"MSH016V1");
    assert_eq!(
        MoleProspectorHullSource::from_bytes(&wrong_mole_identity, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    assert_eq!(
        calibrated_pieces::<PROSPECTOR_PIECES>(
            VERTICES,
            MEMBERSHIP,
            b"MSH008V1",
            PROSPECTOR_DETAILED_VERTICES,
            PROSPECTOR_NATIVE_VERTICES
        ),
        Err(EnvelopeError::InvalidContent)
    );
    edited[0] = u8::MAX;
    assert_eq!(
        calibrated_pieces::<PROSPECTOR_PIECES>(
            VERTICES,
            &edited,
            b"MSH017V1",
            PROSPECTOR_DETAILED_VERTICES,
            PROSPECTOR_NATIVE_VERTICES
        ),
        Err(EnvelopeError::InvalidContent)
    );
    Ok(())
}

#[test]
fn sh017_preserves_tool_cradle_clearance_and_detects_source_contact() -> Result<(), EnvelopeError> {
    let source = MoleProspectorHullSource::bundled()?;
    let start = pose(0.0, 0.0, false);
    let end = pose(0.0, 8.0, false);
    // Independently certified in integration/sh017-mole-prospector/FIT-WITNESS.json.
    let channel = [
        Vec3::new(-0.15, -4.0, 17.0),
        Vec3::new(0.15, -4.0, 17.0),
        Vec3::new(0.0, -3.7, 17.0),
    ];
    assert!(source.sweep_clears_triangles(start, end, 0.0, &[channel])?);
    for index in [568, 576, 1776, 4112, 1784, 4120] {
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
fn sh017_station_uses_original_fit_speed_attitude_and_swept_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let source = MoleProspectorHullSource::bundled()?;
    let station = crate::OriginalStationData::bundled()?;
    let z = station.port().center().z;
    let before = pose(0.0, z + 0.5, true);
    let after = pose(0.0, z - 0.5, true);
    assert!(source.fits_port(station.port(), before, station.station_pose(), 0.0)?);
    assert_eq!(
        source.fits_port(station.port(), before, station.station_pose(), f64::NAN),
        Err(EnvelopeError::InvalidGeometry)
    );
    assert!(station.mole_prospector_motion_clear(&source, before, after, 0.0));
    assert!(station.mole_prospector_entry_clear(&source, before, after, 72.0, 0.0, 0.0));
    assert!(!station.mole_prospector_entry_clear(&source, before, after, 144.0, 0.0, 0.0));
    assert!(!station.mole_prospector_entry_clear(&source, before, after, 72.0, 0.1, 0.0));
    assert!(!station.mole_prospector_entry_clear(
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
    assert!(station.mole_prospector_motion_clear(&source, off_before, off_after, 0.0));
    assert!(!station.mole_prospector_entry_clear(&source, off_before, off_after, 72.0, 0.0, 0.0));

    // Actual SH017 detailed forebody vertex 1776 crosses the strict interior of SH028 face 482
    // while the hull origin remains outside the port. Its separate source witness pins both.
    let rail = source_vertex(VERTICES, 1776)?;
    let x = 69.4 + rail.x;
    let contact_z = z + rail.z;
    let opening = pose(x, contact_z + 1.0, true);
    let penetration = pose(x, contact_z - 1.0, true);
    assert!(station.depth(penetration) > 0.0);
    assert!(station.mole_prospector_motion_clear(&source, opening, opening, 0.0));
    assert!(!station.mole_prospector_motion_clear(&source, opening, penetration, 0.0));

    let launch = station.mole_prospector_launch_start(&source)?;
    assert_eq!(launch.position, [0.0, 0.0, f32::from_bits(0x4491_01da)]);
    let start = crate::station::flight_pose(launch.position, launch.orientation)
        .ok_or(EnvelopeError::InvalidGeometry)?;
    let end = RigidPose {
        position: Vec3::new(start.position.x, start.position.y, start.position.z + 2.4),
        ..start
    };
    assert!(station.mole_prospector_motion_clear(&source, start, end, 0.0));
    Ok(())
}

use super::*;
use crate::docking::Basis3;
use crate::goshawk::source_vertex;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh022/source-triangles.bin");

// These independent IEEE-754 words are derived from the immutable original physical
// JSON and the direct rational scaling of detailed vertex 96, not from PEREGRINE_I_ENGINEERING.
// The Python source tests bind every word back to those inputs.
const ORIGINAL_MASS_KG: u32 = 103972;
const ORIGINAL_COM_BITS: [u64; 3] = [
    0xbfc1_92f5_d623_439e,
    0xbfef_0349_5858_7aea,
    0xc010_18d0_0839_5308,
];
const ORIGINAL_INERTIA_BITS: [u64; 9] = [
    0x4163_ae17_49ef_12de,
    0xc0b5_5fde_0258_5d54,
    0x40e4_04e4_d254_fee2,
    0xc0b5_5fde_0258_5d54,
    0x4170_fea9_0129_f6be,
    0x411d_b880_5823_6eea,
    0x40e4_04e4_d254_fee2,
    0x411d_b880_5823_6eea,
    0x4162_f11a_5a4d_5575,
];
const ORIGINAL_SCALE: [u32; 2] = [800, 59];
const ORIGINAL_SCALE_BITS: u64 = 0x402b_1e5f_7527_0d04;
const ORIGINAL_VERTEX96_BITS: [u64; 3] = [
    0xc021_7dd4_9c34_115b,
    0xbff5_b1e5_f752_70d0,
    0x4033_be0c_262e_dc6c,
];

#[test]
fn sh022_compiled_engineering_and_calibrated_source_match_independent_bits()
-> Result<(), EnvelopeError> {
    let source = PeregrineMkIHullSource::bundled()?;
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
fn sh022_encloses_every_original_triangle_and_refuses_other_or_changed_sources()
-> Result<(), EnvelopeError> {
    let source = PeregrineMkIHullSource::bundled()?;
    assert_eq!(source.pieces().len(), 69);
    assert_eq!(source.engineering(), PEREGRINE_I_ENGINEERING);
    assert_eq!(source.engineering().dry_mass_kilograms, 103972);
    assert!(source.maximum_origin_radius() > 24.5);
    assert_eq!(
        TRIANGLES.len(),
        (PEREGRINE_I_DETAILED_FACES + PEREGRINE_I_NATIVE_FACES) * 12
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
    // The original two shoulders, their longitudinal fork members and aft bridge stay local.
    for (index, piece) in [
        (0, 0),
        (568, 15),
        (1136, 30),
        (1144, 31),
        (1152, 32),
        (1160, 33),
        (1728, 48),
    ] {
        assert_eq!(MEMBERSHIP[index], piece);
    }
    assert_eq!(PEREGRINE_I_NATIVE_FACES, 964);
    assert_eq!(PEREGRINE_I_NATIVE_EXACT_FACE_LIMIT, 512);
    let mut edited = MEMBERSHIP.to_vec();
    edited[0] ^= 1;
    assert_eq!(
        PeregrineMkIHullSource::from_bytes(VERTICES, &edited),
        Err(EnvelopeError::ChangedSource)
    );
    let mut coordinates = VERTICES.to_vec();
    coordinates[24] ^= 1;
    assert_eq!(
        PeregrineMkIHullSource::from_bytes(&coordinates, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        PeregrineMkIHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let custom = include_bytes!("../../../../assets/fleet/sh010/source.vertices");
    assert_eq!(
        PeregrineMkIHullSource::from_bytes(custom, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut wrong_sibling_identity = VERTICES.to_vec();
    wrong_sibling_identity[..8].copy_from_slice(b"MSH023V1");
    assert_eq!(
        PeregrineMkIHullSource::from_bytes(&wrong_sibling_identity, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    assert_eq!(
        calibrated_pieces::<PEREGRINE_I_PIECES>(
            VERTICES,
            MEMBERSHIP,
            b"MSH008V1",
            PEREGRINE_I_DETAILED_VERTICES,
            PEREGRINE_I_NATIVE_VERTICES
        ),
        Err(EnvelopeError::InvalidContent)
    );
    edited[0] = u8::MAX;
    assert_eq!(
        calibrated_pieces::<PEREGRINE_I_PIECES>(
            VERTICES,
            &edited,
            b"MSH022V1",
            PEREGRINE_I_DETAILED_VERTICES,
            PEREGRINE_I_NATIVE_VERTICES
        ),
        Err(EnvelopeError::InvalidContent)
    );
    Ok(())
}

#[test]
fn sh022_preserves_hooked_forward_bow_gap_and_detects_source_contact() -> Result<(), EnvelopeError>
{
    let source = PeregrineMkIHullSource::bundled()?;
    let start = pose(0.0, 0.0, false);
    let end = pose(0.0, 10.0, false);
    // Independently certified across the original hooked forward bow opening.
    let channel = [
        Vec3::new(-0.15, -0.65, 19.0),
        Vec3::new(0.15, -0.65, 19.0),
        Vec3::new(0.0, -0.35, 19.0),
    ];
    assert!(source.sweep_clears_triangles(start, end, 0.0, &[channel])?);
    for index in [0, 568, 1136, 1144, 1152, 1160, 1728] {
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
fn sh022_station_uses_original_fit_speed_attitude_and_swept_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let source = PeregrineMkIHullSource::bundled()?;
    let station = crate::OriginalStationData::bundled()?;
    let z = station.port().center().z;
    let before = pose(0.0, z + 0.5, true);
    let after = pose(0.0, z - 0.5, true);
    assert!(source.fits_port(station.port(), before, station.station_pose(), 0.0)?);
    assert_eq!(
        source.fits_port(station.port(), before, station.station_pose(), f64::NAN),
        Err(EnvelopeError::InvalidGeometry)
    );
    assert!(station.peregrine_mk_i_motion_clear(&source, before, after, 0.0));
    assert!(station.peregrine_mk_i_entry_clear(&source, before, after, 72.0, 0.0, 0.0));
    assert!(!station.peregrine_mk_i_entry_clear(&source, before, after, 144.0, 0.0, 0.0));
    assert!(!station.peregrine_mk_i_entry_clear(&source, before, after, 72.0, 0.1, 0.0));
    assert!(!station.peregrine_mk_i_entry_clear(
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
    assert!(station.peregrine_mk_i_motion_clear(&source, off_before, off_after, 0.0));
    assert!(!station.peregrine_mk_i_entry_clear(&source, off_before, off_after, 72.0, 0.0, 0.0));

    // Actual SH022 detailed forebody vertex 96 crosses the strict interior of SH028 face 482
    // while the hull origin remains outside the port. Its separate source witness pins both.
    let rail = source_vertex(VERTICES, 96)?;
    let x = 69.4 + rail.x;
    let contact_z = z + rail.z;
    let opening = pose(x, contact_z + 1.0, true);
    let penetration = pose(x, contact_z - 1.0, true);
    assert!(station.depth(penetration) > 0.0);
    assert!(station.peregrine_mk_i_motion_clear(&source, opening, opening, 0.0));
    assert!(!station.peregrine_mk_i_motion_clear(&source, opening, penetration, 0.0));

    let launch = station.peregrine_mk_i_launch_start(&source)?;
    assert_eq!(launch.position, [0.0, 0.0, f32::from_bits(0x4490_9320)]);
    let start = crate::station::flight_pose(launch.position, launch.orientation)
        .ok_or(EnvelopeError::InvalidGeometry)?;
    let end = RigidPose {
        position: Vec3::new(start.position.x, start.position.y, start.position.z + 2.4),
        ..start
    };
    assert!(station.peregrine_mk_i_motion_clear(&source, start, end, 0.0));
    Ok(())
}

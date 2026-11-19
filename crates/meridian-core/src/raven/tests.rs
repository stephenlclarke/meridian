use super::*;
use crate::docking::Basis3;
use crate::goshawk::source_vertex;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh015/source-triangles.bin");

// These independent IEEE-754 words are derived from the immutable original physical
// JSON and the direct rational scaling of detailed vertex 96, not from RAVEN_ENGINEERING.
// The Python source tests bind every word back to those inputs.
const ORIGINAL_MASS_KG: u32 = 57064;
const ORIGINAL_COM_BITS: [u64; 3] = [
    0xbecd_62bd_568a_d7b5,
    0x3fe0_1648_b234_cf6b,
    0xc008_8ba2_56fe_1e87,
];
const ORIGINAL_INERTIA_BITS: [u64; 9] = [
    0x4143_5061_a66a_01d2,
    0x4027_d1b2_d552_bbc1,
    0xc056_3685_979b_dea7,
    0x4027_d1b2_d552_bbc1,
    0x415a_113e_87f3_cee2,
    0x40ee_2d76_6ff8_5cca,
    0xc056_3685_979b_dea7,
    0x40ee_2d76_6ff8_5cca,
    0x4152_8616_5e7d_4f5a,
];
const ORIGINAL_SCALE: [u32; 2] = [30_000, 2_521];
const ORIGINAL_SCALE_BITS: u64 = 0x4027_ccd1_ffcc_0208;
const ORIGINAL_VERTEX96_BITS: [u64; 3] = [
    0x3fed_b3d6_f99a_3ffa,
    0x0000_0000_0000_0000,
    0x402d_8f48_672b_f848,
];

#[test]
fn sh015_compiled_engineering_and_calibrated_source_match_independent_bits()
-> Result<(), EnvelopeError> {
    let source = RavenHullSource::bundled()?;
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
fn sh015_encloses_every_original_triangle_and_refuses_other_or_changed_sources()
-> Result<(), EnvelopeError> {
    let source = RavenHullSource::bundled()?;
    assert_eq!(source.pieces().len(), 70);
    assert_eq!(source.engineering(), RAVEN_ENGINEERING);
    assert_eq!(source.engineering().dry_mass_kilograms, 57064);
    assert!(source.maximum_origin_radius() > 17.0);
    assert_eq!(
        TRIANGLES.len(),
        (RAVEN_DETAILED_FACES + RAVEN_NATIVE_FACES) * 12
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
    // The original bilateral wings retain independent supports, not one spar-spanning hull.
    assert_eq!(MEMBERSHIP[568], 15);
    assert_eq!(MEMBERSHIP[576], 16);
    assert_ne!(MEMBERSHIP[568], MEMBERSHIP[576]);
    let mut edited = MEMBERSHIP.to_vec();
    edited[0] ^= 1;
    assert_eq!(
        RavenHullSource::from_bytes(VERTICES, &edited),
        Err(EnvelopeError::ChangedSource)
    );
    let mut coordinates = VERTICES.to_vec();
    coordinates[24] ^= 1;
    assert_eq!(
        RavenHullSource::from_bytes(&coordinates, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        RavenHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    assert_eq!(
        calibrated_pieces::<RAVEN_PIECES>(
            VERTICES,
            MEMBERSHIP,
            b"MSH008V1",
            RAVEN_DETAILED_VERTICES,
            RAVEN_NATIVE_VERTICES
        ),
        Err(EnvelopeError::InvalidContent)
    );
    edited[0] = u8::MAX;
    assert_eq!(
        calibrated_pieces::<RAVEN_PIECES>(
            VERTICES,
            &edited,
            b"MSH015V1",
            RAVEN_DETAILED_VERTICES,
            RAVEN_NATIVE_VERTICES
        ),
        Err(EnvelopeError::InvalidContent)
    );
    Ok(())
}

#[test]
fn sh015_preserves_bilateral_clearance_and_detects_source_contact() -> Result<(), EnvelopeError> {
    let source = RavenHullSource::bundled()?;
    let start = pose(0.0, 0.0, false);
    let end = pose(0.0, 8.0, false);
    // Independently certified in integration/sh015-raven/FIT-WITNESS.json.
    for centre in [-7.5, 7.5] {
        let channel = [
            Vec3::new(centre - 0.15, 1.0, 10.0),
            Vec3::new(centre + 0.15, 1.0, 10.0),
            Vec3::new(centre, 1.3, 10.0),
        ];
        assert!(source.sweep_clears_triangles(start, end, 0.0, &[channel])?);
    }
    for index in [96, 568, 576, 1728, 1736] {
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
fn sh015_station_uses_original_fit_speed_attitude_and_swept_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let source = RavenHullSource::bundled()?;
    let station = crate::OriginalStationData::bundled()?;
    let z = station.port().center().z;
    let before = pose(0.0, z + 0.5, true);
    let after = pose(0.0, z - 0.5, true);
    assert!(source.fits_port(station.port(), before, station.station_pose(), 0.0)?);
    assert_eq!(
        source.fits_port(station.port(), before, station.station_pose(), f64::NAN),
        Err(EnvelopeError::InvalidGeometry)
    );
    assert!(station.raven_motion_clear(&source, before, after, 0.0));
    assert!(station.raven_entry_clear(&source, before, after, 72.0, 0.0, 0.0));
    assert!(!station.raven_entry_clear(&source, before, after, 144.0, 0.0, 0.0));
    assert!(!station.raven_entry_clear(&source, before, after, 72.0, 0.1, 0.0));
    assert!(!station.raven_entry_clear(&source, before, pose(0.0, z - 0.5, false), 72.0, 0.0, 0.0));
    let off_before = pose(100.0, z + 0.5, true);
    let off_after = pose(100.0, z - 0.5, true);
    assert!(!source.fits_port(station.port(), off_before, station.station_pose(), 0.0)?);
    assert!(station.raven_motion_clear(&source, off_before, off_after, 0.0));
    assert!(!station.raven_entry_clear(&source, off_before, off_after, 72.0, 0.0, 0.0));

    // Actual SH015 detailed forebody vertex 96 crosses the strict interior of SH028 face 482
    // while the hull origin remains outside the port. Its separate source witness pins both.
    let rail = source_vertex(VERTICES, 96)?;
    let x = 69.4 + rail.x;
    let contact_z = z + rail.z;
    let opening = pose(x, contact_z + 1.0, true);
    let penetration = pose(x, contact_z - 1.0, true);
    assert!(station.depth(penetration) > 0.0);
    assert!(station.raven_motion_clear(&source, opening, opening, 0.0));
    assert!(!station.raven_motion_clear(&source, opening, penetration, 0.0));

    let launch = station.raven_launch_start(&source)?;
    assert_eq!(launch.position, [0.0, 0.0, f32::from_bits(0x448f_f1ed)]);
    let start = crate::station::flight_pose(launch.position, launch.orientation)
        .ok_or(EnvelopeError::InvalidGeometry)?;
    let end = RigidPose {
        position: Vec3::new(start.position.x, start.position.y, start.position.z + 2.4),
        ..start
    };
    assert!(station.raven_motion_clear(&source, start, end, 0.0));
    Ok(())
}

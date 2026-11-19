use super::{GoshawkMk2HullSource, MEMBERSHIP, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh009/source-triangles.bin");

#[test]
fn goshawk_mk2_encloses_every_original_face_and_rejects_changed_source() -> Result<(), EnvelopeError>
{
    let source = GoshawkMk2HullSource::bundled()?;
    assert_eq!(source.pieces().len(), 73);
    let vertices = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(vertices.len(), 2672);
    assert_eq!(TRIANGLES.len(), 4544 * 12);
    for (index, slot) in MEMBERSHIP.iter().copied().enumerate() {
        let point = decode_point(&vertices[index]);
        let piece = &source.pieces()[usize::from(slot)];
        assert!(piece.source_points().contains(&point));
        assert_eq!(piece.rounding_metres(), 0.02);
    }
    for triangle in TRIANGLES.as_chunks::<12>().0 {
        let [a, b, c] = triangle.as_chunks::<4>().0 else {
            return Err(EnvelopeError::InvalidContent);
        };
        let ids = [*a, *b, *c].map(|i| u32::from_le_bytes(i) as usize);
        let slot = MEMBERSHIP[ids[0]];
        assert!(ids.iter().all(|i| MEMBERSHIP[*i] == slot));
        let [a, b, c] = ids.map(|i| decode_point(&vertices[i]));
        let centroid = Vec3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        assert!(source.pieces()[usize::from(slot)].contains(centroid));
    }

    let mut changed = VERTICES.to_vec();
    changed[24] ^= 1;
    assert_eq!(
        GoshawkMk2HullSource::from_bytes(&changed, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut changed = MEMBERSHIP.to_vec();
    changed[0] ^= 1;
    assert_eq!(
        GoshawkMk2HullSource::from_bytes(VERTICES, &changed),
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

fn probe(point: Vec3) -> [Vec3; 3] {
    [
        point,
        Vec3::new(point.x + 0.001, point.y, point.z),
        Vec3::new(point.x, point.y + 0.001, point.z),
    ]
}

#[test]
fn goshawk_mk2_forebody_sweep_clears_but_original_arm_blocks_contact() -> Result<(), EnvelopeError>
{
    let source = GoshawkMk2HullSource::bundled()?;
    let start = translated(0.0, 0.0);
    let channel = [
        Vec3::new(-0.15, -1.65, 14.0),
        Vec3::new(0.15, -1.65, 14.0),
        Vec3::new(0.0, -1.35, 14.0),
    ];
    assert!(source.sweep_clears_triangle(start, translated(0.0, 10.0), 0.0, channel)?);
    // Pinned detailed vertex 8, independently extracted in SH009-FIXTURES.json.
    let arm = Vec3::new(-6.989_351_398, -0.847_194_109, 2.591_566_779);
    assert!(!source.sweep_clears_triangle(start, start, 0.0, probe(arm))?);
    Ok(())
}

#[test]
fn goshawk_mk2_ventral_brackets_and_runners_leave_their_source_gap_clear()
-> Result<(), EnvelopeError> {
    let source = GoshawkMk2HullSource::bundled()?;
    let pose = translated(0.0, 0.0);
    // A single support per side would enclose the midpoint between the authored boxes.
    for x in [-5.093_519_248, 5.093_519_248] {
        let corner = Vec3::new(x, -2.941_646_211, 7.085_931_526);
        assert!(source.sweep_clears_triangle(pose, pose, 0.0, probe(corner))?);
    }
    Ok(())
}

fn inward(x: f64, z: f64) -> RigidPose {
    RigidPose {
        orientation: Basis3 {
            x: Vec3::new(-1.0, 0.0, 0.0),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(0.0, 0.0, -1.0),
        },
        ..translated(x, z)
    }
}

#[test]
fn goshawk_mk2_original_sh028_crossing_clears_but_swept_lip_blocks()
-> Result<(), Box<dyn std::error::Error>> {
    let source = GoshawkMk2HullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    // Absolute poses are independently bound to the original SH028 geometry.
    assert!(station.goshawk_mk2_motion_clear(
        &source,
        inward(0.0, 137.32),
        inward(0.0, 135.32),
        0.0,
    ));
    let lip_start = inward(60.847_631_951_266_89, 150.289_609_892_214_94);
    let lip_end = inward(60.847_631_951_266_89, 148.289_609_892_214_94);
    assert!(station.goshawk_mk2_motion_clear(&source, lip_start, lip_start, 0.0));
    assert!(!station.goshawk_mk2_motion_clear(&source, lip_start, lip_end, 0.0));
    Ok(())
}

#[test]
fn goshawk_mk2_carries_the_adopted_stock_tensor_and_split_interface() -> Result<(), EnvelopeError> {
    // Fixed binary64 words independently extracted from the retained MC1 proposal.
    let source = GoshawkMk2HullSource::bundled()?;
    let stock = source.engineering();
    assert_eq!(stock.dry_mass_kilograms, 33_072);
    assert_eq!(
        stock.proposal_sha256,
        "b35cbf08f9075f1bb99b88a374e078fed3f34ad06d8463554d585452981e7ede"
    );
    assert_eq!(
        stock.center_of_mass_metres.map(f64::to_bits),
        [
            0xbee2_9580_178b_5204,
            0x3fc9_60a5_8960_55d8,
            0xc001_3aac_5ab3_2bcb
        ]
    );
    assert_eq!(
        stock
            .inertia_about_center_kg_m2
            .map(|row| row.map(f64::to_bits)),
        [
            [
                0x4144_321f_38c5_aceb,
                0x4008_7ca9_968c_5341,
                0xc02b_fd0a_28a9_e331
            ],
            [
                0x4008_7ca9_968c_5341,
                0x414c_3840_b75e_8c16,
                0x40f9_b517_c5fa_8cae
            ],
            [
                0xc02b_fd0a_28a9_e331,
                0x40f9_b517_c5fa_8cae,
                0x4133_337f_e5ad_d4ff
            ],
        ]
    );
    assert_eq!(
        stock.empty_interface.map(|half| half.mass_kilograms),
        [70, 70]
    );
    assert_eq!(
        stock
            .empty_interface
            .map(|half| half.anchor_metres.map(f64::to_bits)),
        [
            [
                0xc012_ae00_1558_52d9,
                0xc010_30d9_a0df_501e,
                0x4021_2315_367f_1af9
            ],
            [
                0x4012_ae00_1558_52d9,
                0xc010_30d9_a0df_501e,
                0x4021_2315_367f_1af9
            ],
        ]
    );
    Ok(())
}

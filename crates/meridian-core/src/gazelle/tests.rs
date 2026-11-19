use super::*;
use crate::docking::Basis3;
use crate::goshawk::source_vertex;
use std::collections::BTreeSet;

const CANONICAL: &[u8] = include_bytes!("../../../../assets/fleet/sh019/source.vertices");
const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh019/source-triangles.bin");
const FACE_MEMBERSHIP: &[u8] =
    include_bytes!("../../../../assets/fleet/sh019/source-face-membership.bin");
const SOURCE_IDS: &[u8] =
    include_bytes!("../../../../assets/fleet/sh019/support-source-indices.bin");

fn coordinates(point: Vec3) -> [u64; 3] {
    [point.x, point.y, point.z].map(f64::to_bits)
}

#[test]
fn sh019_authored_metres_and_original_native_face_limit() -> Result<(), EnvelopeError> {
    let source = GazelleHullSource::bundled()?;
    assert_eq!(source.metres_per_source_unit().to_bits(), 1.0_f64.to_bits());
    assert_eq!(GAZELLE_NATIVE_FACES, 304);
    assert_eq!(GAZELLE_NATIVE_EXACT_FACE_LIMIT, 512);
    assert_eq!(GAZELLE_DETAILED_VERTICES, 3008);
    assert_eq!(GAZELLE_NATIVE_VERTICES, 172);
    assert_eq!(source_vertex(CANONICAL, 2862)?, Vec3::new(0.22, -3.9, 21.5));
    assert_eq!(
        source_vertex(CANONICAL, 1778)?,
        Vec3::new(-3.405, -1.0, -16.79)
    );
    assert!(source.maximum_origin_radius() > 21.8);
    Ok(())
}

#[test]
fn sh019_every_original_triangle_is_local_and_only_loop_seams_duplicate()
-> Result<(), EnvelopeError> {
    let source = GazelleHullSource::bundled()?;
    let count = GAZELLE_DETAILED_VERTICES + GAZELLE_NATIVE_VERTICES;
    let supports = GAZELLE_DETAILED_SUPPORT_POINTS + GAZELLE_NATIVE_SUPPORT_POINTS;
    assert_eq!(source.pieces().len(), 37);
    assert_eq!(CANONICAL.len(), 16 + 24 * count);
    assert_eq!(SUPPORT.len(), 16 + 24 * supports);
    assert_eq!(MEMBERSHIP.len(), supports);
    assert_eq!(SOURCE_IDS.len(), 4 * supports);
    assert_eq!(
        TRIANGLES.len(),
        12 * (GAZELLE_DETAILED_FACES + GAZELLE_NATIVE_FACES)
    );
    assert_eq!(TRIANGLES.len() / 12, FACE_MEMBERSHIP.len());

    let mut owners: [BTreeSet<usize>; GAZELLE_PIECES] = core::array::from_fn(|_| BTreeSet::new());
    let mut multiplicity = vec![0; count];
    for (i, (word, &owner)) in SOURCE_IDS
        .as_chunks::<4>()
        .0
        .iter()
        .zip(MEMBERSHIP)
        .enumerate()
    {
        let canonical = u32::from_le_bytes(*word) as usize;
        let piece = usize::from(owner);
        assert_eq!(
            i < GAZELLE_DETAILED_SUPPORT_POINTS,
            canonical < GAZELLE_DETAILED_VERTICES
        );
        assert!(owners[piece].insert(canonical));
        multiplicity[canonical] += 1;
        let point = source_vertex(CANONICAL, canonical)?;
        assert_eq!(coordinates(point), coordinates(source_vertex(SUPPORT, i)?));
        assert!(source.pieces()[piece].source_points().contains(&point));
        assert!(source.pieces()[piece].contains(point));
    }
    for (i, copies) in multiplicity.iter().copied().enumerate() {
        let detailed_seam = (676..1444).contains(&i) && (i - 676) % 96 < 12;
        let native_start = GAZELLE_DETAILED_VERTICES + 44;
        let native_seam = (native_start..native_start + 32).contains(&i);
        assert_eq!(copies, if detailed_seam || native_seam { 2 } else { 1 });
    }
    for (index, (face, &piece)) in TRIANGLES
        .as_chunks::<12>()
        .0
        .iter()
        .zip(FACE_MEMBERSHIP)
        .enumerate()
    {
        let [a, b, c] = face.as_chunks::<4>().0 else {
            return Err(EnvelopeError::InvalidContent);
        };
        let indices = [*a, *b, *c].map(|word| u32::from_le_bytes(word) as usize);
        assert!(
            indices
                .iter()
                .all(|i| owners[usize::from(piece)].contains(i))
        );
        assert!(
            indices
                .iter()
                .all(|&i| (i < GAZELLE_DETAILED_VERTICES) == (index < GAZELLE_DETAILED_FACES))
        );
        let [a, b, c] = [
            source_vertex(CANONICAL, indices[0])?,
            source_vertex(CANONICAL, indices[1])?,
            source_vertex(CANONICAL, indices[2])?,
        ];
        let centre = Vec3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        assert!(source.pieces()[usize::from(piece)].contains(centre));
    }
    for owner in &owners[2..10] {
        assert_eq!(owner.len(), 116);
    }
    assert!(
        source
            .pieces()
            .iter()
            .all(|piece| piece.rounding_metres() == 0.02)
    );
    Ok(())
}

#[test]
fn sh019_refuses_packed_sibling_canonical_or_changed_support() {
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    let custom = include_bytes!("../../../../assets/fleet/sh010/source.vertices");
    for wrong in [explorer.as_slice(), custom.as_slice(), CANONICAL] {
        assert_eq!(
            GazelleHullSource::from_bytes(wrong, MEMBERSHIP),
            Err(EnvelopeError::ChangedSource)
        );
    }
    let mut support = SUPPORT.to_vec();
    support[24] ^= 1;
    assert_eq!(
        GazelleHullSource::from_bytes(&support, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut membership = MEMBERSHIP.to_vec();
    membership[0] ^= 1;
    assert_eq!(
        GazelleHullSource::from_bytes(SUPPORT, &membership),
        Err(EnvelopeError::ChangedSource)
    );
    membership[0] = u8::MAX;
    assert_eq!(
        calibrated_pieces::<GAZELLE_PIECES>(
            SUPPORT,
            &membership,
            b"MSH019S1",
            GAZELLE_DETAILED_SUPPORT_POINTS,
            GAZELLE_NATIVE_SUPPORT_POINTS
        ),
        Err(EnvelopeError::InvalidContent)
    );
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
fn sh019_preserves_three_service_loop_corridors_and_detects_original_contact()
-> Result<(), EnvelopeError> {
    let source = GazelleHullSource::bundled()?;
    let start = pose(0.0, 0.0, false);
    let end = pose(0.0, 8.0, false);
    for (x, y) in [(0.0, 4.1), (-3.0, -3.7), (3.0, -3.7)] {
        let probe = [
            Vec3::new(x - 0.12, y, -7.0),
            Vec3::new(x + 0.12, y, -7.0),
            Vec3::new(x, y + 0.18, -7.0),
        ];
        assert!(source.sweep_clears_triangles(start, end, 0.0, &[probe])?);
    }
    for index in [676, 772, 1060, 1444, 1844, 2862] {
        let p = source_vertex(CANONICAL, index)?;
        let contact = [
            p,
            Vec3::new(p.x + 0.003, p.y, p.z),
            Vec3::new(p.x, p.y + 0.003, p.z),
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
fn sh019_station_uses_original_fit_speed_attitude_and_swept_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let source = GazelleHullSource::bundled()?;
    let station = crate::OriginalStationData::bundled()?;
    let z = station.port().center().z;
    let before = pose(0.0, z + 0.5, true);
    let after = pose(0.0, z - 0.5, true);
    assert!(source.fits_port(station.port(), before, station.station_pose(), 0.0)?);
    assert_eq!(
        source.fits_port(station.port(), before, station.station_pose(), f64::NAN),
        Err(EnvelopeError::InvalidGeometry)
    );
    assert!(station.gazelle_motion_clear(&source, before, after, 0.0));
    assert!(station.gazelle_entry_clear(&source, before, after, 72.0, 0.0, 0.0));
    assert!(!station.gazelle_entry_clear(&source, before, after, 144.0, 0.0, 0.0));
    assert!(!station.gazelle_entry_clear(&source, before, after, 72.0, 0.1, 0.0));
    assert!(!station.gazelle_entry_clear(
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
    assert!(station.gazelle_motion_clear(&source, off_before, off_after, 0.0));
    assert!(!station.gazelle_entry_clear(&source, off_before, off_after, 72.0, 0.0, 0.0));
    let forward = source_vertex(CANONICAL, 2862)?;
    let x = 69.4 + forward.x;
    let contact_z = z + forward.z;
    let start = pose(x, contact_z + 1.0, true);
    let penetration = pose(x, contact_z - 1.0, true);
    assert!(station.depth(penetration) > 0.0);
    assert!(station.gazelle_motion_clear(&source, start, start, 0.0));
    assert!(!station.gazelle_motion_clear(&source, start, penetration, 0.0));
    let launch = station.gazelle_launch_start(&source)?;
    assert_eq!(launch.position, [0.0, 0.0, f32::from_bits(0x4490_2429)]);
    let pose = crate::station::flight_pose(launch.position, launch.orientation)
        .ok_or(EnvelopeError::InvalidGeometry)?;
    let end = RigidPose {
        position: Vec3::new(pose.position.x, pose.position.y, pose.position.z + 2.4),
        ..pose
    };
    assert!(station.gazelle_motion_clear(&source, pose, end, 0.0));
    Ok(())
}

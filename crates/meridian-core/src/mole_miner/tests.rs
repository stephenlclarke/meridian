use super::{MEMBERSHIP, MoleMinerHullSource, VERTEX_COUNT, VERTICES, decode_point, groups};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh016/source-triangles.bin");

#[test]
fn mole_miner_original_faces_belong_to_complete_local_supports() -> Result<(), EnvelopeError> {
    let hull = MoleMinerHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    let mut owners = vec![Vec::new(); VERTEX_COUNT];
    for (slot, ids) in groups(MEMBERSHIP)?.into_iter().enumerate() {
        let piece = &hull.pieces()[slot];
        assert_eq!(piece.rounding_metres(), 0.02);
        for (index, point) in ids.into_iter().zip(piece.source_points()) {
            assert_eq!(*point, decode_point(&points[index]));
            owners[index].push(slot);
        }
    }
    assert!(owners.iter().all(|slots| !slots.is_empty()));
    assert_eq!(TRIANGLES.len(), (12_836 + 1100) * 12);
    for encoded in TRIANGLES.as_chunks::<12>().0 {
        let [a, b, c] = encoded.as_chunks::<4>().0 else {
            return Err(EnvelopeError::InvalidContent);
        };
        let [a, b, c] = [*a, *b, *c].map(|word| u32::from_le_bytes(word) as usize);
        assert!(
            owners[a]
                .iter()
                .any(|slot| owners[b].contains(slot) && owners[c].contains(slot))
        );
    }
    assert!(owners.iter().any(|slots| slots.len() > 1));

    let mut changed = VERTICES.to_vec();
    changed[16] ^= 1;
    assert_eq!(
        MoleMinerHullSource::from_bytes(&changed, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        MoleMinerHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut membership = MEMBERSHIP.to_vec();
    membership[4] ^= 1;
    assert_eq!(
        MoleMinerHullSource::from_bytes(VERTICES, &membership),
        Err(EnvelopeError::ChangedSource)
    );
    Ok(())
}

fn translated(z: f64) -> RigidPose {
    RigidPose {
        position: Vec3::new(0.0, 0.0, z),
        orientation: Basis3::IDENTITY,
    }
}

#[test]
fn mole_miner_tooling_aperture_collar_clearances_and_solid_heads() -> Result<(), EnvelopeError> {
    let hull = MoleMinerHullSource::bundled()?;
    let aperture = [
        Vec3::new(-0.1, -5.1, 18.5),
        Vec3::new(0.1, -5.1, 18.5),
        Vec3::new(0.0, -4.9, 18.5),
    ];
    assert!(hull.sweep_clears_triangle(translated(0.0), translated(8.0), 0.0, aperture)?);
    for x in [-10.300944804206456, 5.969797555793544] {
        let y = -4.24489933546377;
        let z = 13.4215575;
        let collar = [
            Vec3::new(x, y, z),
            Vec3::new(x + 0.001, y, z),
            Vec3::new(x, y + 0.001, z),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(0.06), 0.0, collar)?);
    }
    for (x, end) in [(-5.43, -5.39), (5.39, 5.43)] {
        let groove = [
            Vec3::new(x, -3.39, 14.29),
            Vec3::new(x, -3.33, 14.29),
            Vec3::new(end, -3.36, 14.29),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(0.30), 0.0, groove)?);
    }
    for index in [1760, 4096] {
        let contact = decode_point(&VERTICES[16..].as_chunks::<24>().0[index]);
        let tri = [
            contact,
            Vec3::new(contact.x + 0.001, contact.y, contact.z),
            Vec3::new(contact.x, contact.y + 0.001, contact.z),
        ];
        assert!(!hull.sweep_clears_triangle(translated(0.0), translated(0.0), 0.0, tri)?);
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
fn mole_miner_original_station_aligned_motion_and_offset_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let ship = MoleMinerHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    let x = 61.565938864628826;
    let before = inward(x, 159.75085880640466);
    let after = inward(x, 157.75085880640466);
    assert!(station.mole_miner_motion_clear(&ship, inward(0.0, 137.32), inward(0.0, 135.32), 0.0));
    assert!(station.mole_miner_motion_clear(&ship, before, before, 0.0));
    assert!(!station.mole_miner_motion_clear(&ship, before, after, 0.0));
    Ok(())
}

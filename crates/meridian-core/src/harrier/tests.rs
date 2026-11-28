use super::{HarrierHullSource, MEMBERSHIP, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh026/source-triangles.bin");

#[test]
fn harrier_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = HarrierHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 180);
    assert_eq!(points.len(), 6070);
    assert_eq!(TRIANGLES.len(), (8884 + 512) * 12);
    for (index, slot) in MEMBERSHIP.iter().copied().enumerate() {
        let point = decode_point(&points[index]);
        let piece = &hull.pieces()[usize::from(slot)];
        assert!(piece.source_points().contains(&point));
        assert_eq!(piece.rounding_metres(), 0.02);
    }
    for face in TRIANGLES.as_chunks::<12>().0 {
        let [a, b, c] = face.as_chunks::<4>().0 else {
            return Err(EnvelopeError::InvalidContent);
        };
        let indices = [*a, *b, *c].map(|i| u32::from_le_bytes(i) as usize);
        let slot = MEMBERSHIP[indices[0]];
        assert!(indices.iter().all(|&i| MEMBERSHIP[i] == slot));
        let [a, b, c] = indices.map(|i| decode_point(&points[i]));
        let center = Vec3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        assert!(hull.pieces()[usize::from(slot)].contains(center));
    }
    let mut rewritten = VERTICES.to_vec();
    rewritten[16] ^= 1;
    assert_eq!(
        HarrierHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        HarrierHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        HarrierHullSource::from_bytes(VERTICES, &shifted),
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
fn harrier_both_pod_channels_and_empty_rail_runs_clear() -> Result<(), EnvelopeError> {
    let hull = HarrierHullSource::bundled()?;
    for x in [-4.72, 4.72] {
        let triangle = [
            Vec3::new(x - 0.02, -2.275, -0.45),
            Vec3::new(x + 0.02, -2.275, -0.45),
            Vec3::new(x, -2.225, -0.45),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(5.35), 0.0, triangle)?);
    }
    for x in [-6.2977, 6.2977] {
        let triangle = [
            Vec3::new(x - 0.045, -4.26, 12.65),
            Vec3::new(x + 0.045, -4.26, 12.65),
            Vec3::new(x, -4.225, 12.65),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(4.65), 0.0, triangle)?);
    }
    for index in [5588, 5620] {
        let vertex = decode_point(&VERTICES[16..].as_chunks::<24>().0[index]);
        let triangle = [
            vertex,
            Vec3::new(vertex.x + 0.001, vertex.y, vertex.z),
            Vec3::new(vertex.x, vertex.y + 0.001, vertex.z),
        ];
        assert!(!hull.sweep_clears_triangle(translated(0.0), translated(0.0), 0.0, triangle)?);
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
fn harrier_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = HarrierHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.harrier_motion_clear(&hull, inward(0.0, 137.32), inward(0.0, 135.32), 0.0));
    let before = inward(59.38091603053436, 152.49175572519084);
    let after = inward(59.38091603053436, 150.49175572519084);
    assert!(station.harrier_motion_clear(&hull, before, before, 0.0));
    assert!(!station.harrier_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

#[test]
fn harrier_stock_routes_clear_pod_and_empty_rail_channels_and_refuse_solid_rails()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::hull_package::{Clearance, HullPackage, validate_package};

    let mut package = HullPackage::from_json(include_bytes!(
        "../../../../integration/sh026-harrier/HARRIER-STUDIO.json"
    ))?;
    let original = HarrierHullSource::bundled()?;
    assert_eq!(package.pieces()?.as_slice(), original.pieces().as_slice());
    let clear = validate_package(&package);
    assert!(clear.valid && clear.clearances.iter().all(|route| route.passed));
    assert_eq!(clear.clearances.len(), 4);
    let evidence: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../integration/sh026-harrier/FIXTURES.json"
    ))?;
    let indices: Vec<usize> = serde_json::from_value(evidence["sourceRailContacts"].clone())?;
    assert_eq!(indices.len(), 2);
    for index in indices {
        let start = package.detail.vertices[index];
        let mut end = start;
        end[1] += 0.01;
        package.clearances.push(Clearance {
            name: format!("solid-rail-{index}"),
            start,
            end,
            radius_metres: 0.02,
        });
    }
    let blocked = validate_package(&package);
    assert!(!blocked.valid);
    assert_eq!(blocked.support_groups, clear.support_groups);
    assert_eq!(blocked.clearances.len(), 6);
    assert_eq!(blocked.clearances[..4], clear.clearances);
    assert!(blocked.clearances[4..].iter().all(|route| !route.passed));
    Ok(())
}

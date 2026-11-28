use super::{MEMBERSHIP, OkapiHullSource, VERTICES, decode_point};
use crate::OriginalStationData;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::EnvelopeError;

const TRIANGLES: &[u8] = include_bytes!("../../../../assets/fleet/sh029/source-triangles.bin");

#[test]
fn okapi_original_detailed_and_native_faces_share_complete_local_supports()
-> Result<(), EnvelopeError> {
    let hull = OkapiHullSource::bundled()?;
    let points = VERTICES[16..].as_chunks::<24>().0;
    assert_eq!(hull.pieces().len(), 26);
    assert_eq!(points.len(), 1324);
    assert_eq!(TRIANGLES.len(), (2376 + 120) * 12);
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
        OkapiHullSource::from_bytes(&rewritten, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let explorer = include_bytes!("../../../../assets/fleet/sh008/source.vertices");
    assert_eq!(
        OkapiHullSource::from_bytes(explorer, MEMBERSHIP),
        Err(EnvelopeError::ChangedSource)
    );
    let mut shifted = MEMBERSHIP.to_vec();
    shifted[0] ^= 1;
    assert_eq!(
        OkapiHullSource::from_bytes(VERTICES, &shifted),
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
fn okapi_pallet_and_pipe_lanes_and_rail_gaps_clear_but_original_rails_contact()
-> Result<(), EnvelopeError> {
    let hull = OkapiHullSource::bundled()?;
    for x in [-3.0, 3.0] {
        let pallet = [
            Vec3::new(x - 0.1, -0.4, 3.3),
            Vec3::new(x + 0.1, -0.4, 3.3),
            Vec3::new(x, -0.2, 3.3),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(16.6), 0.0, pallet)?);
    }
    for x in [-3.51, 3.51] {
        let pipe = [
            Vec3::new(x - 0.04, 2.1, -3.62),
            Vec3::new(x + 0.04, 2.1, -3.62),
            Vec3::new(x, 2.15, -3.62),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(3.9), 0.0, pipe)?);
    }
    for x in [-6.0, 0.0, 6.0] {
        let rail = [
            Vec3::new(x - 0.03, 1.75, 0.6),
            Vec3::new(x + 0.03, 1.75, 0.6),
            Vec3::new(x, 1.75, 0.54),
        ];
        assert!(hull.sweep_clears_triangle(translated(0.0), translated(10.0), 0.0, rail)?);
    }
    for index in [210, 226, 242] {
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
fn okapi_original_sh028_aligned_crossing_clears_before_offset_native_lip_contact()
-> Result<(), Box<dyn std::error::Error>> {
    let hull = OkapiHullSource::bundled()?;
    let station = OriginalStationData::bundled()?;
    assert!(station.okapi_motion_clear(&hull, inward(0.0, 137.32), inward(0.0, 135.32), 0.0));
    let before = inward(67.90400000000001, 153.32);
    let after = inward(67.90400000000001, 151.32);
    assert!(station.okapi_motion_clear(&hull, before, before, 0.0));
    assert!(!station.okapi_motion_clear(&hull, before, after, 0.0));
    Ok(())
}

#[test]
fn okapi_stock_routes_clear_the_gaps_and_refuse_all_three_original_rails()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::hull_package::{Clearance, HullPackage, validate_package};

    let mut package = HullPackage::from_json(include_bytes!(
        "../../../../integration/sh029-okapi/OKAPI-STUDIO.json"
    ))?;
    let original = OkapiHullSource::bundled()?;
    assert_eq!(package.pieces()?.as_slice(), original.pieces().as_slice());
    let clear = validate_package(&package);
    assert!(clear.valid && clear.clearances.iter().all(|route| route.passed));
    assert_eq!(clear.clearances.len(), 7);
    let mut single = package.clone();
    single.detail.groups.fill(0);
    single.native.groups.fill(0);
    let combined = validate_package(&single);
    assert_eq!(combined.support_groups.len(), 1);
    assert!(!combined.valid);
    assert_eq!(combined.clearances.len(), 7);
    assert!(combined.clearances.iter().all(|route| !route.passed));
    let evidence: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../integration/sh029-okapi/FIXTURES.json"
    ))?;
    let indices: Vec<usize> = serde_json::from_value(evidence["sourceRailContacts"].clone())?;
    assert_eq!(indices.len(), 3);
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
    assert_eq!(blocked.clearances.len(), 10);
    assert_eq!(blocked.clearances[..7], clear.clearances);
    assert!(blocked.clearances[7..].iter().all(|route| !route.passed));
    Ok(())
}

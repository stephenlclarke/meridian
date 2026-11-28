//! The stock capsule uses the single validated rounded vessel for each station sweep.
use super::*;
use crate::hull_package::{Clearance, HullPackage, validate_package};

const SOURCE: &[u8] =
    include_bytes!("../../../../../integration/sh040-newt-escape-pod/NEWT-STUDIO.json");
const ORIGINAL_TENSOR: &[u8] =
    include_bytes!("../../../../../integration/sh040-newt-escape-pod/NEWT-ORIGINAL-TENSOR.json");

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
fn newt_closed_vessel_clears_the_aligned_station_and_hits_the_offset_lip() {
    let capsule = HullPackage::from_json(SOURCE).unwrap_or_else(|e| panic!("{e}"));
    let pieces = capsule.pieces().unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].source_points().len(), 304);
    let station = OriginalStationData::bundled().unwrap_or_else(|e| panic!("{e}"));
    assert!(station.rounded_station_motion_clear(
        &pieces,
        inward(0., 137.32),
        inward(0., 135.32),
        0.
    ));
    let x = 69.95473684210526;
    let before = inward(x, 138.51999999999998);
    assert!(station.rounded_station_motion_clear(&pieces, before, before, 0.));
    assert!(!station.rounded_station_motion_clear(
        &pieces,
        before,
        inward(x, 136.51999999999998),
        0.
    ));
}

#[test]
fn newt_validator_preserves_four_exterior_routes_and_refuses_closed_sides_and_raw_tensor() {
    let raw = HullPackage::from_json(ORIGINAL_TENSOR).unwrap_or_else(|e| panic!("{e}"));
    assert!(!validate_package(&raw).valid);
    let mut capsule = HullPackage::from_json(SOURCE).unwrap_or_else(|e| panic!("{e}"));
    let good = validate_package(&capsule);
    assert!(good.valid && good.diagnostics.is_empty());
    assert_eq!(good.clearances.len(), 4);
    assert!(good.clearances.iter().all(|r| r.passed));
    let receipt: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../integration/sh040-newt-escape-pod/CONTACT-RESULT.json"
    ))
    .unwrap_or_else(|e| panic!("{e}"));
    let solids: Vec<Clearance> =
        serde_json::from_value(receipt["solidRoutes"].clone()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(solids.len(), 4);
    for route in solids {
        capsule.clearances = vec![route];
        let report = validate_package(&capsule);
        assert!(!report.valid);
        assert_eq!(report.clearances.len(), 1);
        assert_eq!(report.clearances[0].name, capsule.clearances[0].name);
        assert!(!report.clearances[0].passed);
        assert_eq!(report.support_groups, good.support_groups);
    }
}

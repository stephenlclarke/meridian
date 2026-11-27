//! Use the stock package's production collision pieces against the original station.
use super::*;
use crate::hull_package::HullPackage;

const PACKAGE: &[u8] =
    include_bytes!("../../../../../integration/sh041-serval-fighter/SERVAL-STUDIO.json");
const MEMBERSHIP: &[u8] =
    include_bytes!("../../../../../assets/fleet/sh041/source-piece-membership.bin");

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
fn serval_stock_pieces_preserve_both_meshes_and_the_sh028_lip_crossing() {
    let package = HullPackage::from_json(PACKAGE).unwrap_or_else(|e| panic!("Serval package: {e}"));
    let pieces = package
        .pieces()
        .unwrap_or_else(|e| panic!("Serval collision: {e:?}"));
    assert_eq!(pieces.len(), 23);
    let points = package
        .detail
        .vertices
        .iter()
        .chain(&package.native.vertices);
    assert_eq!(points.clone().count(), MEMBERSHIP.len());
    for (point, &slot) in points.zip(MEMBERSHIP) {
        assert!(
            pieces[usize::from(slot)]
                .source_points()
                .contains(&Vec3::new(point[0], point[1], point[2]))
        );
    }

    let station = OriginalStationData::bundled().unwrap_or_else(|e| panic!("SH028: {e}"));
    assert!(station.rounded_station_motion_clear(
        &pieces,
        inward(0.0, 150.0),
        inward(0.0, 116.0),
        0.0
    ));
    let offset_start = inward(69.9, 150.0);
    assert!(station.rounded_station_motion_clear(&pieces, offset_start, offset_start, 0.0));
    assert!(!station.rounded_station_motion_clear(&pieces, offset_start, inward(69.9, 147.0), 0.0));
}

#[test]
fn serval_runtime_passes_all_three_channels_and_refuses_all_three_solids() {
    use crate::hull_package::{Clearance, validate_package};

    let mut package = HullPackage::from_json(PACKAGE).unwrap_or_else(|e| panic!("{e}"));
    let positive = validate_package(&package);
    assert!(positive.valid);
    assert_eq!(positive.clearances.len(), 3);
    assert!(positive.clearances.iter().all(|row| row.passed));

    let evidence: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../integration/sh041-serval-fighter/CONTACT-WITNESSES.json"
    ))
    .unwrap_or_else(|e| panic!("{e}"));
    let contacts = evidence["contacts"]
        .as_array()
        .unwrap_or_else(|| panic!("missing contacts"));
    assert_eq!(contacts.len(), 3);
    for row in contacts {
        let route: Clearance =
            serde_json::from_value(row["route"].clone()).unwrap_or_else(|e| panic!("{e}"));
        package.clearances.push(route);
    }
    let blocked = validate_package(&package);
    assert!(!blocked.valid);
    assert_eq!(blocked.support_groups, positive.support_groups);
    assert_eq!(blocked.clearances.len(), 6);
    assert_eq!(blocked.clearances[..3], positive.clearances);
    for (actual, expected) in blocked.clearances[3..].iter().zip(&package.clearances[3..]) {
        assert_eq!(actual.name, expected.name);
        assert!(!actual.passed);
    }
    assert!(package.pieces().is_err());
}

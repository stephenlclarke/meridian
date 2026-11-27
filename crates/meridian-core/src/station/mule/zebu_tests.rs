//! Use the stock package's production collision pieces against the original station.
use super::*;
use crate::hull_package::HullPackage;

const PACKAGE: &[u8] = include_bytes!("../../../../../integration/sh036-zebu/ZEBU-STUDIO.json");
const MEMBERSHIP: &[u8] =
    include_bytes!("../../../../../assets/fleet/sh036/source-piece-membership.bin");

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
fn zebu_stock_pieces_preserve_both_meshes_and_the_sh028_lip_crossing() {
    let package = HullPackage::from_json(PACKAGE).unwrap_or_else(|e| panic!("Zebu package: {e}"));
    let pieces = package
        .pieces()
        .unwrap_or_else(|e| panic!("Zebu collision: {e:?}"));
    assert_eq!(pieces.len(), 24);
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
        inward(0.0, 163.0),
        inward(0.0, 112.0),
        0.0
    ));
    let offset_start = inward(69.9, 163.0);
    assert!(station.rounded_station_motion_clear(&pieces, offset_start, offset_start, 0.0));
    assert!(!station.rounded_station_motion_clear(&pieces, offset_start, inward(69.9, 160.0), 0.0));
}

#[test]
fn zebu_runtime_passes_all_three_channels_and_refuses_all_three_solids() {
    use crate::hull_package::{Clearance, validate_package};

    let mut package = HullPackage::from_json(PACKAGE).unwrap_or_else(|e| panic!("{e}"));
    let positive = validate_package(&package);
    assert!(positive.valid);
    assert_eq!(positive.clearances.len(), 3);
    assert!(positive.clearances.iter().all(|row| row.passed));

    let evidence: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../integration/sh036-zebu/CONTACT-WITNESSES.json"
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

#[test]
fn zebu_one_convex_group_blocks_the_original_passages() {
    use crate::hull_package::validate_package;

    let mut package = HullPackage::from_json(PACKAGE).unwrap_or_else(|e| panic!("{e}"));
    package.detail.groups.fill(0);
    package.native.groups.fill(0);
    let report = validate_package(&package);
    assert_eq!(report.support_groups.len(), 1);
    assert!(!report.valid);
    assert_eq!(report.clearances.len(), 3);
    assert!(report.clearances.iter().all(|route| !route.passed));
}

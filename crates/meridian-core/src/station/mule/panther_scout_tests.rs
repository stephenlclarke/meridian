//! The archived Scout and the station both use their shipped production collision path.
use super::*;
use crate::hull_package::{Clearance, HullPackage, validate_package};

fn scout() -> HullPackage {
    HullPackage::from_json(include_bytes!(
        "../../../../../integration/sh033-panther-scout/PANTHER-SCOUT-STUDIO.json"
    ))
    .unwrap_or_else(|e| panic!("Scout package: {e}"))
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
fn panther_scout_uses_its_arch_groups_through_the_original_station_lip() {
    let package = scout();
    let pieces = package
        .pieces()
        .unwrap_or_else(|e| panic!("Scout pieces: {e:?}"));
    assert_eq!(pieces.len(), 35);
    for mesh in [&package.detail, &package.native] {
        for (point, &slot) in mesh.vertices.iter().zip(&mesh.groups) {
            assert!(
                pieces[slot as usize]
                    .source_points()
                    .contains(&Vec3::new(point[0], point[1], point[2]))
            );
        }
    }
    let station = OriginalStationData::bundled().unwrap_or_else(|e| panic!("SH028: {e}"));
    assert!(station.rounded_station_motion_clear(
        &pieces,
        inward(0.0, 156.0),
        inward(0.0, 110.0),
        0.0
    ));
    let offset = inward(69.5, 154.32);
    assert!(station.rounded_station_motion_clear(&pieces, offset, offset, 0.0));
    assert!(!station.rounded_station_motion_clear(&pieces, offset, inward(69.5, 152.32), 0.0));
}

#[test]
fn panther_scout_keeps_four_openings_and_refuses_each_solid() {
    let mut package = scout();
    let pass = validate_package(&package);
    assert!(pass.valid);
    assert_eq!(pass.clearances.len(), 4);
    assert!(pass.clearances.iter().all(|r| r.passed));
    let fixture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../integration/sh033-panther-scout/CLEARANCE-FIXTURES.json"
    ))
    .unwrap_or_else(|e| panic!("{e}"));
    let solids = fixture["solidRoutes"]
        .as_array()
        .unwrap_or_else(|| panic!("Missing solids"));
    assert_eq!(solids.len(), 4);
    for row in solids {
        let probe: Clearance = serde_json::from_value(serde_json::json!({
            "name": row["name"], "start": row["start"], "end": row["end"], "radiusMetres": row["radiusMetres"]
        })).unwrap_or_else(|e| panic!("{e}"));
        package.clearances = vec![probe];
        let report = validate_package(&package);
        assert!(!report.valid);
        assert_eq!(report.support_groups, pass.support_groups);
        assert_eq!(report.clearances.len(), 1);
        assert_eq!(report.clearances[0].name, package.clearances[0].name);
        assert!(!report.clearances[0].passed);
    }
}

#[test]
fn panther_scout_whole_hull_control_closes_all_four_passages() {
    let mut package = scout();
    package.detail.groups.fill(0);
    package.native.groups.fill(0);
    let report = validate_package(&package);
    assert_eq!(report.support_groups.len(), 1);
    assert!(!report.valid);
    assert_eq!(report.clearances.len(), 4);
    assert!(report.clearances.iter().all(|route| !route.passed));
}

mod common;

use std::fs;

use common::{OwnedDirectory, TestResult, arguments_with_files};
use meridian_core::{OriginalStationData, StationError};

#[test]
fn current_core_accepts_actual_cli_outputs_and_refuses_each_corruption() -> TestResult {
    let root = OwnedDirectory::new()?;
    let directory = root.path().join("current consumer \u{e9} output");
    let output = arguments_with_files("station", "--output", &directory)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let station = fs::read(directory.join("sh028.physics"))?;
    let hull = fs::read(directory.join("sh025.physics"))?;
    let port = fs::read(directory.join("sh028.port"))?;
    assert_eq!(
        station,
        fs::read(common::reference().join("sh028.physics"))?
    );
    let actual = OriginalStationData::from_bytes(&station, &hull, &port)?;
    let bundled = OriginalStationData::bundled()?;
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/current-consumers.json"))?;
    assert_eq!(
        Some(actual.station_triangle_count() as u64),
        fixture["physical"]["sh028.physics"]["triangles"].as_u64()
    );
    assert_eq!(
        Some(actual.hull_triangle_count() as u64),
        fixture["physical"]["sh025.physics"]["triangles"].as_u64()
    );
    assert_eq!(actual.launch_start()?, bundled.launch_start()?);
    assert_eq!(
        meridian_core::station::SOURCE_REVISION,
        meridian_import::REVISION
    );

    // Exercise the real public loader's content boundary, with all other bytes intact.
    for changed in 0..3 {
        let mut inputs = [station.clone(), hull.clone(), port.clone()];
        inputs[changed][0] ^= 1;
        assert!(matches!(
            OriginalStationData::from_bytes(&inputs[0], &inputs[1], &inputs[2]),
            Err(StationError::ContentMismatch)
        ));
    }
    assert_eq!(fs::read(directory.join("sh028.physics"))?, station);
    assert_eq!(fs::read(directory.join("sh025.physics"))?, hull);
    assert_eq!(fs::read(directory.join("sh028.port"))?, port);
    Ok(())
}

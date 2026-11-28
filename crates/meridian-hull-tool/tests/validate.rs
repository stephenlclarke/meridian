use serde_json::{Value, json};
use std::{
    fs, io,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct Directory(PathBuf);
impl Directory {
    fn new() -> io::Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "meridian-hull-tool-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> Value {
    let mesh = json!({"vertices":[[0,0,0],[1,0,0],[0,1,0],[0,0,1]],"triangles":[[0,1,2],[0,3,1],[0,2,3],[1,3,2]],"groups":[47,47,47,47]});
    json!({"schema":1,"id":"custom.42","name":"Custom user hull","metresPerUnit":2,"roundingMetres":0.02,
        "detail":mesh,"native":mesh,"materials":[{"name":"hull","rgb":[50,60,70]}],"faceMaterials":[0,0,0,0],
        "engineering":null,
        "flight":{"maxSpeed":null},"clearances":[{"name":"clear","start":[-1,-1,0],"end":[3,-1,0],"radiusMetres":0.2}]})
}

fn invoke(
    directory: &Directory,
    input: &[u8],
) -> Result<(Option<i32>, Value), Box<dyn std::error::Error>> {
    let source = directory.0.join("source.json");
    let output = directory.0.join("result.json");
    fs::write(&source, input)?;
    let status = Command::new(env!("CARGO_BIN_EXE_meridian-hull-tool"))
        .arg("validate")
        .arg(&source)
        .arg("--report")
        .arg(&output)
        .output()?
        .status;
    let report = serde_json::from_slice(&fs::read(output)?)?;
    assert_eq!(fs::read(source)?, input);
    Ok((status.code(), report))
}

#[test]
fn accepts_arbitrary_custom_package_with_unknown_speed_and_scaled_supports()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = Directory::new()?;
    let (code, report) = invoke(&directory, &serde_json::to_vec(&fixture())?)?;
    assert_eq!(code, Some(0));
    assert_eq!(report["valid"], true);
    assert_eq!(report["clearances"][0]["passed"], true);
    assert_eq!(report["supportGroups"][0]["id"], 47);
    assert_eq!(
        report["supportGroups"][0]["points"][1],
        json!([2.0, 0.0, 0.0])
    );
    assert_eq!(
        report["supportGroups"][0]["points"]
            .as_array()
            .map(Vec::len),
        Some(8)
    );
    Ok(())
}

#[test]
fn reports_invalid_blocked_and_malformed_packages() -> Result<(), Box<dyn std::error::Error>> {
    let directory = Directory::new()?;
    let mut source = fixture();
    source["clearances"][0]["start"] = json!([-1, 0.5, 0.5]);
    source["clearances"][0]["end"] = json!([3, 0.5, 0.5]);
    let (code, report) = invoke(&directory, &serde_json::to_vec(&source)?)?;
    assert_eq!(code, Some(1));
    assert_eq!(report["valid"], false);
    assert_eq!(report["clearances"][0]["passed"], false);
    assert!(
        report["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|text| text.contains("blocked or indeterminate"))
    );
    let (code, report) = invoke(&Directory::new()?, b"{invalid")?;
    assert_eq!(code, Some(1));
    assert_eq!(report["valid"], false);
    assert!(
        report["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|text| text.contains("Invalid hull package JSON"))
    );
    let (code, report) = invoke(&Directory::new()?, b"{\"schema\":1,\"schema\":1}")?;
    assert_eq!(code, Some(1));
    assert_eq!(report["valid"], false);
    Ok(())
}

#[test]
fn refuses_overwriting_inputs_and_existing_reports() -> Result<(), Box<dyn std::error::Error>> {
    let directory = Directory::new()?;
    let source = directory.0.join("same.json");
    let bytes = serde_json::to_vec(&fixture())?;
    fs::write(&source, &bytes)?;
    let result = Command::new(env!("CARGO_BIN_EXE_meridian-hull-tool"))
        .arg("validate")
        .arg(&source)
        .arg("--report")
        .arg(&source)
        .output()?;
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(fs::read(&source)?, bytes);
    let result = Command::new(env!("CARGO_BIN_EXE_meridian-hull-tool"))
        .arg("validate")
        .output()?;
    assert_eq!(result.status.code(), Some(2));
    Ok(())
}

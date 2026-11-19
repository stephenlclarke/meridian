//! Command-line boundary for offline hull authoring. Never writes runtime assets or saves.
use meridian_core::hull_package::{
    HullPackage, MAX_PACKAGE_JSON_BYTES, ValidationReport, validate_package,
};
use std::{
    env,
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const HELP: &str = "meridian-hull-tool validate PACKAGE --report NEW_REPORT\nValidate a self-contained schema-1 hull package. Exit 0 = valid, 1 = invalid package, 2 = usage/I/O error. Existing reports are not overwritten.";

fn read_package(path: &Path) -> Result<ValidationReport, String> {
    let file = File::open(path).map_err(|error| format!("Read package: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("Package metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("Package must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_PACKAGE_JSON_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Read package: {error}"))?;
    if bytes.len() as u64 > MAX_PACKAGE_JSON_BYTES {
        return Ok(ValidationReport::invalid(
            "Package exceeds the 64 MiB input limit",
        ));
    }
    Ok(match HullPackage::from_json(&bytes) {
        Ok(package) => validate_package(&package),
        Err(error) => ValidationReport::invalid(format!("Invalid hull package JSON: {error}")),
    })
}

fn write_report(path: &Path, report: &ValidationReport) -> Result<(), String> {
    let encoded =
        serde_json::to_vec_pretty(report).map_err(|error| format!("Encode report: {error}"))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("Create new report: {error}"))?;
    output
        .write_all(&encoded)
        .and_then(|()| output.write_all(b"\n"))
        .and_then(|()| output.sync_all())
        .map_err(|error| format!("Write report: {error}"))
}

fn run(args: Vec<OsString>) -> Result<u8, String> {
    if args.len() == 1 && matches!(args[0].to_str(), Some("--help" | "-h")) {
        println!("{HELP}");
        return Ok(0);
    }
    let [command, package, flag, output] = args.as_slice() else {
        return Err(HELP.into());
    };
    if command != "validate" || flag != "--report" || package.is_empty() || output.is_empty() {
        return Err(HELP.into());
    }
    let report = read_package(&PathBuf::from(package))?;
    write_report(&PathBuf::from(output), &report)?;
    if report.valid {
        Ok(0)
    } else {
        for diagnostic in &report.diagnostics {
            eprintln!("{}", diagnostic.message);
        }
        Ok(1)
    }
}

fn main() -> ExitCode {
    match run(env::args_os().skip(1).collect()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

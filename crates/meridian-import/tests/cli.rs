mod common;

use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

use common::{OwnedDirectory, TestResult, arguments_with_files, cli, fixture, reference};
use meridian_import::sha256;

#[test]
fn actual_binary_creates_all_original_bytes_and_a_new_rust_receipt() -> TestResult {
    let root = OwnedDirectory::new()?;
    let destination = root.path().join("new output with spaces");
    let output = arguments_with_files("all", "--output", &destination)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(destination.join("rust-import-manifest.json"))?)?;
    assert_eq!(json["producer"]["name"], "meridian-import");
    assert!(json.get("importer").is_none());
    assert_eq!(fs::read_dir(&destination)?.count(), 7);
    for name in [
        "sh025.mesh",
        "sh028.mesh",
        "sh025.physics",
        "sh028.physics",
        "sh028.port",
    ] {
        let actual = fs::read(destination.join(name))?;
        assert_eq!(actual, fs::read(reference().join(name))?, "{name}");
        assert!(json["artifacts"].as_array().is_some_and(|values| {
            values.iter().any(|r| {
                r["file"] == name && r["sha256"].as_str() == Some(sha256(&actual).as_str())
            })
        }));
    }
    assert_eq!(
        fs::read(destination.join("SOURCE-NOTICE.txt"))?,
        fs::read(reference().join("Elite-L337-MIT.txt"))?
    );
    assert!(!destination.join("rust-import-manifest.json.tmp").exists());
    Ok(())
}

#[test]
fn check_mode_verifies_the_existing_reference_without_writing() -> TestResult {
    let root = OwnedDirectory::new()?;
    let directory = root.path().join("reference");
    fs::create_dir(&directory)?;
    let names = [
        "sh025.mesh",
        "sh028.mesh",
        "sh025.physics",
        "sh028.physics",
        "sh028.port",
        "manifest.json",
        "station-manifest.json",
        "Elite-L337-MIT.txt",
    ];
    for name in names {
        fs::copy(reference().join(name), directory.join(name))?;
    }
    let before: Vec<_> = names
        .iter()
        .map(|n| fs::read(directory.join(n)))
        .collect::<Result<_, _>>()?;
    let output = arguments_with_files("all", "--check", &directory)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after: Vec<_> = names
        .iter()
        .map(|n| fs::read(directory.join(n)))
        .collect::<Result<_, _>>()?;
    assert_eq!(before, after);
    assert_eq!(fs::read_dir(&directory)?.count(), 8);
    Ok(())
}

#[test]
fn check_refuses_changed_binary_manifest_or_notice_without_repair() -> TestResult {
    let root = OwnedDirectory::new()?;
    for changed in [
        "sh025.mesh",
        "sh028.mesh",
        "sh025.physics",
        "sh028.physics",
        "sh028.port",
        "manifest.json",
        "station-manifest.json",
        "Elite-L337-MIT.txt",
    ] {
        let directory = root.path().join(changed);
        fs::create_dir(&directory)?;
        for name in [
            "sh025.mesh",
            "sh028.mesh",
            "sh025.physics",
            "sh028.physics",
            "sh028.port",
            "manifest.json",
            "station-manifest.json",
            "Elite-L337-MIT.txt",
        ] {
            fs::copy(reference().join(name), directory.join(name))?;
        }
        let path = directory.join(changed);
        let mut bytes = fs::read(&path)?;
        bytes[0] ^= 1;
        fs::write(&path, &bytes)?;
        let output = arguments_with_files("all", "--check", &directory)?;
        assert!(!output.status.success());
        assert_eq!(fs::read(&path)?, bytes);
        assert!(!directory.join("rust-import-manifest.json").exists());
    }
    Ok(())
}

#[test]
fn invalid_source_and_existing_destination_never_replace_data() -> TestResult {
    let root = OwnedDirectory::new()?;
    let bad = root.path().join("foreign.toml");
    fs::write(&bad, "id='SH025'\n")?;
    let absent = root.path().join("must remain absent");
    let output = cli(&[
        "ship".as_ref(),
        "--source".as_ref(),
        bad.as_os_str(),
        "--output".as_ref(),
        absent.as_os_str(),
    ])?;
    assert!(!output.status.success() && !absent.exists());
    let mut appended = fs::read(fixture("SH025.toml"))?;
    appended.push(b'\n');
    fs::write(&bad, appended)?;
    let output = cli(&[
        "ship".as_ref(),
        "--source".as_ref(),
        bad.as_os_str(),
        "--output".as_ref(),
        absent.as_os_str(),
    ])?;
    assert!(!output.status.success() && !absent.exists());
    let existing = root.path().join("existing");
    fs::create_dir(&existing)?;
    fs::write(existing.join("sentinel"), "unchanged")?;
    let output = arguments_with_files("all", "--output", &existing)?;
    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(existing.join("sentinel"))?, "unchanged");
    assert_eq!(fs::read_dir(existing)?.count(), 1);
    Ok(())
}

#[test]
fn fixed_git_lookup_uses_exact_original_objects_not_head_or_working_files() -> TestResult {
    let root = OwnedDirectory::new()?;
    let repo = root.path().join("--object database");
    fs::create_dir(&repo)?;
    let initialized = Command::new("git")
        .args(["init", "--quiet", "--object-format=sha1"])
        .current_dir(&repo)
        .output()?;
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture("git-fixture.json"))?)?;
    let objects = manifest["objects"]
        .as_array()
        .ok_or("missing Git object fixture")?;
    for object in objects {
        let kind = object["kind"].as_str().ok_or("missing object kind")?;
        let file = object["file"].as_str().ok_or("missing object file")?;
        let bytes = fs::read(fixture(file))?;
        assert_eq!(
            sha256(&bytes),
            object["sha256"].as_str().ok_or("missing SHA256")?
        );
        let mut command = Command::new("git")
            .args(["hash-object", "-w", "-t", kind, "--stdin"])
            .current_dir(&repo)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        command
            .stdin
            .take()
            .ok_or("missing Git stdin")?
            .write_all(&bytes)?;
        let output = command.wait_with_output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)?.trim(),
            object["sha1"].as_str().ok_or("missing original Git SHA")?
        );
    }
    let working = repo.join("Sources/L337Core/Resources/Ships");
    fs::create_dir_all(&working)?;
    fs::write(working.join("SH025.toml"), "foreign working-tree source")?;
    fs::write(working.join("SH028.toml"), "foreign working-tree source")?;
    assert!(
        !Command::new("git")
            .args(["rev-parse", "--verify", "HEAD"])
            .current_dir(&repo)
            .output()?
            .status
            .success()
    );
    let destination = root.path().join("git output");
    let output = cli(&[
        "all".as_ref(),
        "--repository".as_ref(),
        repo.as_os_str(),
        "--output".as_ref(),
        destination.as_os_str(),
    ])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(destination.join("sh028.mesh"))?,
        fs::read(reference().join("sh028.mesh"))?
    );
    assert_eq!(
        fs::read_to_string(working.join("SH025.toml"))?,
        "foreign working-tree source"
    );
    Ok(())
}

#[test]
fn actual_help_and_bad_arguments_do_not_need_or_create_a_source() -> TestResult {
    let help = cli(&["--help".as_ref()])?;
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout)?.contains("--repository"));
    for args in [
        vec!["SH029"],
        vec!["ship", "--source"],
        vec![
            "all",
            "--repository",
            "missing",
            "--id",
            "SH029",
            "--check",
            "missing",
        ],
    ] {
        let values: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
        assert!(!cli(&values)?.status.success());
    }
    Ok(())
}

#[test]
fn missing_inputs_empty_git_database_and_file_repository_refuse() -> TestResult {
    let root = OwnedDirectory::new()?;
    let absent = root.path().join("absent source");
    let destination = root.path().join("no output");
    let output = cli(&[
        "ship".as_ref(),
        "--source".as_ref(),
        absent.as_os_str(),
        "--output".as_ref(),
        destination.as_os_str(),
    ])?;
    assert!(!output.status.success() && !destination.exists());
    let file = root.path().join("not a directory");
    fs::write(&file, "unchanged")?;
    let output = cli(&[
        "ship".as_ref(),
        "--repository".as_ref(),
        file.as_os_str(),
        "--output".as_ref(),
        destination.as_os_str(),
    ])?;
    assert!(!output.status.success() && !destination.exists());
    let repo = root.path().join("empty repository");
    fs::create_dir(&repo)?;
    assert!(
        Command::new("git")
            .args(["init", "--quiet", "--object-format=sha1"])
            .current_dir(&repo)
            .output()?
            .status
            .success()
    );
    let output = cli(&[
        "ship".as_ref(),
        "--repository".as_ref(),
        repo.as_os_str(),
        "--output".as_ref(),
        destination.as_os_str(),
    ])?;
    assert!(!output.status.success() && !destination.exists());
    assert_eq!(fs::read_to_string(file)?, "unchanged");
    Ok(())
}

#[test]
fn missing_reference_and_missing_output_parent_refuse_without_inventing_directories() -> TestResult
{
    let root = OwnedDirectory::new()?;
    let empty = root.path().join("empty reference");
    fs::create_dir(&empty)?;
    assert!(
        !arguments_with_files("all", "--check", &empty)?
            .status
            .success()
    );
    assert_eq!(fs::read_dir(&empty)?.count(), 0);
    let missing_parent = root.path().join("absent parent");
    assert!(
        !arguments_with_files("all", "--output", &missing_parent.join("output"))?
            .status
            .success()
    );
    assert!(!missing_parent.exists());
    Ok(())
}

#[test]
fn actual_ship_and_station_modes_emit_only_their_selected_formats() -> TestResult {
    let root = OwnedDirectory::new()?;
    let ship_input = fixture("SH025.toml");
    let ship_output = root.path().join("ship");
    let output = cli(&[
        "ship".as_ref(),
        "--source".as_ref(),
        ship_input.as_os_str(),
        "--output".as_ref(),
        ship_output.as_os_str(),
    ])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_dir(&ship_output)?.count(), 3);
    assert_eq!(
        fs::read(ship_output.join("sh025.mesh"))?,
        fs::read(reference().join("sh025.mesh"))?
    );
    assert!(!ship_output.join("sh025.physics").exists());
    let output = cli(&[
        "ship".as_ref(),
        "--source".as_ref(),
        ship_input.as_os_str(),
        "--check".as_ref(),
        reference().as_os_str(),
    ])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let station_output = root.path().join("station");
    let output = arguments_with_files("station", "--output", &station_output)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_dir(&station_output)?.count(), 6);
    assert!(!station_output.join("sh025.mesh").exists());
    assert_eq!(
        fs::read(station_output.join("sh025.physics"))?,
        fs::read(reference().join("sh025.physics"))?
    );
    let output = arguments_with_files("station", "--check", &reference())?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

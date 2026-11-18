use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct OwnedDirectory(PathBuf);

impl OwnedDirectory {
    pub fn new() -> io::Result<Self> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "meridian-import-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

pub fn reference() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/legacy")
}

pub fn cli(args: &[&std::ffi::OsStr]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_meridian-import"))
        .args(args)
        .output()
}

pub fn arguments_with_files(mode: &str, option: &str, destination: &Path) -> io::Result<Output> {
    let ship = fixture("SH025.toml");
    let station = fixture("SH028.toml");
    cli(&[
        mode.as_ref(),
        "--ship-source".as_ref(),
        ship.as_os_str(),
        "--station-source".as_ref(),
        station.as_os_str(),
        option.as_ref(),
        destination.as_os_str(),
    ])
}

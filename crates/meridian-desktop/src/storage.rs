//! Host-only file custody. World validates bytes; this module never edits gameplay data.
use std::{
    collections::hash_map::RandomState,
    fs::{self, File, OpenOptions},
    hash::{BuildHasher, Hasher},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const SAVE_NAME: &str = "commander.meridian";

#[derive(Debug)]
pub enum SaveCommit {
    #[cfg(unix)]
    DirectorySynced,
    // The new file is published. Do not tell the player that the prior file remains current.
    Published {
        directory_sync_error: Option<io::Error>,
    },
}

pub struct Store {
    directory: PathBuf,
    save_name: &'static str,
    // An OS lease survives for this Store's lifetime and releases on process exit, including crashes.
    lease: File,
}

impl Drop for Store {
    fn drop(&mut self) {
        // A forked child can retain the open file description until exec closes its copy.
        // Release ownership now. The File still closes normally when Drop returns.
        let _ = self.lease.unlock();
    }
}

fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

struct PendingFile(PathBuf);
impl Drop for PendingFile {
    fn drop(&mut self) {
        // Only our create_new temporary file is eligible for cleanup. Never remove the save or lease.
        let _ = fs::remove_file(&self.0);
    }
}

impl Store {
    pub fn open(directory: PathBuf) -> io::Result<Self> {
        Self::open_named(directory, SAVE_NAME, "session.lock")
    }

    pub fn open_medusa_preview(directory: PathBuf) -> io::Result<Self> {
        Self::open_named(directory, "medusa-preview.meridian", "medusa-preview.lock")
    }

    fn open_named(
        directory: PathBuf,
        save_name: &'static str,
        lock_name: &str,
    ) -> io::Result<Self> {
        fs::create_dir_all(&directory)?;
        let lease = private_options()
            .create(true)
            .truncate(false)
            .open(directory.join(lock_name))?;
        lease.try_lock().map_err(|error| {
            io::Error::other(format!(
                "Cannot acquire this save's exclusive session lease: {error}"
            ))
        })?;
        Ok(Self {
            directory,
            save_name,
            lease,
        })
    }

    pub fn load(&self, max_bytes: usize) -> io::Result<Option<Vec<u8>>> {
        let path = self.directory.join(self.save_name);
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let limit = u64::try_from(max_bytes)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Invalid save byte limit")
            })?;
        let mut bytes = Vec::new();
        file.take(limit).read_to_end(&mut bytes)?;
        if bytes.len() > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Save exceeds the core format's byte limit",
            ));
        }
        Ok(Some(bytes))
    }

    pub fn save(&self, bytes: &[u8], max_bytes: usize) -> io::Result<SaveCommit> {
        if bytes.len() > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Encoded save exceeds its format limit",
            ));
        }
        self.publish_with(|file| file.write_all(bytes))
    }

    fn publish_with(
        &self,
        write: impl FnOnce(&mut File) -> io::Result<()>,
    ) -> io::Result<SaveCommit> {
        let nonce = RandomState::new().build_hasher().finish();
        let temporary = self
            .directory
            .join(format!(".save-{}-{nonce:016x}.pending", std::process::id()));
        let mut file = private_options().create_new(true).open(&temporary)?;
        let pending = PendingFile(temporary);
        let staged = write(&mut file).and_then(|()| file.sync_all());
        drop(file);
        staged?;
        fs::rename(&pending.0, self.directory.join(self.save_name))?;
        // The staging pathname no longer exists. Drop cannot touch the published destination.
        drop(pending);
        Ok(sync_directory(&self.directory))
    }
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> SaveCommit {
    match File::open(directory).and_then(|file| file.sync_all()) {
        Ok(()) => SaveCommit::DirectorySynced,
        Err(error) => SaveCommit::Published {
            directory_sync_error: Some(error),
        },
    }
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> SaveCommit {
    // std has no portable directory flush. File data was synced and same-directory rename succeeded.
    SaveCommit::Published {
        directory_sync_error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let nonce = RandomState::new().build_hasher().finish();
            let path = std::env::temp_dir().join(format!(
                "meridian-store-test-{}-{nonce:016x}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap_or_else(|e| panic!("{e}"));
            Self(path)
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn store(path: &Path) -> Store {
        Store::open(path.to_path_buf()).unwrap_or_else(|e| panic!("{e}"))
    }
    fn saved(result: io::Result<SaveCommit>) {
        match result.unwrap_or_else(|e| panic!("{e}")) {
            #[cfg(unix)]
            SaveCommit::DirectorySynced => {}
            SaveCommit::Published {
                directory_sync_error,
            } => assert!(directory_sync_error.is_none()),
        }
    }
    fn child(path: &Path, mode: &str) {
        let executable = std::env::current_exe().unwrap_or_else(|e| panic!("{e}"));
        let status = Command::new(executable)
            .args([
                "--exact",
                "storage::tests::child_process_custody_check",
                "--nocapture",
            ])
            .env("MERIDIAN_STORAGE_TEST_DIRECTORY", path)
            .env("MERIDIAN_STORAGE_TEST_MODE", mode)
            .status()
            .unwrap_or_else(|e| panic!("{e}"));
        if mode == "exit_without_drop" {
            assert_eq!(status.code(), Some(17));
        } else {
            assert!(status.success());
        }
    }

    #[test]
    fn child_process_custody_check() {
        // This helper is also discovered normally; it acts only in the explicitly spawned test child.
        let Some(path) = std::env::var_os("MERIDIAN_STORAGE_TEST_DIRECTORY") else {
            return;
        };
        match std::env::var("MERIDIAN_STORAGE_TEST_MODE").as_deref() {
            Ok("locked") => assert!(Store::open(path.into()).is_err()),
            Ok("exit_without_drop") => {
                let _held = store(Path::new(&path));
                // Deliberately bypass Rust Drop; the parent verifies OS lease release after exit.
                std::process::exit(17);
            }
            Ok("continued") => assert_eq!(
                store(Path::new(&path))
                    .load(100)
                    .unwrap_or_else(|e| panic!("{e}")),
                Some(b"second valid save".to_vec())
            ),
            _ => panic!("unknown explicit child test mode"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn dropping_owner_releases_lease_while_a_duplicate_descriptor_remains_open() {
        let directory = TestDirectory::new();
        let owner = store(&directory.0);
        // Model the descriptor temporarily inherited by a concurrent fork before exec.
        let inherited = owner.lease.try_clone().unwrap_or_else(|e| panic!("{e}"));
        assert!(Store::open(directory.0.clone()).is_err());
        drop(owner);

        let next_owner = store(&directory.0);
        assert!(Store::open(directory.0.clone()).is_err());
        drop(inherited);
        assert!(Store::open(directory.0.clone()).is_err());
        drop(next_owner);
        let _new_owner = store(&directory.0);
    }

    #[test]
    fn actual_new_process_observes_exclusive_lease_then_cold_file_continuation() {
        let directory = TestDirectory::new();
        let active = store(&directory.0);
        assert_eq!(active.load(100).unwrap_or_else(|e| panic!("{e}")), None);
        active
            .save(b"first save", 100)
            .unwrap_or_else(|e| panic!("{e}"));
        child(&directory.0, "locked");
        saved(active.save(b"second valid save", 100));
        drop(active);
        child(&directory.0, "exit_without_drop");
        child(&directory.0, "continued");
        assert!(directory.0.join("session.lock").exists());
    }

    #[test]
    fn partial_write_and_oversize_refuse_before_replacing_the_existing_save() {
        let directory = TestDirectory::new();
        let active = store(&directory.0);
        saved(active.save(b"prior good", 100));
        let failed = active.publish_with(|file| {
            file.write_all(b"truncated candidate")?;
            Err(io::Error::other("injected write failure"))
        });
        assert!(failed.is_err());
        assert_eq!(
            active.load(100).unwrap_or_else(|e| panic!("{e}")),
            Some(b"prior good".to_vec())
        );
        assert!(active.save(b"too long", 2).is_err());
        assert!(active.load(2).is_err());
        assert_eq!(
            fs::read(directory.0.join(SAVE_NAME)).unwrap_or_else(|e| panic!("{e}")),
            b"prior good"
        );
        let pending = fs::read_dir(&directory.0)
            .unwrap_or_else(|e| panic!("{e}"))
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".pending"))
            .count();
        assert_eq!(pending, 0);
    }

    #[test]
    fn rename_failure_leaves_the_existing_destination_and_removes_only_owned_staging() {
        let directory = TestDirectory::new();
        let active = store(&directory.0);
        let destination = directory.0.join(SAVE_NAME);
        fs::create_dir(&destination).unwrap_or_else(|e| panic!("{e}"));
        fs::write(destination.join("sentinel"), b"do not touch").unwrap_or_else(|e| panic!("{e}"));
        assert!(active.save(b"candidate", 100).is_err());
        assert_eq!(
            fs::read(destination.join("sentinel")).unwrap_or_else(|e| panic!("{e}")),
            b"do not touch"
        );
        assert_eq!(
            fs::read_dir(&directory.0)
                .unwrap_or_else(|e| panic!("{e}"))
                .count(),
            2
        );
    }
}

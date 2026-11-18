use std::{fs, io::Write, path::Path};

use serde_json::json;

use crate::{ImportError, Model, REVISION, Result, sha256};

const REFERENCE: &str = include_str!("../fixtures/reference.json");
const NOTICE: &[u8] = include_bytes!("../fixtures/SOURCE-NOTICE.txt");

#[derive(Debug)]
pub struct Artifact {
    name: &'static str,
    bytes: Vec<u8>,
}

impl Artifact {
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn verified(name: &'static str, bytes: Vec<u8>) -> Result<Self> {
        let reference: serde_json::Value =
            serde_json::from_str(REFERENCE).map_err(|e| ImportError::Decode(e.to_string()))?;
        let expected = &reference["outputs"][name];
        let actual_hash = sha256(&bytes);
        if expected["bytes"].as_u64() != Some(bytes.len() as u64)
            || expected["sha256"].as_str() != Some(actual_hash.as_str())
        {
            return Err(ImportError::Mismatch(format!(
                "converted {name} differs from the pinned existing bytes"
            )));
        }
        Ok(Self { name, bytes })
    }
}

#[derive(Debug)]
pub struct ImportBatch {
    artifacts: Vec<Artifact>,
    receipt: Vec<u8>,
    station_recipe: bool,
}

impl ImportBatch {
    pub fn artifacts(&self) -> &[Artifact] {
        &self.artifacts
    }
    pub fn receipt(&self) -> &[u8] {
        &self.receipt
    }

    pub(crate) fn new(
        artifacts: Vec<Artifact>,
        models: Vec<Model>,
        station_recipe: bool,
    ) -> Result<Self> {
        let reference: serde_json::Value =
            serde_json::from_str(REFERENCE).map_err(|e| ImportError::Decode(e.to_string()))?;
        let sources: Vec<_> = models
            .into_iter()
            .map(|model| {
                let r = model.reference();
                json!({"id": r.id, "path": r.path, "sha256": r.sha256, "bytes": r.bytes})
            })
            .collect();
        let outputs: Vec<_> = artifacts
            .iter()
            .map(|a| json!({"file": a.name, "bytes": a.bytes.len(), "sha256": sha256(&a.bytes)}))
            .collect();
        let manifest = json!({
            "schema": "meridian-import-v1",
            "producer": {"name": "meridian-import", "version": env!("CARGO_PKG_VERSION"), "source": "crates/meridian-import"},
            "sourceRevision": REVISION, "sources": sources,
            "legacyRecipeReferences": reference["legacyRecipes"],
            "legacyManifestReferences": reference["legacyManifests"],
            "artifacts": outputs,
            "sourceNotice": {"file": "SOURCE-NOTICE.txt", "bytes": NOTICE.len(), "sha256": sha256(NOTICE)},
            "formats": {"display": "Original triangle order LEf32 xyz+linear RGB,24 bytes/vertex; no UV/textures", "physical": "MPHYS001+LEu32 counts+LEf64 xyz+LEu32 indices", "port": "MPORT001+LEf64 center/outward/up/depth+LEu32 aperture count+LEf64 xyz"},
            "scope": "Selected original SH025/SH028 serialization only; existing bounded physical LOD, no new hull, complex geometry certification, source permission or mission admission. Legacy Python manifests are reference fixtures, not the Rust producer identity."
        });
        let mut receipt =
            serde_json::to_vec_pretty(&manifest).map_err(|e| ImportError::Decode(e.to_string()))?;
        receipt.push(b'\n');
        Ok(Self {
            artifacts,
            receipt,
            station_recipe,
        })
    }

    /// Compare original asset/manifest/notice fixtures without changing the directory.
    pub fn check_reference_directory(&self, directory: &Path) -> Result<()> {
        for artifact in &self.artifacts {
            let path = directory.join(artifact.name);
            let actual =
                fs::read(&path).map_err(|e| ImportError::io("read reference", &path, e))?;
            if actual != artifact.bytes {
                return Err(ImportError::Mismatch(artifact.name.to_owned()));
            }
        }
        let reference: serde_json::Value =
            serde_json::from_str(REFERENCE).map_err(|e| ImportError::Decode(e.to_string()))?;
        let mut manifests = Vec::new();
        if self.artifacts.iter().any(|a| a.name == "sh025.mesh") {
            manifests.push("manifest.json");
        }
        if self.station_recipe {
            manifests.push("station-manifest.json");
        }
        for name in manifests {
            let path = directory.join(name);
            let actual =
                fs::read(&path).map_err(|e| ImportError::io("read original manifest", &path, e))?;
            if reference["legacyManifests"][name]["sha256"].as_str()
                != Some(sha256(&actual).as_str())
            {
                return Err(ImportError::Mismatch(name.to_owned()));
            }
        }
        let path = directory.join("Elite-L337-MIT.txt");
        let actual =
            fs::read(&path).map_err(|e| ImportError::io("read original notice", &path, e))?;
        if actual.as_slice() != NOTICE {
            return Err(ImportError::Mismatch("original source notice".to_owned()));
        }
        Ok(())
    }

    /// Create only a new explicit destination, after conversion has fully succeeded.
    pub fn write_new_directory(&self, directory: &Path) -> Result<()> {
        fs::create_dir(directory).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                ImportError::DestinationExists(directory.display().to_string())
            } else {
                ImportError::io("create destination", directory, e)
            }
        })?;
        for artifact in &self.artifacts {
            write_new(&directory.join(artifact.name), &artifact.bytes)?;
        }
        write_new(&directory.join("SOURCE-NOTICE.txt"), NOTICE)?;
        // Publish the receipt only after every output and the receipt temporary have flushed.
        let temporary = directory.join("rust-import-manifest.json.tmp");
        write_new(&temporary, &self.receipt)?;
        let destination = directory.join("rust-import-manifest.json");
        fs::rename(&temporary, &destination)
            .map_err(|e| ImportError::io("publish receipt", &destination, e))
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| ImportError::io("create output", path, e))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| ImportError::io("write output", path, e))
}

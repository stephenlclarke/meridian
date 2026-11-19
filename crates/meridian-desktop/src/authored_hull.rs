//! Explicit read-only desktop boundary for session-local, core-validated hull packages.
use crate::{
    docked_render::MODELS,
    docked_screens::{AuthoredRecord, AvailableModel, Catalogue, ModelPreview},
    mule_render::showroom::{AuthoredFace, AuthoredVisual},
};
use meridian_core::docking::Vec3;
use meridian_core::hull_package::{
    HullPackage, LOCAL_INSPECTION_PREFIX as LOCAL_PREFIX, MAX_PACKAGE_JSON_BYTES, validate_package,
};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PackagePath {
    path: PathBuf,
    isolated: bool,
}

impl PackagePath {
    pub(crate) fn new(path: PathBuf, isolated: bool) -> Self {
        Self { path, isolated }
    }

    pub(crate) fn is_isolated(&self) -> bool {
        self.isolated
    }
}

struct AuthoredHull {
    source_id: String,
    record: AuthoredRecord,
    geometry_key: String,
    visual: AuthoredVisual,
}

#[derive(Default)]
pub(crate) struct AuthoredHulls {
    hulls: Vec<AuthoredHull>,
}

impl AuthoredHulls {
    pub(crate) fn load(paths: &[PackagePath]) -> Result<Self, String> {
        if paths.is_empty() {
            return Err("Expected at least one explicit hull package".into());
        }
        let mut catalogue = Catalogue::bundled().map_err(str::to_owned)?;
        let mut hulls = Vec::<AuthoredHull>::new();
        for input in paths {
            let hull = AuthoredHull::read(input)
                .map_err(|error| format!("Authored hull {}: {error}", input.path.display()))?;
            if hulls.iter().any(|previous| {
                previous.source_id.trim().to_lowercase() == hull.source_id.trim().to_lowercase()
            }) {
                return Err(format!(
                    "Duplicate authored source identity: {}",
                    hull.source_id
                ));
            }
            catalogue.add_authored(hull.record.clone())?;
            hulls.push(hull);
        }
        Ok(Self { hulls })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.hulls.is_empty()
    }

    pub(crate) fn records(&self) -> impl Iterator<Item = &AuthoredRecord> {
        self.hulls.iter().map(|hull| &hull.record)
    }

    /// The single isolated identity authorized for the explicit one-click entry.
    pub(crate) fn inspection_id(&self) -> Option<&str> {
        let mut isolated = self
            .records()
            .filter(|record| record.id.starts_with(LOCAL_PREFIX));
        let id = isolated.next()?.id.as_str();
        isolated.next().is_none().then_some(id)
    }

    pub(crate) fn available_models(&self) -> Vec<AvailableModel<'_>> {
        let mut models: Vec<AvailableModel<'_>> = MODELS.to_vec();
        models.extend(self.hulls.iter().map(|hull| AvailableModel {
            object: &hull.record.id,
            geometry_key: &hull.geometry_key,
        }));
        models
    }

    pub(crate) fn visual_for(&self, preview: &ModelPreview) -> Option<&AuthoredVisual> {
        self.hulls
            .iter()
            .find(|hull| {
                hull.record.id == preview.object && hull.geometry_key == preview.geometry_key
            })
            .map(|hull| &hull.visual)
    }
}

impl AuthoredHull {
    fn read(input: &PackagePath) -> Result<Self, String> {
        let bytes = read_package(&input.path).map_err(|error| error.to_string())?;
        let package = HullPackage::from_json(&bytes).map_err(|error| error.to_string())?;
        Self::from_package(&package, input.isolated, fingerprint(&bytes))
    }

    fn from_package(
        package: &HullPackage,
        isolated: bool,
        fingerprint: u64,
    ) -> Result<Self, String> {
        let source_id = package.id().to_owned();
        let id = if isolated {
            format!("{LOCAL_PREFIX}{source_id}")
        } else {
            source_id.clone()
        };
        let name = if isolated {
            format!("{} (local package)", package.name())
        } else {
            package.name().to_owned()
        };
        let (visual, mut specifications) = package_presentation(package)?;
        specifications.insert(0, format!("Package source ID: {source_id}"));
        specifications.push("Session inspection only; no purchase, actor or save authority".into());
        Ok(Self {
            geometry_key: format!("authored-package/{id}/{fingerprint:016x}"),
            record: AuthoredRecord {
                id,
                name,
                specifications,
            },
            source_id,
            visual,
        })
    }
}

fn read_package(path: &Path) -> io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() > MAX_PACKAGE_JSON_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected a regular hull package file within the byte limit (no symlinks)",
        ));
    }
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Hull package is no longer a regular file",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_PACKAGE_JSON_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PACKAGE_JSON_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Hull package exceeds the byte limit",
        ));
    }
    Ok(bytes)
}

// A session cache/geometry discriminator, not a source signature or permission check.
fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    })
}

fn package_presentation(package: &HullPackage) -> Result<(AuthoredVisual, Vec<String>), String> {
    let report = validate_package(package);
    if !report.valid {
        return Err(format!(
            "Core hull validation refused: {}",
            report
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    let pieces = package
        .pieces()
        .map_err(|report| format!("Core collision package refused: {:?}", report.diagnostics))?;
    if pieces.len() != report.support_groups.len() {
        return Err("Core collision package changed after validation".into());
    }
    // The immutable definition stores source units; collision and every published dimension use metres.
    let vertices: Vec<[f64; 3]> = package
        .detail
        .vertices
        .iter()
        .map(|point| point.map(|coordinate| coordinate * package.metres_per_unit))
        .collect();
    let mut faces = Vec::with_capacity(package.detail.triangles.len());
    for (index, face) in package.detail.triangles.iter().enumerate() {
        let material = package
            .materials
            .get(package.face_materials[index])
            .ok_or("Authored material is missing")?;
        let mapping = package
            .face_textures
            .as_ref()
            .and_then(|mappings| mappings.get(index));
        faces.push(AuthoredFace {
            indices: *face,
            rgb: material.rgb,
            alpha: material.alpha,
            emissive: material.emissive,
            texture: mapping
                .and_then(|mapping| mapping.texture)
                .map(meridian_core::hull_package::Texture::atlas_index),
            uv: mapping.map_or([[0.0; 2]; 3], |mapping| mapping.uv),
        });
    }
    let (visual, dimensions) = AuthoredVisual::from_geometry(&vertices, faces)?;
    let [x, y, z] = dimensions;
    let mut specs = vec![
        format!("Dimensions (m): {x:.3} x {y:.3} x {z:.3}"),
        format!(
            "Detailed faces: {}; native faces: {}",
            package.detail.triangles.len(),
            package.native.triangles.len()
        ),
        format!(
            "Core collision supports: {} ({} m rounding)",
            pieces.len(),
            package.rounding_metres
        ),
        format!(
            "Local origin (core contact): {}",
            if pieces
                .iter()
                .any(|piece| piece.contains(Vec3::new(0.0, 0.0, 0.0)))
            {
                "BLOCKED"
            } else {
                "CLEAR"
            }
        ),
        format!(
            "Materials: {}; mapped faces: {}",
            package.materials.len(),
            package.face_textures.as_ref().map_or(0, |textures| textures
                .iter()
                .filter(|mapping| mapping.texture.is_some())
                .count())
        ),
        package
            .engineering
            .as_ref()
            .map_or("Source mass: unspecified".into(), |engineering| {
                format!("Source mass: {} kg (not acquired)", engineering.mass_kg)
            }),
        package
            .flight
            .max_speed
            .map_or("Source maximum speed: unspecified".into(), |speed| {
                format!("Source maximum speed: {speed} (not flight admission)")
            }),
    ];
    if report.clearances.is_empty() {
        specs.push("Core clearance sweeps: none authored".into());
    }
    for clearance in report.clearances {
        specs.push(format!(
            "Core sweep {}: {}",
            clearance.name,
            if clearance.passed { "CLEAR" } else { "BLOCKED" }
        ));
    }
    Ok((visual, specs))
}

#[cfg(test)]
pub(crate) fn fixture_path(changed: bool) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../integration/authored-hulls")
        .join(if changed {
            "DEMO-CHANGED.json"
        } else {
            "DEMO.json"
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        docked_screens::{Orientation, Rect},
        profile::tests::TestDirectory,
    };

    fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
        value.unwrap_or_else(|error| panic!("{error:?}"))
    }
    fn preview(hulls: &AuthoredHulls, id: &str) -> ModelPreview {
        let models = hulls.available_models();
        let selected = models
            .iter()
            .find(|model| model.object == id)
            .unwrap_or_else(|| panic!("missing {id}"));
        ModelPreview {
            object: id.into(),
            geometry_key: selected.geometry_key.into(),
            bounds: Rect::new(0.0, 0.0, 200.0, 200.0),
            orientation: Orientation::default(),
        }
    }

    #[test]
    fn arbitrary_package_geometry_material_dimensions_and_core_clearance_change_together() {
        let first = ok(AuthoredHulls::load(&[PackagePath::new(
            fixture_path(false),
            false,
        )]));
        let changed = ok(AuthoredHulls::load(&[PackagePath::new(
            fixture_path(true),
            false,
        )]));
        let a = preview(&first, "TEST901");
        let b = preview(&changed, "TEST901");
        assert_ne!(a.geometry_key, b.geometry_key);
        let one = first
            .visual_for(&a)
            .unwrap_or_else(|| panic!("first visual"));
        let two = changed
            .visual_for(&b)
            .unwrap_or_else(|| panic!("changed visual"));
        assert_ne!(one.bytes(), two.bytes());
        assert_ne!(&one.bytes()[..12], &two.bytes()[..12]);
        assert_ne!(&one.bytes()[24..40], &two.bytes()[24..40]);
        assert!(first.visual_for(&b).is_none() && changed.visual_for(&a).is_none());
        assert!(first.records().any(|r| {
            r.specifications
                .iter()
                .any(|s| s == "Dimensions (m): 2.000 x 2.000 x 2.000")
        }));
        assert!(changed.records().any(|r| {
            r.specifications
                .iter()
                .any(|s| s == "Dimensions (m): 5.000 x 2.000 x 2.000")
        }));
        for registry in [&first, &changed] {
            assert!(registry.records().any(|r| {
                r.specifications
                    .iter()
                    .any(|s| s == "Core sweep Side pass: CLEAR")
            }));
        }
        let directory = TestDirectory::new();
        let blocked = directory.0.join("blocked.json");
        let changed = ok(fs::read_to_string(fixture_path(true))).replace("5.0, -0.9", "2.5, -0.9");
        ok(fs::write(&blocked, changed));
        let failure = AuthoredHulls::load(&[PackagePath::new(blocked, false)])
            .err()
            .unwrap_or_else(|| panic!("blocked core sweep accepted"));
        assert!(
            failure.contains("Side pass") && failure.contains("blocked"),
            "{failure}"
        );
    }

    #[test]
    fn conflicts_and_invalid_inputs_refuse_but_explicit_original_inspection_is_separate() {
        let directory = TestDirectory::new();
        let path = directory.0.join("original-id.json");
        let fixture = ok(fs::read_to_string(fixture_path(false)))
            .replace("TEST901", "SH023")
            .replace("Fixture Skiff", "Peregrine Mk III");
        ok(fs::write(&path, &fixture));
        let failure = AuthoredHulls::load(&[PackagePath::new(path.clone(), false)])
            .err()
            .unwrap_or_else(|| panic!("original collision accepted"));
        assert!(failure.contains("identity conflict"), "{failure}");
        let isolated = ok(AuthoredHulls::load(&[PackagePath::new(path.clone(), true)]));
        let local = preview(&isolated, "LOCAL:SH023");
        let original = preview(&isolated, "SH023");
        assert!(isolated.visual_for(&local).is_some());
        assert!(isolated.visual_for(&original).is_none());
        let mut crossed = local;
        crossed.object = original.object;
        assert!(isolated.visual_for(&crossed).is_none());
        assert!(
            AuthoredHulls::load(&[
                PackagePath::new(path.clone(), true),
                PackagePath::new(path, true)
            ])
            .is_err()
        );
        let invalid = directory.0.join("invalid.json");
        ok(fs::write(&invalid, b"{}"));
        assert!(AuthoredHulls::load(&[PackagePath::new(invalid, false)]).is_err());
        assert!(AuthoredHulls::load(&[PackagePath::new(directory.0.clone(), false)]).is_err());
        let reserved = directory.0.join("reserved.json");
        ok(fs::write(
            &reserved,
            fixture.replace("SH023", "  LoCaL:other  "),
        ));
        for isolated in [false, true] {
            let error = AuthoredHulls::load(&[PackagePath::new(reserved.clone(), isolated)])
                .err()
                .unwrap_or_else(|| panic!("reserved source identity accepted"));
            assert!(error.contains("reserved LOCAL:"), "{error}");
        }
    }
}

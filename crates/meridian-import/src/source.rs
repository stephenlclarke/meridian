use std::{fs, io::Read, path::Path, process::Command};

use serde::Deserialize;

use crate::{ImportError, Result, sha256};

pub const REVISION: &str = "a11fe16924d87d06021816f9037952afcbac08db";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Sh025,
    Sh028,
}

pub(crate) struct Reference {
    pub id: &'static str,
    pub name: &'static str,
    pub path: &'static str,
    pub sha256: &'static str,
    pub bytes: usize,
    pub display: (usize, usize),
    pub physical: (usize, usize),
}

impl Model {
    pub(crate) fn reference(self) -> Reference {
        match self {
            Self::Sh025 => Reference {
                id: "SH025",
                name: "Peregrine Mk V",
                path: "Sources/L337Core/Resources/Ships/SH025.toml",
                sha256: "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883",
                bytes: 690857,
                display: (2072, 3484),
                physical: (536, 964),
            },
            Self::Sh028 => Reference {
                id: "SH028",
                name: "Lion Citadel",
                path: "Sources/L337Core/Resources/Ships/SH028.toml",
                sha256: "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9",
                bytes: 1397484,
                display: (5888, 10056),
                physical: (272, 488),
            },
        }
    }

    pub fn identifier(self) -> &'static str {
        self.reference().id
    }

    pub(crate) fn git_spec(self) -> String {
        format!("{REVISION}:{}", self.reference().path)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Document {
    pub id: String,
    pub name: String,
    pub design: Design,
    #[serde(rename = "nativeDesign")]
    pub native_design: Design,
    #[serde(rename = "nativeDocking")]
    pub docking: Option<Port>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Design {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[u32; 3]>,
    #[serde(rename = "faceMaterials")]
    pub face_materials: Vec<u32>,
    pub materials: Vec<Material>,
    pub license: String,
    #[serde(rename = "appearanceLicense")]
    pub appearance_license: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Material {
    pub colour: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point {
    pub fn components(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Port {
    pub center: Point,
    pub outward: Point,
    pub up: Point,
    #[serde(rename = "tunnelDepth")]
    pub tunnel_depth: f64,
    pub aperture: Vec<Point>,
}

pub(crate) fn verify_bytes(model: Model, raw: &[u8]) -> Result<()> {
    let reference = model.reference();
    if raw.len() != reference.bytes || sha256(raw) != reference.sha256 {
        return Err(ImportError::Identity(
            "Immutable source size/SHA256 differs from the selected a11 reference",
        ));
    }
    Ok(())
}

pub(crate) fn parse_document(raw: &[u8]) -> Result<Document> {
    let text = std::str::from_utf8(raw).map_err(|e| ImportError::Decode(e.to_string()))?;
    toml_edit::de::from_str(text).map_err(|e| ImportError::Decode(e.to_string()))
}

pub(crate) fn validate_identity(model: Model, document: &Document) -> Result<()> {
    let reference = model.reference();
    if document.id != reference.id || document.name != reference.name {
        return Err(ImportError::Identity("Selected source ID/name differs"));
    }
    if (document.design.vertices.len(), document.design.faces.len()) != reference.display
        || (
            document.native_design.vertices.len(),
            document.native_design.faces.len(),
        ) != reference.physical
    {
        return Err(ImportError::Geometry(
            "Selected display/native geometry counts differ",
        ));
    }
    Ok(())
}

pub(crate) fn decode(model: Model, raw: &[u8]) -> Result<Document> {
    verify_bytes(model, raw)?;
    let document = parse_document(raw)?;
    validate_document(model, &document)?;
    Ok(document)
}

pub(crate) fn validate_document(model: Model, document: &Document) -> Result<()> {
    validate_identity(model, document)?;
    document.design.validate()?;
    document.native_design.validate()?;
    if model == Model::Sh028 {
        document
            .docking
            .as_ref()
            .ok_or(ImportError::Geometry("Missing selected station port"))?
            .validate()?;
    }
    Ok(())
}

pub fn read_file(path: &Path, model: Model) -> Result<Vec<u8>> {
    let expected = model.reference().bytes;
    let file = fs::File::open(path).map_err(|e| ImportError::io("open source", path, e))?;
    let mut raw = Vec::with_capacity(expected);
    file.take((expected + 1) as u64)
        .read_to_end(&mut raw)
        .map_err(|e| ImportError::io("read source", path, e))?;
    verify_bytes(model, &raw)?;
    Ok(raw)
}

pub fn read_repository(repository: &Path, model: Model) -> Result<Vec<u8>> {
    let directory = repository
        .canonicalize()
        .map_err(|e| ImportError::io("resolve repository", repository, e))?;
    if !directory.is_dir() {
        return Err(ImportError::Identity(
            "Source repository must be a directory",
        ));
    }
    // Only the selected fixed revision/path reaches Git argv. No working file or HEAD is read.
    let output = Command::new("git")
        .args(["--no-pager", "show", "--no-ext-diff", "--no-textconv"])
        .arg(model.git_spec())
        .current_dir(&directory)
        .output()
        .map_err(|e| ImportError::io("run Git in", &directory, e))?;
    if !output.status.success() {
        return Err(ImportError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    verify_bytes(model, &output.stdout)?;
    Ok(output.stdout)
}

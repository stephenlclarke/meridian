//! Offline, deterministic hull authoring. Validation creates no playable actor or save rights.
use crate::{
    docking::{Basis3, RigidPose, Vec3},
    mule::SmoothPiece,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod preview;
pub use preview::{PreviewFit, PreviewTriangle};

const MAX_POINTS_PER_MESH: usize = 100_000;
pub const MAX_PACKAGE_JSON_BYTES: u64 = 64 * 1024 * 1024;
pub const LOCAL_INSPECTION_PREFIX: &str = "LOCAL:";
const MAX_DETAILED_FACES: usize = 200_000;
const MAX_NATIVE_FACES: usize = 512;
const MAX_GROUPS: usize = 1_024;
const MAX_CLEARANCES: usize = 256;
const COORDINATE_LIMIT_METRES: f64 = 1.0e6;

/// Coordinates in each mesh use source units. Engineering and clearance coordinates use metres.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HullPackage {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub metres_per_unit: f64,
    pub rounding_metres: f64,
    pub detail: Mesh,
    pub native: Mesh,
    pub materials: Vec<Material>,
    pub face_materials: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_textures: Option<Vec<FaceTexture>>,
    /// Missing physical engineering stays unknown; authoring never invents a stock budget.
    pub engineering: Option<Engineering>,
    pub flight: Flight,
    #[serde(default)]
    pub clearances: Vec<Clearance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<serde_json::Value>,
}

impl HullPackage {
    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        if bytes.len() as u64 > MAX_PACKAGE_JSON_BYTES {
            return Err(serde::de::Error::custom(
                "Package exceeds the 64 MiB input limit",
            ));
        }
        serde_json::from_slice(bytes)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return collision pieces only after the whole authoring package has passed validation.
    pub fn pieces(&self) -> Result<Vec<SmoothPiece>, ValidationReport> {
        let report = validate_package(self);
        if !report.valid {
            return Err(report);
        }
        report
            .support_groups
            .iter()
            .map(|group| {
                SmoothPiece::new(
                    group.points.iter().copied().map(point).collect(),
                    group.rounding_metres,
                )
                .map_err(|error| {
                    ValidationReport::invalid(format!(
                        "Support group {} refused: {error}",
                        group.id
                    ))
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Mesh {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
    /// Group labels are local to this package, not catalogue indices. A label is shared across LODs.
    pub groups: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Material {
    pub name: String,
    pub rgb: [u8; 3],
    #[serde(default = "opaque")]
    pub alpha: u8,
    #[serde(default)]
    pub emissive: bool,
}

const fn opaque() -> u8 {
    255
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Texture {
    #[serde(rename = "l337-patrol-diffuse")]
    Patrol,
    #[serde(rename = "l337-strike-diffuse")]
    Strike,
    #[serde(rename = "l337-service-details")]
    Service,
}

impl Texture {
    pub const fn atlas_index(self) -> u32 {
        match self {
            Self::Patrol => 0,
            Self::Strike => 1,
            Self::Service => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FaceTexture {
    pub texture: Option<Texture>,
    pub uv: [[f64; 2]; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Engineering {
    pub mass_kg: f64,
    pub center_of_mass: [f64; 3],
    pub inertia: [[f64; 3]; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Flight {
    /// Unknown performance is represented explicitly by null and never grants flight admission.
    pub max_speed: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Clearance {
    pub name: String,
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub radius_metres: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClearanceResult {
    pub name: String,
    pub passed: bool,
}

/// Exact support points, in metres. The enclosure is their convex hull plus rounding, not an AABB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SupportGroup {
    pub id: u32,
    pub points: Vec<[f64; 3]>,
    pub rounding_metres: f64,
}

impl SupportGroup {
    pub fn points(&self) -> &[[f64; 3]] {
        &self.points
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub valid: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub clearances: Vec<ClearanceResult>,
    pub support_groups: Vec<SupportGroup>,
}

impl std::fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.valid {
            return f.write_str("Valid hull package");
        }
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index != 0 {
                f.write_str("; ")?;
            }
            f.write_str(&diagnostic.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationReport {}

impl ValidationReport {
    pub fn invalid(message: impl Into<String>) -> Self {
        let mut report = Self::default();
        report.reject(message);
        report
    }

    pub fn support_groups(&self) -> &[SupportGroup] {
        &self.support_groups
    }

    fn reject(&mut self, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            message: message.into(),
        });
    }
}

fn finite_coordinate(point: [f64; 3]) -> bool {
    point
        .into_iter()
        .all(|value| value.is_finite() && value.abs() <= COORDINATE_LIMIT_METRES)
}

fn point(value: [f64; 3]) -> Vec3 {
    Vec3::new(value[0], value[1], value[2])
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

/// Positive definite symmetric matrix via normalized leading principal minors.
fn positive_definite(m: [[f64; 3]; 3]) -> bool {
    let a = m[0][0];
    let b = m[0][1];
    let c = m[0][2];
    let d = m[1][1];
    let e = m[1][2];
    let f = m[2][2];
    a > 0.0
        && a * d - b * b > 0.0
        && a * (d * f - e * e) - b * (b * f - c * e) + c * (b * e - c * d) > 0.0
}

fn valid_inertia(inertia: [[f64; 3]; 3]) -> bool {
    if !inertia.iter().flatten().all(|value| value.is_finite()) {
        return false;
    }
    let scale = inertia
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0, f64::max);
    if scale <= 0.0
        || inertia[0][1] != inertia[1][0]
        || inertia[0][2] != inertia[2][0]
        || inertia[1][2] != inertia[2][1]
    {
        return false;
    }
    let m = inertia.map(|row| row.map(|value| value / scale));
    let half_trace = (m[0][0] + m[1][1] + m[2][2]) / 2.0;
    // Physical principal-moment triangle inequalities: half(trace(I)) I₃ − I is a mass second moment.
    let second = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            if i == j {
                half_trace - m[i][j]
            } else {
                -m[i][j]
            }
        })
    });
    positive_definite(m) && positive_definite(second)
}

fn triangle_nonzero(points: [[f64; 3]; 3]) -> bool {
    let [a, b, c] = points;
    let u = std::array::from_fn::<_, 3, _>(|i| b[i] - a[i]);
    let v = std::array::from_fn::<_, 3, _>(|i| c[i] - a[i]);
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    cross
        .into_iter()
        .any(|value| value != 0.0 && value.is_finite())
}

fn validate_mesh(
    label: &str,
    mesh: &Mesh,
    scale: f64,
    face_limit: usize,
    report: &mut ValidationReport,
) -> Option<Vec<[f64; 3]>> {
    if mesh.vertices.is_empty()
        || mesh.vertices.len() > MAX_POINTS_PER_MESH
        || mesh.triangles.is_empty()
        || mesh.triangles.len() > face_limit
        || mesh.groups.len() != mesh.vertices.len()
    {
        report.reject(format!(
            "{label}: invalid vertex, triangle or group cardinality (face limit {face_limit})"
        ));
        return None;
    }
    let vertices: Vec<_> = mesh
        .vertices
        .iter()
        .map(|p| p.map(|value| value * scale))
        .collect();
    if !vertices.iter().copied().all(finite_coordinate) {
        report.reject(format!(
            "{label}: nonfinite or out-of-range metre coordinates"
        ));
        return None;
    }
    let mut referenced = vec![false; vertices.len()];
    for (index, &[a, b, c]) in mesh.triangles.iter().enumerate() {
        if [a, b, c].into_iter().any(|x| x >= vertices.len()) || a == b || a == c || b == c {
            report.reject(format!("{label}: invalid triangle {index} indices"));
            return None;
        }
        if mesh.groups[a] != mesh.groups[b] || mesh.groups[a] != mesh.groups[c] {
            report.reject(format!("{label}: triangle {index} crosses support groups"));
            return None;
        }
        if !triangle_nonzero([vertices[a], vertices[b], vertices[c]]) {
            report.reject(format!("{label}: triangle {index} has no area"));
            return None;
        }
        for vertex in [a, b, c] {
            referenced[vertex] = true;
        }
    }
    if referenced.iter().any(|used| !used) {
        report.reject(format!("{label}: at least one vertex has no face"));
        return None;
    }
    Some(vertices)
}

fn validate_appearance(package: &HullPackage, report: &mut ValidationReport) {
    if package.materials.is_empty()
        || package.materials.len() > 65_536
        || package.materials.iter().any(|m| !valid_text(&m.name))
        || package.face_materials.len() != package.detail.triangles.len()
        || package
            .face_materials
            .iter()
            .any(|&index| index >= package.materials.len())
    {
        report.reject("Invalid detailed face material coverage or material name/index");
    }
    if let Some(mapping) = &package.face_textures
        && (mapping.len() != package.detail.triangles.len()
            || mapping
                .iter()
                .flat_map(|face| face.uv.iter().flatten())
                .any(|value| !value.is_finite() || !(*value as f32).is_finite()))
    {
        report.reject("Invalid detailed face texture coverage or nonfinite UV");
    }
}

fn clearance_passes(clearance: &Clearance, pieces: &[SmoothPiece]) -> bool {
    let pose = |p: [f64; 3]| RigidPose {
        position: point(p.map(|value| -value)),
        orientation: Basis3::IDENTITY,
    };
    // In relative coordinates the hull moves oppositely past a stationary, padded point.
    // This is the same conservative continuous swept support routine used by flight collision.
    let zero = Vec3::new(0.0, 0.0, 0.0);
    pieces.iter().all(|piece| {
        matches!(
            piece.sweep_clears_triangle_with_padding(
                pose(clearance.start),
                pose(clearance.end),
                0.0,
                [zero; 3],
                clearance.radius_metres
            ),
            Ok(true)
        )
    })
}

/// Build immutable support groups from both LODs, then prove each declared continuous clearance.
/// The report is authoring evidence only and never changes World or authorizes a new hull.
pub fn validate_package(package: &HullPackage) -> ValidationReport {
    let mut report = ValidationReport::default();
    if package.schema != 1 || !valid_text(&package.id) || !valid_text(&package.name) {
        report.reject("Expected schema 1 and nonempty hull ID and name");
    }
    if package
        .id
        .trim()
        .to_ascii_uppercase()
        .starts_with(LOCAL_INSPECTION_PREFIX)
    {
        report.reject("Authored source ID uses reserved LOCAL: inspection namespace");
    }
    let valid_scale = package.metres_per_unit.is_finite() && package.metres_per_unit > 0.0;
    let valid_rounding = package.rounding_metres.is_finite()
        && package.rounding_metres > 0.0
        && package.rounding_metres <= 0.02;
    if !valid_scale || !valid_rounding {
        report.reject("Scale must be positive and rounding must be in (0, 0.02] metres");
    }
    if package.engineering.as_ref().is_some_and(|engineering| {
        !engineering.mass_kg.is_finite()
            || engineering.mass_kg <= 0.0
            || !finite_coordinate(engineering.center_of_mass)
            || !valid_inertia(engineering.inertia)
    }) {
        report.reject("Engineering requires positive mass, finite centre of mass and symmetric physically positive inertia");
    }
    if package.flight.max_speed.is_some_and(|speed| {
        !speed.is_finite() || speed <= 0.0 || !(speed as f32).is_finite() || (speed as f32) <= 0.0
    }) {
        report.reject("Flight maximum speed must be finite and positive");
    }
    validate_appearance(package, &mut report);
    if package.clearances.len() > MAX_CLEARANCES {
        report.reject("Too many declared clearance routes");
        return report;
    }
    let mut names = BTreeSet::new();
    let mut routes_valid = true;
    for route in &package.clearances {
        let valid = valid_text(&route.name)
            && names.insert(&route.name)
            && finite_coordinate(route.start)
            && finite_coordinate(route.end)
            && route.radius_metres.is_finite()
            && route.radius_metres >= 0.0
            && route.radius_metres <= COORDINATE_LIMIT_METRES;
        if !valid {
            routes_valid = false;
        }
        report.clearances.push(ClearanceResult {
            name: route.name.clone(),
            passed: false,
        });
    }
    if !routes_valid {
        report.reject("Invalid or duplicate clearance name, endpoint or radius");
    }
    if !valid_scale || !valid_rounding {
        return report;
    }
    let detail = validate_mesh(
        "detail",
        &package.detail,
        package.metres_per_unit,
        MAX_DETAILED_FACES,
        &mut report,
    );
    let native = validate_mesh(
        "native",
        &package.native,
        package.metres_per_unit,
        MAX_NATIVE_FACES,
        &mut report,
    );
    let (Some(detail), Some(native)) = (detail, native) else {
        return report;
    };
    // The desktop will use this exact presentation fit, so f64-valid geometry that
    // collapses in the actual f32 vertex stream cannot pass Studio preflight.
    let fit = match PreviewFit::new(&detail) {
        Ok(fit) => fit,
        Err(error) => {
            report.reject(format!("Detailed preview: {error}"));
            return report;
        }
    };
    for (index, &triangle) in package.detail.triangles.iter().enumerate() {
        if let Err(error) = fit.triangle(triangle) {
            report.reject(format!("Detailed preview triangle {index}: {error}"));
            return report;
        }
    }
    let mut groups = BTreeMap::<u32, Vec<[f64; 3]>>::new();
    for (vertices, labels) in [
        (detail, &package.detail.groups),
        (native, &package.native.groups),
    ] {
        for (vertex, id) in vertices.into_iter().zip(labels) {
            groups.entry(*id).or_default().push(vertex);
        }
    }
    if groups.len() > MAX_GROUPS {
        report.reject("Too many local support groups");
        return report;
    }
    let mut pieces = Vec::with_capacity(groups.len());
    for (id, points) in groups {
        let built = SmoothPiece::new(
            points.iter().copied().map(point).collect(),
            package.rounding_metres,
        );
        match built {
            Ok(piece) => {
                report.support_groups.push(SupportGroup {
                    id,
                    points,
                    rounding_metres: package.rounding_metres,
                });
                pieces.push(piece);
            }
            Err(error) => {
                report.reject(format!("Support group {id} refused: {error}"));
                report.support_groups.clear();
                return report;
            }
        }
    }
    if routes_valid {
        for (route, result) in package.clearances.iter().zip(&mut report.clearances) {
            result.passed = clearance_passes(route, &pieces);
        }
        let blocked: Vec<_> = report
            .clearances
            .iter()
            .filter(|item| !item.passed)
            .map(|item| item.name.clone())
            .collect();
        for name in blocked {
            report.reject(format!("Clearance {name} is blocked or indeterminate"));
        }
    }
    report.valid = report.diagnostics.is_empty();
    report
}

#[cfg(test)]
mod tests;

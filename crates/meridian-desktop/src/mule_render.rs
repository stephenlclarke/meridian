//! SH001 presentation only. No actor creation, hull admission or gameplay authority.
mod gpu;
#[cfg(test)]
mod gpu_tests;
mod rounded;
pub(crate) mod showroom;
#[cfg(test)]
mod tests;
pub use gpu::Pass;
use meridian_core::FlightView;
use meridian_core::docking::{RigidPose, Vec3};
use meridian_core::mule::{
    MULE_PIECES, MULE_PRESENTATION_REVISION, MuleHullSource, MulePresentation,
};
use meridian_core::traffic::NpcSnapshot;

/// Borrow the actual World-owned actor and its matching immutable contact surface.
/// Root's World adapter supplies this only after SH001 admission; never substitute SH025.
#[derive(Clone, Copy)]
pub struct Scene<'a> {
    pub actor: &'a NpcSnapshot,
    pub envelope: &'a MulePresentation,
    pub observer: RigidPose,
}

/// World-validated visual input. Only the isolated validation route changes the
/// camera for the rest of the scene.
#[derive(Clone, Copy)]
pub(crate) enum Placement<'a> {
    Internal(Scene<'a>),
    Ordinary {
        actor: &'a NpcSnapshot,
        envelope: &'a MulePresentation,
    },
}
impl Placement<'_> {
    pub(crate) fn internal_observer(self) -> Option<RigidPose> {
        match self {
            Self::Internal(scene) => Some(scene.observer),
            Self::Ordinary { .. } => None,
        }
    }
}
impl<'a> Placement<'a> {
    pub(crate) fn for_camera(
        self,
        flight_camera: impl FnOnce() -> Result<RigidPose, String>,
    ) -> Result<Scene<'a>, String> {
        match self {
            Self::Internal(scene) => Ok(scene),
            Self::Ordinary { actor, envelope } => Ok(Scene {
                actor,
                envelope,
                observer: flight_camera()?,
            }),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VisualVertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
    uv: [f32; 2],
    texture: u32,
    emissive: u32,
}
const VISUAL: &[u8] = include_bytes!("../../../assets/fleet/sh001/visual.bin");
const VERTEX_COUNT: usize = 7236 * 3;
const SABLE_NO_MAP_LAYER: u32 = u32::MAX;
const GAZELLE_NO_MAP_LAYER: u32 = 3;
const MAX_POINTS: usize = 5344;
type SmoothPieceArray = [meridian_core::mule::SmoothPiece; MULE_PIECES];
const TEXTURES: [&[u8]; 3] = [
    include_bytes!("../../../assets/fleet/sh001/l337-patrol-diffuse.rgba"),
    include_bytes!("../../../assets/fleet/sh001/l337-strike-diffuse.rgba"),
    include_bytes!("../../../assets/fleet/sh001/l337-service-details.rgba"),
];

fn visual() -> Result<Vec<VisualVertex>, &'static str> {
    decode_visual(VISUAL, b"MULEVIS1", VERTEX_COUNT)
}
fn decode_visual(
    bytes: &[u8],
    magic: &[u8; 8],
    count: usize,
) -> Result<Vec<VisualVertex>, &'static str> {
    decode_visual_layers(bytes, magic, count, VisualTexturePolicy::TexturedOnly)
}

#[derive(Clone, Copy)]
enum VisualTexturePolicy {
    TexturedOnly,
    SableLiner,
    Gazelle,
}

impl VisualTexturePolicy {
    fn matches_format(self, magic: &[u8; 8]) -> bool {
        match self {
            Self::TexturedOnly => magic != b"SBLRVIZ1" && magic != b"SH019VZ1",
            Self::SableLiner => magic == b"SBLRVIZ1",
            Self::Gazelle => magic == b"SH019VZ1",
        }
    }

    fn allows(self, layer: u32) -> bool {
        layer < 3
            || match self {
                Self::TexturedOnly => false,
                Self::SableLiner => layer == SABLE_NO_MAP_LAYER,
                Self::Gazelle => layer == GAZELLE_NO_MAP_LAYER,
            }
    }
}

fn decode_visual_layers(
    bytes: &[u8],
    magic: &[u8; 8],
    count: usize,
    policy: VisualTexturePolicy,
) -> Result<Vec<VisualVertex>, &'static str> {
    if !policy.matches_format(magic)
        || bytes.get(..8) != Some(magic)
        || bytes.get(8..12) != Some(&(count as u32).to_le_bytes())
        || bytes.get(12..16) != Some(&56u32.to_le_bytes())
        || bytes.len() != 16 + count * 56
    {
        return Err("Detailed visual encoding changed");
    }
    // include_bytes need not have Pod alignment. Read each complete record without unsafe.
    let result: Vec<VisualVertex> = bytes[16..]
        .as_chunks::<56>()
        .0
        .iter()
        .map(|record| bytemuck::pod_read_unaligned(record.as_slice()))
        .collect();
    if result.iter().any(|v| {
        v.position
            .iter()
            .chain(&v.normal)
            .chain(&v.color)
            .chain(&v.uv)
            .any(|x| !x.is_finite())
            || !policy.allows(v.texture)
            || v.emissive > 1
    }) {
        return Err("Invalid detailed visual attributes");
    }
    Ok(result)
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Triangle {
    a: [f32; 4],
    b: [f32; 4],
    c: [f32; 4],
    normal: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Piece {
    range: [u32; 4],
    low: [f32; 4],
    high: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    projection: [f32; 4],
    origin: [f32; 4],
    x: [f32; 4],
    y: [f32; 4],
    z: [f32; 4],
    camera: [f32; 4],
    banks: [f32; 4],
    low: [f32; 4],
    high: [f32; 4],
    pieces: [Piece; MULE_PIECES],
}

fn subtract(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn vector(v: Vec3) -> [f32; 4] {
    [v.x as f32, v.y as f32, v.z as f32, 0.0]
}
fn view_vector(view: FlightView, v: Vec3) -> Vec3 {
    match view {
        FlightView::Front => v,
        FlightView::Rear => Vec3::new(-v.x, v.y, -v.z),
        FlightView::Left => Vec3::new(v.z, v.y, -v.x),
        FlightView::Right => Vec3::new(-v.z, v.y, v.x),
    }
}
fn valid_pose(p: RigidPose, tolerance: f64) -> bool {
    let b = p.orientation;
    [p.position, b.x, b.y, b.z].iter().all(|v| {
        [v.x, v.y, v.z]
            .iter()
            .all(|a| a.is_finite() && a.abs() <= 1e12)
    }) && b.is_valid_rotation()
        && [b.x, b.y, b.z]
            .iter()
            .all(|v| (v.dot(*v) - 1.0).abs() < tolerance)
        && b.x.dot(b.y).abs() < tolerance
        && b.x.dot(b.z).abs() < tolerance
        && b.y.dot(b.z).abs() < tolerance
        && (b.x.x * (b.y.y * b.z.z - b.y.z * b.z.y) - b.x.y * (b.y.x * b.z.z - b.y.z * b.z.x)
            + b.x.z * (b.y.x * b.z.y - b.y.y * b.z.x)
            - 1.0)
            .abs()
            < tolerance
}

/// Source-convex boundary certificate: at most ten micrometres Hausdorff error.
/// The 2cm ball is analytic, never polygonized. Encoding adds <=4 micrometres per coordinate.
fn supports(
    presentation: &MulePresentation,
) -> Result<(Vec<Triangle>, [Piece; MULE_PIECES]), &'static str> {
    let source = MuleHullSource::bundled().map_err(|_| "SH001 immutable source unavailable")?;
    if presentation.pieces.as_ref() != source.pieces()
        || presentation.revision != MULE_PRESENTATION_REVISION
    {
        return Err("SH001 presentation/contact source mismatch");
    }
    support_mesh(presentation.pieces.as_ref(), MAX_POINTS)
}

fn support_mesh<const N: usize>(
    source: &[meridian_core::mule::SmoothPiece; N],
    expected_points: usize,
) -> Result<(Vec<Triangle>, [Piece; N]), &'static str> {
    let mut points = Vec::new();
    let mut source_count = 0;
    let mut pieces = [Piece {
        range: [0; 4],
        low: [0.0; 4],
        high: [0.0; 4],
    }; N];
    for (index, piece) in source.iter().enumerate() {
        let start = points.len();
        source_count += piece.source_points().len();
        for point in piece.source_points() {
            let encoded = vector(*point);
            if encoded[..3]
                .iter()
                .zip([point.x, point.y, point.z])
                .any(|(a, b)| !a.is_finite() || (f64::from(*a) - b).abs() > 0.000004)
            {
                return Err("Hull support precision exceeds four micrometres");
            }
        }
        let hull = rounded::build(piece.source_points())?;
        for face in hull.faces {
            let [a, b, c] = face.0.map(|i| piece.source_points()[i]);
            points.push(triangle(a, b, c)?);
        }
        let (low, high) = piece.source_bounds();
        pieces[index] = Piece {
            range: [start as u32, (points.len() - start) as u32, 0, 0],
            low: vector(low),
            high: vector(high),
        };
        pieces[index].low[3] = piece.rounding_metres() as f32;
        pieces[index].high[3] = hull.error_metres as f32;
    }
    if points.len() > 2 * expected_points - 4 * N {
        return Err("Hull total render facet bound exceeded");
    }
    if source_count != expected_points {
        return Err("Hull support point count changed");
    }
    Ok((points, pieces))
}

fn triangle(a: Vec3, b: Vec3, c: Vec3) -> Result<Triangle, &'static str> {
    let ab = subtract(b, a);
    let ac = subtract(c, a);
    let n = Vec3::new(
        ab.y * ac.z - ab.z * ac.y,
        ab.z * ac.x - ab.x * ac.z,
        ab.x * ac.y - ab.y * ac.x,
    );
    let length = n.dot(n).sqrt();
    if !length.is_finite() || length < 1e-12 {
        return Err("SH001 degenerate render facet");
    }
    let normal = vector(Vec3::new(n.x / length, n.y / length, n.z / length));
    Ok(Triangle {
        a: vector(a),
        b: vector(b),
        c: vector(c),
        normal,
    })
}

#[derive(Clone, Copy)]
pub struct View {
    pub camera: RigidPose,
    pub flight: FlightView,
    pub projection: [f32; 4],
    pub tick: u64,
}
fn uniforms(
    scene: Scene<'_>,
    view: View,
    srgb: bool,
    pieces: [Piece; MULE_PIECES],
) -> Result<Uniforms, &'static str> {
    let actor = scene.actor;
    let envelope = scene.envelope;
    // Identity and resources are borrowed, never allocated or modified by this renderer.
    // The current SH001 contract has 240.00 units per bank; reject a mismatched snapshot.
    if view.camera != scene.observer
        || actor.hull_id != "SH001"
        || envelope.revision != MULE_PRESENTATION_REVISION
        || !valid_pose(actor.pose, 1e-10)
        || envelope.front_shield_fraction != f32::from(actor.front_shield_hundredths) / 24000.0
        || envelope.rear_shield_fraction != f32::from(actor.rear_shield_hundredths) / 24000.0
        || actor.front_shield_hundredths > 24000
        || actor.rear_shield_hundredths > 24000
        || envelope.maximum_opacity != meridian_core::mule::MULE_MAX_SHIMMER_OPACITY
        || envelope.close_fade_metres != [32.0, 64.0]
    {
        return Err("SH001 actual actor/surface/view mismatch");
    }
    frame_uniforms(actor.pose, envelope, view, srgb, pieces)
}
fn frame_uniforms(
    model: RigidPose,
    envelope: &MulePresentation,
    view: View,
    srgb: bool,
    pieces: [Piece; MULE_PIECES],
) -> Result<Uniforms, &'static str> {
    let View {
        camera,
        flight,
        projection,
        tick,
    } = view;
    if !valid_pose(model, 1e-10)
        || !valid_pose(camera, 1e-5)
        || projection.iter().any(|v| !v.is_finite() || *v <= 0.0)
        || projection[3] <= projection[2]
    {
        return Err("SH001 invalid frame view");
    }
    let transform = |v| view_vector(flight, camera.orientation.inverse_coordinates(v));
    let origin = vector(transform(subtract(model.position, camera.position)));
    let local_camera = model
        .orientation
        .inverse_coordinates(subtract(camera.position, model.position));
    // Far solids still draw; ray work is limited to a local 64m neighbourhood of the envelope.
    let mut output = Uniforms {
        projection,
        origin,
        x: vector(transform(model.orientation.x)),
        y: vector(transform(model.orientation.y)),
        z: vector(transform(model.orientation.z)),
        camera: vector(local_camera),
        banks: [
            envelope.front_shield_fraction,
            envelope.rear_shield_fraction,
            envelope.maximum_opacity,
            64.0,
        ],
        low: [f32::INFINITY, f32::INFINITY, f32::INFINITY, 0.0],
        high: [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY, 0.0],
        pieces,
    };
    output.origin[3] = if srgb { 1.0 } else { 0.0 };
    output.camera[3] = (tick % 4096) as f32 / 60.0;
    for piece in &output.pieces {
        for axis in 0..3 {
            output.low[axis] = output.low[axis].min(piece.low[axis] - piece.low[3] - 0.001);
            output.high[axis] = output.high[axis].max(piece.high[axis] + piece.low[3] + 0.001);
        }
    }
    if !output
        .origin
        .iter()
        .chain(&output.camera)
        .all(|v| v.is_finite())
    {
        return Err("SH001 nonfinite camera transform");
    }
    Ok(output)
}

/// Linear-light box mips retain the original base bytes and opaque alpha.
fn mip(bytes: &[u8], side: usize) -> Vec<u8> {
    let next = side / 2;
    let mut result = vec![255; next * next * 4];
    for y in 0..next {
        for x in 0..next {
            for channel in 0..3 {
                let mut sum = 0.0f32;
                for dy in 0..2 {
                    for dx in 0..2 {
                        let v = f32::from(bytes[((2 * y + dy) * side + 2 * x + dx) * 4 + channel])
                            / 255.0;
                        sum += if v <= 0.04045 {
                            v / 12.92
                        } else {
                            ((v + 0.055) / 1.055).powf(2.4)
                        };
                    }
                }
                let v = sum / 4.0;
                let srgb = if v <= 0.0031308 {
                    12.92 * v
                } else {
                    1.055 * v.powf(1.0 / 2.4) - 0.055
                };
                result[(y * next + x) * 4 + channel] =
                    (srgb * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    result
}

/// Projects the actual packed hull using the production uniforms and the WGSL
/// solid vertex clip transform. This CPU check does not measure occlusion.
#[cfg(test)]
pub(crate) fn detailed_hull_fits_view(scene: Scene<'_>, view: View) -> Result<bool, &'static str> {
    let blank = Piece {
        range: [0; 4],
        low: [0.0; 4],
        high: [0.0; 4],
    };
    let u = uniforms(scene, view, true, [blank; MULE_PIECES])?;
    let [aspect, focal, near, far] = u.projection;
    Ok(visual()?.into_iter().all(|vertex| {
        let [x, y, z]: [f32; 3] = core::array::from_fn(|axis| {
            u.origin[axis]
                + u.x[axis] * vertex.position[0]
                + u.y[axis] * vertex.position[1]
                + u.z[axis] * vertex.position[2]
        });
        let clip_z = z * far / (far - near) - near * far / (far - near);
        z > 0.0
            && (0.0..=z).contains(&clip_z)
            && (x * focal / aspect).abs() <= 0.95 * z
            && (y * focal).abs() <= 0.95 * z
    }))
}

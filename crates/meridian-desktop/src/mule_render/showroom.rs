//! Source-authenticated Goshawk Showroom presentation. Inputs grant no World authority.
use super::{Piece, Triangle, Vec3, VisualVertex, gpu, subtract, support_mesh, vector};
use crate::docked_screens::{ModelPreview, Orientation, Rect};
use meridian_core::{
    docking::Basis3,
    goshawk::{
        GOSHAWK_DETAILED_VERTICES, GOSHAWK_NATIVE_CANDIDATE_VERTICES, GOSHAWK_PIECES,
        GoshawkHullSource,
    },
    goshawk_custom::{
        CUSTOM_DETAILED_FACES, CUSTOM_DETAILED_VERTICES, CUSTOM_NATIVE_VERTICES, CUSTOM_PIECES,
        GoshawkCustomHullSource,
    },
    goshawk_mk2::{GOSHAWK_MK2_PIECES, GoshawkMk2HullSource},
    mule::SmoothPiece,
    raven::{
        RAVEN_DETAILED_FACES, RAVEN_DETAILED_VERTICES, RAVEN_NATIVE_VERTICES, RAVEN_PIECES,
        RavenHullSource,
    },
};

pub(crate) const GEOMETRY_KEY: &str = "assets/fleet/sh008/visual.bin";
pub(crate) const MK2_GEOMETRY_KEY: &str = "assets/fleet/sh009/visual.bin";
pub(crate) const CUSTOM_GEOMETRY_KEY: &str = "assets/fleet/sh010/visual.bin";
pub(crate) const RAVEN_GEOMETRY_KEY: &str = "assets/fleet/sh015/visual.bin";
const VERTICES: usize = 3_484 * 3;
const MK2_SOURCE_POINTS: usize = 2_104 + 568;
mod visual_assets;
const EMPTY_PIECE: Piece = Piece {
    range: [0; 4],
    low: [0.0; 4],
    high: [0.0; 4],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hull {
    Explorer,
    Mk2,
    Custom,
    Raven,
}

impl Hull {
    pub(crate) const COUNT: usize = 4;

    pub(crate) fn from_model(model: &ModelPreview) -> Option<Self> {
        match (model.object.as_str(), model.geometry_key.as_str()) {
            ("SH008", GEOMETRY_KEY) => Some(Self::Explorer),
            ("SH009", MK2_GEOMETRY_KEY) => Some(Self::Mk2),
            ("SH010", CUSTOM_GEOMETRY_KEY) => Some(Self::Custom),
            ("SH015", RAVEN_GEOMETRY_KEY) => Some(Self::Raven),
            _ => None,
        }
    }

    fn geometry_key(self) -> &'static str {
        match self {
            Self::Explorer => GEOMETRY_KEY,
            Self::Mk2 => MK2_GEOMETRY_KEY,
            Self::Custom => CUSTOM_GEOMETRY_KEY,
            Self::Raven => RAVEN_GEOMETRY_KEY,
        }
    }

    fn object_id(self) -> &'static str {
        match self {
            Self::Explorer => "SH008",
            Self::Mk2 => "SH009",
            Self::Custom => "SH010",
            Self::Raven => "SH015",
        }
    }

    fn piece_count(self) -> usize {
        match self {
            Self::Explorer => GOSHAWK_PIECES,
            Self::Mk2 => GOSHAWK_MK2_PIECES,
            Self::Custom => CUSTOM_PIECES,
            Self::Raven => RAVEN_PIECES,
        }
    }

    fn uniform_size(self) -> usize {
        size_of::<Uniforms>() + self.piece_count() * size_of::<Piece>()
    }

    fn supports(self) -> Result<Supports, String> {
        match self {
            Self::Explorer => {
                let source = GoshawkHullSource::bundled().map_err(|e| e.to_string())?;
                Ok(Supports::new(
                    self,
                    source.pieces().as_ref(),
                    GOSHAWK_DETAILED_VERTICES + GOSHAWK_NATIVE_CANDIDATE_VERTICES,
                ))
            }
            Self::Mk2 => {
                let source = GoshawkMk2HullSource::bundled().map_err(|e| e.to_string())?;
                Ok(Supports::new(
                    self,
                    source.pieces().as_ref(),
                    MK2_SOURCE_POINTS,
                ))
            }
            Self::Custom => {
                let source = GoshawkCustomHullSource::bundled().map_err(|e| e.to_string())?;
                Ok(Supports::new(
                    self,
                    source.pieces(),
                    CUSTOM_DETAILED_VERTICES + CUSTOM_NATIVE_VERTICES,
                ))
            }
            Self::Raven => {
                let source = RavenHullSource::bundled().map_err(|e| e.to_string())?;
                Ok(Supports::new(
                    self,
                    source.pieces(),
                    RAVEN_DETAILED_VERTICES + RAVEN_NATIVE_VERTICES,
                ))
            }
        }
    }
}

pub(crate) fn hull(model: &ModelPreview) -> Option<Hull> {
    Hull::from_model(model)
}

pub(crate) fn matches(model: &ModelPreview) -> bool {
    hull(model).is_some()
}

fn shader(hull: Hull) -> String {
    include_str!("mule.wgsl").replacen(
        "const MULE_PIECES: u32 = 34u;",
        &format!("const MULE_PIECES: u32 = {}u;", hull.piece_count()),
        1,
    )
}

struct Supports {
    triangles: Vec<Triangle>,
    pieces: Vec<Piece>,
    shimmer: bool,
}

impl Supports {
    fn new<const N: usize>(hull: Hull, pieces: &[SmoothPiece; N], expected_points: usize) -> Self {
        match support_mesh(pieces, expected_points) {
            Ok((triangles, pieces)) => Self {
                triangles,
                pieces: pieces.to_vec(),
                shimmer: true,
            },
            Err(reason) => {
                eprintln!(
                    "Meridian {} showroom shimmer unavailable: {reason}",
                    hull.object_id()
                );
                Self {
                    triangles: vec![bytemuck::Zeroable::zeroed()],
                    pieces: vec![EMPTY_PIECE; N],
                    shimmer: false,
                }
            }
        }
    }
}

/// Exact WGSL header. The hull-specific contiguous `Piece` array immediately follows it.
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
}

struct View {
    center: Vec3,
    radius: f64,
    low: [f32; 4],
    high: [f32; 4],
}
impl View {
    fn new(vertices: &[VisualVertex], pieces: &[Piece]) -> Self {
        let mut low = [f32::INFINITY; 4];
        let mut high = [f32::NEG_INFINITY; 4];
        let mut visual_low = [f32::INFINITY; 3];
        let mut visual_high = [f32::NEG_INFINITY; 3];
        for v in vertices {
            for axis in 0..3 {
                visual_low[axis] = visual_low[axis].min(v.position[axis]);
                visual_high[axis] = visual_high[axis].max(v.position[axis]);
            }
        }
        let [cx, cy, cz] = std::array::from_fn::<_, 3, _>(|axis| {
            (f64::from(visual_low[axis]) + f64::from(visual_high[axis])) * 0.5
        });
        let center = Vec3::new(cx, cy, cz);
        let radius = vertices
            .iter()
            .map(|v| {
                let delta = subtract(point(v.position), center);
                delta.dot(delta).sqrt()
            })
            .fold(0.0, f64::max);
        for piece in pieces {
            let stand_off = piece.low[3] + 0.001;
            for axis in 0..3 {
                low[axis] = low[axis].min(piece.low[axis] - stand_off);
                high[axis] = high[axis].max(piece.high[axis] + stand_off);
            }
        }
        low[3] = 0.0;
        high[3] = 0.0;
        Self {
            center,
            radius,
            low,
            high,
        }
    }

    fn uniforms(
        &self,
        orientation: Orientation,
        aspect: f64,
        srgb: bool,
    ) -> Result<Uniforms, String> {
        if ![
            orientation.pitch,
            orientation.roll,
            orientation.yaw,
            aspect,
            self.radius,
        ]
        .into_iter()
        .all(f64::is_finite)
            || aspect <= 0.0
            || self.radius <= 0.0
        {
            return Err("Invalid Goshawk showroom view".into());
        }
        // The original bow points +Z. The Showroom's camera looks along +Z, so start
        // bow-on and retain the existing pitch, roll and yaw controls in the preview.
        let orientation = Orientation {
            yaw: orientation.yaw + std::f64::consts::PI,
            ..orientation
        };
        let rotated = |axis| {
            let [x, y, z] = crate::docked_render::rotate(axis, orientation);
            Vec3::new(x, y, z)
        };
        let basis = Basis3 {
            x: rotated([1.0, 0.0, 0.0]),
            y: rotated([0.0, 1.0, 0.0]),
            z: rotated([0.0, 0.0, 1.0]),
        };
        let distance = self.radius * 2.25;
        let focal = 0.84 * aspect.min(1.0) * (2.25_f64.powi(2) - 1.0).sqrt();
        let translation = subtract(Vec3::new(0.0, 0.0, distance), basis.apply(self.center));
        let mut origin = vector(translation);
        origin[3] = if srgb { 1.0 } else { 0.0 };
        Ok(Uniforms {
            projection: [aspect as f32, focal as f32, 0.1, 128.0],
            origin,
            x: vector(basis.x),
            y: vector(basis.y),
            z: vector(basis.z),
            camera: vector(
                basis.inverse_coordinates(subtract(Vec3::new(0.0, 0.0, 0.0), translation)),
            ),
            // Static showroom illumination. These fractions do not represent gameplay banks.
            banks: [1.0, 1.0, 0.018, 64.0],
            low: self.low,
            high: self.high,
        })
    }
}

fn point([x, y, z]: [f32; 3]) -> Vec3 {
    Vec3::new(f64::from(x), f64::from(y), f64::from(z))
}

pub(crate) struct Pass {
    hull: Hull,
    hardware: gpu::Hardware,
    view: View,
    viewport: [f32; 4],
    shimmer: bool,
    draw_count: u32,
}

impl Pass {
    pub(crate) fn for_hull(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        hull: Hull,
    ) -> Result<Self, String> {
        let vertices = visual_assets::load(hull.geometry_key())?;
        let supports = hull.supports()?;
        let draw_count =
            u32::try_from(vertices.len()).map_err(|_| "Goshawk visual count overflow")?;
        let hardware = gpu::Hardware::new(
            device,
            queue,
            format,
            gpu::Inputs {
                shader: &shader(hull),
                uniform_size: hull.uniform_size() as u64,
                points: &supports.triangles,
                visual: &vertices,
            },
        )?;
        // The pieces are immutable for the lifetime of this hull-bound pass. Only the header
        // changes with orientation and viewport, so each hull's exact pieces are uploaded once.
        queue.write_buffer(
            &hardware.uniform,
            size_of::<Uniforms>() as u64,
            bytemuck::cast_slice(&supports.pieces),
        );
        Ok(Self {
            hull,
            hardware,
            view: View::new(&vertices, &supports.pieces),
            viewport: [0.0; 4],
            shimmer: supports.shimmer,
            draw_count,
        })
    }

    pub(crate) fn hull(&self) -> Hull {
        self.hull
    }

    pub(crate) fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        model: &ModelPreview,
        bounds: Rect,
        scale: [f64; 2],
        srgb: bool,
    ) -> Result<(), String> {
        if Hull::from_model(model) != Some(self.hull) {
            return Err("Goshawk Showroom pass does not match selected hull".into());
        }
        let [x, y, width, height] = [
            bounds.x * scale[0],
            bounds.y * scale[1],
            bounds.width * scale[0],
            bounds.height * scale[1],
        ];
        let viewport = [x as f32, y as f32, width as f32, height as f32];
        if !viewport.into_iter().all(f32::is_finite)
            || viewport[0] < 0.0
            || viewport[1] < 0.0
            || viewport[2] <= 0.0
            || viewport[3] <= 0.0
        {
            return Err("Invalid Goshawk Showroom viewport".into());
        }
        let data = self
            .view
            .uniforms(model.orientation, width / height, srgb)?;
        self.viewport = viewport;
        queue.write_buffer(&self.hardware.uniform, 0, bytemuck::bytes_of(&data));
        Ok(())
    }

    pub(crate) fn draw(&self, render: &mut wgpu::RenderPass<'_>) {
        self.set_viewport(render);
        self.draw_solid(render);
        self.draw_shimmer(render);
    }

    fn set_viewport(&self, render: &mut wgpu::RenderPass<'_>) {
        let [x, y, width, height] = self.viewport;
        render.set_viewport(x, y, width, height, 0.0, 1.0);
    }

    fn draw_solid(&self, render: &mut wgpu::RenderPass<'_>) {
        render.set_pipeline(&self.hardware.solid);
        render.set_bind_group(0, &self.hardware.group, &[]);
        render.set_vertex_buffer(0, self.hardware.vertices.slice(..));
        render.draw(0..self.draw_count, 0..1);
    }

    fn draw_shimmer(&self, render: &mut wgpu::RenderPass<'_>) {
        if self.shimmer {
            render.set_pipeline(&self.hardware.shimmer);
            render.set_bind_group(0, &self.hardware.group, &[]);
            render.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod minimum_qualification;

#[cfg(test)]
mod minimum_rgb;

#[cfg(all(test, target_os = "macos"))]
mod metal_qualification;

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::naga::{self, ArraySize, TypeInner};

    fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|error| panic!("{error:?}"))
    }

    fn assert_projection(vertices: &[VisualVertex], pieces: &[Piece]) {
        let view = View::new(vertices, pieces);
        for aspect in [0.5, 1.0, 4.0] {
            let data = ok(view.uniforms(Orientation::default(), aspect, true));
            let [aspect, focal, near, far] = data.projection;
            for v in vertices {
                let [a, b, c] = v.position;
                let [x, y, z]: [f32; 3] = std::array::from_fn(|i| {
                    data.origin[i] + a * data.x[i] + b * data.y[i] + c * data.z[i]
                });
                assert!(z > near && z < far);
                assert!((x * focal / aspect).abs() <= z);
                assert!((y * focal).abs() <= z);
            }
        }
    }

    fn assert_shader_layout(hull: Hull) {
        let module = ok(naga::front::wgsl::parse_str(&shader(hull)));
        ok(naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module));
        let input = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some("Inputs"))
            .map(|(_, t)| &t.inner)
            .unwrap_or_else(|| panic!("missing shader Inputs"));
        let TypeInner::Struct { members, span } = input else {
            panic!("unexpected shader Inputs")
        };
        assert_eq!(*span as usize, hull.uniform_size());
        let pieces = members.last().unwrap_or_else(|| panic!("missing pieces"));
        assert_eq!(pieces.offset as usize, size_of::<Uniforms>());
        assert_eq!(size_of::<Uniforms>(), 9 * 16);
        let camera = &members[5];
        assert_eq!(
            camera.offset as usize,
            std::mem::offset_of!(Uniforms, camera)
        );
        let TypeInner::Array {
            size: ArraySize::Constant(count),
            stride,
            ..
        } = &module.types[pieces.ty].inner
        else {
            panic!("unexpected GPU support array");
        };
        assert_eq!(count.get() as usize, hull.piece_count());
        assert_eq!(*stride as usize, size_of::<Piece>());
    }

    fn assert_local_supports<const N: usize>(hull: Hull, original: &[SmoothPiece; N]) -> Supports {
        let supports = ok(hull.supports());
        assert!(
            supports.shimmer,
            "{hull:?} source support certification failed"
        );
        assert_eq!(supports.pieces.len(), original.len());
        assert!(!supports.triangles.is_empty());
        let mut first = 0;
        for (rendered, source) in supports.pieces.iter().zip(original) {
            assert_eq!(rendered.range[0], first);
            assert!(rendered.range[1] > 0);
            first += rendered.range[1];
            let (low, high) = source.source_bounds();
            assert_eq!(rendered.low[..3], vector(low)[..3]);
            assert_eq!(rendered.high[..3], vector(high)[..3]);
            assert_eq!(rendered.low[3], source.rounding_metres() as f32);
            assert_eq!(rendered.low[3], 0.02);
        }
        assert_eq!(first as usize, supports.triangles.len());
        supports
    }

    #[test]
    fn goshawk_original_visual_and_all_local_supports_bind_to_the_shader() {
        let hull = Hull::Explorer;
        let vertices = ok(visual_assets::load(hull.geometry_key()));
        assert_eq!(vertices.len(), 3_484 * 3);
        let source = ok(GoshawkHullSource::bundled());
        let supports = assert_local_supports(hull, source.pieces().as_ref());
        assert_projection(&vertices, &supports.pieces);
        assert_shader_layout(hull);
    }

    #[test]
    fn goshawk_mk2_original_visual_and_73_local_supports_bind_to_the_shader() {
        let hull = Hull::Mk2;
        let vertices = ok(visual_assets::load(hull.geometry_key()));
        assert_eq!(vertices.len(), 3_532 * 3);
        let source = ok(GoshawkMk2HullSource::bundled());
        let supports = assert_local_supports(hull, source.pieces().as_ref());
        assert_eq!(supports.pieces.len(), 73);
        assert_projection(&vertices, &supports.pieces);
        assert_shader_layout(hull);
    }

    #[test]
    fn custom_original_visual_and_all_local_supports_bind_to_its_own_shader() {
        let hull = Hull::Custom;
        let vertices = ok(visual_assets::load(hull.geometry_key()));
        assert_eq!(vertices.len(), CUSTOM_DETAILED_FACES * 3);
        let source = ok(GoshawkCustomHullSource::bundled());
        let supports = assert_local_supports(hull, source.pieces());
        assert_eq!(supports.pieces.len(), CUSTOM_PIECES);
        assert_projection(&vertices, &supports.pieces);
        assert_shader_layout(hull);
    }

    #[test]
    fn raven_original_visual_and_all_local_supports_bind_to_its_own_shader() {
        let hull = Hull::Raven;
        let vertices = ok(visual_assets::load(hull.geometry_key()));
        assert_eq!(vertices.len(), RAVEN_DETAILED_FACES * 3);
        let source = ok(RavenHullSource::bundled());
        let supports = assert_local_supports(hull, source.pieces());
        assert_eq!(supports.pieces.len(), RAVEN_PIECES);
        assert_projection(&vertices, &supports.pieces);
        assert_shader_layout(hull);
    }

    #[test]
    fn source_selection_requires_matching_ship_id_and_geometry_key() {
        let preview = |object: &str, geometry_key: &str| ModelPreview {
            object: object.into(),
            geometry_key: geometry_key.into(),
            bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
            orientation: Orientation::default(),
        };
        for (object, key, hull) in [
            ("SH008", GEOMETRY_KEY, Hull::Explorer),
            ("SH009", MK2_GEOMETRY_KEY, Hull::Mk2),
            ("SH010", CUSTOM_GEOMETRY_KEY, Hull::Custom),
            ("SH015", RAVEN_GEOMETRY_KEY, Hull::Raven),
        ] {
            let model = preview(object, key);
            assert_eq!(Hull::from_model(&model), Some(hull));
            assert!(matches(&model));
        }
        for model in [
            preview("SH008", MK2_GEOMETRY_KEY),
            preview("SH009", GEOMETRY_KEY),
            preview("SH025", MK2_GEOMETRY_KEY),
            preview("SH008", CUSTOM_GEOMETRY_KEY),
            preview("SH009", CUSTOM_GEOMETRY_KEY),
            preview("SH010", GEOMETRY_KEY),
            preview("SH010", MK2_GEOMETRY_KEY),
            preview("SH025", CUSTOM_GEOMETRY_KEY),
            preview("SH015", GEOMETRY_KEY),
            preview("SH015", MK2_GEOMETRY_KEY),
            preview("SH015", CUSTOM_GEOMETRY_KEY),
            preview("SH008", RAVEN_GEOMETRY_KEY),
            preview("SH009", RAVEN_GEOMETRY_KEY),
            preview("SH010", RAVEN_GEOMETRY_KEY),
        ] {
            assert_eq!(Hull::from_model(&model), None);
            assert!(!matches(&model));
        }
    }
}

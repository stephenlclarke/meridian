//! Source-authenticated Goshawk showroom presentation. Inputs grant no World authority.
use super::{
    Piece, Triangle, Vec3, VisualVertex, decode_visual, gpu, subtract, support_mesh, vector,
};
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
    mule::EnvelopeError,
    peregrine_mk_i::{
        PEREGRINE_I_DETAILED_FACES, PEREGRINE_I_DETAILED_VERTICES, PEREGRINE_I_NATIVE_VERTICES,
        PEREGRINE_I_PIECES, PeregrineMkIHullSource,
    },
    peregrine_mk3::{
        PEREGRINE_MK3_DETAILED_FACES, PEREGRINE_MK3_DETAILED_VERTICES,
        PEREGRINE_MK3_NATIVE_VERTICES, PEREGRINE_MK3_PIECES, PeregrineMk3HullSource,
    },
};

pub(crate) const GEOMETRY_KEY: &str = "assets/fleet/sh008/visual.bin";
const VISUAL: &[u8] = include_bytes!("../../../../assets/fleet/sh008/visual.bin");
const VERTICES: usize = 3_484 * 3;
const VISUAL_FNV: u64 = 0x7a17_0c0c_6cdc_d5fd;
pub(crate) const CUSTOM_GEOMETRY_KEY: &str = "assets/fleet/sh010/visual.bin";
const CUSTOM_VISUAL: &[u8] = include_bytes!("../../../../assets/fleet/sh010/visual.bin");
const CUSTOM_VISUAL_FNV: u64 = 0xfe21_3755_8ea3_8ccb;
pub(crate) const PEREGRINE_I_GEOMETRY_KEY: &str = "assets/fleet/sh022/visual.bin";
const PEREGRINE_I_VISUAL: &[u8] = include_bytes!("../../../../assets/fleet/sh022/visual.bin");
const PEREGRINE_I_VISUAL_FNV: u64 = 0x0259_dab8_59ed_15d8;
pub(crate) const PEREGRINE_III_GEOMETRY_KEY: &str = "assets/fleet/sh023/visual.bin";
const PEREGRINE_III_VISUAL: &[u8] = include_bytes!("../../../../assets/fleet/sh023/visual.bin");
const PEREGRINE_III_VISUAL_FNV: u64 = 0x09d8_c8bd_7b6a_6a63;
const EMPTY_PIECE: Piece = Piece {
    range: [0; 4],
    low: [0.0; 4],
    high: [0.0; 4],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hull {
    Explorer,
    Custom,
    PeregrineMkI,
    PeregrineMkIII,
}

pub(crate) fn hull(model: &ModelPreview) -> Option<Hull> {
    match (model.object.as_str(), model.geometry_key.as_str()) {
        ("SH008", GEOMETRY_KEY) => Some(Hull::Explorer),
        ("SH010", CUSTOM_GEOMETRY_KEY) => Some(Hull::Custom),
        ("SH022", PEREGRINE_I_GEOMETRY_KEY) => Some(Hull::PeregrineMkI),
        ("SH023", PEREGRINE_III_GEOMETRY_KEY) => Some(Hull::PeregrineMkIII),
        _ => None,
    }
}

pub(crate) fn matches(model: &ModelPreview) -> bool {
    hull(model).is_some()
}

fn authenticated_visual(
    bytes: &[u8],
    expected_hash: u64,
    magic: &[u8; 8],
    count: usize,
) -> Result<Vec<VisualVertex>, &'static str> {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    });
    if hash != expected_hash {
        return Err("Goshawk preview bytes changed");
    }
    decode_visual(bytes, magic, count)
}

fn visual() -> Result<Vec<VisualVertex>, &'static str> {
    authenticated_visual(VISUAL, VISUAL_FNV, b"GOSHVIZ1", VERTICES)
}

fn shader(piece_count: usize) -> String {
    include_str!("mule.wgsl").replacen(
        "const MULE_PIECES: u32 = 34u;",
        &format!("const MULE_PIECES: u32 = {piece_count}u;"),
        1,
    )
}

impl Hull {
    fn visual(self) -> Result<Vec<VisualVertex>, &'static str> {
        match self {
            Self::Explorer => visual(),
            Self::Custom => authenticated_visual(
                CUSTOM_VISUAL,
                CUSTOM_VISUAL_FNV,
                b"SH010VZ1",
                CUSTOM_DETAILED_FACES * 3,
            ),
            Self::PeregrineMkI => authenticated_visual(
                PEREGRINE_I_VISUAL,
                PEREGRINE_I_VISUAL_FNV,
                b"SH022VZ1",
                PEREGRINE_I_DETAILED_FACES * 3,
            ),
            Self::PeregrineMkIII => authenticated_visual(
                PEREGRINE_III_VISUAL,
                PEREGRINE_III_VISUAL_FNV,
                b"PRG3VIZ1",
                PEREGRINE_MK3_DETAILED_FACES * 3,
            ),
        }
    }

    fn piece_count(self) -> usize {
        match self {
            Self::Explorer => GOSHAWK_PIECES,
            Self::Custom => CUSTOM_PIECES,
            Self::PeregrineMkI => PEREGRINE_I_PIECES,
            Self::PeregrineMkIII => PEREGRINE_MK3_PIECES,
        }
    }

    fn supports(self) -> Result<(Vec<Triangle>, Vec<Piece>), SupportError> {
        match self {
            Self::Explorer => {
                let source = GoshawkHullSource::bundled().map_err(SupportError::Source)?;
                let (triangles, pieces) = support_mesh(
                    source.pieces().as_ref(),
                    GOSHAWK_DETAILED_VERTICES + GOSHAWK_NATIVE_CANDIDATE_VERTICES,
                )
                .map_err(SupportError::Render)?;
                Ok((triangles, pieces.to_vec()))
            }
            Self::Custom => {
                let source = GoshawkCustomHullSource::bundled().map_err(SupportError::Source)?;
                let (triangles, pieces) = support_mesh(
                    source.pieces(),
                    CUSTOM_DETAILED_VERTICES + CUSTOM_NATIVE_VERTICES,
                )
                .map_err(SupportError::Render)?;
                Ok((triangles, pieces.to_vec()))
            }
            Self::PeregrineMkI => {
                let source = PeregrineMkIHullSource::bundled().map_err(SupportError::Source)?;
                let (triangles, pieces) = support_mesh(
                    source.pieces(),
                    PEREGRINE_I_DETAILED_VERTICES + PEREGRINE_I_NATIVE_VERTICES,
                )
                .map_err(SupportError::Render)?;
                Ok((triangles, pieces.to_vec()))
            }
            Self::PeregrineMkIII => {
                let source = PeregrineMk3HullSource::bundled().map_err(SupportError::Source)?;
                let (triangles, pieces) = support_mesh(
                    source.pieces(),
                    PEREGRINE_MK3_DETAILED_VERTICES + PEREGRINE_MK3_NATIVE_VERTICES,
                )
                .map_err(SupportError::Render)?;
                Ok((triangles, pieces.to_vec()))
            }
        }
    }
}

#[derive(Debug)]
enum SupportError {
    Source(EnvelopeError),
    Render(&'static str),
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
}

struct View {
    center: Vec3,
    radius: f64,
    low: [f32; 4],
    high: [f32; 4],
}
impl View {
    fn new(vertices: &[VisualVertex], pieces: impl AsRef<[Piece]>) -> Self {
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
        for piece in pieces.as_ref() {
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
    hardware: gpu::Hardware,
    view: View,
    viewport: [f32; 4],
    shimmer: bool,
    hull: Hull,
    vertex_count: u32,
}

impl Pass {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Result<Self, String> {
        Self::construct(device, queue, format, Hull::Explorer)
    }

    pub(crate) fn for_hull(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        hull: Hull,
    ) -> Result<Self, String> {
        match hull {
            Hull::Explorer => Self::new(device, queue, format),
            Hull::Custom | Hull::PeregrineMkI | Hull::PeregrineMkIII => {
                Self::construct(device, queue, format, hull)
            }
        }
    }

    pub(crate) fn hull(&self) -> Hull {
        self.hull
    }

    fn construct(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        hull: Hull,
    ) -> Result<Self, String> {
        let vertices = hull.visual()?;
        let (triangles, pieces, shimmer) = match hull.supports() {
            Ok((triangles, pieces)) => (triangles, pieces, true),
            Err(SupportError::Source(reason)) => return Err(reason.to_string()),
            Err(SupportError::Render(reason)) => {
                eprintln!("Meridian {hull:?} showroom shimmer unavailable: {reason}");
                (
                    vec![bytemuck::Zeroable::zeroed()],
                    vec![EMPTY_PIECE; hull.piece_count()],
                    false,
                )
            }
        };
        let hardware = gpu::Hardware::new(
            device,
            queue,
            format,
            gpu::Inputs {
                shader: &shader(hull.piece_count()),
                uniform_size: (size_of::<Uniforms>() + pieces.len() * size_of::<Piece>()) as u64,
                points: &triangles,
                visual: &vertices,
            },
        )?;
        // The uniform header and all source-local support records are contiguous in WGSL.
        // Support data is immutable for this pass, so write it only when constructing the asset.
        queue.write_buffer(
            &hardware.uniform,
            size_of::<Uniforms>() as u64,
            bytemuck::cast_slice(&pieces),
        );
        Ok(Self {
            hardware,
            view: View::new(&vertices, &pieces),
            viewport: [0.0; 4],
            shimmer,
            hull,
            vertex_count: vertices.len() as u32,
        })
    }

    pub(crate) fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        model: &ModelPreview,
        bounds: Rect,
        scale: [f64; 2],
        srgb: bool,
    ) -> Result<(), String> {
        if hull(model) != Some(self.hull) {
            return Err("Goshawk Showroom hull mismatch".into());
        }
        let [x, y, width, height] = [
            bounds.x * scale[0],
            bounds.y * scale[1],
            bounds.width * scale[0],
            bounds.height * scale[1],
        ];
        let data = self
            .view
            .uniforms(model.orientation, width / height, srgb)?;
        self.viewport = [x as f32, y as f32, width as f32, height as f32];
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
        render.draw(0..self.vertex_count, 0..1);
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

    #[test]
    fn goshawk_original_visual_and_all_local_supports_bind_to_the_shader() {
        check_variant(Hull::Explorer, 3484, GOSHAWK_PIECES);
    }

    #[test]
    fn custom_original_visual_and_all_local_supports_bind_to_its_own_shader() {
        check_variant(Hull::Custom, CUSTOM_DETAILED_FACES, CUSTOM_PIECES);
        assert!(authenticated_visual(CUSTOM_VISUAL, VISUAL_FNV, b"GOSHVIZ1", VERTICES).is_err());
        assert!(
            authenticated_visual(
                VISUAL,
                CUSTOM_VISUAL_FNV,
                b"SH010VZ1",
                CUSTOM_DETAILED_FACES * 3
            )
            .is_err()
        );
        let mut model = ModelPreview {
            object: "SH010".into(),
            geometry_key: CUSTOM_GEOMETRY_KEY.into(),
            bounds: Rect::new(0.0, 0.0, 256.0, 256.0),
            orientation: Orientation::default(),
        };
        assert_eq!(hull(&model), Some(Hull::Custom));
        model.geometry_key = GEOMETRY_KEY.into();
        assert!(!matches(&model));
        model.object = "SH008".into();
        assert_eq!(hull(&model), Some(Hull::Explorer));
        model.geometry_key = CUSTOM_GEOMETRY_KEY.into();
        assert!(!matches(&model));
    }

    #[test]
    fn peregrine_mk_i_original_visual_and_all_local_supports_bind_to_its_own_shader() {
        check_variant(
            Hull::PeregrineMkI,
            PEREGRINE_I_DETAILED_FACES,
            PEREGRINE_I_PIECES,
        );
        assert!(
            authenticated_visual(PEREGRINE_I_VISUAL, VISUAL_FNV, b"GOSHVIZ1", VERTICES).is_err()
        );
        assert!(
            authenticated_visual(
                PEREGRINE_I_VISUAL,
                CUSTOM_VISUAL_FNV,
                b"SH010VZ1",
                CUSTOM_DETAILED_FACES * 3
            )
            .is_err()
        );
        assert!(
            authenticated_visual(
                CUSTOM_VISUAL,
                PEREGRINE_I_VISUAL_FNV,
                b"SH022VZ1",
                PEREGRINE_I_DETAILED_FACES * 3
            )
            .is_err()
        );
        assert!(
            authenticated_visual(
                VISUAL,
                PEREGRINE_I_VISUAL_FNV,
                b"SH022VZ1",
                PEREGRINE_I_DETAILED_FACES * 3
            )
            .is_err()
        );
        let mut model = ModelPreview {
            object: "SH022".into(),
            geometry_key: PEREGRINE_I_GEOMETRY_KEY.into(),
            bounds: Rect::new(0.0, 0.0, 256.0, 256.0),
            orientation: Orientation::default(),
        };
        assert_eq!(hull(&model), Some(Hull::PeregrineMkI));
        model.geometry_key = GEOMETRY_KEY.into();
        assert!(!matches(&model));
        model.object = "SH008".into();
        assert_eq!(hull(&model), Some(Hull::Explorer));
        model.geometry_key = PEREGRINE_I_GEOMETRY_KEY.into();
        assert!(!matches(&model));
        model.object = "SH010".into();
        assert!(!matches(&model));
        model.geometry_key = CUSTOM_GEOMETRY_KEY.into();
        assert_eq!(hull(&model), Some(Hull::Custom));
        model.object = "SH022".into();
        assert!(!matches(&model));
        model.geometry_key = PEREGRINE_III_GEOMETRY_KEY.into();
        assert!(!matches(&model));
        model.object = "SH023".into();
        assert_eq!(hull(&model), Some(Hull::PeregrineMkIII));
        model.geometry_key = PEREGRINE_I_GEOMETRY_KEY.into();
        assert!(!matches(&model));
        assert!(
            authenticated_visual(
                PEREGRINE_I_VISUAL,
                PEREGRINE_I_VISUAL_FNV,
                b"PRG3VIZ1",
                PEREGRINE_I_DETAILED_FACES * 3,
            )
            .is_err()
        );
    }

    #[test]
    fn peregrine_mk_iii_original_visual_and_all_local_supports_bind_to_its_own_shader() {
        check_variant(
            Hull::PeregrineMkIII,
            PEREGRINE_MK3_DETAILED_FACES,
            PEREGRINE_MK3_PIECES,
        );
    }

    #[test]
    fn peregrine_mk_i_and_mk_iii_refuse_actual_crossed_visual_payloads() {
        let mk3 = PEREGRINE_III_VISUAL;
        const MK3_FNV: u64 = PEREGRINE_III_VISUAL_FNV;
        const MK3_MAGIC: &[u8; 8] = b"PRG3VIZ1";
        const MK3_VERTICES: usize = PEREGRINE_MK3_DETAILED_FACES * 3;
        assert_eq!(ok(Hull::PeregrineMkIII.visual()).len(), MK3_VERTICES);
        assert_eq!(
            ok(Hull::PeregrineMkI.visual()).len(),
            PEREGRINE_I_DETAILED_FACES * 3
        );
        assert!(
            authenticated_visual(
                mk3,
                PEREGRINE_I_VISUAL_FNV,
                b"SH022VZ1",
                PEREGRINE_I_DETAILED_FACES * 3
            )
            .is_err()
        );
        assert!(
            authenticated_visual(PEREGRINE_I_VISUAL, MK3_FNV, MK3_MAGIC, MK3_VERTICES).is_err()
        );
        // Even with each payload's real fingerprint, the other original's header must refuse.
        assert!(
            authenticated_visual(mk3, MK3_FNV, b"SH022VZ1", PEREGRINE_I_DETAILED_FACES * 3)
                .is_err()
        );
        assert!(
            authenticated_visual(
                PEREGRINE_I_VISUAL,
                PEREGRINE_I_VISUAL_FNV,
                MK3_MAGIC,
                MK3_VERTICES
            )
            .is_err()
        );
        let native = include_bytes!("../../../../assets/fleet/sh023/native-visual.bin");
        assert!(authenticated_visual(native, MK3_FNV, MK3_MAGIC, MK3_VERTICES).is_err());
        let source_i = include_bytes!("../../../../assets/fleet/sh022/source.vertices");
        let membership_i =
            include_bytes!("../../../../assets/fleet/sh022/source-piece-membership.bin");
        let source_iii = include_bytes!("../../../../assets/fleet/sh023/source.vertices");
        let membership_iii =
            include_bytes!("../../../../assets/fleet/sh023/source-piece-membership.bin");
        assert!(PeregrineMkIHullSource::from_bytes(source_iii, membership_iii).is_err());
        assert!(PeregrineMk3HullSource::from_bytes(source_i, membership_i).is_err());
        assert!(PeregrineMkIHullSource::from_bytes(source_i, membership_iii).is_err());
        assert!(PeregrineMk3HullSource::from_bytes(source_iii, membership_i).is_err());
    }

    fn check_variant(hull: Hull, face_count: usize, piece_count: usize) {
        let vertices = ok(hull.visual());
        assert_eq!(vertices.len(), face_count * 3);
        let (triangles, pieces) = ok(hull.supports());
        assert_eq!(pieces.len(), piece_count);
        assert!(!triangles.is_empty());
        assert!(
            pieces
                .iter()
                .all(|piece| piece.range[1] > 0 && piece.low[3] == 0.02)
        );
        let view = View::new(&vertices, pieces);
        for aspect in [0.5, 1.0, 4.0] {
            let data = ok(view.uniforms(Orientation::default(), aspect, true));
            let [aspect, focal, near, far] = data.projection;
            for v in &vertices {
                let [a, b, c] = v.position;
                let [x, y, z]: [f32; 3] = std::array::from_fn(|i| {
                    data.origin[i] + a * data.x[i] + b * data.y[i] + c * data.z[i]
                });
                assert!(z > near && z < far);
                assert!((x * focal / aspect).abs() <= z);
                assert!((y * focal).abs() <= z);
            }
        }
        let module = ok(naga::front::wgsl::parse_str(&shader(piece_count)));
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
        assert_eq!(
            *span as usize,
            size_of::<Uniforms>() + piece_count * size_of::<Piece>()
        );
        let pieces = members.last().unwrap_or_else(|| panic!("missing pieces"));
        assert_eq!(pieces.offset as usize, size_of::<Uniforms>());
        let TypeInner::Array {
            size: ArraySize::Constant(count),
            stride,
            ..
        } = &module.types[pieces.ty].inner
        else {
            panic!("unexpected GPU support array");
        };
        assert_eq!(count.get() as usize, piece_count);
        assert_eq!(*stride as usize, size_of::<Piece>());
    }
}

//! Source-authenticated SH008 showroom presentation. Inputs grant no World authority.
use super::{Piece, Vec3, VisualVertex, decode_visual, gpu, subtract, support_mesh, vector};
use crate::docked_screens::{ModelPreview, Orientation, Rect};
use meridian_core::{
    docking::Basis3,
    goshawk::{
        GOSHAWK_DETAILED_VERTICES, GOSHAWK_NATIVE_CANDIDATE_VERTICES, GOSHAWK_PIECES,
        GoshawkHullSource,
    },
};

pub(crate) const GEOMETRY_KEY: &str = "assets/fleet/sh008/visual.bin";
const VISUAL: &[u8] = include_bytes!("../../../../assets/fleet/sh008/visual.bin");
const VERTICES: usize = 3_484 * 3;
const VISUAL_FNV: u64 = 0x7a17_0c0c_6cdc_d5fd;
const EMPTY_PIECE: Piece = Piece {
    range: [0; 4],
    low: [0.0; 4],
    high: [0.0; 4],
};

pub(crate) fn matches(model: &ModelPreview) -> bool {
    model.object == "SH008" && model.geometry_key == GEOMETRY_KEY
}

fn visual() -> Result<Vec<VisualVertex>, &'static str> {
    let hash = VISUAL.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    });
    if hash != VISUAL_FNV {
        return Err("SH008 preview bytes changed");
    }
    decode_visual(VISUAL, b"GOSHVIZ1", VERTICES)
}

fn shader() -> String {
    include_str!("mule.wgsl").replacen(
        "const MULE_PIECES: u32 = 34u;",
        &format!("const MULE_PIECES: u32 = {GOSHAWK_PIECES}u;"),
        1,
    )
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
    // The 69 GPU pieces occupy three contiguous Pod arrays of 23.
    pieces: [[Piece; 23]; 3],
}

struct View {
    center: Vec3,
    radius: f64,
    pieces: [Piece; GOSHAWK_PIECES],
    low: [f32; 4],
    high: [f32; 4],
}
impl View {
    fn new(vertices: &[VisualVertex], pieces: [Piece; GOSHAWK_PIECES]) -> Self {
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
        for piece in &pieces {
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
            pieces,
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
            return Err("Invalid SH008 showroom view".into());
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
            pieces: std::array::from_fn(|group| {
                std::array::from_fn(|i| self.pieces[group * 23 + i])
            }),
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
}

impl Pass {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Result<Self, String> {
        let vertices = visual()?;
        let source = GoshawkHullSource::bundled().map_err(|e| e.to_string())?;
        let (triangles, pieces, shimmer) = match support_mesh(
            source.pieces().as_ref(),
            GOSHAWK_DETAILED_VERTICES + GOSHAWK_NATIVE_CANDIDATE_VERTICES,
        ) {
            Ok((triangles, pieces)) => (triangles, pieces, true),
            Err(reason) => {
                eprintln!("Meridian Goshawk showroom shimmer unavailable: {reason}");
                (
                    vec![bytemuck::Zeroable::zeroed()],
                    [EMPTY_PIECE; GOSHAWK_PIECES],
                    false,
                )
            }
        };
        let hardware = gpu::Hardware::new(
            device,
            queue,
            format,
            gpu::Inputs {
                shader: &shader(),
                uniform_size: size_of::<Uniforms>() as u64,
                points: &triangles,
                visual: &vertices,
            },
        )?;
        Ok(Self {
            hardware,
            view: View::new(&vertices, pieces),
            viewport: [0.0; 4],
            shimmer,
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
        render.draw(0..VERTICES as u32, 0..1);
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
        let vertices = ok(visual());
        assert_eq!(vertices.len(), 3_484 * 3);
        let source = ok(GoshawkHullSource::bundled());
        let (triangles, pieces) = ok(support_mesh(source.pieces().as_ref(), 2608));
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
        let module = ok(naga::front::wgsl::parse_str(&shader()));
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
        assert_eq!(*span as usize, size_of::<Uniforms>());
        let pieces = members.last().unwrap_or_else(|| panic!("missing pieces"));
        assert_eq!(
            pieces.offset as usize,
            std::mem::offset_of!(Uniforms, pieces)
        );
        let TypeInner::Array {
            size: ArraySize::Constant(count),
            stride,
            ..
        } = &module.types[pieces.ty].inner
        else {
            panic!("unexpected GPU support array");
        };
        assert_eq!(count.get() as usize, GOSHAWK_PIECES);
        assert_eq!(*stride as usize, size_of::<Piece>());
    }
}

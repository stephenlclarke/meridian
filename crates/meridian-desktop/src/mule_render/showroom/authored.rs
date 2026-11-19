//! Generic detailed-face renderer for session-local hull packages only.
use super::{EMPTY_PIECE, Rect, Triangle, Uniforms, View, VisualVertex, gpu, shader};
use crate::docked_screens::ModelPreview;
use std::ops::Range;

pub(crate) struct AuthoredFace {
    pub indices: [usize; 3],
    pub rgb: [u8; 3],
    pub alpha: u8,
    pub emissive: bool,
    pub texture: Option<u32>,
    pub uv: [[f64; 2]; 3],
}

pub(crate) struct AuthoredVisual {
    vertices: Vec<VisualVertex>,
}

impl AuthoredVisual {
    /// Presentation-only uniform fit. Core collision retains its original metre-space geometry.
    pub(crate) fn from_geometry(
        vertices: &[[f64; 3]],
        faces: impl IntoIterator<Item = AuthoredFace>,
    ) -> Result<(Self, [f64; 3]), String> {
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for point in vertices {
            if point.iter().any(|n| !n.is_finite()) {
                return Err("Nonfinite authored visual point".into());
            }
            for axis in 0..3 {
                low[axis] = low[axis].min(point[axis]);
                high[axis] = high[axis].max(point[axis]);
            }
        }
        let dimensions: [f64; 3] = std::array::from_fn(|axis| high[axis] - low[axis]);
        let extent = dimensions.into_iter().fold(0.0, f64::max);
        if !extent.is_finite()
            || extent <= 0.0
            || dimensions.into_iter().any(|n| !n.is_finite() || n < 0.0)
        {
            return Err("Authored visual has no finite extent".into());
        }
        let center: [f64; 3] = std::array::from_fn(|axis| low[axis] + dimensions[axis] * 0.5);
        let fitted: Vec<[f64; 3]> = vertices
            .iter()
            .map(|point| std::array::from_fn(|axis| (point[axis] - center[axis]) / extent * 32.0))
            .collect();
        let mut corners = Vec::new();
        for face in faces {
            let [Some(a), Some(b), Some(c)] = face.indices.map(|index| fitted.get(index).copied())
            else {
                return Err("Authored visual face index is invalid".into());
            };
            let normal = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| -> Option<[f64; 3]> {
                let [x, y, z] = std::array::from_fn(|i| b[i] - a[i]);
                let [u, v, w] = std::array::from_fn(|i| c[i] - a[i]);
                let n = [y * w - z * v, z * u - x * w, x * v - y * u];
                let length = n[0].hypot(n[1]).hypot(n[2]);
                (length.is_finite() && length > 0.0).then(|| n.map(|axis| axis / length))
            };
            let normal64 = normal(a, b, c).ok_or("Authored visual face has no finite normal")?;
            let points = [a, b, c].map(|p| p.map(|n| n as f32));
            let [a32, b32, c32] = points.map(|p| p.map(f64::from));
            if normal(a32, b32, c32).is_none() {
                return Err("Authored visual face exceeds renderer coordinate precision".into());
            }
            if face.texture.is_some_and(|layer| layer >= 3) {
                return Err("Unknown authored texture atlas layer".into());
            }
            let uv = face.uv.map(|p| p.map(|n| n as f32));
            if !uv.iter().flatten().all(|n| n.is_finite()) {
                return Err("Authored UV exceeds renderer precision".into());
            }
            let [r, g, b] = face.rgb;
            let color = [r, g, b, face.alpha].map(|n| f32::from(n) / 255.0);
            for (position, uv) in points.into_iter().zip(uv) {
                corners.push(VisualVertex {
                    position,
                    normal: normal64.map(|n| n as f32),
                    color,
                    uv,
                    texture: face.texture.unwrap_or(u32::MAX),
                    emissive: u32::from(face.emissive),
                });
            }
        }
        if corners.is_empty() {
            return Err("Authored visual has no faces".into());
        }
        Ok((Self { vertices: corners }, dimensions))
    }

    #[cfg(test)]
    pub(crate) fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.vertices)
    }
}

fn authored_shader() -> String {
    format!("{}\n{}", shader(1), include_str!("authored.wgsl"))
}

pub(crate) struct AuthoredPass {
    hardware: gpu::Hardware,
    view: View,
    viewport: [f32; 4],
    object: String,
    key: String,
    opaque: Vec<Range<u32>>,
    transparent: Vec<([f32; 3], u32)>,
}

impl AuthoredPass {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        preview: &ModelPreview,
        visual: &AuthoredVisual,
    ) -> Result<Self, String> {
        let vertices = &visual.vertices;
        let length =
            u32::try_from(vertices.len()).map_err(|_| "Authored preview exceeds draw count")?;
        if u64::from(length) * size_of::<VisualVertex>() as u64 > device.limits().max_buffer_size {
            return Err("Authored preview exceeds device vertex buffer limit".into());
        }
        let dummy: [Triangle; 1] = [bytemuck::Zeroable::zeroed(); 1];
        let pieces = [EMPTY_PIECE; 1];
        let hardware = gpu::Hardware::new(
            device,
            queue,
            format,
            gpu::Inputs {
                shader: &authored_shader(),
                uniform_size: (size_of::<Uniforms>() + size_of::<super::Piece>()) as u64,
                points: &dummy,
                visual: vertices,
                authored_materials: true,
            },
        )?;
        queue.write_buffer(
            &hardware.uniform,
            size_of::<Uniforms>() as u64,
            bytemuck::cast_slice(&pieces),
        );
        let mut opaque: Vec<Range<u32>> = Vec::new();
        let mut transparent = Vec::new();
        for (i, face) in vertices.as_chunks::<3>().0.iter().enumerate() {
            let start = (i * 3) as u32;
            if face[0].color[3] == 1.0 {
                if let Some(run) = opaque.last_mut().filter(|run| run.end == start) {
                    run.end += 3;
                } else {
                    opaque.push(start..start + 3);
                }
            } else if face[0].color[3] > 0.0 {
                let center = std::array::from_fn(|axis| {
                    face.iter().map(|corner| corner.position[axis]).sum::<f32>() / 3.0
                });
                transparent.push((center, start));
            }
        }
        Ok(Self {
            hardware,
            view: View::new(vertices, pieces),
            viewport: [0.0; 4],
            object: preview.object.clone(),
            key: preview.geometry_key.clone(),
            opaque,
            transparent,
        })
    }

    pub(crate) fn matches(&self, preview: &ModelPreview) -> bool {
        self.object == preview.object && self.key == preview.geometry_key
    }

    pub(crate) fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        preview: &ModelPreview,
        bounds: Rect,
        scale: [f64; 2],
        srgb: bool,
    ) -> Result<(), String> {
        if !self.matches(preview) {
            return Err("Authored Showroom identity mismatch".into());
        }
        let [x, y, width, height] = [
            bounds.x * scale[0],
            bounds.y * scale[1],
            bounds.width * scale[0],
            bounds.height * scale[1],
        ];
        if ![x, y, width, height].into_iter().all(f64::is_finite) || width <= 0.0 || height <= 0.0 {
            return Err("Invalid authored Showroom viewport".into());
        }
        let uniforms = self
            .view
            .uniforms(preview.orientation, width / height, srgb)?;
        self.viewport = [x, y, width, height].map(|n| n as f32);
        queue.write_buffer(&self.hardware.uniform, 0, bytemuck::bytes_of(&uniforms));
        let depth = |center: [f32; 3]| {
            uniforms.x[2] * center[0] + uniforms.y[2] * center[1] + uniforms.z[2] * center[2]
        };
        self.transparent
            .sort_by(|(a, ai), (b, bi)| depth(*b).total_cmp(&depth(*a)).then(ai.cmp(bi)));
        Ok(())
    }

    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        let [x, y, w, h] = self.viewport;
        pass.set_viewport(x, y, w, h, 0.0, 1.0);
        pass.set_pipeline(&self.hardware.solid);
        pass.set_bind_group(0, &self.hardware.group, &[]);
        pass.set_vertex_buffer(0, self.hardware.vertices.slice(..));
        for range in &self.opaque {
            pass.draw(range.clone(), 0..1);
        }
        // Approximate face-centre sorting for partial alpha, after opaque depth is established.
        for &(_, start) in &self.transparent {
            pass.draw(start..start + 3, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::naga::{
        self,
        valid::{Capabilities, ValidationFlags, Validator},
    };

    #[test]
    fn authored_shader_and_corner_material_attributes_are_portable() {
        let module = naga::front::wgsl::parse_str(&authored_shader())
            .unwrap_or_else(|error| panic!("{error:?}"));
        Validator::new(ValidationFlags::all(), Capabilities::empty())
            .validate(&module)
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert!(
            module
                .entry_points
                .iter()
                .any(|entry| entry.name == "authored_fragment")
        );
        let (visual, dims) = AuthoredVisual::from_geometry(
            &[[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            [
                AuthoredFace {
                    indices: [0, 1, 2],
                    rgb: [64, 128, 255],
                    alpha: 180,
                    emissive: true,
                    texture: Some(2),
                    uv: [[0.0, 0.0], [3.0, -2.0], [0.5, 1.0]],
                },
                AuthoredFace {
                    indices: [2, 1, 0],
                    rgb: [255, 0, 0],
                    alpha: 255,
                    emissive: false,
                    texture: None,
                    uv: [[0.0; 2]; 3],
                },
            ],
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(dims, [2.0, 1.0, 0.0]);
        assert_eq!(
            visual.vertices[0].color,
            [64.0 / 255.0, 128.0 / 255.0, 1.0, 180.0 / 255.0]
        );
        assert_eq!(visual.vertices[1].uv, [3.0, -2.0]);
        assert_eq!(visual.vertices[0].texture, 2);
        assert_eq!(visual.vertices[0].emissive, 1);
        assert_eq!(visual.vertices[3].texture, u32::MAX);
        assert_eq!(visual.vertices[3].normal, [0.0, 0.0, -1.0]);
    }
}

//! Generic detailed-face renderer for session-local hull packages only.
use super::{EMPTY_PIECE, Rect, Triangle, Uniforms, View, VisualVertex, gpu, shader};
use crate::docked_screens::ModelPreview;
use meridian_core::hull_package::PreviewFit;
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
        let fit = PreviewFit::new(vertices)?;
        let mut corners = Vec::new();
        for face in faces {
            let triangle = fit.triangle(face.indices)?;
            if face.texture.is_some_and(|layer| layer >= 3) {
                return Err("Unknown authored texture atlas layer".into());
            }
            let uv = face.uv.map(|p| p.map(|n| n as f32));
            if !uv.iter().flatten().all(|n| n.is_finite()) {
                return Err("Authored UV exceeds renderer precision".into());
            }
            let [r, g, b] = face.rgb;
            let color = [r, g, b, face.alpha].map(|n| f32::from(n) / 255.0);
            for (position, uv) in triangle.positions.into_iter().zip(uv) {
                corners.push(VisualVertex {
                    position,
                    normal: triangle.normal,
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
        Ok((Self { vertices: corners }, fit.dimensions()))
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

    #[test]
    fn authored_encoder_refuses_same_f32_collapsed_triangle_as_core_fit() {
        let points = [[0., 0., 0.], [1e-10, 0., 0.], [0., 1e-10, 0.], [1., 1., 1.]];
        let fit = PreviewFit::new(&points).unwrap_or_else(|error| panic!("{error}"));
        let expected = fit
            .triangle([0, 1, 2])
            .err()
            .unwrap_or_else(|| panic!("collapsed shared fit accepted"));
        let result = AuthoredVisual::from_geometry(
            &points,
            [AuthoredFace {
                indices: [0, 1, 2],
                rgb: [255; 3],
                alpha: 255,
                emissive: false,
                texture: None,
                uv: [[0.; 2]; 3],
            }],
        );
        let actual = result
            .err()
            .unwrap_or_else(|| panic!("collapsed desktop geometry accepted"));
        assert_eq!(actual, expected);
    }

    #[test]
    fn stock_sparrowhawk_original_fits_actual_minimum_station_showroom_panes() {
        assert_stock_minimum("Sparrowhawk", "SH030");
    }

    #[test]
    fn stock_kite_original_fits_actual_minimum_station_showroom_panes() {
        assert_stock_minimum("Kite Interceptor", "SH031");
    }

    #[test]
    fn stock_stag_original_fits_actual_minimum_station_showroom_panes() {
        assert_stock_minimum("Stag Liner", "SH035");
    }

    fn assert_stock_minimum(name: &str, id: &str) {
        let frames = super::super::minimum_qualification::station_frames_for(name);
        for pair in frames {
            let [solo] = pair.solo.models.as_slice() else {
                panic!("expected ordinary solo pane");
            };
            let [baseline, selected] = pair.comparison.models.as_slice() else {
                panic!("expected ordinary Compare panes");
            };
            assert_eq!(baseline.object, "SH025");
            assert_eq!(solo.orientation, selected.orientation);
            for model in [solo, selected] {
                assert_eq!(model.object, id);
                let visual = crate::authored_hull::stock_visual_for(model)
                    .unwrap_or_else(|error| panic!("{error}"))
                    .unwrap_or_else(|| panic!("missing stock preview"));
                let view = View::new(&visual.vertices, [EMPTY_PIECE]);
                let bounds = model.bounds;
                let uniforms = view
                    .uniforms(model.orientation, bounds.width / bounds.height, true)
                    .unwrap_or_else(|error| panic!("{error}"));
                let [aspect, focal, near, far] = uniforms.projection;
                for vertex in &visual.vertices {
                    let [a, b, c] = vertex.position;
                    let [x, y, z]: [f32; 3] = std::array::from_fn(|i| {
                        uniforms.origin[i]
                            + a * uniforms.x[i]
                            + b * uniforms.y[i]
                            + c * uniforms.z[i]
                    });
                    assert!(z > near && z < far);
                    let pixel = [
                        bounds.x + (1.0 + f64::from(x * focal / aspect / z)) * bounds.width / 2.0,
                        bounds.y + (1.0 - f64::from(y * focal / z)) * bounds.height / 2.0,
                    ];
                    assert!(
                        pixel[0] >= bounds.x + 2.0
                            && pixel[0] <= bounds.x + bounds.width - 2.0
                            && pixel[1] >= bounds.y + 2.0
                            && pixel[1] <= bounds.y + bounds.height - 2.0
                    );
                }
            }
        }
    }
}

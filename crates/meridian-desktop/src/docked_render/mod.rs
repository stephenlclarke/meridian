//! GPU-ready docked screen geometry. Draws the exact immutable UI frame, not gameplay state.
use crate::{
    docked_screens::{AvailableModel, Frame, Ink, ModelPreview, Orientation, Rect},
    mesh::Vertex,
};
mod gpu;
pub use gpu::Pass;
const MAX_GLYPHS: usize = 8192;
const MAX_GEOMETRY: usize = 180_000;
const MAX_MODELS: usize = 2;
const MAX_MODEL_VERTICES: usize = 10452 * MAX_MODELS * 9;
const SHIP_KEY: &str = "assets/legacy/sh025.mesh";
pub const MODELS: [AvailableModel<'static>; 5] = [
    AvailableModel {
        object: "SH025",
        geometry_key: SHIP_KEY,
    },
    AvailableModel {
        object: "SH008",
        geometry_key: crate::mule_render::showroom::GEOMETRY_KEY,
    },
    AvailableModel {
        object: "SH010",
        geometry_key: crate::mule_render::showroom::CUSTOM_GEOMETRY_KEY,
    },
    AvailableModel {
        object: "SH016",
        geometry_key: crate::mule_render::showroom::MOLE_MINER_GEOMETRY_KEY,
    },
    AvailableModel {
        object: "SH017",
        geometry_key: crate::mule_render::showroom::PROSPECTOR_GEOMETRY_KEY,
    },
];
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextVertex {
    position: [f32; 2],
    uv: [f32; 2],
    colour: [f32; 3],
}
struct Model {
    vertices: Vec<Vertex>,
    center: [f64; 3],
    radius: f64,
}
impl Model {
    fn sh025() -> Result<Self, String> {
        let bytes = include_bytes!("../../../../assets/legacy/sh025.mesh");
        let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        });
        if bytes.len() != 250848 || hash != SHIP_FNV {
            return Err("SH025 preview bytes do not match the approved asset".into());
        }
        let vertices = crate::mesh::load_ship()?;
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for v in &vertices {
            for i in 0..3 {
                low[i] = low[i].min(f64::from(v.position[i]));
                high[i] = high[i].max(f64::from(v.position[i]));
            }
        }
        let center = std::array::from_fn(|i| (low[i] + high[i]) / 2.0);
        let radius = vertices
            .iter()
            .map(|v| {
                (0..3)
                    .map(|i| (f64::from(v.position[i]) - center[i]).powi(2))
                    .sum::<f64>()
                    .sqrt()
            })
            .fold(0.0, f64::max);
        if !radius.is_finite() || radius <= 0.0 {
            return Err("invalid SH025 preview bounds".into());
        }
        Ok(Self {
            vertices,
            center,
            radius,
        })
    }
    fn matches(&self, p: &ModelPreview) -> bool {
        p.object == MODELS[0].object && p.geometry_key == MODELS[0].geometry_key
    }
}
// Exact compiled asset binding; SHA256/provenance is retained in assets/legacy/manifest.json.
const SHIP_FNV: u64 = 7713003236297754695;
fn valid(r: Rect) -> bool {
    [r.x, r.y, r.width, r.height, r.x + r.width, r.y + r.height]
        .into_iter()
        .all(f64::is_finite)
        && r.width > 0.0
        && r.height > 0.0
}
fn intersection(a: Rect, b: Rect) -> Option<Rect> {
    if !valid(a) || !valid(b) {
        return None;
    }
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    (right > x && bottom > y).then(|| Rect::new(x, y, right - x, bottom - y))
}
fn colour(ink: Ink, srgb: bool) -> [f32; 3] {
    let rgba = ink.rgba();
    [24, 16, 8].map(|shift| {
        let v = ((rgba >> shift) & 255) as f32 / 255.0;
        if !srgb {
            v
        } else if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    })
}
fn position(p: [f64; 2], size: [f64; 2]) -> [f32; 2] {
    [
        (2.0 * p[0] / size[0] - 1.0) as f32,
        (1.0 - 2.0 * p[1] / size[1]) as f32,
    ]
}
#[derive(Clone, Copy)]
struct ClipVertex {
    p: [f64; 3],
    c: [f32; 3],
}
fn triangle(out: &mut Vec<Vertex>, input: [ClipVertex; 3], clip: Rect, size: [f64; 2]) {
    let mut a = [input[0]; 16];
    let mut b = a;
    a[..3].copy_from_slice(&input);
    let mut count = 3;
    for (axis, edge, sign) in [
        (0, clip.x, 1.0),
        (0, clip.x + clip.width, -1.0),
        (1, clip.y, 1.0),
        (1, clip.y + clip.height, -1.0),
    ] {
        if count == 0 {
            return;
        }
        let mut next = 0;
        let mut prev = a[count - 1];
        let mut pd = (prev.p[axis] - edge) * sign;
        for cur in a.iter().take(count).copied() {
            let cd = (cur.p[axis] - edge) * sign;
            if (pd >= 0.0) != (cd >= 0.0) {
                let t = pd / (pd - cd);
                b[next] = ClipVertex {
                    p: std::array::from_fn(|i| prev.p[i] + t * (cur.p[i] - prev.p[i])),
                    c: std::array::from_fn(|i| prev.c[i] + t as f32 * (cur.c[i] - prev.c[i])),
                };
                next += 1;
            }
            if cd >= 0.0 {
                b[next] = cur;
                next += 1;
            }
            prev = cur;
            pd = cd;
        }
        std::mem::swap(&mut a, &mut b);
        count = next;
    }
    for i in 1..count.saturating_sub(1) {
        for v in [a[0], a[i], a[i + 1]] {
            let [x, y] = position([v.p[0], v.p[1]], size);
            out.push(Vertex {
                position: [x, y, v.p[2] as f32],
                color: v.c,
            });
        }
    }
}
fn disk(
    out: &mut Vec<Vertex>,
    centre: [f64; 2],
    radius: f64,
    clip: Rect,
    c: [f32; 3],
    size: [f64; 2],
) {
    if !radius.is_finite() || radius <= 0.0 || !centre.into_iter().all(f64::is_finite) {
        return;
    }
    for i in 0..32 {
        let point = |n| {
            let a = n as f64 * std::f64::consts::TAU / 32.0;
            ClipVertex {
                p: [
                    centre[0] + radius * a.cos(),
                    centre[1] + radius * a.sin(),
                    0.0,
                ],
                c,
            }
        };
        triangle(
            out,
            [
                ClipVertex {
                    p: [centre[0], centre[1], 0.0],
                    c,
                },
                point(i),
                point(i + 1),
            ],
            clip,
            size,
        );
    }
}
fn clip_segment(mut a: [f64; 2], mut b: [f64; 2], clip: Rect) -> Option<([f64; 2], [f64; 2])> {
    if !valid(clip) || !a.into_iter().chain(b).all(f64::is_finite) {
        return None;
    }
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    if !dx.is_finite() || !dy.is_finite() {
        return None;
    }
    let mut low: f64 = 0.0;
    let mut high: f64 = 1.0;
    for (p, q) in [
        (-dx, a[0] - clip.x),
        (dx, clip.x + clip.width - a[0]),
        (-dy, a[1] - clip.y),
        (dy, clip.y + clip.height - a[1]),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                low = low.max(t);
            } else {
                high = high.min(t);
            }
            if low > high {
                return None;
            }
        }
    }
    b = [a[0] + high * dx, a[1] + high * dy];
    a = [a[0] + low * dx, a[1] + low * dy];
    Some((a, b))
}
fn stroke(
    out: &mut Vec<Vertex>,
    a: [f64; 2],
    b: [f64; 2],
    clip: Rect,
    c: [f32; 3],
    size: [f64; 2],
) {
    let Some((a, b)) = clip_segment(a, b, clip) else {
        return;
    };
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = dx.hypot(dy);
    if length <= 1e-12 {
        return;
    }
    // Keep the whole one-point stroke inside its clip, including its normal extent.
    let n = [-dy / length * 0.5, dx / length * 0.5];
    let point = |p: [f64; 2], sign: f64| {
        let [x, y] = position(
            [
                (p[0] + sign * n[0]).clamp(clip.x, clip.x + clip.width),
                (p[1] + sign * n[1]).clamp(clip.y, clip.y + clip.height),
            ],
            size,
        );
        Vertex {
            position: [x, y, 0.0],
            color: c,
        }
    };
    out.extend_from_slice(&[
        point(a, 1.0),
        point(a, -1.0),
        point(b, 1.0),
        point(b, 1.0),
        point(a, -1.0),
        point(b, -1.0),
    ]);
}
fn outline(out: &mut Vec<Vertex>, r: Rect, clip: Rect, c: [f32; 3], size: [f64; 2]) {
    for (a, b) in [
        ([r.x, r.y], [r.x + r.width, r.y]),
        ([r.x + r.width, r.y], [r.x + r.width, r.y + r.height]),
        ([r.x + r.width, r.y + r.height], [r.x, r.y + r.height]),
        ([r.x, r.y + r.height], [r.x, r.y]),
    ] {
        stroke(out, a, b, clip, c, size);
    }
}
fn ring(
    out: &mut Vec<Vertex>,
    centre: [f64; 2],
    radius: f64,
    clip: Rect,
    c: [f32; 3],
    size: [f64; 2],
) {
    if !radius.is_finite() || radius <= 0.0 || !centre.into_iter().all(f64::is_finite) {
        return;
    }
    for i in 0..96 {
        let point = |n| {
            let a = n as f64 * std::f64::consts::TAU / 96.0;
            [centre[0] + radius * a.cos(), centre[1] + radius * a.sin()]
        };
        stroke(out, point(i), point(i + 1), clip, c, size);
    }
}
fn label(
    out: &mut Vec<TextVertex>,
    l: &crate::docked_screens::Label,
    canvas: Rect,
    srgb: bool,
) -> Result<(), String> {
    let Some(clip) = intersection(l.clip, canvas) else {
        return Ok(());
    };
    if !l.scale.is_finite()
        || l.scale <= 0.0
        || l.scale > 16.0
        || !l.at.into_iter().all(f64::is_finite)
    {
        return Err("invalid docked label geometry".into());
    }
    let size = [canvas.width, canvas.height];
    let c = colour(l.ink, srgb);
    for (index, ch) in l.text.chars().enumerate() {
        if out.len() / 6 >= MAX_GLYPHS {
            return Err("docked labels exceed bounded glyph capacity".into());
        }
        if ch == ' ' {
            continue;
        }
        let code = if ch.is_ascii() {
            ch as u32
        } else {
            u32::from(b'?')
        };
        let glyph = Rect::new(
            l.at[0] + index as f64 * 6.0 * l.scale,
            l.at[1],
            5.0 * l.scale,
            7.0 * l.scale,
        );
        let Some(r) = intersection(glyph, clip) else {
            continue;
        };
        let uv = |x: f64, y: f64| {
            [
                ((f64::from(code % 16) * 5.0 + (x - glyph.x) / l.scale) / 80.0) as f32,
                ((f64::from(code / 16) * 7.0 + (y - glyph.y) / l.scale) / 56.0) as f32,
            ]
        };
        let point = |x, y| TextVertex {
            position: position([x, y], size),
            uv: uv(x, y),
            colour: c,
        };
        out.extend_from_slice(&[
            point(r.x, r.y),
            point(r.x, r.y + r.height),
            point(r.x + r.width, r.y),
            point(r.x + r.width, r.y),
            point(r.x, r.y + r.height),
            point(r.x + r.width, r.y + r.height),
        ]);
    }
    Ok(())
}
fn atlas() -> Vec<u8> {
    let mut pixels = vec![0; 80 * 56];
    for code in 0..128u8 {
        let glyph = crate::hud::glyph(char::from(code));
        for (y, row) in glyph.into_iter().enumerate() {
            for x in 0..5 {
                pixels[(usize::from(code) / 16 * 7 + y) * 80 + usize::from(code) % 16 * 5 + x] =
                    if row & (1 << (4 - x)) != 0 { 255 } else { 0 };
            }
        }
    }
    pixels
}
pub(crate) fn rotate(mut v: [f64; 3], o: Orientation) -> [f64; 3] {
    for (axis, angle) in [(1, o.yaw), (0, o.pitch), (2, o.roll)] {
        let (s, c) = angle.rem_euclid(std::f64::consts::TAU).sin_cos();
        let (i, j) = match axis {
            0 => (1, 2),
            1 => (2, 0),
            _ => (0, 1),
        };
        let a = v[i];
        let b = v[j];
        v[i] = c * a - s * b;
        v[j] = s * a + c * b;
    }
    v
}
#[derive(Default)]
struct Geometry {
    shapes: Vec<Vertex>,
    text: Vec<TextVertex>,
    models: Vec<Vertex>,
    unavailable_models: usize,
}
impl Geometry {
    fn with_capacity() -> Self {
        Self {
            shapes: Vec::with_capacity(MAX_GEOMETRY),
            text: Vec::with_capacity(MAX_GLYPHS * 6),
            models: Vec::with_capacity(MAX_MODEL_VERTICES),
            unavailable_models: 0,
        }
    }
    fn compose(
        &mut self,
        frame: &Frame,
        model: &Model,
        srgb: bool,
        planet_system: Option<meridian_core::navigation::SystemId>,
    ) -> Result<(), String> {
        self.shapes.clear();
        self.text.clear();
        self.models.clear();
        self.unavailable_models = 0;
        let canvas = frame.viewport.canvas();
        if !valid(canvas) || frame.viewport.width < 320.0 || frame.viewport.height < 240.0 {
            return Err("invalid docked viewport".into());
        }
        let size = [canvas.width, canvas.height];
        if frame.labels.len() > 2048
            || frame.lines.len() > 2048
            || frame.markers.len() > 2048
            || frame.range_rings.len() > 8
            || frame.models.len() > MAX_MODELS
            || frame.planets.len() > 1
        {
            return Err("docked frame exceeds bounded primitive counts".into());
        }
        let estimate = frame.lines.len() * 6
            + frame.range_rings.len() * 576
            + frame
                .markers
                .iter()
                .map(|m| 480 + if m.current { 576 } else { 0 } + if m.selected { 24 } else { 0 })
                .sum::<usize>()
            + 24;
        if estimate > MAX_GEOMETRY
            || frame.labels.iter().map(|l| l.text.len()).sum::<usize>() > MAX_GLYPHS * 4
        {
            return Err("docked frame exceeds fixed geometry/text bounds".into());
        }
        for l in &frame.labels {
            label(&mut self.text, l, canvas, srgb)?;
        }
        for l in &frame.lines {
            if let Some(clip) = intersection(l.clip, canvas) {
                stroke(
                    &mut self.shapes,
                    l.from,
                    l.to,
                    clip,
                    colour(l.ink, srgb),
                    size,
                );
            }
        }
        for r in &frame.range_rings {
            if let Some(clip) = intersection(r.clip, canvas) {
                ring(
                    &mut self.shapes,
                    r.centre,
                    r.radius,
                    clip,
                    colour(r.ink, srgb),
                    size,
                );
            }
        }
        for m in &frame.markers {
            let Some(clip) = intersection(m.clip, canvas) else {
                continue;
            };
            let c = colour(m.ink, srgb);
            disk(&mut self.shapes, m.point, m.radius, clip, c, size);
            if m.current {
                ring(
                    &mut self.shapes,
                    m.point,
                    m.radius + 3.0,
                    clip,
                    colour(Ink::Green, srgb),
                    size,
                );
            }
            if m.selected {
                outline(
                    &mut self.shapes,
                    Rect::new(
                        m.point[0] - m.radius - 5.0,
                        m.point[1] - m.radius - 5.0,
                        m.radius * 2.0 + 10.0,
                        m.radius * 2.0 + 10.0,
                    ),
                    clip,
                    colour(Ink::White, srgb),
                    size,
                );
            }
        }
        if let Some(r) = frame.hovered {
            outline(&mut self.shapes, r, canvas, colour(Ink::Yellow, srgb), size);
        }
        self.append_previews(frame, model, canvas, srgb, planet_system)?;
        if self.shapes.len() > MAX_GEOMETRY || self.models.len() > MAX_MODEL_VERTICES {
            return Err("docked geometry exceeds fixed buffer capacity".into());
        }
        Ok(())
    }
    fn append_previews(
        &mut self,
        frame: &Frame,
        model: &Model,
        canvas: Rect,
        srgb: bool,
        planet_system: Option<meridian_core::navigation::SystemId>,
    ) -> Result<(), String> {
        for p in &frame.models {
            if crate::mule_render::showroom::matches(p) {
                continue;
            }
            if !model.matches(p) {
                self.unavailable_models += 1;
                label(
                    &mut self.text,
                    &crate::docked_screens::Label {
                        text: "PREVIEW UNAVAILABLE".into(),
                        at: [p.bounds.x + 8.0, p.bounds.y + 8.0],
                        ink: Ink::Blue,
                        clip: p.bounds,
                        scale: 1.25,
                    },
                    canvas,
                    srgb,
                )?;
                continue;
            }
            append_model(&mut self.models, p, model, canvas)?;
        }
        for p in &frame.planets {
            if planet_system != Some(p.system) {
                label(
                    &mut self.text,
                    &crate::docked_screens::Label {
                        text: "PLANET PREVIEW UNAVAILABLE".into(),
                        at: [p.bounds.x + 8.0, p.bounds.y + 8.0],
                        ink: Ink::Blue,
                        clip: p.bounds,
                        scale: 1.25,
                    },
                    canvas,
                    srgb,
                )?;
            }
        }
        Ok(())
    }
}
fn append_model(
    out: &mut Vec<Vertex>,
    preview: &ModelPreview,
    model: &Model,
    canvas: Rect,
) -> Result<(), String> {
    let Some(bounds) = intersection(preview.bounds, canvas) else {
        return Ok(());
    };
    let o = preview.orientation;
    if ![o.pitch, o.roll, o.yaw].into_iter().all(f64::is_finite) {
        return Err("nonfinite model orientation".into());
    }
    let span = (preview.bounds.width.min(preview.bounds.height) - 16.0).max(0.0);
    let scale = span / (model.radius * 2.0);
    let center = preview.bounds.centre();
    for vertices in model.vertices.as_chunks::<3>().0 {
        let input = vertices.map(|v| {
            let p = rotate(
                std::array::from_fn(|i| f64::from(v.position[i]) - model.center[i]),
                o,
            );
            ClipVertex {
                p: [
                    center[0] + p[0] * scale,
                    center[1] - p[1] * scale,
                    0.5 - p[2] / model.radius * 0.4,
                ],
                c: v.color,
            }
        });
        triangle(out, input, bounds, [canvas.width, canvas.height]);
    }
    Ok(())
}
#[cfg(test)]
mod tests;

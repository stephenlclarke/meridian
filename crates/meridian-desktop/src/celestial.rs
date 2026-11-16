//! Celestial shading adapted from frozen L337 a11fe169; see celestial-reference-manifest.json.
//! Inputs are immutable presentation data, never collision/mining/mission authority.
use bytemuck::{Pod, Zeroable};
use meridian_core::{FlightView, Snapshot, docking::Vec3};
use std::sync::Arc;
mod gpu;
pub use gpu::Pass;

pub const MAP_WIDTH: usize = 256;
pub const MAP_HEIGHT: usize = 128;
pub const MAP_SAMPLES: usize = MAP_WIDTH * MAP_HEIGHT;

pub use meridian_core::celestial::BodyId;
mod provider;
pub use provider::WorldAdapter;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub ocean: [f32; 3],
    pub land: [f32; 3],
    pub ice: f32,
    pub sea_level: f32,
    pub highland: [f32; 3],
    pub highland_strength: f32,
    pub vegetation: [f32; 3],
    pub vegetation_strength: f32,
    pub cloud: [f32; 3],
    pub cloud_low: f32,
    pub cloud_high: f32,
    pub cloud_opacity: f32,
    pub atmosphere: [f32; 3],
    pub atmosphere_strength: f32,
    pub surface_haze: f32,
    pub rim_width: f32,
    pub rim_opacity: f32,
    pub night: [f32; 3],
    pub night_intensity: f32,
    pub night_scatter: f32,
}
impl Palette {
    fn valid(self) -> bool {
        let colours = [
            self.ocean,
            self.land,
            self.highland,
            self.vegetation,
            self.cloud,
            self.atmosphere,
            self.night,
        ];
        let levels = [
            self.ice,
            self.sea_level,
            self.highland_strength,
            self.vegetation_strength,
            self.cloud_low,
            self.cloud_high,
            self.cloud_opacity,
            self.atmosphere_strength,
            self.surface_haze,
            self.rim_width,
            self.rim_opacity,
            self.night_scatter,
        ];
        colours
            .into_iter()
            .flatten()
            .chain(levels)
            .all(|v| v.is_finite() && (0.0..=1.0).contains(&v))
            && self.rim_width > 0.0
            && self.cloud_high > self.cloud_low
            && self.sea_level + 0.04 < 0.8
            && self.night_intensity.is_finite()
            && (0.0..=2.0).contains(&self.night_intensity)
    }
}
/// A validated immutable map object. Cache identity is Arc identity, not a caller's mutable revision.
pub struct SurfaceData {
    id: BodyId,
    palette: Palette,
    samples: Vec<[f32; 4]>,
}
impl SurfaceData {
    pub fn new(id: BodyId, palette: Palette, maps: [&[f32]; 4]) -> Result<Arc<Self>, &'static str> {
        if !palette.valid()
            || maps.iter().any(|m| {
                m.len() != MAP_SAMPLES
                    || m.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            })
        {
            return Err("invalid celestial palette or terrain/cloud/settlement maps");
        }
        let samples = (0..MAP_SAMPLES).map(|i| maps.map(|map| map[i])).collect();
        Ok(Arc::new(Self {
            id,
            palette,
            samples,
        }))
    }
}
#[derive(Clone)]
pub struct Planet {
    pub id: BodyId,
    pub position: Vec3,
    pub radius: f64,
    pub angle: f32,
    pub surface: Arc<SurfaceData>,
}
#[derive(Debug, Clone, Copy)]
pub struct Sun {
    pub id: BodyId,
    pub position: Vec3,
    pub radius: f64,
}
/// The simulation owner supplies body identity/position/radius and physical-time-derived angle.
/// No synthetic bodies or defaults are created when either value is absent.
pub struct Scene {
    pub planet: Option<Planet>,
    pub sun: Option<Sun>,
}
impl Scene {
    fn validate(&self, snapshot: Snapshot) -> Result<(), &'static str> {
        let ids = [self.planet.as_ref().map(|p| p.id), self.sun.map(|s| s.id)];
        for id in ids.into_iter().flatten() {
            if id.entity == 0
                || !snapshot
                    .navigation
                    .is_some_and(|n| n.system == id.system && n.visit == id.visit)
            {
                return Err("celestial identity does not match current system visit");
            }
        }
        if self.planet.as_ref().is_some_and(|p| p.surface.id != p.id)
            || ids[0].is_some() && ids[0] == ids[1]
        {
            return Err("celestial surface or body identity mismatch");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Projection {
    pub viewport: [u32; 2],
    pub clip: [f32; 4],
    pub center: [f32; 2],
    pub focal: f32,
    pub near: f32,
    pub far: f32,
}
impl Projection {
    fn valid(self) -> bool {
        let [x, y, w, h] = self.clip;
        self.viewport.into_iter().all(|v| v > 0)
            && self
                .clip
                .into_iter()
                .chain(self.center)
                .chain([self.focal, self.near, self.far])
                .all(f32::is_finite)
            && x >= 0.0
            && y >= 0.0
            && w > 0.0
            && h > 0.0
            && x + w <= self.viewport[0] as f32
            && y + h <= self.viewport[1] as f32
            && self.focal > 0.0
            && self.near > 0.0
            && self.far > self.near
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Disk {
    pub center: [f32; 2],
    pub radius: f32,
    pub view_depth: f32,
    pub world_radius: f32,
}
/// Source-style sphere/frustum intersection keeps visible limbs when the centre is offscreen.
pub fn project_sphere(
    position: Vec3,
    radius: f64,
    p: Projection,
    extent: f32,
) -> Result<Option<Disk>, &'static str> {
    if !p.valid()
        || !finite(position)
        || !radius.is_finite()
        || radius <= 0.0
        || radius > f64::from(f32::MAX)
        || !extent.is_finite()
        || !(1.0..=2.0).contains(&extent)
    {
        return Err("invalid celestial sphere or projection");
    }
    let [x, y, w, h] = p.clip.map(f64::from);
    let [cx, cy] = p.center.map(f64::from);
    let focal = f64::from(p.focal);
    let distance = length(position);
    if !distance.is_finite() {
        return Err("celestial range overflow");
    }
    let (center, r) = if distance <= radius {
        ([x + w / 2.0, y + h / 2.0], w.hypot(h))
    } else {
        if position.z + radius <= 0.0 || position.z - radius >= f64::from(p.far) {
            return Ok(None);
        }
        let left = (x - cx) / focal;
        let right = (x + w - cx) / focal;
        let bottom = (cy - y - h) / focal;
        let top = (cy - y) / focal;
        let expanded = radius * f64::from(extent);
        if position.x - left * position.z < -expanded * left.hypot(1.0)
            || -position.x + right * position.z < -expanded * right.hypot(1.0)
            || position.y - bottom * position.z < -expanded * bottom.hypot(1.0)
            || -position.y + top * position.z < -expanded * top.hypot(1.0)
        {
            return Ok(None);
        }
        let depth = position.z.max(radius * 0.05).max(1.0);
        let r = (focal * radius / depth).min(4.0 * w.max(h));
        (
            [cx + position.x / radius * r, cy - position.y / radius * r],
            r,
        )
    };
    let nearest = [center[0].clamp(x, x + w), center[1].clamp(y, y + h)];
    if (center[0] - nearest[0]).hypot(center[1] - nearest[1]) > r * f64::from(extent) {
        return Ok(None);
    }
    if !center
        .into_iter()
        .chain([r, position.z, radius])
        .all(|v| v.is_finite() && v.abs() <= f64::from(f32::MAX))
    {
        return Err("celestial projection overflow");
    }
    Ok(Some(Disk {
        center: center.map(|v| v as f32),
        radius: r as f32,
        view_depth: position.z as f32,
        world_radius: radius as f32,
    }))
}
fn finite(v: Vec3) -> bool {
    [v.x, v.y, v.z].into_iter().all(f64::is_finite)
}
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn scale(v: Vec3, s: f64) -> Vec3 {
    Vec3::new(v.x * s, v.y * s, v.z * s)
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
/// Displayed sphere normals: x right, y up, z towards the observer; no fixed decorative sun.
pub fn flight_light(planet: Vec3, sun: Option<Vec3>) -> [f32; 4] {
    let Some(sun) = sun.filter(|v| finite(*v)) else {
        return [0.0; 4];
    };
    if !finite(planet) {
        return [0.0; 4];
    }
    let light = sub(sun, planet);
    let n = length(light);
    if !n.is_finite() || n <= 1e-9 {
        return [0.0; 4];
    }
    let distance = length(planet);
    let towards = if distance > 1e-9 {
        scale(planet, -1.0 / distance)
    } else {
        Vec3::new(0.0, 0.0, -1.0)
    };
    let mut right = sub(Vec3::new(1.0, 0.0, 0.0), scale(towards, towards.x));
    if length(right) < 1e-9 {
        right = cross(towards, Vec3::new(0.0, 1.0, 0.0));
    }
    right = scale(right, 1.0 / length(right));
    let up = cross(right, towards);
    let direction = scale(light, 1.0 / n);
    [
        direction.dot(right) as f32,
        direction.dot(up) as f32,
        direction.dot(towards) as f32,
        1.0,
    ]
}
fn view_position(
    snapshot: Snapshot,
    view: FlightView,
    position: Vec3,
) -> Result<Vec3, &'static str> {
    let norm = snapshot.orientation.into_iter().map(|v| v * v).sum::<f32>();
    if !finite(position) || !norm.is_finite() || (norm - 1.0).abs() > 0.001 {
        return Err("non-finite celestial position or invalid camera orientation");
    }
    let delta = [
        position.x - f64::from(snapshot.position[0]),
        position.y - f64::from(snapshot.position[1]),
        position.z - f64::from(snapshot.position[2]),
    ];
    if !delta
        .into_iter()
        .all(|v| v.is_finite() && v.abs() <= f64::from(f32::MAX))
    {
        return Err("celestial camera overflow");
    }
    let [x, y, z] = crate::mesh::rotate_inverse(snapshot.orientation, delta.map(|v| v as f32));
    let [x, y, z] = match view {
        FlightView::Front => [x, y, z],
        FlightView::Rear => [-x, y, -z],
        FlightView::Left => [z, y, -x],
        FlightView::Right => [-z, y, x],
    };
    Ok(Vec3::new(f64::from(x), f64::from(y), f64::from(z)))
}
struct FramePlan {
    uniforms: [Option<Uniforms>; 2],
    clip: [u32; 4],
}
/// Validate the complete frame before any GPU upload, so a bad second body cannot publish half a frame.
fn frame_plan(
    scene: &Scene,
    snapshot: Snapshot,
    view: FlightView,
    p: Projection,
) -> Result<FramePlan, &'static str> {
    scene.validate(snapshot)?;
    if !p.valid() {
        return Err("invalid celestial viewport");
    }
    let [x, y, w, h] = p.clip;
    let clip = [
        x.ceil() as u32,
        y.ceil() as u32,
        ((x + w).floor() as u32).saturating_sub(x.ceil() as u32),
        ((y + h).floor() as u32).saturating_sub(y.ceil() as u32),
    ];
    let mut uniforms = [None; 2];
    if clip[2] == 0 || clip[3] == 0 {
        return Ok(FramePlan { uniforms, clip });
    }
    let sun = scene
        .sun
        .map(|s| view_position(snapshot, view, s.position))
        .transpose()?;
    if let Some(planet) = &scene.planet {
        if !planet.angle.is_finite() {
            return Err("non-finite planet angle");
        }
        let position = view_position(snapshot, view, planet.position)?;
        if let Some(disk) = project_sphere(
            position,
            planet.radius,
            p,
            1.0 + planet.surface.palette.rim_width,
        )? {
            uniforms[0] = Some(Uniforms::new(
                disk,
                p,
                Some(planet.surface.palette),
                planet.angle.rem_euclid(std::f32::consts::TAU),
                flight_light(position, sun),
            ));
        }
    }
    if let Some(s) = scene.sun
        && let Some(disk) = project_sphere(sun.ok_or("missing sun view")?, s.radius, p, 1.15)?
    {
        uniforms[1] = Some(Uniforms::new(disk, p, None, 0.0, [0.0; 4]));
    }
    Ok(FramePlan { uniforms, clip })
}
#[derive(Clone, Copy)]
pub struct DockedPreview<'a> {
    pub scene: Option<&'a Scene>,
    pub snapshot: Snapshot,
    pub request: Option<&'a crate::docked_screens::PlanetPreview>,
    pub viewport: [u32; 2],
    pub scale: f64,
}
fn docked_plan(input: DockedPreview<'_>) -> Result<Option<FramePlan>, &'static str> {
    let DockedPreview {
        scene,
        snapshot,
        request,
        viewport,
        scale,
    } = input;
    let (Some(scene), Some(request)) = (scene, request) else {
        return Ok(None);
    };
    scene.validate(snapshot)?;
    let Some(planet) = scene
        .planet
        .as_ref()
        .filter(|p| p.id.system == request.system)
    else {
        return Ok(None);
    };
    if !planet.radius.is_finite()
        || planet.radius <= 0.0
        || planet.radius > 10000.0
        || !planet.angle.is_finite()
    {
        return Err("invalid actual planet preview geometry");
    }
    let r = request.bounds;
    if !scale.is_finite()
        || scale <= 0.0
        || ![r.x, r.y, r.width, r.height]
            .into_iter()
            .all(f64::is_finite)
        || r.width <= 0.0
        || r.height <= 0.0
    {
        return Err("invalid planet preview bounds");
    }
    let left = (r.x * scale).max(0.0);
    let top = (r.y * scale).max(0.0);
    let right = ((r.x + r.width) * scale).min(f64::from(viewport[0]));
    let bottom = ((r.y + r.height) * scale).min(f64::from(viewport[1]));
    if right <= left || bottom <= top {
        return Ok(None);
    }
    let clip = [
        left.ceil() as u32,
        top.ceil() as u32,
        (right.floor() as u32).saturating_sub(left.ceil() as u32),
        (bottom.floor() as u32).saturating_sub(top.ceil() as u32),
    ];
    if clip[2] == 0 || clip[3] == 0 {
        return Ok(None);
    }
    let projection = Projection {
        viewport,
        clip: [
            left as f32,
            top as f32,
            (right - left) as f32,
            (bottom - top) as f32,
        ],
        center: [
            ((r.x + r.width / 2.0) * scale) as f32,
            ((r.y + r.height / 2.0) * scale) as f32,
        ],
        focal: 1.0,
        near: 1.0,
        far: 100000.0,
    };
    let disk = Disk {
        center: projection.center,
        radius: (r.width.min(r.height) * scale * 0.42) as f32,
        view_depth: (planet.radius * 3.0) as f32,
        world_radius: planet.radius as f32,
    };
    let position = view_position(snapshot, FlightView::Front, planet.position)?;
    let sun = scene
        .sun
        .map(|s| view_position(snapshot, FlightView::Front, s.position))
        .transpose()?;
    let u = Uniforms::new(
        disk,
        projection,
        Some(planet.surface.palette),
        planet.angle,
        flight_light(position, sun),
    );
    if !projection.valid() || !u.geometry.into_iter().chain(u.viewport).all(f32::is_finite) {
        return Err("invalid projected planet preview");
    }
    Ok(Some(FramePlan {
        uniforms: [Some(u), None],
        clip,
    }))
}
/// Thirteen vec4s match WGSL uniform layout without implicit host padding.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    viewport: [f32; 4],
    geometry: [f32; 4],
    depth: [f32; 4],
    light: [f32; 4],
    ocean: [f32; 4],
    land: [f32; 4],
    highland: [f32; 4],
    vegetation: [f32; 4],
    cloud: [f32; 4],
    cloud_levels: [f32; 4],
    atmosphere: [f32; 4],
    night_levels: [f32; 4],
    night: [f32; 4],
}
impl Uniforms {
    fn new(d: Disk, p: Projection, palette: Option<Palette>, angle: f32, light: [f32; 4]) -> Self {
        let mut u = Self::zeroed();
        u.viewport = [
            p.viewport[0] as f32,
            p.viewport[1] as f32,
            d.center[0],
            d.center[1],
        ];
        u.geometry = [d.radius, angle, d.view_depth, d.world_radius];
        u.depth = [
            p.near,
            p.far,
            if palette.is_some() { 0.0 } else { 1.0 },
            1.15,
        ];
        u.light = light;
        if let Some(c) = palette {
            let v = |rgb: [f32; 3], w| [rgb[0], rgb[1], rgb[2], w];
            u.ocean = v(c.ocean, c.ice);
            u.land = v(c.land, c.sea_level);
            u.highland = v(c.highland, c.highland_strength);
            u.vegetation = v(c.vegetation, c.vegetation_strength);
            u.cloud = v(c.cloud, c.cloud_opacity);
            u.cloud_levels = [
                c.cloud_low,
                c.cloud_high,
                c.surface_haze,
                c.atmosphere_strength,
            ];
            u.atmosphere = v(c.atmosphere, c.rim_width);
            u.night_levels = [c.night_intensity, c.night_scatter, c.rim_opacity, 0.0];
            u.night = v(c.night, 0.0);
            u.depth[3] = 1.0 + c.rim_width;
        }
        u
    }
}
#[cfg(test)]
mod tests;

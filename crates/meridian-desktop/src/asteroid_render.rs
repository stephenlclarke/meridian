//! Physical belt geometry and work cues derived from the game's immutable mining snapshots.
use crate::mesh::{self, StationRender, Vertex};
use meridian_core::{
    FlightMode, Snapshot,
    docking::Vec3,
    mining::{ASTEROID_FACES, AsteroidId, BELT_BODY_COUNT, BeltSnapshot, MiningShot},
};
pub const VERTEX_COUNT: usize = BELT_BODY_COUNT * ASTEROID_FACES.len() * 3;
pub const BEAM_VERTEX_COUNT: usize = 12;
#[derive(Debug, Clone, Copy)]
pub struct MiningVisual {
    pub opening: Snapshot,
    pub shot: MiningShot,
}

pub struct Scene<'a> {
    pub belt: &'a BeltSnapshot,
    pub visual: Option<MiningVisual>,
}

impl Scene<'_> {
    pub fn append(
        self,
        snapshot: &Snapshot,
        station: StationRender<'_>,
        sun: Option<Vec3>,
        output: &mut Vec<Vertex>,
    ) -> Result<(), String> {
        let start = output.len();
        append(snapshot, station, self.belt, sun, output)?;
        if let Some(visual) = self.visual
            && let Err(error) = append_beam(snapshot, station, self.belt, visual, output)
        {
            output.truncate(start);
            return Err(error);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cue {
    pub id: AsteroidId,
    pub position: Vec3,
    pub commodity: meridian_core::station_services::CommodityId,
    pub remaining: u8,
    pub initial: u8,
    pub extracted: u8,
    pub energy: f64,
    pub remaining_energy: f64,
    pub range: f64,
    pub bearing_degrees: f64,
    pub elevation_degrees: f64,
    pub front_contact: bool,
}
impl Cue {
    pub fn line(self) -> Result<String, String> {
        let definition = self
            .commodity
            .definition()
            .map_err(|e| format!("Commodity definition unavailable: {e:?}"))?;
        let unit = match definition.unit {
            meridian_core::station_services::CargoUnit::Tonne => "t",
            meridian_core::station_services::CargoUnit::Kilogram => "kg",
            meridian_core::station_services::CargoUnit::Gram => "g",
        };
        Ok(format!(
            "ROCK {} {}: {}/{} {} | WORK {:.0}/{:.0} | RANGE {:.0} | {} {:.0} {} {:.0}{}{}",
            self.id.ordinal() + 1,
            definition.name,
            self.remaining,
            self.initial,
            unit,
            self.remaining_energy,
            self.energy,
            self.range,
            if self.bearing_degrees < 0.0 {
                "LEFT"
            } else {
                "RIGHT"
            },
            self.bearing_degrees.abs(),
            if self.elevation_degrees < 0.0 {
                "DOWN"
            } else {
                "UP"
            },
            self.elevation_degrees.abs(),
            if self.remaining == 0 { " DEPLETED" } else { "" },
            if self.front_contact {
                " FRONT CONTACT"
            } else {
                ""
            }
        ))
    }
}
fn finite(v: Vec3) -> bool {
    [v.x, v.y, v.z].into_iter().all(f64::is_finite)
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn validate(snapshot: Snapshot, belt: &BeltSnapshot) -> Result<(), String> {
    let norm = snapshot.orientation.into_iter().map(|v| v * v).sum::<f32>();
    if !snapshot.position.into_iter().all(f32::is_finite)
        || !norm.is_finite()
        || (norm - 1.0).abs() > 0.001
    {
        return Err("invalid asteroid camera snapshot".into());
    }
    let nav = snapshot
        .navigation
        .ok_or("asteroid display requires an actual commander")?;
    if nav.system != belt.destination.system || !finite(belt.destination.position) {
        return Err("asteroid belt does not belong to current system".into());
    }
    let mut ids = 0u16;
    for b in &belt.bodies {
        let ordinal = usize::from(b.id.ordinal());
        if b.id.system() != nav.system
            || ordinal >= BELT_BODY_COUNT
            || ids & (1 << ordinal) != 0
            || !finite(b.position)
            || [b.position.x, b.position.y, b.position.z]
                .into_iter()
                .any(|v| v.abs() > f64::from(f32::MAX) / 16.0)
            || b.vertices.into_iter().any(|v| !finite(v))
            || !b.energy.is_finite()
            || b.energy <= 0.0
            || !b.remaining_energy.is_finite()
            || !(0.0..=b.energy).contains(&b.remaining_energy)
            || (b.remaining_energy == 0.0) != (b.remaining_units == 0)
            || (b.remaining_energy > 0.0 && b.extracted_units != 0)
            || u16::from(b.remaining_units) + u16::from(b.extracted_units)
                != u16::from(b.initial_units)
        {
            return Err("invalid asteroid identity, vertices or depletion snapshot".into());
        }
        ids |= 1 << ordinal;
    }
    Ok(())
}
/// Emits every face from the SAME world vertices/faces used by actual ray and collision admission.
/// Partial damage retains the mesh; terminal work removes its intact collider and faces.
pub fn append(
    snapshot: &Snapshot,
    station: StationRender<'_>,
    belt: &BeltSnapshot,
    sun: Option<Vec3>,
    output: &mut Vec<Vertex>,
) -> Result<(), String> {
    validate(*snapshot, belt)?;
    if snapshot.mode == FlightMode::Docked {
        return Ok(());
    }
    let camera = mesh::scene_camera_pose(snapshot, station)?;
    let start = output.len();
    for body in belt
        .bodies
        .iter()
        .filter(|body| body.remaining_energy > 0.0)
    {
        for face in ASTEROID_FACES {
            let [a, b, c] = face.map(|i| body.vertices[i]);
            let ab = sub(b, a);
            let ac = sub(c, a);
            let normal = Vec3::new(
                ab.y * ac.z - ab.z * ac.y,
                ab.z * ac.x - ab.x * ac.z,
                ab.x * ac.y - ab.y * ac.x,
            );
            let magnitude = normal.dot(normal).sqrt();
            if !magnitude.is_finite() || magnitude <= 0.0 {
                output.truncate(start);
                return Err("degenerate physical asteroid face".into());
            }
            let illumination = sun.filter(|s| finite(*s)).map_or(0.0, |s| {
                let d = sub(s, body.position);
                let distance = d.dot(d).sqrt();
                if distance > 0.0 && distance.is_finite() {
                    (normal.dot(d) / magnitude / distance).max(0.0) as f32
                } else {
                    0.0
                }
            });
            let strength = 0.24 + 0.76 * illumination;
            let base = [0.31, 0.23, 0.14];
            let color = base.map(|v| v * strength);
            for world in [a, b, c] {
                let view = camera
                    .orientation
                    .inverse_coordinates(sub(world, camera.position));
                let position = [view.x as f32, view.y as f32, view.z as f32];
                if !position.into_iter().all(f32::is_finite) {
                    output.truncate(start);
                    return Err("asteroid camera transform overflow".into());
                }
                output.push(Vertex { position, color });
            }
        }
    }
    Ok(())
}
/// World-space opening pose and actual admitted shot must be retained by Root on success only.
/// This description cannot mine, debit resources, mint a tool or grant a target capability.
pub fn append_beam(
    snapshot: &Snapshot,
    station: StationRender<'_>,
    belt: &BeltSnapshot,
    visual: MiningVisual,
    output: &mut Vec<Vertex>,
) -> Result<(), String> {
    validate(*snapshot, belt)?;
    let current = snapshot.navigation;
    let opening = visual.opening.navigation;
    if snapshot.mode != FlightMode::Flight
        || visual.opening.mode != FlightMode::Flight
        || visual.shot.tick != visual.opening.tick
        || snapshot.tick < visual.shot.tick
        || snapshot.tick - visual.shot.tick >= 5
        || current.zip(opening).is_none_or(|(a, b)| {
            a.system != b.system || a.visit != b.visit || a.travel.is_some() || b.travel.is_some()
        })
    {
        return Ok(());
    }
    let body = belt
        .bodies
        .iter()
        .find(|b| b.id == visual.shot.contact.asteroid)
        .ok_or("mining flash does not reference an actual body")?;
    if visual.shot.commodity != body.commodity
        || visual.shot.remaining_energy != body.remaining_energy
        || visual.shot.extracted_units
            != if body.remaining_energy == 0.0 {
                body.initial_units
            } else {
                0
            }
        || !finite(visual.shot.contact.point)
    {
        return Err("invalid committed mining visual".into());
    }
    let camera = mesh::scene_camera_pose(snapshot, station)?;
    let start_world = Vec3::new(
        f64::from(visual.opening.position[0]),
        f64::from(visual.opening.position[1]),
        f64::from(visual.opening.position[2]),
    );
    let a = camera
        .orientation
        .inverse_coordinates(sub(start_world, camera.position));
    let b = camera
        .orientation
        .inverse_coordinates(sub(visual.shot.contact.point, camera.position));
    let make = |p: Vec3, x: f64| Vertex {
        position: [(p.x + x) as f32, p.y as f32, p.z as f32],
        color: [0.08, 0.8, 1.0],
    };
    let quad = [make(a, -0.8), make(a, 0.8), make(b, 0.8), make(b, -0.8)];
    if quad
        .iter()
        .any(|v| v.position.into_iter().any(|p| !p.is_finite()))
    {
        return Err("mining flash camera overflow".into());
    }
    let extent = sub(b, a);
    let length = extent.dot(extent).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return Ok(());
    }
    let along = (12.0 / length).min(0.5);
    let center = Vec3::new(
        a.x + extent.x * along,
        a.y + extent.y * along,
        a.z + extent.z * along,
    );
    let flash = [
        make(Vec3::new(center.x, center.y - 0.35, center.z), -0.35),
        make(Vec3::new(center.x, center.y - 0.35, center.z), 0.35),
        make(Vec3::new(center.x, center.y + 0.35, center.z), 0.35),
        make(Vec3::new(center.x, center.y + 0.35, center.z), -0.35),
    ];
    for q in [quad, flash] {
        output.extend([q[0], q[1], q[2], q[0], q[2], q[3]]);
    }
    Ok(())
}
/// Player-relative identity, finite ore and bearings are descriptive, never work eligibility.
pub fn cues(snapshot: Snapshot, belt: &BeltSnapshot) -> Result<[Cue; BELT_BODY_COUNT], String> {
    validate(snapshot, belt)?;
    // Navigation cues remain player relative, independent of the exterior chase camera.
    Ok(std::array::from_fn(|i| {
        let b = belt.bodies[i];
        let delta = [
            b.position.x - f64::from(snapshot.position[0]),
            b.position.y - f64::from(snapshot.position[1]),
            b.position.z - f64::from(snapshot.position[2]),
        ];
        let [x, y, z] = mesh::rotate_inverse(snapshot.orientation, delta.map(|v| v as f32));
        Cue {
            id: b.id,
            position: b.position,
            commodity: b.commodity,
            remaining: b.remaining_units,
            initial: b.initial_units,
            extracted: b.extracted_units,
            energy: b.energy,
            remaining_energy: b.remaining_energy,
            range: delta.into_iter().map(|v| v * v).sum::<f64>().sqrt(),
            bearing_degrees: f64::from(x.atan2(z).to_degrees()),
            elevation_degrees: f64::from(y.atan2(x.hypot(z)).to_degrees()),
            front_contact: belt.front_contact.is_some_and(|c| c.asteroid == b.id),
        }
    }))
}
/// A chart address selects a description only. The belt-centre sentinel is not a rock ID.
pub fn hud_cue(
    snapshot: Snapshot,
    belt: &BeltSnapshot,
    selected: Option<(meridian_core::navigation::SystemId, u64)>,
) -> Result<Option<Cue>, String> {
    let cues = cues(snapshot, belt)?;
    Ok(selected
        .and_then(|(system, ordinal)| {
            cues.iter()
                .find(|cue| cue.id.system() == system && u64::from(cue.id.ordinal()) == ordinal)
        })
        .or_else(|| cues.iter().find(|cue| cue.front_contact))
        .copied())
}

pub fn hud_line(
    snapshot: Snapshot,
    belt: &BeltSnapshot,
    selected: Option<(meridian_core::navigation::SystemId, u64)>,
) -> Result<Option<String>, String> {
    hud_cue(snapshot, belt, selected)?
        .map(Cue::line)
        .transpose()
}
#[cfg(test)]
mod tests;

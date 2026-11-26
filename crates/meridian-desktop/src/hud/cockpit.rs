//! L337 a11fe169 CockpitDisplay.swift gauge grouping/scanner and StationNavigation.swift.
//! Only World snapshots supply readings. Missing simulations have no invented needle.
use super::{glyph, quad};
use crate::mesh::{Vertex, rotate_inverse};
use meridian_core::equipment::WorkTargetId;
use meridian_core::traffic::{MAX_TRAFFIC_ACTORS, NpcPhase, Population, TrafficSnapshot};
use meridian_core::{
    EcmSnapshot, FlightMode, FlightView, Snapshot, SolarScoopStatus, SolarSnapshot, TargetIdentity,
    World,
};

const YELLOW: [f32; 3] = [1.0, 1.0, 0.0];
const GREEN: [f32; 3] = [0.0, 1.0, 0.0];
const BLUE: [f32; 3] = [68.0 / 255.0, 136.0 / 255.0, 1.0];
const RED: [f32; 3] = [1.0, 0.0, 0.0];
const CYAN: [f32; 3] = [0.0, 1.0, 1.0];
// Projectile IDs use a checked u8 ordinal. Include every possible retained launch,
// plus the station and three practice bodies. NPCs have their own disjoint suffix.
const NPC_CONTACT_START: usize = 4 + crate::mesh::MAX_MISSILE_PROJECTILES;
pub const MAX_CONTACTS: usize = NPC_CONTACT_START + MAX_TRAFFIC_ACTORS;
// Station, finite ordinary actors, one mission Mule, belt rocks and pending physical ore.
pub const MAX_SCANNER_TARGETS: usize = 2
    + MAX_TRAFFIC_ACTORS
    + meridian_core::mining::BELT_BODY_COUNT
    + meridian_core::mining::MAX_PENDING_ORE;
// Context markers use three line quads; admitted contacts use two and one selected
// bracket uses four. Keep the bounded GPU allocation large enough for the complete belt.
pub const MAX_VERTICES: usize = 30_000
    + MAX_CONTACTS * 18
    + MAX_SCANNER_TARGETS * 12
    + 24
    + (9 - 3) * 8 * 6
    + (40 + 12) * 35 * 6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScannerTarget {
    pub identity: TargetIdentity,
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cockpit {
    pub flight: Snapshot,
    pub view: FlightView,
    pub shield: [Option<f32>; 2],
    pub energy: Option<f32>,
    pub heat: Option<f32>,
    pub fuel: Option<f32>,
    pub solar: Option<SolarSnapshot>,
    pub solar_limited: bool,
    pub ecm: Option<EcmSnapshot>,
    pub missiles: usize,
    pub armed: bool,
    pub locked: bool,
    pub destroyed: bool,
    /// Non-selectable context markers in front flight coordinates, independent of camera.
    pub contacts: [Option<([f32; 3], [f32; 3])>; MAX_CONTACTS],
    /// Exactly the selectable World scanner list, in its cycle order.
    pub scanner_targets: Vec<ScannerTarget>,
    pub station: Option<[f32; 3]>,
}
impl Cockpit {
    /// Capture once on the simulation-owning event loop; drawing has no World reference.
    pub fn capture(world: &World, flight: Snapshot) -> Result<Self, String> {
        let combat = world.combat_snapshot();
        let resources = world.flight_resources_snapshot();
        let missile = world.missile_snapshot();
        let station = flight.station.map(|s| relative(flight, s.pose.position));
        let mut contacts = [None; MAX_CONTACTS];
        let projected = world.scanner_targets();
        if !projected
            .iter()
            .any(|(identity, _)| matches!(identity, TargetIdentity::Station { .. }))
        {
            contacts[0] = station.map(|position| (position, GREEN));
        }
        if let Some(beacons) = world.practice_snapshot().and_then(|p| p.beacons) {
            for (i, b) in beacons.into_iter().enumerate() {
                contacts[i + 1] = Some((relative(flight, b.pose.position), CYAN));
            }
        }
        for (contact, projectile) in contacts[4..NPC_CONTACT_START]
            .iter_mut()
            .zip(world.missile_projectiles().filter(|p| p.ended.is_none()))
        {
            *contact = Some((relative(flight, projectile.position), RED));
        }
        let traffic = world.traffic_snapshot();
        if let Some(traffic) = &traffic {
            fill_traffic_contacts(flight, traffic, &mut contacts)?;
            // An actor already in the authoritative selectable list has no duplicate
            // context marker. The retained unsupported/depleted bodies remain distinct.
            for (index, actor) in traffic.actors.iter().enumerate() {
                if projected.iter().any(|(identity, _)| matches!(identity,
                    TargetIdentity::Ship(id) | TargetIdentity::Work(WorkTargetId::Wreck(id)) if *id == actor.id))
                {
                    contacts[NPC_CONTACT_START + index] = None;
                }
            }
        }
        let scanner_targets = capture_scanner_targets(world, flight, &projected, traffic.as_ref())?;
        Ok(Self {
            flight,
            view: world.flight_view(),
            shield: [
                combat.map(|c| {
                    c.player.front_shield_hundredths as f32
                        / c.player.shield_capacity_hundredths as f32
                }),
                combat.map(|c| {
                    c.player.rear_shield_hundredths as f32
                        / c.player.shield_capacity_hundredths as f32
                }),
            ],
            energy: combat.map(|c| {
                f32::from(c.player.energy_hundredths)
                    / f32::from(meridian_core::ENERGY_CAPACITY_HUNDREDTHS)
            }),
            heat: resources.map(|l| f32::from(l.heat_fifths) / (5.0 * 255.0)),
            fuel: flight
                .navigation
                .map(|n| f32::from(n.fuel_units) / f32::from(n.fuel_capacity_units)),
            solar: world.solar_snapshot(),
            solar_limited: world
                .solar_save_limit_reached(meridian_core::save::MAX_SOLAR_HISTORY_BYTES),
            ecm: world.ecm_snapshot(),
            missiles: world
                .missile_rounds()
                .map_or(0, |rounds| rounds.iter().flatten().count()),
            armed: missile.is_some_and(|m| m.armed),
            locked: missile.is_some_and(|m| m.lock.is_some() || m.npc_lock.is_some()),
            destroyed: combat.is_some_and(|c| c.player.destroyed),
            contacts,
            scanner_targets,
            station,
        })
    }
    pub fn status(&self) -> &'static str {
        if self.destroyed {
            "DESTROYED"
        } else if self.flight.mode == FlightMode::Docked {
            "DOCKED"
        } else if self.flight.mode == FlightMode::Launching {
            "LAUNCHING"
        } else {
            "FLIGHT"
        } // Hostile/Safe requires the unported active hostile-body authority.
    }
    pub fn solar_label(&self) -> Option<(&'static str, [f32; 3])> {
        let solar = self.solar?;
        if self.destroyed && solar.cabin_temperature >= 1.0 {
            return Some(("CABIN OVERHEAT - HULL DESTROYED", RED));
        }
        if solar.cabin_temperature >= 0.9 {
            return Some(("CABIN CRITICAL - LEAVE CORONA", RED));
        }
        if solar.cabin_temperature >= 0.75 {
            return Some(("CABIN DANGER - LEAVE CORONA", RED));
        }
        if self.solar_limited {
            return Some(("SAVE HISTORY FULL - FLIGHT ACTIVE", YELLOW));
        }
        match solar.status {
            SolarScoopStatus::Inactive => None,
            SolarScoopStatus::NoScoops => Some(("SOLAR CORONA - NO SCOOPS", YELLOW)),
            SolarScoopStatus::TurnTowardSun => Some(("SOLAR SCOOP - FACE SUN AND CRUISE", YELLOW)),
            SolarScoopStatus::Collecting => Some(("SOLAR SCOOP ACTIVE", GREEN)),
            SolarScoopStatus::TankFull => Some(("SOLAR SCOOP - TANK FULL", GREEN)),
        }
    }
    pub fn station_navigation(&self) -> Option<(f32, f32, f32)> {
        let p = self.station?;
        let [x, y, z] = view_position(self.view, p);
        Some((
            p.iter().map(|v| v * v).sum::<f32>().sqrt(),
            x.atan2(z).to_degrees(),
            y.atan2(x.hypot(z)).to_degrees(),
        ))
    }
}

fn capture_scanner_targets(
    world: &World,
    flight: Snapshot,
    projected: &[(TargetIdentity, meridian_core::docking::Vec3)],
    traffic: Option<&TrafficSnapshot>,
) -> Result<Vec<ScannerTarget>, String> {
    if projected.len() > MAX_SCANNER_TARGETS {
        return Err("Selectable scanner targets exceed the admitted body bound".into());
    }
    let selected = world.targeting_snapshot().map(|target| {
        if projected
            .iter()
            .any(|(identity, _)| *identity == target.identity)
        {
            target.identity
        } else {
            target.tracked_fragment.map_or(target.identity, |id| {
                TargetIdentity::Work(WorkTargetId::Ore(id))
            })
        }
    });
    let mule = world.mule_actor_snapshot();
    projected
        .iter()
        .map(|&(identity, point)| {
            let position = relative(flight, point);
            if position.iter().any(|value| !value.is_finite()) {
                return Err("World produced a non-finite selectable scanner contact".into());
            }
            let color = match identity {
                TargetIdentity::Station { .. } => GREEN,
                TargetIdentity::Work(WorkTargetId::Asteroid(_)) => YELLOW,
                TargetIdentity::Work(WorkTargetId::Ore(_)) => CYAN,
                TargetIdentity::Work(WorkTargetId::Wreck(_)) => BLUE,
                TargetIdentity::Ship(id) => {
                    let actor = traffic
                        .and_then(|s| s.actors.iter().find(|actor| actor.id == id))
                        .or_else(|| {
                            mule.as_ref()
                                .filter(|m| m.actor().id == id)
                                .map(|m| m.actor())
                        })
                        .ok_or("Selectable scanner ship has no current actor")?;
                    if actor.electrical_disabled || actor.phase == NpcPhase::Destroyed {
                        BLUE
                    } else if actor.wanted > 0 {
                        RED
                    } else {
                        YELLOW
                    }
                }
            };
            Ok(ScannerTarget {
                identity,
                position,
                color,
                selected: selected == Some(identity),
            })
        })
        .collect()
}

fn fill_traffic_contacts(
    flight: Snapshot,
    traffic: &TrafficSnapshot,
    contacts: &mut [Option<([f32; 3], [f32; 3])>; MAX_CONTACTS],
) -> Result<(), String> {
    if traffic.actors.len() > MAX_TRAFFIC_ACTORS {
        return Err("Scanner traffic exceeds the admitted body bound".into());
    }
    let system = flight
        .navigation
        .ok_or("Scanner traffic requires its actual navigation frame")?
        .system;
    let mut captured = [None; MAX_TRAFFIC_ACTORS];
    for (index, actor) in traffic.actors.iter().enumerate() {
        if actor.id.system() != system
            || actor.hull_id != "SH025"
            || !matches!(actor.population, Population::Trader | Population::Pirate)
        {
            return Err("Scanner traffic has no admitted hull in this frame".into());
        }
        if actor.phase == NpcPhase::Complete {
            continue;
        }
        let position = relative(flight, actor.pose.position);
        if position.iter().any(|value| !value.is_finite()) {
            return Err("Scanner traffic produced a non-finite contact".into());
        }
        // Color is descriptive: a retained destroyed body, positive wanted level,
        // or an ordinary live contact. It grants no hostile/target/lock authority.
        let color = if actor.phase == NpcPhase::Destroyed {
            BLUE
        } else if actor.wanted > 0 {
            RED
        } else {
            YELLOW
        };
        captured[index] = Some((position, color));
    }
    contacts[NPC_CONTACT_START..].copy_from_slice(&captured);
    Ok(())
}
fn relative(s: Snapshot, p: meridian_core::docking::Vec3) -> [f32; 3] {
    rotate_inverse(
        s.orientation,
        [
            p.x as f32 - s.position[0],
            p.y as f32 - s.position[1],
            p.z as f32 - s.position[2],
        ],
    )
}
fn view_position(view: FlightView, [x, y, z]: [f32; 3]) -> [f32; 3] {
    match view {
        FlightView::Front => [x, y, z],
        FlightView::Rear => [-x, y, -z],
        FlightView::Left => [z, y, -x],
        FlightView::Right => [-z, y, x],
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub top: f32,
    pub row: f32,
    pub scale: f32,
    pub side: f32,
    pub radar: f32,
    pub instrument_scale: f32,
    pub left_columns: usize,
}
impl Layout {
    pub fn new(width: u32, height: u32) -> Self {
        let w = width as f32;
        let scale = 1.25;
        let instrument_scale: f32 = if w < 640.0 {
            1.25
        } else if w < 1100.0 {
            1.5
        } else {
            2.0
        };
        let row = (instrument_scale * 7.0).ceil() + 6.0;
        let radar = (w * 0.30).clamp(104.0, 560.0);
        let side = (w - radar - 48.0) / 2.0;
        let left_columns = if side >= 400.0 { 2 } else { 1 };
        let rows = (6.0 / left_columns as f32).ceil().max(4.0);
        let scanner_height: f32 = if w < 900.0 { 80.0 } else { 108.0 };
        Self {
            top: (height as f32 - (16.0 + row + (row * rows).max(scanner_height)))
                .max(120.0)
                .min(height as f32),
            row,
            scale,
            side,
            radar,
            instrument_scale,
            left_columns,
        }
    }
}
fn line(out: &mut Vec<Vertex>, a: [f32; 2], b: [f32; 2], color: [f32; 3], size: [f32; 2]) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = dx.hypot(dy);
    if length <= f32::EPSILON {
        return;
    }
    let n = [-dy / length * 0.6, dx / length * 0.6];
    let vertex = |p: [f32; 2], sign: f32| Vertex {
        position: [
            2.0 * (p[0] + n[0] * sign).clamp(0.0, size[0]) / size[0] - 1.0,
            1.0 - 2.0 * (p[1] + n[1] * sign).clamp(0.0, size[1]) / size[1],
            0.0,
        ],
        color,
    };
    out.extend_from_slice(&[
        vertex(a, 1.0),
        vertex(a, -1.0),
        vertex(b, 1.0),
        vertex(b, 1.0),
        vertex(a, -1.0),
        vertex(b, -1.0),
    ]);
}
fn label(
    out: &mut Vec<Vertex>,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    color: [f32; 3],
    size: [f32; 2],
) {
    for (i, c) in text.chars().enumerate() {
        for (row, bits) in glyph(c).into_iter().enumerate() {
            for bit in 0..5 {
                if bits & (1 << (4 - bit)) == 0 {
                    continue;
                }
                let left = x + (i * 6 + bit) as f32 * scale;
                let top = y + row as f32 * scale;
                if left >= 0.0 && top >= 0.0 && left + scale <= size[0] && top + scale <= size[1] {
                    quad(out, [left, top, left + scale, top + scale], color, size);
                }
            }
        }
    }
}
struct Gauge<'a> {
    name: &'a str,
    value: Option<f32>,
    color: [f32; 3],
}
fn gauge(out: &mut Vec<Vertex>, reading: Gauge<'_>, x: f32, y: f32, l: Layout, size: [f32; 2]) {
    let Gauge { name, value, color } = reading;
    label(out, name, x, y, l.scale, YELLOW, size);
    let bx = x + (l.side * 0.75).min(13.0 * 6.0 * l.instrument_scale);
    let end = (x + l.side).min(bx + 160.0);
    let by = y + 5.0;
    line(out, [bx, by], [end, by], BLUE, size);
    if let Some(v) = value.filter(|v| v.is_finite()) {
        line(
            out,
            [bx, by],
            [bx + (end - bx) * v.clamp(0.0, 1.0), by],
            color,
            size,
        );
    } else {
        label(out, "N/A", bx, y, l.scale, CYAN, size);
    }
}
fn scanner(out: &mut Vec<Vertex>, c: &Cockpit, l: Layout, size: [f32; 2]) {
    let cx = size[0] / 2.0;
    let rx = (l.radar - 12.0) / 2.0;
    let ry = if size[0] < 900.0 { 32.0 } else { 46.0 };
    let cy = l.top + 8.0 + l.row + 16.0 + ry;
    label(
        out,
        "SCANNER",
        cx - 21.0 * l.scale,
        l.top + 8.0 + l.row,
        l.scale,
        YELLOW,
        size,
    );
    for i in 0..64 {
        let point = |n| {
            let a = n as f32 * std::f32::consts::TAU / 64.0;
            [cx + rx * a.cos(), cy + ry * a.sin()]
        };
        line(out, point(i), point(i + 1), GREEN, size);
    }
    line(out, [cx - rx, cy], [cx + rx, cy], GREEN, size);
    line(out, [cx, cy - ry], [cx, cy + ry], GREEN, size);
    let location = |[x, y, z]: [f32; 3]| {
        let sx = cx + (x / 4400.0 * rx).clamp(-rx + 2.0, rx - 2.0);
        let sy = cy + (-z / 8500.0 * ry).clamp(-ry + 2.0, ry - 2.0);
        let lift = (y / 3000.0 * ry).clamp(-ry / 2.0, ry / 2.0);
        (sx, sy, sy - lift)
    };
    // Context contacts use a cross rather than the selectable bar.
    for (position, color) in c.contacts.iter().flatten() {
        let (x, base, y) = location(*position);
        line(out, [x, base], [x, y], *color, size);
        line(out, [x - 2.0, y - 2.0], [x + 2.0, y + 2.0], *color, size);
        line(out, [x - 2.0, y + 2.0], [x + 2.0, y - 2.0], *color, size);
    }
    for target in &c.scanner_targets {
        let (x, base, y) = location(target.position);
        line(out, [x, base], [x, y], target.color, size);
        line(out, [x - 2.0, y], [x + 2.0, y], target.color, size);
        if target.selected {
            let points = [
                [x - 4.0, y - 4.0],
                [x + 4.0, y - 4.0],
                [x + 4.0, y + 4.0],
                [x - 4.0, y + 4.0],
            ];
            for (a, b) in points.into_iter().zip(points.into_iter().cycle().skip(1)) {
                line(out, a, b, CYAN, size);
            }
        }
    }
}
/// Appends bounded geometry after the existing review/control overlay, without gameplay access.
pub fn append(c: &Cockpit, width: u32, height: u32, out: &mut Vec<Vertex>) {
    if width < 320 || height < 240 {
        return;
    }
    let size = [width as f32, height as f32];
    let l = Layout::new(width, height);
    quad(out, [0.0, l.top, size[0], size[1]], [0.0, 0.0, 0.0], size);
    line(out, [0.0, l.top], [size[0], l.top], YELLOW, size);
    label(
        out,
        &format!("STATUS: {}", c.status()),
        12.0,
        l.top + 8.0,
        l.scale,
        if c.destroyed { RED } else { GREEN },
        size,
    );
    let x = (size[0] * 0.38).floor();
    label(out, "Missiles", x, l.top + 8.0, l.scale, YELLOW, size);
    let mut next = x + 8.0 * 6.0 * l.scale + 8.0;
    if c.missiles == 0 {
        label(out, "None", next, l.top + 8.0, l.scale, [1.0; 3], size);
        next += 4.0 * 6.0 * l.scale;
    } else {
        let points = [
            [5.0, 0.0],
            [2.0, 4.0],
            [2.0, 9.0],
            [0.0, 12.0],
            [5.0, 10.0],
            [10.0, 12.0],
            [8.0, 9.0],
            [8.0, 4.0],
            [5.0, 0.0],
        ];
        for _ in 0..c.missiles {
            let scale = l.instrument_scale / 1.5;
            for pair in points.windows(2) {
                line(
                    out,
                    [next + pair[0][0] * scale, l.top + 8.0 + pair[0][1] * scale],
                    [next + pair[1][0] * scale, l.top + 8.0 + pair[1][1] * scale],
                    if c.armed { RED } else { GREEN },
                    size,
                );
            }
            next += 14.0 * scale;
        }
    }
    label(
        out,
        if c.armed { "Armed" } else { "Unarmed" },
        next + 8.0,
        l.top + 8.0,
        l.scale,
        if c.armed { RED } else { [1.0; 3] },
        size,
    );
    let left = [
        ("FRONT SHIELD", c.shield[0], GREEN),
        ("REAR SHIELD", c.shield[1], GREEN),
        ("FUEL", c.fuel, GREEN),
        (
            "CABIN TEMP",
            c.solar.map(|solar| solar.cabin_temperature as f32),
            RED,
        ),
        ("LASER HEAT", c.heat, RED),
        ("ALTITUDE", None, YELLOW),
    ];
    let rows = 6 / l.left_columns;
    let column_width = (l.side - (l.left_columns - 1) as f32 * 12.0) / l.left_columns as f32;
    for (i, (name, value, color)) in left.into_iter().enumerate() {
        let x = 12.0 + (i / rows) as f32 * (column_width + 12.0);
        let y = l.top + 8.0 + l.row + (i % rows) as f32 * l.row;
        gauge(
            out,
            Gauge { name, value, color },
            x,
            y,
            Layout {
                side: column_width,
                ..l
            },
            size,
        );
    }
    let instrument_width = l.side.min(13.0 * 6.0 * l.instrument_scale + 160.0);
    let right = size[0] - 12.0 - instrument_width;
    let right_layout = Layout {
        side: instrument_width,
        ..l
    };
    for (i, (name, value)) in [
        ("SPEED", Some(c.flight.speed / meridian_core::MAX_SPEED)),
        ("ROLL", None),
        ("PITCH", None),
    ]
    .into_iter()
    .enumerate()
    {
        gauge(
            out,
            Gauge {
                name,
                value,
                color: GREEN,
            },
            right,
            l.top + 8.0 + l.row * (i + 1) as f32,
            right_layout,
            size,
        );
    }
    label(
        out,
        "ENERGY",
        right,
        l.top + 8.0 + 4.0 * l.row,
        l.scale,
        YELLOW,
        size,
    );
    let bx = right + (instrument_width * 0.75).min(13.0 * 6.0 * l.instrument_scale);
    let end = (right + instrument_width).min(bx + 160.0);
    if let Some(energy) = c.energy {
        for bank in 0..4 {
            let y = l.top + 8.0 + 4.0 * l.row + bank as f32 * 3.0 + 4.0;
            line(out, [bx, y], [end, y], BLUE, size);
            line(
                out,
                [bx, y],
                [
                    bx + (end - bx) * (energy * 4.0 - bank as f32).clamp(0.0, 1.0),
                    y,
                ],
                YELLOW,
                size,
            );
        }
    } else {
        label(
            out,
            "N/A",
            bx,
            l.top + 8.0 + 4.0 * l.row,
            l.scale,
            CYAN,
            size,
        );
    }
    scanner(out, c, l, size);
    append_station_navigation(out, c, l, size);
    if c.flight.mode != FlightMode::Docked
        && let Some((text, color)) = c.solar_label()
    {
        label(out, text, 12.0, l.top - 14.0, l.scale, color, size);
    }
    if c.flight.mode == FlightMode::Flight
        && !c.destroyed
        && let Some(ecm) = c.ecm.filter(|s| s.fitted.is_some())
    {
        let (message, color) = if ecm.active.is_some() {
            ("ECM ACTIVE", GREEN)
        } else if ecm.remaining_ticks > 0 {
            ("ECM COOLDOWN", CYAN)
        } else {
            ("ECM FITTED", YELLOW)
        };
        label(
            out,
            message,
            size[0] - 12.0 - message.len() as f32 * 6.0 * l.scale,
            l.top - 14.0,
            l.scale,
            color,
            size,
        );
    }
}

fn append_station_navigation(out: &mut Vec<Vertex>, c: &Cockpit, l: Layout, size: [f32; 2]) {
    if c.flight.mode != FlightMode::Docked
        && let Some((distance, bearing, elevation)) = c.station_navigation()
    {
        let range = if distance >= 1_000_000_000.0 {
            format!("{distance:.1e}")
        } else if distance >= 1_000_000.0 {
            format!("{:.1}M", distance / 1_000_000.0)
        } else if distance >= 1000.0 {
            format!("{:.1}K", distance / 1000.0)
        } else {
            format!("{distance:.0}")
        };
        let bearing_text = if bearing.abs().round() == 0.0 {
            "AHEAD 0 DEG".into()
        } else {
            format!(
                "{}{} {:.0} DEG",
                if bearing.abs() > 90.0 { "BEHIND " } else { "" },
                if bearing < 0.0 { "LEFT" } else { "RIGHT" },
                bearing.abs()
            )
        };
        let elevation_text = if elevation.abs().round() == 0.0 {
            "LEVEL 0 DEG".into()
        } else {
            format!(
                "{} {:.0} DEG",
                if elevation < 0.0 { "DOWN" } else { "UP" },
                elevation.abs()
            )
        };
        for (i, text) in [
            format!("STATION {range} UNITS"),
            bearing_text,
            elevation_text,
        ]
        .iter()
        .enumerate()
        {
            label(
                out,
                text,
                12.0,
                12.0 + i as f32 * 14.0,
                l.scale,
                YELLOW,
                size,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_core::FlightInput;
    fn world() -> World {
        World::new_commander(90011).unwrap_or_else(|error| panic!("{error}"))
    }
    #[test]
    fn paid_round_in_ninth_pylon_is_not_hidden_by_three_slot_snapshot_prefix() {
        let mut world = world();
        let review = world
            .review_ammunition(meridian_core::AmmunitionAction::Purchase, 0)
            .unwrap_or_else(|error| panic!("{error}"));
        let prepared = world
            .prepare_ammunition(review, 0)
            .unwrap_or_else(|error| panic!("{error}"));
        let item = prepared.commit();
        for action in [
            meridian_core::AmmunitionAction::Store { item },
            meridian_core::AmmunitionAction::Refit { item, pylon: 8 },
        ] {
            let review = world
                .review_ammunition(action, 0)
                .unwrap_or_else(|error| panic!("{error}"));
            world
                .prepare_ammunition(review, 0)
                .unwrap_or_else(|error| panic!("{error}"))
                .commit();
        }
        let before = world.save_bytes().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            world
                .missile_snapshot()
                .unwrap_or_else(|| panic!("snapshot"))
                .rounds
                .iter()
                .flatten()
                .count(),
            3
        );
        let cockpit = Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(cockpit.missiles, 4);
        let mut vertices = Vec::with_capacity(super::super::MAX_VERTICES);
        append(&cockpit, 640, 360, &mut vertices);
        assert!(vertices.len() <= MAX_VERTICES);
        assert_eq!(
            world.save_bytes().unwrap_or_else(|error| panic!("{error}")),
            before
        );
    }
    #[test]
    fn every_test_shield_grade_uses_its_own_capacity_in_the_cockpit() {
        let mut world = world();
        let review = world
            .review_xyzzy(meridian_core::XyzzyAction::Enter, 0)
            .unwrap_or_else(|e| panic!("{e}"));
        world
            .prepare_xyzzy(review, 0)
            .unwrap_or_else(|e| panic!("{e}"))
            .commit();
        for grade in meridian_core::ShieldGrade::ALL {
            let action = meridian_core::XyzzyAction::FitShield(grade);
            let review = world
                .review_xyzzy(action, 0)
                .unwrap_or_else(|e| panic!("{e}"));
            world
                .prepare_xyzzy(review, 0)
                .unwrap_or_else(|e| panic!("{e}"))
                .commit();
            let c = Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(c.shield, [Some(1.0); 2], "{}", grade.name());
        }
    }

    #[test]
    fn fresh_instruments_are_world_owned_and_rendering_cannot_change_save() {
        let world = world();
        let before = world.save_bytes().unwrap_or_else(|error| panic!("{error}"));
        let c = Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(c.shield, [Some(1.0); 2]);
        assert_eq!(c.energy, Some(1.0));
        assert_eq!(c.fuel, Some(1.0));
        assert_eq!(c.heat, Some(0.0));
        assert_eq!(c.solar, Some(SolarSnapshot::default()));
        assert_eq!(c.missiles, 3);
        assert!(!c.armed);
        assert!(!c.locked);
        assert_eq!(c.status(), "DOCKED");
        let mut out = Vec::with_capacity(super::super::MAX_VERTICES);
        for [w, h] in [[640, 360], [1280, 720], [2560, 1600]] {
            append(&c, w, h, &mut out);
            out.clear();
        }
        assert_eq!(
            world.save_bytes().unwrap_or_else(|error| panic!("{error}")),
            before
        );
    }
    #[test]
    fn solar_readout_uses_status_and_thermal_warning_precedence() {
        let mut world = world();
        world.request_launch(0).unwrap_or_else(|e| panic!("{e}"));
        world
            .step(FlightInput::default())
            .unwrap_or_else(|e| panic!("{e}"));
        let mut cockpit =
            Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e}"));
        let before = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        for (temperature, status, text) in [
            (0.1, SolarScoopStatus::Collecting, "SOLAR SCOOP ACTIVE"),
            (0.2, SolarScoopStatus::NoScoops, "SOLAR CORONA - NO SCOOPS"),
            (
                0.4,
                SolarScoopStatus::TurnTowardSun,
                "SOLAR SCOOP - FACE SUN AND CRUISE",
            ),
            (0.6, SolarScoopStatus::TankFull, "SOLAR SCOOP - TANK FULL"),
            (
                0.75,
                SolarScoopStatus::Collecting,
                "CABIN DANGER - LEAVE CORONA",
            ),
            (
                0.9,
                SolarScoopStatus::Collecting,
                "CABIN CRITICAL - LEAVE CORONA",
            ),
        ] {
            cockpit.solar = Some(SolarSnapshot {
                status,
                cabin_temperature: temperature,
                fuel_remainder: 0.0,
            });
            assert_eq!(cockpit.solar_label().map(|(label, _)| label), Some(text));
            let mut vertices = Vec::new();
            append(&cockpit, 640, 360, &mut vertices);
            assert!(vertices.len() <= MAX_VERTICES);
        }
        assert_eq!(world.save_bytes().unwrap_or_else(|e| panic!("{e}")), before);
    }
    #[test]
    fn actual_flight_and_pulse_change_speed_energy_and_heat_readings() {
        let mut world = world();
        world
            .request_launch(0)
            .unwrap_or_else(|error| panic!("{error}"));
        for _ in 0..600 {
            world
                .step(FlightInput::default())
                .unwrap_or_else(|error| panic!("{error}"));
            if world.snapshot().mode == FlightMode::Flight {
                break;
            }
        }
        assert_eq!(world.snapshot().mode, FlightMode::Flight);
        world
            .step(FlightInput {
                throttle: 1.0,
                ..FlightInput::default()
            })
            .unwrap_or_else(|error| panic!("{error}"));
        let before = Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e}"));
        world
            .request_fire_front_pulse(world.snapshot().tick)
            .unwrap_or_else(|error| panic!("{error}"));
        let after = Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e}"));
        assert!(after.flight.speed > 0.0);
        assert!(after.energy < before.energy);
        assert!(after.heat > before.heat);
        assert_eq!(after.shield, before.shield);
        assert_eq!(after.fuel, before.fuel);
        assert_eq!(after.status(), "FLIGHT");
    }
    #[test]
    fn original_scanner_axes_and_view_bearings_do_not_confuse_rear_with_front() {
        let world = world();
        let mut c = Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e}"));
        c.station = Some([0.0, 0.0, 100.0]);
        assert_eq!(c.station_navigation(), Some((100.0, 0.0, 0.0)));
        c.view = FlightView::Rear;
        assert_eq!(
            c.station_navigation()
                .unwrap_or_else(|| panic!("missing station"))
                .1
                .abs(),
            180.0
        );
        c.station = Some([-100.0, 0.0, 0.0]);
        c.view = FlightView::Left;
        assert_eq!(
            c.station_navigation()
                .unwrap_or_else(|| panic!("missing station"))
                .1,
            0.0
        );
        c.view = FlightView::Right;
        assert_eq!(
            c.station_navigation()
                .unwrap_or_else(|| panic!("missing station"))
                .1
                .abs(),
            180.0
        );
        c.view = FlightView::Front;
        c.station = Some([0.0, 100.0, 100.0]);
        assert_eq!(
            c.station_navigation()
                .unwrap_or_else(|| panic!("missing station"))
                .2,
            45.0
        );
    }
    #[test]
    fn gauges_are_clamped_and_unknown_readings_have_no_filled_bar() {
        let mut out = Vec::new();
        let size = [640.0, 360.0];
        let l = Layout::new(640, 360);
        for value in [None, Some(f32::NAN), Some(-1.0), Some(2.0)] {
            out.clear();
            gauge(
                &mut out,
                Gauge {
                    name: "FUEL",
                    value,
                    color: GREEN,
                },
                12.0,
                l.top + l.row,
                l,
                size,
            );
            let green = out.iter().filter(|v| v.color == GREEN).count();
            if value.is_some_and(|v| v == 2.0) {
                assert_eq!(green, 6);
            } else {
                assert_eq!(green, 0);
            }
            assert!(
                out.iter()
                    .all(|v| v.position.into_iter().all(f32::is_finite))
            );
        }
    }
    #[test]
    fn cockpit_and_all_fourteen_control_rows_fit_compact_and_large_views_without_allocation_growth()
    {
        let world = world();
        let c = Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e}"));
        let lines = vec!["W".repeat(super::super::MAX_COLUMNS); super::super::MAX_LINES];
        let mut out = Vec::with_capacity(super::super::MAX_VERTICES);
        let capacity = out.capacity();
        for [w, h] in [
            [0, 0],
            [1, 1],
            [320, 240],
            [640, 360],
            [1280, 720],
            [2560, 1600],
        ] {
            super::super::vertices_with_cockpit(&lines, w, h, Some(&c), &mut out);
            assert!(out.len() <= super::super::MAX_VERTICES);
            assert_eq!(out.capacity(), capacity);
            assert!(out.iter().all(|v| {
                v.position
                    .into_iter()
                    .all(|n| n.is_finite() && n.abs() <= 1.000001)
            }));
            if w >= 640 {
                assert_eq!(
                    super::super::line_budget(w, Layout::new(w, h).top as u32),
                    14
                );
            }
        }
    }
    #[test]
    fn reference_layout_breakpoints_keep_logical_point_geometry() {
        let compact = Layout::new(640, 360);
        assert_eq!(
            (
                compact.top,
                compact.row,
                compact.radar,
                compact.side,
                compact.scale,
                compact.left_columns
            ),
            (225.0, 17.0, 192.0, 200.0, 1.25, 1)
        );
        let large = Layout::new(1280, 720);
        assert_eq!(
            (
                large.top,
                large.row,
                large.radar,
                large.side,
                large.scale,
                large.left_columns
            ),
            (576.0, 20.0, 384.0, 424.0, 1.25, 2)
        );
        assert_eq!(Layout::new(639, 360).instrument_scale, 1.25);
        assert_eq!(Layout::new(640, 360).instrument_scale, 1.5);
        assert_eq!(Layout::new(1100, 720).instrument_scale, 2.0);
    }
    #[test]
    fn title_world_has_unknown_resources_instead_of_full_fake_needles() {
        let world = World::at_original_station().unwrap_or_else(|error| panic!("{error}"));
        let c = Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(c.shield, [None; 2]);
        assert_eq!(c.energy, None);
        assert_eq!(c.fuel, None);
        assert_eq!(c.heat, None);
    }
}

#[cfg(test)]
mod traffic_tests;

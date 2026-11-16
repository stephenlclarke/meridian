//! Original SH028/SH025 station scenario, source a11fe16924d87d06021816f9037952afcbac08db.
//! The native LOD is an explicit bounded collision representation, not full display equivalence.

use crate::docking::{Basis3, DockingPort, HullMesh, RigidPose, StationPose, Vec3};
use core::fmt;

pub(crate) mod beam;

const STATION_BYTES: &[u8] = include_bytes!("../../../assets/legacy/sh028.physics");
const HULL_BYTES: &[u8] = include_bytes!("../../../assets/legacy/sh025.physics");
const PORT_BYTES: &[u8] = include_bytes!("../../../assets/legacy/sh028.port");
pub const SOURCE_REVISION: &str = "a11fe16924d87d06021816f9037952afcbac08db";
pub const STATION_SOURCE_SHA256: &str =
    "65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9";
pub const HULL_SOURCE_SHA256: &str =
    "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883";
/// Game.prepareNativeLaunchBodies uses8; native movement/launch converts speed by18.
pub const LAUNCH_SPEED: f32 = 8.0;
pub const DISTANCE_PER_SPEED_UNIT: f32 = 18.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightMode {
    Docked,
    Launching,
    Flight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionRefusal {
    StationContact,
    EntryConditions,
    CelestialContact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationError {
    ContentMismatch,
    InvalidMesh,
    InvalidPort,
    InvalidPose,
    UnsafeLaunch,
}

impl fmt::Display for StationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ContentMismatch => "station assets differ from the pinned original content",
            Self::InvalidMesh => "invalid native collision mesh",
            Self::InvalidPort => "invalid original station port",
            Self::InvalidPose => "invalid station flight pose",
            Self::UnsafeLaunch => "original station exit is not clear",
        })
    }
}
impl std::error::Error for StationError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightStart {
    pub position: [f32; 3],
    pub orientation: [f32; 4],
    /// Original game speed units; physical station-space speed is this value times18.
    pub speed: f32,
}

#[derive(Debug)]
struct CollisionMesh {
    vertices: Vec<Vec3>,
    faces: Vec<Vec<usize>>,
    minimum: Vec3,
    maximum: Vec3,
    radius: f64,
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn scale(a: Vec3, s: f64) -> Vec3 {
    Vec3::new(a.x * s, a.y * s, a.z * s)
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
fn finite(v: Vec3) -> bool {
    [v.x, v.y, v.z]
        .iter()
        .all(|x| x.is_finite() && x.abs() <= 1e12)
}
fn vector(v: [f32; 3]) -> Vec3 {
    Vec3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2]))
}
fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, StationError> {
    let b = bytes
        .get(offset..offset + 4)
        .ok_or(StationError::InvalidMesh)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn f64_at(bytes: &[u8], offset: usize) -> Result<f64, StationError> {
    let b = bytes
        .get(offset..offset + 8)
        .ok_or(StationError::InvalidMesh)?;
    let value = f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    value
        .is_finite()
        .then_some(value)
        .ok_or(StationError::InvalidMesh)
}
fn vertex_at(bytes: &[u8], offset: usize) -> Result<Vec3, StationError> {
    let v = Vec3::new(
        f64_at(bytes, offset)?,
        f64_at(bytes, offset + 8)?,
        f64_at(bytes, offset + 16)?,
    );
    finite(v).then_some(v).ok_or(StationError::InvalidMesh)
}

impl CollisionMesh {
    fn decode(bytes: &[u8]) -> Result<Self, StationError> {
        if bytes.get(..8) != Some(b"MPHYS001") {
            return Err(StationError::InvalidMesh);
        }
        let vertices_count = u32_at(bytes, 8)? as usize;
        let faces_count = u32_at(bytes, 12)? as usize;
        if !(4..=1_000_000).contains(&vertices_count) || !(4..=1_000_000).contains(&faces_count) {
            return Err(StationError::InvalidMesh);
        }
        let face_start = 16usize
            .checked_add(
                vertices_count
                    .checked_mul(24)
                    .ok_or(StationError::InvalidMesh)?,
            )
            .ok_or(StationError::InvalidMesh)?;
        let size = face_start
            .checked_add(
                faces_count
                    .checked_mul(12)
                    .ok_or(StationError::InvalidMesh)?,
            )
            .ok_or(StationError::InvalidMesh)?;
        if bytes.len() != size {
            return Err(StationError::InvalidMesh);
        }
        let mut vertices = Vec::with_capacity(vertices_count);
        for i in 0..vertices_count {
            vertices.push(vertex_at(bytes, 16 + i * 24)?);
        }
        let mut faces = Vec::with_capacity(faces_count);
        for i in 0..faces_count {
            let face = vec![
                u32_at(bytes, face_start + i * 12)? as usize,
                u32_at(bytes, face_start + i * 12 + 4)? as usize,
                u32_at(bytes, face_start + i * 12 + 8)? as usize,
            ];
            if face.iter().any(|index| *index >= vertices_count) {
                return Err(StationError::InvalidMesh);
            }
            faces.push(face);
        }
        let minimum = Vec3::new(
            vertices.iter().map(|v| v.x).fold(f64::INFINITY, f64::min),
            vertices.iter().map(|v| v.y).fold(f64::INFINITY, f64::min),
            vertices.iter().map(|v| v.z).fold(f64::INFINITY, f64::min),
        );
        let maximum = Vec3::new(
            vertices
                .iter()
                .map(|v| v.x)
                .fold(f64::NEG_INFINITY, f64::max),
            vertices
                .iter()
                .map(|v| v.y)
                .fold(f64::NEG_INFINITY, f64::max),
            vertices
                .iter()
                .map(|v| v.z)
                .fold(f64::NEG_INFINITY, f64::max),
        );
        let radius = vertices
            .iter()
            .map(|v| v.dot(*v).sqrt())
            .fold(0.0, f64::max);
        if radius <= 0.0 {
            return Err(StationError::InvalidMesh);
        }
        Ok(Self {
            vertices,
            faces,
            minimum,
            maximum,
            radius,
        })
    }
    fn borrowed(&self) -> HullMesh<'_> {
        HullMesh {
            vertices: &self.vertices,
            faces: &self.faces,
        }
    }
}

/// Immutable source-bound scene data. External byte inputs must match every packaged byte.
/// No caller-supplied descriptive ID can label different geometry as SH028/SH025.
#[derive(Debug)]
pub struct OriginalStationData {
    station: CollisionMesh,
    hull: CollisionMesh,
    port: DockingPort,
}

impl OriginalStationData {
    pub(crate) fn player_radius(&self) -> f64 {
        self.hull.radius
    }

    pub fn bundled() -> Result<Self, StationError> {
        Self::from_bytes(STATION_BYTES, HULL_BYTES, PORT_BYTES)
    }
    pub fn from_bytes(station: &[u8], hull: &[u8], port: &[u8]) -> Result<Self, StationError> {
        if station != STATION_BYTES || hull != HULL_BYTES || port != PORT_BYTES {
            return Err(StationError::ContentMismatch);
        }
        if port.get(..8) != Some(b"MPORT001") {
            return Err(StationError::InvalidPort);
        }
        let count = u32_at(port, 88)? as usize;
        if !(3..=64).contains(&count) || port.len() != 92 + 24 * count {
            return Err(StationError::InvalidPort);
        }
        let mut aperture = Vec::with_capacity(count);
        for i in 0..count {
            aperture.push(vertex_at(port, 92 + i * 24)?);
        }
        let docking = DockingPort::new(
            vertex_at(port, 8)?,
            vertex_at(port, 32)?,
            vertex_at(port, 56)?,
            f64_at(port, 80)?,
            aperture,
        )
        .map_err(|_| StationError::InvalidPort)?;
        let result = Self {
            station: CollisionMesh::decode(station)?,
            hull: CollisionMesh::decode(hull)?,
            port: docking,
        };
        if !result
            .port
            .fits_aligned_mesh(result.hull.borrowed(), 0.0)
            .map_err(|_| StationError::InvalidPort)?
        {
            return Err(StationError::InvalidPort);
        }
        result.launch_start()?;
        Ok(result)
    }
    pub fn port(&self) -> &DockingPort {
        &self.port
    }
    /// OriginalStationSite.initialSpinRate is0. The local scene uses the station as origin.
    pub fn station_pose(&self) -> StationPose {
        StationPose {
            pose: RigidPose {
                position: Vec3::new(0.0, 0.0, 0.0),
                orientation: Basis3::IDENTITY,
            },
            axial_rotation: 0.0,
            rotation_rate: 0.0,
        }
    }
    pub fn station_triangle_count(&self) -> usize {
        self.station.faces.len()
    }
    pub fn hull_triangle_count(&self) -> usize {
        self.hull.faces.len()
    }
    /// NativeStationGeometry.exit: complete station support - actual hull rear +1000.
    pub fn launch_start(&self) -> Result<FlightStart, StationError> {
        let z = self.station.maximum.z - self.hull.minimum.z + 1000.0;
        let rounded = z as f32;
        // Round outward, never inward, when publishing the f32 presentation/flight pose.
        let rounded = if f64::from(rounded) < z {
            f32::from_bits(rounded.to_bits() + 1)
        } else {
            rounded
        };
        let start = FlightStart {
            position: [0.0, 0.0, rounded],
            orientation: [0.0, 0.0, 0.0, 1.0],
            speed: LAUNCH_SPEED,
        };
        let pose =
            flight_pose(start.position, start.orientation).ok_or(StationError::InvalidPose)?;
        // Outward motion increases the source separating-plane gap; check the rounded pose too.
        let rear = self
            .hull
            .vertices
            .iter()
            .map(|v| add(pose.position, pose.orientation.apply(*v)).z)
            .fold(f64::INFINITY, f64::min);
        if rear < self.station.maximum.z + 1000.0 {
            return Err(StationError::UnsafeLaunch);
        }
        let end = RigidPose {
            position: add(pose.position, Vec3::new(0.0, 0.0, 1000.0)),
            orientation: pose.orientation,
        };
        if !self.motion_clear(pose, end, 0.0) {
            return Err(StationError::UnsafeLaunch);
        }
        Ok(start)
    }
    pub fn depth(&self, pose: RigidPose) -> f64 {
        pose.position.z - self.port.center().z
    }
    pub(crate) fn crosses_aperture(&self, start: RigidPose, end: RigidPose) -> bool {
        let old = self.depth(start);
        let next = self.depth(end);
        if old <= 0.0 || next > 0.0 {
            return false;
        }
        let fraction = old / (old - next);
        let point = sub(
            add(
                start.position,
                scale(sub(end.position, start.position), fraction),
            ),
            self.port.center(),
        );
        let aperture = self.port.aperture();
        aperture.iter().enumerate().all(|(i, a)| {
            let edge = sub(aperture[(i + 1) % aperture.len()], *a);
            edge.x * (point.y - a.y) - edge.y * (point.x - a.x) > 0.0
        })
    }
    pub fn initial_flight_is_clear(&self, start: FlightStart) -> bool {
        let Some(pose) = flight_pose(start.position, start.orientation) else {
            return false;
        };
        self.depth(pose) > 0.0 && self.motion_clear(pose, pose, 0.0)
    }
    /// Conservative continuous hull envelope against every native station triangle.
    /// Rotational displacement is bounded by radius*total angular travel, not endpoint sampling.
    pub fn motion_clear(&self, start: RigidPose, end: RigidPose, angular_travel: f64) -> bool {
        if !finite(start.position)
            || !finite(end.position)
            || !start.orientation.is_valid_rotation()
            || !end.orientation.is_valid_rotation()
            || !angular_travel.is_finite()
            || angular_travel < 0.0
        {
            return false;
        }
        // Physical World poses come from normalized quaternions. Refuse a stretch,
        // shear or reflection rather than pretending it is a rotation path.
        for basis in [start.orientation, end.orientation] {
            if (basis.x.dot(cross(basis.y, basis.z)) - 1.0).abs() > 1e-10
                || [basis.x, basis.y, basis.z]
                    .iter()
                    .any(|axis| (axis.dot(*axis) - 1.0).abs() > 1e-10)
                || basis.x.dot(basis.y).abs() > 1e-10
                || basis.x.dot(basis.z).abs() > 1e-10
                || basis.y.dot(basis.z).abs() > 1e-10
            {
                return false;
            }
        }
        let trace = start.orientation.x.dot(end.orientation.x)
            + start.orientation.y.dot(end.orientation.y)
            + start.orientation.z.dot(end.orientation.z);
        let observed_angle = ((trace - 1.0) * 0.5).clamp(-1.0, 1.0).acos();
        // Include rounding in the f32 flight quaternion; never trust an understated sweep.
        let angular_roundoff = 1e-6;
        if observed_angle > angular_travel + angular_roundoff {
            return false;
        }
        let basis_stretch = (1.0001_f64.powi(2) + 2.0 * 0.0001).sqrt();
        let padding = self.numeric_margin(start, end)
            + self.hull.radius * basis_stretch * (angular_travel + angular_roundoff).min(2.0);
        let mut low = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut high = Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for vertex in &self.hull.vertices {
            let v = start.orientation.apply(*vertex);
            low = Vec3::new(low.x.min(v.x), low.y.min(v.y), low.z.min(v.z));
            high = Vec3::new(high.x.max(v.x), high.y.max(v.y), high.z.max(v.z));
        }
        let offset = scale(add(low, high), 0.5);
        let half = add(
            scale(sub(high, low), 0.5),
            Vec3::new(padding, padding, padding),
        );
        let center = add(start.position, offset);
        let movement = sub(end.position, start.position);
        let swept_low = add(
            Vec3::new(
                start.position.x.min(end.position.x),
                start.position.y.min(end.position.y),
                start.position.z.min(end.position.z),
            ),
            sub(low, Vec3::new(padding, padding, padding)),
        );
        let swept_high = add(
            Vec3::new(
                start.position.x.max(end.position.x),
                start.position.y.max(end.position.y),
                start.position.z.max(end.position.z),
            ),
            add(high, Vec3::new(padding, padding, padding)),
        );
        if swept_high.x < self.station.minimum.x
            || swept_low.x > self.station.maximum.x
            || swept_high.y < self.station.minimum.y
            || swept_low.y > self.station.maximum.y
            || swept_high.z < self.station.minimum.z
            || swept_low.z > self.station.maximum.z
        {
            return true;
        }
        !self.station.faces.iter().any(|face| {
            swept_box_hits_triangle(
                center,
                half,
                movement,
                [
                    self.station.vertices[face[0]],
                    self.station.vertices[face[1]],
                    self.station.vertices[face[2]],
                ],
            )
        })
    }
    fn numeric_margin(&self, start: RigidPose, end: RigidPose) -> f64 {
        let scale = start
            .position
            .x
            .abs()
            .max(start.position.y.abs())
            .max(start.position.z.abs())
            .max(end.position.x.abs())
            .max(end.position.y.abs())
            .max(end.position.z.abs())
            + self.hull.radius
            + 1.0;
        64.0 * f64::from(f32::EPSILON) * scale
    }
    /// Original native-station plane-crossing/pose/speed/roll/aperture checks, no receipt shortcut.
    pub fn entry_clear(
        &self,
        start: RigidPose,
        end: RigidPose,
        physical_speed: f64,
        roll_rate: f64,
        angular_travel: f64,
    ) -> bool {
        let old = self.depth(start);
        let depth = self.depth(end);
        if !(old > 0.0 && depth <= 0.0 && depth > -self.port.tunnel_depth())
            || !physical_speed.is_finite()
            || !(0.0..144.0).contains(&physical_speed)
            || !roll_rate.is_finite()
            || roll_rate.abs() >= 0.1
            || end.orientation.z.z >= -0.995
            || end.orientation.y.y <= 0.995
        {
            return false;
        }
        let a = sub(start.position, self.port.center());
        let b = sub(end.position, self.port.center());
        let margin = (self.hull.radius + a.dot(a).sqrt().max(b.dot(b).sqrt())) * angular_travel
            + self.numeric_margin(start, end);
        if !margin.is_finite() {
            return false;
        }
        let station = self.station_pose().pose;
        self.port.fits_aligned_mesh(self.hull.borrowed(), margin) == Ok(true)
            && self
                .port
                .fits_transformed_mesh(self.hull.borrowed(), start, station, margin)
                == Ok(true)
            && self
                .port
                .fits_transformed_mesh(self.hull.borrowed(), end, station, margin)
                == Ok(true)
            && self.motion_clear(start, end, angular_travel)
    }
}

// Explicit normalized quaternion adapter; it never replaces the authoritative World pose.
pub(crate) fn flight_pose(position: [f32; 3], q: [f32; 4]) -> Option<RigidPose> {
    if position.iter().any(|x| !x.is_finite() || x.abs() > 1e12) || q.iter().any(|x| !x.is_finite())
    {
        return None;
    }
    let norm = q.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
    if norm <= f64::MIN_POSITIVE || !norm.is_finite() {
        return None;
    }
    let [x, y, z, w] = q.map(|x| f64::from(x) / norm);
    let orientation = Basis3 {
        x: Vec3::new(
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + w * z),
            2.0 * (x * z - w * y),
        ),
        y: Vec3::new(
            2.0 * (x * y - w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z + w * x),
        ),
        z: Vec3::new(
            2.0 * (x * z + w * y),
            2.0 * (y * z - w * x),
            1.0 - 2.0 * (x * x + y * y),
        ),
    };
    orientation.is_valid_rotation().then_some(RigidPose {
        position: vector(position),
        orientation,
    })
}

/// Continuous SAT: time intervals overlap on box axes, triangle normal and edge cross axes.
/// The fixed box contains every intermediate rotating hull point; false contacts are possible.
fn swept_box_hits_triangle(center: Vec3, half: Vec3, movement: Vec3, triangle: [Vec3; 3]) -> bool {
    let axes = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    ];
    let edges = [
        sub(triangle[1], triangle[0]),
        sub(triangle[2], triangle[1]),
        sub(triangle[0], triangle[2]),
    ];
    let mut candidates = [Vec3::new(0.0, 0.0, 0.0); 13];
    candidates[..3].copy_from_slice(&axes);
    candidates[3] = cross(edges[0], edges[1]);
    for (i, edge) in edges.iter().enumerate() {
        for (j, axis) in axes.iter().enumerate() {
            candidates[4 + i * 3 + j] = cross(*edge, *axis);
        }
    }
    let mut enter = 0.0_f64;
    let mut exit = 1.0_f64;
    for axis in candidates {
        if axis.dot(axis) == 0.0 {
            continue;
        }
        let p = triangle.map(|v| v.dot(axis));
        let low = p.into_iter().fold(f64::INFINITY, f64::min);
        let high = p.into_iter().fold(f64::NEG_INFINITY, f64::max);
        let radius = half.x * axis.x.abs() + half.y * axis.y.abs() + half.z * axis.z.abs();
        let c = center.dot(axis);
        let v = movement.dot(axis);
        if ![low, high, radius, c, v].iter().all(|x| x.is_finite()) {
            return true;
        }
        if v == 0.0 {
            if c < low - radius || c > high + radius {
                return false;
            }
        } else {
            let t0 = (low - radius - c) / v;
            let t1 = (high + radius - c) / v;
            enter = enter.max(t0.min(t1));
            exit = exit.min(t0.max(t1));
            if enter > exit {
                return false;
            }
        }
    }
    enter <= exit
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{FIXED_DELTA_SECONDS, FlightInput, StepError, World};

    fn returning_start(x: f32, speed: f32) -> FlightStart {
        FlightStart {
            position: [x, 0.0, 260.0],
            orientation: [0.0, 1.0, 0.0, 0.0],
            speed,
        }
    }

    #[test]
    fn exact_original_native_meshes_port_and_rounded_launch_support_are_bound() {
        let data = OriginalStationData::bundled().unwrap();
        assert_eq!(data.station.vertices.len(), 272);
        assert_eq!(data.station_triangle_count(), 488);
        assert_eq!(data.hull.vertices.len(), 536);
        assert_eq!(data.hull_triangle_count(), 964);
        assert_eq!(data.port.center(), Vec3::new(0.0, 0.0, 136.32));
        assert_eq!(data.port.tunnel_depth(), 53.52);
        let launch = data.launch_start().unwrap();
        let rear = f64::from(launch.position[2]) + data.hull.minimum.z;
        assert!(rear >= data.station.maximum.z + 1000.0);
        assert_eq!(launch.orientation, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(launch.speed, 8.0);
        let mut altered = STATION_BYTES.to_vec();
        altered[16] ^= 1;
        assert!(matches!(
            OriginalStationData::from_bytes(&altered, HULL_BYTES, PORT_BYTES),
            Err(StationError::ContentMismatch)
        ));
        assert!(matches!(
            OriginalStationData::from_bytes(STATION_BYTES, HULL_BYTES, &[]),
            Err(StationError::ContentMismatch)
        ));
    }

    #[test]
    fn public_launch_fly_turn_approach_and_dock_uses_only_commands_and_ticks() {
        let mut world = World::at_original_station().unwrap();
        assert_eq!(world.snapshot().mode, FlightMode::Docked);
        world.request_launch(world.snapshot().tick).unwrap();
        assert_eq!(world.snapshot().mode, FlightMode::Launching);
        assert_eq!(world.snapshot().speed, 8.0);
        let launch = world.snapshot().position;
        // Real outward flight and ordinary braking, then an actual controlled half-turn.
        for _ in 0..80 {
            world
                .step(FlightInput {
                    brake: true,
                    ..FlightInput::default()
                })
                .unwrap();
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        assert_eq!(world.snapshot().speed, 0.0);
        assert!(world.snapshot().position[2] > launch[2]);
        for _ in 0..400 {
            let q = world.snapshot().orientation;
            let angle = 2.0 * f64::from(q[1]).atan2(f64::from(q[3]));
            let remaining = core::f64::consts::PI - angle;
            if remaining <= 1e-6 {
                break;
            }
            let yaw = (remaining / (0.65 * f64::from(FIXED_DELTA_SECONDS))).clamp(0.0, 1.0) as f32;
            world
                .step(FlightInput {
                    yaw,
                    ..FlightInput::default()
                })
                .unwrap();
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        let turned = flight_pose(world.snapshot().position, world.snapshot().orientation).unwrap();
        assert!(turned.orientation.z.z < -0.99999);
        let mut docked = false;
        for _ in 0..4000 {
            let speed = world.snapshot().speed;
            let throttle = if speed < 3.9 {
                1.0
            } else if speed > 4.1 {
                -1.0
            } else {
                0.0
            };
            world
                .step(FlightInput {
                    throttle,
                    ..FlightInput::default()
                })
                .unwrap();
            let state = world.snapshot();
            assert_eq!(
                state.motion_refusal, None,
                "clear centered approach was refused at {:?}",
                state.position
            );
            if state.mode == FlightMode::Docked {
                docked = true;
                break;
            }
        }
        assert!(
            docked,
            "public physical approach did not reach validated entry"
        );
        assert_eq!(world.snapshot().speed, 0.0);
        assert!(world.snapshot().position[2] <= 136.32);
        let arrived = world.snapshot();
        assert_eq!(world.request_launch(0), Err(StepError::StaleCommand));
        assert_eq!(world.snapshot(), arrived);
        world.request_launch(arrived.tick).unwrap();
        assert_eq!(world.snapshot().mode, FlightMode::Launching);
    }

    #[test]
    fn launch_duplicate_stale_and_invalid_input_are_transactional() {
        let mut world = World::at_original_station().unwrap();
        let initial = world.snapshot();
        assert_eq!(world.request_launch(1), Err(StepError::StaleCommand));
        assert_eq!(world.snapshot(), initial);
        assert_eq!(
            world.step(FlightInput {
                pitch: f32::NAN,
                ..FlightInput::default()
            }),
            Err(StepError::NonFiniteInput)
        );
        assert_eq!(world.snapshot(), initial);
        world.request_launch(0).unwrap();
        let launched = world.snapshot();
        assert_eq!(world.request_launch(0), Err(StepError::InvalidMode));
        assert_eq!(world.snapshot(), launched);
        assert_eq!(
            world.step(FlightInput {
                throttle: 1.1,
                ..FlightInput::default()
            }),
            Err(StepError::InputOutOfRange)
        );
        assert_eq!(world.snapshot(), launched);
        world.step(FlightInput::default()).unwrap();
        assert_eq!(world.snapshot().mode, FlightMode::Flight);
        assert_eq!(world.request_launch(0), Err(StepError::StaleCommand));
    }

    #[test]
    fn actual_public_overspeed_approach_stops_outside_and_can_recover() {
        let mut world = World::in_flight_at_original_station(returning_start(0.0, 9.0)).unwrap();
        let mut refused = false;
        for _ in 0..100 {
            let before = world.snapshot();
            world.step(FlightInput::default()).unwrap();
            let after = world.snapshot();
            if after.motion_refusal.is_some() {
                assert_eq!(after.motion_refusal, Some(MotionRefusal::EntryConditions));
                assert_eq!(after.mode, FlightMode::Flight);
                assert_eq!(after.speed, 0.0);
                assert_eq!(after.position, before.position);
                assert_eq!(after.orientation, before.orientation);
                assert!(after.position[2] > 136.32);
                refused = true;
                break;
            }
        }
        assert!(refused);
        world
            .step(FlightInput {
                brake: true,
                ..FlightInput::default()
            })
            .unwrap();
        assert_eq!(world.snapshot().mode, FlightMode::Flight);
        assert_eq!(world.snapshot().motion_refusal, None);
    }

    #[test]
    fn actual_public_sidewall_and_fast_tunneling_course_never_dock_or_penetrate() {
        for speed in [4.0, 34.3] {
            let mut world =
                World::in_flight_at_original_station(returning_start(100.0, speed)).unwrap();
            let mut contact = false;
            for _ in 0..200 {
                let before = world.snapshot();
                world.step(FlightInput::default()).unwrap();
                let after = world.snapshot();
                assert_ne!(after.mode, FlightMode::Docked);
                if after.motion_refusal.is_some() {
                    assert_eq!(after.motion_refusal, Some(MotionRefusal::StationContact));
                    assert_eq!(after.position, before.position);
                    assert_eq!(after.speed, 0.0);
                    contact = true;
                    break;
                }
            }
            assert!(contact, "source wall was not detected at speed{speed}");
        }
    }

    #[test]
    fn rolled_return_and_unmatched_roll_refuse_real_entry() {
        let mut start = returning_start(0.0, 4.0);
        start.orientation = [0.1_f32.sin(), 0.1_f32.cos(), 0.0, 0.0];
        let mut world = World::in_flight_at_original_station(start).unwrap();
        let mut refused = false;
        for _ in 0..200 {
            world.step(FlightInput::default()).unwrap();
            let s = world.snapshot();
            assert_ne!(s.mode, FlightMode::Docked);
            if s.motion_refusal.is_some() {
                refused = true;
                break;
            }
        }
        assert!(refused);
        let data = OriginalStationData::bundled().unwrap();
        let a = flight_pose([0.0, 0.0, 137.0], [0.0, 1.0, 0.0, 0.0]).unwrap();
        let b = flight_pose([0.0, 0.0, 136.0], [0.0, 1.0, 0.0, 0.0]).unwrap();
        assert!(!data.entry_clear(a, b, 72.0, 0.1, 0.0));
        assert!(!data.entry_clear(a, b, 144.0, 0.0, 0.0));
        assert!(!data.entry_clear(a, a, 72.0, 0.0, 0.0)); // no crossing, no proximity admission
    }

    #[test]
    fn continuous_triangle_test_refuses_clear_endpoints_with_an_obstructed_path() {
        let data = OriginalStationData::bundled().unwrap();
        let a = flight_pose([100.0, 0.0, 400.0], [0.0, 1.0, 0.0, 0.0]).unwrap();
        let b = flight_pose([100.0, 0.0, -400.0], [0.0, 1.0, 0.0, 0.0]).unwrap();
        assert!(data.motion_clear(a, a, 0.0));
        assert!(data.motion_clear(b, b, 0.0));
        assert!(!data.motion_clear(a, b, 0.0));
        let rotated = flight_pose([100.0, 0.0, 400.0], [0.0, 0.0, 0.0, 1.0]).unwrap();
        assert!(!data.motion_clear(a, rotated, 0.0)); // understated rotation cannot claim clearance
        let triangle = [
            Vec3::new(0.0, -2.0, -2.0),
            Vec3::new(0.0, 2.0, -2.0),
            Vec3::new(0.0, 0.0, 2.0),
        ];
        assert!(swept_box_hits_triangle(
            Vec3::new(-10.0, 0.0, 0.0),
            Vec3::new(0.1, 0.1, 0.1),
            Vec3::new(20.0, 0.0, 0.0),
            triangle
        ));
        assert!(!swept_box_hits_triangle(
            Vec3::new(-10.0, 5.0, 0.0),
            Vec3::new(0.1, 0.1, 0.1),
            Vec3::new(20.0, 0.0, 0.0),
            triangle
        ));
    }

    #[test]
    fn invalid_initial_flight_cannot_create_a_dock_or_launder_content() {
        for position in [[0.0, 0.0, 0.0], [f32::NAN, 0.0, 260.0]] {
            let mut start = returning_start(0.0, 4.0);
            start.position = position;
            assert!(World::in_flight_at_original_station(start).is_err());
        }
        let mut start = returning_start(0.0, 4.0);
        start.orientation = [0.0; 4];
        assert!(World::in_flight_at_original_station(start).is_err());
        start = returning_start(0.0, f32::INFINITY);
        assert!(World::in_flight_at_original_station(start).is_err());
        start = returning_start(0.0, 35.0);
        assert!(World::in_flight_at_original_station(start).is_err());
        let world = World::new();
        let mut descriptive = world.snapshot();
        descriptive.mode = FlightMode::Docked;
        assert_eq!(world.snapshot().mode, FlightMode::Flight);
        assert_ne!(world.snapshot(), descriptive);
    }

    #[test]
    fn crossing_the_infinite_port_plane_far_from_the_station_does_not_dock_or_stop() {
        let mut world = World::in_flight_at_original_station(returning_start(1000.0, 4.0)).unwrap();
        for _ in 0..200 {
            world.step(FlightInput::default()).unwrap();
            assert_eq!(world.snapshot().mode, FlightMode::Flight);
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        assert!(world.snapshot().position[2] < 136.32);
    }
}

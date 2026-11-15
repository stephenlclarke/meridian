//! Pure geometry from Elite/L337 a11fe16924d87d06021816f9037952afcbac08db.
//!
//! Sources: DockingGeometry.swift; AutoTargetControl.swift:5-39;
//! NativeStationGeometry.swift:18-70; NativeStationRuntime.swift::nativeStationHullFits;
//! L337Math/{Vector3,Basis3}.swift. See PR-docking-geometry-primitives.md for hashes.
//! Copyright (c) 2026 Stephen Clarke. Reimplemented under the reference MIT license.
//!
//! A passing gate/aperture test is a necessary geometric condition, not docking,
//! collision clearance, a station identity, a receipt or permission to change World.

use core::fmt;

const STATION_COORDINATE_LIMIT: f64 = 1.0e12;
const ROTATION_TOLERANCE: f64 = 0.0001;
const PORT_AXIS_TOLERANCE: f64 = 1.0e-8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    fn subtract(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    fn scale(self, value: f64) -> Self {
        Self::new(self.x * value, self.y * value, self.z * value)
    }

    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    fn station_coordinate(self) -> bool {
        self.finite()
            && self.x.abs() <= STATION_COORDINATE_LIMIT
            && self.y.abs() <= STATION_COORDINATE_LIMIT
            && self.z.abs() <= STATION_COORDINATE_LIMIT
    }
}

/// Column basis: apply(v) = x*v.x + y*v.y + z*v.z, as in L337Math.Basis3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Basis3 {
    pub x: Vec3,
    pub y: Vec3,
    pub z: Vec3,
}

impl Basis3 {
    pub const IDENTITY: Self = Self {
        x: Vec3::new(1.0, 0.0, 0.0),
        y: Vec3::new(0.0, 1.0, 0.0),
        z: Vec3::new(0.0, 0.0, 1.0),
    };

    pub fn apply(self, vector: Vec3) -> Vec3 {
        self.x
            .scale(vector.x)
            .add(self.y.scale(vector.y))
            .add(self.z.scale(vector.z))
    }

    /// Reference transpose/dot transform for admitted rotation bases, not a general inverse.
    pub fn inverse_coordinates(self, vector: Vec3) -> Vec3 {
        Vec3::new(vector.dot(self.x), vector.dot(self.y), vector.dot(self.z))
    }

    /// Preserve the reference tolerance; do not silently normalize or add a determinant rule.
    pub fn is_valid_rotation(self) -> bool {
        [self.x, self.y, self.z]
            .iter()
            .all(|axis| axis.finite() && (axis.length() - 1.0).abs() < ROTATION_TOLERANCE)
            && self.x.dot(self.y).abs() < ROTATION_TOLERANCE
            && self.x.dot(self.z).abs() < ROTATION_TOLERANCE
            && self.y.dot(self.z).abs() < ROTATION_TOLERANCE
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigidPose {
    pub position: Vec3,
    pub orientation: Basis3,
}

impl RigidPose {
    fn valid(self) -> bool {
        self.position.finite() && self.orientation.is_valid_rotation()
    }

    fn system_point(self, point: Vec3) -> Vec3 {
        self.position.add(self.orientation.apply(point))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryError {
    InvalidPort,
    InvalidPose,
    InvalidMotion,
    InvalidMesh,
    InvalidMargin,
    InvalidDistance,
    NonFiniteResult,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPort => "invalid docking port geometry",
            Self::InvalidPose => "invalid rigid pose or rotation",
            Self::InvalidMotion => "non-finite entry motion",
            Self::InvalidMesh => "invalid reference mesh",
            Self::InvalidMargin => "clearance margin must be finite and nonnegative",
            Self::InvalidDistance => "approach distance must be finite and nonnegative",
            Self::NonFiniteResult => "geometry calculation overflowed",
        })
    }
}

impl std::error::Error for GeometryError {}

/// Borrowed source-style vertices/faces. Validation is not a watertight-solid proof.
#[derive(Debug, Clone, Copy)]
pub struct HullMesh<'a> {
    pub vertices: &'a [Vec3],
    pub faces: &'a [Vec<usize>],
}

impl HullMesh<'_> {
    fn valid(self) -> bool {
        self.vertices.len() >= 4
            && self
                .vertices
                .iter()
                .all(|vertex| vertex.station_coordinate())
            && self.vertices.iter().any(|vertex| vertex.length() > 0.0)
            && self.faces.len() >= 4
            && self.faces.iter().all(|face| {
                face.len() >= 3 && face.iter().all(|index| *index < self.vertices.len())
            })
    }
}

/// Explicit authored port data; no built-in station, fallback aperture or source authority.
#[derive(Debug, Clone, PartialEq)]
pub struct DockingPort {
    center: Vec3,
    outward: Vec3,
    up: Vec3,
    tunnel_depth: f64,
    aperture: Vec<Vec3>,
}

impl DockingPort {
    pub fn new(
        center: Vec3,
        outward: Vec3,
        up: Vec3,
        tunnel_depth: f64,
        aperture: Vec<Vec3>,
    ) -> Result<Self, GeometryError> {
        if !center.station_coordinate()
            || !outward.station_coordinate()
            || !up.station_coordinate()
            || (outward.length() - 1.0).abs() >= PORT_AXIS_TOLERANCE
            || (up.length() - 1.0).abs() >= PORT_AXIS_TOLERANCE
            || outward.dot(up).abs() >= PORT_AXIS_TOLERANCE
            || !tunnel_depth.is_finite()
            || tunnel_depth <= 0.0
            || !(3..=64).contains(&aperture.len())
            || !aperture
                .iter()
                .all(|point| point.station_coordinate() && point.z == 0.0)
        {
            return Err(GeometryError::InvalidPort);
        }
        for (index, a) in aperture.iter().copied().enumerate() {
            let next = (index + 1) % aperture.len();
            let edge = aperture[next].subtract(a);
            if !edge.length().is_finite() || edge.length() <= 0.0 {
                return Err(GeometryError::InvalidPort);
            }
            for (other, point) in aperture.iter().copied().enumerate() {
                if other != index && other != next {
                    let cross = edge.x * (point.y - a.y) - edge.y * (point.x - a.x);
                    if !cross.is_finite() || cross <= 0.0 {
                        return Err(GeometryError::InvalidPort);
                    }
                }
            }
        }
        Ok(Self {
            center,
            outward,
            up,
            tunnel_depth,
            aperture,
        })
    }

    pub fn center(&self) -> Vec3 {
        self.center
    }

    pub fn tunnel_depth(&self) -> f64 {
        self.tunnel_depth
    }

    pub fn aperture(&self) -> &[Vec3] {
        &self.aperture
    }

    /// NativeStationGeometry.Port.fits: strict transverse containment in local x/y.
    /// This does not sweep through the station triangles or check another actor.
    pub fn fits_aligned_mesh(
        &self,
        mesh: HullMesh<'_>,
        margin: f64,
    ) -> Result<bool, GeometryError> {
        validate_clearance_inputs(mesh, margin)?;
        // Keep the reference edge-first traversal and compute each edge length once.
        for (index, a) in self.aperture.iter().copied().enumerate() {
            let edge = self.aperture[(index + 1) % self.aperture.len()].subtract(a);
            let length = edge.length();
            for point in mesh.vertices {
                let distance = (edge.x * (point.y - a.y) - edge.y * (point.x - a.x)) / length;
                if !distance.is_finite() || distance <= margin {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// nativeStationHullFits: transform every vertex to station-local x/y minus port center.
    /// The station pose must already include axial rotation. This is a projection test,
    /// not continuous/swept collision clearance, regardless of its Boolean result.
    pub fn fits_transformed_mesh(
        &self,
        mesh: HullMesh<'_>,
        player: RigidPose,
        station: RigidPose,
        margin: f64,
    ) -> Result<bool, GeometryError> {
        validate_clearance_inputs(mesh, margin)?;
        if !player.valid() || !station.valid() {
            return Err(GeometryError::InvalidPose);
        }
        for vertex in mesh.vertices {
            let point = station
                .orientation
                .inverse_coordinates(player.system_point(*vertex).subtract(station.position))
                .subtract(self.center);
            if !point.finite() {
                return Err(GeometryError::NonFiniteResult);
            }
            if !self.point_clears_aperture(point, margin) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn point_clears_aperture(&self, point: Vec3, margin: f64) -> bool {
        for (index, a) in self.aperture.iter().copied().enumerate() {
            let edge = self.aperture[(index + 1) % self.aperture.len()].subtract(a);
            let distance = (edge.x * (point.y - a.y) - edge.y * (point.x - a.x)) / edge.length();
            // Refuse arithmetic overflow as well as boundary contact.
            if !distance.is_finite() || distance <= margin {
                return false;
            }
        }
        true
    }
}

fn validate_clearance_inputs(mesh: HullMesh<'_>, margin: f64) -> Result<(), GeometryError> {
    if !margin.is_finite() || margin < 0.0 {
        return Err(GeometryError::InvalidMargin);
    }
    if !mesh.valid() {
        return Err(GeometryError::InvalidMesh);
    }
    Ok(())
}

/// Station-only model rotation: local +Z roll first, then the column orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StationPose {
    pub pose: RigidPose,
    pub axial_rotation: f64,
    pub rotation_rate: f64,
}

impl StationPose {
    fn validate(self) -> Result<(), GeometryError> {
        if !self.pose.valid() || !self.axial_rotation.is_finite() || !self.rotation_rate.is_finite()
        {
            return Err(GeometryError::InvalidPose);
        }
        Ok(())
    }

    fn model_vector(self, vector: Vec3) -> Vec3 {
        let sine = self.axial_rotation.sin();
        let cosine = self.axial_rotation.cos();
        // Rotate the vector before applying the basis, preserving the source grouping.
        self.pose.orientation.apply(Vec3::new(
            vector.x * cosine - vector.y * sine,
            vector.x * sine + vector.y * cosine,
            vector.z,
        ))
    }

    pub fn model_pose(self) -> Result<RigidPose, GeometryError> {
        self.validate()?;
        Ok(RigidPose {
            position: self.pose.position,
            orientation: Basis3 {
                x: self.model_vector(Vec3::new(1.0, 0.0, 0.0)),
                y: self.model_vector(Vec3::new(0.0, 1.0, 0.0)),
                z: self.model_vector(Vec3::new(0.0, 0.0, 1.0)),
            },
        })
    }

    pub fn entrance_frame(self, port: &DockingPort) -> Result<EntranceFrame, GeometryError> {
        self.validate()?;
        let frame = EntranceFrame {
            position: self.pose.position.add(self.model_vector(port.center)),
            normal: self.model_vector(port.outward),
            up: self.model_vector(port.up),
            model_x: self.model_vector(Vec3::new(1.0, 0.0, 0.0)),
            model_z: self.model_vector(Vec3::new(0.0, 0.0, 1.0)),
            // Reference uses the unrolled basis z.z, not the entrance normal's z.
            match_roll_rate: -self.rotation_rate * self.pose.orientation.z.z,
        };
        if !frame.position.finite()
            || !frame.normal.finite()
            || !frame.up.finite()
            || !frame.match_roll_rate.is_finite()
        {
            return Err(GeometryError::NonFiniteResult);
        }
        Ok(frame)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntryMotion {
    pub roll: f64,
    /// Reference scalar: the gate has only speed < 144, not a lower-speed bound.
    pub speed: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortCoordinates {
    pub lateral: f64,
    pub vertical: f64,
    pub depth: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockingAssessment {
    pub coordinates: PortCoordinates,
    pub roll_phase: f64,
    pub match_roll_rate: f64,
    pub inside_entrance_region: bool,
    pub passes_entry_gate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntranceFrame {
    position: Vec3,
    normal: Vec3,
    up: Vec3,
    model_x: Vec3,
    model_z: Vec3,
    match_roll_rate: f64,
}

impl EntranceFrame {
    pub fn position(self) -> Vec3 {
        self.position
    }

    pub fn outward(self) -> Vec3 {
        self.normal
    }

    pub fn up(self) -> Vec3 {
        self.up
    }

    pub fn approach_point(self, distance: f64) -> Result<Vec3, GeometryError> {
        if !distance.is_finite() || distance < 0.0 {
            return Err(GeometryError::InvalidDistance);
        }
        let point = self.position.add(self.normal.scale(distance));
        point
            .finite()
            .then_some(point)
            .ok_or(GeometryError::NonFiniteResult)
    }

    pub fn assess(self, motion: EntryMotion) -> Result<DockingAssessment, GeometryError> {
        if !motion.roll.is_finite() || !motion.speed.is_finite() {
            return Err(GeometryError::InvalidMotion);
        }
        let relative = self.position.scale(-1.0);
        let coordinates = PortCoordinates {
            lateral: relative.dot(self.model_x),
            vertical: relative.dot(self.up),
            depth: relative.dot(self.normal),
        };
        let longitudinal = relative.dot(self.model_z);
        let roll_phase = self.up.x.atan2(self.up.y);
        if !coordinates.lateral.is_finite()
            || !coordinates.vertical.is_finite()
            || !coordinates.depth.is_finite()
            || !longitudinal.is_finite()
            || !roll_phase.is_finite()
        {
            return Err(GeometryError::NonFiniteResult);
        }
        let inside_entrance_region = coordinates.depth < 0.0
            && coordinates.lateral.abs() < 90.0
            && coordinates.vertical.abs() < 90.0
            && longitudinal.abs() < 90.0;
        let matched_rate = (motion.roll - self.match_roll_rate).abs() < 0.1;
        let passes_entry_gate = coordinates.lateral.abs() < 25.0
            && coordinates.vertical.abs() < 8.0
            && coordinates.depth <= 1.0
            && coordinates.depth > -25.0
            && self.normal.z < -0.98
            && roll_phase.abs() < 0.1
            && motion.speed < 144.0
            && (matched_rate || motion.roll.abs() < 0.15);
        Ok(DockingAssessment {
            coordinates,
            roll_phase,
            match_roll_rate: self.match_roll_rate,
            inside_entrance_region,
            passes_entry_gate,
        })
    }
}

/// AutoTargetControl's native mesh bound plus 400 reference units. No station binding implied.
pub fn station_clearance_radius(vertices: &[Vec3]) -> Result<f64, GeometryError> {
    if vertices.is_empty() || !vertices.iter().all(|vertex| vertex.station_coordinate()) {
        return Err(GeometryError::InvalidMesh);
    }
    let furthest = vertices
        .iter()
        .map(|vertex| vertex.length())
        .fold(0.0_f64, f64::max);
    Ok(furthest + 400.0)
}

/// Native approach uses clearance radius + 1000; non-native fallback stations are not inferred.
pub fn station_approach_distance(vertices: &[Vec3]) -> Result<f64, GeometryError> {
    Ok(station_clearance_radius(vertices)? + 1000.0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // SH028 nativeDocking from immutable a11, not a packaged/admitted Meridian station.
    fn reference_port() -> DockingPort {
        DockingPort::new(
            Vec3::new(0.0, 0.0, 136.32),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            53.52,
            rectangle(62.32, 50.8),
        )
        .unwrap()
    }

    fn rectangle(x: f64, y: f64) -> Vec<Vec3> {
        vec![
            Vec3::new(-x, -y, 0.0),
            Vec3::new(x, -y, 0.0),
            Vec3::new(x, y, 0.0),
            Vec3::new(-x, y, 0.0),
        ]
    }

    fn facing_pose(depth: f64, x: f64, y: f64) -> StationPose {
        StationPose {
            pose: RigidPose {
                position: Vec3::new(x, -y, 136.32 + depth),
                orientation: Basis3 {
                    x: Vec3::new(-1.0, 0.0, 0.0),
                    y: Vec3::new(0.0, 1.0, 0.0),
                    z: Vec3::new(0.0, 0.0, -1.0),
                },
            },
            axial_rotation: 0.0,
            rotation_rate: 0.0,
        }
    }

    fn assessment(depth: f64, x: f64, y: f64) -> DockingAssessment {
        facing_pose(depth, x, y)
            .entrance_frame(&reference_port())
            .unwrap()
            .assess(EntryMotion {
                roll: 0.0,
                speed: 48.0,
            })
            .unwrap()
    }

    fn box_mesh(x: f64, y: f64, z: f64) -> (Vec<Vec3>, Vec<Vec<usize>>) {
        let vertices = vec![
            Vec3::new(-x, -y, -z),
            Vec3::new(x, -y, -z),
            Vec3::new(x, y, -z),
            Vec3::new(-x, y, -z),
            Vec3::new(-x, -y, z),
            Vec3::new(x, -y, z),
            Vec3::new(x, y, z),
            Vec3::new(-x, y, z),
        ];
        let faces = vec![
            vec![0, 1, 2, 3],
            vec![4, 5, 6, 7],
            vec![0, 1, 5, 4],
            vec![1, 2, 6, 5],
            vec![2, 3, 7, 6],
            vec![3, 0, 4, 7],
        ];
        (vertices, faces)
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-10,
            "{actual} != {expected}"
        );
    }

    fn close_vector(actual: Vec3, expected: Vec3) {
        close(actual.x, expected.x);
        close(actual.y, expected.y);
        close(actual.z, expected.z);
    }

    #[test]
    fn immutable_reference_interior_and_entry_depth_fixtures() {
        // Independent expected pairs from NativeMainStationEntranceGateTests.swift.
        for (depth, inside, gate) in [
            (2.0, false, false),
            (1.0, false, true),
            (0.5, false, true),
            (0.0, false, true),
            (-0.5, true, true),
            (-1.0, true, true),
            (-24.0, true, true),
            (-25.0, true, false),
            (-89.0, true, false),
            (-90.0, false, false),
        ] {
            let result = assessment(depth, 0.0, 0.0);
            close(result.coordinates.depth, depth);
            assert_eq!(result.inside_entrance_region, inside, "depth={depth}");
            assert_eq!(result.passes_entry_gate, gate, "depth={depth}");
        }
        assert!(facing_pose(-0.5, 0.0, 0.0).pose.position.length() > 90.0);
    }

    #[test]
    fn transverse_boundaries_are_strict_and_independent() {
        for coordinate in [-90.0, 90.0] {
            assert!(!assessment(-0.5, coordinate, 0.0).inside_entrance_region);
            assert!(!assessment(-0.5, 0.0, coordinate).inside_entrance_region);
        }
        for coordinate in [-89.0, 89.0] {
            assert!(assessment(-0.5, coordinate, 0.0).inside_entrance_region);
            assert!(assessment(-0.5, 0.0, coordinate).inside_entrance_region);
        }
        for x in [-25.0, 25.0] {
            assert!(!assessment(-0.5, x, 0.0).passes_entry_gate);
        }
        for y in [-8.0, 8.0] {
            assert!(!assessment(-0.5, 0.0, y).passes_entry_gate);
        }
        assert!(assessment(-0.5, 24.99, -7.99).passes_entry_gate);
    }

    #[test]
    fn roll_phase_spin_speed_and_fallback_follow_reference_branches() {
        let port = reference_port();
        let mut station = facing_pose(-0.5, 0.0, 0.0);
        station.rotation_rate = 0.3;
        for (phase, gate) in [
            (0.099, true),
            (0.101, false),
            (-0.099, true),
            (-0.101, false),
        ] {
            station.axial_rotation = phase;
            let result = station
                .entrance_frame(&port)
                .unwrap()
                .assess(EntryMotion {
                    roll: 0.3,
                    speed: 20.0,
                })
                .unwrap();
            close(result.roll_phase, phase);
            close(result.match_roll_rate, 0.3);
            assert_eq!(result.passes_entry_gate, gate);
        }
        station.axial_rotation = 0.0;
        station.rotation_rate = 1.0;
        let frame = station.entrance_frame(&port).unwrap();
        for (roll, gate) in [(1.099, true), (1.101, false), (0.149, true), (0.15, false)] {
            assert_eq!(
                frame
                    .assess(EntryMotion { roll, speed: 48.0 })
                    .unwrap()
                    .passes_entry_gate,
                gate
            );
        }
        assert!(
            !frame
                .assess(EntryMotion {
                    roll: 1.0,
                    speed: 144.0
                })
                .unwrap()
                .passes_entry_gate
        );
        assert!(
            frame
                .assess(EntryMotion {
                    roll: 1.0,
                    speed: 143.99
                })
                .unwrap()
                .passes_entry_gate
        );
        // The scalar source rule has no lower-speed bound; this is not travel authority.
        assert!(
            frame
                .assess(EntryMotion {
                    roll: 1.0,
                    speed: -2.0
                })
                .unwrap()
                .passes_entry_gate
        );
    }

    #[test]
    fn facing_z_threshold_is_strict() {
        for (z, gate) in [(-0.98_f64, false), (-0.981_f64, true)] {
            let x = (1.0 - z * z).sqrt();
            let orientation = Basis3 {
                x: Vec3::new(z, 0.0, -x),
                y: Vec3::new(0.0, 1.0, 0.0),
                z: Vec3::new(x, 0.0, z),
            };
            let station = StationPose {
                pose: RigidPose {
                    position: orientation.apply(Vec3::new(0.0, 0.0, -136.32)),
                    orientation,
                },
                axial_rotation: 0.0,
                rotation_rate: 0.0,
            };
            assert_eq!(
                station
                    .entrance_frame(&reference_port())
                    .unwrap()
                    .assess(EntryMotion {
                        roll: 0.0,
                        speed: 48.0
                    })
                    .unwrap()
                    .passes_entry_gate,
                gate
            );
        }
    }

    #[test]
    fn column_orientation_and_local_roll_have_independent_coordinate_fixtures() {
        let yaw = Basis3 {
            x: Vec3::new(0.0, 0.0, -1.0),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(1.0, 0.0, 0.0),
        };
        let station = StationPose {
            pose: RigidPose {
                position: Vec3::new(200.0, 3.0, 4.0),
                orientation: yaw,
            },
            axial_rotation: 0.0,
            rotation_rate: 0.3,
        };
        let frame = station.entrance_frame(&reference_port()).unwrap();
        close_vector(frame.position(), Vec3::new(336.32, 3.0, 4.0));
        close_vector(frame.outward(), Vec3::new(1.0, 0.0, 0.0));
        close_vector(
            frame.approach_point(10.0).unwrap(),
            Vec3::new(346.32, 3.0, 4.0),
        );
        let result = frame
            .assess(EntryMotion {
                roll: 0.0,
                speed: 48.0,
            })
            .unwrap();
        close(result.coordinates.lateral, 4.0);
        close(result.coordinates.vertical, -3.0);
        close(result.coordinates.depth, -336.32);
        assert!(!result.passes_entry_gate);

        // Synthetic offset port: independently expanded +90-degree local Z roll then basis.
        let offset = DockingPort::new(
            Vec3::new(2.0, 3.0, 4.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            1.0,
            rectangle(10.0, 6.0),
        )
        .unwrap();
        let mut rolled = facing_pose(0.0, 0.0, 0.0);
        rolled.pose.position = Vec3::new(1.0, 2.0, 3.0);
        rolled.axial_rotation = core::f64::consts::FRAC_PI_2;
        let rotated = rolled.entrance_frame(&offset).unwrap();
        close_vector(rotated.position(), Vec3::new(4.0, 4.0, -1.0));
        close_vector(rotated.up(), Vec3::new(1.0, 0.0, 0.0));
        close(
            rotated
                .assess(EntryMotion {
                    roll: 0.0,
                    speed: 0.0,
                })
                .unwrap()
                .roll_phase,
            core::f64::consts::FRAC_PI_2,
        );
    }

    #[test]
    fn matched_roll_uses_basis_z_not_arbitrary_port_outward_z() {
        let side = DockingPort::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            1.0,
            rectangle(10.0, 6.0),
        )
        .unwrap();
        let mut station = facing_pose(0.0, 0.0, 0.0);
        station.rotation_rate = 0.3;
        let result = station
            .entrance_frame(&side)
            .unwrap()
            .assess(EntryMotion {
                roll: 0.3,
                speed: 0.0,
            })
            .unwrap();
        close(result.match_roll_rate, 0.3);
        assert!(!result.passes_entry_gate);
    }

    #[test]
    fn real_aperture_projection_and_strict_margin_are_not_corridor_clearance() {
        let (vertices, faces) = box_mesh(62.31, 50.79, 1000.0);
        let mesh = HullMesh {
            vertices: &vertices,
            faces: &faces,
        };
        assert!(reference_port().fits_aligned_mesh(mesh, 0.0).unwrap());
        assert_eq!(reference_port().tunnel_depth(), 53.52);
        // Even a hull much longer than the tunnel passes this transverse-only predicate.
        let (touching, faces) = box_mesh(62.32, 50.8, 1.0);
        assert!(
            !reference_port()
                .fits_aligned_mesh(
                    HullMesh {
                        vertices: &touching,
                        faces: &faces
                    },
                    0.0
                )
                .unwrap()
        );

        let square = DockingPort::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            1.0,
            rectangle(10.0, 6.0),
        )
        .unwrap();
        let (vertices, faces) = box_mesh(9.0, 5.0, 1.0);
        let mesh = HullMesh {
            vertices: &vertices,
            faces: &faces,
        };
        assert!(square.fits_aligned_mesh(mesh, 0.0).unwrap());
        assert!(!square.fits_aligned_mesh(mesh, 1.0).unwrap());
        let (vertices, faces) = box_mesh(8.0, 4.0, 1.0);
        assert!(
            square
                .fits_aligned_mesh(
                    HullMesh {
                        vertices: &vertices,
                        faces: &faces
                    },
                    1.0
                )
                .unwrap()
        );
    }

    #[test]
    fn triangle_half_plane_fixture_refuses_exact_edge_contact() {
        let port = DockingPort::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            1.0,
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(6.0, 0.0, 0.0),
                Vec3::new(0.0, 6.0, 0.0),
            ],
        )
        .unwrap();
        let mut vertices = vec![
            Vec3::new(0.5, 0.5, -1.0),
            Vec3::new(1.5, 0.5, -1.0),
            Vec3::new(0.5, 1.5, -1.0),
            Vec3::new(0.5, 0.5, 1.0),
        ];
        let faces = vec![vec![0, 1, 2], vec![0, 1, 3], vec![1, 2, 3], vec![2, 0, 3]];
        assert!(
            port.fits_aligned_mesh(
                HullMesh {
                    vertices: &vertices,
                    faces: &faces
                },
                0.0
            )
            .unwrap()
        );
        vertices[0] = Vec3::new(4.0, 2.0, -1.0);
        assert!(
            !port
                .fits_aligned_mesh(
                    HullMesh {
                        vertices: &vertices,
                        faces: &faces
                    },
                    0.0
                )
                .unwrap()
        );
    }

    #[test]
    fn transformed_whole_mesh_uses_orientation_translation_and_every_vertex() {
        let port = reference_port();
        let station = RigidPose {
            position: Vec3::new(0.0, 0.0, 0.0),
            orientation: Basis3::IDENTITY,
        };
        let mut player = RigidPose {
            position: Vec3::new(0.0, 0.0, 136.32),
            orientation: Basis3::IDENTITY,
        };
        let (vertices, faces) = box_mesh(60.0, 10.0, 20.0);
        let mesh = HullMesh {
            vertices: &vertices,
            faces: &faces,
        };
        assert!(
            port.fits_transformed_mesh(mesh, player, station, 0.0)
                .unwrap()
        );
        player.orientation = Basis3 {
            x: Vec3::new(0.0, 1.0, 0.0),
            y: Vec3::new(-1.0, 0.0, 0.0),
            z: Vec3::new(0.0, 0.0, 1.0),
        };
        assert!(
            !port
                .fits_transformed_mesh(mesh, player, station, 0.0)
                .unwrap()
        );
        let yaw = Basis3 {
            x: Vec3::new(0.0, 0.0, -1.0),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(1.0, 0.0, 0.0),
        };
        let station = RigidPose {
            position: Vec3::new(100.0, 200.0, 300.0),
            orientation: yaw,
        };
        player = RigidPose {
            position: Vec3::new(236.32, 200.0, 300.0),
            orientation: yaw,
        };
        assert!(
            port.fits_transformed_mesh(mesh, player, station, 0.0)
                .unwrap()
        );
        player.position.z = 200.0;
        assert!(
            !port
                .fits_transformed_mesh(mesh, player, station, 0.0)
                .unwrap()
        );
    }

    #[test]
    fn invalid_apertures_axes_depth_and_source_coordinate_ranges_refuse() {
        let center = Vec3::new(0.0, 0.0, 0.0);
        let outward = Vec3::new(0.0, 0.0, 1.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let mut clockwise = rectangle(10.0, 6.0);
        clockwise.reverse();
        let mut repeated = rectangle(10.0, 6.0);
        repeated[1] = repeated[0];
        let collinear = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let mut lifted = rectangle(10.0, 6.0);
        lifted[0].z = 0.001;
        let mut concave = rectangle(10.0, 6.0);
        concave[2] = Vec3::new(-5.0, 0.0, 0.0);
        for aperture in [
            clockwise,
            repeated,
            collinear,
            lifted,
            concave,
            vec![center; 2],
            vec![center; 65],
        ] {
            assert_eq!(
                DockingPort::new(center, outward, up, 1.0, aperture),
                Err(GeometryError::InvalidPort)
            );
        }
        for depth in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                DockingPort::new(center, outward, up, depth, rectangle(10.0, 6.0)),
                Err(GeometryError::InvalidPort)
            );
        }
        for axis in [
            center,
            Vec3::new(0.0, 0.0, 2.0),
            up,
            Vec3::new(f64::NAN, 0.0, 1.0),
        ] {
            assert_eq!(
                DockingPort::new(center, axis, up, 1.0, rectangle(10.0, 6.0)),
                Err(GeometryError::InvalidPort)
            );
        }
        assert!(
            DockingPort::new(
                Vec3::new(1.0e12, 0.0, 0.0),
                outward,
                up,
                1.0,
                rectangle(10.0, 6.0)
            )
            .is_ok()
        );
        assert_eq!(
            DockingPort::new(
                Vec3::new(1.0e12 + 1.0, 0.0, 0.0),
                outward,
                up,
                1.0,
                rectangle(10.0, 6.0)
            ),
            Err(GeometryError::InvalidPort)
        );
    }

    #[test]
    fn invalid_mesh_topology_margin_and_nonfinite_vertices_refuse() {
        let port = reference_port();
        let (mut vertices, mut faces) = box_mesh(1.0, 1.0, 1.0);
        for margin in [-1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                port.fits_aligned_mesh(
                    HullMesh {
                        vertices: &vertices,
                        faces: &faces
                    },
                    margin
                ),
                Err(GeometryError::InvalidMargin)
            );
        }
        faces[0][0] = usize::MAX;
        assert_eq!(
            port.fits_aligned_mesh(
                HullMesh {
                    vertices: &vertices,
                    faces: &faces
                },
                0.0
            ),
            Err(GeometryError::InvalidMesh)
        );
        faces[0][0] = 0;
        assert_eq!(
            port.fits_aligned_mesh(
                HullMesh {
                    vertices: &vertices,
                    faces: &faces[..3]
                },
                0.0
            ),
            Err(GeometryError::InvalidMesh)
        );
        assert_eq!(
            port.fits_aligned_mesh(
                HullMesh {
                    vertices: &vertices[..3],
                    faces: &faces
                },
                0.0
            ),
            Err(GeometryError::InvalidMesh)
        );
        for invalid in [f64::NAN, f64::INFINITY, 1.0e12 + 1.0] {
            vertices[0].x = invalid;
            assert_eq!(
                port.fits_aligned_mesh(
                    HullMesh {
                        vertices: &vertices,
                        faces: &faces
                    },
                    0.0
                ),
                Err(GeometryError::InvalidMesh)
            );
        }
        vertices.fill(Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(
            port.fits_aligned_mesh(
                HullMesh {
                    vertices: &vertices,
                    faces: &faces
                },
                0.0
            ),
            Err(GeometryError::InvalidMesh)
        );
        assert_eq!(
            port.fits_aligned_mesh(
                HullMesh {
                    vertices: &[],
                    faces: &[]
                },
                0.0
            ),
            Err(GeometryError::InvalidMesh)
        );
    }

    #[test]
    fn finite_validation_refuses_pose_motion_and_arithmetic_overflow() {
        let port = reference_port();
        let initial = facing_pose(-0.5, 0.0, 0.0);
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let frame = initial.entrance_frame(&port).unwrap();
            assert_eq!(
                frame.assess(EntryMotion {
                    roll: value,
                    speed: 48.0
                }),
                Err(GeometryError::InvalidMotion)
            );
            assert_eq!(
                frame.assess(EntryMotion {
                    roll: 0.0,
                    speed: value
                }),
                Err(GeometryError::InvalidMotion)
            );
            let mut invalid = initial;
            invalid.pose.position.x = value;
            assert_eq!(
                invalid.entrance_frame(&port),
                Err(GeometryError::InvalidPose)
            );
            invalid = initial;
            invalid.axial_rotation = value;
            assert_eq!(
                invalid.entrance_frame(&port),
                Err(GeometryError::InvalidPose)
            );
            invalid = initial;
            invalid.rotation_rate = value;
            assert_eq!(
                invalid.entrance_frame(&port),
                Err(GeometryError::InvalidPose)
            );
        }
        let mut scaled = initial;
        scaled.pose.orientation.x.x = -1.01;
        assert_eq!(
            scaled.entrance_frame(&port),
            Err(GeometryError::InvalidPose)
        );
        let yaw = Basis3 {
            x: Vec3::new(0.0, 0.0, -1.0),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(1.0, 0.0, 0.0),
        };
        let huge = StationPose {
            pose: RigidPose {
                position: Vec3::new(f64::MAX, 0.0, 0.0),
                orientation: yaw,
            },
            axial_rotation: 0.0,
            rotation_rate: 0.0,
        };
        assert_eq!(
            huge.entrance_frame(&port).unwrap().approach_point(f64::MAX),
            Err(GeometryError::NonFiniteResult)
        );
        let (vertices, faces) = box_mesh(1.0, 1.0, 1.0);
        assert_eq!(
            port.fits_transformed_mesh(
                HullMesh {
                    vertices: &vertices,
                    faces: &faces
                },
                huge.pose,
                RigidPose {
                    position: Vec3::new(-f64::MAX, 0.0, 0.0),
                    orientation: Basis3::IDENTITY
                },
                0.0
            ),
            Err(GeometryError::NonFiniteResult)
        );
        assert_eq!(initial, facing_pose(-0.5, 0.0, 0.0));
    }

    #[test]
    fn basis_validation_preserves_tolerance_without_normalizing_or_adding_handedness() {
        let reflection = Basis3 {
            x: Vec3::new(-1.0, 0.0, 0.0),
            ..Basis3::IDENTITY
        };
        assert!(reflection.is_valid_rotation());
        let near = Basis3 {
            x: Vec3::new(1.0000999, 0.0, 0.0),
            ..Basis3::IDENTITY
        };
        assert!(near.is_valid_rotation());
        assert_eq!(near.x.x, 1.0000999);
        let outside = Basis3 {
            x: Vec3::new(1.0001001, 0.0, 0.0),
            ..Basis3::IDENTITY
        };
        assert!(!outside.is_valid_rotation());
        let shear = Basis3 {
            y: Vec3::new(0.01, 1.0, 0.0),
            ..Basis3::IDENTITY
        };
        assert!(!shear.is_valid_rotation());
    }

    #[test]
    fn approach_radius_and_point_use_source_padding_without_binding_a_station() {
        let vertices = [Vec3::new(3.0, 4.0, 0.0), Vec3::new(0.0, 0.0, -12.0)];
        assert_eq!(station_clearance_radius(&vertices), Ok(412.0));
        assert_eq!(station_approach_distance(&vertices), Ok(1412.0));
        assert_eq!(
            station_clearance_radius(&[]),
            Err(GeometryError::InvalidMesh)
        );
        assert_eq!(
            station_approach_distance(&[Vec3::new(f64::NAN, 0.0, 0.0)]),
            Err(GeometryError::InvalidMesh)
        );
        let frame = facing_pose(0.5, 0.0, 0.0)
            .entrance_frame(&reference_port())
            .unwrap();
        close_vector(
            frame.approach_point(1412.0).unwrap(),
            Vec3::new(0.0, 0.0, -1411.5),
        );
        assert_eq!(
            frame.approach_point(-1.0),
            Err(GeometryError::InvalidDistance)
        );
        assert_eq!(
            frame.approach_point(f64::NAN),
            Err(GeometryError::InvalidDistance)
        );
        assert_eq!(frame.approach_point(0.0), Ok(frame.position()));
    }
}

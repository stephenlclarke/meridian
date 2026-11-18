//! SH001 source binding and a distinct conservative smooth gameplay enclosure.
//! This never grants hull ownership, equipment, crew, damage, EMP or salvage authority.
mod geometry;
use crate::docking::{Basis3, DockingPort, RigidPose, StationPose, Vec3};
pub use geometry::{EnvelopeRayContact, SmoothPiece};
use geometry::{add, finite, sub, valid_pose};
use std::sync::Arc;

const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh001/all-source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh001/source-piece-membership.bin");
pub const MULE_PIECES: usize = 16;
pub const MULE_RESOURCE_SHA256: &str =
    "1c8cce7fb8130c12780fb522592249805cb06e6b22fcffb169ae651556f47c82";
pub const MULE_CONTRACT_SHA256: &str =
    "27c960f32df859e67b969e2d9a25d9fb15f8a887a253cd20f7cd213a728bf14c";
pub const MULE_DETAILED_VERTICES: usize = 4360;
pub const MULE_NATIVE_VERTICES: usize = 984;
pub const MULE_CLEARANCE_METRES: f64 = 0.02;
pub const MULE_MAX_SHIMMER_OPACITY: f32 = 0.018;
pub const MULE_SHIMMER_NEAR_METRES: f32 = 32.0;
pub const MULE_SHIMMER_FAR_METRES: f32 = 64.0;
const SCALE: f64 = 40000.0 / 3411.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeError {
    ChangedSource,
    InvalidContent,
    InvalidGeometry,
    UnderstatedRotation,
    InvalidShield,
}
impl core::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Mule envelope refused: {self:?}")
    }
}
impl std::error::Error for EnvelopeError {}

/// Original calibrated engineering definition, independent of the approximate contact surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MuleEngineering {
    pub dry_mass_kilograms: u32,
    pub center_of_mass_metres: [f64; 3],
    pub inertia_about_center_kg_m2: [f64; 9],
    pub scale_numerator: u32,
    pub scale_denominator: u32,
}
pub const MULE_ENGINEERING: MuleEngineering = MuleEngineering {
    dry_mass_kilograms: 62470,
    center_of_mass_metres: [
        -0.000173786105968265,
        0.27101762492938664,
        -2.884017656445922,
    ],
    inertia_about_center_kg_m2: [
        7903036.063390594,
        -5.453271721331776,
        -52.25978105589324,
        -5.453271721331776,
        9657301.90416053,
        -15094.614662567205,
        -52.25978105589324,
        -15094.614662567205,
        2570806.5353476247,
    ],
    scale_numerator: 40000,
    scale_denominator: 3411,
};

/// Immutable local surface parameters. The renderer must hide surfaces inside other pieces,
/// use the same rounded convex support surface, and cap union opacity rather than accumulating piece layers.
#[derive(Debug, Clone, PartialEq)]
pub struct MulePresentation {
    pub pieces: Arc<[SmoothPiece; MULE_PIECES]>,
    pub front_shield_fraction: f32,
    pub rear_shield_fraction: f32,
    pub maximum_opacity: f32,
    pub close_fade_metres: [f32; 2],
    pub revision: u32,
}

impl MulePresentation {
    /// Immutable rendering guidance, using the existing front/rear local +Z bank split.
    /// Invalid view inputs fail invisible; this function has no gameplay side effects.
    pub fn shimmer_alpha(
        &self,
        local_point: Vec3,
        distance_metres: f32,
        normal_view_dot: f32,
    ) -> f32 {
        if !self.maximum_opacity.is_finite()
            || !(0.0..=MULE_MAX_SHIMMER_OPACITY).contains(&self.maximum_opacity)
            || !self.front_shield_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.front_shield_fraction)
            || !self.rear_shield_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.rear_shield_fraction)
            || self.close_fade_metres != [MULE_SHIMMER_NEAR_METRES, MULE_SHIMMER_FAR_METRES]
            || !finite(local_point)
            || !distance_metres.is_finite()
            || distance_metres < 0.0
            || !normal_view_dot.is_finite()
            || !(-1.0..=1.0).contains(&normal_view_dot)
        {
            return 0.0;
        }
        let fraction = if local_point.z >= 0.0 {
            self.front_shield_fraction
        } else {
            self.rear_shield_fraction
        };
        let fade = ((self.close_fade_metres[1] - distance_metres)
            / (self.close_fade_metres[1] - self.close_fade_metres[0]))
            .clamp(0.0, 1.0);
        self.maximum_opacity * fraction * fade * fade * (1.0 - normal_view_dot.abs()).powi(4)
    }
}

/// One immutable source enclosure. Mutable actor identity, pose and generations belong to World.
#[derive(Debug, Clone, PartialEq)]
pub struct MuleHullSource {
    pieces: Arc<[SmoothPiece; MULE_PIECES]>,
}
impl MuleHullSource {
    pub fn id(&self) -> &'static str {
        "SH001"
    }
    pub fn bundled() -> Result<Self, EnvelopeError> {
        Self::from_bytes(VERTICES, MEMBERSHIP)
    }
    /// Complete byte equality authenticates every original detailed/native vertex and membership.
    pub fn from_bytes(vertices: &[u8], membership: &[u8]) -> Result<Self, EnvelopeError> {
        if vertices != VERTICES || membership != MEMBERSHIP {
            return Err(EnvelopeError::ChangedSource);
        }
        if vertices.get(..8) != Some(b"MSH001V1")
            || vertices.len() != 16 + 24 * (MULE_DETAILED_VERTICES + MULE_NATIVE_VERTICES)
            || membership.len() != MULE_DETAILED_VERTICES + MULE_NATIVE_VERTICES
        {
            return Err(EnvelopeError::InvalidContent);
        }
        let mut points: [Vec<Vec3>; MULE_PIECES] = core::array::from_fn(|_| Vec::new());
        for (index, piece) in membership.iter().copied().enumerate() {
            if usize::from(piece) >= MULE_PIECES {
                return Err(EnvelopeError::InvalidContent);
            }
            points[usize::from(piece)].push(source_vertex(vertices, index)?);
        }
        let built = points
            .into_iter()
            .map(|points| SmoothPiece::new(points, MULE_CLEARANCE_METRES))
            .collect::<Result<Vec<_>, _>>()?;
        let pieces: [SmoothPiece; MULE_PIECES] = built
            .try_into()
            .map_err(|_| EnvelopeError::InvalidContent)?;
        Ok(Self {
            pieces: Arc::new(pieces),
        })
    }
    pub fn pieces(&self) -> &[SmoothPiece; MULE_PIECES] {
        self.pieces.as_ref()
    }
    pub fn maximum_origin_radius(&self) -> f64 {
        self.pieces
            .iter()
            .map(|piece| piece.origin_radius())
            .fold(0.0, f64::max)
    }
    pub fn engineering(&self) -> MuleEngineering {
        MULE_ENGINEERING
    }
    pub fn contains_local(&self, point: Vec3) -> bool {
        self.pieces.iter().any(|piece| piece.contains(point))
    }
    /// Shields affect visible strength only. Collision/docking geometry never retracts at zero.
    /// Values come from the existing actor's bank snapshot, not a new damage/EMP reducer.
    pub fn presentation(
        &self,
        front: u16,
        rear: u16,
        capacity: u16,
    ) -> Result<MulePresentation, EnvelopeError> {
        if capacity == 0 || front > capacity || rear > capacity {
            return Err(EnvelopeError::InvalidShield);
        }
        Ok(MulePresentation {
            pieces: self.pieces.clone(),
            front_shield_fraction: f32::from(front) / f32::from(capacity),
            rear_shield_fraction: f32::from(rear) / f32::from(capacity),
            maximum_opacity: MULE_MAX_SHIMMER_OPACITY,
            close_fade_metres: [MULE_SHIMMER_NEAR_METRES, MULE_SHIMMER_FAR_METRES],
            revision: 1,
        })
    }
    /// Strict analytic support against every aperture edge; no bounding-box fit substitution.
    pub fn fits_port(
        &self,
        port: &DockingPort,
        player: RigidPose,
        station: StationPose,
        margin: f64,
    ) -> Result<bool, EnvelopeError> {
        if !valid_pose(player)
            || !valid_pose(station.pose)
            || !station.axial_rotation.is_finite()
            || !station.rotation_rate.is_finite()
            || !margin.is_finite()
            || margin < 0.0
        {
            return Err(EnvelopeError::InvalidGeometry);
        }
        let (sine, cosine) = station.axial_rotation.sin_cos();
        let station_basis = Basis3 {
            x: station.pose.orientation.apply(Vec3::new(cosine, sine, 0.0)),
            y: station
                .pose
                .orientation
                .apply(Vec3::new(-sine, cosine, 0.0)),
            z: station.pose.orientation.z,
        };
        let translation = sub(player.position, station.pose.position);
        for (index, a) in port.aperture().iter().copied().enumerate() {
            let edge = sub(port.aperture()[(index + 1) % port.aperture().len()], a);
            let length = (edge.x * edge.x + edge.y * edge.y).sqrt();
            if !length.is_finite() || length <= 0.0 {
                return Err(EnvelopeError::InvalidGeometry);
            }
            let inward = Vec3::new(-edge.y / length, edge.x / length, 0.0);
            let world_inward = station_basis.apply(inward);
            let local_negative = Vec3::new(
                -player.orientation.x.dot(world_inward),
                -player.orientation.y.dot(world_inward),
                -player.orientation.z.dot(world_inward),
            );
            let offset = translation.dot(world_inward) - add(port.center(), a).dot(inward);
            for piece in self.pieces.iter() {
                let minimum = offset - piece.support_bound(local_negative)?;
                let numeric_guard = 2048.0
                    * f64::EPSILON
                    * (1.0 + translation.dot(translation).sqrt() + offset.abs());
                if !minimum.is_finite() || minimum <= margin + numeric_guard {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    /// Earliest source-bound gameplay surface; uncertainty occludes and grants no effect.
    pub fn ray_contact(
        &self,
        pose: RigidPose,
        origin: Vec3,
        direction: Vec3,
        range: f64,
    ) -> Result<EnvelopeRayContact, EnvelopeError> {
        let mut nearest = EnvelopeRayContact::Clear;
        for piece in self.pieces.iter() {
            let contact = piece.ray_contact(pose, origin, direction, range)?;
            if let Some(distance) = contact.lower_bound()
                && (nearest
                    .lower_bound()
                    .is_none_or(|current| distance < current)
                    || (nearest.lower_bound() == Some(distance)
                        && matches!(contact, EnvelopeRayContact::Uncertain { .. })))
            {
                nearest = contact;
            }
        }
        Ok(nearest)
    }

    /// Every piece must have a strict separating certificate for every native station triangle.
    /// The caller supplies the actual immutable station triangles and owned start/end poses.
    pub fn sweep_clears_triangles(
        &self,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
        triangles: &[[Vec3; 3]],
    ) -> Result<bool, EnvelopeError> {
        if triangles.is_empty() {
            return Err(EnvelopeError::InvalidGeometry);
        }
        for triangle in triangles {
            for piece in self.pieces.iter() {
                if !piece.sweep_clears_triangle(start, end, angular_travel, *triangle)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}
fn source_vertex(bytes: &[u8], index: usize) -> Result<Vec3, EnvelopeError> {
    let begin = 16 + 24 * index;
    let mut v = [0.0; 3];
    for (axis, result) in v.iter_mut().enumerate() {
        let data = bytes
            .get(begin + 8 * axis..begin + 8 * (axis + 1))
            .ok_or(EnvelopeError::InvalidContent)?;
        *result =
            f64::from_le_bytes(data.try_into().map_err(|_| EnvelopeError::InvalidContent)?) * SCALE;
    }
    let point = Vec3::new(v[0], v[1], v[2]);
    if !finite(point) {
        return Err(EnvelopeError::InvalidContent);
    }
    Ok(point)
}

#[cfg(test)]
mod tests;

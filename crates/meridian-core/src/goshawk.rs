//! The SH008 source-bound envelope candidate, independent of ordinary actor admission.
use crate::docking::{RigidPose, Vec3};
use crate::mule::{EnvelopeError, MULE_CLEARANCE_METRES, SmoothPiece};
use std::sync::Arc;

pub const GOSHAWK_PIECES: usize = 69;
/// Authenticated SH008 R3 performance.maxSpeed, in FlightStart units.
pub(crate) const GOSHAWK_MAX_SPEED_UNITS: f32 = 41.0;
pub const GOSHAWK_DETAILED_VERTICES: usize = 2072;
pub const GOSHAWK_NATIVE_CANDIDATE_VERTICES: usize = 536;
pub const GOSHAWK_RESOURCE_SHA256: &str =
    "aafcb5dfec2cb5d141316756daae267888bf3541383f4f155c2561a4d7dd0bb7";
pub const GOSHAWK_CONTRACT_SHA256: &str =
    "6815e92d5480692d18576ab5ed5e98cd1f99b1bff622c3c7eb346ed59bcf8d42";
const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh008/source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh008/source-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct GoshawkHullSource {
    pieces: Arc<[SmoothPiece; GOSHAWK_PIECES]>,
}

impl GoshawkHullSource {
    pub fn bundled() -> Result<Self, EnvelopeError> {
        Self::from_bytes(VERTICES, MEMBERSHIP)
    }

    /// Positions are already calibrated to source metres. No runtime scale is applied.
    pub fn from_bytes(vertices: &[u8], membership: &[u8]) -> Result<Self, EnvelopeError> {
        if vertices != VERTICES || membership != MEMBERSHIP {
            return Err(EnvelopeError::ChangedSource);
        }
        Ok(Self {
            pieces: calibrated_pieces(
                vertices,
                membership,
                b"MSH008V1",
                GOSHAWK_DETAILED_VERTICES,
                GOSHAWK_NATIVE_CANDIDATE_VERTICES,
            )?,
        })
    }

    pub fn pieces(&self) -> &Arc<[SmoothPiece; GOSHAWK_PIECES]> {
        &self.pieces
    }

    /// Clearance requires a strict separating certificate for every piece and triangle.
    pub fn sweep_clears_triangles(
        &self,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
        triangles: &[[Vec3; 3]],
    ) -> Result<bool, EnvelopeError> {
        source_sweep_clears(self.pieces.as_ref(), start, end, angular_travel, triangles)
    }
}

/// Decode only after the selected hull has authenticated the two complete byte streams.
pub(crate) fn calibrated_pieces<const N: usize>(
    vertices: &[u8],
    membership: &[u8],
    magic: &[u8; 8],
    detailed: usize,
    native: usize,
) -> Result<Arc<[SmoothPiece; N]>, EnvelopeError> {
    let points = detailed + native;
    if vertices.get(..8) != Some(magic)
        || vertices.get(8..12) != Some(&(detailed as u32).to_le_bytes())
        || vertices.get(12..16) != Some(&(native as u32).to_le_bytes())
        || vertices.len() != 16 + 24 * points
        || membership.len() != points
    {
        return Err(EnvelopeError::InvalidContent);
    }
    let mut assigned: [Vec<Vec3>; N] = core::array::from_fn(|_| Vec::new());
    for (index, piece) in membership.iter().copied().enumerate() {
        assigned
            .get_mut(usize::from(piece))
            .ok_or(EnvelopeError::InvalidContent)?
            .push(source_vertex(vertices, index)?);
    }
    let built = assigned
        .into_iter()
        .map(|points| SmoothPiece::new(points, MULE_CLEARANCE_METRES))
        .collect::<Result<Vec<_>, _>>()?;
    let pieces: [SmoothPiece; N] = built
        .try_into()
        .map_err(|_| EnvelopeError::InvalidContent)?;
    Ok(Arc::new(pieces))
}

pub(crate) fn source_sweep_clears(
    pieces: &[SmoothPiece],
    start: RigidPose,
    end: RigidPose,
    angular_travel: f64,
    triangles: &[[Vec3; 3]],
) -> Result<bool, EnvelopeError> {
    if triangles.is_empty() {
        return Err(EnvelopeError::InvalidGeometry);
    }
    for triangle in triangles {
        for piece in pieces {
            if !piece.sweep_clears_triangle(start, end, angular_travel, *triangle)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub(crate) fn source_vertex(bytes: &[u8], index: usize) -> Result<Vec3, EnvelopeError> {
    let begin = 16 + 24 * index;
    let record = bytes
        .get(begin..begin + 24)
        .ok_or(EnvelopeError::InvalidContent)?;
    let mut coordinates = [0.0; 3];
    for (value, word) in coordinates.iter_mut().zip(record.as_chunks::<8>().0) {
        *value = f64::from_le_bytes(*word);
    }
    Ok(Vec3::new(coordinates[0], coordinates[1], coordinates[2]))
}

#[cfg(test)]
mod tests;

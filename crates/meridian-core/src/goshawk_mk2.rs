//! Source-bound SH009 rounded supports for explicit geometric qualification.
use crate::docking::{RigidPose, Vec3};
use crate::mule::{EnvelopeError, MULE_CLEARANCE_METRES, SmoothPiece};
use std::sync::Arc;

mod engineering;
pub use engineering::{EmptyInterfaceMass, GoshawkMk2Engineering};

pub const GOSHAWK_MK2_PIECES: usize = 73;
const DETAILED_VERTICES: usize = 2104;
const NATIVE_VERTICES: usize = 568;
const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh009/source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh009/source-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct GoshawkMk2HullSource {
    pieces: Arc<[SmoothPiece; GOSHAWK_MK2_PIECES]>,
}

impl GoshawkMk2HullSource {
    pub fn bundled() -> Result<Self, EnvelopeError> {
        Self::from_bytes(VERTICES, MEMBERSHIP)
    }

    /// Accepts only the calibrated SH009 encodings bundled for this hull.
    pub fn from_bytes(vertices: &[u8], membership: &[u8]) -> Result<Self, EnvelopeError> {
        if vertices != VERTICES || membership != MEMBERSHIP {
            return Err(EnvelopeError::ChangedSource);
        }
        let count = DETAILED_VERTICES + NATIVE_VERTICES;
        let header = vertices.get(..16).ok_or(EnvelopeError::InvalidContent)?;
        let data = vertices.get(16..).ok_or(EnvelopeError::InvalidContent)?;
        let (points, trailing) = data.as_chunks::<24>();
        if header.get(..8) != Some(b"MSH009V1")
            || header.get(8..12) != Some(&(DETAILED_VERTICES as u32).to_le_bytes())
            || header.get(12..16) != Some(&(NATIVE_VERTICES as u32).to_le_bytes())
            || !trailing.is_empty()
            || points.len() != count
            || membership.len() != count
        {
            return Err(EnvelopeError::InvalidContent);
        }
        let mut assigned: [Vec<Vec3>; GOSHAWK_MK2_PIECES] = core::array::from_fn(|_| Vec::new());
        for (point, slot) in points.iter().zip(membership) {
            let group = assigned
                .get_mut(usize::from(*slot))
                .ok_or(EnvelopeError::InvalidContent)?;
            group.push(decode_point(point));
        }
        let built = assigned
            .into_iter()
            .map(|points| SmoothPiece::new(points, MULE_CLEARANCE_METRES))
            .collect::<Result<Vec<_>, _>>()?;
        let pieces: [SmoothPiece; GOSHAWK_MK2_PIECES] = built
            .try_into()
            .map_err(|_| EnvelopeError::InvalidContent)?;
        Ok(Self {
            pieces: Arc::new(pieces),
        })
    }

    pub fn pieces(&self) -> &Arc<[SmoothPiece; GOSHAWK_MK2_PIECES]> {
        &self.pieces
    }

    /// Adopted stock mass properties of the authenticated Mk II source.
    pub fn engineering(&self) -> &'static GoshawkMk2Engineering {
        &engineering::STOCK
    }

    /// True only when every piece has strict swept clearance from the triangle.
    pub fn sweep_clears_triangle(
        &self,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
        triangle: [Vec3; 3],
    ) -> Result<bool, EnvelopeError> {
        for piece in self.pieces.iter() {
            if !piece.sweep_clears_triangle(start, end, angular_travel, triangle)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn decode_point(point: &[u8; 24]) -> Vec3 {
    let mut coordinates = [0.0; 3];
    for (value, word) in coordinates.iter_mut().zip(point.as_chunks::<8>().0) {
        *value = f64::from_le_bytes(*word);
    }
    Vec3::new(coordinates[0], coordinates[1], coordinates[2])
}

#[cfg(test)]
mod tests;

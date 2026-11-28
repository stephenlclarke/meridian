//! Source-bound SH158 rounded supports for catalogue inspection and explicit geometry checks.
use crate::docking::{RigidPose, Vec3};
use crate::mule::{EnvelopeError, MULE_CLEARANCE_METRES, SmoothPiece};
use std::sync::Arc;

pub const WOLF_MK_I_PIECES: usize = 93;
const DETAILED_VERTICES: usize = 1200;
const ARCHIVED_NATIVE_VERTICES: usize = 440;
const CANDIDATE_NATIVE_VERTICES: usize = 314;
const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh158/source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh158/source-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct WolfMkIHullSource {
    pieces: Arc<[SmoothPiece; WOLF_MK_I_PIECES]>,
}

impl WolfMkIHullSource {
    pub fn bundled() -> Result<Self, EnvelopeError> {
        Self::from_bytes(VERTICES, MEMBERSHIP)
    }

    /// Accepts only the calibrated SH158 encodings bundled for this hull.
    pub fn from_bytes(vertices: &[u8], membership: &[u8]) -> Result<Self, EnvelopeError> {
        if vertices != VERTICES || membership != MEMBERSHIP {
            return Err(EnvelopeError::ChangedSource);
        }
        let count = DETAILED_VERTICES + ARCHIVED_NATIVE_VERTICES + CANDIDATE_NATIVE_VERTICES;
        let header = vertices.get(..20).ok_or(EnvelopeError::InvalidContent)?;
        let data = vertices.get(20..).ok_or(EnvelopeError::InvalidContent)?;
        let (points, trailing) = data.as_chunks::<24>();
        if header.get(..8) != Some(b"MSH158V1")
            || header.get(8..12) != Some(&(DETAILED_VERTICES as u32).to_le_bytes())
            || header.get(12..16) != Some(&(ARCHIVED_NATIVE_VERTICES as u32).to_le_bytes())
            || header.get(16..20) != Some(&(CANDIDATE_NATIVE_VERTICES as u32).to_le_bytes())
            || !trailing.is_empty()
            || points.len() != count
            || membership.len() != count
        {
            return Err(EnvelopeError::InvalidContent);
        }
        let mut assigned: [Vec<Vec3>; WOLF_MK_I_PIECES] = core::array::from_fn(|_| Vec::new());
        for (point, &slot) in points.iter().zip(membership) {
            assigned
                .get_mut(usize::from(slot))
                .ok_or(EnvelopeError::InvalidContent)?
                .push(decode_point(point));
        }
        let built = assigned
            .into_iter()
            .map(|points| SmoothPiece::new(points, MULE_CLEARANCE_METRES))
            .collect::<Result<Vec<_>, _>>()?;
        let pieces: [SmoothPiece; WOLF_MK_I_PIECES] = built
            .try_into()
            .map_err(|_| EnvelopeError::InvalidContent)?;
        Ok(Self {
            pieces: Arc::new(pieces),
        })
    }

    pub fn pieces(&self) -> &[SmoothPiece; WOLF_MK_I_PIECES] {
        &self.pieces
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

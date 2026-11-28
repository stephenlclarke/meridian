//! Source-bound SH098 rounded supports for catalogue inspection and explicit geometry checks.
use crate::docking::{RigidPose, Vec3};
use crate::mule::{EnvelopeError, MULE_CLEARANCE_METRES, SmoothPiece};
use std::sync::Arc;

pub const ANOA_PIECES: usize = 88;
const DETAILED_VERTICES: usize = 3792;
const ARCHIVED_NATIVE_VERTICES: usize = 1028;
const CANDIDATE_NATIVE_VERTICES: usize = 320;
const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh098/source.vertices");
const MEMBERSHIP: &[u8] =
    include_bytes!("../../../assets/fleet/sh098/collision-piece-membership.bin");
const NO_SECONDARY_PIECE: u8 = u8::MAX;

#[derive(Debug, Clone, PartialEq)]
pub struct AnoaHullSource {
    pieces: Arc<[SmoothPiece; ANOA_PIECES]>,
}

impl AnoaHullSource {
    pub fn bundled() -> Result<Self, EnvelopeError> {
        Self::from_bytes(VERTICES, MEMBERSHIP)
    }

    /// Accepts only the calibrated SH098 encodings bundled for this hull.
    pub fn from_bytes(vertices: &[u8], membership: &[u8]) -> Result<Self, EnvelopeError> {
        if vertices != VERTICES || membership != MEMBERSHIP {
            return Err(EnvelopeError::ChangedSource);
        }
        let count = DETAILED_VERTICES + ARCHIVED_NATIVE_VERTICES + CANDIDATE_NATIVE_VERTICES;
        let header = vertices.get(..20).ok_or(EnvelopeError::InvalidContent)?;
        let data = vertices.get(20..).ok_or(EnvelopeError::InvalidContent)?;
        let (points, trailing) = data.as_chunks::<24>();
        let (memberships, trailing_membership) = membership.as_chunks::<2>();
        if header.get(..8) != Some(b"MSH098V1")
            || header.get(8..12) != Some(&(DETAILED_VERTICES as u32).to_le_bytes())
            || header.get(12..16) != Some(&(ARCHIVED_NATIVE_VERTICES as u32).to_le_bytes())
            || header.get(16..20) != Some(&(CANDIDATE_NATIVE_VERTICES as u32).to_le_bytes())
            || !trailing.is_empty()
            || points.len() != count
            || !trailing_membership.is_empty()
            || memberships.len() != count
        {
            return Err(EnvelopeError::InvalidContent);
        }
        let mut assigned: [Vec<Vec3>; ANOA_PIECES] = core::array::from_fn(|_| Vec::new());
        for (point, &[primary, secondary]) in points.iter().zip(memberships) {
            let point = decode_point(point);
            assigned
                .get_mut(usize::from(primary))
                .ok_or(EnvelopeError::InvalidContent)?
                .push(point);
            if secondary != NO_SECONDARY_PIECE {
                assigned
                    .get_mut(usize::from(secondary))
                    .ok_or(EnvelopeError::InvalidContent)?
                    .push(point);
            }
        }
        let built = assigned
            .into_iter()
            .map(|points| SmoothPiece::new(points, MULE_CLEARANCE_METRES))
            .collect::<Result<Vec<_>, _>>()?;
        let pieces: [SmoothPiece; ANOA_PIECES] = built
            .try_into()
            .map_err(|_| EnvelopeError::InvalidContent)?;
        Ok(Self {
            pieces: Arc::new(pieces),
        })
    }

    pub fn pieces(&self) -> &[SmoothPiece; ANOA_PIECES] {
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

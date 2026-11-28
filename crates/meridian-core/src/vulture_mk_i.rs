//! Source-bound SH037 rounded supports for explicit geometric qualification.
use crate::docking::{RigidPose, Vec3};
use crate::mule::{EnvelopeError, MULE_CLEARANCE_METRES, SmoothPiece};
use std::sync::Arc;

pub const VULTURE_MK_I_PIECES: usize = 8;
const DETAILED_VERTICES: usize = 5132;
const NATIVE_VERTICES: usize = 1812;
const PROXY_VERTICES: usize = 272;
const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh037/source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh037/source-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct VultureMkIHullSource {
    pieces: Arc<[SmoothPiece; VULTURE_MK_I_PIECES]>,
}

impl VultureMkIHullSource {
    pub fn bundled() -> Result<Self, EnvelopeError> {
        Self::from_bytes(VERTICES, MEMBERSHIP)
    }

    /// Accepts only the calibrated SH037 encodings bundled for this hull.
    pub fn from_bytes(vertices: &[u8], membership: &[u8]) -> Result<Self, EnvelopeError> {
        if vertices != VERTICES || membership != MEMBERSHIP {
            return Err(EnvelopeError::ChangedSource);
        }
        let count = DETAILED_VERTICES + NATIVE_VERTICES + PROXY_VERTICES;
        let header = vertices.get(..20).ok_or(EnvelopeError::InvalidContent)?;
        let data = vertices.get(20..).ok_or(EnvelopeError::InvalidContent)?;
        let (points, trailing) = data.as_chunks::<24>();
        if header.get(..8) != Some(b"MSH037V1")
            || header.get(8..12) != Some(&(DETAILED_VERTICES as u32).to_le_bytes())
            || header.get(12..16) != Some(&(NATIVE_VERTICES as u32).to_le_bytes())
            || header.get(16..20) != Some(&(PROXY_VERTICES as u32).to_le_bytes())
            || !trailing.is_empty()
            || points.len() != count
            || membership.len() != count
        {
            return Err(EnvelopeError::InvalidContent);
        }
        let mut assigned: [Vec<Vec3>; VULTURE_MK_I_PIECES] = core::array::from_fn(|_| Vec::new());
        // The frozen proxy strictly encloses the original points assigned to each piece.
        let first_proxy = DETAILED_VERTICES + NATIVE_VERTICES;
        for (point, slot) in points[first_proxy..].iter().zip(&membership[first_proxy..]) {
            let group = assigned
                .get_mut(usize::from(*slot))
                .ok_or(EnvelopeError::InvalidContent)?;
            group.push(decode_point(point));
        }
        let built = assigned
            .into_iter()
            .map(|points| SmoothPiece::new(points, MULE_CLEARANCE_METRES))
            .collect::<Result<Vec<_>, _>>()?;
        let pieces: [SmoothPiece; VULTURE_MK_I_PIECES] = built
            .try_into()
            .map_err(|_| EnvelopeError::InvalidContent)?;
        Ok(Self {
            pieces: Arc::new(pieces),
        })
    }

    pub fn pieces(&self) -> &[SmoothPiece; VULTURE_MK_I_PIECES] {
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

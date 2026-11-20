//! Original SH012 support geometry for controlled qualification against source contact.
use crate::docking::{RigidPose, Vec3};
use crate::mule::{EnvelopeError, MULE_CLEARANCE_METRES, SmoothPiece};
use std::sync::Arc;

pub const SABLE_LINER_PIECES: usize = 47;
const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh012/source.vertices");
const INDICES: &[u8] = include_bytes!("../../../assets/fleet/sh012/piece-indices.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct SableLinerHullSource {
    pieces: Arc<[SmoothPiece; SABLE_LINER_PIECES]>,
}

impl SableLinerHullSource {
    pub fn bundled() -> Result<Self, EnvelopeError> {
        Self::from_bytes(VERTICES, INDICES)
    }

    /// Accepts only the two calibrated SH012 sources and their bound shield layout.
    pub fn from_bytes(vertices: &[u8], indices: &[u8]) -> Result<Self, EnvelopeError> {
        if vertices != VERTICES || indices != INDICES {
            return Err(EnvelopeError::ChangedSource);
        }
        if vertices.get(..8) != Some(b"MSH012V1")
            || vertices.get(8..12) != Some(&2308_u32.to_le_bytes())
            || vertices.get(12..16) != Some(&132_u32.to_le_bytes())
            || vertices.len() != 16 + 2440 * 24
            || indices.get(..8) != Some(b"MSH12PC1")
            || indices.get(8..12) != Some(&(SABLE_LINER_PIECES as u32).to_le_bytes())
        {
            return Err(EnvelopeError::InvalidContent);
        }
        let points: Vec<_> = vertices[16..]
            .as_chunks::<24>()
            .0
            .iter()
            .map(decode_point)
            .collect();
        let mut words = indices[12..].as_chunks::<4>().0;
        let mut pieces = Vec::with_capacity(SABLE_LINER_PIECES);
        for _ in 0..SABLE_LINER_PIECES {
            let (count, next) = words.split_first().ok_or(EnvelopeError::InvalidContent)?;
            let (assigned, remaining) = next
                .split_at_checked(u32::from_le_bytes(*count) as usize)
                .ok_or(EnvelopeError::InvalidContent)?;
            let anchors = assigned
                .iter()
                .map(|word| points.get(u32::from_le_bytes(*word) as usize).copied())
                .collect::<Option<Vec<_>>>()
                .ok_or(EnvelopeError::InvalidContent)?;
            pieces.push(SmoothPiece::new(anchors, MULE_CLEARANCE_METRES)?);
            words = remaining;
        }
        if !words.is_empty() {
            return Err(EnvelopeError::InvalidContent);
        }
        let pieces: [SmoothPiece; SABLE_LINER_PIECES] = pieces
            .try_into()
            .map_err(|_| EnvelopeError::InvalidContent)?;
        Ok(Self {
            pieces: Arc::new(pieces),
        })
    }

    pub fn pieces(&self) -> &Arc<[SmoothPiece; SABLE_LINER_PIECES]> {
        &self.pieces
    }

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
    for (value, chunk) in coordinates.iter_mut().zip(point.as_chunks::<8>().0) {
        *value = f64::from_le_bytes(*chunk);
    }
    Vec3::new(coordinates[0], coordinates[1], coordinates[2])
}

#[cfg(test)]
mod tests;

//! Source-bound SH016 supports for controlled geometric qualification.

use crate::docking::{RigidPose, Vec3};
use crate::mule::{EnvelopeError, MULE_CLEARANCE_METRES, SmoothPiece};
use std::sync::Arc;

pub const MOLE_MINER_PIECES: usize = 203;
const DETAILED_VERTICES: usize = 6760;
const NATIVE_VERTICES: usize = 616;
const VERTEX_COUNT: usize = DETAILED_VERTICES + NATIVE_VERTICES;
const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh016/source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh016/source-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct MoleMinerHullSource {
    pieces: Arc<[SmoothPiece; MOLE_MINER_PIECES]>,
}

impl MoleMinerHullSource {
    pub fn bundled() -> Result<Self, EnvelopeError> {
        Self::from_bytes(VERTICES, MEMBERSHIP)
    }

    /// Loads the pinned, once-calibrated SH016 streams.
    pub fn from_bytes(vertices: &[u8], membership: &[u8]) -> Result<Self, EnvelopeError> {
        if vertices != VERTICES || membership != MEMBERSHIP {
            return Err(EnvelopeError::ChangedSource);
        }
        let (header, points) = vertices
            .split_at_checked(16)
            .ok_or(EnvelopeError::InvalidContent)?;
        let (points, trailing) = points.as_chunks::<24>();
        if header.get(..8) != Some(b"MSH016V1")
            || header.get(8..12) != Some(&(DETAILED_VERTICES as u32).to_le_bytes())
            || header.get(12..16) != Some(&(NATIVE_VERTICES as u32).to_le_bytes())
            || !trailing.is_empty()
            || points.len() != VERTEX_COUNT
        {
            return Err(EnvelopeError::InvalidContent);
        }
        let pieces = groups(membership)?
            .into_iter()
            .map(|group| {
                let source = group
                    .into_iter()
                    .map(|index| decode_point(&points[index]))
                    .collect();
                SmoothPiece::new(source, MULE_CLEARANCE_METRES)
            })
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| EnvelopeError::InvalidContent)?;
        Ok(Self {
            pieces: Arc::new(pieces),
        })
    }

    pub fn pieces(&self) -> &[SmoothPiece; MOLE_MINER_PIECES] {
        &self.pieces
    }

    /// Clears only when every local support has strict swept clearance.
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

fn groups(membership: &[u8]) -> Result<Vec<Vec<usize>>, EnvelopeError> {
    let (words, spare) = membership.as_chunks::<2>();
    let mut remaining = words;
    let mut groups = Vec::with_capacity(MOLE_MINER_PIECES);
    for _ in 0..MOLE_MINER_PIECES {
        let (count, tail) = remaining
            .split_first()
            .ok_or(EnvelopeError::InvalidContent)?;
        let (ids, tail) = tail
            .split_at_checked(usize::from(u16::from_le_bytes(*count)))
            .ok_or(EnvelopeError::InvalidContent)?;
        let group: Vec<_> = ids
            .iter()
            .map(|word| usize::from(u16::from_le_bytes(*word)))
            .collect();
        if group.is_empty() || !group.iter().all(|&index| index < VERTEX_COUNT) {
            return Err(EnvelopeError::InvalidContent);
        }
        groups.push(group);
        remaining = tail;
    }
    if !spare.is_empty() || !remaining.is_empty() {
        return Err(EnvelopeError::InvalidContent);
    }
    Ok(groups)
}

fn decode_point(bytes: &[u8; 24]) -> Vec3 {
    let mut xyz = [0.0; 3];
    for (coordinate, word) in xyz.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *coordinate = f64::from_le_bytes(*word);
    }
    Vec3::new(xyz[0], xyz[1], xyz[2])
}

#[cfg(test)]
mod tests;

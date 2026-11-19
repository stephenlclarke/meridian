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
const POINTS: usize = GOSHAWK_DETAILED_VERTICES + GOSHAWK_NATIVE_CANDIDATE_VERTICES;

/// Adopted SH008 stock mass properties in the calibrated hull-local XYZ frame.
/// The inertia tensor is about the centre of mass, with rows and columns ordered X, Y, Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoshawkEngineering {
    pub dry_mass_kilograms: u32,
    pub center_of_mass_metres: [f64; 3],
    pub inertia_about_center_kg_m2: [[f64; 3]; 3],
    pub proposal_sha256: &'static str,
}

// The two luminous recesses are included in drive mechanism mass.
static STOCK_ENGINEERING: GoshawkEngineering = GoshawkEngineering {
    dry_mass_kilograms: 33_719,
    center_of_mass_metres: [
        -0.000_013_111_041_307_319_17,
        0.311_774_780_438_892_3,
        -2.362_604_393_172_571,
    ],
    inertia_about_center_kg_m2: [
        [
            2_639_403.265_670_668,
            3.492_783_872_195_644_3,
            -14.617_110_010_104_89,
        ],
        [
            3.492_783_872_195_644_3,
            3_740_763.839_742_993_5,
            76_566.515_830_687_48,
        ],
        [
            -14.617_110_010_104_89,
            76_566.515_830_687_48,
            1_305_318.864_628_752_7,
        ],
    ],
    proposal_sha256: "c086123c0e38c12189719bbbcb36120da8c3099e0e959f335520de2851eceb55",
};

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
        if vertices.get(..8) != Some(b"MSH008V1")
            || vertices.get(8..12) != Some(&2072_u32.to_le_bytes())
            || vertices.get(12..16) != Some(&536_u32.to_le_bytes())
            || vertices.len() != 16 + 24 * POINTS
            || membership.len() != POINTS
        {
            return Err(EnvelopeError::InvalidContent);
        }
        let mut assigned: [Vec<Vec3>; GOSHAWK_PIECES] = core::array::from_fn(|_| Vec::new());
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
        let pieces: [SmoothPiece; GOSHAWK_PIECES] = built
            .try_into()
            .map_err(|_| EnvelopeError::InvalidContent)?;
        Ok(Self {
            pieces: Arc::new(pieces),
        })
    }

    pub fn pieces(&self) -> &Arc<[SmoothPiece; GOSHAWK_PIECES]> {
        &self.pieces
    }

    /// Stock mass properties of the authenticated SH008 hull.
    pub fn engineering(&self) -> &'static GoshawkEngineering {
        &STOCK_ENGINEERING
    }

    /// Clearance requires a strict separating certificate for every piece and triangle.
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

//! Authenticated SH019 authored metre geometry; stock engineering is not registered.
use crate::docking::{DockingPort, RigidPose, StationPose, Vec3};
use crate::goshawk::{calibrated_pieces, source_sweep_clears};
use crate::mule::{EnvelopeError, SmoothPiece, rounded_fits_port, rounded_maximum_origin_radius};
use std::sync::Arc;

pub const GAZELLE_PIECES: usize = 37;
pub const GAZELLE_DETAILED_VERTICES: usize = 3008;
pub const GAZELLE_NATIVE_VERTICES: usize = 172;
pub const GAZELLE_DETAILED_FACES: usize = 5900;
pub const GAZELLE_NATIVE_FACES: usize = 304;
pub const GAZELLE_NATIVE_EXACT_FACE_LIMIT: usize = 512;
pub const GAZELLE_DETAILED_SUPPORT_POINTS: usize = 3104;
pub const GAZELLE_NATIVE_SUPPORT_POINTS: usize = 204;
pub const GAZELLE_METRES_PER_SOURCE_UNIT: f64 = 1.0;
pub(crate) const GAZELLE_MAX_SPEED_UNITS: f32 = 43.0;
pub const GAZELLE_RESOURCE_SHA256: &str =
    "b37924110f28d2b0d541af5b7de2feb7c386254350afe6f4a8c162af471169ac";
pub const GAZELLE_CONTRACT_SHA256: &str =
    "9eb70cd7503964f5613f86cd6ceec2cf34f64598e8018779380fc6eca5b08a04";
pub const GAZELLE_NATIVE_SOURCE_SHA256: &str =
    "664c9a5ad747c60982d4f0bf460a3dbd69e286ce47c733a25d8a335cfec6fa6f";

const SUPPORT: &[u8] = include_bytes!("../../../assets/fleet/sh019/support.vertices");
const MEMBERSHIP: &[u8] =
    include_bytes!("../../../assets/fleet/sh019/support-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct GazelleHullSource {
    pieces: Arc<[SmoothPiece; GAZELLE_PIECES]>,
}

impl GazelleHullSource {
    pub fn bundled() -> Result<Self, EnvelopeError> {
        Self::from_bytes(SUPPORT, MEMBERSHIP)
    }

    /// The support stream contains exact copies of canonical source points, including
    /// both local owners of each sector seam. The canonical meshes are not rewritten.
    pub fn from_bytes(vertices: &[u8], membership: &[u8]) -> Result<Self, EnvelopeError> {
        if vertices != SUPPORT || membership != MEMBERSHIP {
            return Err(EnvelopeError::ChangedSource);
        }
        Ok(Self {
            pieces: calibrated_pieces(
                vertices,
                membership,
                b"MSH019S1",
                GAZELLE_DETAILED_SUPPORT_POINTS,
                GAZELLE_NATIVE_SUPPORT_POINTS,
            )?,
        })
    }

    pub fn pieces(&self) -> &[SmoothPiece; GAZELLE_PIECES] {
        self.pieces.as_ref()
    }

    pub(crate) fn shared_pieces(&self) -> &Arc<[SmoothPiece; GAZELLE_PIECES]> {
        &self.pieces
    }

    pub fn metres_per_source_unit(&self) -> f64 {
        GAZELLE_METRES_PER_SOURCE_UNIT
    }

    pub fn maximum_origin_radius(&self) -> f64 {
        rounded_maximum_origin_radius(self.pieces())
    }

    pub fn fits_port(
        &self,
        port: &DockingPort,
        player: RigidPose,
        station: StationPose,
        margin: f64,
    ) -> Result<bool, EnvelopeError> {
        rounded_fits_port(self.pieces(), port, player, station, margin)
    }

    pub fn sweep_clears_triangles(
        &self,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
        triangles: &[[Vec3; 3]],
    ) -> Result<bool, EnvelopeError> {
        source_sweep_clears(self.pieces(), start, end, angular_travel, triangles)
    }
}

#[cfg(test)]
mod tests;

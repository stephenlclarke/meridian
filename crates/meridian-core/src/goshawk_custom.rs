//! Authenticated SH010 engineering and conservative local source geometry.
use crate::docking::{DockingPort, RigidPose, StationPose, Vec3};
use crate::goshawk::{calibrated_pieces, source_sweep_clears};
use crate::mule::{
    EnvelopeError, MuleEngineering, SmoothPiece, rounded_fits_port, rounded_maximum_origin_radius,
};
use std::sync::Arc;

pub const CUSTOM_PIECES: usize = 73;
pub const CUSTOM_DETAILED_VERTICES: usize = 2104;
pub const CUSTOM_NATIVE_VERTICES: usize = 568;
pub const CUSTOM_DETAILED_FACES: usize = 3532;
pub const CUSTOM_NATIVE_FACES: usize = 1012;
pub(crate) const CUSTOM_MAX_SPEED_UNITS: f32 = 40.0;
pub const CUSTOM_RESOURCE_SHA256: &str =
    "66f8a18a52854fad92f8c0b1cb76ef3d0b6f7405d6e8e66e944b1b9e63bb89d3";
pub const CUSTOM_CONTRACT_SHA256: &str =
    "755f776227dcb1d7816b5e426fbe239a98f0f9d1df46fc1ebdbd90c5b15e5983";
pub const CUSTOM_PHYSICAL_SHA256: &str =
    "30aba17c31f65a5d1bcb9a29455f45bb876628ce5906d4af5cf16f249d2a79a6";
/// The preserved SH010 source engineering adoption, independent of the rounded contact surface.
pub const CUSTOM_ENGINEERING: MuleEngineering = MuleEngineering {
    dry_mass_kilograms: 32411,
    center_of_mass_metres: [
        -0.2595889861993016,
        0.09256641423509532,
        -2.1978346594139375,
    ],
    inertia_about_center_kg_m2: [
        2559266.4557026057,
        -7682.528365054656,
        9117.654907317501,
        -7682.528365054656,
        3632506.979960595,
        87112.05639827566,
        9117.654907317501,
        87112.05639827566,
        1248975.7679466447,
    ],
    scale_numerator: 15_000_000,
    scale_denominator: 1_644_589,
};

const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh010/source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh010/source-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct GoshawkCustomHullSource {
    pieces: Arc<[SmoothPiece; CUSTOM_PIECES]>,
}

impl GoshawkCustomHullSource {
    pub fn bundled() -> Result<Self, EnvelopeError> {
        Self::from_bytes(VERTICES, MEMBERSHIP)
    }

    /// Both source sets are already calibrated to metres; no World placement is issued here.
    pub fn from_bytes(vertices: &[u8], membership: &[u8]) -> Result<Self, EnvelopeError> {
        if vertices != VERTICES || membership != MEMBERSHIP {
            return Err(EnvelopeError::ChangedSource);
        }
        Ok(Self {
            pieces: calibrated_pieces(
                vertices,
                membership,
                b"MSH010V1",
                CUSTOM_DETAILED_VERTICES,
                CUSTOM_NATIVE_VERTICES,
            )?,
        })
    }

    pub fn pieces(&self) -> &[SmoothPiece; CUSTOM_PIECES] {
        self.pieces.as_ref()
    }

    pub(crate) fn shared_pieces(&self) -> &Arc<[SmoothPiece; CUSTOM_PIECES]> {
        &self.pieces
    }

    pub fn engineering(&self) -> MuleEngineering {
        CUSTOM_ENGINEERING
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

    /// A sweep is clear only if every source-local rounded piece strictly clears every triangle.
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

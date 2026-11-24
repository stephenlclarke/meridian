//! Authenticated SH015 engineering and conservative local source geometry.
use crate::docking::{DockingPort, RigidPose, StationPose, Vec3};
use crate::goshawk::{calibrated_pieces, source_sweep_clears};
use crate::mule::{
    EnvelopeError, MuleEngineering, SmoothPiece, rounded_fits_port, rounded_maximum_origin_radius,
};
use std::sync::Arc;

pub const RAVEN_PIECES: usize = 70;
pub const RAVEN_DETAILED_VERTICES: usize = 2080;
pub const RAVEN_NATIVE_VERTICES: usize = 544;
pub const RAVEN_DETAILED_FACES: usize = 3496;
pub const RAVEN_NATIVE_FACES: usize = 976;
pub(crate) const RAVEN_MAX_SPEED_UNITS: f32 = 46.0;
pub const RAVEN_RESOURCE_SHA256: &str =
    "2a1173b5437c16144bf46b921e8888b229685bc7d046b8fc8e4a14eb1702c412";
pub const RAVEN_CONTRACT_SHA256: &str =
    "9996fa6457ac946c7f01ed8dcab21275b17f260fd4f4cf106bd9d0fa1df71c66";
pub const RAVEN_PHYSICAL_SHA256: &str =
    "1de6745b7b55bdd273ed863e5e714691fc78eb6a449af1a4af2007094d919952";
/// The preserved SH015 source engineering adoption, independent of the rounded contact surface.
pub const RAVEN_ENGINEERING: MuleEngineering = MuleEngineering {
    dry_mass_kilograms: 57064,
    center_of_mass_metres: [
        -3.503048607046494e-06,
        0.5027202110854722,
        -3.0681807323742905,
    ],
    inertia_about_center_kg_m2: [
        2531523.3001100803,
        11.909567514752668,
        -88.8519038221517,
        11.909567514752668,
        6833402.124255868,
        61803.701168233136,
        -88.8519038221517,
        61803.701168233136,
        4855897.476398313,
    ],
    scale_numerator: 30_000,
    scale_denominator: 2_521,
};

const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh015/source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh015/source-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct RavenHullSource {
    pieces: Arc<[SmoothPiece; RAVEN_PIECES]>,
}

impl RavenHullSource {
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
                b"MSH015V1",
                RAVEN_DETAILED_VERTICES,
                RAVEN_NATIVE_VERTICES,
            )?,
        })
    }

    pub fn pieces(&self) -> &[SmoothPiece; RAVEN_PIECES] {
        self.pieces.as_ref()
    }

    pub(crate) fn shared_pieces(&self) -> &Arc<[SmoothPiece; RAVEN_PIECES]> {
        &self.pieces
    }

    pub fn engineering(&self) -> MuleEngineering {
        RAVEN_ENGINEERING
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

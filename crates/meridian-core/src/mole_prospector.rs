//! Authenticated SH017 engineering and conservative local source geometry.
use crate::docking::{DockingPort, RigidPose, StationPose, Vec3};
use crate::goshawk::{calibrated_pieces, source_sweep_clears};
use crate::mule::{
    EnvelopeError, MuleEngineering, SmoothPiece, rounded_fits_port, rounded_maximum_origin_radius,
};
use std::sync::Arc;

pub const PROSPECTOR_PIECES: usize = 83;
pub const PROSPECTOR_DETAILED_VERTICES: usize = 6760;
pub const PROSPECTOR_NATIVE_VERTICES: usize = 616;
pub const PROSPECTOR_DETAILED_FACES: usize = 12836;
pub const PROSPECTOR_NATIVE_FACES: usize = 1100;
pub(crate) const PROSPECTOR_MAX_SPEED_UNITS: f32 = 29.0;
pub const PROSPECTOR_RESOURCE_SHA256: &str =
    "1a31ae1c3827227f79d7fa826d69c6970d34806400d9ab642d0db0f1ddcd2fb6";
pub const PROSPECTOR_CONTRACT_SHA256: &str =
    "c31408f9cbf0e76af47c583fcf084080addb1d8e84970cc0168d7c065bae385a";
pub const PROSPECTOR_PHYSICAL_SHA256: &str =
    "1e63f301befca35e4b3a118075a247f6c9a228a623fb8b8a04b2043351a70e64";
/// The preserved SH017 source engineering adoption, independent of the rounded contact surface.
pub const PROSPECTOR_ENGINEERING: MuleEngineering = MuleEngineering {
    dry_mass_kilograms: 116421,
    center_of_mass_metres: [
        -0.00011961540323413837,
        -1.731577745833791,
        -2.5307858129987086,
    ],
    inertia_about_center_kg_m2: [
        19505371.008338377,
        190.56085434020005,
        -457.58786455704814,
        190.56085434020005,
        27383562.48292456,
        1399821.5442590625,
        -457.58786455704814,
        1399821.5442590625,
        13020426.954039402,
    ],
    scale_numerator: 920_000,
    scale_denominator: 56_367,
};

const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh017/source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh017/source-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct MoleProspectorHullSource {
    pieces: Arc<[SmoothPiece; PROSPECTOR_PIECES]>,
}

impl MoleProspectorHullSource {
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
                b"MSH017V1",
                PROSPECTOR_DETAILED_VERTICES,
                PROSPECTOR_NATIVE_VERTICES,
            )?,
        })
    }

    pub fn pieces(&self) -> &[SmoothPiece; PROSPECTOR_PIECES] {
        self.pieces.as_ref()
    }

    pub(crate) fn shared_pieces(&self) -> &Arc<[SmoothPiece; PROSPECTOR_PIECES]> {
        &self.pieces
    }

    pub fn engineering(&self) -> MuleEngineering {
        PROSPECTOR_ENGINEERING
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

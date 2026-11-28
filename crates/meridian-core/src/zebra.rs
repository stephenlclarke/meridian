//! Authenticated SH021 engineering and conservative local source geometry.
use crate::docking::{DockingPort, RigidPose, StationPose, Vec3};
use crate::goshawk::{calibrated_pieces, source_sweep_clears};
use crate::mule::{
    EnvelopeError, MuleEngineering, SmoothPiece, rounded_fits_port, rounded_maximum_origin_radius,
};
use std::sync::Arc;

pub const ZEBRA_PIECES: usize = 48;
pub const ZEBRA_DETAILED_VERTICES: usize = 1488;
pub const ZEBRA_NATIVE_VERTICES: usize = 412;
pub const ZEBRA_DETAILED_FACES: usize = 2528;
pub const ZEBRA_NATIVE_FACES: usize = 744;
pub const ZEBRA_NATIVE_EXACT_FACE_LIMIT: usize = 512;
pub(crate) const ZEBRA_MAX_SPEED_UNITS: f32 = 34.0;
pub const ZEBRA_RESOURCE_SHA256: &str =
    "78b8ba0570e4869297e8878c98df5f76503d20136da9e9636309de894ec2394b";
pub const ZEBRA_CONTRACT_SHA256: &str =
    "49908a275fb7e5af7c0715b384e8dcc0a9072e453c94c73826d177fccf3a20ea";
pub const ZEBRA_PHYSICAL_SHA256: &str =
    "8b912a525dc3c5545fed1d7d47caf40a98183e56388b237174fcbfa62db9b63a";
/// The preserved SH021 source engineering adoption, independent of the rounded contact surface.
pub const ZEBRA_ENGINEERING: MuleEngineering = MuleEngineering {
    dry_mass_kilograms: 60108,
    center_of_mass_metres: [
        -0.16599082911601157,
        -1.5846498427771287,
        -3.7233349762449044,
    ],
    inertia_about_center_kg_m2: [
        7434901.512886835,
        -10777.66629331864,
        25896.662714846316,
        -10777.66629331864,
        9811743.07971082,
        101078.5054952639,
        25896.662714846316,
        101078.5054952639,
        3438618.519184986,
    ],
    scale_numerator: 40,
    scale_denominator: 3,
};

const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh021/source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh021/source-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct ZebraHullSource {
    pieces: Arc<[SmoothPiece; ZEBRA_PIECES]>,
}

impl ZebraHullSource {
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
                b"MSH021V1",
                ZEBRA_DETAILED_VERTICES,
                ZEBRA_NATIVE_VERTICES,
            )?,
        })
    }

    pub fn pieces(&self) -> &[SmoothPiece; ZEBRA_PIECES] {
        self.pieces.as_ref()
    }

    pub(crate) fn shared_pieces(&self) -> &Arc<[SmoothPiece; ZEBRA_PIECES]> {
        &self.pieces
    }

    pub fn engineering(&self) -> MuleEngineering {
        ZEBRA_ENGINEERING
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

//! Authenticated SH022 engineering and conservative local source geometry.
use crate::docking::{DockingPort, RigidPose, StationPose, Vec3};
use crate::goshawk::{calibrated_pieces, source_sweep_clears};
use crate::mule::{
    EnvelopeError, MuleEngineering, SmoothPiece, rounded_fits_port, rounded_maximum_origin_radius,
};
use std::sync::Arc;

pub const PEREGRINE_I_PIECES: usize = 69;
pub const PEREGRINE_I_DETAILED_VERTICES: usize = 2072;
pub const PEREGRINE_I_NATIVE_VERTICES: usize = 536;
pub const PEREGRINE_I_DETAILED_FACES: usize = 3484;
pub const PEREGRINE_I_NATIVE_FACES: usize = 964;
pub const PEREGRINE_I_NATIVE_EXACT_FACE_LIMIT: usize = 512;
pub(crate) const PEREGRINE_I_MAX_SPEED_UNITS: f32 = 37.0;
pub const PEREGRINE_I_RESOURCE_SHA256: &str =
    "a4e2830603cded5e8c2435819dcd554b1475096f0b519d2bdcdfd5da3473034d";
pub const PEREGRINE_I_CONTRACT_SHA256: &str =
    "f8502a924dca78b9274ae9ad282cb3d9636ce094087a2ab4365aae7f733a736d";
pub const PEREGRINE_I_PHYSICAL_SHA256: &str =
    "85b41b972c2c274bdcd2c822d5da04ff611c159b2576986196a6d1ff9e01f51e";
/// The preserved SH022 source engineering adoption, independent of the rounded contact surface.
pub const PEREGRINE_I_ENGINEERING: MuleEngineering = MuleEngineering {
    dry_mass_kilograms: 103972,
    center_of_mass_metres: [-0.1372973723978505, -0.9691511846065997, -4.024231079577255],
    inertia_about_center_kg_m2: [
        10318010.310433801,
        -5471.8672232845165,
        40999.1506752947,
        -5471.8672232845165,
        17820304.072745077,
        486944.08607266715,
        40999.1506752947,
        486944.08607266715,
        9930962.821940163,
    ],
    scale_numerator: 800,
    scale_denominator: 59,
};

const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh022/source.vertices");
const MEMBERSHIP: &[u8] = include_bytes!("../../../assets/fleet/sh022/source-piece-membership.bin");

#[derive(Debug, Clone, PartialEq)]
pub struct PeregrineMkIHullSource {
    pieces: Arc<[SmoothPiece; PEREGRINE_I_PIECES]>,
}

impl PeregrineMkIHullSource {
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
                b"MSH022V1",
                PEREGRINE_I_DETAILED_VERTICES,
                PEREGRINE_I_NATIVE_VERTICES,
            )?,
        })
    }

    pub fn pieces(&self) -> &[SmoothPiece; PEREGRINE_I_PIECES] {
        self.pieces.as_ref()
    }

    pub(crate) fn shared_pieces(&self) -> &Arc<[SmoothPiece; PEREGRINE_I_PIECES]> {
        &self.pieces
    }

    pub fn engineering(&self) -> MuleEngineering {
        PEREGRINE_I_ENGINEERING
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

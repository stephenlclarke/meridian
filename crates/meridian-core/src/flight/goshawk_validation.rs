//! Controlled SH008 approach using one World and the authenticated SH028 station.
use super::{FlightInput, FlightStart, StepError, World, flight_pose};
use crate::docking::{RigidPose, Vec3};
use crate::goshawk::{
    GOSHAWK_CONTRACT_SHA256, GOSHAWK_MAX_SPEED_UNITS, GOSHAWK_PIECES, GOSHAWK_RESOURCE_SHA256,
    GoshawkEngineering, GoshawkHullSource,
};
use crate::mule::{SmoothPiece, rounded_maximum_origin_radius};
use crate::station::{DISTANCE_PER_SPEED_UNIT, OriginalStationData, StationError};
use std::sync::{Arc, Weak};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoshawkApproachStatus {
    Approaching,
    Entered,
    StationContact,
    ObserverContact,
    EntryConditions,
}

/// One read-only publication of the World-owned validation hull.
#[derive(Debug, Clone)]
pub struct GoshawkApproachSnapshot {
    pose: RigidPose,
    speed: f32,
    status: GoshawkApproachStatus,
    pieces: Arc<[SmoothPiece; GOSHAWK_PIECES]>,
    engineering: &'static GoshawkEngineering,
    tick: u64,
    scope: Weak<()>,
}

impl GoshawkApproachSnapshot {
    pub fn pose(&self) -> RigidPose {
        self.pose
    }
    pub fn speed_units(&self) -> f32 {
        self.speed
    }
    pub fn status(&self) -> GoshawkApproachStatus {
        self.status
    }
    pub fn world_tick(&self) -> u64 {
        self.tick
    }
    pub fn pieces(&self) -> &Arc<[SmoothPiece; GOSHAWK_PIECES]> {
        &self.pieces
    }
    pub fn engineering(&self) -> &'static GoshawkEngineering {
        self.engineering
    }
    /// Position of the stock centre of mass in station metres.
    pub fn center_of_mass_world_metres(&self) -> Vec3 {
        let [x, y, z] = self.engineering.center_of_mass_metres;
        let offset = self.pose.orientation.apply(Vec3::new(x, y, z));
        Vec3::new(
            self.pose.position.x + offset.x,
            self.pose.position.y + offset.y,
            self.pose.position.z + offset.z,
        )
    }
    pub fn source_identity(&self) -> (&'static str, &'static str) {
        (GOSHAWK_RESOURCE_SHA256, GOSHAWK_CONTRACT_SHA256)
    }
}

#[derive(Debug)]
pub(super) struct Approach {
    source: GoshawkHullSource,
    pose: RigidPose,
    speed: f32,
    status: GoshawkApproachStatus,
    scope: Arc<()>,
}

impl Approach {
    fn advance(&mut self, data: &OriginalStationData, observer: RigidPose) {
        use GoshawkApproachStatus as Status;
        if self.status != Status::Approaching {
            return;
        }
        let before = self.pose;
        let physical_speed = f64::from(self.speed) * f64::from(DISTANCE_PER_SPEED_UNIT);
        let distance = physical_speed * f64::from(crate::FIXED_DELTA_SECONDS);
        let forward = before.orientation.z;
        let proposed = RigidPose {
            position: Vec3::new(
                before.position.x + forward.x * distance,
                before.position.y + forward.y * distance,
                before.position.z + forward.z * distance,
            ),
            ..before
        };
        let crossing = data.depth(before) > 0.0 && data.depth(proposed) <= 0.0;

        self.status = if !data.goshawk_motion_clear(&self.source, before, proposed, 0.0) {
            Status::StationContact
        } else if !data.goshawk_observer_clear(&self.source, before, proposed, observer) {
            Status::ObserverContact
        } else if crossing
            && !data.goshawk_entry_conditions(
                &self.source,
                before,
                proposed,
                physical_speed,
                0.0,
                0.0,
            )
        {
            Status::EntryConditions
        } else {
            self.pose = proposed;
            if crossing {
                Status::Entered
            } else {
                Status::Approaching
            }
        };
        if self.status != Status::Approaching {
            self.speed = 0.0;
        }
    }
}

fn start_pose(start: FlightStart) -> Result<RigidPose, StationError> {
    let norm = start.orientation.iter().map(|v| v * v).sum::<f32>();
    if !(0.0..=GOSHAWK_MAX_SPEED_UNITS).contains(&start.speed)
        || start.speed <= 0.0
        || !norm.is_finite()
        || (norm - 1.0).abs() > 1e-5
    {
        return Err(StationError::InvalidPose);
    }
    let pose = flight_pose(start.position, start.orientation).ok_or(StationError::InvalidPose)?;
    if pose.orientation.z.z >= 0.0 {
        return Err(StationError::InvalidPose);
    }
    Ok(pose)
}

impl World {
    /// Explicit physical validation with one fixed observer and one inbound SH008.
    /// No commander, ordinary population, crew, equipment, or save is created.
    pub fn goshawk_docking_validation_scenario(
        observer: FlightStart,
        approach: FlightStart,
    ) -> Result<Self, StationError> {
        if observer.speed != 0.0 {
            return Err(StationError::InvalidPose);
        }
        let mut world = Self::in_flight_at_original_station(observer)?;
        let data = world
            .station_data
            .as_ref()
            .ok_or(StationError::InvalidPose)?;
        let source = GoshawkHullSource::bundled().map_err(|_| StationError::InvalidMesh)?;
        let pose = start_pose(approach)?;
        let observer = flight_pose(world.state.position, world.state.orientation)
            .ok_or(StationError::InvalidPose)?;
        let radius = rounded_maximum_origin_radius(source.pieces().as_ref());
        let delta = Vec3::new(
            pose.position.x - observer.position.x,
            pose.position.y - observer.position.y,
            pose.position.z - observer.position.z,
        );
        let separation = (radius + data.player_radius()) * 1.001 + 0.001;
        if data.depth(pose) <= radius * 1.001
            || delta.dot(delta) <= separation * separation
            || !data.goshawk_motion_clear(&source, pose, pose, 0.0)
        {
            return Err(StationError::InvalidPose);
        }
        world.goshawk_validation = Some(Approach {
            source,
            pose,
            speed: approach.speed,
            status: GoshawkApproachStatus::Approaching,
            scope: Arc::new(()),
        });
        Ok(world)
    }

    pub fn goshawk_approach_snapshot(&self) -> Option<GoshawkApproachSnapshot> {
        let approach = self.goshawk_validation.as_ref()?;
        Some(GoshawkApproachSnapshot {
            pose: approach.pose,
            speed: approach.speed,
            status: approach.status,
            pieces: approach.source.pieces().clone(),
            engineering: approach.source.engineering(),
            tick: self.state.tick,
            scope: Arc::downgrade(&approach.scope),
        })
    }

    pub fn goshawk_snapshot_is_current(&self, frame: &GoshawkApproachSnapshot) -> bool {
        self.goshawk_validation.as_ref().is_some_and(|approach| {
            frame.scope.ptr_eq(&Arc::downgrade(&approach.scope))
                && Arc::ptr_eq(&frame.pieces, approach.source.pieces())
                && frame.tick == self.state.tick
        })
    }

    pub fn goshawk_observer_pose(&self, frame: &GoshawkApproachSnapshot) -> Option<RigidPose> {
        if !self.goshawk_snapshot_is_current(frame) {
            return None;
        }
        flight_pose(self.state.position, self.state.orientation)
    }

    pub(super) fn controlled_hull_step(
        &mut self,
        input: FlightInput,
    ) -> Option<Result<(), StepError>> {
        if self.goshawk_validation.is_some() {
            Some(self.step_goshawk_validation(input))
        } else if self.mule_validation.is_some() {
            Some(self.step_mule_validation(input))
        } else {
            None
        }
    }

    fn step_goshawk_validation(&mut self, input: FlightInput) -> Result<(), StepError> {
        if input != FlightInput::default() || self.state.speed != 0.0 {
            return Err(StepError::InvalidMode);
        }
        let tick = self
            .state
            .tick
            .checked_add(1)
            .ok_or(StepError::TickOverflow)?;
        let observer = flight_pose(self.state.position, self.state.orientation)
            .ok_or(StepError::NonFiniteState)?;
        let data = self.station_data.as_ref().ok_or(StepError::InvalidMode)?;
        let goshawk = self
            .goshawk_validation
            .as_mut()
            .ok_or(StepError::InvalidMode)?;
        goshawk.advance(data, observer);
        self.state.tick = tick;
        Ok(())
    }
}

#[cfg(test)]
mod tests;

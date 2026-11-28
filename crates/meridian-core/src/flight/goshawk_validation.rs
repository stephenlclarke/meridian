//! Controlled Goshawk approaches using one World and the authenticated SH028 station.
use super::{FlightInput, FlightStart, StepError, World, flight_pose};
use crate::docking::{RigidPose, Vec3};
use crate::goshawk::{
    GOSHAWK_CONTRACT_SHA256, GOSHAWK_MAX_SPEED_UNITS, GOSHAWK_PIECES, GOSHAWK_RESOURCE_SHA256,
    GoshawkEngineering, GoshawkHullSource,
};
use crate::goshawk_custom::{
    CUSTOM_CONTRACT_SHA256, CUSTOM_MAX_SPEED_UNITS, CUSTOM_PIECES, CUSTOM_RESOURCE_SHA256,
    GoshawkCustomHullSource,
};
use crate::mule::{SmoothPiece, rounded_maximum_origin_radius};
use crate::peregrine_mk_i::{
    PEREGRINE_I_CONTRACT_SHA256, PEREGRINE_I_MAX_SPEED_UNITS, PEREGRINE_I_PIECES,
    PEREGRINE_I_RESOURCE_SHA256, PeregrineMkIHullSource,
};
use crate::station::{DISTANCE_PER_SPEED_UNIT, OriginalStationData, StationError};
use crate::zebra::{
    ZEBRA_CONTRACT_SHA256, ZEBRA_MAX_SPEED_UNITS, ZEBRA_PIECES, ZEBRA_RESOURCE_SHA256,
    ZebraHullSource,
};
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
pub struct GoshawkHullApproachSnapshot<const N: usize, E> {
    pose: RigidPose,
    speed: f32,
    status: GoshawkApproachStatus,
    pieces: Arc<[SmoothPiece; N]>,
    engineering: E,
    tick: u64,
    scope: Weak<()>,
    identity: (&'static str, &'static str),
}

pub type GoshawkApproachSnapshot =
    GoshawkHullApproachSnapshot<GOSHAWK_PIECES, &'static GoshawkEngineering>;
pub type GoshawkCustomApproachSnapshot = GoshawkHullApproachSnapshot<CUSTOM_PIECES, ()>;
pub type ZebraApproachSnapshot = GoshawkHullApproachSnapshot<ZEBRA_PIECES, ()>;
pub type PeregrineMkIApproachSnapshot = GoshawkHullApproachSnapshot<PEREGRINE_I_PIECES, ()>;

impl<const N: usize, E> GoshawkHullApproachSnapshot<N, E> {
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
    pub fn pieces(&self) -> &Arc<[SmoothPiece; N]> {
        &self.pieces
    }
    pub fn source_identity(&self) -> (&'static str, &'static str) {
        self.identity
    }
}

impl GoshawkApproachSnapshot {
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
}

#[derive(Debug)]
enum HullSource {
    Explorer(GoshawkHullSource),
    Custom(GoshawkCustomHullSource),
    Zebra(ZebraHullSource),
    PeregrineMkI(PeregrineMkIHullSource),
}

impl HullSource {
    fn pieces(&self) -> &[SmoothPiece] {
        match self {
            Self::Explorer(source) => source.pieces().as_ref(),
            Self::Custom(source) => source.pieces(),
            Self::Zebra(source) => source.pieces(),
            Self::PeregrineMkI(source) => source.pieces(),
        }
    }

    fn max_speed(&self) -> f32 {
        match self {
            Self::Explorer(_) => GOSHAWK_MAX_SPEED_UNITS,
            Self::Custom(_) => CUSTOM_MAX_SPEED_UNITS,
            Self::Zebra(_) => ZEBRA_MAX_SPEED_UNITS,
            Self::PeregrineMkI(_) => PEREGRINE_I_MAX_SPEED_UNITS,
        }
    }

    fn motion_clear(&self, data: &OriginalStationData, start: RigidPose, end: RigidPose) -> bool {
        match self {
            Self::Explorer(source) => data.goshawk_motion_clear(source, start, end, 0.0),
            Self::Custom(source) => data.goshawk_custom_motion_clear(source, start, end, 0.0),
            Self::Zebra(source) => data.zebra_motion_clear(source, start, end, 0.0),
            Self::PeregrineMkI(source) => data.peregrine_mk_i_motion_clear(source, start, end, 0.0),
        }
    }

    fn observer_clear(
        &self,
        data: &OriginalStationData,
        start: RigidPose,
        end: RigidPose,
        observer: RigidPose,
    ) -> bool {
        match self {
            Self::Explorer(source) => data.goshawk_observer_clear(source, start, end, observer),
            Self::Custom(source) => {
                data.goshawk_custom_observer_clear(source, start, end, observer)
            }
            Self::Zebra(source) => data.zebra_observer_clear(source, start, end, observer),
            Self::PeregrineMkI(source) => {
                data.peregrine_mk_i_observer_clear(source, start, end, observer)
            }
        }
    }

    fn entry_conditions(
        &self,
        data: &OriginalStationData,
        start: RigidPose,
        end: RigidPose,
        speed: f64,
    ) -> bool {
        match self {
            Self::Explorer(source) => {
                data.goshawk_entry_conditions(source, start, end, speed, 0.0, 0.0)
            }
            Self::Custom(source) => {
                data.goshawk_custom_entry_conditions(source, start, end, speed, 0.0, 0.0)
            }
            Self::Zebra(source) => data.zebra_entry_conditions(source, start, end, speed, 0.0, 0.0),
            Self::PeregrineMkI(source) => {
                data.peregrine_mk_i_entry_conditions(source, start, end, speed, 0.0, 0.0)
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct Approach {
    source: HullSource,
    pose: RigidPose,
    speed: f32,
    status: GoshawkApproachStatus,
    scope: Arc<()>,
}

impl Approach {
    fn snapshot<const N: usize, E>(
        &self,
        pieces: &Arc<[SmoothPiece; N]>,
        tick: u64,
        engineering: E,
        identity: (&'static str, &'static str),
    ) -> GoshawkHullApproachSnapshot<N, E> {
        GoshawkHullApproachSnapshot {
            pose: self.pose,
            speed: self.speed,
            status: self.status,
            pieces: pieces.clone(),
            engineering,
            tick,
            scope: Arc::downgrade(&self.scope),
            identity,
        }
    }

    fn snapshot_current<const N: usize, E>(
        &self,
        frame: &GoshawkHullApproachSnapshot<N, E>,
        pieces: &Arc<[SmoothPiece; N]>,
        tick: u64,
    ) -> bool {
        frame.scope.ptr_eq(&Arc::downgrade(&self.scope))
            && Arc::ptr_eq(&frame.pieces, pieces)
            && frame.tick == tick
    }

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

        self.status = if !self.source.motion_clear(data, before, proposed) {
            Status::StationContact
        } else if !self.source.observer_clear(data, before, proposed, observer) {
            Status::ObserverContact
        } else if crossing
            && !self
                .source
                .entry_conditions(data, before, proposed, physical_speed)
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

fn start_pose(start: FlightStart, max_speed: f32) -> Result<RigidPose, StationError> {
    let norm = start.orientation.iter().map(|v| v * v).sum::<f32>();
    if !(0.0..=max_speed).contains(&start.speed)
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
        let source = GoshawkHullSource::bundled().map_err(|_| StationError::InvalidMesh)?;
        Self::goshawk_source_scenario(observer, approach, HullSource::Explorer(source))
    }

    /// The separate SH010-only World approach. There is no ordinary player or fleet placement.
    pub fn goshawk_custom_docking_validation_scenario(
        observer: FlightStart,
        approach: FlightStart,
    ) -> Result<Self, StationError> {
        let source = GoshawkCustomHullSource::bundled().map_err(|_| StationError::InvalidMesh)?;
        Self::goshawk_source_scenario(observer, approach, HullSource::Custom(source))
    }

    /// The separate SH021-only World approach. There is no ordinary player or fleet placement.
    pub fn zebra_docking_validation_scenario(
        observer: FlightStart,
        approach: FlightStart,
    ) -> Result<Self, StationError> {
        let source = ZebraHullSource::bundled().map_err(|_| StationError::InvalidMesh)?;
        Self::goshawk_source_scenario(observer, approach, HullSource::Zebra(source))
    }

    /// The separate SH022-only World approach. There is no ordinary player or fleet placement.
    pub fn peregrine_mk_i_docking_validation_scenario(
        observer: FlightStart,
        approach: FlightStart,
    ) -> Result<Self, StationError> {
        let source = PeregrineMkIHullSource::bundled().map_err(|_| StationError::InvalidMesh)?;
        Self::goshawk_source_scenario(observer, approach, HullSource::PeregrineMkI(source))
    }

    fn goshawk_source_scenario(
        observer: FlightStart,
        approach: FlightStart,
        source: HullSource,
    ) -> Result<Self, StationError> {
        if observer.speed != 0.0 {
            return Err(StationError::InvalidPose);
        }
        let mut world = Self::in_flight_at_original_station(observer)?;
        let data = world
            .station_data
            .as_ref()
            .ok_or(StationError::InvalidPose)?;
        let pose = start_pose(approach, source.max_speed())?;
        let observer = flight_pose(world.state.position, world.state.orientation)
            .ok_or(StationError::InvalidPose)?;
        let radius = rounded_maximum_origin_radius(source.pieces());
        let delta = Vec3::new(
            pose.position.x - observer.position.x,
            pose.position.y - observer.position.y,
            pose.position.z - observer.position.z,
        );
        let separation = (radius + data.player_radius()) * 1.001 + 0.001;
        if data.depth(pose) <= radius * 1.001
            || delta.dot(delta) <= separation * separation
            || !source.motion_clear(data, pose, pose)
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
        let HullSource::Explorer(source) = &approach.source else {
            return None;
        };
        Some(approach.snapshot(
            source.pieces(),
            self.state.tick,
            source.engineering(),
            (GOSHAWK_RESOURCE_SHA256, GOSHAWK_CONTRACT_SHA256),
        ))
    }

    pub fn goshawk_custom_approach_snapshot(&self) -> Option<GoshawkCustomApproachSnapshot> {
        let approach = self.goshawk_validation.as_ref()?;
        let HullSource::Custom(source) = &approach.source else {
            return None;
        };
        Some(approach.snapshot(
            source.shared_pieces(),
            self.state.tick,
            (),
            (CUSTOM_RESOURCE_SHA256, CUSTOM_CONTRACT_SHA256),
        ))
    }

    pub fn zebra_approach_snapshot(&self) -> Option<ZebraApproachSnapshot> {
        let approach = self.goshawk_validation.as_ref()?;
        let HullSource::Zebra(source) = &approach.source else {
            return None;
        };
        Some(approach.snapshot(
            source.shared_pieces(),
            self.state.tick,
            (),
            (ZEBRA_RESOURCE_SHA256, ZEBRA_CONTRACT_SHA256),
        ))
    }

    pub fn peregrine_mk_i_approach_snapshot(&self) -> Option<PeregrineMkIApproachSnapshot> {
        let approach = self.goshawk_validation.as_ref()?;
        let HullSource::PeregrineMkI(source) = &approach.source else {
            return None;
        };
        Some(approach.snapshot(
            source.shared_pieces(),
            self.state.tick,
            (),
            (PEREGRINE_I_RESOURCE_SHA256, PEREGRINE_I_CONTRACT_SHA256),
        ))
    }

    pub fn goshawk_snapshot_is_current(&self, frame: &GoshawkApproachSnapshot) -> bool {
        self.goshawk_validation.as_ref().is_some_and(|approach| {
            if let HullSource::Explorer(source) = &approach.source {
                approach.snapshot_current(frame, source.pieces(), self.state.tick)
            } else {
                false
            }
        })
    }

    pub fn goshawk_custom_snapshot_is_current(
        &self,
        frame: &GoshawkCustomApproachSnapshot,
    ) -> bool {
        self.goshawk_validation.as_ref().is_some_and(|approach| {
            if let HullSource::Custom(source) = &approach.source {
                approach.snapshot_current(frame, source.shared_pieces(), self.state.tick)
            } else {
                false
            }
        })
    }

    pub fn zebra_snapshot_is_current(&self, frame: &ZebraApproachSnapshot) -> bool {
        self.goshawk_validation.as_ref().is_some_and(|approach| {
            if let HullSource::Zebra(source) = &approach.source {
                approach.snapshot_current(frame, source.shared_pieces(), self.state.tick)
            } else {
                false
            }
        })
    }

    pub fn peregrine_mk_i_snapshot_is_current(&self, frame: &PeregrineMkIApproachSnapshot) -> bool {
        self.goshawk_validation.as_ref().is_some_and(|approach| {
            if let HullSource::PeregrineMkI(source) = &approach.source {
                approach.snapshot_current(frame, source.shared_pieces(), self.state.tick)
            } else {
                false
            }
        })
    }

    pub fn goshawk_observer_pose(&self, frame: &GoshawkApproachSnapshot) -> Option<RigidPose> {
        if !self.goshawk_snapshot_is_current(frame) {
            return None;
        }
        flight_pose(self.state.position, self.state.orientation)
    }

    pub fn goshawk_custom_observer_pose(
        &self,
        frame: &GoshawkCustomApproachSnapshot,
    ) -> Option<RigidPose> {
        if !self.goshawk_custom_snapshot_is_current(frame) {
            return None;
        }
        flight_pose(self.state.position, self.state.orientation)
    }

    pub fn zebra_observer_pose(&self, frame: &ZebraApproachSnapshot) -> Option<RigidPose> {
        if !self.zebra_snapshot_is_current(frame) {
            return None;
        }
        flight_pose(self.state.position, self.state.orientation)
    }

    pub fn peregrine_mk_i_observer_pose(
        &self,
        frame: &PeregrineMkIApproachSnapshot,
    ) -> Option<RigidPose> {
        if !self.peregrine_mk_i_snapshot_is_current(frame) {
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

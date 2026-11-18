//! Controlled SH001 source scenario. This is not an ambient actor or acquisition path.
use super::{NpcId, NpcPhase, NpcSnapshot, Population};
use crate::docking::{RigidPose, Vec3};
use crate::mule::{
    MULE_CONTRACT_SHA256, MULE_MAX_SPEED_UNITS, MULE_RESOURCE_SHA256, MuleHullSource,
    MulePresentation,
};
use crate::navigation::SystemId;
use crate::station::{DISTANCE_PER_SPEED_UNIT, OriginalStationData};
use std::sync::{Arc, Weak};

const SHIELD_CAPACITY: u16 = 24000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuleValidationRefusal {
    StationContact,
    ObserverContact,
    EntryConditions,
}

/// Read-only World publication. Descriptive actor IDs and source hashes grant no effects.
/// Private fields prevent mixing an actor pose with another publication's presentation.
#[derive(Debug, Clone)]
pub struct MuleActorSnapshot {
    actor: NpcSnapshot,
    presentation: Arc<MulePresentation>,
    tick: u64,
    generation: u64,
    scope: Weak<()>,
    refusal: Option<MuleValidationRefusal>,
    crew_aboard: Option<u8>,
}
impl MuleActorSnapshot {
    pub fn is_renas_encounter(&self) -> bool {
        self.actor.id.system() == SystemId::RENAS && self.actor.id.serial() == u8::MAX - 1
    }
    /// Physical occupancy from the SH001 native crew count. It grants no crew service or effects.
    pub fn crew_aboard(&self) -> Option<u8> {
        self.crew_aboard
    }
    pub fn actor(&self) -> &NpcSnapshot {
        &self.actor
    }
    pub fn presentation(&self) -> &Arc<MulePresentation> {
        &self.presentation
    }
    pub fn world_tick(&self) -> u64 {
        self.tick
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn source_identity(&self) -> (&'static str, &'static str) {
        (MULE_RESOURCE_SHA256, MULE_CONTRACT_SHA256)
    }
    pub fn motion_refusal(&self) -> Option<MuleValidationRefusal> {
        self.refusal
    }
}

#[derive(Debug, Clone)]
pub(crate) struct State {
    actor: NpcSnapshot,
    source: MuleHullSource,
    presentation: Arc<MulePresentation>,
    tick: u64,
    generation: u64,
    scope: Arc<()>,
    renas_deferred: bool,
    refusal: Option<MuleValidationRefusal>,
}
impl State {
    pub(crate) fn new(
        seed: u64,
        source: MuleHullSource,
        pose: RigidPose,
        speed_units: f32,
        approaching: bool,
    ) -> Result<Self, crate::station::StationError> {
        if seed == 0 || !(0.0..=MULE_MAX_SPEED_UNITS).contains(&speed_units) {
            return Err(crate::station::StationError::InvalidPose);
        }
        let presentation = Arc::new(
            source
                .presentation(SHIELD_CAPACITY, SHIELD_CAPACITY, SHIELD_CAPACITY)
                .map_err(|_| crate::station::StationError::InvalidMesh)?,
        );
        Ok(Self {
            actor: NpcSnapshot {
                // Reserved controlled-scenario serial; never placed in ordinary traffic/save.
                id: NpcId {
                    profile: seed,
                    system: SystemId::AVENOR,
                    serial: u8::MAX,
                },
                // Original primaryRole=trader; no population weights/ambient admission inferred.
                population: Population::Trader,
                hull_id: "SH001",
                pose,
                // Widen only after validating the source-domain value.
                speed_units: f64::from(speed_units),
                phase: if approaching {
                    NpcPhase::Approach
                } else {
                    NpcPhase::Departing
                },
                energy_hundredths: 29000,
                electrical_remaining: None,
                electrical_disabled: false,
                // Crew/electrical custody is not created by this validation scenario.
                emp_crew_aboard: None,
                front_shield_hundredths: SHIELD_CAPACITY,
                rear_shield_hundredths: SHIELD_CAPACITY,
                heat_fifths: 0,
                cooldown_ticks: 0,
                cash_tenths: 0,
                cargo: None,
                wanted: 0,
            },
            source,
            presentation,
            tick: 0,
            generation: 1,
            scope: Arc::new(()),
            renas_deferred: false,
            refusal: None,
        })
    }
    pub(crate) fn renas_encounter(
        owner: u64,
        generation: u64,
        source: MuleHullSource,
        pose: RigidPose,
    ) -> Result<Self, crate::station::StationError> {
        let mut state = Self::new(owner, source, pose, 0.0, false)?;
        state.actor.id.system = SystemId::RENAS;
        state.actor.id.serial = u8::MAX - 1;
        state.actor.phase = NpcPhase::Outbound;
        state.generation = generation;
        Ok(state)
    }
    pub(crate) fn renas_is_deferred(&self) -> bool {
        self.renas_deferred
    }
    pub(crate) fn defer_renas(&mut self, deferred: bool) {
        self.renas_deferred = deferred;
    }
    pub(crate) fn renas_publication(&self, tick: u64) -> MuleActorSnapshot {
        let mut frame = self.snapshot();
        frame.tick = tick;
        frame.crew_aboard = Some(2);
        frame
    }
    pub(crate) fn renas_frame_matches(&self, frame: &MuleActorSnapshot, tick: u64) -> bool {
        frame.scope.ptr_eq(&Arc::downgrade(&self.scope))
            && frame.tick == tick
            && frame.generation == self.generation
            && frame.actor == self.actor
            && frame.is_renas_encounter()
            && frame.crew_aboard == Some(2)
            && Arc::ptr_eq(&frame.presentation, &self.presentation)
    }
    pub(crate) fn source(&self) -> &MuleHullSource {
        &self.source
    }
    pub(crate) fn pose(&self) -> RigidPose {
        self.actor.pose
    }
    pub(crate) fn snapshot(&self) -> MuleActorSnapshot {
        MuleActorSnapshot {
            actor: self.actor,
            presentation: self.presentation.clone(),
            tick: self.tick,
            generation: self.generation,
            scope: Arc::downgrade(&self.scope),
            refusal: self.refusal,
            crew_aboard: None,
        }
    }
    pub(crate) fn is_current(&self, frame: &MuleActorSnapshot) -> bool {
        frame.scope.ptr_eq(&Arc::downgrade(&self.scope))
            && frame.tick == self.tick
            && frame.generation == self.generation
            && frame.actor == self.actor
            && Arc::ptr_eq(&frame.presentation, &self.presentation)
    }
    pub(crate) fn advance(
        &self,
        next_tick: u64,
        data: &OriginalStationData,
        observer: RigidPose,
    ) -> Result<Self, crate::StepError> {
        if self.tick.checked_add(1) != Some(next_tick) {
            return Err(crate::StepError::StaleCommand);
        }
        let mut next = self.clone();
        next.generation = self
            .generation
            .checked_add(1)
            .ok_or(crate::StepError::TickOverflow)?;
        next.tick = next_tick;
        if self.actor.phase == NpcPhase::Complete
            || (self.actor.phase == NpcPhase::Approach && self.actor.speed_units == 0.0)
        {
            return Ok(next);
        }
        let before = self.actor.pose;
        let distance = self.actor.speed_units
            * f64::from(DISTANCE_PER_SPEED_UNIT)
            * f64::from(crate::FIXED_DELTA_SECONDS);
        let forward = before.orientation.z;
        let after = RigidPose {
            position: Vec3::new(
                before.position.x + forward.x * distance,
                before.position.y + forward.y * distance,
                before.position.z + forward.z * distance,
            ),
            orientation: before.orientation,
        };
        let approaching = self.actor.phase == NpcPhase::Approach;
        let crosses_entry = approaching && data.depth(before) > 0.0 && data.depth(after) <= 0.0;
        // Classify the physical sweep before the aperture and entry rules.
        // The independent entry predicate does not sweep the station again.
        next.refusal = if !data.mule_motion_clear(&self.source, before, after, 0.0) {
            Some(MuleValidationRefusal::StationContact)
        } else if crosses_entry
            && !data.mule_entry_conditions(
                &self.source,
                before,
                after,
                self.actor.speed_units * f64::from(DISTANCE_PER_SPEED_UNIT),
                0.0,
                0.0,
            )
        {
            Some(MuleValidationRefusal::EntryConditions)
        } else if !data.mule_observer_clear(&self.source, before, after, observer) {
            Some(MuleValidationRefusal::ObserverContact)
        } else {
            None
        };
        if next.refusal.is_some() {
            next.actor.speed_units = 0.0;
        } else {
            next.actor.pose = after;
            if crosses_entry {
                next.actor.phase = NpcPhase::Complete;
                next.actor.speed_units = 0.0;
            } else if !approaching {
                next.actor.phase = NpcPhase::Outbound;
            }
        }
        Ok(next)
    }
}

//! Source-defined solar collection and thermal exposure on the actual local flight scene.
//! The retained trace is run-length encoded accepted motion, never a caller fuel grant.
use super::{FlightInput, FlightMode, Snapshot, World};
use crate::{
    celestial::Body,
    navigation::SystemId,
    save::{Reader, SaveError, Writer},
};
use std::sync::Arc;

mod history;
pub(super) use history::replay_launch_step;
const INVALID: SaveError = SaveError::InvalidState("solar exposure or fuel collection chronology");
const DELTA: f64 = 1.0 / 60.0;
const COOLING: f64 = 1.0 - 0.14 * DELTA;
const CORONA_LIMIT: f64 = 3.0;
const THERMAL_LIMIT: f64 = 4.0;
const NOSE_THRESHOLD: f64 = 0.8;
const MAX_RUN_STEPS: u16 = 600;
const RUNS_PER_CHUNK: usize = 64;
const RECEIPTS_PER_CHUNK: usize = 256;
const NPC_STOPS_PER_CHUNK: usize = 16;

/// Immutable completed chunks are shared across prospective flight steps. Only the
/// current, fixed-size tail is cloned for a normal append or held-control extension.
#[derive(Debug, Clone, PartialEq)]
struct ChunkLog<T, const N: usize> {
    completed: Arc<Vec<Arc<Vec<T>>>>,
    tail: Arc<Vec<T>>,
    len: usize,
}
impl<T, const N: usize> Default for ChunkLog<T, N> {
    fn default() -> Self {
        Self {
            completed: Arc::new(Vec::new()),
            tail: Arc::new(Vec::new()),
            len: 0,
        }
    }
}
impl<T, const N: usize> ChunkLog<T, N> {
    fn len(&self) -> usize {
        self.len
    }
    fn is_empty(&self) -> bool {
        self.len == 0
    }
    fn chunks(&self) -> impl Iterator<Item = &[T]> {
        self.completed
            .iter()
            .map(|chunk| chunk.as_slice())
            .chain(std::iter::once(self.tail.as_slice()))
            .filter(|chunk| !chunk.is_empty())
    }
    fn iter(&self) -> impl Iterator<Item = &T> {
        self.chunks().flatten()
    }
    fn last(&self) -> Option<&T> {
        self.tail.last()
    }
}
impl<T: Clone, const N: usize> ChunkLog<T, N> {
    fn push(&mut self, value: T) {
        if self.tail.len() == N {
            Arc::make_mut(&mut self.completed).push(self.tail.clone());
            self.tail = Arc::new(Vec::new());
        }
        Arc::make_mut(&mut self.tail).push(value);
        self.len += 1;
    }
    fn last_mut(&mut self) -> Option<&mut T> {
        Arc::make_mut(&mut self.tail).last_mut()
    }
    #[cfg(test)]
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        if index / N < self.completed.len() {
            let chunks = Arc::make_mut(&mut self.completed);
            return Arc::make_mut(&mut chunks[index / N]).get_mut(index % N);
        }
        Arc::make_mut(&mut self.tail).get_mut(index % N)
    }
    #[cfg(test)]
    fn remove(&mut self, index: usize) {
        let mut records = self.iter().cloned().collect::<Vec<_>>();
        records.remove(index);
        *self = Self::default();
        for record in records {
            self.push(record);
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SolarScoopStatus {
    #[default]
    Inactive,
    NoScoops,
    TurnTowardSun,
    Collecting,
    TankFull,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SolarSnapshot {
    pub status: SolarScoopStatus,
    pub cabin_temperature: f64,
    pub fuel_remainder: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MotionFrame {
    position: [f32; 3],
    orientation: [f32; 4],
    speed: f32,
}
impl From<Snapshot> for MotionFrame {
    fn from(frame: Snapshot) -> Self {
        Self {
            position: frame.position,
            orientation: frame.orientation,
            speed: frame.speed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Motion {
    Active(FlightInput),
    Stopped { input: FlightInput, kind: StopKind },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StopKind {
    Celestial,
    Traffic,
    Mule,
    Asteroid,
    Station,
    Entry,
}
impl MotionFrame {
    fn advance(self, motion: Motion, hull: super::PlayerHullKind) -> Option<Self> {
        let Motion::Active(input) = motion else {
            return Some(Self { speed: 0.0, ..self });
        };
        input.validate().ok()?;
        let (max_speed, acceleration, pitch_rate, yaw_rate, roll_rate) = hull.flight_rates();
        let yaw = super::axis_rotation(1, input.yaw * yaw_rate * super::FIXED_DELTA_SECONDS);
        let pitch = super::axis_rotation(0, -input.pitch * pitch_rate * super::FIXED_DELTA_SECONDS);
        let roll = super::axis_rotation(2, input.roll * roll_rate * super::FIXED_DELTA_SECONDS);
        let orientation = super::normalize(super::multiply(
            super::multiply(super::multiply(self.orientation, yaw), pitch),
            roll,
        ))?;
        let throttle = if input.brake { -1.0 } else { input.throttle };
        let speed = (self.speed + throttle * acceleration * super::FIXED_DELTA_SECONDS)
            .clamp(0.0, max_speed);
        let mut position = self.position;
        for (axis, direction) in position.iter_mut().zip(super::forward(orientation)) {
            *axis +=
                direction * speed * super::FIXED_DELTA_SECONDS * super::DISTANCE_PER_SPEED_UNIT;
        }
        Some(Self {
            position,
            orientation,
            speed,
        })
    }
    fn ratio_and_nose(self, sun: Body) -> (f64, f64) {
        let toward = std::array::from_fn::<_, 3, _>(|i| {
            [sun.position.x, sun.position.y, sun.position.z][i] - f64::from(self.position[i])
        });
        let distance = toward.iter().map(|axis| axis * axis).sum::<f64>().sqrt();
        let dot = toward
            .into_iter()
            .zip(super::forward(self.orientation))
            .map(|(axis, direction)| axis * f64::from(direction))
            .sum::<f64>()
            / distance.max(1.0);
        (distance / sun.radius, dot)
    }
    fn valid(self, hull: super::PlayerHullKind) -> bool {
        let norm = self
            .orientation
            .iter()
            .map(|value| value * value)
            .sum::<f32>();
        self.position.into_iter().all(f32::is_finite)
            && self.speed.is_finite()
            && (0.0..=hull.flight_rates().0).contains(&self.speed)
            && norm.is_finite()
            && (norm - 1.0).abs() <= 1e-5
            && crate::station::flight_pose(self.position, self.orientation).is_some()
    }
}
impl Motion {
    fn input(self) -> FlightInput {
        match self {
            Self::Active(input) | Self::Stopped { input, .. } => input,
        }
    }
    fn refusal(self) -> Option<super::MotionRefusal> {
        use super::MotionRefusal;
        match self {
            Self::Active(_) => None,
            Self::Stopped { kind, .. } => Some(match kind {
                StopKind::Celestial => MotionRefusal::CelestialContact,
                StopKind::Traffic | StopKind::Mule => MotionRefusal::NpcContact,
                StopKind::Asteroid => MotionRefusal::AsteroidContact,
                StopKind::Station => MotionRefusal::StationContact,
                StopKind::Entry => MotionRefusal::EntryConditions,
            }),
        }
    }
}

fn cool(temperature: f64, ticks: u64) -> f64 {
    temperature * COOLING.powf(ticks as f64)
}

fn accepted_exposure(
    prior: SolarSnapshot,
    ratio: f64,
    nose: f64,
    speed: f64,
    fitted: bool,
    fuel: u16,
    capacity: u16,
) -> (SolarSnapshot, u16) {
    let radiation = if ratio <= THERMAL_LIMIT {
        (0.04 * (3.2 / ratio.max(0.2)).powi(4)).min(2.0)
    } else {
        0.0
    };
    let cabin_temperature = (prior.cabin_temperature
        + DELTA * (radiation - 0.14 * prior.cabin_temperature))
        .clamp(0.0, 1.0);
    let result = |status, fuel_remainder| SolarSnapshot {
        status,
        cabin_temperature,
        fuel_remainder,
    };
    if cabin_temperature >= 1.0 || ratio <= 1.0 || ratio > CORONA_LIMIT {
        return (result(SolarScoopStatus::Inactive, 0.0), 0);
    }
    if !fitted {
        return (result(SolarScoopStatus::NoScoops, 0.0), 0);
    }
    if fuel >= capacity {
        return (result(SolarScoopStatus::TankFull, 0.0), 0);
    }
    if speed <= 0.0 || nose < NOSE_THRESHOLD {
        return (result(SolarScoopStatus::TurnTowardSun, 0.0), 0);
    }
    let accumulated = prior.fuel_remainder + DELTA * 7.0 * (speed / 8.0).min(1.0);
    let gained = (accumulated + 1e-9).floor() as u16;
    if gained >= capacity - fuel {
        return (result(SolarScoopStatus::TankFull, 0.0), capacity - fuel);
    }
    (
        result(
            SolarScoopStatus::Collecting,
            (accumulated - f64::from(gained)).max(0.0),
        ),
        gained,
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Anchor {
    system: SystemId,
    visit: u64,
    tick: u64,
    clock: u64,
    frame: MotionFrame,
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Run {
    opening: Anchor,
    first_revision: u64,
    first_fuel: u16,
    capacity: u16,
    item: u64,
    motion: Motion,
    steps: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Receipt {
    pub(super) revision: u64,
    pub(super) tick: u64,
    pub(super) before: u16,
    pub(super) added: u16,
}
#[derive(Debug, Clone, PartialEq)]
struct NpcRefusal {
    clock: u64,
    scene: crate::traffic::MissileScene,
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Last {
    anchor: Anchor,
    fuel: u16,
    revision: u64,
    condition: SolarSnapshot,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct State {
    runs: ChunkLog<Run, RUNS_PER_CHUNK>,
    receipts: ChunkLog<Receipt, RECEIPTS_PER_CHUNK>,
    npc_refusals: ChunkLog<NpcRefusal, NPC_STOPS_PER_CHUNK>,
    last: Option<Last>,
    legacy_origin: Option<Anchor>,
    legacy_format: bool,
    #[cfg(test)]
    file_limit: Option<usize>,
}
impl State {
    pub(super) fn current(&self, mode: FlightMode, visit: u64, clock: u64) -> SolarSnapshot {
        let Some(last) = self.last.filter(|last| {
            mode != FlightMode::Docked && last.anchor.visit == visit && last.anchor.clock <= clock
        }) else {
            return SolarSnapshot::default();
        };
        let elapsed = clock - last.anchor.clock;
        if elapsed == 0 {
            last.condition
        } else {
            SolarSnapshot {
                cabin_temperature: cool(last.condition.cabin_temperature, elapsed),
                ..SolarSnapshot::default()
            }
        }
    }
    fn before_sample(&self, opening: Anchor) -> SolarSnapshot {
        self.current(FlightMode::Flight, opening.visit, opening.clock)
    }
    pub(super) fn receipts(&self) -> impl Iterator<Item = &Receipt> {
        self.receipts.iter()
    }
    pub(super) fn receipt_count(&self) -> usize {
        self.receipts.len()
    }
    pub(super) fn fatal_clock(&self) -> Option<u64> {
        self.last
            .filter(|last| last.condition.cabin_temperature >= 1.0)
            .map(|last| last.anchor.clock)
    }
    #[cfg(test)]
    pub(in crate::flight) fn phase_fixture(world: &World) -> Option<Self> {
        // Only the isolated cross-producer ordering test uses this marker; it deliberately
        // does not claim a valid solar flight trace or allow a whole-world save.
        let identity = world.combat_identity()?;
        Some(Self {
            last: Some(Last {
                anchor: Self::anchor(
                    world.state,
                    identity.system,
                    identity.visit,
                    world.combat.physical_tick(),
                ),
                fuel: 0,
                revision: 0,
                condition: SolarSnapshot {
                    cabin_temperature: 1.0,
                    ..SolarSnapshot::default()
                },
            }),
            ..Self::default()
        })
    }
    pub(super) fn is_empty(&self) -> bool {
        self.runs.is_empty() && self.legacy_origin.is_none()
    }
    pub(super) fn legacy_header(&self) -> bool {
        self.legacy_format && self.is_empty()
    }
    pub(super) fn file_limit(&self) -> usize {
        #[cfg(test)]
        if let Some(limit) = self.file_limit {
            return limit;
        }
        crate::save::MAX_SOLAR_HISTORY_BYTES
    }
    fn scene(
        world: &World,
        system: SystemId,
        visit: u64,
        tick: u64,
    ) -> Result<crate::celestial::Snapshot, SaveError> {
        let catalogue = world.navigation_catalogue.as_ref().ok_or(INVALID)?;
        let system = catalogue.system(system).map_err(|_| INVALID)?;
        let anchor = world.state.station.ok_or(INVALID)?.pose.position;
        Ok(crate::celestial::scene(system, visit, tick, anchor))
    }
    fn sun(world: &World, system: SystemId, visit: u64, tick: u64) -> Result<Body, SaveError> {
        Ok(Self::scene(world, system, visit, tick)?.sun)
    }
    fn anchor(frame: Snapshot, system: SystemId, visit: u64, clock: u64) -> Anchor {
        Anchor {
            system,
            visit,
            tick: frame.tick,
            clock,
            frame: frame.into(),
        }
    }
    pub(super) fn prepare(
        &self,
        world: &World,
        next: Snapshot,
        navigation: Option<super::NavigationState>,
        clock: u64,
        motion: Option<Motion>,
        alive: bool,
    ) -> Result<(Self, u16), SaveError> {
        let (Some(account), Some(nav), Some(identity)) = (
            world.commander_account.as_ref(),
            navigation,
            world.combat_identity(),
        ) else {
            return Ok((self.clone(), 0));
        };
        if !world.damage.enabled()
            || !alive
            || next.mode != FlightMode::Flight
            || world.state.mode == FlightMode::Docked
        {
            return Ok((self.clone(), 0));
        }
        let nav = nav.snapshot(account.fuel_tenths());
        if nav.visit != identity.visit || nav.system != identity.system {
            return Ok((self.clone(), 0));
        }
        let after = Self::anchor(next, nav.system, nav.visit, clock);
        let sun = Self::sun(world, nav.system, nav.visit, next.tick)?;
        let (ratio, nose) = after.frame.ratio_and_nose(sun);
        // Once a visit has a solar opening, retain every subsequent accepted exterior
        // step as well. A re-entry can no longer insert an independently invented pose.
        let tracked = self.last.is_some_and(|last| last.anchor.visit == nav.visit)
            || self
                .legacy_origin
                .is_some_and(|origin| origin.visit == nav.visit);
        if ratio > THERMAL_LIMIT && !tracked {
            return Ok((self.clone(), 0));
        }
        if !ratio.is_finite() || ratio <= 1.0 || !nose.is_finite() {
            return Err(INVALID);
        }
        let motion = motion.ok_or(INVALID)?;
        if motion.refusal() != next.motion_refusal {
            return Err(INVALID);
        }
        let opening = Self::anchor(
            world.state,
            nav.system,
            nav.visit,
            world.combat.physical_tick(),
        );
        if opening.frame.advance(motion, world.player_hull_kind()) != Some(after.frame)
            || opening.clock.checked_add(1) != Some(after.clock)
            || opening.tick.checked_add(1) != Some(after.tick)
        {
            return Err(INVALID);
        }
        let item = self
            .runs
            .last()
            .filter(|run| run.opening.visit == nav.visit)
            .map_or_else(
                || {
                    world
                        .modular
                        .scoop_item_at(identity.owner, next.tick, nav.visit)
                        .map_or(0, |item| item.id)
                },
                |run| run.item,
            );
        let fuel = account.fuel_tenths();
        let capacity = account.fuel_capacity();
        let revision = account.revision();
        let (condition, gained) = accepted_exposure(
            self.before_sample(opening),
            ratio,
            nose,
            f64::from(after.frame.speed),
            item != 0,
            fuel,
            capacity,
        );
        let next_revision = if gained == 0 {
            revision
        } else {
            revision.checked_add(1).ok_or(INVALID)?
        };
        let mut state = self.clone();
        state.legacy_format = false;
        if matches!(
            motion,
            Motion::Stopped {
                kind: StopKind::Traffic,
                ..
            }
        ) {
            state.npc_refusals.push(NpcRefusal {
                clock: opening.clock,
                scene: world.traffic.missile_scene(
                    nav.system,
                    opening.tick,
                    crate::traffic::SceneStage::Command(world.combat.emissions),
                ),
            });
        }
        let extends = self.last.is_some_and(|last| {
            last.anchor == opening && last.fuel == fuel && last.revision == revision
        }) && self.runs.last().is_some_and(|run| {
            run.item == item
                && run.capacity == capacity
                && run.motion == motion
                && run.steps < MAX_RUN_STEPS
        });
        if extends {
            let run = state.runs.last_mut().ok_or(INVALID)?;
            run.steps += 1;
        } else {
            state.runs.push(Run {
                opening,
                first_revision: revision,
                first_fuel: fuel,
                capacity,
                item,
                motion,
                steps: 1,
            });
        }
        if gained != 0 {
            state.receipts.push(Receipt {
                revision: next_revision,
                tick: after.tick,
                before: fuel,
                added: gained,
            });
        }
        state.last = Some(Last {
            anchor: after,
            fuel: fuel + gained,
            revision: next_revision,
            condition,
        });
        Ok((state, gained))
    }
    pub(super) fn initialize_legacy_origin(&mut self, world: &World) -> Result<(), SaveError> {
        self.legacy_format = true;
        if !world.damage.enabled() || world.state.mode != FlightMode::Flight || !world.pulse.alive()
        {
            return Ok(());
        }
        let identity = world.combat_identity().ok_or(INVALID)?;
        let anchor = Self::anchor(
            world.state,
            identity.system,
            identity.visit,
            world.combat.physical_tick(),
        );
        let sun = Self::sun(world, identity.system, identity.visit, world.state.tick)?;
        if anchor.frame.ratio_and_nose(sun).0 <= THERMAL_LIMIT {
            self.legacy_origin = Some(anchor);
            self.legacy_format = false;
        }
        Ok(())
    }
    pub(super) fn opening_fuel_anchors(&self) -> impl Iterator<Item = (u64, u16)> + '_ {
        self.runs
            .iter()
            .map(|run| (run.first_revision, run.first_fuel))
    }
}

impl World {
    pub fn solar_snapshot(&self) -> Option<SolarSnapshot> {
        let identity = self.combat_identity()?;
        Some(
            self.solar
                .current(self.state.mode, identity.visit, self.combat.physical_tick()),
        )
    }
    /// Storage exhaustion is a save refusal, never a flight-control condition.
    pub fn solar_save_limit_reached(&self, limit: usize) -> bool {
        self.solar.encoded_len() > limit
    }
}

#[cfg(test)]
mod tests;

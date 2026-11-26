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
const INVALID: SaveError = SaveError::InvalidState("solar exposure or fuel collection chronology");
const DELTA: f64 = 1.0 / 60.0;
const COOLING: f64 = 1.0 - 0.14 * DELTA;
const CORONA_LIMIT: f64 = 3.0;
const THERMAL_LIMIT: f64 = 4.0;
const NOSE_THRESHOLD: f64 = 0.8;
const SOURCE_MAX_RUNS: usize = crate::save::MAX_SAVE_BYTES / 99;
const MAX_REPLAY_STEPS: u64 = 250_000;
const MAX_RUN_STEPS: u16 = 600;
// Keep a separately reserved tail for an actual outward flight rather than
// consuming the last representable step while the commander is in the corona.
const EXIT_RUN_RESERVE: usize = 96;
const EXIT_STEP_RESERVE: u64 = 6000;
const SOFT_PROOF_BYTES: usize = 24_000;
const AGGREGATE_SOFT_BYTES: usize = crate::save::MAX_SAVE_BYTES - 14_000;
const AGGREGATE_SHRINK_ALLOWANCE: usize = 4096;

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
    Stopped,
}
impl MotionFrame {
    fn advance(self, motion: Motion) -> Option<Self> {
        let Motion::Active(input) = motion else {
            return Some(Self { speed: 0.0, ..self });
        };
        input.validate().ok()?;
        let yaw = super::axis_rotation(1, input.yaw * super::YAW_RATE * super::FIXED_DELTA_SECONDS);
        let pitch = super::axis_rotation(
            0,
            -input.pitch * super::PITCH_RATE * super::FIXED_DELTA_SECONDS,
        );
        let roll = super::axis_rotation(
            2,
            input.roll * super::ROLL_RATE * super::FIXED_DELTA_SECONDS,
        );
        let orientation = super::normalize(super::multiply(
            super::multiply(super::multiply(self.orientation, yaw), pitch),
            roll,
        ))?;
        let throttle = if input.brake { -1.0 } else { input.throttle };
        let speed = (self.speed + throttle * super::ACCELERATION * super::FIXED_DELTA_SECONDS)
            .clamp(0.0, super::MAX_SPEED);
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
    fn valid(self) -> bool {
        let norm = self
            .orientation
            .iter()
            .map(|value| value * value)
            .sum::<f32>();
        self.position.into_iter().all(f32::is_finite)
            && self.speed.is_finite()
            && (0.0..=super::MAX_SPEED).contains(&self.speed)
            && norm.is_finite()
            && (norm - 1.0).abs() <= 1e-5
            && crate::station::flight_pose(self.position, self.orientation).is_some()
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
#[derive(Debug, Clone, Copy, PartialEq)]
struct Last {
    anchor: Anchor,
    fuel: u16,
    revision: u64,
    condition: SolarSnapshot,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct State {
    runs: Arc<Vec<Run>>,
    receipts: Arc<Vec<Receipt>>,
    last: Option<Last>,
    legacy_origin: Option<Anchor>,
    sealed: bool,
    external_limit: bool,
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
    pub(super) fn receipts(&self) -> &[Receipt] {
        &self.receipts
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
    fn steps(&self) -> u64 {
        self.runs.iter().map(|run| u64::from(run.steps)).sum()
    }
    fn soft_limit(&self) -> bool {
        self.external_limit || Self::limit_for(self.runs.len(), self.steps(), self.encoded_len())
    }
    fn limit_for(runs: usize, steps: u64, bytes: usize) -> bool {
        runs >= SOURCE_MAX_RUNS - EXIT_RUN_RESERVE
            || steps >= MAX_REPLAY_STEPS - EXIT_STEP_RESERVE
            || bytes >= SOFT_PROOF_BYTES
    }
    fn egress_input(frame: MotionFrame, sun: Body) -> Option<FlightInput> {
        let pose = crate::station::flight_pose(frame.position, frame.orientation)?;
        let away = pose
            .orientation
            .inverse_coordinates(crate::docking::Vec3::new(
                pose.position.x - sun.position.x,
                pose.position.y - sun.position.y,
                pose.position.z - sun.position.z,
            ));
        let yaw = away.x.atan2(away.z);
        let pitch = away.y.atan2(away.x.hypot(away.z));
        // A dead band wider than a full per-tick turn prevents alternating controls.
        // Brake until radial orientation, then accelerate outward without teleporting.
        const ALIGNED: f64 = 0.025;
        Some(if yaw.abs() > ALIGNED {
            FlightInput {
                yaw: yaw.signum() as f32,
                brake: true,
                ..FlightInput::default()
            }
        } else if pitch.abs() > ALIGNED {
            FlightInput {
                pitch: pitch.signum() as f32,
                brake: true,
                ..FlightInput::default()
            }
        } else {
            FlightInput {
                throttle: 1.0,
                ..FlightInput::default()
            }
        })
    }
    pub(super) fn escape_control(&self, world: &World) -> Option<FlightInput> {
        if self.sealed || !self.soft_limit() || world.state.mode != FlightMode::Flight {
            return None;
        }
        let identity = world.combat_identity()?;
        let sun = Self::sun(world, identity.system, identity.visit, world.state.tick).ok()?;
        let frame = MotionFrame::from(world.state);
        if frame.ratio_and_nose(sun).0 <= THERMAL_LIMIT {
            Self::egress_input(frame, sun)
        } else {
            None
        }
    }
    pub(super) fn blocks_entry(&self, world: &World, next: Snapshot) -> bool {
        if world.state.mode != FlightMode::Flight || !world.damage.enabled() {
            return false;
        }
        let Some(identity) = world.combat_identity() else {
            return false;
        };
        let Ok(sun) = Self::sun(world, identity.system, identity.visit, world.state.tick) else {
            return false;
        };
        let entering = MotionFrame::from(next).ratio_and_nose(sun).0 <= THERMAL_LIMIT;
        if !entering {
            return false;
        }
        if self.sealed {
            return true;
        }
        MotionFrame::from(world.state).ratio_and_nose(sun).0 > THERMAL_LIMIT
            && world
                .encoded_current_size()
                .is_ok_and(|size| size >= AGGREGATE_SOFT_BYTES)
    }
    fn sun(world: &World, system: SystemId, visit: u64, tick: u64) -> Result<Body, SaveError> {
        let catalogue = world.navigation_catalogue.as_ref().ok_or(INVALID)?;
        let system = catalogue.system(system).map_err(|_| INVALID)?;
        let anchor = world.state.station.ok_or(INVALID)?.pose.position;
        Ok(crate::celestial::scene(system, visit, tick, anchor).sun)
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
        if self.sealed {
            if ratio <= THERMAL_LIMIT {
                return Err(INVALID);
            }
            return Ok((self.clone(), 0));
        }
        // Once a visit has a solar opening, retain every subsequent accepted exterior
        // step as well. A re-entry can no longer insert an independently invented pose.
        let tracked = self.last.is_some_and(|last| last.anchor.visit == nav.visit)
            || self
                .legacy_origin
                .is_some_and(|origin| origin.visit == nav.visit);
        if ratio > THERMAL_LIMIT && !tracked {
            if self.soft_limit() {
                let mut sealed = self.clone();
                sealed.sealed = true;
                return Ok((sealed, 0));
            }
            return Ok((self.clone(), 0));
        }
        if !ratio.is_finite() || ratio <= 1.0 || !nose.is_finite() {
            return Err(INVALID);
        }
        let motion = motion.ok_or(INVALID)?;
        let opening = Self::anchor(
            world.state,
            nav.system,
            nav.visit,
            world.combat.physical_tick(),
        );
        if opening.frame.advance(motion) != Some(after.frame)
            || opening.clock.checked_add(1) != Some(after.clock)
            || opening.tick.checked_add(1) != Some(after.tick)
        {
            return Err(INVALID);
        }
        let item = world
            .modular
            .scoop_item_at(identity.owner, next.tick, nav.visit)
            .map_or(0, |item| item.id);
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
        if self.steps() >= MAX_REPLAY_STEPS {
            return Err(SaveError::TooLarge);
        }
        let mut state = self.clone();
        let extends = self.last.is_some_and(|last| {
            last.anchor == opening && last.fuel == fuel && last.revision == revision
        }) && self.runs.last().is_some_and(|run| {
            run.item == item
                && run.capacity == capacity
                && run.motion == motion
                && run.steps < MAX_RUN_STEPS
        });
        if extends {
            let run = Arc::make_mut(&mut state.runs).last_mut().ok_or(INVALID)?;
            run.steps += 1;
        } else {
            if self.runs.len() >= SOURCE_MAX_RUNS {
                return Err(SaveError::TooLarge);
            }
            Arc::make_mut(&mut state.runs).push(Run {
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
            Arc::make_mut(&mut state.receipts).push(Receipt {
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
        if !state.soft_limit() {
            let growth = state.encoded_len().saturating_sub(self.encoded_len());
            let total = world
                .encoded_current_size()?
                .checked_add(growth)
                .ok_or(INVALID)?;
            state.external_limit = total >= AGGREGATE_SOFT_BYTES;
        }
        if ratio > THERMAL_LIMIT && state.soft_limit() {
            state.sealed = true;
        }
        Ok((state, gained))
    }
    pub(super) fn initialize_legacy_origin(&mut self, world: &World) -> Result<(), SaveError> {
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
    pub fn solar_history_limited(&self) -> bool {
        self.solar.sealed || self.solar.soft_limit()
    }
}

#[cfg(test)]
mod tests;

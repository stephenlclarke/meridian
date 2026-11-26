//! Local flight admission comes from the launch and jump owners, independently
//! of ECM commands and their resource receipts.
use super::{EcmActivation, INVALID, World};
use crate::flight::{CombatIdentity, FlightMode, Snapshot, solar::Motion};
use crate::navigation::{JumpTiming, NavigationState, SystemId};
use crate::save::{MAX_SAVE_BYTES, Reader, SaveError, Writer};
use crate::traffic::{MissileScene, SceneStage};
use std::sync::Arc;

const MAX_VISITS: usize = MAX_SAVE_BYTES / 64;
const MAX_LAUNCH_STEPS: usize = MAX_SAVE_BYTES / 20;

#[derive(Debug, Clone, PartialEq)]
struct LaunchStep {
    motion: Motion,
    traffic: MissileScene,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Departure {
    destination: SystemId,
    tick: u64,
    clock: u64,
    spool: u32,
    before_jump: Option<u16>,
}
impl Departure {
    fn matches(self, timing: JumpTiming, owner: CombatIdentity) -> bool {
        timing.source == owner.system
            && timing.visit == owner.visit
            && timing.destination == self.destination
            && timing.departure_tick == self.tick
            && timing.departure_clock == self.clock
            && timing.spool == self.spool
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Closure {
    Dock {
        tick: u64,
        clock: u64,
        cancelled_jump: bool,
    },
    Jump {
        tick: u64,
        clock: u64,
    },
}
impl Closure {
    fn clock(self) -> u64 {
        match self {
            Self::Dock { clock, .. } | Self::Jump { clock, .. } => clock,
        }
    }
    fn tick(self) -> u64 {
        match self {
            Self::Dock { tick, .. } | Self::Jump { tick, .. } => tick,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
struct Visit {
    owner: CombatIdentity,
    item: u64,
    tick: u64,
    clock: u64,
    /// None is an actual hyperspace arrival from JumpJournal. Some starts at
    /// the immutable station launch seed, including all refused movement steps.
    launch: Option<Vec<LaunchStep>>,
    departure: Option<Departure>,
    closure: Option<Closure>,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Local {
    visits: Arc<Vec<Visit>>,
}
impl Local {
    pub(super) fn is_empty(&self) -> bool {
        self.visits.is_empty()
    }
    pub(super) fn unchanged_since(&self, old: &Self) -> bool {
        Arc::ptr_eq(&self.visits, &old.visits)
    }
    fn current_mut(&mut self, owner: CombatIdentity) -> Option<&mut Visit> {
        if self
            .visits
            .last()
            .is_none_or(|v| v.owner != owner || v.closure.is_some())
        {
            return None;
        }
        Arc::make_mut(&mut self.visits).last_mut()
    }
    fn insert(
        &mut self,
        owner: CombatIdentity,
        item: u64,
        tick: u64,
        clock: u64,
        launch: bool,
    ) -> Result<(), SaveError> {
        if self.visits.len() >= MAX_VISITS
            || self
                .visits
                .last()
                .is_some_and(|v| v.owner.visit >= owner.visit || v.closure.is_none())
        {
            return Err(INVALID);
        }
        Arc::make_mut(&mut self.visits).push(Visit {
            owner,
            item,
            tick,
            clock,
            launch: launch.then(Vec::new),
            departure: None,
            closure: None,
        });
        Ok(())
    }
    pub(super) fn begin_launch(
        &mut self,
        owner: CombatIdentity,
        item: u64,
        tick: u64,
        clock: u64,
    ) -> Result<(), SaveError> {
        self.insert(owner, item, tick, clock, true)
    }
    pub(super) fn begin_departure(
        &mut self,
        owner: CombatIdentity,
        destination: SystemId,
        tick: u64,
        clock: u64,
        spool: u32,
        before_jump: Option<u16>,
    ) -> Result<(), SaveError> {
        let visit = self.current_mut(owner).ok_or(INVALID)?;
        if visit.departure.is_some() {
            return Err(INVALID);
        }
        visit.departure = Some(Departure {
            destination,
            tick,
            clock,
            spool,
            before_jump,
        });
        Ok(())
    }
    pub(super) fn step(
        &mut self,
        world: &World,
        next: Snapshot,
        navigation: Option<NavigationState>,
        clock: u64,
        motion: Option<Motion>,
    ) -> Result<(), SaveError> {
        let Some(owner) = world.combat_identity() else {
            return Ok(());
        };
        let arrival = navigation
            .map(|n| n.snapshot(0))
            .filter(|n| n.visit != owner.visit);
        if world.state.mode != FlightMode::Launching
            && next.mode != FlightMode::Docked
            && arrival.is_none()
        {
            return Ok(());
        }
        let Some(visit) = self.current_mut(owner) else {
            return Ok(());
        };
        if world.state.mode == FlightMode::Launching {
            let steps = visit.launch.as_mut().ok_or(INVALID)?;
            if steps.len() >= MAX_LAUNCH_STEPS {
                return Err(SaveError::TooLarge);
            }
            steps.push(LaunchStep {
                motion: motion.ok_or(INVALID)?,
                traffic: world.traffic.missile_scene(
                    owner.system,
                    world.state.tick,
                    SceneStage::Command(world.combat.emissions),
                ),
            });
        }
        if next.mode == FlightMode::Docked {
            let cancelled_jump = world
                .navigation
                .is_some_and(|nav| nav.snapshot(0).travel.is_some());
            if cancelled_jump != visit.departure.is_some() {
                return Err(INVALID);
            }
            visit.closure = Some(Closure::Dock {
                tick: next.tick,
                clock,
                cancelled_jump,
            });
        } else if let Some(nav) = arrival {
            if visit.departure.is_none() {
                return Err(INVALID);
            }
            let item = visit.item;
            visit.closure = Some(Closure::Jump {
                tick: next.tick,
                clock,
            });
            self.insert(
                CombatIdentity {
                    system: nav.system,
                    visit: nav.visit,
                    ..owner
                },
                item,
                next.tick,
                clock,
                false,
            )?;
        }
        Ok(())
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.u16(self.visits.len() as u16);
        for v in self.visits.iter() {
            write_owner(w, v.owner);
            w.u64(v.item);
            w.u64(v.tick);
            w.u64(v.clock);
            w.option(v.launch.as_ref(), |w, steps| {
                w.u16(steps.len() as u16);
                for step in steps {
                    step.motion.encode(w);
                    step.traffic.encode(w);
                }
            });
            w.option(v.departure.as_ref(), |w, d| {
                write_system(w, d.destination);
                w.u64(d.tick);
                w.u64(d.clock);
                w.u32(d.spool);
                w.option(d.before_jump.as_ref(), |w, n| w.u16(*n));
            });
            w.option(v.closure.as_ref(), |w, c| match *c {
                Closure::Dock {
                    tick,
                    clock,
                    cancelled_jump,
                } => {
                    w.u8(0);
                    w.u64(tick);
                    w.u64(clock);
                    w.bool(cancelled_jump);
                }
                Closure::Jump { tick, clock } => {
                    w.u8(1);
                    w.u64(tick);
                    w.u64(clock);
                }
            });
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let count = usize::from(r.u16()?);
        if count > MAX_VISITS {
            return Err(INVALID);
        }
        let mut visits = Vec::with_capacity(count);
        for _ in 0..count {
            visits.push(Visit {
                owner: read_owner(r)?,
                item: r.u64()?,
                tick: r.u64()?,
                clock: r.u64()?,
                launch: r.option(|r| {
                    let n = usize::from(r.u16()?);
                    if n > MAX_LAUNCH_STEPS {
                        return Err(INVALID);
                    }
                    let mut steps = Vec::with_capacity(n);
                    for _ in 0..n {
                        steps.push(LaunchStep {
                            motion: Motion::decode(r)?,
                            traffic: MissileScene::decode(r)?,
                        });
                    }
                    Ok(steps)
                })?,
                departure: r.option(|r| {
                    Ok(Departure {
                        destination: read_system(r)?,
                        tick: r.u64()?,
                        clock: r.u64()?,
                        spool: r.u32()?,
                        before_jump: r.option(|r| r.u16())?,
                    })
                })?,
                closure: r.option(|r| {
                    let tag = r.u8()?;
                    let tick = r.u64()?;
                    let clock = r.u64()?;
                    match tag {
                        0 => Ok(Closure::Dock {
                            tick,
                            clock,
                            cancelled_jump: r.bool()?,
                        }),
                        1 => Ok(Closure::Jump { tick, clock }),
                        _ => Err(INVALID),
                    }
                })?,
            });
        }
        Ok(Self {
            visits: Arc::new(visits),
        })
    }

    pub(super) fn validate(
        &self,
        world: &World,
        modular: &crate::flight::modular::State,
        activations: &[EcmActivation],
    ) -> Result<(), SaveError> {
        let current = world.combat_identity().ok_or(INVALID)?;
        let nav = world.navigation.ok_or(INVALID)?;
        let end = world.combat.physical_tick();
        let mut cancelled_berths = world.damage.cancelled_ecm_berths();
        let mut launch_clocks = world.damage.ecm_launch_clocks();
        let mut previous: Option<&Visit> = None;
        for v in self.visits.iter() {
            if v.owner.owner != current.owner
                || v.owner.hull != current.hull
                || v.owner.visit < 2
                || v.owner.visit > current.visit
                || v.tick > world.state.tick
                || v.clock > end
                || v.clock > v.tick
                || v.tick - v.clock > world.state.tick.checked_sub(end).ok_or(INVALID)?
                || previous.is_some_and(|p| {
                    p.owner.visit >= v.owner.visit
                        || p.closure
                            .is_none_or(|c| c.clock() > v.clock || c.tick() > v.tick)
                        || (matches!(p.closure, Some(Closure::Jump { .. }))
                            && (p.owner.visit.checked_add(1) != Some(v.owner.visit)
                                || v.launch.is_some()))
                })
                || world.jump_journal.system_at_visit(v.owner.visit, v.tick)? != v.owner.system
                || modular
                    .ecm_item_at(current.owner, v.tick, v.owner.visit)
                    .is_none_or(|item| item.id != v.item)
            {
                return Err(INVALID);
            }
            let (jumps, arrival) = world.jump_journal.visit_origin(v.owner.visit);
            let launch = v
                .owner
                .visit
                .checked_sub(1)
                .and_then(|n| n.checked_sub(jumps))
                .ok_or(INVALID)?;
            if launch > nav.launches() {
                return Err(INVALID);
            }
            let berth = world.damage.berth_clock(launch).ok_or(INVALID)?;
            let next_berth = world
                .damage
                .berth_clock(launch.checked_add(1).ok_or(INVALID)?);
            let completed = world.jump_journal.completed_timing(v.owner.visit)?;
            if v.clock < berth {
                return Err(INVALID);
            }
            let local_start = match (&v.launch, arrival) {
                (Some(steps), None) if v.clock == berth => {
                    let local = self.replay_launch(world, v, steps)?;
                    if let Some(clock) = local
                        && launch_clocks.next() != Some(clock)
                    {
                        return Err(INVALID);
                    }
                    local
                }
                (None, Some(clock)) if v.clock == clock => {
                    let prior = world
                        .jump_journal
                        .completed_timing(v.owner.visit - 1)?
                        .ok_or(INVALID)?;
                    if prior.arrival_tick != v.tick || prior.arrival_clock != v.clock || prior.destination != v.owner.system
                        || previous.is_none_or(|p|p.owner.visit.checked_add(1)!=Some(v.owner.visit) || p.item != v.item
                            || !matches!(p.closure, Some(Closure::Jump{tick,clock}) if tick==v.tick && clock==v.clock)) { return Err(INVALID); }
                    Some(v.clock)
                }
                _ => return Err(INVALID),
            };
            let drive = modular
                .drive_at_visit(current.owner, v.tick, v.owner.visit)
                .ok_or(INVALID)?;
            let pending = if v.owner == current && world.state.mode != FlightMode::Docked {
                nav.pending_timing(world.state.tick, end, drive)?
            } else {
                None
            };
            match (completed, pending, next_berth, v.closure) {
                (Some(t), None, _, Some(Closure::Jump { tick, clock }))
                    if t.arrival_tick == tick
                        && t.arrival_clock == clock
                        && next_berth.is_none_or(|berth| clock < berth)
                        && v.departure.is_some_and(|d| d.matches(t, v.owner)) => {}
                (None, Some(t), None, None)
                    if v.departure.is_some_and(|d| d.matches(t, v.owner)) => {}
                (
                    None,
                    None,
                    Some(berth),
                    Some(Closure::Dock {
                        tick,
                        clock,
                        cancelled_jump,
                    }),
                ) if berth == clock
                    && cancelled_jump == v.departure.is_some()
                    && (v.owner != current
                        || nav.dock().is_some_and(|dock| dock.tick() == tick)) => {}
                (None, None, None, None)
                    if v.owner == current
                        && world.state.mode != FlightMode::Docked
                        && v.departure.is_none() => {}
                _ => return Err(INVALID),
            }
            if let Some(c) = v.closure
                && (c.clock() <= v.clock
                    || c.clock() > end
                    || v.tick.checked_add(c.clock() - v.clock) != Some(c.tick()))
            {
                return Err(INVALID);
            }
            if matches!(
                v.closure,
                Some(Closure::Dock {
                    cancelled_jump: true,
                    ..
                })
            ) && cancelled_berths.next()
                != Some((
                    launch.checked_add(1).ok_or(INVALID)?,
                    v.closure.ok_or(INVALID)?.clock(),
                ))
            {
                return Err(INVALID);
            }
            if v.closure.is_none() && v.tick.checked_add(end - v.clock) != Some(world.state.tick) {
                return Err(INVALID);
            }
            if let Some(d) = v.departure {
                let same_clock = activations
                    .iter()
                    .find(|a| {
                        a.owner == v.owner && a.physical_tick == d.clock && a.world_tick == d.tick
                    })
                    .map(|a| a.sequence);
                if local_start.is_none_or(|start| d.clock < start)
                    || d.clock > end
                    || v.tick
                        .checked_add(d.clock.checked_sub(v.clock).ok_or(INVALID)?)
                        != Some(d.tick)
                    || drive.spool_ticks() != d.spool
                    || d.destination == v.owner.system
                    || same_clock != d.before_jump
                {
                    return Err(INVALID);
                }
                let catalogue = world.navigation_catalogue.as_ref().ok_or(INVALID)?;
                if catalogue
                    .distance_ly(v.owner.system, d.destination)
                    .is_err()
                {
                    return Err(INVALID);
                }
                if let Some(Closure::Dock { clock, .. }) = v.closure
                    && (clock <= d.clock || clock - d.clock > u64::from(d.spool))
                {
                    return Err(INVALID);
                }
            }
            for a in activations.iter().filter(|a| a.owner == v.owner) {
                if local_start.is_none_or(|start| a.physical_tick < start)
                    || a.item != v.item
                    || v.tick
                        .checked_add(a.physical_tick.checked_sub(v.clock).ok_or(INVALID)?)
                        != Some(a.world_tick)
                    || v.closure.is_some_and(|c| a.physical_tick >= c.clock())
                    || v.departure.is_some_and(|d| {
                        a.physical_tick > d.clock
                            || (a.physical_tick == d.clock && d.before_jump != Some(a.sequence))
                    })
                {
                    return Err(INVALID);
                }
            }
            previous = Some(v);
        }
        if activations
            .iter()
            .any(|a| !self.visits.iter().any(|v| v.owner == a.owner))
        {
            return Err(INVALID);
        }
        if cancelled_berths.next().is_some()
            || launch_clocks.next().is_some()
            || previous.is_some_and(|p| matches!(p.closure, Some(Closure::Jump { .. })))
        {
            return Err(INVALID);
        }
        if world.state.mode != FlightMode::Docked
            && modular
                .ecm_item_at(current.owner, world.state.tick, current.visit)
                .is_some()
            && self
                .visits
                .last()
                .is_none_or(|v| v.owner != current || v.closure.is_some())
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn replay_launch(
        &self,
        world: &World,
        v: &Visit,
        steps: &[LaunchStep],
    ) -> Result<Option<u64>, SaveError> {
        let start = world
            .station_data
            .as_ref()
            .ok_or(INVALID)?
            .launch_start()
            .map_err(|_| INVALID)?;
        let mut frame = Snapshot {
            position: start.position,
            orientation: start.orientation,
            speed: start.speed,
            tick: v.tick,
            mode: FlightMode::Launching,
            motion_refusal: None,
            navigation: None,
            ..world.state
        };
        for step in steps {
            frame = crate::flight::solar::replay_launch_step(
                world,
                v.owner,
                frame,
                step.motion,
                &step.traffic,
            )?;
        }
        let clock = v.clock.checked_add(steps.len() as u64).ok_or(INVALID)?;
        let current = world.combat_identity().ok_or(INVALID)?;
        if clock > world.combat.physical_tick() || v.closure.is_some_and(|c| clock > c.clock()) {
            return Err(INVALID);
        }
        if v.owner == current
            && clock == world.combat.physical_tick()
            && (frame
                .position
                .into_iter()
                .zip(world.state.position)
                .any(|(a, b)| (a - b).abs() > 0.05)
                || frame
                    .orientation
                    .into_iter()
                    .zip(world.state.orientation)
                    .any(|(a, b)| (a - b).abs() > 0.00002)
                || (frame.speed - world.state.speed).abs() > 0.00002
                || frame.mode != world.state.mode
                || frame.motion_refusal != world.state.motion_refusal)
        {
            return Err(INVALID);
        }
        match frame.mode {
            FlightMode::Flight
                if v.owner != current || world.state.mode != FlightMode::Launching =>
            {
                Ok(Some(clock))
            }
            FlightMode::Launching
                if v.owner == current
                    && world.state.mode == FlightMode::Launching
                    && v.closure.is_none()
                    && clock == world.combat.physical_tick() =>
            {
                Ok(None)
            }
            FlightMode::Docked if matches!(v.closure,Some(Closure::Dock{clock:c,..}) if c==clock) => {
                Ok(None)
            }
            _ => Err(INVALID),
        }
    }
}
fn write_system(w: &mut Writer, system: SystemId) {
    w.u8(system.galaxy);
    w.u16(system.system);
}
fn read_system(r: &mut Reader<'_>) -> Result<SystemId, SaveError> {
    Ok(SystemId {
        galaxy: r.u8()?,
        system: r.u16()?,
    })
}
fn write_owner(w: &mut Writer, owner: CombatIdentity) {
    w.u64(owner.owner);
    w.u64(owner.hull);
    write_system(w, owner.system);
    w.u64(owner.visit);
}
fn read_owner(r: &mut Reader<'_>) -> Result<CombatIdentity, SaveError> {
    Ok(CombatIdentity {
        owner: r.u64()?,
        hull: r.u64()?,
        system: read_system(r)?,
        visit: r.u64()?,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::flight::missiles::tests::stop;
    use crate::flight::practice::tests::{aim_at, launch};
    use crate::{EcmError, EcmTrigger, FlightInput, ModuleAction, ModuleKind, XyzzyAction};

    fn fit(world: &mut World, action: ModuleAction) {
        let tick = world.state.tick;
        let review = world.review_module(action, tick).unwrap();
        world.prepare_module(review, tick).unwrap().commit();
    }
    fn fitted_world(seed: u64) -> World {
        let mut world = World::new_commander(seed).unwrap();
        let review = world.review_xyzzy(XyzzyAction::Enter, 0).unwrap();
        world.prepare_xyzzy(review, 0).unwrap().commit();
        let scoop = world.fitted_fuel_scoops().unwrap();
        fit(&mut world, ModuleAction::Store { item: scoop.item });
        let review = world.review_xyzzy(XyzzyAction::FitEcm, 0).unwrap();
        world.prepare_xyzzy(review, 0).unwrap().commit();
        world
    }
    fn cold(world: &World) -> World {
        World::from_save_bytes(&world.save_bytes().unwrap()).unwrap()
    }
    fn forge(world: &mut World, owner: CombatIdentity, item: u64, tick: u64, clock: u64) {
        Arc::make_mut(&mut world.ecm.activations).push(EcmActivation {
            sequence: 1,
            owner,
            item,
            world_tick: tick,
            physical_tick: clock,
            trigger: EcmTrigger::Manual,
        });
    }
    fn refused_with_balanced_resources(world: &World) {
        assert!(
            world
                .damage
                .validate_resources(world, world.navigation.unwrap().dock_sequence())
                .is_ok(),
            "isolated negative must still balance shared energy and shield pools"
        );
        assert!(
            world
                .ecm
                .local
                .validate(world, &world.modular, world.ecm.activations())
                .is_err()
        );
        let bytes = world.unchecked_test_save().unwrap();
        assert!(World::from_save_bytes(&bytes).is_err());
    }

    #[test]
    fn real_launch_admission_colds_and_stripped_or_forged_motion_cannot_admit_an_activation() {
        let mut world = fitted_world(18601);
        world.request_launch(0).unwrap();
        assert_eq!(world.state.mode, FlightMode::Launching);
        assert_eq!(world.request_ecm(0), Err(EcmError::FlightRequired));
        world = cold(&world); // Real pending launch, no field or accepted movement yet.
        world.step(FlightInput::default()).unwrap();
        assert_eq!(world.state.mode, FlightMode::Flight);
        // The separate physical launch marker must not consume the ordinary
        // command slot at that same clock.
        world.request_fire_front_laser(world.state.tick).unwrap();
        world.request_ecm(world.state.tick).unwrap();
        let mut stripped = cold(&world);
        Arc::make_mut(&mut stripped.ecm.local.visits)[0]
            .launch
            .as_mut()
            .unwrap()
            .clear();
        refused_with_balanced_resources(&stripped);
        let mut forged = cold(&world);
        let step = &mut Arc::make_mut(&mut forged.ecm.local.visits)[0]
            .launch
            .as_mut()
            .unwrap()[0];
        // A genuine opening scene had clear space. A forged traffic stop must
        // not turn it into an admitted historical Launching interval.
        step.motion = Motion::Stopped {
            input: FlightInput::default(),
            kind: crate::flight::solar::StopKind::Traffic,
        };
        refused_with_balanced_resources(&forged);
        let mut before = cold(&world);
        let a = &mut Arc::make_mut(&mut before.ecm.activations)[0];
        a.world_tick = 0;
        a.physical_tick = 0;
        before.pulse = crate::flight::damage::State::replayed_pulse_for_test(&before).unwrap();
        refused_with_balanced_resources(&before);
    }

    #[test]
    fn isolated_native_launch_obstacle_replay_keeps_mode_launching_after_a_refused_step() {
        // Actual traffic identity and station geometry. The historical obstacle
        // sample below is a component fixture, not an admitted saved flight.
        let mut world = World::new_commander(18604).unwrap();
        launch(&mut world);
        for _ in 0..600 {
            world
                .step(FlightInput {
                    throttle: 1.0,
                    ..FlightInput::default()
                })
                .unwrap();
        }
        let owner = world.combat_identity().unwrap();
        let data = world.station_data.as_ref().unwrap();
        let start = data.launch_start().unwrap();
        let opening_tick = world.state.tick - 1;
        let mut traffic = world.traffic.missile_scene(
            owner.system,
            opening_tick,
            SceneStage::Command(world.combat.emissions),
        );
        traffic.local_tick -= 1;
        let actor = traffic
            .bodies
            .iter_mut()
            .find(|body| body.id.serial() == 1)
            .unwrap();
        let limit = data.player_clearance_radius() * 2.0 + 1.0;
        actor.pose.position =
            crate::docking::Vec3::new(0.0, 0.0, f64::from(start.position[2]) + limit + 1.0);
        let opening = Snapshot {
            position: start.position,
            orientation: start.orientation,
            speed: start.speed,
            tick: opening_tick,
            mode: FlightMode::Launching,
            motion_refusal: None,
            ..world.state
        };
        let refused = Motion::Stopped {
            input: FlightInput::default(),
            kind: crate::flight::solar::StopKind::Traffic,
        };
        let after =
            crate::flight::solar::replay_launch_step(&world, owner, opening, refused, &traffic)
                .unwrap();
        assert_eq!(after.mode, FlightMode::Launching);
        assert_eq!(after.position, opening.position);
        assert_eq!(after.speed, 0.0);
        assert!(
            crate::flight::solar::replay_launch_step(
                &world,
                owner,
                opening,
                Motion::Active(FlightInput::default()),
                &traffic
            )
            .is_err()
        );
    }

    #[test]
    fn real_same_clock_before_departure_survives_pending_and_arrival_but_later_commands_do_not() {
        let mut world = fitted_world(18602);
        launch(&mut world);
        let tick = world.state.tick;
        let accepted = world.request_ecm(tick).unwrap();
        world.request_jump(SystemId::RENAS, tick).unwrap();
        assert_eq!(world.request_ecm(tick), Err(EcmError::FlightRequired));
        world = cold(&world);
        assert_eq!(
            world.ecm_snapshot().unwrap().last_activation,
            Some(accepted)
        );
        let remaining = world
            .snapshot()
            .navigation
            .unwrap()
            .travel
            .unwrap()
            .remaining_ticks;
        for _ in 0..remaining {
            world.step(FlightInput::default()).unwrap();
        }
        assert_eq!(world.combat_identity().unwrap().system, SystemId::RENAS);
        world = cold(&world);
        assert_eq!(world.ecm_snapshot().unwrap().active, None);

        let mut pending = fitted_world(18603);
        launch(&mut pending);
        let old = pending.combat_identity().unwrap();
        let item = pending.fitted_ecm().unwrap().id;
        let depart = pending.state.tick;
        let clock = pending.combat.physical_tick();
        pending.request_jump(SystemId::RENAS, depart).unwrap();
        assert_eq!(pending.request_ecm(depart), Err(EcmError::FlightRequired));
        let mut same = cold(&pending);
        forge(&mut same, old, item, depart, clock);
        same.pulse = same.pulse.prepare_ecm_discharge().unwrap();
        refused_with_balanced_resources(&same);
        pending.step(FlightInput::default()).unwrap();
        let mut during = cold(&pending);
        forge(&mut during, old, item, depart + 1, clock + 1);
        during.pulse = during.pulse.prepare_ecm_discharge().unwrap();
        refused_with_balanced_resources(&during);
        let remaining = pending
            .snapshot()
            .navigation
            .unwrap()
            .travel
            .unwrap()
            .remaining_ticks;
        for _ in 0..remaining {
            pending.step(FlightInput::default()).unwrap();
        }
        for _ in 0..120 {
            pending.step(FlightInput::default()).unwrap();
        }
        let mut historical = cold(&pending);
        forge(&mut historical, old, item, depart + 1, clock + 1);
        historical.pulse =
            crate::flight::damage::State::replayed_pulse_for_test(&historical).unwrap();
        refused_with_balanced_resources(&historical);
    }

    fn approach(world: &mut World, stop_z: f32) {
        for _ in 0..9000 {
            let before = world.snapshot();
            if before.mode == FlightMode::Docked || before.position[2] <= stop_z {
                return;
            }
            let speed = if before.position[2] > 2500.0 {
                34.3
            } else {
                4.0
            };
            world
                .step(FlightInput {
                    throttle: ((speed - before.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                    ..FlightInput::default()
                })
                .unwrap();
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        panic!("actual physical approach did not complete");
    }
    #[test]
    fn real_cancelled_jump_retains_independent_berth_witness_and_rejects_erased_spool() {
        let mut world = fitted_world(51);
        let items = world.modular_equipment_snapshot().unwrap().items;
        let basic = items
            .iter()
            .find(|i| i.kind == ModuleKind::Drive(crate::navigation::DriveGrade::Basic))
            .unwrap();
        let advanced = items
            .iter()
            .find(|i| i.kind == ModuleKind::Drive(crate::navigation::DriveGrade::Advanced))
            .unwrap();
        fit(
            &mut world,
            ModuleAction::Install {
                item: basic.id,
                replacing: Some(advanced.id),
            },
        );
        launch(&mut world);
        stop(&mut world);
        aim_at(&mut world, crate::docking::Vec3::new(0.0, 0.0, 0.0));
        approach(&mut world, 700.0);
        let depart = world.state.tick;
        let clock = world.combat.physical_tick();
        let owner = world.combat_identity().unwrap();
        let item = world.fitted_ecm().unwrap().id;
        world.request_jump(SystemId::RENAS, depart).unwrap();
        assert_eq!(world.request_ecm(depart), Err(EcmError::FlightRequired));
        approach(&mut world, 0.0);
        assert_eq!(world.state.mode, FlightMode::Docked);
        assert!(
            world.combat.physical_tick() > clock + 1 && world.combat.physical_tick() - clock < 600
        );
        assert_eq!(
            world.damage.cancelled_ecm_berths().collect::<Vec<_>>(),
            [(2, world.combat.physical_tick())]
        );
        let mut forged = cold(&world);
        forge(&mut forged, owner, item, depart + 1, clock + 1);
        refused_with_balanced_resources(&forged); // berth restores the same full pools.
        let mut erased = cold(&world);
        let visit = &mut Arc::make_mut(&mut erased.ecm.local.visits)[0];
        visit.departure = None;
        if let Some(Closure::Dock { cancelled_jump, .. }) = &mut visit.closure {
            *cancelled_jump = false;
        } else {
            panic!("physical berth receipt");
        }
        forge(&mut erased, owner, item, depart + 1, clock + 1);
        refused_with_balanced_resources(&erased); // independent berth proof still says cancelled jump.
    }
}

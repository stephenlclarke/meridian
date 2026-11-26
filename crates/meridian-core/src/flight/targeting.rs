//! Original L337 targeting-computer selection/disclosure (a11fe169, TargetingComputer.swift,
//! AutoTargetControl.swift and TargetTelemetry.swift). This owns only transient game selection;
//! it grants no weapon, docking, equipment, or other World authority.
use super::{CombatIdentity, FlightInput, FlightMode, FlightView, StepError, World};
use crate::docking::{RigidPose, Vec3};
use crate::equipment::{TargetingTier, WorkTarget, WorkTargetId, WorkTargetRole};
use crate::navigation::SystemId;
use crate::traffic::{NpcId, NpcPhase, NpcSnapshot, Population};
mod wreck_avoidance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetIdentity {
    Ship(NpcId),
    Station { system: SystemId, visit: u64 },
    Work(WorkTargetId),
}

impl TargetIdentity {
    pub fn label(self) -> String {
        match self {
            Self::Ship(id) => format!("Target #{}", id.serial()),
            Self::Station { .. } => "Target #0".into(),
            Self::Work(WorkTargetId::Asteroid(id)) => format!("Rock #{}", id.ordinal() + 1),
            Self::Work(WorkTargetId::Ore(id)) => {
                format!("Ore #{}-{}", id.asteroid().ordinal() + 1, id.fragment() + 1)
            }
            Self::Work(WorkTargetId::Wreck(id)) => format!("Wreck #{}", id.serial()),
        }
    }

    fn order(self) -> (u8, u8, u8) {
        match self {
            Self::Ship(id) => (0, id.serial(), 0),
            Self::Station { .. } => (1, 0, 0),
            Self::Work(WorkTargetId::Asteroid(id)) => (2, id.ordinal(), 0),
            Self::Work(WorkTargetId::Ore(id)) => (3, id.asteroid().ordinal(), id.fragment()),
            Self::Work(WorkTargetId::Wreck(id)) => (4, id.serial(), 0),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TargetFilter {
    #[default]
    Ships,
    Mining,
    Salvage,
    All,
}

impl TargetFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ships => "Ships",
            Self::Mining => "Mining",
            Self::Salvage => "Salvage",
            Self::All => "All",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Ships => Self::Mining,
            Self::Mining => Self::Salvage,
            Self::Salvage => Self::All,
            Self::All => Self::Ships,
        }
    }

    fn includes_work(self, role: WorkTargetRole) -> bool {
        match role {
            WorkTargetRole::RockHold | WorkTargetRole::OreTracking => {
                matches!(self, Self::Mining | Self::All)
            }
            WorkTargetRole::WreckTracking => matches!(self, Self::Salvage | Self::All),
        }
    }
}

/// Measured by the desktop's actual front-view mesh clipping path. World still resolves
/// every supplied ID against its own live, same-visit actors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetingViewport {
    pub size: [u32; 2],
    pub front: Vec<NpcId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetMode {
    Tracking,
    Station,
    Work(WorkTargetRole),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TargetingSnapshot {
    pub tracked_fragment: Option<crate::mining::OreId>,
    pub identity: TargetIdentity,
    pub tier: TargetingTier,
    pub filter: TargetFilter,
    pub mode: TargetMode,
    pub guidance: &'static str,
    pub pose: RigidPose,
    pub model: Option<&'static str>,
    pub status: Option<&'static str>,
    pub fields: Vec<(&'static str, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Candidate {
    Ship(NpcSnapshot),
    Station(TargetIdentity, crate::docking::StationPose),
    Work(WorkTarget),
}

impl Candidate {
    fn identity(self) -> TargetIdentity {
        match self {
            Self::Ship(actor) => TargetIdentity::Ship(actor.id),
            Self::Station(identity, _) => identity,
            Self::Work(work) => TargetIdentity::Work(work.id),
        }
    }

    fn position(self) -> Vec3 {
        match self {
            Self::Ship(actor) => actor.pose.position,
            Self::Station(_, pose) => pose.pose.position,
            Self::Work(work) => work.position,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub(super) struct State {
    selected: Option<TargetIdentity>,
    work_status: Option<&'static str>,
    wreck_avoidance: Option<wreck_avoidance::Steering>,
    origin: Option<CombatIdentity>,
    tier: Option<TargetingTier>,
    filter: TargetFilter,
    cycle_filter: TargetFilter,
    size: [u32; 2],
    membership: Vec<TargetIdentity>,
    remaining: Vec<TargetIdentity>,
    visited: Vec<TargetIdentity>,
}

impl State {
    fn cycle(
        &mut self,
        origin: CombatIdentity,
        tier: TargetingTier,
        size: [u32; 2],
        ordered: Vec<TargetIdentity>,
    ) -> Option<TargetIdentity> {
        if self.origin != Some(origin) {
            *self = Self {
                filter: self.filter,
                ..Self::default()
            };
            self.origin = Some(origin);
        }
        let mut membership = ordered.clone();
        membership.sort_by_key(|id| id.order());
        let changed = self.tier != Some(tier)
            || self.size != size
            || self.membership != membership
            || self.cycle_filter != self.filter;
        if changed || self.remaining.is_empty() {
            if !changed || ordered.iter().all(|id| self.visited.contains(id)) {
                self.visited.clear();
            }
            self.visited.retain(|id| membership.contains(id));
            self.remaining = ordered
                .into_iter()
                .filter(|id| !self.visited.contains(id))
                .collect();
        }
        self.tier = Some(tier);
        self.cycle_filter = self.filter;
        self.size = size;
        self.membership = membership;
        self.work_status = None;
        self.wreck_avoidance = None;
        self.selected = self.remaining.first().copied();
        if let Some(id) = self.selected {
            self.remaining.remove(0);
            self.visited.push(id);
        }
        self.selected
    }
}

fn is_live(actor: &NpcSnapshot) -> bool {
    actor.energy_hundredths > 0 && !matches!(actor.phase, NpcPhase::Complete | NpcPhase::Destroyed)
}

fn offset(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn grouped(candidate: Candidate, visible: &[NpcId], work_visible: &[WorkTargetId]) -> u8 {
    match candidate {
        Candidate::Ship(actor) if visible.contains(&actor.id) => 0,
        Candidate::Ship(_) => 1,
        Candidate::Station(..) => 2,
        Candidate::Work(work) if work_visible.contains(&work.id) => 0,
        Candidate::Work(_) => 1,
    }
}

fn target_bearing(player: RigidPose, position: Vec3) -> [f64; 2] {
    let local = player
        .orientation
        .inverse_coordinates(offset(position, player.position));
    [
        local.x.atan2(local.z),
        local.y.atan2(local.x.hypot(local.z)),
    ]
}

impl World {
    fn targeting_origin(&self) -> Option<CombatIdentity> {
        let snapshot = self.snapshot();
        if snapshot.mode != FlightMode::Flight
            || snapshot.navigation?.travel.is_some()
            || self.emp.electrical_disabled()
            || self
                .combat_snapshot()
                .is_some_and(|combat| combat.player.destroyed)
        {
            return None;
        }
        self.combat_identity()
    }

    fn targeting_candidates(&self, origin: CombatIdentity, tier: TargetingTier) -> Vec<Candidate> {
        let mut ships = self
            .traffic_snapshot()
            .map_or_else(Vec::new, |traffic| traffic.actors);
        if let Some(mule) = self
            .mule_actor_snapshot()
            .filter(|mule| mule.is_renas_encounter())
        {
            ships.push(*mule.actor());
        }
        let mut candidates: Vec<_> = ships
            .into_iter()
            .filter(|actor| {
                is_live(actor)
                    && actor.id.profile() == origin.owner
                    && actor.id.system() == origin.system
            })
            .map(Candidate::Ship)
            .collect();
        if tier >= TargetingTier::Docking
            && let Some(station) = self.state.station
        {
            candidates.push(Candidate::Station(
                TargetIdentity::Station {
                    system: origin.system,
                    visit: origin.visit,
                },
                station,
            ));
        }
        candidates
    }

    pub fn cycle_targeting(
        &mut self,
        expected_tick: u64,
        viewport: TargetingViewport,
    ) -> Result<bool, StepError> {
        self.cycle_targeting_with_work(expected_tick, viewport, &[])
    }

    pub fn cycle_targeting_with_work(
        &mut self,
        expected_tick: u64,
        viewport: TargetingViewport,
        work_visible: &[WorkTargetId],
    ) -> Result<bool, StepError> {
        if expected_tick != self.state.tick {
            return Err(StepError::StaleCommand);
        }
        if self.is_paused() {
            return Err(StepError::Paused);
        }
        let origin = self.targeting_origin().ok_or(StepError::InvalidMode)?;
        let tier = self.targeting_tier().ok_or(StepError::InvalidMode)?;
        if viewport.size.contains(&0) {
            return Err(StepError::InvalidMode);
        }
        let player = super::flight_pose(self.state.position, self.state.orientation)
            .ok_or(StepError::NonFiniteState)?;
        let filter = self.targeting.filter;
        let mut candidates = if matches!(filter, TargetFilter::Ships | TargetFilter::All) {
            let mut ships = self.targeting_candidates(origin, tier);
            ships.retain(|candidate| {
                tier >= TargetingTier::Advanced
                    || grouped(*candidate, &viewport.front, work_visible) == 0
            });
            ships
        } else {
            Vec::new()
        };
        if filter != TargetFilter::Ships {
            candidates.extend(
                self.work_targets()
                    .into_iter()
                    .filter(|work| filter.includes_work(work.role))
                    .filter(|work| !matches!(work.id, WorkTargetId::Wreck(id)
                        if filter == TargetFilter::All && candidates.iter().any(|c| matches!(c, Candidate::Ship(ship) if ship.id == id))))
                    .map(Candidate::Work)
                    .collect::<Vec<_>>(),
            );
        }
        candidates.sort_by(|a, b| {
            let distance = |candidate: Candidate| {
                let delta = offset(candidate.position(), player.position);
                delta.dot(delta)
            };
            grouped(*a, &viewport.front, work_visible)
                .cmp(&grouped(*b, &viewport.front, work_visible))
                .then_with(|| distance(*a).total_cmp(&distance(*b)))
                .then_with(|| a.identity().order().cmp(&b.identity().order()))
        });
        let ordered = candidates
            .iter()
            .copied()
            .map(Candidate::identity)
            .collect();
        let station_was_selected = matches!(
            self.targeting.selected,
            Some(TargetIdentity::Station { .. })
        );
        let selected = self.targeting.cycle(origin, tier, viewport.size, ordered);
        if (station_was_selected || selected.is_some())
            && !matches!(selected, Some(TargetIdentity::Station { .. }))
        {
            self.auto_docking = None;
        }
        self.targeting.work_status = selected
            .and_then(|id| {
                candidates
                    .iter()
                    .find(|candidate| candidate.identity() == id)
            })
            .and_then(|candidate| self.automatic_work_target(*candidate))
            .map(|_| "Approaching work target");
        // Selection is a cockpit command, not an automatic launch or weapon command.
        if selected.is_some() {
            self.request_flight_view(FlightView::Front, expected_tick)
                .map_err(|_| StepError::InvalidMode)?;
        }
        Ok(selected.is_some())
    }

    pub fn cancel_targeting(&mut self) {
        if matches!(
            self.targeting.selected,
            Some(TargetIdentity::Station { .. })
        ) {
            self.auto_docking = None;
        }
        self.targeting.work_status = None;
        self.targeting.selected = None;
        self.targeting.wreck_avoidance = None;
    }

    pub fn target_filter(&self) -> TargetFilter {
        self.targeting.filter
    }

    pub fn cycle_target_filter(&mut self) -> Result<TargetFilter, StepError> {
        if self.is_paused() {
            return Err(StepError::Paused);
        }
        if self.targeting_origin().is_none() {
            return Err(StepError::InvalidMode);
        }
        self.cancel_targeting();
        self.targeting.filter = self.targeting.filter.next();
        Ok(self.targeting.filter)
    }

    fn current_target(&self) -> Option<(TargetingTier, Candidate)> {
        let selected = self.targeting.selected?;
        let origin = self.targeting_origin()?;
        if self.targeting.origin != Some(origin) {
            return None;
        }
        let tier = self.targeting_tier()?;
        if let TargetIdentity::Work(id) = selected {
            return self
                .work_targets()
                .into_iter()
                .find(|work| work.id == id && self.targeting.filter.includes_work(work.role))
                .map(|work| (tier, Candidate::Work(work)));
        }
        self.targeting_candidates(origin, tier)
            .into_iter()
            .find(|c| c.identity() == selected)
            .map(|c| (tier, c))
    }

    pub fn targeting_station_selected(&self) -> bool {
        matches!(self.current_target(), Some((_, Candidate::Station(..))))
    }

    pub fn selected_work_target(&self) -> Option<WorkTarget> {
        match self.current_target()? {
            (_, Candidate::Work(work)) => Some(work),
            _ => None,
        }
    }

    fn automatic_work_target(&self, candidate: Candidate) -> Option<WorkTarget> {
        match candidate {
            Candidate::Work(work) if work.role == WorkTargetRole::RockHold => {
                if let WorkTargetId::Asteroid(id) = work.id
                    && let Some(fragment) = self.mining_collection_target(id)
                {
                    return Some(WorkTarget {
                        id: WorkTargetId::Ore(fragment.id),
                        role: WorkTargetRole::OreTracking,
                        position: fragment.position,
                    });
                }
                Some(work)
            }
            Candidate::Work(work) => Some(work),
            Candidate::Ship(ship)
                if ship.electrical_disabled && self.work_targeting_fit().salvage =>
            {
                Some(WorkTarget {
                    id: WorkTargetId::Wreck(ship.id),
                    role: WorkTargetRole::WreckTracking,
                    position: ship.pose.position,
                })
            }
            _ => None,
        }
    }

    pub fn selected_disabled_salvage_target(&self) -> Option<NpcId> {
        let candidate = self.current_target()?.1;
        let WorkTargetId::Wreck(id) = self.automatic_work_target(candidate)?.id else {
            return None;
        };
        self.traffic_snapshot()?
            .actors
            .into_iter()
            .find(|actor| actor.id == id && actor.electrical_disabled && is_live(actor))
            .map(|actor| actor.id)
    }

    pub fn automatic_work_status(&self) -> Option<&'static str> {
        self.targeting.work_status
    }

    pub(super) fn advance_selected_work(&mut self) {
        let selected = self.current_target().map(|(_, candidate)| candidate);
        let work = selected.and_then(|candidate| self.automatic_work_target(candidate));
        let wreck = work.and_then(|target| match target.id {
            WorkTargetId::Wreck(id) => Some(id),
            _ => None,
        });
        if self
            .modular
            .salvage_exchange_source()
            .is_some_and(|id| Some(id) != wreck)
            && self.restore_auto_salvage_tool().is_err()
        {
            self.targeting.work_status = Some("Cutter return unavailable");
            return;
        }
        let mining_lock = matches!(
            selected.map(Candidate::identity),
            Some(TargetIdentity::Work(WorkTargetId::Asteroid(_)))
        );
        if let Some(WorkTargetId::Ore(ore)) = work.map(|target| target.id) {
            // Pursuing a released fragment keeps the original asteroid lock and tool.
            // The intake itself depends on the scoops, including for a direct ore lock.
            let status = self.auto_scoop_mined_ore_tick(ore.asteroid());
            let completed = self.current_target().is_none();
            if completed {
                self.cancel_targeting();
            } else {
                self.targeting.work_status = Some(status);
            }
            if (!mining_lock || completed) && self.restore_auto_mining_tool().is_err() {
                self.targeting.work_status = Some("Tool change unavailable");
            }
            return;
        }
        if !mining_lock {
            if self.restore_auto_mining_tool().is_err() {
                self.targeting.work_status = Some("Tool change unavailable");
                return;
            }
        } else if let Err(err) = self.select_auto_mining_tool() {
            self.targeting.work_status = Some(match err {
                crate::mining::MiningError::ProtectedActivity => "Protected activity",
                crate::mining::MiningError::NoCompatibleFittedTool => "Fit mining laser",
                _ => "Tool change unavailable",
            });
            return;
        }
        if let Some(id) = wreck
            && let Err(error) = self.select_auto_salvage_tool(id)
        {
            self.targeting.work_status = Some(match error {
                super::ModuleError::MissingItem => "Salvage cutter required",
                super::ModuleError::Capacity => "No free cutter mount",
                super::ModuleError::MissingPrerequisite if self.fitted_fuel_scoops().is_none() => {
                    "Scoops not fitted"
                }
                _ => "Cutter unavailable",
            });
            return;
        }
        let Some(work) = work else {
            self.targeting.work_status = None;
            return;
        };
        if self.targeting.wreck_avoidance.is_some() {
            self.targeting.work_status = Some("Braking for clearance");
            return;
        }
        let Some(standoff) = self.work_standoff(work) else {
            return;
        };
        let Some(pose) = super::flight_pose(self.state.position, self.state.orientation) else {
            return;
        };
        let vector = offset(work.position, pose.position);
        let range = vector.dot(vector).sqrt();
        // Recheck the accepted pose and live target after both have moved this tick.
        let at_work_speed = self
            .moving_work_course(work, pose)
            .map_or(self.state.speed <= 0.1, |(_, _, relative_speed)| {
                relative_speed <= 1.0
            });
        let holding = (range - standoff).abs() <= 0.5
            && at_work_speed
            && vector.dot(pose.orientation.z) >= range * 0.999;
        self.targeting.work_status = Some(if !holding {
            "Approaching work target"
        } else {
            match work.id {
                WorkTargetId::Asteroid(id) => self.auto_mine_tick(id),
                WorkTargetId::Wreck(id) if self.selected_disabled_salvage_target() == Some(id) => {
                    if self.fitted_salvage_cutter().is_none() {
                        "Cutter not fitted"
                    } else {
                        "Occupied: confirm cut"
                    }
                }
                WorkTargetId::Wreck(id) => self.auto_salvage_tick(id),
                WorkTargetId::Ore(_) => return,
            }
        });
    }

    /// One selected live actor, not an owned missile lock. Manual steering releases tracking;
    /// station guidance is provided separately by the native docking controller.
    pub fn targeting_flight_input(&mut self, input: FlightInput) -> FlightInput {
        if self.targeting.selected.is_none() {
            return input;
        }
        let Some((_, target)) = self.current_target() else {
            self.cancel_targeting();
            return input;
        };
        let work = self.automatic_work_target(target);
        if input.pitch != 0.0
            || input.yaw != 0.0
            || input.roll != 0.0
            || (work.is_some() || matches!(target, Candidate::Station(..) | Candidate::Work(_)))
                && (input.throttle != 0.0 || input.brake)
        {
            self.cancel_targeting();
            return input;
        }
        if matches!(target, Candidate::Station(..)) {
            return input;
        }
        let Some(player) = super::flight_pose(self.state.position, self.state.orientation) else {
            return input;
        };
        let mut aim = work.map_or_else(|| target.position(), |work| work.position);
        let collecting = work.is_some_and(|w| matches!(w.id, WorkTargetId::Ore(_)));
        if !collecting && let Some(standoff) = work.and_then(|w| self.work_standoff(w)) {
            let delta = offset(aim, player.position);
            if delta.dot(delta).sqrt() < standoff - 0.25 {
                // Only forward thrust is available. Face away to recover the work distance,
                // then turn back towards the target for cutting or mining.
                aim = offset(player.position, delta);
            }
        }
        if let Some((point, _, _)) = work.and_then(|target| self.moving_work_course(target, player))
        {
            aim = point;
        }
        let [bearing, mut elevation] = target_bearing(player, aim);
        if collecting {
            let range = offset(aim, player.position);
            if work
                .and_then(|target| self.work_standoff(target))
                .is_some_and(|distance| range.dot(range).sqrt() <= distance + 1.0)
            {
                // Turn above the fragment at the working distance, once its entire
                // mesh can enter the lower-forward aperture.
                elevation += 45.0_f64.atan2(55.0);
            }
        }
        let tolerance = 1.5 / (f64::from(self.targeting.size[1].max(1)) * 0.8);
        let control = |angle: f64, rate: f32| {
            if angle.abs() <= tolerance {
                0.0
            } else {
                (angle / f64::from(rate * super::FIXED_DELTA_SECONDS)).clamp(-1.0, 1.0) as f32
            }
        };
        let yaw = control(bearing, super::YAW_RATE);
        let pitch = control(elevation, super::PITCH_RATE);
        let output = FlightInput {
            yaw,
            pitch,
            ..input
        };
        if let Some(work) = work {
            let desired = self.work_hold_input(work, output);
            if let WorkTargetId::Wreck(id) = work.id
                && self.auto_salvage_eligible(id)
                && let Some(standoff) = self.work_standoff(work)
            {
                return wreck_avoidance::protect(
                    &mut self.targeting.wreck_avoidance,
                    self.state,
                    work.position,
                    standoff,
                    desired,
                );
            }
            self.targeting.wreck_avoidance = None;
            desired
        } else {
            self.targeting.wreck_avoidance = None;
            output
        }
    }

    pub fn targeting_snapshot(&self) -> Option<TargetingSnapshot> {
        let (tier, candidate) = self.current_target()?;
        let identity = candidate.identity();
        let displayed_work = match candidate {
            Candidate::Work(work) => Some(self.automatic_work_target(candidate).unwrap_or(work)),
            _ => None,
        };
        let (mode, guidance, pose, model, status, fields) = match candidate {
            Candidate::Ship(actor) => {
                let identified = tier >= TargetingTier::Identification;
                let model = identified.then_some(match actor.hull_id {
                    "SH025" => "Peregrine Mk V",
                    "SH001" => "Mule",
                    _ => "Unidentified craft",
                });
                let status = if actor.electrical_disabled {
                    Some("EMP disabled")
                } else if identified {
                    Some(if actor.population == Population::Pirate {
                        "Hostile"
                    } else {
                        "Neutral"
                    })
                } else {
                    None
                };
                let local_system = self
                    .navigation_catalogue
                    .as_ref()
                    .and_then(|catalogue| catalogue.system(actor.id.system()).ok())
                    .map(|system| system.name);
                let player = super::flight_pose(self.state.position, self.state.orientation)?;
                let tolerance = 1.5 / (f64::from(self.targeting.size[1].max(1)) * 0.8);
                let guidance = if target_bearing(player, actor.pose.position)
                    .into_iter()
                    .all(|angle| angle.abs() <= tolerance)
                {
                    "Tracking"
                } else {
                    "Facing target"
                };
                (
                    TargetMode::Tracking,
                    guidance,
                    actor.pose,
                    model,
                    status,
                    ship_fields(actor, tier, local_system),
                )
            }
            Candidate::Station(_, pose) => {
                let detailed = tier >= TargetingTier::Identification;
                let player = super::flight_pose(self.state.position, self.state.orientation)?;
                let delta = offset(pose.pose.position, player.position);
                let status = self.auto_docking_status().unwrap_or("Aligning");
                let fields = if detailed {
                    vec![
                        (
                            "Entrance",
                            if status == "Ready" {
                                "Ready"
                            } else {
                                "Aligning"
                            }
                            .into(),
                        ),
                        (
                            "Rotation",
                            format!("{:.1} deg/s", pose.rotation_rate.to_degrees()),
                        ),
                        ("Range", format!("{:.0} units", delta.dot(delta).sqrt())),
                    ]
                } else {
                    Vec::new()
                };
                (
                    TargetMode::Station,
                    status,
                    pose.model_pose().ok()?,
                    detailed.then_some("Space station"),
                    detailed.then_some(status),
                    fields,
                )
            }
            Candidate::Work(selected) => {
                let work = displayed_work.unwrap_or(selected);
                let (model, guidance) = match work.role {
                    WorkTargetRole::RockHold => ("Asteroid", "Rock guidance"),
                    WorkTargetRole::OreTracking => ("Ore", "Ore tracking"),
                    WorkTargetRole::WreckTracking => ("Wreck", "Wreck tracking"),
                };
                let mut pose = RigidPose {
                    position: work.position,
                    orientation: crate::docking::Basis3::IDENTITY,
                };
                if let WorkTargetId::Wreck(id) = work.id
                    && let Some(actor) = self
                        .traffic_snapshot()
                        .and_then(|traffic| traffic.actors.into_iter().find(|actor| actor.id == id))
                {
                    pose = actor.pose;
                }
                let identified = tier >= TargetingTier::Identification;
                (
                    TargetMode::Work(work.role),
                    guidance,
                    pose,
                    identified.then_some(model),
                    None,
                    Vec::new(),
                )
            }
        };
        Some(TargetingSnapshot {
            tracked_fragment: displayed_work.and_then(|work| match work.id {
                WorkTargetId::Ore(id) => Some(id),
                _ => None,
            }),
            identity,
            tier,
            filter: self.targeting.filter,
            mode,
            guidance,
            pose,
            model,
            status,
            fields,
        })
    }

    /// Explicit T command: copy an eligible selected ship into the existing in-game lock.
    /// Existing missile admission retains full authority; this never arms or discharges.
    pub fn request_selected_target_missile_lock(
        &mut self,
        tick: u64,
    ) -> Result<NpcId, super::MissileError> {
        if tick != self.state.tick {
            return Err(super::MissileError::StaleCommand);
        }
        let Some((_, Candidate::Ship(actor))) = self.current_target() else {
            return Err(super::MissileError::NoLiveLock);
        };
        self.request_npc_missile_lock(Some(actor.id), tick)?;
        Ok(actor.id)
    }
}

fn ship_fields(
    actor: NpcSnapshot,
    tier: TargetingTier,
    local_system: Option<&str>,
) -> Vec<(&'static str, String)> {
    let mut fields = Vec::new();
    if tier >= TargetingTier::Identification {
        fields.push((
            "Legal status",
            if actor.wanted == 0 {
                "Clean".into()
            } else {
                format!("Wanted ({})", actor.wanted)
            },
        ));
    }
    if tier >= TargetingTier::Pirate {
        let cargo = actor.cargo.and_then(|(id, quantity)| {
            id.definition()
                .ok()
                .map(|item| format!("{} {} {}", item.name, quantity, item.unit.symbol()))
        });
        let cargo = cargo.unwrap_or_else(|| {
            if actor.hull_id == "SH025" && actor.population == Population::Pirate {
                "Empty".into()
            } else {
                "Unknown".into()
            }
        });
        let route = if actor.hull_id == "SH025" && actor.population == Population::Trader {
            local_system.map(|name| format!("{name} (local/inferred)"))
        } else {
            None
        };
        fields.extend([
            ("Cargo", cargo),
            ("Origin", route.clone().unwrap_or_else(|| "Unknown".into())),
            ("Destination", route.unwrap_or_else(|| "Unknown".into())),
        ]);
    }
    if tier >= TargetingTier::Military {
        fields.extend([
            (
                "Front shield",
                format!("{:.2}", f64::from(actor.front_shield_hundredths) / 100.0),
            ),
            (
                "Rear shield",
                format!("{:.2}", f64::from(actor.rear_shield_hundredths) / 100.0),
            ),
            (
                "Laser heat",
                format!("{:.1}", f64::from(actor.heat_fifths) / 5.0),
            ),
            ("Fuel", "Unknown".into()),
            ("Missiles", "Unknown".into()),
            ("ECM equipped", "Unknown".into()),
            ("ECM active", "Unknown".into()),
        ]);
    }
    fields
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::flight::{
        missiles::tests::{fly_axis, stop},
        practice::tests::{consent, launch},
    };
    use crate::{MissileError, ModuleAction, ModuleKind, XyzzyAction};

    fn viewport(front: Vec<NpcId>) -> TargetingViewport {
        TargetingViewport {
            size: [1280, 720],
            front,
        }
    }

    fn module(world: &mut World, action: ModuleAction) {
        let tick = world.snapshot().tick;
        let review = world.review_module(action, tick).unwrap();
        world.prepare_module(review, tick).unwrap().commit();
    }

    fn fitted(tier: TargetingTier) -> World {
        let mut world = World::new_commander(12005).unwrap();
        let review = world.review_xyzzy(XyzzyAction::Enter, 0).unwrap();
        world.prepare_xyzzy(review, 0).unwrap().commit();
        if tier != TargetingTier::Military {
            let military = world
                .modular_equipment_snapshot()
                .unwrap()
                .items
                .into_iter()
                .find(|item| item.kind == ModuleKind::Targeting(TargetingTier::Military))
                .unwrap()
                .id;
            module(&mut world, ModuleAction::Store { item: military });
            for grade in TargetingTier::ALL
                .into_iter()
                .take_while(|grade| *grade <= tier)
            {
                module(
                    &mut world,
                    ModuleAction::Purchase {
                        kind: ModuleKind::Targeting(grade),
                    },
                );
            }
        }
        assert_eq!(world.targeting_tier(), Some(tier));
        world
    }

    fn actual_ship(world: &mut World) -> NpcSnapshot {
        launch(world);
        stop(world);
        fly_axis(world, 0, -600.0, 10.0);
        for _ in 0..1000 {
            if let Some(actor) = world
                .traffic_snapshot()
                .unwrap()
                .actors
                .into_iter()
                .find(|a| a.population == Population::Trader && is_live(a))
            {
                return actor;
            }
            world.step(FlightInput::default()).unwrap();
        }
        panic!("real local trader did not appear");
    }

    #[test]
    fn basic_uses_measured_front_only_and_keeps_the_live_identity_after_view_change() {
        let mut world = fitted(TargetingTier::Basic);
        let actor = actual_ship(&mut world);
        let tick = world.snapshot().tick;
        world.request_flight_view(FlightView::Rear, tick).unwrap();
        assert!(!world.cycle_targeting(tick, viewport(vec![])).unwrap());
        assert!(
            world
                .cycle_targeting(tick, viewport(vec![actor.id]))
                .unwrap()
        );
        assert_eq!(world.flight_view(), FlightView::Front);
        let target = world.targeting_snapshot().unwrap();
        assert_eq!(target.identity, TargetIdentity::Ship(actor.id));
        assert!(target.model.is_none() && target.fields.is_empty());
        world.request_flight_view(FlightView::Left, tick).unwrap();
        assert_eq!(
            world.targeting_snapshot().unwrap().identity,
            target.identity
        );
        let manual = FlightInput {
            yaw: 0.5,
            ..FlightInput::default()
        };
        assert_eq!(world.targeting_flight_input(manual), manual);
        assert!(world.targeting_snapshot().is_none());
    }

    #[test]
    fn advanced_cycles_offscreen_without_station_and_military_discloses_only_actual_data() {
        let mut world = fitted(TargetingTier::Advanced);
        let actor = actual_ship(&mut world);
        let tick = world.snapshot().tick;
        assert!(world.cycle_targeting(tick, viewport(vec![])).unwrap());
        assert_eq!(
            world.targeting_snapshot().unwrap().identity,
            TargetIdentity::Ship(actor.id)
        );
        assert!(!world.targeting_station_selected());
        assert!(ship_fields(actor, TargetingTier::Docking, Some("Avenor")).is_empty());
        assert_eq!(
            ship_fields(actor, TargetingTier::Identification, Some("Avenor")).len(),
            1
        );
        let pirate = ship_fields(actor, TargetingTier::Pirate, Some("Avenor"));
        assert_eq!(pirate.len(), 4);
        assert!(
            pirate
                .iter()
                .any(|(key, value)| *key == "Origin" && value == "Avenor (local/inferred)")
        );
        let military = ship_fields(actor, TargetingTier::Military, Some("Avenor"));
        assert!(military.iter().any(|(key, value)| *key == "Front shield"
            && *value == format!("{:.2}", f64::from(actor.front_shield_hundredths) / 100.0)));
        assert!(
            military
                .iter()
                .any(|(key, value)| *key == "Fuel" && value == "Unknown")
        );
    }

    #[test]
    fn explicit_selected_missile_lock_respects_admission_and_never_arms_or_discharges() {
        let mut world = fitted(TargetingTier::Military);
        let actor = actual_ship(&mut world);
        let tick = world.snapshot().tick;
        let before = world.missile_snapshot().unwrap();
        assert_eq!(
            world.request_selected_target_missile_lock(tick),
            Err(MissileError::NoLiveLock)
        );
        assert!(
            world
                .cycle_targeting(tick, viewport(vec![actor.id]))
                .unwrap()
        );
        assert_eq!(world.missile_snapshot().unwrap(), before);
        assert_eq!(
            world.request_selected_target_missile_lock(tick),
            Ok(actor.id)
        );
        let mut expected = before;
        expected.npc_lock = Some(actor.id);
        assert_eq!(world.missile_snapshot().unwrap(), expected);

        // Compose an actual active practice fixture at the command boundary to verify that
        // this wrapper propagates the existing protected-service veto for a live actor.
        let mut protected = World::new_commander(12005).unwrap();
        consent(&mut protected);
        launch(&mut protected);
        world.practice = protected.practice;
        assert_eq!(
            world.request_selected_target_missile_lock(tick),
            Err(MissileError::ProtectedTraffic)
        );
        assert_eq!(world.missile_snapshot().unwrap(), expected);
        world.practice = None;

        // With the actual ship visited, station follows all ship groups.
        for _ in 0..5 {
            if world.targeting_station_selected() {
                break;
            }
            assert!(
                world
                    .cycle_targeting(tick, viewport(vec![actor.id]))
                    .unwrap()
            );
        }
        assert!(world.targeting_station_selected());
        assert_eq!(
            world.request_selected_target_missile_lock(tick),
            Err(MissileError::NoLiveLock)
        );
        assert_eq!(world.missile_snapshot().unwrap(), expected);
        world.cancel_targeting();
        assert!(world.targeting_snapshot().is_none());
        assert!(!world.auto_docking_active());
    }

    #[test]
    fn no_identity_fast_path_and_transient_selection_is_not_restored() {
        let mut empty = World::new();
        let input = FlightInput {
            throttle: 0.5,
            ..FlightInput::default()
        };
        assert_eq!(empty.targeting_flight_input(input), input);
        assert!(empty.targeting_snapshot().is_none());
        let mut world = fitted(TargetingTier::Docking);
        launch(&mut world);
        let tick = world.snapshot().tick;
        assert!(world.cycle_targeting(tick, viewport(vec![])).unwrap());
        assert!(world.targeting_station_selected());
        let restored = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
        assert!(restored.targeting_snapshot().is_none());
        world.step(FlightInput::default()).unwrap();
        assert!(world.auto_docking_active());
        world.cancel_targeting();
        assert!(!world.auto_docking_active());
    }

    #[test]
    fn selected_asteroid_starts_approach_and_manual_brake_releases_it() {
        let mut world = fitted(TargetingTier::Military);
        launch(&mut world);
        world.cycle_target_filter().unwrap();
        let tick = world.snapshot().tick;
        assert!(world.cycle_targeting(tick, viewport(vec![])).unwrap());
        let rock = world.selected_work_target().unwrap();
        // Start on a clear exterior ray so this tests guidance, not station clearance.
        world.state.position = [
            rock.position.x as f32,
            rock.position.y as f32,
            rock.position.z as f32 - 300.0,
        ];
        world.state.orientation = [0.0, 0.0, 0.0, 1.0];
        world.state.speed = 0.0;
        world.step(FlightInput::default()).unwrap();
        assert_eq!(
            world.automatic_work_status(),
            Some("Approaching work target")
        );
        assert!(world.state.speed > 0.0);
        world
            .step(FlightInput {
                brake: true,
                ..FlightInput::default()
            })
            .unwrap();
        assert!(world.automatic_work_status().is_none());
        assert!(world.selected_work_target().is_none());
    }

    #[test]
    fn work_filter_cycles_real_rock_ids_and_preserves_stable_remaining_order() {
        let mut world = fitted(TargetingTier::Military);
        launch(&mut world);
        let tick = world.snapshot().tick;
        assert_eq!(world.target_filter(), TargetFilter::Ships);
        assert_eq!(world.cycle_target_filter().unwrap(), TargetFilter::Mining);
        let rocks = world.work_targets();
        let last = rocks.last().unwrap().id;
        assert!(matches!(last, WorkTargetId::Asteroid(_)));
        let prior = world.save_bytes().unwrap();
        assert!(
            world
                .cycle_targeting_with_work(tick, viewport(vec![]), &[last])
                .unwrap()
        );
        assert_eq!(world.selected_work_target().unwrap().id, last);
        assert_eq!(
            world.targeting_snapshot().unwrap().mode,
            TargetMode::Work(WorkTargetRole::RockHold)
        );
        assert_eq!(
            world.request_selected_target_missile_lock(tick),
            Err(MissileError::NoLiveLock)
        );
        assert_eq!(world.save_bytes().unwrap(), prior);
        let mut visited = vec![last];
        for _ in 1..rocks.len() {
            assert!(
                world
                    .cycle_targeting_with_work(tick, viewport(vec![]), &[])
                    .unwrap()
            );
            let selected = world.selected_work_target().unwrap().id;
            assert!(!visited.contains(&selected));
            visited.push(selected);
        }
        assert_eq!(visited.len(), rocks.len());
        let manual = FlightInput {
            throttle: 0.5,
            ..FlightInput::default()
        };
        assert_eq!(world.targeting_flight_input(manual), manual);
        assert!(world.selected_work_target().is_none());
        assert_eq!(world.cycle_target_filter().unwrap(), TargetFilter::Salvage);
        assert!(!world.cycle_targeting(tick, viewport(vec![])).unwrap());
        assert_eq!(world.cycle_target_filter().unwrap(), TargetFilter::All);
        assert!(
            world
                .cycle_targeting_with_work(tick, viewport(vec![]), &[last])
                .unwrap()
        );
        assert_eq!(world.selected_work_target().unwrap().id, last);
        assert_eq!(world.cycle_target_filter().unwrap(), TargetFilter::Ships);
        assert!(world.selected_work_target().is_none());
    }

    #[test]
    fn work_filters_refuse_unfitted_specialties_and_cold_restore_resets_filter() {
        let mut world = fitted(TargetingTier::Military);
        let work = world.modular_equipment_snapshot().unwrap().items;
        for kind in [ModuleKind::MiningTargeting, ModuleKind::SalvageTargeting] {
            let item = work.iter().find(|item| item.kind == kind).unwrap().id;
            module(&mut world, ModuleAction::Store { item });
        }
        launch(&mut world);
        let tick = world.snapshot().tick;
        assert_eq!(world.cycle_target_filter().unwrap(), TargetFilter::Mining);
        assert!(!world.cycle_targeting(tick, viewport(vec![])).unwrap());
        assert_eq!(world.cycle_target_filter().unwrap(), TargetFilter::Salvage);
        assert!(!world.cycle_targeting(tick, viewport(vec![])).unwrap());
        let restored = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
        assert_eq!(restored.target_filter(), TargetFilter::Ships);
    }

    #[test]
    fn accepted_rock_obstruction_releases_selected_guidance_without_idle_retry_or_account_effects()
    {
        let mut world = fitted(TargetingTier::Military);
        launch(&mut world);
        let belt = world.mining_belt_snapshot().unwrap();
        let blocker = belt.bodies[0];
        let destination = belt.bodies[6];
        assert_eq!(blocker.position.x, destination.position.x);
        assert_eq!(blocker.position.y, destination.position.y);
        assert!(blocker.position.z < destination.position.z);

        // Start the isolated ordinary-flight fixture on the clear exterior ray to the
        // selected rock. A different intact, real belt body obstructs that approach.
        world.state.position = [
            blocker.position.x as f32,
            blocker.position.y as f32,
            blocker.position.z as f32 - 250.0,
        ];
        world.state.orientation = [0.0, 0.0, 0.0, 1.0];
        world.state.speed = 0.0;
        let opening = world.snapshot();
        let pose = super::super::flight_pose(opening.position, opening.orientation).unwrap();
        assert!(world.mining_motion_clear(pose, pose, 0.0));
        assert_eq!(world.cycle_target_filter().unwrap(), TargetFilter::Mining);
        let target = WorkTargetId::Asteroid(destination.id);
        assert!(
            world
                .cycle_targeting_with_work(opening.tick, viewport(vec![]), &[target])
                .unwrap()
        );
        assert_eq!(world.selected_work_target().unwrap().id, target);

        let account_revision = world.commander_account.as_ref().unwrap().revision();
        let credits = world.credits_tenths();
        let ammunition = world.ammunition_snapshot();
        let work_count = world.mining.work_count();
        let mut stopped = None;
        for _ in 0..300 {
            let previous = world.snapshot();
            world.step(FlightInput::default()).unwrap();
            let current = world.snapshot();
            if let Some(refusal) = current.motion_refusal {
                assert_eq!(refusal, crate::MotionRefusal::AsteroidContact);
                assert_eq!(current.position, previous.position);
                assert_eq!(current.orientation, previous.orientation);
                assert_eq!(current.speed, 0.0);
                stopped = Some(current);
                break;
            }
            assert_eq!(world.selected_work_target().unwrap().id, target);
        }
        let stopped = stopped
            .unwrap_or_else(|| panic!("ordinary automatic approach did not meet the blocker"));
        assert!(stopped.position[2] > opening.position[2]);
        assert!(world.selected_work_target().is_none());
        assert!(world.targeting_snapshot().is_none());
        assert_eq!(world.target_filter(), TargetFilter::Mining);
        assert!(!world.auto_docking_active());

        for offset in 1..=8 {
            world.step(FlightInput::default()).unwrap();
            let idle = world.snapshot();
            assert_eq!(idle.tick, stopped.tick + offset);
            assert_eq!(idle.motion_refusal, None);
            assert_eq!(idle.position, stopped.position);
            assert_eq!(idle.orientation, stopped.orientation);
            assert_eq!(idle.speed, 0.0);
            assert!(world.selected_work_target().is_none());
        }
        assert_eq!(
            world.commander_account.as_ref().unwrap().revision(),
            account_revision
        );
        assert_eq!(world.credits_tenths(), credits);
        assert_eq!(world.ammunition_snapshot(), ammunition);
        assert_eq!(world.mining.work_count(), work_count);
        let after = world.mining_belt_snapshot().unwrap();
        assert_eq!(after.bodies, belt.bodies);
        assert_eq!(after.pending_ore, belt.pending_ore);
    }
}

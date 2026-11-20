//! Original L337 targeting-computer selection/disclosure (a11fe169, TargetingComputer.swift,
//! AutoTargetControl.swift and TargetTelemetry.swift). This owns only transient game selection;
//! it grants no weapon, docking, equipment, or other World authority.
use super::{CombatIdentity, FlightInput, FlightMode, FlightView, StepError, World};
use crate::docking::{RigidPose, Vec3};
use crate::equipment::TargetingTier;
use crate::navigation::SystemId;
use crate::traffic::{NpcId, NpcPhase, NpcSnapshot, Population};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetIdentity {
    Ship(NpcId),
    Station { system: SystemId, visit: u64 },
}

impl TargetIdentity {
    pub fn label(self) -> String {
        match self {
            Self::Ship(id) => format!("Target #{}", id.serial()),
            Self::Station { .. } => "Target #0".into(),
        }
    }

    fn order(self) -> u8 {
        match self {
            Self::Ship(id) => id.serial(),
            Self::Station { .. } => 0,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct TargetingSnapshot {
    pub identity: TargetIdentity,
    pub tier: TargetingTier,
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
}

impl Candidate {
    fn identity(self) -> TargetIdentity {
        match self {
            Self::Ship(actor) => TargetIdentity::Ship(actor.id),
            Self::Station(identity, _) => identity,
        }
    }

    fn position(self) -> Vec3 {
        match self {
            Self::Ship(actor) => actor.pose.position,
            Self::Station(_, pose) => pose.pose.position,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub(super) struct State {
    selected: Option<TargetIdentity>,
    origin: Option<CombatIdentity>,
    tier: Option<TargetingTier>,
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
            *self = Self::default();
            self.origin = Some(origin);
        }
        let mut membership = ordered.clone();
        membership.sort_by_key(|id| id.order());
        let changed = self.tier != Some(tier) || self.size != size || self.membership != membership;
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
        self.size = size;
        self.membership = membership;
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

fn grouped(candidate: Candidate, visible: &[NpcId]) -> u8 {
    match candidate {
        Candidate::Ship(actor) if visible.contains(&actor.id) => 0,
        Candidate::Ship(_) => 1,
        Candidate::Station(..) => 2,
    }
}

fn ship_bearing(player: RigidPose, actor: NpcSnapshot) -> [f64; 2] {
    let local = player
        .orientation
        .inverse_coordinates(offset(actor.pose.position, player.position));
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
        let mut candidates = self.targeting_candidates(origin, tier);
        candidates.retain(|candidate| {
            tier >= TargetingTier::Advanced || grouped(*candidate, &viewport.front) == 0
        });
        candidates.sort_by(|a, b| {
            let distance = |candidate: Candidate| {
                let delta = offset(candidate.position(), player.position);
                delta.dot(delta)
            };
            grouped(*a, &viewport.front)
                .cmp(&grouped(*b, &viewport.front))
                .then_with(|| distance(*a).total_cmp(&distance(*b)))
                .then_with(|| a.identity().order().cmp(&b.identity().order()))
        });
        let ordered = candidates.into_iter().map(Candidate::identity).collect();
        let station_was_selected = matches!(
            self.targeting.selected,
            Some(TargetIdentity::Station { .. })
        );
        let selected = self.targeting.cycle(origin, tier, viewport.size, ordered);
        if station_was_selected && !matches!(selected, Some(TargetIdentity::Station { .. })) {
            self.auto_docking = None;
        }
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
        self.targeting.selected = None;
    }

    fn current_target(&self) -> Option<(TargetingTier, Candidate)> {
        let selected = self.targeting.selected?;
        let origin = self.targeting_origin()?;
        if self.targeting.origin != Some(origin) {
            return None;
        }
        let tier = self.targeting_tier()?;
        self.targeting_candidates(origin, tier)
            .into_iter()
            .find(|c| c.identity() == selected)
            .map(|c| (tier, c))
    }

    pub fn targeting_station_selected(&self) -> bool {
        matches!(self.current_target(), Some((_, Candidate::Station(..))))
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
        if input.pitch != 0.0
            || input.yaw != 0.0
            || input.roll != 0.0
            || matches!(target, Candidate::Station(..)) && (input.throttle != 0.0 || input.brake)
        {
            self.cancel_targeting();
            return input;
        }
        let Candidate::Ship(actor) = target else {
            return input;
        };
        let Some(player) = super::flight_pose(self.state.position, self.state.orientation) else {
            return input;
        };
        let [bearing, elevation] = ship_bearing(player, actor);
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
        FlightInput {
            yaw,
            pitch,
            ..input
        }
    }

    pub fn targeting_snapshot(&self) -> Option<TargetingSnapshot> {
        let (tier, candidate) = self.current_target()?;
        let identity = candidate.identity();
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
                let guidance = if ship_bearing(player, actor)
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
        };
        Some(TargetingSnapshot {
            identity,
            tier,
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
}

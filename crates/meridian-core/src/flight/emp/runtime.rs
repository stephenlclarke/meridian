//! Actual held input and finite electrical publication under the sole World owner.
use super::{Arc, EmpError, EmpOrigin, FlightMode, MAX_PULSES, State, World, policy, scene};
use crate::traffic::{EmpResourceAction, NpcId};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmpChargePhase {
    Charging,
    Cancelled,
    Released,
}
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Charge {
    pub(super) item: u64,
    pub(super) target: NpcId,
    pub(super) start: scene::Scene,
    pub(super) phase: EmpChargePhase,
    pub(super) ended_at: Option<u64>,
    pub(super) pulse: Option<u16>,
}
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Pulse {
    pub(super) origin: EmpOrigin,
    pub(super) charge: Charge,
    pub(super) release: scene::Scene,
    pub(super) contacts: Vec<scene::Contact>,
    pub(super) player_shield_absorbed: u16,
    pub(super) player_electrical_drained: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Episode {
    pub(super) origin: EmpOrigin,
    pub(super) victim: NpcId,
    pub(super) disabled_at: u64,
    pub(super) recovered_at: Option<(u64, u64)>,
}
pub(in crate::flight) struct Frame<'a> {
    pub(in crate::flight) next: super::super::Snapshot,
    pub(in crate::flight) navigation: Option<crate::navigation::NavigationState>,
    pub(in crate::flight) traffic: &'a crate::traffic::State,
    pub(in crate::flight) missiles: &'a super::super::missiles::State,
    pub(in crate::flight) pulse: super::super::weapons::PulseResources,
    pub(in crate::flight) combat: super::super::combat::State,
}
pub(in crate::flight) struct Stage {
    pub(in crate::flight) state: State,
    pub(in crate::flight) traffic: crate::traffic::State,
    pub(in crate::flight) pulse: super::super::weapons::PulseResources,
    pub(in crate::flight) combat: super::super::combat::State,
}
impl Stage {
    fn recover(&mut self, tick: u64, clock: u64) -> Result<(), super::super::StepError> {
        // Only current, admitted local custody is supported. Departure is refused while a live
        // victim is disabled; no unqualified off-system archive or occupied capsule is fabricated.
        for index in 0..self.state.episodes.len() {
            let episode = self.state.episodes[index];
            if episode.recovered_at.is_none()
                && clock
                    > episode
                        .disabled_at
                        .checked_add(policy::RECOVERY_TICKS)
                        .ok_or(super::super::StepError::TickOverflow)?
                && self
                    .traffic
                    .emp_resources(episode.victim)
                    .is_some_and(|(h, _, _, p)| h > 0 && p.is_some_and(|p| p.disabled))
            {
                self.traffic
                    .emp_event(
                        episode.origin,
                        episode.victim,
                        tick,
                        clock,
                        EmpResourceAction::Recover,
                    )
                    .map_err(|_| super::super::StepError::TickOverflow)?;
                Arc::make_mut(&mut self.state.episodes)[index].recovered_at = Some((clock, tick));
            }
        }
        Ok(())
    }
}
impl State {
    pub(in crate::flight) fn input_blocked(&self) -> bool {
        self.meter.disabled_at.is_some()
            || self
                .charge
                .as_ref()
                .is_some_and(|c| c.phase == EmpChargePhase::Charging)
    }
    pub(in crate::flight) fn electrical_disabled(&self) -> bool {
        self.meter.disabled_at.is_some()
    }
    pub(in crate::flight) fn travel_blocked(&self, traffic: &crate::traffic::State) -> bool {
        self.input_blocked()
            || self.episodes.iter().any(|episode| {
                episode.recovered_at.is_none()
                    && traffic
                        .emp_resources(episode.victim)
                        .is_some_and(|(hull, _, _, pool)| {
                            hull > 0 && pool.is_some_and(|p| p.disabled)
                        })
            })
    }
    pub(super) fn recent_attack(
        traffic: &crate::traffic::State,
        owner: super::super::CombatIdentity,
        target: NpcId,
        tick: u64,
    ) -> bool {
        traffic.player_hits().iter().any(|hit| {
            hit.attacker == target
                && hit.player_hull == owner.hull
                && hit.visit == owner.visit
                && target.system() == owner.system
                && tick
                    .checked_sub(hit.world_tick)
                    .is_some_and(|elapsed| elapsed <= policy::RECOVERY_TICKS)
        })
    }
    pub(super) fn energy_stage(world: &World) -> Result<u8, EmpError> {
        let items = world
            .modular_equipment_snapshot()
            .ok_or(EmpError::IncompatibleEnergy)?
            .items;
        let item = items
            .iter()
            .find(|i| i.slot == Some(super::super::ModuleSlot::Energy))
            .ok_or(EmpError::IncompatibleEnergy)?;
        match item.kind {
            super::super::ModuleKind::BasicEnergySystem => Ok(0),
            super::super::ModuleKind::ExtraEnergyUnit => Ok(1),
            super::super::ModuleKind::NavalEnergyUnit => Ok(2),
            super::super::ModuleKind::EliteEnergyUnit => Ok(3),
            _ => Err(EmpError::IncompatibleEnergy),
        }
    }
    fn cancel(&mut self, clock: u64) {
        if let Some(charge) = &mut self.charge
            && charge.phase == EmpChargePhase::Charging
        {
            charge.phase = EmpChargePhase::Cancelled;
            charge.ended_at = Some(clock);
        }
    }
    pub(in crate::flight) fn cancel_for_ecm(&mut self, clock: u64) {
        self.cancel(clock);
        self.held_tick = None;
    }
    pub(in crate::flight) fn prepare_step(
        &self,
        world: &World,
        frame: Frame<'_>,
    ) -> Result<Stage, super::super::StepError> {
        let Frame {
            next,
            navigation,
            traffic,
            missiles,
            pulse,
            combat,
        } = frame;
        let mut stage = Stage {
            state: self.clone(),
            traffic: traffic.clone(),
            pulse,
            combat,
        };
        let Some(item) = self
            .owned()
            .map_err(|_| super::super::StepError::NonFiniteState)?
        else {
            return Ok(stage);
        };
        let clock = combat.physical_tick();
        if world.state.mode == FlightMode::Docked {
            return Ok(stage);
        }
        if clock
            != self
                .meter_clock
                .checked_add(1)
                .ok_or(super::super::StepError::TickOverflow)?
        {
            return Err(super::super::StepError::NonFiniteState);
        }
        let rate = u32::from(world.energy_recharge_hundredths()) * (policy::SCALE / 100);
        if self.main_present || item.fitted {
            stage.state.main_present = true;
            stage
                .state
                .meter
                .post_main(clock, rate)
                .map_err(|_| super::super::StepError::TickOverflow)?;
        }
        let scope_matches = navigation.zip(world.navigation).is_some_and(|(new, old)| {
            let new = new.snapshot(0);
            let old = old.snapshot(0);
            new.visit == old.visit && new.system == old.system
        }) && next.mode != FlightMode::Docked;
        stage.state.meter_clock = clock;
        stage.state.posted_tick = stage.state.main_present.then_some(next.tick);
        let held = self.held_tick == Some(world.state.tick);
        stage.state.held_tick = None;
        if !held || next.mode != FlightMode::Flight || !stage.pulse.alive() {
            stage.state.cancel(clock);
            if next.mode == FlightMode::Docked {
                stage.state.charge = None;
            }
        } else if let Some(charge) = self.charge.clone()
            && charge.phase == EmpChargePhase::Charging
            && clock
                >= charge
                    .start
                    .physical_tick
                    .saturating_add(policy::WARMUP_TICKS)
        {
            let mut released = charge;
            released.phase = EmpChargePhase::Released;
            released.ended_at = Some(clock);
            stage.state.charge = Some(released.clone());
            // Every candidate, including every splash victim and both resource owners, remains
            // private until all admissions succeed. A failed release is consumed, never retried.
            if let Ok(result) = stage.state.release(
                world,
                Frame {
                    next,
                    navigation,
                    traffic: &stage.traffic,
                    missiles,
                    pulse: stage.pulse,
                    combat: stage.combat,
                },
                released,
            ) {
                stage = result;
            }
        }
        let fired = stage.state.pulses.len() != self.pulses.len();
        if !fired && item.fitted && scope_matches {
            stage.state.bank_clock = clock;
            if self.bank_clock == self.meter_clock
                && stage.state.main_present
                && stage.pulse.alive()
            {
                stage
                    .state
                    .meter
                    .transfer(item.grades)
                    .map_err(|_| super::super::StepError::TickOverflow)?;
            }
        }
        if !held
            && self
                .charge
                .as_ref()
                .is_some_and(|c| c.phase != EmpChargePhase::Charging)
        {
            stage.state.charge = None;
        }
        stage.recover(next.tick, clock)?;
        Ok(stage)
    }
    fn release(
        &self,
        world: &World,
        frame: Frame<'_>,
        mut charge: Charge,
    ) -> Result<Stage, EmpError> {
        let Frame {
            next,
            navigation,
            traffic,
            missiles,
            pulse,
            combat,
        } = frame;
        let item = self
            .owned()
            .map_err(EmpError::Save)?
            .filter(|i| i.fitted && i.item == charge.item)
            .ok_or(EmpError::MissingItem)?;
        let owner = world.combat_identity().ok_or(EmpError::NoCommander)?;
        if world.npc_missile_protected()
            || owner != charge.start.owner
            || navigation.is_none_or(|n| n.snapshot(0).system != owner.system)
            || self.meter.disabled_at.is_some()
            || self.meter.main == 0
            || self.pulses.len() >= MAX_PULSES
            || !Self::recent_attack(traffic, owner, charge.target, next.tick)
        {
            return Err(EmpError::NoRecentAggressor);
        }
        let physical = combat.physical_tick();
        let profile = item.grades.profile(Self::energy_stage(world)?)?;
        let pose = super::super::flight_pose(next.position, next.orientation)
            .ok_or(EmpError::InvalidContact)?;
        let scene = scene::Scene::capture(
            traffic,
            missiles,
            world.combat.emissions,
            owner,
            pose,
            next.tick,
            physical,
        )?;
        let contacts = scene.contacts(world, profile, charge.target)?;
        for contact in &contacts {
            if let scene::Victim::Npc(id) = contact.victim {
                if !Self::recent_attack(traffic, owner, id, next.tick) {
                    return Err(EmpError::ProtectedTarget);
                }
                if traffic
                    .emp_resources(id)
                    .is_none_or(|(h, _, _, p)| h == 0 || p.is_some_and(|p| p.disabled))
                {
                    return Err(EmpError::NoTarget);
                }
            }
        }
        let origin = EmpOrigin {
            owner,
            item: item.item,
            pulse: u16::try_from(self.pulses.len() + 1).map_err(|_| EmpError::Capacity)?,
        };
        let mut candidate = Stage {
            state: self.clone(),
            traffic: traffic.clone(),
            pulse: pulse
                .prepare_emp_discharge()
                .map_err(|_| EmpError::Resources)?,
            combat,
        };
        candidate.state.bank_clock = physical;
        candidate
            .state
            .meter
            .discharge()
            .map_err(|_| EmpError::Resources)?;
        let mut absorbed = 0;
        let mut drained = 0;
        for contact in &contacts {
            match contact.victim {
                scene::Victim::Npc(id) => {
                    candidate
                        .traffic
                        .emp_event(
                            origin,
                            id,
                            next.tick,
                            physical,
                            EmpResourceAction::Drain {
                                front: contact.front,
                                power_hundredths: contact.power_hundredths,
                            },
                        )
                        .map_err(|_| EmpError::Capacity)?;
                    if candidate
                        .traffic
                        .emp_resources(id)
                        .is_some_and(|(_, _, _, p)| p.is_some_and(|p| p.disabled))
                    {
                        physical
                            .checked_add(policy::RECOVERY_TICKS)
                            .ok_or(EmpError::Capacity)?;
                        Arc::make_mut(&mut candidate.state.episodes).push(Episode {
                            origin,
                            victim: id,
                            disabled_at: physical,
                            recovered_at: None,
                        });
                    }
                }
                scene::Victim::Player => {
                    absorbed = candidate
                        .combat
                        .absorb_emp(contact.front, contact.power_hundredths);
                    drained = candidate
                        .state
                        .meter
                        .drain(
                            u32::from(contact.power_hundredths - absorbed) * (policy::SCALE / 100),
                            physical,
                        )
                        .map_err(EmpError::Save)?;
                }
            }
        }
        charge.pulse = Some(origin.pulse);
        candidate.state.charge = Some(charge.clone());
        Arc::make_mut(&mut candidate.state.pulses).push(Pulse {
            origin,
            charge,
            release: scene,
            contacts,
            player_shield_absorbed: absorbed,
            player_electrical_drained: drained,
        });
        Ok(candidate)
    }
}
impl World {
    /// Held input is accepted only for this tick and never restored as live input from a save.
    pub fn request_emp_trigger(&mut self, held: bool, tick: u64) -> Result<(), EmpError> {
        if tick != self.state.tick {
            return Err(EmpError::StaleReview);
        }
        if self.is_paused() {
            return Err(EmpError::Paused);
        }
        if self.commander_account.is_none() {
            return Err(EmpError::NoCommander);
        }
        if !held {
            self.emp.held_tick = None;
            return Ok(());
        }
        if self.ecm_timer().is_some() {
            return Err(EmpError::Resources);
        }
        if self.emp.charge.is_some() {
            self.emp.held_tick = Some(tick);
            return Ok(());
        }
        if self.state.mode != FlightMode::Flight || !self.pulse.alive() {
            return Err(EmpError::FlightRequired);
        }
        if self
            .navigation
            .is_some_and(|n| n.snapshot(0).travel.is_some())
        {
            return Err(EmpError::ActiveTravel);
        }
        if self.npc_missile_protected() {
            return Err(EmpError::ProtectedTarget);
        }
        if self.emp.pulses.len() >= MAX_PULSES {
            return Err(EmpError::Capacity);
        }
        let item = self
            .owned_emp()
            .filter(|item| item.fitted)
            .ok_or(EmpError::MissingItem)?;
        if !self.emp.main_present
            || self.emp.meter.main == 0
            || self.emp.posted_tick != Some(tick)
            || self.emp.meter.disabled_at.is_some()
            || self.emp.meter.bank < policy::PULSE_COST
        {
            return Err(EmpError::Resources);
        }
        self.pulse
            .prepare_emp_discharge()
            .map_err(|_| EmpError::Resources)?;
        let owner = self.combat_identity().ok_or(EmpError::NoCommander)?;
        let player = super::super::flight_pose(self.state.position, self.state.orientation)
            .ok_or(EmpError::InvalidContact)?;
        let profile = item.grades.profile(State::energy_stage(self)?)?;
        let first = self
            .traffic
            .beam(
                self.station_data.as_ref().ok_or(EmpError::InvalidContact)?,
                owner.system,
                player.position,
                player.orientation.z,
                profile.range,
            )
            .ok_or(EmpError::NoTarget)?;
        if !State::recent_attack(&self.traffic, owner, first.id, tick) {
            return Err(EmpError::NoRecentAggressor);
        }
        if self
            .traffic
            .emp_resources(first.id)
            .is_none_or(|(h, _, _, p)| h == 0 || p.is_some_and(|p| p.disabled))
        {
            return Err(EmpError::NoTarget);
        }
        let scene = scene::Scene::capture(
            &self.traffic,
            &self.missiles,
            self.combat.emissions,
            owner,
            player,
            tick,
            self.combat.physical_tick(),
        )?;
        scene.contacts(self, profile, first.id)?;
        let mut candidate = self.emp.clone();
        candidate.charge = Some(Charge {
            item: item.item,
            target: first.id,
            start: scene,
            phase: EmpChargePhase::Charging,
            ended_at: None,
            pulse: None,
        });
        candidate.held_tick = Some(tick);
        self.save_bytes_with_emp(
            self.commander_account
                .as_ref()
                .ok_or(EmpError::NoCommander)?,
            &candidate,
        )
        .map_err(EmpError::Save)?;
        self.emp = candidate;
        Ok(())
    }
}

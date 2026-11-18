//! Independent provenance and effect joins. Decoded descriptors never authorize a producer.
use super::*;
use crate::traffic::{EmpResourceAction, EmpResourceEvent};
impl State {
    fn item_at(&self, world: &World, scene: &scene::Scene) -> Result<OwnedEmp, SaveError> {
        let equipment = self
            .equipment
            .iter()
            .filter(|e| e.tick <= scene.world_tick)
            .copied()
            .collect();
        let prefix = Self {
            equipment: Arc::new(equipment),
            ..Self::default()
        };
        let item = prefix.owned()?.filter(|i| i.fitted).ok_or(INVALID)?;
        if item.owner != scene.owner.owner
            || item.hull != scene.owner.hull
            || item.purchase_dock.visit() >= scene.owner.visit
        {
            return Err(INVALID);
        }
        for e in prefix.equipment.iter() {
            if e.dock.visit() >= scene.owner.visit
                || world
                    .damage
                    .berth_clock(e.dock.sequence())
                    .is_none_or(|c| c > scene.physical_tick)
            {
                return Err(INVALID);
            }
        }
        Ok(item)
    }
    fn profile_at(
        &self,
        world: &World,
        modular: &super::super::modular::State,
        scene: &scene::Scene,
    ) -> Result<policy::Profile, SaveError> {
        let item = self.item_at(world, scene)?;
        let stage = modular
            .emp_energy_changes(world)?
            .into_iter()
            .rev()
            .find(|(clock, _)| *clock <= scene.physical_tick)
            .and_then(|(_, stage)| stage)
            .ok_or(INVALID)?;
        item.grades.profile(stage).map_err(|_| INVALID)
    }
    fn validate_start(
        &self,
        world: &World,
        modular: &super::super::modular::State,
        charge: &runtime::Charge,
        before: u16,
    ) -> Result<(), SaveError> {
        charge.start.validate(world, before)?;
        let item = self.item_at(world, &charge.start)?;
        if charge.item != item.item
            || !State::recent_attack(
                &world.traffic,
                charge.start.owner,
                charge.target,
                charge.start.world_tick,
            )
            || !world
                .damage
                .emp_ready_at(world, charge.start.physical_tick)?
        {
            return Err(INVALID);
        }
        let profile = self.profile_at(world, modular, &charge.start)?;
        let contacts = charge
            .start
            .contacts(world, profile, charge.target)
            .map_err(|_| INVALID)?;
        if contacts.is_empty() {
            return Err(INVALID);
        }
        let (hull, _, _, pool) = world.traffic.emp_before(
            charge.target,
            charge.start.local_tick,
            charge.start.world_tick,
            before,
        )?;
        if hull == 0 || pool.is_some_and(|p| p.disabled) {
            return Err(INVALID);
        }
        Ok(())
    }
    fn pulse_events(
        &self,
        world: &World,
        modular: &super::super::modular::State,
    ) -> Result<(Vec<EmpResourceEvent>, Vec<runtime::Episode>), SaveError> {
        let mut events = Vec::new();
        let mut episodes = Vec::new();
        let mut last_release = None;
        for (index, pulse) in self.pulses.iter().enumerate() {
            let ordinal = u16::try_from(index + 1).map_err(|_| INVALID)?;
            self.validate_start(world, modular, &pulse.charge, ordinal)?;
            pulse.release.validate(world, ordinal)?;
            let c = &pulse.charge;
            let s = &c.start;
            let r = &pulse.release;
            if pulse.origin
                != (EmpOrigin {
                    owner: s.owner,
                    item: c.item,
                    pulse: ordinal,
                })
                || c.phase != EmpChargePhase::Released
                || c.pulse != Some(ordinal)
                || c.ended_at != Some(r.physical_tick)
                || r.owner != s.owner
                || r.physical_tick.checked_sub(s.physical_tick) != Some(policy::WARMUP_TICKS)
                || r.world_tick.checked_sub(s.world_tick) != Some(policy::WARMUP_TICKS)
                || r.local_tick.checked_sub(s.local_tick) != Some(policy::WARMUP_TICKS)
                || last_release.is_some_and(|clock| s.physical_tick <= clock)
            {
                return Err(INVALID);
            }
            let profile = self.profile_at(world, modular, r)?;
            let expected = r.contacts(world, profile, c.target).map_err(|_| INVALID)?;
            if expected != pulse.contacts || expected.len() > crate::traffic::MAX_TRAFFIC_ACTORS + 1
            {
                return Err(INVALID);
            }
            let mut player = false;
            for contact in expected {
                match contact.victim {
                    scene::Victim::Npc(id) => {
                        if !State::recent_attack(&world.traffic, r.owner, id, r.world_tick) {
                            return Err(INVALID);
                        }
                        let (hull, front, rear, pool) =
                            world
                                .traffic
                                .emp_before(id, r.local_tick, r.world_tick, ordinal)?;
                        if hull == 0 || pool.is_some_and(|p| p.disabled) {
                            return Err(INVALID);
                        }
                        let shield = if contact.front { front } else { rear };
                        let overflow = u32::from(contact.power_hundredths.saturating_sub(shield))
                            * (policy::SCALE / 100);
                        let electrical = pool.map_or(policy::MAIN_CAPACITY, |p| p.remaining);
                        if overflow >= electrical {
                            episodes.push(runtime::Episode {
                                origin: pulse.origin,
                                victim: id,
                                disabled_at: r.physical_tick,
                                recovered_at: None,
                            });
                        }
                        events.push(EmpResourceEvent {
                            origin: pulse.origin,
                            victim: id,
                            local_tick: r.local_tick,
                            world_tick: r.world_tick,
                            physical_tick: r.physical_tick,
                            action: EmpResourceAction::Drain {
                                front: contact.front,
                                power_hundredths: contact.power_hundredths,
                            },
                        });
                    }
                    scene::Victim::Player => {
                        if player || pulse.player_shield_absorbed > contact.power_hundredths {
                            return Err(INVALID);
                        }
                        player = true;
                    }
                }
            }
            if !player
                && (pulse.player_shield_absorbed != 0 || pulse.player_electrical_drained != 0)
            {
                return Err(INVALID);
            }
            last_release = Some(r.physical_tick);
        }
        Ok((events, episodes))
    }
    fn validate_episodes(
        &self,
        world: &World,
        expected: &mut [runtime::Episode],
        events: &mut Vec<EmpResourceEvent>,
    ) -> Result<(), SaveError> {
        if self.episodes.len() != expected.len() {
            return Err(INVALID);
        }
        for (stored, derived) in self.episodes.iter().zip(expected) {
            let pulse = self
                .pulses
                .get(usize::from(derived.origin.pulse) - 1)
                .ok_or(INVALID)?;
            let r = &pulse.release;
            let recovery_clock = derived
                .disabled_at
                .checked_add(policy::RECOVERY_TICKS + 1)
                .ok_or(INVALID)?;
            let recovery_tick = r
                .world_tick
                .checked_add(policy::RECOVERY_TICKS + 1)
                .ok_or(INVALID)?;
            if let Some((physical, tick)) = stored.recovered_at {
                if physical != recovery_clock || tick < recovery_tick || tick > world.state.tick {
                    return Err(INVALID);
                }
                let local = r
                    .local_tick
                    .checked_add(policy::RECOVERY_TICKS + 1)
                    .ok_or(INVALID)?;
                let (hull, _, _, pool) = world.traffic.emp_before_recovery(
                    derived.victim,
                    local,
                    tick,
                    derived.origin,
                )?;
                if hull == 0 || pool.is_none_or(|p| !p.disabled) {
                    return Err(INVALID);
                }
                derived.recovered_at = Some((physical, tick));
                events.push(EmpResourceEvent {
                    origin: derived.origin,
                    victim: derived.victim,
                    local_tick: local,
                    world_tick: tick,
                    physical_tick: physical,
                    action: EmpResourceAction::Recover,
                });
            } else if world.combat.physical_tick() >= recovery_clock
                && world
                    .traffic
                    .emp_resources(derived.victim)
                    .is_some_and(|(h, _, _, _)| h > 0)
            {
                return Err(INVALID);
            }
            if stored != derived {
                return Err(INVALID);
            }
            // Local custody cannot leave while a surviving victim remains electrically disabled.
            if world
                .jump_journal
                .emp_arrival_clocks()
                .into_iter()
                .any(|arrival| {
                    arrival > derived.disabled_at
                        && !world.traffic.emp_dead_by(
                            derived.victim,
                            r.local_tick.saturating_add(arrival - derived.disabled_at),
                        )
                        && arrival
                            <= stored
                                .recovered_at
                                .map_or(world.combat.physical_tick(), |(clock, _)| clock)
                })
            {
                return Err(INVALID);
            }
        }
        Ok(())
    }
    pub(in crate::flight) fn validate(
        &self,
        world: &World,
        account: &crate::freight::g02::CommanderAccount,
        modular: &super::super::modular::State,
    ) -> Result<(u32, u64), SaveError> {
        let terms = self.terms(world, account)?;
        let Some(item) = self.owned()? else {
            if self != &Self::default() || !world.traffic.emp_events().is_empty() {
                return Err(INVALID);
            }
            return Ok(terms);
        };
        let navigation = world.navigation.ok_or(INVALID)?;
        if navigation.snapshot(0).travel.is_some() && self.travel_blocked(&world.traffic) {
            return Err(INVALID);
        }
        if self.pulses.len() > MAX_PULSES
            || self.episodes.len() > MAX_PULSES * crate::traffic::MAX_TRAFFIC_ACTORS
            || (navigation.launches() == 0 && self.meter_clock != 0)
            || (world.state.mode == FlightMode::Docked
                && self.meter_clock
                    != world
                        .damage
                        .berth_clock(navigation.dock_sequence())
                        .ok_or(INVALID)?)
            || self.meter.bank > item.grades.bank_capacity()
        {
            return Err(INVALID);
        }
        let modules = modular.ledger(world, account)?;
        let utilities = modules
            .items
            .iter()
            .filter(|i| i.slot == Some(super::super::ModuleSlot::UtilityFuelScoops))
            .count();
        if usize::from(account.equipment_state().units_used())
            + utilities
            + usize::from(item.fitted)
            > usize::from(
                crate::equipment::definition()
                    .map_err(|_| INVALID)?
                    .utility_mounts_capacity,
            )
        {
            return Err(INVALID);
        }
        let ammunition = world.ammunition.ledger(world, account)?;
        if ammunition.items.iter().filter(|i| !i.consumed).count()
            + 5
            + usize::from(account.equipment_state().owned().is_some())
            + modular.paid_count()
            > 256
        {
            return Err(INVALID);
        }
        self.replay_meter(world, modular)?;
        let (mut events, mut episodes) = self.pulse_events(world, modular)?;
        self.validate_episodes(world, &mut episodes, &mut events)?;
        events.sort_by_key(|e| (e.world_tick, matches!(e.action, EmpResourceAction::Recover)));
        if events != world.traffic.emp_events() {
            return Err(INVALID);
        }
        if let Some(c) = &self.charge {
            if world.state.mode == FlightMode::Docked {
                return Err(INVALID);
            }
            if let Some(ordinal) = c.pulse {
                if self
                    .pulses
                    .last()
                    .is_none_or(|p| p.origin.pulse != ordinal || &p.charge != c)
                {
                    return Err(INVALID);
                }
            } else {
                self.validate_start(world, modular, c, self.pulses.len() as u16 + 1)?;
                if self
                    .pulses
                    .last()
                    .is_some_and(|p| c.start.physical_tick <= p.release.physical_tick)
                {
                    return Err(INVALID);
                }
                let elapsed = c
                    .ended_at
                    .unwrap_or(self.meter_clock)
                    .checked_sub(c.start.physical_tick)
                    .ok_or(INVALID)?;
                match c.phase {
                    EmpChargePhase::Charging
                        if c.ended_at.is_none()
                            && elapsed < policy::WARMUP_TICKS
                            && c.start.owner == world.combat_identity().ok_or(INVALID)? => {}
                    EmpChargePhase::Cancelled
                        if c.ended_at.is_some() && elapsed <= policy::WARMUP_TICKS => {}
                    EmpChargePhase::Released
                        if c.ended_at.is_some() && elapsed == policy::WARMUP_TICKS => {}
                    _ => return Err(INVALID),
                }
                if c.ended_at.is_some_and(|end| end > self.meter_clock) {
                    return Err(INVALID);
                }
            }
        }
        Ok(terms)
    }
}

//! Exact shared player pools. World alone seals spends, berth resets and incoming hits.
use super::weapons::PulseResources;
use super::{FireError, FlightMode, OrdinaryTarget, StepError, World};
use crate::save::{MAX_SAVE_BYTES, Reader, SaveError, Writer};
use std::{collections::BTreeMap, sync::Arc};
mod capacity;
const INVALID: SaveError = SaveError::InvalidState("player resource replay");
// Nine encoded bytes per event, admitted against the existing whole-save budget.
const MAX_EVENTS: usize = MAX_SAVE_BYTES / 9;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Berth,
    Pulse,
    Laser(super::OrdinaryWeapon),
    Mining(u8),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Event {
    clock: u64,
    kind: Kind,
}
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct State {
    enabled: bool,
    events: Arc<Vec<Event>>,
}
struct LaserSpend {
    emission: u64,
    clock: u64,
    weapon: super::OrdinaryWeapon,
    item: u64,
}

impl State {
    pub(super) fn fresh() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }
    pub(super) fn enabled(&self) -> bool {
        self.enabled
    }
    fn append(&self, clock: u64, kind: Kind) -> Result<Self, ()> {
        if !self.enabled {
            return Ok(self.clone());
        }
        if self.events.len() >= MAX_EVENTS {
            return Err(());
        }
        let mut next = self.clone();
        Arc::make_mut(&mut next.events).push(Event { clock, kind });
        Ok(next)
    }
    pub(super) fn pulse(&self, clock: u64) -> Result<Self, FireError> {
        self.append(clock, Kind::Pulse)
            .map_err(|()| FireError::SequenceExhausted)
    }
    pub(super) fn laser(
        &self,
        clock: u64,
        weapon: super::OrdinaryWeapon,
    ) -> Result<Self, FireError> {
        if weapon == super::OrdinaryWeapon::FrontPulseLaser {
            return self.pulse(clock);
        }
        if !self.enabled || !weapon.is_laser() {
            return Err(FireError::NoFittedFrontLaser);
        }
        self.append(clock, Kind::Laser(weapon))
            .map_err(|()| FireError::SequenceExhausted)
    }
    pub(super) fn has_rapid_spend(&self) -> bool {
        self.events
            .iter()
            .any(|event| matches!(event.kind, Kind::Laser(weapon) if weapon.cooldown_ticks() == 6))
    }
    pub(super) fn quiet_since(&self, tick: u64, anchor: u64, physical: u64) -> bool {
        let Some(elapsed) = tick.checked_sub(anchor) else {
            return false;
        };
        let Some(through) = physical.checked_sub(elapsed) else {
            return false;
        };
        self.events
            .iter()
            .all(|event| event.kind == Kind::Berth || event.clock <= through)
    }
    pub(super) fn berth_clock(&self, sequence: u64) -> Option<u64> {
        if !self.enabled {
            return None;
        }
        if sequence == 1 {
            return Some(0);
        }
        self.events
            .iter()
            .filter(|event| event.kind == Kind::Berth)
            .nth(usize::try_from(sequence.checked_sub(2)?).ok()?)
            .map(|event| event.clock)
    }
    pub(super) fn mining(&self, clock: u64, body: u8) -> Result<Self, FireError> {
        if usize::from(body) >= crate::mining::BELT_BODY_COUNT || !self.enabled {
            return Err(FireError::InvalidGeometry);
        }
        self.append(clock, Kind::Mining(body))
            .map_err(|()| FireError::SequenceExhausted)
    }
    pub(super) fn step(
        &self,
        opening: FlightMode,
        next: FlightMode,
        clock: u64,
    ) -> Result<Self, StepError> {
        if opening != FlightMode::Docked && next == FlightMode::Docked {
            self.append(clock, Kind::Berth)
                .map_err(|()| StepError::TickOverflow)
        } else {
            Ok(self.clone())
        }
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.bool(self.enabled);
        w.u16(self.events.len() as u16);
        for event in self.events.iter() {
            w.u64(event.clock);
            w.u8(match event.kind {
                Kind::Berth => 0,
                Kind::Pulse => 1,
                Kind::Laser(weapon) => weapon.tag() + 1,
                Kind::Mining(body) => 32 + body,
            });
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        let enabled = r.bool()?;
        let count = usize::from(r.u16()?);
        if count > MAX_EVENTS {
            return Err(INVALID);
        }
        let mut events = Vec::with_capacity(count);
        for _ in 0..count {
            let clock = r.u64()?;
            let kind = match r.u8()? {
                0 => Kind::Berth,
                1 => Kind::Pulse,
                tag @ 3..=6 if version >= 16 => {
                    Kind::Laser(super::OrdinaryWeapon::from_tag(tag - 1)?)
                }
                tag @ 32..=43 if version >= 16 => Kind::Mining(tag - 32),
                _ => return Err(INVALID),
            };
            events.push(Event { clock, kind });
        }
        Ok(Self {
            enabled,
            events: Arc::new(events),
        })
    }
    fn sealed_spends(
        &self,
        world: &World,
        dock_sequence: u64,
        end: u64,
    ) -> Result<Vec<ResourceEvent>, SaveError> {
        let mut missile_spends: Vec<_> = world.missiles.resource_emissions().collect();
        missile_spends.sort_unstable_by_key(|&(emission, _)| emission);
        let mut missiles = missile_spends.into_iter().peekable();
        let mut next_emission = 1;
        let mut prior = 0;
        let mut mining_count = 0_usize;
        let mut berths = 1_u64;
        let mut events = Vec::with_capacity(self.events.len() + 128);
        let mut lasers = Vec::new();
        for event in self.events.iter() {
            if event.clock <= prior || event.clock > end {
                return Err(INVALID);
            }
            prior = event.clock;
            if event.kind == Kind::Berth {
                berths += 1;
                events.push((event.clock, 0, 0, Effect::Berth));
                continue;
            }
            next_emission = merge_missile_emissions(&mut missiles, next_emission, event.clock)?;
            let effect = match event.kind {
                Kind::Mining(body) => {
                    let hit = world.mining.strokes().get(mining_count).ok_or(INVALID)?;
                    if hit.physical_tick != event.clock
                        || hit.contact.asteroid.ordinal() != body
                        || hit.emission != next_emission
                    {
                        return Err(INVALID);
                    }
                    mining_count += 1;
                    Effect::Mining
                }
                kind => {
                    let weapon = match kind {
                        Kind::Pulse => super::OrdinaryWeapon::FrontPulseLaser,
                        Kind::Laser(weapon) => weapon,
                        _ => return Err(INVALID),
                    };
                    let fitted = world
                        .modular
                        .front_at_physical(world, event.clock)
                        .ok_or(INVALID)?;
                    if super::OrdinaryWeapon::from_module(fitted.kind) != Some(weapon) {
                        return Err(INVALID);
                    }
                    lasers.push(LaserSpend {
                        emission: next_emission,
                        clock: event.clock,
                        weapon,
                        item: fitted.id,
                    });
                    Effect::Laser(weapon)
                }
            };
            world.ecm.validate_emission(event.clock, next_emission)?;
            next_emission += 1;
            events.push((event.clock, 3, 0, effect));
        }
        for (emission, _) in missiles {
            if emission != next_emission {
                return Err(INVALID);
            }
            next_emission += 1;
        }
        if mining_count != world.mining.strokes().len()
            || berths != dock_sequence
            || next_emission - 1 != world.combat.emissions
        {
            return Err(INVALID);
        }
        validate_laser_receipts(world, end, &lasers)?;
        Ok(events)
    }
    pub(super) fn emp_berths(&self) -> Vec<u64> {
        self.events
            .iter()
            .filter(|event| event.kind == Kind::Berth)
            .map(|event| event.clock)
            .collect()
    }
    pub(super) fn emp_ready_at(&self, world: &World, clock: u64) -> Result<bool, SaveError> {
        let dock = world.navigation.ok_or(INVALID)?.dock_sequence();
        let mut events = self.resource_events(world, dock)?;
        events.retain(|&(at, _, _, _)| at <= clock);
        Ok(replay(&events, clock)?
            .pulse
            .prepare_emp_discharge()
            .is_ok())
    }
    pub(super) fn alive_before_commands(&self, world: &World) -> Result<bool, SaveError> {
        let dock = world.navigation.ok_or(INVALID)?.dock_sequence();
        let end = world.combat.physical_tick();
        let mut events = self.resource_events(world, dock)?;
        events.retain(|&(clock, phase, _, _)| clock < end || phase < 3);
        Ok(replay(&events, end)?.pulse.alive())
    }
    fn resource_events(
        &self,
        world: &World,
        dock_sequence: u64,
    ) -> Result<Vec<ResourceEvent>, SaveError> {
        let end = world.combat.physical_tick();
        let mut events = self.sealed_spends(world, dock_sequence, end)?;
        for hit in world.missiles.player_resource_hits() {
            let OrdinaryTarget::Player { front } = hit.target else {
                return Err(INVALID);
            };
            events.push((
                hit.physical_tick,
                1,
                hit.incident,
                Effect::Hit {
                    front,
                    damage: 16000,
                },
            ));
        }
        let identity = world.combat_identity().ok_or(INVALID)?;
        for (index, hit) in world.traffic.player_hits().iter().enumerate() {
            if hit.visit > identity.visit || hit.physical_tick > end {
                return Err(INVALID);
            }
            events.push((
                hit.physical_tick,
                2,
                index as u64,
                Effect::Hit {
                    front: hit.front,
                    damage: 1500,
                },
            ));
        }
        for (sequence, (clock, rate)) in
            world.modular.energy_changes(world)?.into_iter().enumerate()
        {
            if clock > end {
                return Err(INVALID);
            }
            events.push((clock, 0, sequence as u64 + 1, Effect::Recharge(rate)));
        }
        for (sequence, (clock, contact)) in world.emp.resource_effects().into_iter().enumerate() {
            if clock > end {
                return Err(INVALID);
            }
            events.push((clock, 3, sequence as u64, Effect::Emp { contact }));
        }
        for (ordinal, clock) in world.ecm.activation_clocks().enumerate() {
            if clock > end {
                return Err(INVALID);
            }
            // An ECM spend commutes with same-tick laser spends: both require positive
            // energy afterward and ECM changes neither heat nor the weapon cooldown.
            events.push((clock, 4, ordinal as u64, Effect::Ecm));
        }
        events.sort_by_key(|&(clock, phase, sequence, _)| (clock, phase, sequence));
        Ok(events)
    }
    /// Called once by the missile validator, after its genuine impact graph is checked.
    pub(super) fn validate_resources(
        &self,
        world: &World,
        dock_sequence: u64,
    ) -> Result<(), SaveError> {
        if !self.enabled {
            return if self.events.is_empty()
                && world.traffic.player_hits().is_empty()
                && world.mining.strokes().is_empty()
                && world.emp.resource_effects().is_empty()
                && world.ecm.activation_clocks().next().is_none()
            {
                Ok(())
            } else {
                Err(INVALID)
            };
        }
        let end = world.combat.physical_tick();
        let events = self.resource_events(world, dock_sequence)?;
        let pools = replay(&events, end)?;
        if pools.pulse != world.pulse || pools.shields != world.combat.shields() {
            return Err(INVALID);
        }
        Ok(())
    }
}
fn merge_missile_emissions(
    missiles: &mut std::iter::Peekable<impl Iterator<Item = (u64, u64)>>,
    mut next_emission: u64,
    event_clock: u64,
) -> Result<u64, SaveError> {
    while let Some(&(emission, clock)) = missiles.peek() {
        if emission < next_emission {
            return Err(INVALID);
        }
        if emission != next_emission {
            if clock < event_clock {
                return Err(INVALID);
            }
            break;
        }
        if clock > event_clock {
            return Err(INVALID);
        }
        let _ = missiles.next();
        next_emission += 1;
    }
    Ok(next_emission)
}

fn validate_laser_receipts(
    world: &World,
    end: u64,
    lasers: &[LaserSpend],
) -> Result<(), SaveError> {
    let spend = |emission| {
        lasers
            .binary_search_by_key(&emission, |shot| shot.emission)
            .ok()
            .and_then(|index| lasers.get(index))
            .ok_or(INVALID)
    };
    let ordinary = world.combat.last_impact().into_iter().chain(
        world
            .practice
            .iter()
            .chain(world.practice_history.iter())
            .filter_map(|practice| practice.ordinary_hit),
    );
    let player_beams = world.traffic.beam_receipts().iter().filter_map(|beam| {
        if let crate::traffic::BeamSource::Player { item, emission, .. } = beam.source {
            Some((beam, item, emission))
        } else {
            None
        }
    });
    validate_exclusive_laser_targets(
        ordinary.clone(),
        player_beams.clone().map(|(_, _, emission)| emission),
    )?;
    for hit in ordinary {
        if hit.weapon.is_laser() {
            let shot = spend(hit.emission)?;
            if (shot.clock, shot.weapon, shot.item) != (hit.physical_tick, hit.weapon, hit.item) {
                return Err(INVALID);
            }
        }
    }
    for (beam, item, emission) in player_beams {
        let shot = spend(emission)?;
        let elapsed = world
            .state
            .tick
            .checked_sub(beam.world_tick)
            .ok_or(INVALID)?;
        let since_shot = end.checked_sub(shot.clock).ok_or(INVALID)?;
        if (shot.weapon, shot.item) != (beam.weapon, item)
            || shot.clock > beam.world_tick
            || since_shot > elapsed
        {
            return Err(INVALID);
        }
    }
    Ok(())
}

// The active combat slot and a practice record may mirror the same station impact.
// A single discharge cannot also produce a player-to-NPC hit.
fn validate_exclusive_laser_targets(
    ordinary: impl IntoIterator<Item = super::OrdinaryImpact>,
    npc_emissions: impl IntoIterator<Item = u64>,
) -> Result<(), SaveError> {
    let mut station = BTreeMap::new();
    for hit in ordinary.into_iter().filter(|hit| hit.weapon.is_laser()) {
        if station
            .insert(hit.emission, hit)
            .is_some_and(|record| record != hit)
        {
            return Err(INVALID);
        }
    }
    if npc_emissions
        .into_iter()
        .any(|emission| station.contains_key(&emission))
    {
        return Err(INVALID);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum Effect {
    Berth,
    Laser(super::OrdinaryWeapon),
    Recharge(u16),
    Mining,
    Hit { front: bool, damage: u16 },
    Emp { contact: Option<(bool, u16, u16)> },
    Ecm,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Pools {
    pulse: PulseResources,
    shields: [u16; 2],
}
type ResourceEvent = (u64, u8, u64, Effect);
fn replay(events: &[ResourceEvent], end: u64) -> Result<Pools, SaveError> {
    let mut pools = Pools {
        pulse: PulseResources::default(),
        shields: [30000; 2],
    };
    let mut clock = 0;
    let mut recharge = 7;
    for &(at, phase, _, effect) in events {
        advance_with_rate(&mut pools, at.checked_sub(clock).ok_or(INVALID)?, recharge);
        match effect {
            Effect::Berth => {
                pools.pulse.replay_berth();
                pools.shields = [30000; 2];
            }
            Effect::Laser(weapon) => {
                pools.pulse = pools
                    .pulse
                    .prepare_laser_discharge(weapon)
                    .map_err(|_| INVALID)?;
            }
            Effect::Recharge(rate) => recharge = rate,
            Effect::Mining => {
                pools.pulse = pools
                    .pulse
                    .prepare_mining_discharge()
                    .map_err(|_| INVALID)?;
            }
            Effect::Ecm => {
                pools.pulse = pools.pulse.prepare_ecm_discharge().map_err(|_| INVALID)?
            }
            Effect::Emp { contact } => {
                pools.pulse = pools.pulse.prepare_emp_discharge().map_err(|_| INVALID)?;
                if let Some((front, power, expected)) = contact {
                    let shield = &mut pools.shields[usize::from(!front)];
                    let absorbed = (*shield).min(power);
                    if absorbed != expected {
                        return Err(INVALID);
                    }
                    *shield -= absorbed;
                }
            }
            Effect::Hit { front, damage } => {
                if phase == 2 && !pools.pulse.alive() {
                    return Err(INVALID);
                }
                let shield = &mut pools.shields[usize::from(!front)];
                let absorbed = (*shield).min(damage);
                *shield -= absorbed;
                pools.pulse.damage_energy(damage - absorbed);
            }
        }
        clock = at;
    }
    advance_with_rate(&mut pools, end.checked_sub(clock).ok_or(INVALID)?, recharge);
    Ok(pools)
}
fn advance_with_rate(pools: &mut Pools, elapsed: u64, recharge: u16) {
    pools.pulse.replay_advance_with_rate(elapsed, recharge);
    for shield in &mut pools.shields {
        *shield = u64::from(*shield)
            .saturating_add(elapsed.saturating_mul(3))
            .min(30000) as u16;
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
fn advance(pools: &mut Pools, elapsed: u64) {
    advance_with_rate(pools, elapsed, 7);
}

#[cfg(test)]
mod rapid_energy_tests;

//! Exact fresh-profile resource replay. No configurable opening balance or deficit scalar.
use super::{CombatIdentity, FlightMode, OrdinaryImpact, OrdinaryWeapon, World};
use crate::navigation::{DockContext, DockOrigin, SystemId};
use crate::save::{Reader, SaveError, Writer};
use std::collections::BTreeSet;
const INVALID: SaveError = SaveError::InvalidState("resource event replay or retained discharge");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Shot {
    pub(super) tick: u64,
    pub(super) physical: u64,
    pub(super) system: SystemId,
    pub(super) visit: u64,
    pub(super) item: u64,
    pub(super) weapon: OrdinaryWeapon,
    pub(super) emission: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Event {
    Launch {
        dock: DockContext,
        tick: u64,
    },
    Dock(DockContext),
    Arrival {
        tick: u64,
        system: SystemId,
    },
    Shot(Shot),
    Damage(u8),
    /// Root's sealed MC1 physical extraction path only; standalone source admits no work yield.
    Work {
        shot: Shot,
        asteroid: u8,
    },
}
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct History {
    events: Vec<Event>,
}
impl History {
    pub(super) fn push(&mut self, event: Event) {
        self.events.push(event);
    }
    fn shots(&self) -> impl Iterator<Item = Shot> + '_ {
        self.events.iter().filter_map(|e| match e {
            Event::Shot(s) | Event::Work { shot: s, .. } => Some(*s),
            _ => None,
        })
    }
    pub(super) fn rapid(&self) -> bool {
        self.shots().any(|s| s.weapon.cooldown_ticks() == 6)
    }
    pub(super) fn shot_at(&self, tick: u64) -> Option<Shot> {
        self.events.iter().find_map(|e| match e {
            Event::Shot(s) if s.tick == tick => Some(*s),
            _ => None,
        })
    }
    pub(super) fn contains(&self, hit: OrdinaryImpact) -> bool {
        self.events
            .iter()
            .filter_map(|e| {
                if let Event::Shot(s) = e {
                    Some(*s)
                } else {
                    None
                }
            })
            .any(|s| {
                s.tick == hit.tick
                    && s.physical == hit.physical_tick
                    && s.system == hit.attacker.system
                    && s.visit == hit.attacker.visit
                    && s.item == hit.item
                    && s.weapon == hit.weapon
                    && s.emission == hit.emission
            })
    }
    pub(super) fn validate(&self, world: &World) -> Result<(), SaveError> {
        let account = world.commander_account.as_ref().ok_or(INVALID)?;
        let navigation = world.navigation.ok_or(INVALID)?;
        let nav = navigation.validate_save(
            world.navigation_catalogue.as_ref().ok_or(INVALID)?,
            world.state.mode,
            world.state.tick,
            account.fuel_tenths(),
        )?;
        let mut replay = Replay::default();
        let mut emissions = BTreeSet::new();
        for p in world.missile_projectiles() {
            if p.origin.emission == 0
                || p.origin.emission > world.combat.emissions
                || !emissions.insert(p.origin.emission)
            {
                return Err(INVALID);
            }
        }
        let impacts = world.retained_resource_impacts();
        let mut seen_damage = BTreeSet::new();
        let mut work = [0_u64; 12];
        let mut last_resource_emission = 0;
        for event in &self.events {
            match *event {
                Event::Launch { dock, tick } => {
                    validate_dock(world, dock)?;
                    replay.until(tick, false)?;
                    if replay.airborne
                        || dock.sequence() != replay.docks
                        || dock.visit() != replay.visit
                        || dock.system() != replay.system
                        || dock.tick() > tick
                    {
                        return Err(INVALID);
                    }
                    replay.airborne = true;
                    replay.launch_tick = tick;
                    replay.launches += 1;
                    replay.visit = replay.visit.checked_add(1).ok_or(INVALID)?;
                    replay.recharge = world
                        .replay_energy_recharge(tick, replay.visit)
                        .ok_or(INVALID)?;
                    replay.command_tick = None;
                }
                Event::Dock(dock) => {
                    validate_dock(world, dock)?;
                    if !replay.airborne || dock.tick() <= replay.tick {
                        return Err(INVALID);
                    }
                    replay.until(dock.tick(), true)?;
                    if dock.origin() != DockOrigin::PhysicalEntry
                        || dock.sequence() != replay.docks + 1
                        || dock.visit() != replay.visit
                        || dock.system() != replay.system
                    {
                        return Err(INVALID);
                    }
                    replay.airborne = false;
                    replay.docks += 1;
                }
                Event::Arrival { tick, system } => {
                    if !replay.airborne || tick <= replay.tick {
                        return Err(INVALID);
                    }
                    replay.until(tick, false)?;
                    let distance = world
                        .navigation_catalogue
                        .as_ref()
                        .ok_or(INVALID)?
                        .distance_ly(replay.system, system)
                        .map_err(|_| INVALID)?;
                    if distance <= 0.0 || distance > crate::navigation::STOCK_JUMP_REACH_LY {
                        return Err(INVALID);
                    }
                    replay.spent = replay
                        .spent
                        .checked_add(u64::from(
                            crate::navigation::stock_jump_fuel_cost(distance)
                                .map_err(|_| INVALID)?,
                        ))
                        .ok_or(INVALID)?;
                    replay.system = system;
                    replay.visit = replay.visit.checked_add(1).ok_or(INVALID)?;
                    replay.jumps += 1;
                }
                Event::Shot(shot) | Event::Work { shot, .. } => {
                    replay.until(shot.tick, false)?;
                    let identity = CombatIdentity {
                        owner: nav.owner,
                        hull: nav.hull,
                        system: shot.system,
                        visit: shot.visit,
                    };
                    if !replay.airborne
                        || shot.tick <= replay.launch_tick
                        || shot.physical != replay.physical
                        || shot.system != replay.system
                        || shot.visit != replay.visit
                        || !shot.weapon.is_laser()
                        || world.owned_front_laser_at(shot.item, shot.tick, identity)
                            != Some(shot.weapon)
                        || shot.emission <= last_resource_emission
                        || shot.emission > world.combat.emissions
                        || !emissions.insert(shot.emission)
                    {
                        return Err(INVALID);
                    }
                    last_resource_emission = shot.emission;
                    replay.pool = if let Event::Work { asteroid, .. } = *event {
                        if shot.weapon != OrdinaryWeapon::FrontMiningLaser {
                            return Err(INVALID);
                        }
                        let count = work.get_mut(usize::from(asteroid)).ok_or(INVALID)?;
                        *count += 1;
                        replay
                            .pool
                            .prepare_mining_discharge()
                            .map_err(|_| INVALID)?
                    } else {
                        replay
                            .pool
                            .prepare_laser_discharge(shot.weapon)
                            .map_err(|_| INVALID)?
                    };
                    replay.command_tick = Some(shot.tick);
                }
                Event::Damage(ordinal) => {
                    let hit = impacts
                        .iter()
                        .find(|(i, _)| *i == ordinal)
                        .map(|(_, h)| *h)
                        .ok_or(INVALID)?;
                    if replay.command_tick == Some(hit.tick)
                        || !replay.pool.alive()
                        || !replay.airborne
                        || !seen_damage.insert(ordinal)
                    {
                        return Err(INVALID);
                    }
                    replay.until(hit.tick, false)?;
                    if hit.physical_tick != replay.physical
                        || hit.attacker.system != replay.system
                        || hit.attacker.visit != replay.visit
                        || hit.weapon != OrdinaryWeapon::StandardMissile
                    {
                        return Err(INVALID);
                    }
                    replay
                        .shields
                        .resolve_impact(&mut replay.pool, hit.target, hit.weapon);
                }
            }
        }
        replay.until(world.state.tick, false)?;
        let expected_mode = if replay.airborne {
            if world.state.tick == replay.launch_tick {
                FlightMode::Launching
            } else {
                FlightMode::Flight
            }
        } else {
            FlightMode::Docked
        };
        if world.state.mode != expected_mode
            || world.pulse != replay.pool
            || world.combat.shield_resources() != replay.shields.shield_resources()
            || world.combat.physical_tick() != replay.physical
            || nav.launches != replay.launches
            || nav.jumps != replay.jumps
            || nav.dock_sequence != replay.docks
            || nav.visit != replay.visit
            || nav.fuel_spent != replay.spent
            || navigation.snapshot(account.fuel_tenths()).system != replay.system
            || emissions.len() as u64 != world.combat.emissions
            || seen_damage.len() != impacts.len()
            || world
                .actual_resource_work_counts()
                .map_or(work != [0; 12], |actual| actual != work)
        {
            return Err(INVALID);
        }
        Ok(())
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.u32(self.events.len() as u32);
        for e in &self.events {
            match *e {
                Event::Launch { dock, tick } => {
                    w.u8(0);
                    dock.encode_save(w);
                    w.u64(tick);
                }
                Event::Dock(d) => {
                    w.u8(1);
                    d.encode_save(w);
                }
                Event::Arrival { tick, system } => {
                    w.u8(2);
                    w.u64(tick);
                    system_write(w, system);
                }
                Event::Shot(s) => {
                    w.u8(3);
                    s.encode(w);
                }
                Event::Damage(i) => {
                    w.u8(4);
                    w.u8(i);
                }
                Event::Work { shot, asteroid } => {
                    w.u8(5);
                    shot.encode(w);
                    w.u8(asteroid);
                }
            }
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let n = r.u32()? as usize;
        if n > crate::save::MAX_SAVE_BYTES / 2 {
            return Err(SaveError::TooLarge);
        }
        let mut events = Vec::new();
        for _ in 0..n {
            events.push(match r.u8()? {
                0 => Event::Launch {
                    dock: DockContext::decode_save(r)?,
                    tick: r.u64()?,
                },
                1 => Event::Dock(DockContext::decode_save(r)?),
                2 => Event::Arrival {
                    tick: r.u64()?,
                    system: system_read(r)?,
                },
                3 => Event::Shot(Shot::decode(r)?),
                4 => Event::Damage(r.u8()?),
                5 => Event::Work {
                    shot: Shot::decode(r)?,
                    asteroid: r.u8()?,
                },
                _ => return Err(SaveError::InvalidEncoding),
            });
        }
        Ok(Self { events })
    }
}
fn validate_dock(world: &World, dock: DockContext) -> Result<(), SaveError> {
    let current = world.combat_identity().ok_or(INVALID)?;
    if dock.owner().seed() != current.owner
        || dock.ship().ordinal() != current.hull
        || dock.ship().owner() != dock.owner()
        || dock.tick() > world.state.tick
        || (dock.sequence() == 1
            && (dock.origin() != DockOrigin::InitialStart
                || dock.visit() != 1
                || dock.system() != SystemId::AVENOR
                || dock.tick() != 0))
        || (dock.sequence() > 1 && (dock.origin() != DockOrigin::PhysicalEntry || dock.tick() == 0))
        || world
            .current_dock_context()
            .is_some_and(|d| d.sequence() == dock.sequence() && d != dock)
    {
        return Err(INVALID);
    }
    world
        .navigation_catalogue
        .as_ref()
        .ok_or(INVALID)?
        .system(dock.system())
        .map_err(|_| INVALID)?;
    Ok(())
}
struct Replay {
    tick: u64,
    physical: u64,
    airborne: bool,
    launch_tick: u64,
    launches: u64,
    jumps: u64,
    docks: u64,
    visit: u64,
    spent: u64,
    system: SystemId,
    recharge: u16,
    pool: super::weapons::PulseResources,
    shields: super::combat::State,
    command_tick: Option<u64>,
}
impl Default for Replay {
    fn default() -> Self {
        Self {
            tick: 0,
            physical: 0,
            airborne: false,
            launch_tick: 0,
            launches: 0,
            jumps: 0,
            docks: 1,
            visit: 1,
            spent: 0,
            system: SystemId::AVENOR,
            recharge: super::weapons::ENERGY_RECHARGE_HUNDREDTHS,
            pool: super::weapons::PulseResources::default(),
            shields: super::combat::State::default(),
            command_tick: None,
        }
    }
}
impl Replay {
    fn until(&mut self, tick: u64, dock: bool) -> Result<(), SaveError> {
        let n = tick.checked_sub(self.tick).ok_or(INVALID)?;
        if self.airborne {
            if n > 0 && !self.pool.alive() {
                return Err(INVALID);
            }
            self.physical = self.physical.checked_add(n).ok_or(INVALID)?;
            self.pool.replay_advance(n, self.recharge, dock);
            self.shields.replay_advance(n, dock);
        } else if dock {
            return Err(INVALID);
        }
        if n > 0 {
            self.command_tick = None;
        }
        self.tick = tick;
        Ok(())
    }
}
fn system_write(w: &mut Writer, s: SystemId) {
    w.u8(s.galaxy);
    w.u16(s.system);
}
fn system_read(r: &mut Reader<'_>) -> Result<SystemId, SaveError> {
    Ok(SystemId {
        galaxy: r.u8()?,
        system: r.u16()?,
    })
}
impl Shot {
    fn encode(self, w: &mut Writer) {
        w.u64(self.tick);
        w.u64(self.physical);
        system_write(w, self.system);
        w.u64(self.visit);
        w.u64(self.item);
        w.u8(self.weapon.tag());
        w.u64(self.emission);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            tick: r.u64()?,
            physical: r.u64()?,
            system: system_read(r)?,
            visit: r.u64()?,
            item: r.u64()?,
            weapon: OrdinaryWeapon::from_tag(r.u8()?)?,
            emission: r.u64()?,
        })
    }
}
impl World {
    pub(super) fn rapid_resource_history(&self) -> bool {
        self.resource_history.as_ref().is_some_and(History::rapid)
    }
    pub(super) fn resource_cooldown_at(&self, tick: u64) -> Option<u8> {
        self.resource_history
            .as_ref()
            .map_or(Some(super::PULSE_COOLDOWN_TICKS), |h| {
                h.shot_at(tick).map(|s| s.weapon.cooldown_ticks())
            })
    }
    pub(super) fn resource_hit_matches(&self, hit: OrdinaryImpact) -> bool {
        self.resource_history
            .as_ref()
            .is_none_or(|h| h.contains(hit))
    }
    pub(super) fn resource_quiet_anchor(&self, tick: u64, anchor: u64) -> bool {
        if !self.rapid_resource_history() {
            return self.pulse.fits_quiet_anchor(tick, anchor);
        }
        anchor <= tick
            && self
                .resource_history
                .as_ref()
                .is_some_and(|h| h.shots().all(|s| s.tick <= anchor))
    }
    /// This isolated crate has no MC1 mining owner. Root must bind its actual per-asteroid
    /// depletion counters here; work records are otherwise categorically refused.
    pub(super) fn actual_resource_work_counts(&self) -> Option<[u64; 12]> {
        None
    }
}
#[cfg(test)]
mod tests;

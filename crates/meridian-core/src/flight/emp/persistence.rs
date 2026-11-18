//! Current-format EMP state only. Decode neither purchases hardware nor charges a bank.
use super::*;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::traffic::{NpcId, NpcPhase};
impl State {
    pub(in crate::flight) fn encode(&self, w: &mut Writer) {
        w.u16(self.equipment.len() as u16);
        for e in self.equipment.iter() {
            w.bool(e.secret);
            match e.action {
                EmpEquipmentAction::Purchase => w.u8(0),
                EmpEquipmentAction::Fit { item } => {
                    w.u8(1);
                    w.u64(item);
                }
                EmpEquipmentAction::Store { item } => {
                    w.u8(2);
                    w.u64(item);
                }
                EmpEquipmentAction::Upgrade { item, axis } => {
                    w.u8(3);
                    w.u64(item);
                    w.u8(match axis {
                        EmpAxis::Power => 0,
                        EmpAxis::Capacity => 1,
                        EmpAxis::Recharge => 2,
                    });
                }
            }
            e.dock.encode_save(w);
            w.u64(e.tick);
            w.u64(e.revision);
        }
        w.u64(self.meter_clock);
        w.u64(self.bank_clock);
        w.bool(self.main_present);
        w.u32(self.meter.main);
        w.u32(self.meter.bank);
        w.option(self.meter.disabled_at.as_ref(), |w, n| w.u64(*n));
        for n in [
            self.meter.generated,
            self.meter.discarded,
            self.meter.main_debited,
            self.meter.bank_gained,
            self.meter.bank_spent,
            self.meter.pulse_drained,
        ] {
            w.u64(n);
        }
        w.option(self.charge.as_ref(), |w, c| c.encode(w));
        w.u8(self.pulses.len() as u8);
        for p in self.pulses.iter() {
            p.origin.encode(w);
            p.charge.encode(w);
            p.release.encode(w);
            w.u8(p.contacts.len() as u8);
            for c in &p.contacts {
                c.encode(w);
            }
            w.u16(p.player_shield_absorbed);
            w.u32(p.player_electrical_drained);
        }
        w.u8(self.episodes.len() as u8);
        for e in self.episodes.iter() {
            e.origin.encode(w);
            e.victim.encode(w);
            w.u64(e.disabled_at);
            w.option(e.recovered_at.as_ref(), |w, (clock, tick)| {
                w.u64(*clock);
                w.u64(*tick);
            });
        }
    }
    pub(in crate::flight) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let n = usize::from(r.u16()?);
        if n > MAX_EQUIPMENT_EVENTS {
            return Err(SaveError::TooLarge);
        }
        let mut equipment = Vec::new();
        for _ in 0..n {
            let secret = r.bool()?;
            let action = match r.u8()? {
                0 => EmpEquipmentAction::Purchase,
                1 => EmpEquipmentAction::Fit { item: r.u64()? },
                2 => EmpEquipmentAction::Store { item: r.u64()? },
                3 => EmpEquipmentAction::Upgrade {
                    item: r.u64()?,
                    axis: match r.u8()? {
                        0 => EmpAxis::Power,
                        1 => EmpAxis::Capacity,
                        2 => EmpAxis::Recharge,
                        _ => return Err(INVALID),
                    },
                },
                _ => return Err(INVALID),
            };
            equipment.push(EquipmentEvent {
                secret,
                action,
                dock: DockContext::decode_save(r)?,
                tick: r.u64()?,
                revision: r.u64()?,
            });
        }
        let meter_clock = r.u64()?;
        let bank_clock = r.u64()?;
        let main_present = r.bool()?;
        let meter = meter::Meter {
            main: r.u32()?,
            bank: r.u32()?,
            disabled_at: r.option(|r| r.u64())?,
            generated: r.u64()?,
            discarded: r.u64()?,
            main_debited: r.u64()?,
            bank_gained: r.u64()?,
            bank_spent: r.u64()?,
            pulse_drained: r.u64()?,
        };
        let charge = r.option(runtime::Charge::decode)?;
        let n = usize::from(r.u8()?);
        if n > MAX_PULSES {
            return Err(SaveError::TooLarge);
        }
        let mut pulses = Vec::new();
        for _ in 0..n {
            let origin = EmpOrigin::decode(r)?;
            let charge = runtime::Charge::decode(r)?;
            let release = scene::Scene::decode(r)?;
            let n = usize::from(r.u8()?);
            if n > crate::traffic::MAX_TRAFFIC_ACTORS + 1 {
                return Err(INVALID);
            }
            let contacts = (0..n)
                .map(|_| scene::Contact::decode(r))
                .collect::<Result<Vec<_>, _>>()?;
            pulses.push(runtime::Pulse {
                origin,
                charge,
                release,
                contacts,
                player_shield_absorbed: r.u16()?,
                player_electrical_drained: r.u32()?,
            });
        }
        let n = usize::from(r.u8()?);
        if n > MAX_PULSES * crate::traffic::MAX_TRAFFIC_ACTORS {
            return Err(SaveError::TooLarge);
        }
        let mut episodes = Vec::new();
        for _ in 0..n {
            episodes.push(runtime::Episode {
                origin: EmpOrigin::decode(r)?,
                victim: NpcId::decode(r)?,
                disabled_at: r.u64()?,
                recovered_at: r.option(|r| Ok((r.u64()?, r.u64()?)))?,
            });
        }
        Ok(Self {
            equipment: Arc::new(equipment),
            meter,
            meter_clock,
            bank_clock,
            main_present,
            charge,
            pulses: Arc::new(pulses),
            episodes: Arc::new(episodes),
            held_tick: None,
            posted_tick: None,
        })
    }
}
impl runtime::Charge {
    fn encode(&self, w: &mut Writer) {
        w.u64(self.item);
        self.target.encode(w);
        self.start.encode(w);
        w.u8(match self.phase {
            EmpChargePhase::Charging => 0,
            EmpChargePhase::Cancelled => 1,
            EmpChargePhase::Released => 2,
        });
        w.option(self.ended_at.as_ref(), |w, n| w.u64(*n));
        w.option(self.pulse.as_ref(), |w, n| w.u16(*n));
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            item: r.u64()?,
            target: NpcId::decode(r)?,
            start: scene::Scene::decode(r)?,
            phase: match r.u8()? {
                0 => EmpChargePhase::Charging,
                1 => EmpChargePhase::Cancelled,
                2 => EmpChargePhase::Released,
                _ => return Err(INVALID),
            },
            ended_at: r.option(|r| r.u64())?,
            pulse: r.option(|r| r.u16())?,
        })
    }
}
impl scene::Scene {
    fn encode(&self, w: &mut Writer) {
        identity(w, self.owner);
        w.u64(self.ordinary_cursor);
        w.u64(self.local_tick);
        w.u64(self.world_tick);
        w.u64(self.physical_tick);
        rigid(w, self.player);
        w.u8(self.bodies.len() as u8);
        for b in &self.bodies {
            b.id.encode(w);
            rigid(w, b.pose);
            w.u8(phase(b.phase));
        }
        w.u16(self.missiles.len() as u16);
        for (id, p) in &self.missiles {
            id.encode(w);
            rigid(w, *p);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let owner = read_identity(r)?;
        let ordinary_cursor = r.u64()?;
        let local_tick = r.u64()?;
        let world_tick = r.u64()?;
        let physical_tick = r.u64()?;
        let player = read_rigid(r)?;
        let n = usize::from(r.u8()?);
        if n > crate::traffic::MAX_TRAFFIC_ACTORS {
            return Err(INVALID);
        }
        let mut bodies = Vec::new();
        for _ in 0..n {
            bodies.push(scene::Body {
                id: NpcId::decode(r)?,
                pose: read_rigid(r)?,
                phase: read_phase(r.u8()?)?,
            });
        }
        let n = usize::from(r.u16()?);
        if n > 256 {
            return Err(INVALID);
        }
        let mut missiles = Vec::new();
        for _ in 0..n {
            missiles.push((super::super::ProjectileId::decode(r)?, read_rigid(r)?));
        }
        Ok(Self {
            owner,
            ordinary_cursor,
            local_tick,
            world_tick,
            physical_tick,
            player,
            bodies,
            missiles,
        })
    }
}
impl scene::Contact {
    fn encode(self, w: &mut Writer) {
        match self.victim {
            scene::Victim::Npc(id) => {
                w.u8(0);
                id.encode(w);
            }
            scene::Victim::Player => w.u8(1),
        }
        point(w, self.point);
        w.bool(self.front);
        w.u16(self.power_hundredths);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            victim: match r.u8()? {
                0 => scene::Victim::Npc(NpcId::decode(r)?),
                1 => scene::Victim::Player,
                _ => return Err(INVALID),
            },
            point: read_point(r)?,
            front: r.bool()?,
            power_hundredths: r.u16()?,
        })
    }
}
fn identity(w: &mut Writer, id: super::super::CombatIdentity) {
    w.u64(id.owner);
    w.u64(id.hull);
    w.u8(id.system.galaxy);
    w.u16(id.system.system);
    w.u64(id.visit);
}
fn read_identity(r: &mut Reader<'_>) -> Result<super::super::CombatIdentity, SaveError> {
    Ok(super::super::CombatIdentity {
        owner: r.u64()?,
        hull: r.u64()?,
        system: crate::navigation::SystemId {
            galaxy: r.u8()?,
            system: r.u16()?,
        },
        visit: r.u64()?,
    })
}
fn point(w: &mut Writer, p: Vec3) {
    for n in [p.x, p.y, p.z] {
        w.u64(n.to_bits());
    }
}
fn read_point(r: &mut Reader<'_>) -> Result<Vec3, SaveError> {
    Ok(Vec3::new(
        f64::from_bits(r.u64()?),
        f64::from_bits(r.u64()?),
        f64::from_bits(r.u64()?),
    ))
}
fn rigid(w: &mut Writer, p: RigidPose) {
    point(w, p.position);
    for v in [p.orientation.x, p.orientation.y, p.orientation.z] {
        point(w, v);
    }
}
fn read_rigid(r: &mut Reader<'_>) -> Result<RigidPose, SaveError> {
    Ok(RigidPose {
        position: read_point(r)?,
        orientation: Basis3 {
            x: read_point(r)?,
            y: read_point(r)?,
            z: read_point(r)?,
        },
    })
}
fn phase(p: NpcPhase) -> u8 {
    match p {
        NpcPhase::Departing => 0,
        NpcPhase::Outbound => 1,
        NpcPhase::ReturnAlign => 2,
        NpcPhase::Inbound => 3,
        NpcPhase::Complete => 4,
        NpcPhase::Hunting => 5,
        NpcPhase::Retreating => 6,
        NpcPhase::Destroyed => 7,
        NpcPhase::Approach => 8,
        NpcPhase::FinalAlign => 9,
    }
}
fn read_phase(n: u8) -> Result<NpcPhase, SaveError> {
    Ok(match n {
        0 => NpcPhase::Departing,
        1 => NpcPhase::Outbound,
        2 => NpcPhase::ReturnAlign,
        3 => NpcPhase::Inbound,
        4 => NpcPhase::Complete,
        5 => NpcPhase::Hunting,
        6 => NpcPhase::Retreating,
        7 => NpcPhase::Destroyed,
        8 => NpcPhase::Approach,
        9 => NpcPhase::FinalAlign,
        _ => return Err(INVALID),
    })
}

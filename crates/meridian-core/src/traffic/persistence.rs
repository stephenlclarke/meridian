use super::*;
use crate::save::{Reader, Writer};
impl State {
    pub(crate) fn validate(
        &self,
        seed: u64,
        world_tick: u64,
        player_emissions: u64,
        data: &OriginalStationData,
        markets: &StationMarkets,
    ) -> Result<(), SaveError> {
        if !self.enabled {
            return if self.seed == 0
                && self.clocks == [0; 2]
                && self.actors == [None; MAX_TRAFFIC_ACTORS]
                && self.shots.is_empty()
            {
                Ok(())
            } else {
                Err(INVALID)
            };
        }
        if self.seed != seed
            || seed == 0
            || u128::from(self.clocks[0]) + u128::from(self.clocks[1]) > u128::from(world_tick)
            || self.shots.len() > MAX_TRAFFIC_SHOTS
        {
            return Err(INVALID);
        }
        for (index, a) in self.actors.iter().enumerate() {
            let Some(a) = a else {
                continue;
            };
            self.validate_identity(*a, index, seed, world_tick, data)?;
            Self::validate_economy(*a, seed, world_tick, markets)?;
            self.validate_resources(*a)?;
        }
        self.validate_separation(data)?;
        self.validate_shots(world_tick, player_emissions, data)
    }
    fn validate_identity(
        &self,
        a: Actor,
        index: usize,
        seed: u64,
        world_tick: u64,
        data: &OriginalStationData,
    ) -> Result<(), SaveError> {
        let si = index / 2;
        let serial = (index % 2 + 1) as u8;
        let system = if si == 0 {
            SystemId::AVENOR
        } else {
            SystemId::RENAS
        };
        if a.id
            != (NpcId {
                profile: seed,
                system,
                serial,
            })
            || a.born_world_tick == 0
            || a.born_world_tick > world_tick
            || a.birth_clock > self.clocks[si]
            || a.birth_clock < if serial == 1 { 480 } else { 900 }
            || !finite(a.position)
            || [a.position.x, a.position.y, a.position.z]
                .into_iter()
                .any(|n| n.abs() > 1e12)
            || !a.yaw.is_finite()
            || a.yaw.abs() > core::f64::consts::PI + 1e-9
            || !a.speed.is_finite()
            || !(0.0..=34.3).contains(&a.speed)
            || !a.pose().orientation.is_valid_rotation()
        {
            return Err(INVALID);
        }
        if a.sold != a.berth.is_some() {
            return Err(INVALID);
        }
        if let Some(receipt) = a.berth {
            let before = pose(receipt.before, receipt.yaw);
            if !finite(receipt.before)
                || !receipt.yaw.is_finite()
                || !receipt.speed.is_finite()
                || !(0.0..=34.3).contains(&receipt.speed)
                || !receipt.angular.is_finite()
                || !(0.0..=0.65 / 60.0 + 1e-9).contains(&receipt.angular)
                || a.speed != 0.0
                || !data.crosses_aperture(before, a.pose())
                || !data.entry_clear(before, a.pose(), receipt.speed * 18.0, 0.0, receipt.angular)
            {
                return Err(INVALID);
            }
        }
        if (a.phase == NpcPhase::Destroyed) != (a.resources.energy == 0) {
            return Err(INVALID);
        }
        if a.solid()
            && (length(sub(a.position, Vec3::new(0.0, 0.0, 16000.0)))
                <= data.player_clearance_radius() * 2.0 + 64.0
                || !data.motion_clear(a.pose(), a.pose(), 0.0))
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_economy(
        a: Actor,
        seed: u64,
        world_tick: u64,
        markets: &StationMarkets,
    ) -> Result<(), SaveError> {
        let serial = a.id.serial;
        let system = a.id.system;
        if serial == 1 {
            let (id, units) = manifest(seed, system);
            let site = site(system).ok_or(INVALID)?;
            let q = markets.commodity_quote(site, id);
            let expected = CAPITAL
                .checked_sub(q.ask_tenths.checked_mul(u32::from(units)).ok_or(INVALID)?)
                .and_then(|n| {
                    n.checked_add(if a.sold {
                        q.bid_tenths * u32::from(units)
                    } else {
                        0
                    })
                })
                .ok_or(INVALID)?;
            if a.manifest != id
                || a.units != units
                || a.wallet != expected
                || a.sold != (a.phase == NpcPhase::Complete)
                || a.sold != a.sold_world_tick.is_some()
                || a.sold_world_tick
                    .is_some_and(|t| t < a.born_world_tick || t > world_tick)
                || matches!(a.phase, NpcPhase::Hunting | NpcPhase::Retreating)
            {
                return Err(INVALID);
            }
        } else if !pirate_present(seed, system)
            || a.units != 0
            || a.wallet != 0
            || a.sold
            || a.sold_world_tick.is_some()
            || a.manifest != CommodityId::Minerals
            || !matches!(
                a.phase,
                NpcPhase::Hunting | NpcPhase::Retreating | NpcPhase::Destroyed
            )
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_resources(&self, a: Actor) -> Result<(), SaveError> {
        let system = a.id.system;
        let si = system_index(system).ok_or(INVALID)?;
        if a.aggressor.is_some_and(|id| {
            self.actor_index(id).is_none()
                || id.system != system
                || id == a.id
                || !self
                    .shots
                    .iter()
                    .any(|shot| shot.victim == a.id && shot.source == BeamSource::Npc(id))
        }) {
            return Err(INVALID);
        }
        let incoming = self.shots.iter().filter(|s| s.victim == a.id).count();
        let outgoing = self
            .shots
            .iter()
            .filter(|s| s.source == BeamSource::Npc(a.id))
            .count();
        let last_hit = self
            .shots
            .iter()
            .rfind(|s| s.victim == a.id)
            .map(|s| s.local_tick);
        let end = if a.phase == NpcPhase::Destroyed {
            last_hit.ok_or(INVALID)?
        } else {
            self.clocks[si]
        };
        let age = end
            .checked_sub(a.birth_clock)
            .and_then(|v| v.checked_add(1))
            .ok_or(INVALID)?;
        a.resources.validate(age, outgoing, incoming)?;
        let last = self
            .shots
            .iter()
            .rfind(|s| s.source == BeamSource::Npc(a.id));
        let expected = last.map_or(0, |s| {
            18_u64.saturating_sub(end.saturating_sub(s.local_tick)) as u8
        });
        if a.resources.cooldown != expected {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_separation(&self, data: &OriginalStationData) -> Result<(), SaveError> {
        for (index, a) in self.actors.iter().enumerate() {
            let Some(a) = a.filter(|a| a.solid()) else {
                continue;
            };
            if self.actors.iter().skip(index + 1).flatten().any(|b| {
                b.solid()
                    && b.id.system == a.id.system
                    && length(sub(a.position, b.position))
                        <= data.player_clearance_radius() * 2.0 + 1.0
            }) {
                return Err(INVALID);
            }
        }
        Ok(())
    }
    fn validate_shots(
        &self,
        world_tick: u64,
        player_emissions: u64,
        data: &OriginalStationData,
    ) -> Result<(), SaveError> {
        let mut player_emission = 0;
        let mut prior_world = 0;
        let mut last_npc = [None; MAX_TRAFFIC_ACTORS];
        for shot in self.shots.iter() {
            let si = system_index(shot.victim.system).ok_or(INVALID)?;
            let victim = self.actor_index(shot.victim).ok_or(INVALID)?;
            if shot.world_tick == 0
                || shot.world_tick > world_tick
                || shot.world_tick < prior_world
                || shot.local_tick > self.clocks[si]
                || !finite(shot.origin)
                || !finite(shot.direction)
                || !finite(shot.point)
                || (shot.direction.dot(shot.direction) - 1.0).abs() > 1e-6
                || !shot.victim_pose.orientation.is_valid_rotation()
                || !finite(shot.victim_pose.position)
            {
                return Err(INVALID);
            }
            prior_world = shot.world_tick;
            let distance = data
                .player_beam_distance(shot.victim_pose, shot.origin, shot.direction, 6000.0)
                .map_err(|_| INVALID)?
                .ok_or(INVALID)?;
            if data
                .beam_block_distance(data.station_pose(), shot.origin, shot.direction, 6000.0)
                .map_err(|_| INVALID)?
                .is_some_and(|wall| wall <= distance)
            {
                return Err(INVALID);
            }
            if length(sub(
                add(shot.origin, scale(shot.direction, distance)),
                shot.point,
            )) > 1e-6
                || shot.front
                    != (shot
                        .victim_pose
                        .orientation
                        .z
                        .dot(sub(shot.origin, shot.victim_pose.position))
                        >= 0.0)
            {
                return Err(INVALID);
            }
            if shot.local_tick < self.actors[victim].ok_or(INVALID)?.birth_clock
                || shot.world_tick < self.actors[victim].ok_or(INVALID)?.born_world_tick
            {
                return Err(INVALID);
            }
            match shot.source {
                BeamSource::Npc(id) => {
                    let index = self.actor_index(id).ok_or(INVALID)?;
                    if id.serial != 2
                        || shot.victim.serial != 1
                        || id.system != shot.victim.system
                        || shot.local_tick < self.actors[index].ok_or(INVALID)?.birth_clock
                        || last_npc[index].is_some_and(|tick: u64| {
                            tick.checked_add(18)
                                .is_none_or(|ready| shot.local_tick < ready)
                        })
                    {
                        return Err(INVALID);
                    }
                    last_npc[index] = Some(shot.local_tick);
                }
                BeamSource::Player {
                    owner,
                    hull,
                    item,
                    emission,
                } => {
                    if owner != self.seed
                        || hull != owner.checked_add(1).ok_or(INVALID)?
                        || item == 0
                        || emission <= player_emission
                        || emission > player_emissions
                    {
                        return Err(INVALID);
                    }
                    player_emission = emission;
                }
            }
        }
        Ok(())
    }
    pub(crate) fn encode(&self, w: &mut Writer) {
        w.bool(self.enabled);
        w.u64(self.seed);
        for clock in self.clocks {
            w.u64(clock);
        }
        for a in &self.actors {
            w.option(a.as_ref(), |w, a| a.encode(w));
        }
        w.u16(self.shots.len() as u16);
        for s in self.shots.iter() {
            s.encode(w);
        }
    }
    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let enabled = r.bool()?;
        let seed = r.u64()?;
        let clocks = [r.u64()?, r.u64()?];
        let actors = r.array(|r| r.option(Actor::decode))?;
        let count = usize::from(r.u16()?);
        if count > MAX_TRAFFIC_SHOTS {
            return Err(INVALID);
        }
        let mut shots = Vec::with_capacity(count);
        for _ in 0..count {
            shots.push(TrafficBeam::decode(r)?);
        }
        Ok(Self {
            enabled,
            seed,
            clocks,
            actors,
            shots: Arc::new(shots),
        })
    }
    #[cfg(test)]
    pub(crate) fn legacy_empty(&self) -> bool {
        self.actors.iter().all(Option::is_none) && self.shots.is_empty()
    }
}
impl NpcId {
    fn encode(self, w: &mut Writer) {
        w.u64(self.profile);
        w.u8(self.system.galaxy);
        w.u16(self.system.system);
        w.u8(self.serial);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            profile: r.u64()?,
            system: SystemId {
                galaxy: r.u8()?,
                system: r.u16()?,
            },
            serial: r.u8()?,
        })
    }
}
fn point(w: &mut Writer, v: Vec3) {
    for n in [v.x, v.y, v.z] {
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
impl Actor {
    fn encode(self, w: &mut Writer) {
        self.id.encode(w);
        w.u8(match self.phase {
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
        });
        point(w, self.position);
        w.u64(self.yaw.to_bits());
        w.u64(self.speed.to_bits());
        w.u64(self.birth_clock);
        w.u64(self.born_world_tick);
        w.option(self.sold_world_tick.as_ref(), |w, t| w.u64(*t));
        w.u8(self.manifest as u8);
        w.u8(self.units);
        w.u32(self.wallet);
        w.bool(self.sold);
        self.resources.encode(w);
        w.option(self.aggressor.as_ref(), |w, id| id.encode(w));
        w.option(self.berth.as_ref(), |w, receipt| {
            point(w, receipt.before);
            for n in [receipt.yaw, receipt.speed, receipt.angular] {
                w.u64(n.to_bits());
            }
        });
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            id: NpcId::decode(r)?,
            phase: match r.u8()? {
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
            },
            position: read_point(r)?,
            yaw: f64::from_bits(r.u64()?),
            speed: f64::from_bits(r.u64()?),
            birth_clock: r.u64()?,
            born_world_tick: r.u64()?,
            sold_world_tick: r.option(|r| r.u64())?,
            manifest: CommodityId::from_index(usize::from(r.u8()?)).ok_or(INVALID)?,
            units: r.u8()?,
            wallet: r.u32()?,
            sold: r.bool()?,
            resources: Resources::decode(r)?,
            aggressor: r.option(NpcId::decode)?,
            berth: r.option(|r| {
                Ok(BerthReceipt {
                    before: read_point(r)?,
                    yaw: f64::from_bits(r.u64()?),
                    speed: f64::from_bits(r.u64()?),
                    angular: f64::from_bits(r.u64()?),
                })
            })?,
        })
    }
}
impl Resources {
    fn encode(self, w: &mut Writer) {
        for n in [self.energy, self.front, self.rear, self.heat] {
            w.u16(n);
        }
        w.u8(self.cooldown);
        w.u16(self.shots);
        w.u16(self.incoming);
        for n in [
            self.energy_regen,
            self.front_regen,
            self.rear_regen,
            self.heat_cooled,
            self.absorbed_front,
            self.absorbed_rear,
            self.hull_damage,
            self.overkill,
        ] {
            w.u64(n);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            energy: r.u16()?,
            front: r.u16()?,
            rear: r.u16()?,
            heat: r.u16()?,
            cooldown: r.u8()?,
            shots: r.u16()?,
            incoming: r.u16()?,
            energy_regen: r.u64()?,
            front_regen: r.u64()?,
            rear_regen: r.u64()?,
            heat_cooled: r.u64()?,
            absorbed_front: r.u64()?,
            absorbed_rear: r.u64()?,
            hull_damage: r.u64()?,
            overkill: r.u64()?,
        })
    }
}
impl TrafficBeam {
    fn encode(self, w: &mut Writer) {
        match self.source {
            BeamSource::Npc(id) => {
                w.u8(0);
                id.encode(w);
            }
            BeamSource::Player {
                owner,
                hull,
                item,
                emission,
            } => {
                w.u8(1);
                for n in [owner, hull, item, emission] {
                    w.u64(n);
                }
            }
        }
        self.victim.encode(w);
        w.u64(self.world_tick);
        w.u64(self.local_tick);
        point(w, self.origin);
        point(w, self.direction);
        point(w, self.point);
        rigid(w, self.victim_pose);
        w.bool(self.front);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            source: match r.u8()? {
                0 => BeamSource::Npc(NpcId::decode(r)?),
                1 => BeamSource::Player {
                    owner: r.u64()?,
                    hull: r.u64()?,
                    item: r.u64()?,
                    emission: r.u64()?,
                },
                _ => return Err(INVALID),
            },
            victim: NpcId::decode(r)?,
            world_tick: r.u64()?,
            local_tick: r.u64()?,
            origin: read_point(r)?,
            direction: read_point(r)?,
            point: read_point(r)?,
            victim_pose: read_rigid(r)?,
            front: r.bool()?,
        })
    }
}

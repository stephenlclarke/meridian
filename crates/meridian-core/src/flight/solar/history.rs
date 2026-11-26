use super::*;
use crate::freight::g02::CommanderAccount;

// One trace is compact: fixed-size runs, a receipt only for a whole unit, and
// a final derived snapshot. Ordinary flight without solar exposure adds no bytes.
const RUN_BYTES: usize = 99;
const RECEIPT_BYTES: usize = 20;
const ANCHOR_BYTES: usize = 59;
const LAST_BYTES: usize = 86;
const NPC_SCENE_BYTES: usize = 512;
const NPC_REFUSAL_BYTES: usize = 8 + 2 + NPC_SCENE_BYTES;

fn log_bytes<T, const N: usize>(log: &ChunkLog<T, N>, record_bytes: usize) -> usize {
    4usize
        .saturating_add(log.len().saturating_mul(record_bytes))
        .saturating_add(log.len().div_ceil(N).saturating_mul(2))
}

fn encode_log<T, const N: usize>(
    w: &mut Writer,
    log: &ChunkLog<T, N>,
    record_bytes: usize,
    mut encode: impl FnMut(&mut Writer, &T) -> Result<(), SaveError>,
) -> Result<(), SaveError> {
    w.u32(u32::try_from(log.len()).map_err(|_| SaveError::TooLarge)?);
    for chunk in log.chunks() {
        w.u16(u16::try_from(chunk.len() * record_bytes).map_err(|_| SaveError::TooLarge)?);
        for entry in chunk {
            encode(w, entry)?;
        }
    }
    Ok(())
}

fn decode_log<T: Clone, const N: usize>(
    r: &mut Reader<'_>,
    record_bytes: usize,
    mut decode: impl FnMut(&mut Reader<'_>) -> Result<T, SaveError>,
) -> Result<ChunkLog<T, N>, SaveError> {
    let mut remaining = r.u32()? as usize;
    if remaining > crate::save::MAX_SOLAR_HISTORY_BYTES / record_bytes {
        return Err(SaveError::TooLarge);
    }
    let mut result = ChunkLog::default();
    while remaining > 0 {
        let count = remaining.min(N);
        let length = usize::from(r.u16()?);
        if length != count * record_bytes {
            return Err(SaveError::InvalidEncoding);
        }
        let mut chunk = r.subreader(length)?;
        for _ in 0..count {
            result.push(decode(&mut chunk)?);
        }
        if !chunk.finished() {
            return Err(SaveError::InvalidEncoding);
        }
        remaining -= count;
    }
    Ok(result)
}

impl MotionFrame {
    fn encode(self, w: &mut Writer) {
        for value in self
            .position
            .into_iter()
            .chain(self.orientation)
            .chain([self.speed])
        {
            w.f32(value);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            position: r.array(|r| r.f32())?,
            orientation: r.array(|r| r.f32())?,
            speed: r.f32()?,
        })
    }
    fn near(self, other: Self) -> bool {
        self.position
            .into_iter()
            .zip(other.position)
            .all(|(a, b)| (a - b).abs() <= 0.05)
            && self
                .orientation
                .into_iter()
                .zip(other.orientation)
                .all(|(a, b)| (a - b).abs() <= 0.00002)
            && (self.speed - other.speed).abs() <= 0.00002
    }
}
impl Anchor {
    fn encode(self, w: &mut Writer) {
        w.u8(self.system.galaxy);
        w.u16(self.system.system);
        w.u64(self.visit);
        w.u64(self.tick);
        w.u64(self.clock);
        self.frame.encode(w);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            system: SystemId {
                galaxy: r.u8()?,
                system: r.u16()?,
            },
            visit: r.u64()?,
            tick: r.u64()?,
            clock: r.u64()?,
            frame: MotionFrame::decode(r)?,
        })
    }
    fn validate(self, world: &World) -> Result<(), SaveError> {
        let nav = world.navigation.ok_or(INVALID)?.snapshot(0);
        let end = world.combat.physical_tick();
        if self.visit < 2
            || self.visit > nav.visit
            || self.tick > world.state.tick
            || self.clock > end
            || self.clock > self.tick
            || self.tick - self.clock > world.state.tick.checked_sub(end).ok_or(INVALID)?
            || world.jump_journal.system_at_visit(self.visit, self.tick)? != self.system
            || !self.frame.valid()
        {
            return Err(INVALID);
        }
        let (jumps, arrival) = world.jump_journal.visit_origin(self.visit);
        let launch = self
            .visit
            .checked_sub(1)
            .and_then(|visit| visit.checked_sub(jumps))
            .ok_or(INVALID)?;
        let berth = world.damage.berth_clock(launch).ok_or(INVALID)?;
        let (origin, clock) = if let Some(arrival) = arrival {
            ([0.0_f32, 0.0, 16000.0], arrival)
        } else {
            (
                world
                    .station_data
                    .as_ref()
                    .ok_or(INVALID)?
                    .launch_start()
                    .map_err(|_| INVALID)?
                    .position,
                berth,
            )
        };
        if clock < berth
            || world
                .damage
                .berth_clock(launch.checked_add(1).ok_or(INVALID)?)
                .is_some_and(|next| self.clock >= next)
        {
            return Err(INVALID);
        }
        let elapsed = self.clock.checked_sub(clock).ok_or(INVALID)?;
        let bound = elapsed as f64
            * f64::from(
                super::super::MAX_SPEED
                    * super::super::FIXED_DELTA_SECONDS
                    * super::super::DISTANCE_PER_SPEED_UNIT,
            )
            + 1.0;
        let distance = origin
            .into_iter()
            .zip(self.frame.position)
            .map(|(a, b)| (f64::from(a) - f64::from(b)).powi(2))
            .sum::<f64>()
            .sqrt();
        if distance > bound {
            return Err(INVALID);
        }
        Ok(())
    }
    fn near(self, other: Self) -> bool {
        (self.system, self.visit, self.tick, self.clock)
            == (other.system, other.visit, other.tick, other.clock)
            && self.frame.near(other.frame)
    }
}
impl Motion {
    fn encode(self, w: &mut Writer) {
        let tag = match self {
            Self::Active(_) => 1,
            Self::Stopped { kind, .. } => match kind {
                StopKind::Celestial => 2,
                StopKind::Traffic => 3,
                StopKind::Mule => 4,
                StopKind::Asteroid => 5,
                StopKind::Station => 6,
                StopKind::Entry => 7,
            },
        };
        let input = self.input();
        w.u8(tag);
        for value in [input.pitch, input.yaw, input.roll, input.throttle] {
            w.f32(value);
        }
        w.bool(input.brake);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let tag = r.u8()?;
        let input = FlightInput {
            pitch: r.f32()?,
            yaw: r.f32()?,
            roll: r.f32()?,
            throttle: r.f32()?,
            brake: r.bool()?,
        };
        input.validate().map_err(|_| INVALID)?;
        if tag == 1 {
            return Ok(Self::Active(input));
        }
        let kind = match tag {
            2 => StopKind::Celestial,
            3 => StopKind::Traffic,
            4 => StopKind::Mule,
            5 => StopKind::Asteroid,
            6 => StopKind::Station,
            7 => StopKind::Entry,
            _ => return Err(INVALID),
        };
        Ok(Self::Stopped { input, kind })
    }
}
impl NpcRefusal {
    pub(super) fn encode(&self, w: &mut Writer) -> Result<(), SaveError> {
        let mut scene = Writer::new();
        self.scene.encode(&mut scene);
        // Existing scene type has at most four fixed-size actors; the padded
        // record keeps chunk decoding bounded before reading its variable scene.
        if scene.bytes.len() > NPC_SCENE_BYTES {
            return Err(SaveError::TooLarge);
        }
        w.u64(self.clock);
        w.u16(scene.bytes.len() as u16);
        w.bytes.extend(&scene.bytes);
        w.bytes
            .resize(w.bytes.len() + NPC_SCENE_BYTES - scene.bytes.len(), 0);
        Ok(())
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let clock = r.u64()?;
        let length = usize::from(r.u16()?);
        if length > NPC_SCENE_BYTES {
            return Err(INVALID);
        }
        let mut payload = r.subreader(length)?;
        let scene = crate::traffic::MissileScene::decode(&mut payload)?;
        if !payload.finished() {
            return Err(INVALID);
        }
        for _ in length..NPC_SCENE_BYTES {
            if r.u8()? != 0 {
                return Err(INVALID);
            }
        }
        Ok(Self { clock, scene })
    }
}

pub(super) fn replay_motion<'a>(
    world: &World,
    anchor: Anchor,
    motion: Motion,
    scene: &crate::celestial::Snapshot,
    mule: Option<&super::super::mule_renas::HistoricMuleCollider>,
    witnesses: &mut impl Iterator<Item = &'a NpcRefusal>,
) -> Result<MotionFrame, SaveError> {
    let input = motion.input();
    let attempted = anchor.frame.advance(Motion::Active(input)).ok_or(INVALID)?;
    let data = world.station_data.as_ref().ok_or(INVALID)?;
    if scene.blocks_motion(
        anchor.frame.position,
        anchor.frame.position,
        data.player_radius(),
    ) {
        return Err(INVALID);
    }
    let celestial_blocked = scene.blocks_motion(
        anchor.frame.position,
        attempted.position,
        data.player_radius(),
    );
    if let Motion::Stopped {
        kind: StopKind::Celestial,
        ..
    } = motion
    {
        return if celestial_blocked {
            anchor.frame.advance(motion).ok_or(INVALID)
        } else {
            Err(INVALID)
        };
    }
    if celestial_blocked {
        return Err(INVALID);
    }
    let before = crate::station::flight_pose(anchor.frame.position, anchor.frame.orientation)
        .ok_or(INVALID)?;
    let after =
        crate::station::flight_pose(attempted.position, attempted.orientation).ok_or(INVALID)?;
    let angular = f64::from(
        input.pitch.abs() * super::super::PITCH_RATE
            + input.yaw.abs() * super::super::YAW_RATE
            + input.roll.abs() * super::super::ROLL_RATE,
    ) * f64::from(super::super::FIXED_DELTA_SECONDS);
    let next_tick = anchor.tick.checked_add(1).ok_or(INVALID)?;
    let asteroid_clear =
        || world.mining_motion_clear_at_step(anchor.system, next_tick, before, after, angular);
    let crosses = || data.crosses_aperture(before, after);
    let station_clear = || data.motion_clear(before, after, angular);
    let mule_blocked = || {
        world
            .historic_renas_mule_motion_blocked(mule, before, after, angular)
            .map_err(|_| INVALID)
    };
    let Motion::Stopped { kind, .. } = motion else {
        if crosses() || !station_clear() || !asteroid_clear() || mule_blocked()? {
            return Err(INVALID);
        }
        return Ok(attempted);
    };
    let justified = match kind {
        StopKind::Celestial => false,
        StopKind::Traffic => {
            let witness = witnesses.next().ok_or(INVALID)?;
            let s = &witness.scene;
            if witness.clock != anchor.clock
                || s.world_tick != anchor.tick
                || s.system != anchor.system
                || !matches!(s.stage, crate::traffic::SceneStage::Command(emission) if emission <= world.combat.emissions)
            {
                return Err(INVALID);
            }
            world
                .traffic
                .validate_missile_scene(s, world.state.tick, data)?;
            !s.player_motion_clear(
                before.position,
                after.position,
                data.player_clearance_radius(),
            )
        }
        StopKind::Mule => mule_blocked()?,
        StopKind::Asteroid => !asteroid_clear(),
        StopKind::Station => asteroid_clear() && !crosses() && !station_clear(),
        StopKind::Entry => {
            asteroid_clear()
                && crosses()
                && !data.entry_clear(
                    before,
                    after,
                    f64::from(attempted.speed * super::super::DISTANCE_PER_SPEED_UNIT),
                    f64::from(input.roll * super::super::ROLL_RATE),
                    angular,
                )
        }
    };
    if !justified {
        return Err(INVALID);
    }
    anchor.frame.advance(motion).ok_or(INVALID)
}
impl SolarSnapshot {
    fn encode(self, w: &mut Writer) {
        w.u8(match self.status {
            SolarScoopStatus::Inactive => 0,
            SolarScoopStatus::NoScoops => 1,
            SolarScoopStatus::TurnTowardSun => 2,
            SolarScoopStatus::Collecting => 3,
            SolarScoopStatus::TankFull => 4,
        });
        w.u64(self.cabin_temperature.to_bits());
        w.u64(self.fuel_remainder.to_bits());
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let status = match r.u8()? {
            0 => SolarScoopStatus::Inactive,
            1 => SolarScoopStatus::NoScoops,
            2 => SolarScoopStatus::TurnTowardSun,
            3 => SolarScoopStatus::Collecting,
            4 => SolarScoopStatus::TankFull,
            _ => return Err(INVALID),
        };
        let cabin_temperature = f64::from_bits(r.u64()?);
        let fuel_remainder = f64::from_bits(r.u64()?);
        if !(0.0..=1.0).contains(&cabin_temperature) || !(0.0..=1.0).contains(&fuel_remainder) {
            return Err(INVALID);
        }
        Ok(Self {
            status,
            cabin_temperature,
            fuel_remainder,
        })
    }
    fn near(self, other: Self) -> bool {
        self.status == other.status
            && (self.cabin_temperature - other.cabin_temperature).abs() <= 0.000002
            && (self.fuel_remainder - other.fuel_remainder).abs() <= 0.000002
    }
}
impl State {
    pub(in crate::flight) fn encoded_len(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            (12 + 1
                + usize::from(self.legacy_origin.is_some()) * ANCHOR_BYTES
                + 1
                + usize::from(self.last.is_some()) * LAST_BYTES)
                .saturating_add(log_bytes(&self.runs, RUN_BYTES))
                .saturating_add(log_bytes(&self.receipts, RECEIPT_BYTES))
                .saturating_add(log_bytes(&self.npc_refusals, NPC_REFUSAL_BYTES))
        }
    }
    pub(in crate::flight) fn encode(&self, w: &mut Writer) -> Result<(), SaveError> {
        w.option(self.legacy_origin.as_ref(), |w, anchor| anchor.encode(w));
        encode_log(w, &self.runs, RUN_BYTES, |w, run| {
            run.opening.encode(w);
            w.u64(run.first_revision);
            w.u16(run.first_fuel);
            w.u16(run.capacity);
            w.u64(run.item);
            run.motion.encode(w);
            w.u16(run.steps);
            Ok(())
        })?;
        encode_log(w, &self.receipts, RECEIPT_BYTES, |w, receipt| {
            w.u64(receipt.revision);
            w.u64(receipt.tick);
            w.u16(receipt.before);
            w.u16(receipt.added);
            Ok(())
        })?;
        encode_log(w, &self.npc_refusals, NPC_REFUSAL_BYTES, |w, witness| {
            witness.encode(w)
        })?;
        w.option(self.last.as_ref(), |w, last| {
            last.anchor.encode(w);
            w.u16(last.fuel);
            w.u64(last.revision);
            last.condition.encode(w);
        });
        Ok(())
    }
    pub(in crate::flight) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let legacy_origin = r.option(Anchor::decode)?;
        let runs = decode_log(r, RUN_BYTES, |r| {
            Ok(Run {
                opening: Anchor::decode(r)?,
                first_revision: r.u64()?,
                first_fuel: r.u16()?,
                capacity: r.u16()?,
                item: r.u64()?,
                motion: Motion::decode(r)?,
                steps: r.u16()?,
            })
        })?;
        let receipts = decode_log(r, RECEIPT_BYTES, |r| {
            Ok(Receipt {
                revision: r.u64()?,
                tick: r.u64()?,
                before: r.u16()?,
                added: r.u16()?,
            })
        })?;
        let npc_refusals = decode_log(r, NPC_REFUSAL_BYTES, NpcRefusal::decode)?;
        let last = r.option(|r| {
            Ok(Last {
                anchor: Anchor::decode(r)?,
                fuel: r.u16()?,
                revision: r.u64()?,
                condition: SolarSnapshot::decode(r)?,
            })
        })?;
        let result = Self {
            runs,
            receipts,
            npc_refusals,
            last,
            legacy_origin,
            legacy_format: false,
            #[cfg(test)]
            file_limit: None,
        };
        if result.is_empty() {
            return Err(INVALID);
        }
        Ok(result)
    }

    pub(in crate::flight) fn validate(
        &self,
        world: &World,
        account: &CommanderAccount,
        modular: &crate::flight::modular::State,
    ) -> Result<(), SaveError> {
        if self.encoded_len() > self.file_limit() {
            return Err(SaveError::TooLarge);
        }
        if self.is_empty() {
            let mut origin = Self::default();
            origin.initialize_legacy_origin(world)?;
            return if self.receipts.is_empty()
                && self.npc_refusals.is_empty()
                && self.last.is_none()
                && origin.legacy_origin.is_none()
            {
                Ok(())
            } else {
                Err(INVALID)
            };
        }
        if !world.damage.enabled() {
            return Err(INVALID);
        }
        if self.legacy_format {
            return Err(INVALID);
        }
        if let Some(origin) = self.legacy_origin {
            origin.validate(world)?;
            let sun = Self::sun(world, origin.system, origin.visit, origin.tick)?;
            if !(1.0..=THERMAL_LIMIT).contains(&origin.frame.ratio_and_nose(sun).0) {
                return Err(INVALID);
            }
        }
        let owner = world.combat_identity().ok_or(INVALID)?.owner;
        let mut previous: Option<Last> = None;
        let mut receipts = self.receipts.iter();
        let mut witnesses = self.npc_refusals.iter();
        let mut mule_visit = None;
        let mut mule = None;
        for run in self.runs.iter() {
            run.opening.validate(world)?;
            if run.steps == 0
                || run.steps > MAX_RUN_STEPS
                || run.first_revision > account.revision()
                || run.first_fuel > run.capacity
                || modular.fuel_capacity_at_revision(owner, run.first_revision)? != run.capacity
                || modular
                    .scoop_item_at(owner, run.opening.tick, run.opening.visit)
                    .map_or(0, |item| item.id)
                    != run.item
            {
                return Err(INVALID);
            }
            let mut condition = SolarSnapshot::default();
            let scene = Self::scene(
                world,
                run.opening.system,
                run.opening.visit,
                run.opening.tick,
            )?;
            if mule_visit != Some(run.opening.visit) {
                mule = world
                    .historic_renas_mule_collider(run.opening.system, run.opening.visit)
                    .map_err(|_| INVALID)?;
                mule_visit = Some(run.opening.visit);
            }
            let sun = scene.sun;
            let opening_ratio = run.opening.frame.ratio_and_nose(sun).0;
            let first_of_visit = previous.is_none_or(|last| last.anchor.visit != run.opening.visit);
            if let Some(last) = previous {
                if last.condition.cabin_temperature >= 1.0
                    || run.opening.visit < last.anchor.visit
                    || run.opening.tick < last.anchor.tick
                    || run.opening.clock < last.anchor.clock
                    || run.first_revision < last.revision
                    || run.opening.tick - run.opening.clock < last.anchor.tick - last.anchor.clock
                {
                    return Err(INVALID);
                }
                if last.anchor.visit == run.opening.visit {
                    if run.first_fuel != last.fuel || !last.anchor.near(run.opening) {
                        return Err(INVALID);
                    }
                    condition = last.condition;
                } else if opening_ratio <= THERMAL_LIMIT {
                    return Err(INVALID);
                }
            } else if opening_ratio <= THERMAL_LIMIT
                && self
                    .legacy_origin
                    .is_none_or(|origin| !origin.near(run.opening))
            {
                return Err(INVALID);
            }
            let mut anchor = run.opening;
            let mut fuel = run.first_fuel;
            let mut revision = run.first_revision;
            for step in 0..run.steps {
                let next = replay_motion(
                    world,
                    anchor,
                    run.motion,
                    &scene,
                    mule.as_ref(),
                    &mut witnesses,
                )?;
                let (ratio, nose) = next.ratio_and_nose(sun);
                if !next.valid()
                    || !ratio.is_finite()
                    || !nose.is_finite()
                    || ratio <= 1.0
                    || (first_of_visit
                        && step == 0
                        && opening_ratio > THERMAL_LIMIT
                        && ratio > THERMAL_LIMIT + 0.00001)
                {
                    return Err(INVALID);
                }
                anchor.tick = anchor.tick.checked_add(1).ok_or(INVALID)?;
                anchor.clock = anchor.clock.checked_add(1).ok_or(INVALID)?;
                anchor.frame = next;
                let (snapshot, gained) = accepted_exposure(
                    condition,
                    ratio,
                    nose,
                    f64::from(next.speed),
                    run.item != 0,
                    fuel,
                    run.capacity,
                );
                condition = snapshot;
                if gained != 0 {
                    revision = revision.checked_add(1).ok_or(INVALID)?;
                    if receipts.next()
                        != Some(&Receipt {
                            revision,
                            tick: anchor.tick,
                            before: fuel,
                            added: gained,
                        })
                    {
                        return Err(INVALID);
                    }
                    fuel += gained;
                }
                if condition.cabin_temperature >= 1.0 && step + 1 != run.steps {
                    return Err(INVALID);
                }
            }
            anchor.validate(world)?;
            if revision > account.revision() {
                return Err(INVALID);
            }
            previous = Some(Last {
                anchor,
                fuel,
                revision,
                condition,
            });
        }
        if receipts.next().is_some() || witnesses.next().is_some() {
            return Err(INVALID);
        }
        match (previous, self.last) {
            (Some(replayed), Some(saved))
                if replayed.anchor.near(saved.anchor)
                    && replayed.fuel == saved.fuel
                    && replayed.revision == saved.revision
                    && replayed.condition.near(saved.condition)
                    && (replayed.condition.cabin_temperature >= 1.0)
                        == (saved.condition.cabin_temperature >= 1.0) =>
            {
                let current = world.combat_identity().ok_or(INVALID)?;
                if saved.condition.cabin_temperature >= 1.0
                    && (saved.anchor.clock != world.combat.physical_tick() || world.pulse.alive())
                {
                    return Err(INVALID);
                }
                if saved.anchor.visit == current.visit
                    && saved.anchor.clock == world.combat.physical_tick()
                    && world.state.mode == FlightMode::Flight
                    && (!saved.anchor.frame.near(world.state.into())
                        || self.runs.last().and_then(|run| run.motion.refusal())
                            != world.state.motion_refusal)
                {
                    return Err(INVALID);
                }
            }
            (None, None) => {}
            _ => return Err(INVALID),
        }
        if world.state.mode == FlightMode::Flight && world.pulse.alive() {
            let identity = world.combat_identity().ok_or(INVALID)?;
            let current = Self::anchor(
                world.state,
                identity.system,
                identity.visit,
                world.combat.physical_tick(),
            );
            let ratio = current
                .frame
                .ratio_and_nose(Self::sun(
                    world,
                    identity.system,
                    identity.visit,
                    current.tick,
                )?)
                .0;
            let tracked = self
                .last
                .is_some_and(|last| last.anchor.visit == current.visit)
                || self
                    .legacy_origin
                    .is_some_and(|origin| origin.visit == current.visit);
            if (tracked || ratio <= THERMAL_LIMIT)
                && self.last.is_none_or(|last| !last.anchor.near(current))
                && self
                    .legacy_origin
                    .is_none_or(|origin| !origin.near(current))
            {
                return Err(INVALID);
            }
        }
        Ok(())
    }
}

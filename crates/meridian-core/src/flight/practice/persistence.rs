use super::{
    DockContext, FirstHit, FlightMode, NavigationState, OriginalStationData, Practice,
    PracticeOutcome, Range, RecordedPose, Retirement, RigidPose, Snapshot, SystemId, geometry,
    lifecycle, reception,
};
use crate::navigation::SavedNavigationSummary;
use crate::save::{Reader, SaveError, Writer};
const INVALID: SaveError = SaveError::InvalidState("T02 practice consent, visit or receipts");

impl Practice {
    pub(in crate::flight) fn encode(self, w: &mut Writer) {
        self.encode_version(w, 9);
    }
    fn encode_version(self, w: &mut Writer, version: u16) {
        self.consent.encode_save(w);
        w.u64(self.confirmed_tick);
        w.option(self.range.as_ref(), |w, range| {
            range.opening.encode(w);
            w.u64(range.opened_tick);
            for hit in range.hits {
                w.option(hit.as_ref(), |w, hit| {
                    w.u64(hit.tick);
                    hit.firing_pose.encode(w);
                    if version >= 8 {
                        if let Some((facing, emission)) = hit.integral {
                            w.u8(facing.integral_tag(2));
                            w.u64(emission);
                        } else {
                            w.option(hit.missile.as_ref(), |w, origin| origin.encode(w));
                        }
                    }
                });
            }
        });
        w.option(self.retired.as_ref(), |w, retired| {
            w.u64(retired.tick);
            w.u64(retired.visit);
        });
        if version >= 8 {
            w.u8(self.pending_missiles);
        }
        if version >= 9 {
            w.u8(self.attempt);
            w.option(self.settlement.as_ref(), |w, o| {
                w.u8(match o {
                    PracticeOutcome::Complete => 0,
                    PracticeOutcome::Failed => 1,
                    PracticeOutcome::Abandoned => 2,
                })
            });
            w.option(self.cancel.as_ref(), |w, c| c.encode(w));
        }
    }
    pub(in crate::flight) fn decode(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        let mut practice = Self {
            attempt: 0,
            settlement: None,
            cancel: None,
            consent: DockContext::decode_save(r)?,
            confirmed_tick: r.u64()?,
            range: r.option(|r| {
                Ok(Range {
                    opening: RecordedPose::decode(r)?,
                    opened_tick: r.u64()?,
                    hits: r.array(|r| {
                        r.option(|r| {
                            let tick = r.u64()?;
                            let firing_pose = RecordedPose::decode(r)?;
                            let source = if version >= 8 { r.u8()? } else { 0 };
                            let (missile, integral) = match source {
                                0 => (None, None),
                                1 => (Some(super::super::MissileOrigin::decode(r)?), None),
                                tag @ 2..=4 if version >= 17 => (
                                    None,
                                    Some((
                                        super::super::medusa_lasers::INTEGRAL[usize::from(tag - 2)],
                                        r.u64()?,
                                    )),
                                ),
                                _ => return Err(INVALID),
                            };
                            Ok(FirstHit {
                                tick,
                                firing_pose,
                                missile,
                                integral,
                            })
                        })
                    })?,
                })
            })?,
            reception: reception::Reception::default(),
            ordinary_hit: None,
            pending_missiles: 0,
            retired: r.option(|r| {
                Ok(Retirement {
                    tick: r.u64()?,
                    visit: r.u64()?,
                })
            })?,
        };
        if version >= 8 {
            practice.pending_missiles = r.u8()?;
        }
        if version >= 9 {
            practice.attempt = r.u8()?;
            practice.settlement = r.option(|r| match r.u8()? {
                0 => Ok(PracticeOutcome::Complete),
                1 => Ok(PracticeOutcome::Failed),
                2 => Ok(PracticeOutcome::Abandoned),
                _ => Err(INVALID),
            })?;
            practice.cancel = r.option(lifecycle::Closure::decode)?;
        }
        Ok(practice)
    }
    pub(in crate::flight) fn validate(
        self,
        nav: SavedNavigationSummary,
        navigation: NavigationState,
        flight: Snapshot,
        data: &OriginalStationData,
        world: &super::super::World,
    ) -> Result<(), SaveError> {
        let pulse = world.pulse;
        self.validate_closure(nav, flight.tick)?;
        let sortie = self.sortie().ok_or(INVALID)?;
        self.validate_consent(nav, flight.tick)?;
        self.validate_lifetime(nav, navigation, flight, sortie)?;
        if let Some(range) = self.range {
            let last_active_tick = self.retired.map_or(flight.tick, |end| end.tick - 1);
            if range.opened_tick <= self.confirmed_tick || range.opened_tick > last_active_tick {
                return Err(INVALID);
            }
            range.validate(last_active_tick, flight, data, world)?;
            if let Some(latest_hit) = range
                .hits
                .iter()
                .flatten()
                .filter(|hit| hit.missile.is_none())
                .map(|hit| hit.tick)
                .max()
            {
                // Every published step advances one global tick. Only a subset
                // advances the weapon clock; docking and arrival retain cooldown.
                // A later discharge can only raise the remainder of this floor.
                let elapsed = flight.tick - latest_hit;
                let minimum =
                    u64::from(range.laser_cadence(latest_hit, world)?).saturating_sub(elapsed);
                if u64::from(pulse.remaining_cooldown()) < minimum {
                    return Err(INVALID);
                }
            }
        }
        self.validate_reception(nav, flight, world, navigation.dock())
    }
    fn validate_consent(self, nav: SavedNavigationSummary, tick: u64) -> Result<(), SaveError> {
        use crate::navigation::DockOrigin;
        let dock = self.consent;
        if dock.owner().seed() != nav.owner
            || dock.ship().ordinal() != nav.hull
            || dock.ship().owner() != dock.owner()
            || dock.system() != SystemId::AVENOR
            || dock.sequence() == 0
            || dock.sequence() > nav.dock_sequence
            || dock.visit() < dock.sequence()
            || dock.visit() > nav.visit
            || dock.tick() > self.confirmed_tick
            || self.confirmed_tick > tick
        {
            return Err(INVALID);
        }
        match dock.origin() {
            DockOrigin::InitialStart
                if dock.tick() == 0 && dock.sequence() == 1 && dock.visit() == 1 =>
            {
                Ok(())
            }
            DockOrigin::PhysicalEntry if dock.tick() > 0 && dock.sequence() > 1 => Ok(()),
            _ => Err(INVALID),
        }
    }
    fn validate_lifetime(
        self,
        nav: SavedNavigationSummary,
        navigation: NavigationState,
        flight: Snapshot,
        sortie: u64,
    ) -> Result<(), SaveError> {
        if self.cancel.is_some() && self.range.is_none() && self.retired.is_none() {
            return if self.cancel.is_some_and(|c| c.dock == self.consent) {
                Ok(())
            } else {
                Err(INVALID)
            };
        }
        if let Some(retired) = self.retired {
            if retired.tick <= self.confirmed_tick
                || retired.tick > flight.tick
                || retired.visit < sortie
                || retired.visit - sortie > 1
                || retired.visit > nav.visit
                || (retired.visit == sortie && nav.dock_sequence == self.consent.sequence())
            {
                return Err(INVALID);
            }
            return Ok(());
        }
        if navigation.dock() == Some(self.consent) && self.range.is_none() {
            return Ok(());
        }
        if nav.visit != sortie
            || navigation.snapshot(0).system != SystemId::AVENOR
            || flight.mode == FlightMode::Docked
            || nav.dock_sequence != self.consent.sequence()
            || (self.range.is_some() && flight.mode != FlightMode::Flight)
        {
            return Err(INVALID);
        }
        Ok(())
    }
}

impl Range {
    fn laser_cadence(&self, tick: u64, world: &super::super::World) -> Result<u8, SaveError> {
        if let Some((facing, emission)) = self
            .hits
            .iter()
            .flatten()
            .filter(|hit| hit.tick == tick)
            .find_map(|hit| hit.integral)
        {
            let (_, weapon) = world
                .laser_fit_at_tick(facing, tick, emission)
                .ok_or(INVALID)?;
            Ok(weapon.cooldown_ticks())
        } else {
            world.front_cadence_at_tick(tick).ok_or(INVALID)
        }
    }
    fn validate(
        self,
        last_active_tick: u64,
        flight: Snapshot,
        data: &OriginalStationData,
        world: &super::super::World,
    ) -> Result<(), SaveError> {
        let station = flight.station.ok_or(INVALID)?;
        let opening = self.opening.checked_pose_at(self.opened_tick, flight)?;
        if !geometry::range_clear(data, station, opening) {
            return Err(INVALID);
        }
        let mut ticks = [0; 3];
        for (ordinal, slot) in self.hits.into_iter().enumerate() {
            let Some(hit) = slot else {
                continue;
            };
            if hit.tick < self.opened_tick || hit.tick > last_active_tick {
                return Err(INVALID);
            }
            let pose = hit.firing_pose.checked_pose_at(hit.tick, flight)?;
            if hit.missile.is_some() {
                continue;
            } // World cross-validates the retained projectile contact.
            let pose = hit.integral.map_or(pose, |(f, _)| f.laser_ray(pose));
            let station_distance = data
                .beam_block_distance(
                    station,
                    pose.position,
                    pose.orientation.z,
                    crate::PULSE_RANGE,
                )
                .map_err(|_| INVALID)?;
            if geometry::nearest(opening, pose, station_distance)
                .is_none_or(|(winner, _)| winner != ordinal)
            {
                return Err(INVALID);
            }
            ticks[ordinal] = hit.tick;
        }
        ticks.sort_unstable();
        let mut hits = ticks.into_iter().filter(|tick| *tick != 0);
        if let Some(mut prior) = hits.next() {
            for tick in hits {
                if tick - prior < u64::from(self.laser_cadence(prior, world)?) {
                    return Err(INVALID);
                }
                prior = tick;
            }
        }
        Ok(())
    }
}

impl RecordedPose {
    fn encode(self, w: &mut Writer) {
        for value in self.position.into_iter().chain(self.orientation) {
            w.f32(value);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            position: r.array(|r| r.f32())?,
            orientation: r.array(|r| r.f32())?,
        })
    }
    fn checked_pose_at(self, tick: u64, flight: Snapshot) -> Result<RigidPose, SaveError> {
        if tick == flight.tick
            && (self.position.map(f32::to_bits) != flight.position.map(f32::to_bits)
                || self.orientation.map(f32::to_bits) != flight.orientation.map(f32::to_bits))
        {
            return Err(INVALID);
        }
        let norm = self
            .orientation
            .iter()
            .map(|x| f64::from(*x).powi(2))
            .sum::<f64>();
        let pose = self.pose().ok_or(INVALID)?;
        let distance_limit = 16000.0
            + tick as f64
                * f64::from(
                    super::super::MAX_SPEED
                        * super::super::DISTANCE_PER_SPEED_UNIT
                        * super::super::FIXED_DELTA_SECONDS,
                )
                * 1.1;
        if (norm - 1.0).abs() > 1e-5
            || self
                .position
                .iter()
                .any(|x| f64::from(*x).abs() > distance_limit)
        {
            return Err(INVALID);
        }
        Ok(pose)
    }
}

use super::*;
use crate::flight::weapons::PulseResources;
use crate::navigation::SavedNavigationSummary;
use crate::save::{Reader, SaveError, Writer};
const INVALID: SaveError = SaveError::InvalidState("T02 practice consent, visit or receipts");

impl Practice {
    pub(in crate::flight) fn encode(self, w: &mut Writer) {
        self.consent.encode_save(w);
        w.u64(self.confirmed_tick);
        w.option(self.range.as_ref(), |w, range| {
            range.opening.encode(w);
            w.u64(range.opened_tick);
            for hit in range.hits {
                w.option(hit.as_ref(), |w, hit| {
                    w.u64(hit.tick);
                    hit.firing_pose.encode(w);
                });
            }
        });
        w.option(self.retired.as_ref(), |w, retired| {
            w.u64(retired.tick);
            w.u64(retired.visit);
        });
    }
    pub(in crate::flight) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            consent: DockContext::decode_save(r)?,
            confirmed_tick: r.u64()?,
            range: r.option(|r| {
                Ok(Range {
                    opening: RecordedPose::decode(r)?,
                    opened_tick: r.u64()?,
                    hits: r.array(|r| {
                        r.option(|r| {
                            Ok(FirstHit {
                                tick: r.u64()?,
                                firing_pose: RecordedPose::decode(r)?,
                                missile: None,
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
        })
    }
    pub(in crate::flight) fn validate(
        self,
        nav: SavedNavigationSummary,
        navigation: NavigationState,
        flight: Snapshot,
        data: &OriginalStationData,
        pulse: PulseResources,
    ) -> Result<(), SaveError> {
        let sortie = self.sortie().ok_or(INVALID)?;
        self.validate_consent(nav, flight.tick)?;
        self.validate_lifetime(nav, navigation, flight, sortie)?;
        if let Some(range) = self.range {
            let last_active_tick = self.retired.map_or(flight.tick, |end| end.tick - 1);
            if range.opened_tick <= self.confirmed_tick || range.opened_tick > last_active_tick {
                return Err(INVALID);
            }
            range.validate(last_active_tick, flight, data)?;
            if let Some(latest_hit) = range.hits.iter().flatten().map(|hit| hit.tick).max() {
                // Every published step advances one global tick. Only a subset
                // advances the weapon clock; docking and arrival retain cooldown.
                // A later discharge can only raise the remainder of this floor.
                let elapsed = flight.tick - latest_hit;
                let minimum = u64::from(crate::PULSE_COOLDOWN_TICKS).saturating_sub(elapsed);
                if u64::from(pulse.remaining_cooldown()) < minimum {
                    return Err(INVALID);
                }
            }
        }
        self.validate_reception(nav, flight, pulse, navigation.dock())
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
    fn validate(
        self,
        last_active_tick: u64,
        flight: Snapshot,
        data: &OriginalStationData,
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
            if hit.missile.is_some() {
                return Err(SaveError::InvalidState(
                    "unsupported missile hit save boundary",
                ));
            }
            if hit.tick < self.opened_tick || hit.tick > last_active_tick {
                return Err(INVALID);
            }
            let pose = hit.firing_pose.checked_pose_at(hit.tick, flight)?;
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
                if tick - prior < u64::from(crate::PULSE_COOLDOWN_TICKS) {
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

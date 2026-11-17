//! Exact completed hyperspace legs from the initial commander state.
use super::{
    BASIC_SPOOL_TICKS, DockContext, FUEL_CAPACITY, NavigationState, OriginalNavigation,
    STOCK_JUMP_REACH_LY, SystemId,
    persistence::{read_system, write_system},
    stock_jump_fuel_cost,
};
use crate::save::{MAX_SAVE_BYTES, Reader, SaveError, Writer};

const INVALID: SaveError = SaveError::InvalidState("completed jump journal");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavigationGrade {
    Empty,
    Basic,
    Intermediate,
    Advanced,
}
impl NavigationGrade {
    pub(crate) fn module_reach_ly(self) -> Option<u8> {
        match self {
            Self::Empty => None,
            Self::Basic => Some(7),
            Self::Intermediate => Some(14),
            Self::Advanced => Some(21),
        }
    }
    pub(crate) fn reach_ly(self) -> f64 {
        self.module_reach_ly().map_or(STOCK_JUMP_REACH_LY, |reach| {
            f64::from(reach).max(STOCK_JUMP_REACH_LY)
        })
    }
    fn tag(self) -> u8 {
        match self {
            Self::Empty => 0,
            Self::Basic => 1,
            Self::Intermediate => 2,
            Self::Advanced => 3,
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        match r.u8()? {
            0 => Ok(Self::Empty),
            1 => Ok(Self::Basic),
            2 => Ok(Self::Intermediate),
            3 => Ok(Self::Advanced),
            _ => Err(SaveError::InvalidEncoding),
        }
    }
}

pub(crate) struct NavigationFitChange {
    pub(crate) dock: DockContext,
    pub(crate) tick: u64,
    pub(crate) grade: NavigationGrade,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CompletedJump {
    source: SystemId,
    destination: SystemId,
    debit: u16,
    source_visit: u64,
    arrival_tick: u64,
    grade: NavigationGrade,
}
impl CompletedJump {
    pub(crate) const ENCODED_SIZE: usize = 25;

    pub(crate) fn arrived(
        source: NavigationState,
        arrived: NavigationState,
        debit: u16,
        tick: u64,
        grade: NavigationGrade,
    ) -> Self {
        Self {
            source: source.system,
            destination: arrived.system,
            debit,
            source_visit: source.visit,
            arrival_tick: tick,
            grade,
        }
    }
    fn encode(self, w: &mut Writer) {
        write_system(w, self.source);
        write_system(w, self.destination);
        w.u16(self.debit);
        w.u64(self.source_visit);
        w.u64(self.arrival_tick);
        w.u8(self.grade.tag());
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            source: read_system(r)?,
            destination: read_system(r)?,
            debit: r.u16()?,
            source_visit: r.u64()?,
            arrival_tick: r.u64()?,
            grade: NavigationGrade::decode(r)?,
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct JumpJournal {
    legs: Vec<CompletedJump>,
}
impl JumpJournal {
    const MAX_LEGS: usize = MAX_SAVE_BYTES / CompletedJump::ENCODED_SIZE;

    pub(crate) fn has_room(&self) -> bool {
        self.legs.len() < Self::MAX_LEGS
    }
    pub(crate) fn append(&mut self, leg: CompletedJump) {
        self.legs.push(leg);
    }
    pub(crate) fn encode(&self, w: &mut Writer) {
        w.u16(self.legs.len() as u16);
        for leg in &self.legs {
            leg.encode(w);
        }
    }
    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let count = usize::from(r.u16()?);
        if count > Self::MAX_LEGS {
            return Err(SaveError::TooLarge);
        }
        let legs = (0..count)
            .map(|_| CompletedJump::decode(r))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { legs })
    }

    pub(crate) fn validate(
        &self,
        nav: NavigationState,
        catalogue: &OriginalNavigation,
        tick: u64,
        changes: &[NavigationFitChange],
    ) -> Result<(), SaveError> {
        let spool = u64::from(BASIC_SPOOL_TICKS);
        let pending_departure = nav
            .pending
            .map(|pending| -> Result<_, SaveError> {
                let arrival = tick
                    .checked_add(u64::from(pending.remaining_ticks))
                    .ok_or(INVALID)?;
                Ok((
                    pending.source_visit,
                    arrival.checked_sub(spool).ok_or(INVALID)?,
                ))
            })
            .transpose()?;
        let mut system = SystemId::AVENOR;
        let mut spent = 0_u64;
        let mut jumps = 0_u64;
        let mut earliest_visit = 1;
        let mut earliest_tick = 0;
        let mut grade = NavigationGrade::Basic;
        let mut changes = changes.iter().peekable();

        for (index, next) in self
            .legs
            .iter()
            .map(Some)
            .chain(std::iter::once(None))
            .enumerate()
        {
            let departure = match next {
                Some(leg) => Some((
                    leg.source_visit,
                    leg.arrival_tick.checked_sub(spool).ok_or(INVALID)?,
                )),
                None => pending_departure,
            };
            if departure.is_some_and(|(_, start)| start < earliest_tick) {
                return Err(INVALID);
            }
            while let Some(change) = changes.peek().copied() {
                let berth_jumps = change
                    .dock
                    .visit()
                    .checked_sub(change.dock.sequence())
                    .ok_or(INVALID)?;
                if berth_jumps > jumps {
                    break;
                }
                if berth_jumps != jumps
                    || change.dock.system() != system
                    || change.dock.visit() < earliest_visit
                    || change.tick < earliest_tick
                    || (index > 0 && change.dock.tick() <= earliest_tick)
                    || departure.is_some_and(|(visit, start)| {
                        change.dock.visit() >= visit || change.tick > start
                    })
                {
                    return Err(INVALID);
                }
                grade = change.grade;
                let _ = changes.next();
            }
            let Some(leg) = next else {
                break;
            };

            let distance = catalogue
                .distance_ly(leg.source, leg.destination)
                .map_err(|_| INVALID)?;
            if leg.source != system
                || leg.source == leg.destination
                || leg.grade != grade
                || distance <= 0.0
                || distance > grade.reach_ly().min(f64::from(FUEL_CAPACITY) / 10.0)
                || stock_jump_fuel_cost(distance).map_err(|_| INVALID)? != leg.debit
                || leg.source_visit < earliest_visit
                || jumps
                    .checked_add(2)
                    .is_none_or(|visit| leg.source_visit < visit)
                || leg.arrival_tick <= earliest_tick
                || leg.arrival_tick > tick
            {
                return Err(INVALID);
            }
            spent = spent.checked_add(u64::from(leg.debit)).ok_or(INVALID)?;
            jumps = jumps.checked_add(1).ok_or(INVALID)?;
            earliest_visit = leg.source_visit.checked_add(1).ok_or(INVALID)?;
            earliest_tick = leg.arrival_tick;
            system = leg.destination;
        }
        if changes.next().is_some()
            || jumps != nav.completed_jumps
            || spent != nav.fuel_spent
            || system != nav.system
            || earliest_visit > nav.visit
        {
            return Err(INVALID);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_later_paid_fit_cannot_authorize_a_long_earlier_stock_leg() {
        // This forged route's two stock debits total 172. The old pooled
        // allowance gave them 180, even though the first leg alone needs 99.
        const LARERIA: SystemId = SystemId {
            galaxy: 0,
            system: 91,
        };
        let catalogue = OriginalNavigation::bundled().unwrap_or_else(|e| panic!("{e:?}"));
        let mut nav = NavigationState::new(15901, 15902, 15907);
        nav.launches = 2;
        nav.completed_jumps = 3;
        nav.fuel_spent = 206;
        nav.visit = 6;
        nav.system = SystemId::AVENOR;
        nav.dock_sequence = 2;
        nav.dock = None;
        let mut berth_nav = nav;
        berth_nav.system = SystemId::RENAS;
        berth_nav.visit = 4;
        let dock = berth_nav.context(1300, super::super::DockOrigin::PhysicalEntry);
        let change = NavigationFitChange {
            dock,
            tick: 1300,
            grade: NavigationGrade::Intermediate,
        };
        let journal = JumpJournal {
            legs: vec![
                CompletedJump {
                    source: SystemId::AVENOR,
                    destination: LARERIA,
                    debit: 99,
                    source_visit: 2,
                    arrival_tick: 601,
                    grade: NavigationGrade::Basic,
                },
                CompletedJump {
                    source: LARERIA,
                    destination: SystemId::RENAS,
                    debit: 73,
                    source_visit: 3,
                    arrival_tick: 1201,
                    grade: NavigationGrade::Basic,
                },
                CompletedJump {
                    source: SystemId::RENAS,
                    destination: SystemId::AVENOR,
                    debit: 34,
                    source_visit: 5,
                    arrival_tick: 1901,
                    grade: NavigationGrade::Intermediate,
                },
            ],
        };
        assert_eq!(
            journal.validate(nav, &catalogue, 1901, &[change]),
            Err(INVALID)
        );
    }
}

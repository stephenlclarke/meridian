//! Station-time evidence for the single paid Naval Energy Unit at Aelavia.
use super::{Event, INVALID, State, World};
use crate::{navigation::DockContext, save::SaveError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NavalBerth {
    Occupied,
    Launched(u64),
}

impl NavalBerth {
    pub(super) fn encoded_tick(self) -> u64 {
        match self {
            Self::Occupied => 0,
            Self::Launched(tick) => tick,
        }
    }

    pub(super) fn from_tick(tick: u64) -> Self {
        match tick {
            0 => Self::Occupied,
            tick => Self::Launched(tick),
        }
    }
}

impl State {
    /// None means this launch does not close the sale berth. A tag-21 receipt
    /// already reserves its tick; replacing tag 3 and its kind with tag 21 and
    /// a u64 launch tick grows the modular record by seven bytes.
    pub(in crate::flight) fn naval_launch_growth(
        &self,
        dock: Option<DockContext>,
    ) -> Option<usize> {
        let dock = dock.filter(|dock| dock.system() == crate::navigation::SystemId::AELAVIA)?;
        match self.naval_berth {
            Some(NavalBerth::Occupied) => Some(0),
            Some(NavalBerth::Launched(_)) => None,
            None => self
                .events
                .iter()
                .any(|e| {
                    !e.xyzzy
                        && e.dock == dock
                        && super::aelavia_naval_purchase(dock.system(), e.action)
                })
                .then_some(7),
        }
    }

    pub(in crate::flight) fn record_naval_launch(&mut self, tick: u64) {
        self.naval_berth = Some(NavalBerth::Launched(tick));
    }

    pub(super) fn validate_naval_berth(
        &self,
        world: &World,
        sale: &Event,
    ) -> Result<(), SaveError> {
        let dock = sale.dock;
        world.jump_journal.validate_berth_at(dock, sale.tick)?;
        if world.current_dock_context() == Some(dock) {
            return match self.naval_berth {
                Some(NavalBerth::Launched(_)) => Err(INVALID),
                _ => Ok(()),
            };
        }

        let physical = world.damage.berth_clock(dock.sequence()).ok_or(INVALID)?;
        let current_offset = world
            .state
            .tick
            .checked_sub(world.combat.physical_tick())
            .ok_or(INVALID)?;
        // Docked ticks accumulate. A later clock pair supplies an upper bound even
        // when intervening berths make the individual docked intervals unknowable.
        let latest = physical.checked_add(current_offset).ok_or(INVALID)?;
        let exact = self
            .sale_flight_offset(world, dock, current_offset)?
            .map(|offset| physical.checked_add(offset).ok_or(INVALID))
            .transpose()?;
        let limit = match self.naval_berth {
            Some(NavalBerth::Occupied) => return Err(INVALID),
            Some(NavalBerth::Launched(tick)) => {
                if tick > latest || exact.is_some_and(|observed| observed != tick) {
                    return Err(INVALID);
                }
                tick
            }
            None => exact.unwrap_or(latest),
        };
        if sale.tick > limit || limit < dock.tick() {
            return Err(INVALID);
        }
        Ok(())
    }

    fn sale_flight_offset(
        &self,
        world: &World,
        dock: DockContext,
        current_offset: u64,
    ) -> Result<Option<u64>, SaveError> {
        let nav = world.navigation.ok_or(INVALID)?;
        // The physical clock advances on every exterior step, including the step
        // that docks. Until the next berth, world - physical is constant.
        if nav.dock_sequence() == dock.sequence() {
            return Ok(Some(current_offset));
        }
        let visit = dock.visit().checked_add(1).ok_or(INVALID)?;
        if let Some(jump) = world.jump_journal.completed_timing(visit)? {
            return jump
                .departure_tick
                .checked_sub(jump.departure_clock)
                .map(Some)
                .ok_or(INVALID);
        }
        let next = dock.sequence().checked_add(1).ok_or(INVALID)?;
        let next_berth = world
            .current_dock_context()
            .filter(|b| b.sequence() == next)
            .or_else(|| self.history.berth_at_sequence(next))
            .or_else(|| {
                self.events
                    .iter()
                    .map(|e| e.dock)
                    .find(|b| b.sequence() == next)
            });
        if let Some(berth) = next_berth {
            let entry_clock = world.damage.berth_clock(next).ok_or(INVALID)?;
            return berth
                .tick()
                .checked_sub(entry_clock)
                .map(Some)
                .ok_or(INVALID);
        }
        world
            .solar
            .first_clock_pair(visit)
            .map(|(tick, clock)| tick.checked_sub(clock).ok_or(INVALID))
            .transpose()
    }
}

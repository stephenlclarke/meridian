#[cfg(test)]
use super::NavigationError;
use super::{
    BASIC_SPOOL_TICKS, CommanderId, DockContext, DockOrigin, NavigationState, OriginalNavigation,
    PendingJump, PlayerShipId, StarterBasicDrive, SystemId, stock_jump_fuel_cost,
};
use crate::save::{Reader, SaveError, Writer};
use crate::station::FlightMode;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SavedNavigationSummary {
    pub(crate) owner: u64,
    pub(crate) hull: u64,
    pub(crate) drive: u64,
    pub(crate) visit: u64,
    pub(crate) dock_sequence: u64,
    pub(crate) jumps: u64,
    pub(crate) launches: u64,
    pub(crate) fuel_spent: u64,
}

pub(super) fn write_system(w: &mut Writer, s: SystemId) {
    w.u8(s.galaxy);
    w.u16(s.system);
}
pub(super) fn read_system(r: &mut Reader<'_>) -> Result<SystemId, SaveError> {
    Ok(SystemId {
        galaxy: r.u8()?,
        system: r.u16()?,
    })
}
fn write_ship(w: &mut Writer, s: PlayerShipId) {
    w.u64(s.owner.0);
    w.u64(s.ordinal);
}
fn read_ship(r: &mut Reader<'_>) -> Result<PlayerShipId, SaveError> {
    Ok(PlayerShipId {
        owner: CommanderId(r.u64()?),
        ordinal: r.u64()?,
    })
}
fn write_drive(w: &mut Writer, d: &StarterBasicDrive) {
    w.u64(d.owner.0);
    write_ship(w, d.ship);
    w.u64(d.item_ordinal);
}
fn read_drive(r: &mut Reader<'_>) -> Result<StarterBasicDrive, SaveError> {
    Ok(StarterBasicDrive {
        owner: CommanderId(r.u64()?),
        ship: read_ship(r)?,
        item_ordinal: r.u64()?,
    })
}
impl DockContext {
    pub(crate) fn encode_save(&self, w: &mut Writer) {
        w.u64(self.owner.0);
        write_ship(w, self.ship);
        write_system(w, self.system);
        w.u64(self.visit);
        w.u64(self.sequence);
        w.u64(self.tick);
        w.u8(match self.origin {
            DockOrigin::InitialStart => 0,
            DockOrigin::PhysicalEntry => 1,
        });
    }
    pub(crate) fn decode_save(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let owner = CommanderId(r.u64()?);
        let ship = read_ship(r)?;
        let system = read_system(r)?;
        let visit = r.u64()?;
        let sequence = r.u64()?;
        let tick = r.u64()?;
        let origin = match r.u8()? {
            0 => DockOrigin::InitialStart,
            1 => DockOrigin::PhysicalEntry,
            _ => return Err(SaveError::InvalidEncoding),
        };
        Ok(Self {
            owner,
            ship,
            system,
            visit,
            sequence,
            tick,
            origin,
        })
    }
}
impl NavigationState {
    pub(crate) fn encode_save(&self, w: &mut Writer) {
        w.u64(self.owner.0);
        write_ship(w, self.ship);
        write_drive(w, &self.drive);
        write_system(w, self.system);
        w.u64(self.visit);
        w.u64(self.dock_sequence);
        w.option(self.dock.as_ref(), |w, dock| dock.encode_save(w));
        w.option(self.pending.as_ref(), |w, p| {
            write_system(w, p.destination);
            w.u32(p.remaining_ticks);
            w.u16(p.fuel_debit);
            write_system(w, p.source);
            w.u64(p.source_visit);
            write_drive(w, &p.drive);
        });
        w.u64(self.launches);
        w.u64(self.completed_jumps);
        w.u64(self.fuel_spent);
    }
    pub(crate) fn decode_save(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            owner: CommanderId(r.u64()?),
            ship: read_ship(r)?,
            drive: read_drive(r)?,
            system: read_system(r)?,
            visit: r.u64()?,
            dock_sequence: r.u64()?,
            dock: r.option(DockContext::decode_save)?,
            pending: r.option(|r| {
                Ok(PendingJump {
                    destination: read_system(r)?,
                    remaining_ticks: r.u32()?,
                    fuel_debit: r.u16()?,
                    source: read_system(r)?,
                    source_visit: r.u64()?,
                    drive: read_drive(r)?,
                })
            })?,
            launches: r.u64()?,
            completed_jumps: r.u64()?,
            fuel_spent: r.u64()?,
        })
    }
    pub(crate) fn validate_pending_reach(
        &self,
        catalogue: &OriginalNavigation,
        fitted_reach_ly: f64,
    ) -> Result<(), SaveError> {
        if let Some(pending) = self.pending {
            let invalid = SaveError::InvalidState("pending jump");
            if catalogue
                .distance_ly(pending.source, pending.destination)
                .map_err(|_| invalid)?
                > fitted_reach_ly
            {
                return Err(invalid);
            }
        }
        Ok(())
    }

    pub(crate) fn validate_save(
        &self,
        catalogue: &OriginalNavigation,
        mode: FlightMode,
        tick: u64,
        fuel: u16,
    ) -> Result<SavedNavigationSummary, SaveError> {
        let invalid = SaveError::InvalidState("navigation identity or chronology");
        if self.owner.0 == 0
            || self.owner.0.checked_add(10).is_none()
            || self.ship.owner != self.owner
            || self.ship.ordinal != self.owner.0 + 1
            || self.drive.owner != self.owner
            || self.drive.ship != self.ship
            || self.drive.item_ordinal != self.owner.0 + 6
            || u128::from(self.visit)
                != 1 + u128::from(self.launches) + u128::from(self.completed_jumps)
            || self.dock_sequence == 0
            // Launch is an explicit docked command and does not advance the simulation tick.
            // Every later dock sequence is one distinct physical-entry step. Launches consume
            // those admissions, leaving exactly one unconsumed admission iff currently docked.
            || u128::from(self.launches) + u128::from(u8::from(mode == FlightMode::Docked))
                != u128::from(self.dock_sequence)
            || self.dock_sequence - 1 > tick
            || self.completed_jumps > tick
            || self.completed_jumps == 0
                && (self.fuel_spent != 0 || self.system != SystemId::AVENOR)
            || self.completed_jumps > 0 && self.fuel_spent < self.completed_jumps
            || self.launches == 0 && mode != FlightMode::Docked
            || u128::from(self.fuel_spent) > u128::from(self.completed_jumps) * u128::from(super::FUEL_CAPACITY)
        {
            return Err(invalid);
        }
        catalogue
            .system(self.system)
            .map_err(|_| SaveError::InvalidState("unknown current system"))?;
        if (mode == FlightMode::Docked) != self.dock.is_some() {
            return Err(invalid);
        }
        if let Some(dock) = self.dock {
            if dock.owner != self.owner
                || dock.ship != self.ship
                || dock.system != self.system
                || dock.visit != self.visit
                || dock.sequence != self.dock_sequence
                || dock.tick > tick
            {
                return Err(invalid);
            }
            match dock.origin {
                DockOrigin::InitialStart
                    if self.system == SystemId::AVENOR
                        && self.visit == 1
                        && self.dock_sequence == 1
                        && dock.tick == 0 => {}
                DockOrigin::PhysicalEntry if self.dock_sequence >= 2 && dock.tick > 0 => {}
                _ => return Err(invalid),
            }
        }
        if let Some(p) = self.pending {
            let distance = catalogue
                .distance_ly(p.source, p.destination)
                .map_err(|_| invalid)?;
            if mode != FlightMode::Flight
                || p.source != self.system
                || p.source_visit != self.visit
                || p.drive != self.drive
                || p.remaining_ticks == 0
                || p.remaining_ticks > BASIC_SPOOL_TICKS
                || distance <= 0.0
                || distance > f64::from(fuel) / 10.0
                || stock_jump_fuel_cost(distance).map_err(|_| invalid)? != p.fuel_debit
            {
                return Err(SaveError::InvalidState("pending jump"));
            }
        }
        Ok(SavedNavigationSummary {
            owner: self.owner.0,
            hull: self.ship.ordinal,
            drive: self.drive.item_ordinal,
            visit: self.visit,
            dock_sequence: self.dock_sequence,
            jumps: self.completed_jumps,
            launches: self.launches,
            fuel_spent: self.fuel_spent,
        })
    }
}

#[cfg(test)]
mod counter_tests {
    use super::{FlightMode, NavigationError, NavigationState, OriginalNavigation};

    #[test]
    fn exhausted_launch_and_admission_counters_refuse_before_mutation() {
        // Internal arithmetic boundaries only; the public producer/Continue proof lives
        // in flight persistence tests and uses no caller-created navigation authority.
        for (visit, launches) in [(u64::MAX, 0), (1, u64::MAX)] {
            let mut navigation = NavigationState::new(101, 102, 107);
            navigation.visit = visit;
            navigation.launches = launches;
            let before = navigation;
            assert_eq!(navigation.launch(), Err(NavigationError::SequenceExhausted));
            assert_eq!(navigation, before);
        }
        let catalogue = match OriginalNavigation::bundled() {
            Ok(catalogue) => catalogue,
            Err(error) => panic!("bundled navigation: {error:?}"),
        };
        let mut maximum_tick = NavigationState::new(101, 102, 107);
        assert!(
            maximum_tick
                .validate_save(&catalogue, FlightMode::Docked, u64::MAX, 130)
                .is_ok()
        );
        assert_eq!(maximum_tick.launch(), Ok(()));
        assert!(
            maximum_tick
                .validate_save(&catalogue, FlightMode::Launching, u64::MAX, 130)
                .is_ok()
        );
        let mut navigation = NavigationState::new(101, 102, 107);
        navigation.dock_sequence = u64::MAX;
        let before = navigation;
        assert_eq!(
            navigation.step(true, u64::MAX, 130),
            Err(NavigationError::SequenceExhausted)
        );
        assert_eq!(navigation, before);
    }
}

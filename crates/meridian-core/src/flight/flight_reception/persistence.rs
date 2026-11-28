use super::{
    Attempt, FLIGHT_RECEPTION_MAX_BERTH_RECORDS, Fit, FlightReceptionOutcome, FlightView, INVALID,
    MAX_ACTIVE, MAX_RECORDS, Reader, Reading, Sample, SaveError, SavedNavigationSummary, SightBody,
    State, Terminal, World, Writer, same_berth,
};
use crate::docking::Vec3;
use crate::flight::{
    persistence::{read_flight, validate_flight, write_flight},
    practice,
};
use crate::navigation::{DockContext, DockOrigin, SystemId};

impl State {
    pub(in crate::flight) fn encode(&self, w: &mut Writer) {
        self.encode_version(w, 12);
    }
    pub(in crate::flight) fn encode_version(&self, w: &mut Writer, version: u16) {
        w.u8(match self.view {
            FlightView::Front => 0,
            FlightView::Rear => 1,
            FlightView::Left => 2,
            FlightView::Right => 3,
        });
        w.u8(self.attempts.len() as u8);
        for a in &self.attempts {
            a.encode(w, version);
        }
        w.option(self.trace.as_ref(), |w, id| w.u8(*id));
    }
    pub(in crate::flight) fn decode(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        let view = match r.u8()? {
            0 => FlightView::Front,
            1 => FlightView::Rear,
            2 => FlightView::Left,
            3 => FlightView::Right,
            _ => return Err(INVALID),
        };
        let count = usize::from(r.u8()?);
        if count > MAX_RECORDS {
            return Err(INVALID);
        }
        let mut attempts = Vec::with_capacity(count);
        for _ in 0..count {
            attempts.push(Attempt::decode(r, version)?);
        }
        Ok(Self {
            view,
            attempts,
            trace: r.option(|r| r.u8())?,
        })
    }
    pub(in crate::flight) fn validate(
        &self,
        world: &World,
        nav: SavedNavigationSummary,
        practice: Option<practice::Practice>,
        history: &[practice::Practice],
    ) -> Result<(), SaveError> {
        let (own_records, own_active) = self.occupancy();
        let (t04_records, t04_active) = world.navigation_trade.occupancy();
        let (contract_records, contract_active) = world.modular.contract_occupancy();
        let records = own_records
            + contract_records
            + t04_records
            + history.len()
            + usize::from(practice.is_some())
            + world
                .freight_session
                .as_ref()
                .map_or(0, |s| s.state.retained_mission_records());
        let active = own_active
            + contract_active
            + t04_active
            + usize::from(practice.is_some_and(|p| !p.reception_is_terminal()))
            + usize::from(world.active_freight_job().is_some());
        if records > MAX_RECORDS || active > MAX_ACTIVE {
            return Err(INVALID);
        }
        let mut first_complete = None;
        for (i, a) in self.attempts.iter().copied().enumerate() {
            if self.attempts[..i]
                .iter()
                .filter(|old| old.dock.system() == a.dock.system())
                .count()
                >= FLIGHT_RECEPTION_MAX_BERTH_RECORDS
            {
                return Err(INVALID);
            }
            a.validate(world, nav)?;
            if i + 1 < self.attempts.len() && a.terminal.is_none() {
                return Err(INVALID);
            }
            if i > 0 {
                validate_next(self.attempts[i - 1], a)?;
            }
            if a.terminal
                .is_some_and(|t| t.outcome == FlightReceptionOutcome::Complete)
                && first_complete.is_none()
            {
                first_complete = Some(i as u8);
            }
        }
        if self.trace != first_complete {
            return Err(INVALID);
        }
        Ok(())
    }
}
fn validate_next(old: Attempt, next: Attempt) -> Result<(), SaveError> {
    let terminal = old.terminal.ok_or(INVALID)?;
    if terminal.tick > next.accepted_tick {
        return Err(INVALID);
    }
    if let Some(dock) = terminal.dock
        && (dock.sequence() > next.dock.sequence()
            || dock.visit() > next.dock.visit()
            || dock.tick() > next.dock.tick()
            || (dock.sequence() == next.dock.sequence() && dock != next.dock)
            || (dock.sequence() < next.dock.sequence() && terminal.tick >= next.dock.tick()))
    {
        return Err(INVALID);
    }
    Ok(())
}
impl Attempt {
    fn encode(self, w: &mut Writer, version: u16) {
        self.dock.encode_save(w);
        w.u64(self.accepted_tick);
        w.u64(self.fit.inventory);
        w.u64(self.fit.equipment_revision);
        for item in self.fit.items {
            w.option(item.as_ref(), |w, item| w.u64(*item));
        }
        w.option(self.reading.as_ref(), |w, reading| {
            reading.first.encode(w, version);
            reading.last.encode(w, version);
        });
        w.option(self.terminal.as_ref(), |w, t| {
            w.u8(match t.outcome {
                FlightReceptionOutcome::Complete => 0,
                FlightReceptionOutcome::Abandoned => 1,
                FlightReceptionOutcome::Failed => 2,
            });
            w.u64(t.tick);
            w.option(t.dock.as_ref(), |w, d| d.encode_save(w));
        });
    }
    fn decode(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        let dock = DockContext::decode_save(r)?;
        let accepted_tick = r.u64()?;
        let inventory = r.u64()?;
        let equipment_revision = r.u64()?;
        let fit = Fit {
            inventory,
            equipment_revision,
            items: r.array(|r| r.option(|r| r.u64()))?,
        };
        let reading = r.option(|r| {
            Ok(Reading {
                first: Sample::decode(r, version)?,
                last: Sample::decode(r, version)?,
            })
        })?;
        let terminal = r.option(|r| {
            let outcome = match r.u8()? {
                0 => FlightReceptionOutcome::Complete,
                1 => FlightReceptionOutcome::Abandoned,
                2 => FlightReceptionOutcome::Failed,
                _ => return Err(INVALID),
            };
            Ok(Terminal {
                outcome,
                tick: r.u64()?,
                dock: r.option(DockContext::decode_save)?,
            })
        })?;
        Ok(Self {
            dock,
            accepted_tick,
            fit,
            reading,
            terminal,
        })
    }
    fn validate(self, world: &World, nav: SavedNavigationSummary) -> Result<(), SaveError> {
        validate_dock(self.dock, world, nav)?;
        if self.accepted_tick < self.dock.tick() || self.accepted_tick > world.state.tick {
            return Err(INVALID);
        }
        self.fit.validate(world, nav.owner)?;
        if let Some(reading) = self.reading {
            reading.validate(self, world, nav)?;
        }
        if let Some(terminal) = self.terminal {
            self.validate_terminal(terminal, world, nav)?;
        } else if self.must_fail(
            world.player_ship_id(),
            world.combat_snapshot().is_some_and(|s| s.player.destroyed),
        ) {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_terminal(
        self,
        t: Terminal,
        world: &World,
        nav: SavedNavigationSummary,
    ) -> Result<(), SaveError> {
        if t.tick < self.accepted_tick
            || t.tick > world.state.tick
            || self.reading.is_some_and(|r| r.last.flight.tick > t.tick)
        {
            return Err(INVALID);
        }
        if t.outcome == FlightReceptionOutcome::Failed {
            if t.dock.is_some()
                || !self.must_fail(
                    world.player_ship_id(),
                    world.combat_snapshot().is_some_and(|s| s.player.destroyed),
                )
            {
                return Err(INVALID);
            }
            return Ok(());
        }
        let dock = t.dock.ok_or(INVALID)?;
        validate_dock(dock, world, nav)?;
        if !same_berth(self.dock, dock)
            || dock.tick() > t.tick
            || dock.sequence() < self.dock.sequence()
            || dock.visit() < self.dock.visit()
            || dock.tick() < self.dock.tick()
            || (dock.sequence() == self.dock.sequence() && dock != self.dock)
        {
            return Err(INVALID);
        }
        if t.outcome == FlightReceptionOutcome::Complete
            && !(Attempt {
                terminal: None,
                ..self
            })
            .return_ready(dock)
        {
            return Err(INVALID);
        }
        Ok(())
    }
}
fn validate_dock(
    d: DockContext,
    world: &World,
    nav: SavedNavigationSummary,
) -> Result<(), SaveError> {
    if d.owner().seed() != nav.owner
        || d.ship().owner() != d.owner()
        || Some(d.ship().ordinal()) != nav.owner.checked_add(1)
        || ![SystemId::AVENOR, SystemId::RENAS].contains(&d.system())
        || d.sequence() == 0
        || d.sequence() > nav.dock_sequence
        || d.visit() < d.sequence()
        || d.visit() > nav.visit
        || d.tick() > world.state.tick
        || world
            .current_dock_context()
            .is_some_and(|current| current.sequence() == d.sequence() && current != d)
    {
        return Err(INVALID);
    }
    match d.origin() {
        DockOrigin::InitialStart
            if d.sequence() == 1
                && d.visit() == 1
                && d.tick() == 0
                && d.system() == SystemId::AVENOR =>
        {
            Ok(())
        }
        DockOrigin::PhysicalEntry if d.sequence() > 1 && d.visit() > 1 && d.tick() > 0 => Ok(()),
        _ => Err(INVALID),
    }
}
impl Fit {
    fn validate(self, world: &World, owner: u64) -> Result<(), SaveError> {
        let current = world.flight_reception_fit().ok_or(INVALID)?;
        if Some(self.inventory) != owner.checked_add(2)
            || self.inventory != current.inventory
            || self.equipment_revision > current.equipment_revision
        {
            return Err(INVALID);
        }
        for (i, item) in self.items.into_iter().enumerate() {
            if item.is_some_and(|id| Some(id) != owner.checked_add(3 + i as u64))
                || (i < 5 && item.is_none())
                || (item.is_none() && current.items[i].is_some())
            {
                return Err(INVALID);
            }
        }
        Ok(())
    }
}
impl Reading {
    fn validate(
        self,
        attempt: Attempt,
        world: &World,
        nav: SavedNavigationSummary,
    ) -> Result<(), SaveError> {
        if !Self::qualifies(self.first, self.last)
            || self.first.flight.tick <= attempt.accepted_tick
            || self.first.visit <= attempt.dock.visit()
            || self.last.visit > nav.visit
        {
            return Err(INVALID);
        }
        for (sample, end_boundary) in [(self.first, false), (self.last, true)] {
            validate_flight(world, &sample.flight)?;
            if sample.flight.station.is_some()
                || sample.flight.navigation.is_some()
                || sample.flight.tick > world.state.tick
                || sample.physical_tick > world.combat.physical_tick()
                || sample.physical_tick > sample.flight.tick
                || !world.flight_sample_bodies_valid(sample, end_boundary)
                || !world.flight_sample_sight_clear(sample)
                || !same_tick_flight_matches(sample, world, nav)
            {
                return Err(INVALID);
            }
        }
        Ok(())
    }
}
impl Sample {
    fn encode(self, w: &mut Writer, version: u16) {
        write_flight(w, &self.flight);
        w.u64(self.visit);
        w.u64(self.physical_tick);
        for body in self
            .bodies
            .into_iter()
            .take(if version >= 12 { 9 } else { 3 })
        {
            w.option(body.as_ref(), |w, body| {
                w.u8(body.ordinal);
                for value in [body.position.x, body.position.y, body.position.z] {
                    w.u64(value.to_bits());
                }
            });
        }
    }
    fn decode(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        Ok(Self {
            flight: read_flight(r, version)?,
            visit: r.u64()?,
            physical_tick: r.u64()?,
            bodies: {
                let mut bodies = [None; 9];
                for body in bodies.iter_mut().take(if version >= 12 { 9 } else { 3 }) {
                    *body = r.option(|r| {
                        Ok(SightBody {
                            ordinal: r.u8()?,
                            position: Vec3::new(
                                f64::from_bits(r.u64()?),
                                f64::from_bits(r.u64()?),
                                f64::from_bits(r.u64()?),
                            ),
                        })
                    })?;
                }
                bodies
            },
        })
    }
}

fn same_tick_flight_matches(sample: Sample, world: &World, nav: SavedNavigationSummary) -> bool {
    if sample.flight.tick != world.state.tick {
        return true;
    }
    sample.flight.position == world.state.position
        && sample.flight.orientation == world.state.orientation
        && sample.flight.speed == world.state.speed
        && sample.flight.mode == world.state.mode
        && sample.flight.motion_refusal == world.state.motion_refusal
        && sample.visit == nav.visit
        && sample.physical_tick == world.combat.physical_tick()
}

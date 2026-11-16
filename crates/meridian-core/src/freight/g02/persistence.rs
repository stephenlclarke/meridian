use super::{
    AVENOR, AccountData, ActorId, ClueAward, CommanderAccount, CompletionReceipt, Data, DockFacts,
    EquipmentOrigin, GoodsId, Job, JobId, JobStatus, OwnedStarterItem, RENAS, StarterComponent,
    StarterFit, State, StationKind, System, Transfer, validate_account,
};
use crate::navigation::SavedNavigationSummary;
use crate::save::{Reader, SaveError, Writer};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SavedDomainSummary {
    pub(crate) revision: u64,
    pub(crate) freight_revision: u64,
    pub(crate) credits: u32,
    pub(crate) fuel: u16,
    pub(crate) food: u16,
    pub(crate) other_commodities: [u64; 16],
    pub(crate) legal: u8,
    pub(crate) launches: u64,
    pub(crate) completed: u8,
    pub(crate) equipment_revision: u64,
    pub(crate) equipment_paid: u32,
    pub(crate) consumed_missiles: u8,
}
fn write_actor(w: &mut Writer, a: ActorId) {
    w.u64(a.entity);
    w.u64(a.generation);
}
fn read_actor(r: &mut Reader<'_>) -> Result<ActorId, SaveError> {
    Ok(ActorId {
        entity: r.u64()?,
        generation: r.u64()?,
    })
}
fn write_dock(w: &mut Writer, d: DockFacts) {
    w.u64(d.owner);
    w.u64(d.hull);
    w.u8(d.system.galaxy);
    w.u16(d.system.system);
    w.u8(match d.kind {
        StationKind::MainOrbital => 0,
        StationKind::Other => 1,
    });
    write_actor(w, d.station_actor);
    w.u64(d.arrival_sequence);
}
fn read_dock(r: &mut Reader<'_>) -> Result<DockFacts, SaveError> {
    let owner = r.u64()?;
    let hull = r.u64()?;
    let system = System {
        galaxy: r.u8()?,
        system: r.u16()?,
    };
    let kind = match r.u8()? {
        0 => StationKind::MainOrbital,
        1 => StationKind::Other,
        _ => return Err(SaveError::InvalidEncoding),
    };
    Ok(DockFacts {
        owner,
        hull,
        system,
        kind,
        station_actor: read_actor(r)?,
        arrival_sequence: r.u64()?,
    })
}
fn write_job_id(w: &mut Writer, id: JobId) {
    w.u64(id.owner);
    w.u8(id.ordinal);
}
fn read_job_id(r: &mut Reader<'_>) -> Result<JobId, SaveError> {
    Ok(JobId {
        owner: r.u64()?,
        ordinal: r.u8()?,
    })
}
fn write_completion(w: &mut Writer, c: &CompletionReceipt) {
    write_job_id(w, c.job);
    w.u64(c.sequence);
    write_actor(w, c.receiver);
    write_dock(w, c.dock);
}
fn read_completion(r: &mut Reader<'_>) -> Result<CompletionReceipt, SaveError> {
    Ok(CompletionReceipt {
        job: read_job_id(r)?,
        sequence: r.u64()?,
        receiver: read_actor(r)?,
        dock: read_dock(r)?,
    })
}
fn write_item(w: &mut Writer, item: &OwnedStarterItem) {
    w.u64(item.id);
    w.u64(item.owner);
    let (kind, pylon) = match item.component {
        StarterComponent::BasicShield => (0, 0),
        StarterComponent::BasicNavigation => (1, 0),
        StarterComponent::BasicEnergy => (2, 0),
        StarterComponent::BasicDrive => (3, 0),
        StarterComponent::FrontPulseLaser => (4, 0),
        StarterComponent::Missile(pylon) => (5, pylon),
    };
    w.u8(kind);
    w.u8(pylon);
    w.u8(match item.origin {
        EquipmentOrigin::StandardBirth => 0,
        EquipmentOrigin::Unpaid => 1,
    });
    w.option(item.fitted_hull.as_ref(), |w, hull| w.u64(*hull));
}
fn read_item(r: &mut Reader<'_>) -> Result<OwnedStarterItem, SaveError> {
    let id = r.u64()?;
    let owner = r.u64()?;
    let component = match (r.u8()?, r.u8()?) {
        (0, 0) => StarterComponent::BasicShield,
        (1, 0) => StarterComponent::BasicNavigation,
        (2, 0) => StarterComponent::BasicEnergy,
        (3, 0) => StarterComponent::BasicDrive,
        (4, 0) => StarterComponent::FrontPulseLaser,
        (5, pylon @ 0..=2) => StarterComponent::Missile(pylon),
        _ => return Err(SaveError::InvalidEncoding),
    };
    let origin = match r.u8()? {
        0 => EquipmentOrigin::StandardBirth,
        1 => EquipmentOrigin::Unpaid,
        _ => return Err(SaveError::InvalidEncoding),
    };
    Ok(OwnedStarterItem {
        id,
        owner,
        component,
        origin,
        fitted_hull: r.option(|r| r.u64())?,
    })
}
impl CommanderAccount {
    pub(crate) fn encode_save(&self, w: &mut Writer) {
        self.encode_version(w, 12);
    }
    #[cfg(test)]
    pub(crate) fn encode_legacy_version(&self, w: &mut Writer, version: u16) {
        self.encode_version(w, version);
    }
    fn encode_version(&self, w: &mut Writer, version: u16) {
        let d = self.data;
        w.u64(d.owner);
        w.u64(d.hull);
        w.u64(d.revision);
        w.u32(d.credits);
        w.u16(d.other_tonnes);
        w.u16(d.food_tonnes);
        w.u16(d.reserved_food_tonnes);
        w.u16(d.fuel_tenths);
        w.option(d.fit.as_ref(), |w, fit| {
            w.u64(fit.owner);
            w.u64(fit.hull);
            w.u64(fit.inventory);
            for item in &fit.items {
                if version >= 8 {
                    w.option(item.as_ref(), write_item);
                } else if let Some(item) = item {
                    write_item(w, item);
                }
            }
        });
        for quantity in d.other_commodities {
            w.u64(quantity);
        }
        for quantity in d.reserved_commodities {
            w.u64(quantity);
        }
        w.u8(d.legal);
        d.equipment.encode(w);
        if version >= 12 {
            w.u64(d.ammunition_revision);
        }
    }
    pub(crate) fn decode_save(r: &mut Reader<'_>, version: u16) -> Result<Self, SaveError> {
        let mut account = Self {
            data: AccountData {
                owner: r.u64()?,
                hull: r.u64()?,
                revision: r.u64()?,
                credits: r.u32()?,
                other_tonnes: r.u16()?,
                food_tonnes: r.u16()?,
                reserved_food_tonnes: r.u16()?,
                fuel_tenths: r.u16()?,
                fit: r.option(|r| {
                    Ok(StarterFit {
                        owner: r.u64()?,
                        hull: r.u64()?,
                        inventory: r.u64()?,
                        items: r.array(|r| {
                            if version >= 8 {
                                r.option(read_item)
                            } else {
                                read_item(r).map(Some)
                            }
                        })?,
                    })
                })?,
                other_commodities: r.array(|r| r.u64())?,
                reserved_commodities: r.array(|r| r.u64())?,
                legal: r.u8()?,
                equipment: crate::equipment::State::decode(r)?,
                ammunition_revision: if version >= 12 { r.u64()? } else { 0 },
            },
        };
        if version < 12 {
            account.data.ammunition_revision = account.data.fit.map_or(0, |fit| {
                fit.items[5..].iter().filter(|i| i.is_none()).count() as u64
            });
        }
        Ok(account)
    }
    pub(crate) fn validate_saved_domain(
        &self,
        state: &State,
        nav: SavedNavigationSummary,
        epoch: u64,
    ) -> Result<SavedDomainSummary, SaveError> {
        validate_account(&state.data, &self.data)
            .map_err(|_| SaveError::InvalidState("owned account/fit/cargo"))?;
        let fit = self
            .data
            .fit
            .ok_or(SaveError::InvalidState("missing fit"))?;
        if self.data.owner != nav.owner
            || self.data.hull != nav.hull
            || fit.items[3].is_none_or(|item| item.id != nav.drive)
            || self.data.other_tonnes != 0
            || self.data.reserved_food_tonnes != 0
            || self.data.reserved_commodities != [0; 16]
        {
            return Err(SaveError::InvalidState(
                "unsupported or foreign account property",
            ));
        }
        if fit.inventory != nav.owner + 2
            || fit.items.iter().enumerate().any(|(i, item)| {
                (i < 5 && item.is_none())
                    || item.is_some_and(|item| {
                        item.id != nav.owner + 3 + i as u64 || item.fitted_hull != Some(nav.hull)
                    })
            })
        {
            return Err(SaveError::InvalidState("starter identity or fitting"));
        }
        state.validate_saved(nav, epoch)?;
        Ok(SavedDomainSummary {
            revision: self.data.revision,
            freight_revision: state.data.revision,
            credits: self.data.credits,
            fuel: self.data.fuel_tenths,
            food: self.data.food_tonnes,
            other_commodities: self.data.other_commodities,
            legal: self.data.legal,
            launches: nav.launches,
            completed: state.snapshot().completed_jobs,
            equipment_revision: self.data.equipment.revision(),
            equipment_paid: self.data.equipment.paid(),
            consumed_missiles: fit.items[5..].iter().filter(|i| i.is_none()).count() as u8,
        })
    }
}
fn write_job(w: &mut Writer, j: &Job) {
    write_job_id(w, j.id);
    for unit in j.issued {
        w.u64(unit.owner);
        w.u8(unit.serial);
    }
    w.u8(j.held);
    w.u8(j.delivered);
    w.u8(match j.status {
        JobStatus::Active => 0,
        JobStatus::Delivered => 1,
        JobStatus::Abandoned => 2,
    });
    w.u64(j.issue_sequence);
    write_dock(w, j.issuer);
    w.u8(j.transfer_count);
    for t in &j.transfers {
        w.option(t.as_ref(), |w, t| {
            w.u64(t.sequence);
            w.u8(t.mask);
            write_actor(w, t.receiver);
            write_dock(w, t.dock);
        });
    }
    w.option(j.completion.as_ref(), write_completion);
}
fn read_job(r: &mut Reader<'_>) -> Result<Job, SaveError> {
    let id = read_job_id(r)?;
    let issued = r.array(|r| {
        Ok(GoodsId {
            owner: r.u64()?,
            serial: r.u8()?,
        })
    })?;
    let held = r.u8()?;
    let delivered = r.u8()?;
    let status = match r.u8()? {
        0 => JobStatus::Active,
        1 => JobStatus::Delivered,
        2 => JobStatus::Abandoned,
        _ => return Err(SaveError::InvalidEncoding),
    };
    Ok(Job {
        id,
        issued,
        held,
        delivered,
        status,
        issue_sequence: r.u64()?,
        issuer: read_dock(r)?,
        transfer_count: r.u8()?,
        transfers: r.array(|r| {
            r.option(|r| {
                Ok(Transfer {
                    sequence: r.u64()?,
                    mask: r.u8()?,
                    receiver: read_actor(r)?,
                    dock: read_dock(r)?,
                })
            })
        })?,
        completion: r.option(read_completion)?,
    })
}
impl State {
    pub(crate) fn encode_save(&self, w: &mut Writer) {
        w.u64(self.data.owner);
        w.u64(self.data.revision);
        w.u64(self.data.sequence);
        w.u8(self.data.allocated);
        for job in &self.data.jobs {
            w.option(job.as_ref(), write_job);
        }
        w.option(self.data.clue.as_ref(), |w, clue| {
            write_completion(w, &clue.completion)
        });
        w.u64(self.ui_sequence);
    }
    pub(crate) fn decode_save(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            data: Data {
                owner: r.u64()?,
                revision: r.u64()?,
                sequence: r.u64()?,
                allocated: r.u8()?,
                jobs: r.array(|r| r.option(read_job))?,
                clue: r.option(|r| {
                    Ok(ClueAward {
                        completion: read_completion(r)?,
                    })
                })?,
            },
            ui_sequence: r.u64()?,
            pending: None,
        })
    }
    pub(crate) fn saved_review_sequence(&self) -> u64 {
        self.ui_sequence
    }
    pub(crate) fn invalidate_saved_review(&mut self, floor: u64) -> Result<(), SaveError> {
        self.ui_sequence = self
            .ui_sequence
            .max(floor)
            .checked_add(1)
            .ok_or(SaveError::SequenceExhausted)?;
        self.pending = None;
        Ok(())
    }
    fn validate_saved(&self, nav: SavedNavigationSummary, epoch: u64) -> Result<(), SaveError> {
        let invalid = SaveError::InvalidState("freight source or job chronology");
        if self.data.owner != nav.owner
            || self.data.allocated > 2
            || self.data.revision != self.data.sequence
            || self.ui_sequence < self.data.sequence
        {
            return Err(invalid);
        }
        let mut sequence = 0_u64;
        let mut first_completion = None;
        let mut previous_completion: Option<CompletionReceipt> = None;
        for (index, job) in self.data.jobs.iter().enumerate() {
            if index >= usize::from(self.data.allocated) {
                if job.is_some() {
                    return Err(invalid);
                }
                continue;
            }
            let job = job.as_ref().ok_or(invalid)?;
            if job.id
                != (JobId {
                    owner: nav.owner,
                    ordinal: index as u8,
                })
                || job
                    .issued
                    .iter()
                    .enumerate()
                    .any(|(i, u)| u.owner != nav.owner || usize::from(u.serial) != index * 8 + i)
                || job.status == JobStatus::Active && index + 1 != usize::from(self.data.allocated)
            {
                return Err(invalid);
            }
            if let Some(c) = previous_completion
                && (job.issuer.arrival_sequence <= c.dock.arrival_sequence
                    || job.issuer.station_actor.generation <= c.dock.station_actor.generation)
            {
                return Err(invalid);
            }
            sequence = validate_job(job, sequence, nav, epoch)?;
            if let Some(c) = job.completion {
                if first_completion.is_none() {
                    first_completion = Some(ClueAward { completion: c });
                }
                previous_completion = Some(c);
            }
        }
        if sequence != self.data.sequence || self.data.clue != first_completion {
            return Err(invalid);
        }
        Ok(())
    }
}
fn validate_dock(
    d: DockFacts,
    system: System,
    nav: SavedNavigationSummary,
) -> Result<(), SaveError> {
    if d.owner != nav.owner
        || d.hull != nav.hull
        || d.system != system
        || d.kind != StationKind::MainOrbital
        || d.arrival_sequence == 0
        || d.arrival_sequence > nav.dock_sequence
        || d.station_actor.entity != 1
        || d.station_actor.generation == 0
        || d.station_actor.generation > nav.visit
    {
        return Err(SaveError::InvalidState("historical freight dock identity"));
    }
    Ok(())
}
fn advance(sequence: u64) -> Result<u64, SaveError> {
    sequence.checked_add(1).ok_or(SaveError::SequenceExhausted)
}
fn validate_job(
    j: &Job,
    prior: u64,
    nav: SavedNavigationSummary,
    epoch: u64,
) -> Result<u64, SaveError> {
    let invalid = SaveError::InvalidState("freight transfer/custody/completion");
    let mut sequence = advance(prior)?;
    if j.issue_sequence != sequence || j.transfer_count > 8 {
        return Err(invalid);
    }
    validate_dock(j.issuer, AVENOR, nav)?;
    let mut remaining = u8::MAX;
    let mut delivered = 0_u8;
    let mut last: Option<Transfer> = None;
    for (i, t) in j.transfers.iter().enumerate() {
        if i >= usize::from(j.transfer_count) {
            if t.is_some() {
                return Err(invalid);
            }
            continue;
        }
        let t = t.as_ref().ok_or(invalid)?;
        sequence = advance(sequence)?;
        validate_dock(t.dock, RENAS, nav)?;
        if remaining == 0
            || t.mask == 0
            || t.mask & !remaining != 0
            || t.mask != remaining && t.mask != 1_u8 << remaining.trailing_zeros()
            || t.sequence != sequence
            || t.receiver.entity != 3
            || t.receiver.generation == 0
            || t.receiver.generation > epoch
            || t.dock.arrival_sequence <= j.issuer.arrival_sequence
            || t.dock.station_actor.generation <= j.issuer.station_actor.generation
            || nav.jumps == 0
        {
            return Err(invalid);
        }
        if let Some(previous) = last
            && (t.dock.arrival_sequence < previous.dock.arrival_sequence
                || t.receiver.generation < previous.receiver.generation)
        {
            return Err(invalid);
        }
        remaining &= !t.mask;
        delivered |= t.mask;
        last = Some(*t);
    }
    if j.delivered != delivered {
        return Err(invalid);
    }
    match j.status {
        JobStatus::Active if remaining != 0 && j.held == remaining && j.completion.is_none() => {}
        JobStatus::Abandoned if remaining != 0 && j.held == 0 && j.completion.is_none() => {
            sequence = advance(sequence)?;
        }
        JobStatus::Delivered if remaining == 0 && j.held == 0 => {
            let t = last.ok_or(invalid)?;
            if j.completion
                != Some(CompletionReceipt {
                    job: j.id,
                    sequence: t.sequence,
                    receiver: t.receiver,
                    dock: t.dock,
                })
            {
                return Err(invalid);
            }
        }
        _ => return Err(invalid),
    }
    Ok(sequence)
}

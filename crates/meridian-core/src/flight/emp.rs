//! Distinct owned EMP electrical weapon. A snapshot or saved descriptor grants no command authority.
mod equipment;
mod meter;
mod persistence;
mod policy;
mod replay;
mod runtime;
mod scene;
mod snapshot;
mod validation;
use super::{FlightMode, World};
use crate::navigation::DockContext;
use crate::save::{Reader, SaveError, Writer};
pub use equipment::{EmpEquipmentOffer, PreparedEmpEquipment};
pub use policy::EmpGrades;
pub use runtime::EmpChargePhase;
pub(in crate::flight) use runtime::Frame;
pub use snapshot::EmpSnapshot;
use std::sync::Arc;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmpAxis {
    Power,
    Capacity,
    Recharge,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmpEquipmentAction {
    Purchase,
    Fit { item: u64 },
    Store { item: u64 },
    Upgrade { item: u64, axis: EmpAxis },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmpError {
    NoCommander,
    Paused,
    BerthRequired,
    StaleReview,
    MissingItem,
    AlreadyOwned,
    NoChange,
    MaximumGrade,
    TechnologyUnavailable,
    InsufficientFunds,
    UtilityCapacity,
    IncompatibleEnergy,
    FlightRequired,
    NoTarget,
    InvalidContact,
    Resources,
    Capacity,
    NoRecentAggressor,
    ProtectedTarget,
    PaidBaseRequired,
    ActiveTravel,
    Save(SaveError),
}
impl core::fmt::Display for EmpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "EMP refused: {self:?}")
    }
}
impl std::error::Error for EmpError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmpEquipmentReview {
    action: EmpEquipmentAction,
    dock: DockContext,
    tick: u64,
    epoch: u64,
    revision: u64,
}
/// Actual paid identity. The bank starts empty; a readout never charges it or fits the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmpProvenance {
    Paid,
    Xyzzy,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedEmp {
    pub item: u64,
    pub owner: u64,
    pub hull: u64,
    pub fitted: bool,
    pub grades: EmpGrades,
    pub purchase_dock: DockContext,
    pub purchase_tick: u64,
    pub acquisition_revision: u64,
    pub paid_tenths: u32,
    pub provenance: EmpProvenance,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EquipmentEvent {
    secret: bool,
    action: EmpEquipmentAction,
    dock: DockContext,
    tick: u64,
    revision: u64,
}
impl EquipmentEvent {
    fn terms(self, grades: &mut EmpGrades) -> Result<(u32, u8), SaveError> {
        let offer = match self.action {
            EmpEquipmentAction::Purchase => (policy::BASE_PRICE, policy::BASE_TECHNOLOGY),
            EmpEquipmentAction::Upgrade { axis, .. } => {
                let offer = grades.upgrade_offer(axis).map_err(|_| INVALID)?;
                *grades = grades.advancing(axis).map_err(|_| INVALID)?;
                offer
            }
            EmpEquipmentAction::Store { .. } if self.secret => return Err(INVALID),
            _ => (0, 0),
        };
        Ok(if self.secret { (0, 0) } else { offer })
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct State {
    equipment: Arc<Vec<EquipmentEvent>>,
    meter: meter::Meter,
    meter_clock: u64,
    bank_clock: u64,
    main_present: bool,
    charge: Option<runtime::Charge>,
    pulses: Arc<Vec<runtime::Pulse>>,
    episodes: Arc<Vec<runtime::Episode>>,
    held_tick: Option<u64>,
    posted_tick: Option<u64>,
}
const INVALID: SaveError = SaveError::InvalidState("owned EMP provenance, fit or grade history");
const MAX_EQUIPMENT_EVENTS: usize = 128;
const MAX_PULSES: usize = 16;
impl State {
    pub(in crate::flight) fn owned(&self) -> Result<Option<OwnedEmp>, SaveError> {
        let mut item: Option<OwnedEmp> = None;
        for event in self.equipment.iter() {
            match event.action {
                EmpEquipmentAction::Purchase => {
                    if item.is_some() {
                        return Err(INVALID);
                    }
                    item = Some(OwnedEmp {
                        item: event
                            .dock
                            .owner()
                            .seed()
                            .checked_add(400011)
                            .ok_or(INVALID)?,
                        owner: event.dock.owner().seed(),
                        hull: event.dock.ship().ordinal(),
                        fitted: true,
                        grades: EmpGrades::default(),
                        purchase_dock: event.dock,
                        purchase_tick: event.tick,
                        acquisition_revision: event.revision,
                        paid_tenths: if event.secret { 0 } else { policy::BASE_PRICE },
                        provenance: if event.secret {
                            EmpProvenance::Xyzzy
                        } else {
                            EmpProvenance::Paid
                        },
                    });
                }
                action => {
                    let current = item.as_mut().ok_or(INVALID)?;
                    match action {
                        EmpEquipmentAction::Fit { item }
                            if item == current.item && !current.fitted =>
                        {
                            current.fitted = true
                        }
                        EmpEquipmentAction::Store { item }
                            if item == current.item && current.fitted =>
                        {
                            current.fitted = false
                        }
                        EmpEquipmentAction::Upgrade { item, axis } if item == current.item => {
                            if !event.secret && current.provenance != EmpProvenance::Paid {
                                return Err(INVALID);
                            }
                            current.grades = current.grades.advancing(axis).map_err(|_| INVALID)?;
                        }
                        _ => return Err(INVALID),
                    }
                }
            }
        }
        Ok(item)
    }
    fn terms(
        &self,
        world: &World,
        account: &crate::freight::g02::CommanderAccount,
    ) -> Result<(u32, u64), SaveError> {
        if self.equipment.len() > MAX_EQUIPMENT_EVENTS {
            return Err(SaveError::TooLarge);
        }
        let mut paid = 0_u32;
        let mut grades = EmpGrades::default();
        let mut previous: Option<EquipmentEvent> = None;
        for event in self.equipment.iter() {
            super::ammunition::validate_berth_event(
                world,
                account,
                event.dock,
                event.tick,
                event.revision,
                previous.map(|e| (e.dock.sequence(), e.tick, e.revision)),
            )?;
            if event.secret
                && !world
                    .xyzzy
                    .authorizes(event.dock, event.tick, event.revision)
            {
                return Err(INVALID);
            }
            let (price, required) = event.terms(&mut grades)?;
            let site = super::services::service_site(event.dock.system()).ok_or(INVALID)?;
            if crate::equipment::definition()
                .map_err(|_| INVALID)?
                .offer(site)
                .technology
                < required
            {
                return Err(INVALID);
            }
            paid = paid.checked_add(price).ok_or(INVALID)?;
            previous = Some(*event);
        }
        self.owned()?;
        Ok((paid, self.equipment.len() as u64))
    }
    pub(super) fn utility_mounts(&self) -> u16 {
        u16::from(self.owned().ok().flatten().is_some_and(|item| item.fitted))
    }
    pub(super) fn utility_mounts_at_revision(&self, revision: u64) -> u16 {
        let mut fitted = false;
        for event in self
            .equipment
            .iter()
            .take_while(|event| event.revision <= revision)
        {
            match event.action {
                EmpEquipmentAction::Purchase | EmpEquipmentAction::Fit { .. } => fitted = true,
                EmpEquipmentAction::Store { .. } => fitted = false,
                EmpEquipmentAction::Upgrade { .. } => {}
            }
        }
        u16::from(fitted)
    }
}

pub(crate) const EMP_RESOURCE_SCALE: u32 = policy::SCALE;
/// A distinct EMP origin; it is neither an ECM activation nor ordinary laser/missile ordnance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmpOrigin {
    pub owner: super::CombatIdentity,
    pub item: u64,
    pub pulse: u16,
}
impl EmpOrigin {
    pub(crate) fn encode(self, w: &mut Writer) {
        w.u64(self.owner.owner);
        w.u64(self.owner.hull);
        w.u8(self.owner.system.galaxy);
        w.u16(self.owner.system.system);
        w.u64(self.owner.visit);
        w.u64(self.item);
        w.u16(self.pulse);
    }
    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            owner: super::CombatIdentity {
                owner: r.u64()?,
                hull: r.u64()?,
                system: crate::navigation::SystemId {
                    galaxy: r.u8()?,
                    system: r.u16()?,
                },
                visit: r.u64()?,
            },
            item: r.u64()?,
            pulse: r.u16()?,
        })
    }
}

pub(in crate::flight) const EMP_HEAT_FIFTHS: u16 = policy::HEAT_FIFTHS;
pub(in crate::flight) const EMP_COOLDOWN_TICKS: u8 = policy::COOLDOWN_TICKS;
// Physical tick plus optional (front facing, incident power hundredths, shield absorbed hundredths).
type ResourceEffect = (u64, Option<(bool, u16, u16)>);
impl State {
    pub(in crate::flight) fn resource_effects(&self) -> Vec<ResourceEffect> {
        self.pulses
            .iter()
            .map(|pulse| {
                (
                    pulse.release.physical_tick,
                    pulse.contacts.iter().find_map(|contact| {
                        (contact.victim == scene::Victim::Player).then_some((
                            contact.front,
                            contact.power_hundredths,
                            pulse.player_shield_absorbed,
                        ))
                    }),
                )
            })
            .collect()
    }
}

impl State {
    pub(in crate::flight) fn proof_grows_from(&self, old: &Self) -> bool {
        self.equipment.len() > old.equipment.len()
            || self.pulses.len() > old.pulses.len()
            || self.episodes.len() > old.episodes.len()
            || (self.meter.disabled_at.is_some() && old.meter.disabled_at.is_none())
            || match (&self.charge, &old.charge) {
                (Some(_), None) => true,
                (Some(new), Some(old)) => {
                    (new.ended_at.is_some() && old.ended_at.is_none())
                        || (new.pulse.is_some() && old.pulse.is_none())
                }
                _ => false,
            }
            || self
                .episodes
                .iter()
                .zip(old.episodes.iter())
                .any(|(n, o)| n.recovered_at.is_some() && o.recovered_at.is_none())
    }
}

#[cfg(test)]
mod tests;

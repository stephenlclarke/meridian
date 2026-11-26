//! Original common-project passenger accommodation layouts.
use super::{
    Event, Ledger, ModuleAction, ModuleError, ModuleKind, ModuleSlot, OwnedModule, State, World,
};
use crate::navigation::{DockContext, SystemId};
use crate::station_services::{self as services, AccountProjection, Reservations};

pub const CABIN_TONNES: u16 = 5;
pub const PASSAGE_REWARD_TENTHS: u32 = 6_100;
pub const PASSAGE_CLUE_ID: &str = "l337.unwritten-route/clue/cabin-journey-family-trace";
const MAX_PASSAGE_FUEL: u16 = 410;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassengerAction {
    RequestReplies,
    Board,
    Receive,
    Cancel,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PassageStatus {
    #[default]
    Waiting,
    Refused,
    Accepted,
    Aboard,
    Delivered,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassengerIdentity {
    pub owner: u64,
    pub batch: &'static str,
    pub person: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassengerJourneySnapshot {
    pub people: [PassengerIdentity; 2],
    pub status: PassageStatus,
    pub requests: u8,
    /// One private place (ordinal zero) in each separately identified cabin.
    pub cabins: Option<[u64; 2]>,
    pub reply_is_current: bool,
    pub terminal: Option<PassageTerminalReceipt>,
    pub clue: Option<PassageTerminalReceipt>,
}

/// An owner-bound receipt reconstructed only from validated accepted boarding and terminal
/// events, including the independent accepted berth proofs and the exact private cabin IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassageTerminalReceipt {
    pub owner: u64,
    pub hull: u64,
    pub boarding_revision: u64,
    pub terminal_revision: u64,
    pub boarding: DockContext,
    pub receiving: DockContext,
    pub cabins: [u64; 2],
    pub status: PassageStatus,
}

impl PassageTerminalReceipt {
    pub const fn clue_id(self) -> Option<&'static str> {
        if matches!(self.status, PassageStatus::Delivered) {
            Some(PASSAGE_CLUE_ID)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CabinStatus {
    pub item: u64,
    pub grade: CabinGrade,
    pub fitted: bool,
    pub life_support_working: bool,
    pub stasis_working: bool,
    pub passenger: Option<PassengerIdentity>,
    pub free_awake_places: u16,
    pub free_stasis_places: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Offer {
    dock: DockContext,
    epoch: u64,
    fit_revision: u64,
}

#[derive(Debug, Default)]
pub(super) struct Passage {
    status: PassageStatus,
    requests: u8,
    cabins: Option<[u64; 2]>,
    offer: Option<Offer>,
    boarding: Option<(u64, DockContext)>,
    terminal: Option<PassageTerminalReceipt>,
    pub(super) events: u64,
}

impl Passage {
    pub(super) fn occupied(&self, item: u64) -> bool {
        self.status == PassageStatus::Aboard && self.cabins.is_some_and(|ids| ids.contains(&item))
    }

    pub(super) fn reward(&self) -> u32 {
        if self.status == PassageStatus::Delivered {
            PASSAGE_REWARD_TENTHS
        } else {
            0
        }
    }

    fn current(&self, dock: Option<DockContext>, epoch: u64, revision: u64) -> bool {
        self.offer.is_some_and(|offer| {
            Some(offer.dock) == dock && offer.epoch == epoch && offer.fit_revision == revision
        })
    }

    fn snapshot(
        &self,
        owner: u64,
        dock: Option<DockContext>,
        epoch: u64,
        revision: u64,
    ) -> PassengerJourneySnapshot {
        PassengerJourneySnapshot {
            people: [
                PassengerIdentity {
                    owner,
                    batch: "avenor-cabin-journey-01",
                    person: "mira-talen",
                    name: "Mira Talen",
                },
                PassengerIdentity {
                    owner,
                    batch: "avenor-cabin-journey-01",
                    person: "oren-vey",
                    name: "Oren Vey",
                },
            ],
            status: self.status,
            requests: self.requests,
            cabins: self.cabins,
            reply_is_current: matches!(
                self.status,
                PassageStatus::Accepted | PassageStatus::Refused
            ) && self.current(dock, epoch, revision),
            terminal: self.terminal,
            clue: self.terminal.filter(|receipt| receipt.clue_id().is_some()),
        }
    }
}

/// Passage actions need no caller-supplied passenger, cabin, fuel or epoch.
/// Receipt data uses the existing event item field only for this event family.
pub(super) fn receipt_data(world: &World, action: PassengerAction) -> Result<u64, ModuleError> {
    if matches!(action, PassengerAction::Receive | PassengerAction::Cancel) {
        return Ok(0);
    }
    let epoch = world
        .freight_session
        .as_ref()
        .ok_or(ModuleError::NoCommander)?
        .epoch();
    let fuel = world
        .commander_account
        .as_ref()
        .ok_or(ModuleError::NoCommander)?
        .fuel_tenths();
    pack_receipt(epoch, fuel)
}

pub(super) fn pack_receipt(epoch: u64, fuel: u16) -> Result<u64, ModuleError> {
    if epoch == 0 || fuel > MAX_PASSAGE_FUEL {
        return Err(ModuleError::PassageConsent);
    }
    epoch
        .checked_mul(1 << 16)
        .and_then(|value| value.checked_add(u64::from(fuel)))
        .ok_or(ModuleError::SequenceExhausted)
}

pub(super) fn receipt_epoch(item: u64) -> u64 {
    item >> 16
}

pub(super) fn receipt_fuel(item: u64) -> Option<u16> {
    let fuel = item as u16;
    (receipt_epoch(item) > 0 && fuel <= MAX_PASSAGE_FUEL).then_some(fuel)
}

impl Ledger {
    pub(super) fn apply_passage(
        &mut self,
        event: Event,
        action: PassengerAction,
    ) -> Result<(), ModuleError> {
        use PassageStatus as Status;
        use PassengerAction as Action;
        if matches!(self.passage.status, Status::Delivered | Status::Cancelled) {
            return Err(ModuleError::PassageClosed);
        }
        match action {
            Action::RequestReplies | Action::Board => {
                if event.dock.system() != SystemId::AVENOR || self.passage.status == Status::Aboard
                {
                    return Err(ModuleError::BerthRequired);
                }
                let fuel = receipt_fuel(event.item).ok_or(ModuleError::PassageConsent)?;
                let epoch = receipt_epoch(event.item);
                let mut ids = self
                    .items
                    .iter()
                    .filter(|item| fitted_cabin(item).is_some_and(CabinGrade::supports_awake))
                    .map(|item| item.id);
                let eligible = ids.next().zip(ids.next()).map(|(a, b)| [a, b]);
                let eligible = eligible.filter(|_| {
                    fuel >= crate::freight::g02::DEPARTURE_FUEL_TENTHS
                        && self.jump_reach_ly() >= crate::freight::g02::ROUTE_DISTANCE_LY
                });
                if action == Action::RequestReplies {
                    if self.passage.requests >= 64 {
                        return Err(ModuleError::Capacity);
                    }
                    self.passage.requests += 1;
                    self.passage.cabins = eligible;
                    self.passage.status = if eligible.is_some() {
                        Status::Accepted
                    } else {
                        Status::Refused
                    };
                    self.passage.offer = Some(Offer {
                        dock: event.dock,
                        epoch,
                        fit_revision: self.revision,
                    });
                } else if self.passage.status != Status::Accepted
                    || self.passage.cabins != eligible
                    || !self.passage.current(Some(event.dock), epoch, self.revision)
                {
                    return Err(ModuleError::PassageConsent);
                } else {
                    self.passage.status = Status::Aboard;
                    self.passage.boarding = Some((event.revision, event.dock));
                }
            }
            Action::Receive | Action::Cancel => {
                if event.item != 0 || self.passage.status != Status::Aboard {
                    return Err(ModuleError::PassageConsent);
                }
                let source = self.passage.offer.ok_or(ModuleError::PassageConsent)?.dock;
                let system = if action == Action::Receive {
                    SystemId::RENAS
                } else {
                    SystemId::AVENOR
                };
                if event.dock.system() != system
                    || (action == Action::Receive && event.dock.visit() <= source.visit())
                {
                    return Err(ModuleError::BerthRequired);
                }
                self.passage.status = if action == Action::Receive {
                    Status::Delivered
                } else {
                    Status::Cancelled
                };
                let (boarding_revision, boarding) =
                    self.passage.boarding.ok_or(ModuleError::PassageConsent)?;
                self.passage.terminal = Some(PassageTerminalReceipt {
                    owner: event.dock.owner().seed(),
                    hull: event.dock.ship().ordinal(),
                    boarding_revision,
                    terminal_revision: event.revision,
                    boarding,
                    receiving: event.dock,
                    cabins: self.passage.cabins.ok_or(ModuleError::PassageConsent)?,
                    status: self.passage.status,
                });
            }
        }
        self.passage.events = self
            .passage
            .events
            .checked_add(1)
            .ok_or(ModuleError::SequenceExhausted)?;
        Ok(())
    }
}

impl State {
    /// The World only keeps admitted events. This inexpensive projection avoids full account,
    /// station and navigation validation every time flight or trading reads the remaining hold.
    /// Candidate saves separately reconcile the complete ledger before publication.
    pub(in crate::flight) fn cabin_tonnes(&self) -> Result<u16, services::Error> {
        let mut fitted = std::collections::BTreeMap::new();
        for event in &self.events {
            match event.action {
                ModuleAction::Purchase {
                    kind: ModuleKind::Cabin(_),
                } => {
                    fitted.insert(event.item, true);
                }
                ModuleAction::Store { item } => {
                    if let Some(value) = fitted.get_mut(&item) {
                        *value = false;
                    }
                }
                ModuleAction::Install { item, .. } => {
                    if let Some(value) = fitted.get_mut(&item) {
                        *value = true;
                    }
                }
                _ => {}
            }
        }
        u16::try_from(fitted.values().filter(|value| **value).count())
            .ok()
            .and_then(|count| count.checked_mul(CABIN_TONNES))
            .ok_or(services::Error::InvalidAccount)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CabinGrade {
    PackedStasis,
    Economy,
    Standard,
    Luxury,
    SingleStateroom,
    DoubleStateroom,
}

impl CabinGrade {
    pub const ALL: [Self; 6] = [
        Self::PackedStasis,
        Self::Economy,
        Self::Standard,
        Self::Luxury,
        Self::SingleStateroom,
        Self::DoubleStateroom,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::PackedStasis => "Packed Stasis Cabin",
            Self::Economy => "Economy Cabin",
            Self::Standard => "Standard Cabin",
            Self::Luxury => "Luxury Cabin",
            Self::SingleStateroom => "Single Stateroom",
            Self::DoubleStateroom => "Double Stateroom",
        }
    }

    pub const fn places(self) -> u16 {
        [20, 8, 4, 2, 1, 2][self as usize]
    }

    pub const fn price_tenths(self) -> u32 {
        [10_000, 25_000, 50_000, 100_000, 200_000, 200_000][self as usize]
    }

    pub const fn supports_awake(self) -> bool {
        !matches!(self, Self::PackedStasis)
    }
}

pub(super) fn fitted_cabin(item: &OwnedModule) -> Option<CabinGrade> {
    if item.slot != Some(ModuleSlot::PassengerCabin) {
        return None;
    }
    match item.kind {
        ModuleKind::Cabin(grade) => Some(grade),
        _ => None,
    }
}

impl Ledger {
    pub(in crate::flight) fn cabin_tonnes(&self) -> u16 {
        self.items.iter().filter_map(fitted_cabin).count() as u16 * CABIN_TONNES
    }
}

pub(in crate::flight) fn add_cabin_mass(
    projection: AccountProjection,
    tonnes: u16,
) -> Result<AccountProjection, services::Error> {
    Reservations {
        capacity: tonnes,
        commodity: None,
    }
    .apply(projection)
}

impl World {
    pub(in crate::flight) fn cabin_tonnes(&self) -> Result<u16, services::Error> {
        self.modular.cabin_tonnes()
    }

    pub fn passenger_journey_snapshot(&self) -> Option<PassengerJourneySnapshot> {
        let account = self.commander_account.as_ref()?;
        let ledger = self.modular.ledger(self, account).ok()?;
        Some(ledger.passage.snapshot(
            self.combat_identity()?.owner,
            self.current_dock_context(),
            self.freight_session.as_ref()?.epoch(),
            ledger.revision,
        ))
    }

    /// The original life-support and stasis flags are healthy at purchase. Meridian does not
    /// yet have independent cabin-component deterioration, repair or loss receipts.
    pub fn passenger_cabins(&self) -> Option<Vec<CabinStatus>> {
        let account = self.commander_account.as_ref()?;
        let ledger = self.modular.ledger(self, account).ok()?;
        let journey = ledger.passage.snapshot(
            self.combat_identity()?.owner,
            self.current_dock_context(),
            self.freight_session.as_ref()?.epoch(),
            ledger.revision,
        );
        Some(
            ledger
                .items
                .iter()
                .filter_map(|item| {
                    let ModuleKind::Cabin(grade) = item.kind else {
                        return None;
                    };
                    let fitted = item.slot == Some(ModuleSlot::PassengerCabin);
                    let passenger = if ledger.passage.occupied(item.id) {
                        journey
                            .cabins
                            .and_then(|ids| ids.iter().position(|id| *id == item.id))
                            .map(|index| journey.people[index])
                    } else {
                        None
                    };
                    let available = if fitted && passenger.is_none() {
                        grade.places()
                    } else {
                        0
                    };
                    Some(CabinStatus {
                        item: item.id,
                        grade,
                        fitted,
                        life_support_working: true,
                        stasis_working: true,
                        passenger,
                        free_awake_places: if grade.supports_awake() { available } else { 0 },
                        free_stasis_places: available,
                    })
                })
                .collect(),
        )
    }
}

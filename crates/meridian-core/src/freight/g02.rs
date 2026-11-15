//! Current G02 consented-freight rules, not a source of physical station authority.
//!
//! Only the owning World may enter the crate-private command/publication boundary.
//! Its facts must come from actual contact, dock, equipment and profile state. The
//! World navigation/contact lifecycle supplies them; prototype worlds cannot. See WORLD-G02.md.

mod persistence;
use super::{CLUE_ID, REWARD_TENTHS};
use crate::station_services as services;
pub(crate) use persistence::SavedDomainSummary;

pub const SOURCE_ID: &str = "l337.contract/consented-freight";
pub const BATCH_ID: &str = "avenor-utility-parts-01";
pub const CONTENT_DIGEST: &str = "37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572";
pub const AVENOR: System = System {
    galaxy: 0,
    system: 7,
};
pub const RENAS: System = System {
    galaxy: 0,
    system: 185,
};
pub const ROUTE_DISTANCE_LY: f64 = 3.364975176728651;
pub const DEPARTURE_FUEL_TENTHS: u16 = 34;
const HOLD_TONNES: u16 = 92; // Current supported first-playable SH025, not all possible G02 ships.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct System {
    pub galaxy: u8,
    pub system: u16,
}

/// A requested entity identity is data, never consent or a docking permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorId {
    pub entity: u64,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobId {
    owner: u64,
    ordinal: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoodsId {
    owner: u64,
    serial: u8,
}

impl GoodsId {
    pub fn serial(self) -> u8 {
        self.serial
    }
    pub fn owner(self) -> u64 {
        self.owner
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Active,
    Delivered,
    Abandoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactDefinition {
    Arlen,
    RenasUtilityCrew,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationKind {
    MainOrbital,
    Other,
}

/// Internal projection of a current World-owned admission, NOT a receipt factory.
/// World must never construct this from Snapshot.mode or a position comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DockFacts {
    pub(crate) owner: u64,
    pub(crate) hull: u64,
    pub(crate) system: System,
    pub(crate) kind: StationKind,
    pub(crate) station_actor: ActorId,
    pub(crate) arrival_sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContactFacts {
    pub(crate) actor: ActorId,
    pub(crate) station_actor: ActorId,
    pub(crate) definition: ContactDefinition,
    pub(crate) roster_revision: u64,
}

/// No public constructor or public command can manufacture these World facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorldFacts {
    pub(crate) epoch: u64,
    pub(crate) paused: bool,
    pub(crate) content_digest: Option<&'static str>,
    pub(crate) dock: Option<DockFacts>,
    pub(crate) contact: Option<ContactFacts>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EquipmentOrigin {
    StandardBirth,
    Unpaid,
}

/// IDs allocated once by the owning fresh-profile boundary, never a decoder or mission command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FreshCommanderIds {
    pub(crate) owner: u64,
    pub(crate) hull: u64,
    pub(crate) inventory: u64,
    pub(crate) items: [u64; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StarterComponent {
    BasicShield,
    BasicNavigation,
    BasicEnergy,
    BasicDrive,
    FrontPulseLaser,
    Missile(u8),
}

const STARTER_COMPONENTS: [StarterComponent; 8] = [
    StarterComponent::BasicShield,
    StarterComponent::BasicNavigation,
    StarterComponent::BasicEnergy,
    StarterComponent::BasicDrive,
    StarterComponent::FrontPulseLaser,
    StarterComponent::Missile(0),
    StarterComponent::Missile(1),
    StarterComponent::Missile(2),
];

/// Read-only owned property. Construction is private to the actual fresh-account transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OwnedStarterItem {
    id: u64,
    owner: u64,
    component: StarterComponent,
    origin: EquipmentOrigin,
    fitted_hull: Option<u64>,
}

impl OwnedStarterItem {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }
    pub(crate) fn owner(&self) -> u64 {
        self.owner
    }
    pub(crate) fn component(&self) -> StarterComponent {
        self.component
    }
    pub(crate) fn fitted_hull(&self) -> Option<u64> {
        self.fitted_hull
    }
}

/// Actual fixed starter property retained with the account, not a caller-projected valid-fit flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StarterFit {
    owner: u64,
    hull: u64,
    inventory: u64,
    items: [OwnedStarterItem; 8],
}

fn validate_birth_ids(ids: FreshCommanderIds) -> Result<(), Error> {
    let mut all = [0; 11];
    all[0] = ids.owner;
    all[1] = ids.hull;
    all[2] = ids.inventory;
    all[3..].copy_from_slice(&ids.items);
    for (index, id) in all.iter().enumerate() {
        if *id == 0 || all[..index].contains(id) {
            return Err(Error::InvalidFit);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AccountData {
    owner: u64,
    hull: u64,
    revision: u64,
    credits: u32,
    other_tonnes: u16,
    food_tonnes: u16,
    reserved_food_tonnes: u16,
    fuel_tenths: u16,
    fit: Option<StarterFit>,
}

/// One World-owned account, not a second mission wallet or an authority actor.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommanderAccount {
    data: AccountData,
}

impl CommanderAccount {
    /// Called once by the real fresh-profile/owned-starter creation path, never on load/restock.
    pub(crate) fn born_sh025(ids: FreshCommanderIds) -> Result<Self, Error> {
        validate_birth_ids(ids)?;
        let items = core::array::from_fn(|index| OwnedStarterItem {
            id: ids.items[index],
            owner: ids.owner,
            component: STARTER_COMPONENTS[index],
            origin: EquipmentOrigin::StandardBirth,
            fitted_hull: Some(ids.hull),
        });
        Ok(Self {
            data: AccountData {
                owner: ids.owner,
                hull: ids.hull,
                revision: 0,
                credits: 7200,
                other_tonnes: 0,
                food_tonnes: 0,
                reserved_food_tonnes: 0,
                fuel_tenths: 130,
                fit: Some(StarterFit {
                    owner: ids.owner,
                    hull: ids.hull,
                    inventory: ids.inventory,
                    items,
                }),
            },
        })
    }

    pub(crate) fn starter_items(&self) -> Option<&[OwnedStarterItem; 8]> {
        self.data.fit.as_ref().map(|fit| &fit.items)
    }

    pub(crate) fn credits_tenths(&self) -> u32 {
        self.data.credits
    }
    pub(crate) fn fuel_tenths(&self) -> u16 {
        self.data.fuel_tenths
    }

    /// Accounting only: World must first validate its actual shop/service request.
    /// No public reward/deposit function exists; G02 has no acceptance charge.
    pub(crate) fn debit(&mut self, amount: u32) -> Result<(), Error> {
        let credits = self
            .data
            .credits
            .checked_sub(amount)
            .ok_or(Error::InsufficientFunds)?;
        let revision = self
            .data
            .revision
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        self.data.credits = credits;
        self.data.revision = revision;
        Ok(())
    }

    pub(crate) fn service_projection(
        &self,
        freight: &State,
    ) -> Result<services::AccountProjection, services::Error> {
        validate_account(&freight.data, &self.data).map_err(service_error)?;
        Ok(services::AccountProjection {
            owner: self.data.owner,
            hull: self.data.hull,
            revision: self.data.revision,
            credits_tenths: self.data.credits,
            fuel_tenths: self.data.fuel_tenths,
            food_tonnes: self.data.food_tonnes,
            reserved_food_tonnes: self.data.reserved_food_tonnes,
            used_tonnes: u16::try_from(occupied_tonnes(&freight.data, &self.data))
                .map_err(|_| services::Error::InvalidAccount)?,
        })
    }

    /// The only ordinary service mutation: checked account and stock publish together.
    /// No caller-supplied price, cash balance, fuel amount or cargo grant enters this boundary.
    pub(crate) fn execute_station_service(
        &mut self,
        freight: &State,
        markets: &mut services::StationMarkets,
        facts: services::ServiceFacts,
        command: services::Command,
    ) -> Result<services::Event, services::Error> {
        let before = self.service_projection(freight)?;
        let plan = markets.prepare(before, Some(facts), command)?;
        let change =
            plan.checked_change(markets, self.service_projection(freight)?, Some(facts))?;
        let after = change.account_after;
        let mut candidate = Self { data: self.data };
        match change.event {
            services::Event::BoughtFood { paid_tenths, .. }
            | services::Event::Refuelled { paid_tenths, .. } => {
                candidate.debit(paid_tenths).map_err(service_error)?;
            }
            services::Event::SoldFood { .. } => {
                candidate.data.credits = after.credits_tenths;
                candidate.data.revision = after.revision;
            }
        }
        candidate.data.food_tonnes = after.food_tonnes;
        candidate.data.fuel_tenths = after.fuel_tenths;
        // This also checks actual owned fit, all remaining identified freight and reservations.
        // Copying the account preserves equipment/other cargo instead of reconstructing it.
        if candidate.service_projection(freight)? != after {
            return Err(services::Error::InvalidAccount);
        }
        self.data = candidate.data;
        *markets = change.market;
        Ok(change.event)
    }

    /// World navigation, after its real passage checks, consumes normal owned fuel.
    pub(crate) fn debit_fuel(&mut self, amount: u16) -> Result<(), Error> {
        let fuel = self
            .data
            .fuel_tenths
            .checked_sub(amount)
            .ok_or(Error::InsufficientFuel)?;
        let revision = self
            .data
            .revision
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        self.data.fuel_tenths = fuel;
        self.data.revision = revision;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    WrongOwner,
    MissingEquipment,
    UnpaidEquipment,
    InvalidFit,
    NotDocked,
    WrongStation,
    WrongActor,
    ContentUnavailable,
    Paused,
    InsufficientFuel,
    InsufficientFunds,
    HoldFull,
    ActiveJob,
    SourceExhausted,
    UnknownJob,
    NotActive,
    StaleReview,
    WrongChoice,
    ForeignUnit,
    AlreadyTransferred,
    OutOfOrder,
    CreditOverflow,
    SequenceExhausted,
    StalePublication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Purpose {
    Accept,
    Receive(JobId),
    Abandon(JobId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Binding {
    account: AccountData,
    world: WorldFacts,
    revision: u64,
}

/// Descriptive review; private fields and live pending-review matching prevent fabricated consent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Review {
    owner: u64,
    hull: u64,
    nonce: u64,
    purpose: Purpose,
    next_unit: Option<GoodsId>,
    remaining: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingReview {
    review: Review,
    binding: Binding,
}

impl Review {
    pub fn next_unit(self) -> Option<GoodsId> {
        self.next_unit
    }
    pub fn remaining_units(self) -> u8 {
        self.remaining
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Accept,
    ReceiveOne(GoodsId),
    ReceiveAll,
    Abandon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    ReviewOffer { actor: ActorId },
    ReviewReceiving { actor: ActorId, job: JobId },
    ReviewAbandonment { actor: ActorId, job: JobId },
    Confirm { review: Review, choice: Choice },
    Back,
}

/// Created only by committing the last original-unit transfer; never accepted as command authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionReceipt {
    job: JobId,
    sequence: u64,
    receiver: ActorId,
    dock: DockFacts,
}

impl CompletionReceipt {
    pub fn job(self) -> JobId {
        self.job
    }
    pub fn sequence(self) -> u64 {
        self.sequence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClueAward {
    completion: CompletionReceipt,
}

impl ClueAward {
    pub fn source_id(self) -> &'static str {
        SOURCE_ID
    }
    pub fn clue_id(self) -> &'static str {
        CLUE_ID
    }
    pub fn completion(self) -> CompletionReceipt {
        self.completion
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Reviewed(Review),
    Back,
    Accepted {
        job: JobId,
        units: [GoodsId; 8],
    },
    Received {
        job: JobId,
        units: [Option<GoodsId>; 8],
        remaining: u8,
        completion: Option<CompletionReceipt>,
        clue_awarded: bool,
    },
    Abandoned {
        job: JobId,
        removed: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Transfer {
    sequence: u64,
    mask: u8,
    receiver: ActorId,
    dock: DockFacts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Job {
    id: JobId,
    issued: [GoodsId; 8],
    held: u8,
    delivered: u8,
    status: JobStatus,
    issue_sequence: u64,
    issuer: DockFacts,
    transfers: [Option<Transfer>; 8],
    transfer_count: u8,
    completion: Option<CompletionReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Data {
    owner: u64,
    revision: u64,
    sequence: u64,
    allocated: u8,
    jobs: [Option<Job>; 2],
    clue: Option<ClueAward>,
}

/// Source/custody/wallet operations are synchronous under the single owning World.
/// No public initializer, admission callback or receipt-acceptance factory is exposed.
#[derive(Debug)]
pub struct State {
    data: Data,
    ui_sequence: u64,
    pending: Option<PendingReview>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    pub available_source_units: u8,
    pub held_tonnes: u8,
    pub completed_jobs: u8,
    pub clue: Option<ClueAward>,
}

/// A bounded prospective transaction. Dropping it models failed publication without mutation.
/// It is not Clone; commit rechecks the full original account, state, UI and World context.
#[derive(Debug)]
pub(crate) struct Prepared {
    old: Data,
    old_ui: u64,
    old_pending: Option<PendingReview>,
    account: AccountData,
    world: WorldFacts,
    next: Data,
    next_ui: u64,
    next_pending: Option<PendingReview>,
    next_account: AccountData,
    event: Event,
}

impl State {
    pub(crate) fn new_commander(owner: u64) -> Self {
        Self {
            data: Data {
                owner,
                revision: 0,
                sequence: 0,
                allocated: 0,
                jobs: [None; 2],
                clue: None,
            },
            ui_sequence: 0,
            pending: None,
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            available_source_units: 16 - 8 * self.data.allocated,
            held_tonnes: held(&self.data),
            completed_jobs: self
                .data
                .jobs
                .iter()
                .flatten()
                .filter(|job| job.status == JobStatus::Delivered)
                .count() as u8,
            clue: self.data.clue,
        }
    }

    pub(crate) fn active_job(&self) -> Option<JobId> {
        self.data
            .jobs
            .iter()
            .flatten()
            .find(|job| job.status == JobStatus::Active && job.held != 0)
            .map(|job| job.id)
    }

    pub fn issued_units(&self, id: JobId) -> Option<[GoodsId; 8]> {
        job(&self.data, id).ok().map(|job| job.issued)
    }

    pub fn job_status(&self, id: JobId) -> Option<JobStatus> {
        job(&self.data, id).ok().map(|job| job.status)
    }

    pub(crate) fn prepare(
        &self,
        account: &CommanderAccount,
        world: &WorldFacts,
        command: Command,
    ) -> Result<Prepared, Error> {
        let mut plan = Prepared {
            old: self.data,
            old_ui: self.ui_sequence,
            old_pending: self.pending,
            account: account.data,
            world: *world,
            next: self.data,
            next_ui: self.ui_sequence,
            next_pending: self.pending,
            next_account: account.data,
            event: Event::Back,
        };
        plan.next_ui = self
            .ui_sequence
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        if command == Command::Back {
            plan.next_pending = None;
            return Ok(plan);
        }
        validate_account(&self.data, &account.data)?;
        let dock = current_dock(&account.data, world)?;
        match command {
            Command::ReviewOffer { actor } => {
                require_contact(world, dock, actor, ContactDefinition::Arlen, AVENOR)?;
                require_acceptance(&self.data, &account.data)?;
                let review = Review {
                    owner: self.data.owner,
                    hull: account.data.hull,
                    nonce: plan.next_ui,
                    purpose: Purpose::Accept,
                    next_unit: None,
                    remaining: 8,
                };
                plan.next_pending = Some(PendingReview {
                    review,
                    binding: Binding {
                        account: account.data,
                        world: *world,
                        revision: self.data.revision,
                    },
                });
                plan.event = Event::Reviewed(review);
            }
            Command::ReviewReceiving { actor, job: id }
            | Command::ReviewAbandonment { actor, job: id } => {
                let current = job(&self.data, id)?;
                if current.status != JobStatus::Active {
                    return Err(Error::NotActive);
                }
                let receiving = matches!(command, Command::ReviewReceiving { .. });
                require_contact(
                    world,
                    dock,
                    actor,
                    if receiving {
                        ContactDefinition::RenasUtilityCrew
                    } else {
                        ContactDefinition::Arlen
                    },
                    if receiving { RENAS } else { AVENOR },
                )?;
                let review = Review {
                    owner: self.data.owner,
                    hull: account.data.hull,
                    nonce: plan.next_ui,
                    purpose: if receiving {
                        Purpose::Receive(id)
                    } else {
                        Purpose::Abandon(id)
                    },
                    next_unit: next_unit(current),
                    remaining: current.held.count_ones() as u8,
                };
                plan.next_pending = Some(PendingReview {
                    review,
                    binding: Binding {
                        account: account.data,
                        world: *world,
                        revision: self.data.revision,
                    },
                });
                plan.event = Event::Reviewed(review);
            }
            Command::Confirm { review, choice } => {
                if self.pending
                    != Some(PendingReview {
                        review,
                        binding: Binding {
                            account: account.data,
                            world: *world,
                            revision: self.data.revision,
                        },
                    })
                {
                    return Err(Error::StaleReview);
                }
                plan.next_pending = None;
                plan.next.revision = self
                    .data
                    .revision
                    .checked_add(1)
                    .ok_or(Error::SequenceExhausted)?;
                plan.next.sequence = self
                    .data
                    .sequence
                    .checked_add(1)
                    .ok_or(Error::SequenceExhausted)?;
                plan.next_account.revision = account
                    .data
                    .revision
                    .checked_add(1)
                    .ok_or(Error::SequenceExhausted)?;
                match (review.purpose, choice) {
                    (Purpose::Accept, Choice::Accept) => {
                        require_contact(
                            world,
                            dock,
                            world.contact.ok_or(Error::WrongActor)?.actor,
                            ContactDefinition::Arlen,
                            AVENOR,
                        )?;
                        require_acceptance(&self.data, &account.data)?;
                        let ordinal = self.data.allocated;
                        let id = JobId {
                            owner: self.data.owner,
                            ordinal,
                        };
                        let issued = core::array::from_fn(|index| GoodsId {
                            owner: self.data.owner,
                            serial: ordinal * 8 + index as u8,
                        });
                        plan.next.jobs[ordinal as usize] = Some(Job {
                            id,
                            issued,
                            held: u8::MAX,
                            delivered: 0,
                            status: JobStatus::Active,
                            issue_sequence: plan.next.sequence,
                            issuer: dock,
                            transfers: [None; 8],
                            transfer_count: 0,
                            completion: None,
                        });
                        plan.next.allocated += 1;
                        plan.event = Event::Accepted {
                            job: id,
                            units: issued,
                        };
                    }
                    (Purpose::Receive(id), Choice::ReceiveOne(_))
                    | (Purpose::Receive(id), Choice::ReceiveAll) => {
                        self.prepare_receiving(id, choice, dock, &mut plan)?;
                    }
                    (Purpose::Abandon(id), Choice::Abandon) => {
                        require_contact(
                            world,
                            dock,
                            world.contact.ok_or(Error::WrongActor)?.actor,
                            ContactDefinition::Arlen,
                            AVENOR,
                        )?;
                        let current = job(&self.data, id)?;
                        if current.status != JobStatus::Active {
                            return Err(Error::NotActive);
                        }
                        let mut updated = *current;
                        let removed = updated.held.count_ones() as u8;
                        updated.held = 0;
                        updated.status = JobStatus::Abandoned;
                        plan.next.jobs[id.ordinal as usize] = Some(updated);
                        plan.event = Event::Abandoned { job: id, removed };
                    }
                    _ => return Err(Error::WrongChoice),
                }
            }
            Command::Back => return Err(Error::WrongChoice),
        }
        Ok(plan)
    }

    fn prepare_receiving(
        &self,
        id: JobId,
        choice: Choice,
        dock: DockFacts,
        plan: &mut Prepared,
    ) -> Result<(), Error> {
        let world = &plan.world;
        require_contact(
            world,
            dock,
            world.contact.ok_or(Error::WrongActor)?.actor,
            ContactDefinition::RenasUtilityCrew,
            RENAS,
        )?;
        let current = job(&self.data, id)?;
        if current.status != JobStatus::Active {
            return Err(Error::NotActive);
        }
        let mask = match choice {
            Choice::ReceiveOne(unit) => one_unit_mask(current, unit)?,
            Choice::ReceiveAll => current.held,
            _ => return Err(Error::WrongChoice),
        };
        let remaining = current.held & !mask;
        let receiver = world.contact.ok_or(Error::WrongActor)?.actor;
        let completion = if remaining == 0 {
            plan.next_account.credits = plan
                .account
                .credits
                .checked_add(REWARD_TENTHS)
                .ok_or(Error::CreditOverflow)?;
            Some(CompletionReceipt {
                job: id,
                sequence: plan.next.sequence,
                receiver,
                dock,
            })
        } else {
            None
        };
        let mut updated = *current;
        updated.held = remaining;
        updated.delivered |= mask;
        updated.transfers[updated.transfer_count as usize] = Some(Transfer {
            sequence: plan.next.sequence,
            mask,
            receiver,
            dock,
        });
        updated.transfer_count += 1;
        updated.completion = completion;
        if completion.is_some() {
            updated.status = JobStatus::Delivered;
        }
        plan.next.jobs[id.ordinal as usize] = Some(updated);
        let clue_awarded = completion.is_some() && self.data.clue.is_none();
        if clue_awarded {
            plan.next.clue = completion.map(|completion| ClueAward { completion });
        }
        let units = core::array::from_fn(|index| {
            (mask & (1 << index) != 0).then_some(current.issued[index])
        });
        plan.event = Event::Received {
            job: id,
            units,
            remaining: remaining.count_ones() as u8,
            completion,
            clue_awarded,
        };
        Ok(())
    }

    /// Call only after the owning publication succeeds; a changed World/account refuses atomically.
    pub(crate) fn commit(
        &mut self,
        account: &mut CommanderAccount,
        world: &WorldFacts,
        plan: Prepared,
    ) -> Result<Event, Error> {
        if self.data != plan.old
            || self.ui_sequence != plan.old_ui
            || self.pending != plan.old_pending
            || account.data != plan.account
            || *world != plan.world
        {
            return Err(Error::StalePublication);
        }
        self.data = plan.next;
        self.ui_sequence = plan.next_ui;
        self.pending = plan.next_pending;
        account.data = plan.next_account;
        Ok(plan.event)
    }
}

fn held(data: &Data) -> u8 {
    data.jobs
        .iter()
        .flatten()
        .map(|job| job.held.count_ones() as u8)
        .sum()
}

fn job(data: &Data, id: JobId) -> Result<&Job, Error> {
    if id.owner != data.owner {
        return Err(Error::WrongOwner);
    }
    data.jobs
        .get(id.ordinal as usize)
        .and_then(Option::as_ref)
        .filter(|job| job.id == id)
        .ok_or(Error::UnknownJob)
}

fn next_unit(job: &Job) -> Option<GoodsId> {
    (job.held != 0).then(|| job.issued[job.held.trailing_zeros() as usize])
}

fn one_unit_mask(job: &Job, unit: GoodsId) -> Result<u8, Error> {
    if unit.owner != job.id.owner {
        return Err(Error::ForeignUnit);
    }
    let index = job
        .issued
        .iter()
        .position(|candidate| *candidate == unit)
        .ok_or(Error::ForeignUnit)?;
    let mask = 1_u8 << index;
    if job.held & mask == 0 {
        return Err(Error::AlreadyTransferred);
    }
    if next_unit(job) != Some(unit) {
        return Err(Error::OutOfOrder);
    }
    Ok(mask)
}

fn validate_account(data: &Data, account: &AccountData) -> Result<(), Error> {
    if account.owner != data.owner {
        return Err(Error::WrongOwner);
    }
    let fit = account.fit.ok_or(Error::MissingEquipment)?;
    if fit.owner != account.owner || fit.hull != account.hull {
        return Err(Error::WrongOwner);
    }
    validate_birth_ids(FreshCommanderIds {
        owner: account.owner,
        hull: account.hull,
        inventory: fit.inventory,
        items: fit.items.map(|item| item.id),
    })?;
    for (index, item) in fit.items.iter().enumerate() {
        if item.owner != account.owner {
            return Err(Error::WrongOwner);
        }
        if item.origin == EquipmentOrigin::Unpaid {
            return Err(Error::UnpaidEquipment);
        }
        if item.component != STARTER_COMPONENTS[index]
            || item.fitted_hull.is_some_and(|hull| hull != account.hull)
        {
            return Err(Error::InvalidFit);
        }
    }
    if fit.items[3].fitted_hull != Some(account.hull) {
        return Err(Error::MissingEquipment);
    }
    if account.fuel_tenths > 130 {
        return Err(Error::InvalidFit);
    }
    if account.reserved_food_tonnes > account.food_tonnes
        || occupied_tonnes(data, account) > u32::from(HOLD_TONNES)
    {
        return Err(Error::InvalidFit);
    }
    Ok(())
}

fn occupied_tonnes(data: &Data, account: &AccountData) -> u32 {
    u32::from(account.other_tonnes) + u32::from(account.food_tonnes) + u32::from(held(data))
}

fn service_error(error: Error) -> services::Error {
    match error {
        Error::InsufficientFunds => services::Error::InsufficientFunds,
        Error::SequenceExhausted => services::Error::SequenceExhausted,
        Error::WrongOwner => services::Error::WrongOwner,
        _ => services::Error::InvalidAccount,
    }
}

fn current_dock(account: &AccountData, world: &WorldFacts) -> Result<DockFacts, Error> {
    if world.paused {
        return Err(Error::Paused);
    }
    if world.content_digest != Some(CONTENT_DIGEST) {
        return Err(Error::ContentUnavailable);
    }
    let dock = world.dock.ok_or(Error::NotDocked)?;
    if dock.owner != account.owner || dock.hull != account.hull {
        return Err(Error::WrongOwner);
    }
    if dock.kind == StationKind::Other || dock.arrival_sequence == 0 {
        return Err(Error::WrongStation);
    }
    Ok(dock)
}

fn require_contact(
    world: &WorldFacts,
    dock: DockFacts,
    requested: ActorId,
    definition: ContactDefinition,
    system: System,
) -> Result<(), Error> {
    if dock.system != system {
        return Err(Error::WrongStation);
    }
    let actual = world.contact.ok_or(Error::WrongActor)?;
    if actual.actor != requested
        || actual.station_actor != dock.station_actor
        || actual.definition != definition
    {
        return Err(Error::WrongActor);
    }
    Ok(())
}

fn require_acceptance(data: &Data, account: &AccountData) -> Result<(), Error> {
    if data
        .jobs
        .iter()
        .flatten()
        .any(|job| job.status == JobStatus::Active)
    {
        return Err(Error::ActiveJob);
    }
    if data.allocated >= 2 {
        return Err(Error::SourceExhausted);
    }
    if occupied_tonnes(data, account) + 8 > u32::from(HOLD_TONNES) {
        return Err(Error::HoldFull);
    }
    if account.fuel_tenths < DEPARTURE_FUEL_TENTHS {
        return Err(Error::InsufficientFuel);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // Deliberately synthetic internal facts: these exercise domain commands, never physical arrival.
    struct Fixture {
        state: State,
        account: CommanderAccount,
        world: WorldFacts,
    }

    impl Fixture {
        fn new(owner: u64) -> Self {
            let station = ActorId {
                entity: 10,
                generation: 1,
            };
            Self {
                state: State::new_commander(owner),
                account: CommanderAccount::born_sh025(FreshCommanderIds {
                    owner,
                    hull: owner + 100,
                    inventory: owner + 200,
                    items: core::array::from_fn(|index| owner + 300 + index as u64),
                })
                .unwrap(),
                world: WorldFacts {
                    epoch: 1,
                    paused: false,
                    content_digest: Some(CONTENT_DIGEST),
                    dock: Some(DockFacts {
                        owner,
                        hull: owner + 100,
                        system: AVENOR,
                        kind: StationKind::MainOrbital,
                        station_actor: station,
                        arrival_sequence: 1,
                    }),
                    contact: Some(ContactFacts {
                        actor: ActorId {
                            entity: 11,
                            generation: 1,
                        },
                        station_actor: station,
                        definition: ContactDefinition::Arlen,
                        roster_revision: 1,
                    }),
                },
            }
        }

        fn actor(&self) -> ActorId {
            self.world.contact.unwrap().actor
        }

        fn apply(&mut self, command: Command) -> Result<Event, Error> {
            let plan = self.state.prepare(&self.account, &self.world, command)?;
            self.state.commit(&mut self.account, &self.world, plan)
        }

        fn review(&mut self, command: Command) -> Review {
            match self.apply(command).unwrap() {
                Event::Reviewed(review) => review,
                other => panic!("expected review, got {other:?}"),
            }
        }

        fn accept(&mut self) -> (JobId, [GoodsId; 8]) {
            let review = self.review(Command::ReviewOffer {
                actor: self.actor(),
            });
            match self
                .apply(Command::Confirm {
                    review,
                    choice: Choice::Accept,
                })
                .unwrap()
            {
                Event::Accepted { job, units } => (job, units),
                other => panic!("expected allocation, got {other:?}"),
            }
        }

        fn receiving(&mut self) {
            self.world.epoch += 1;
            let dock = self.world.dock.as_mut().unwrap();
            dock.system = RENAS;
            dock.arrival_sequence += 1;
            self.world.contact.as_mut().unwrap().definition = ContactDefinition::RenasUtilityCrew;
        }

        fn source(&mut self) {
            self.world.epoch += 1;
            let dock = self.world.dock.as_mut().unwrap();
            dock.system = AVENOR;
            dock.arrival_sequence += 1;
            self.world.contact.as_mut().unwrap().definition = ContactDefinition::Arlen;
        }

        fn receive(&mut self, job: JobId, choice: Choice) -> Event {
            let review = self.review(Command::ReviewReceiving {
                actor: self.actor(),
                job,
            });
            self.apply(Command::Confirm { review, choice }).unwrap()
        }

        fn refuses(&mut self, command: Command, expected: Error) {
            let before = (
                self.state.data,
                self.state.ui_sequence,
                self.state.pending,
                self.account.data,
            );
            assert_eq!(self.apply(command), Err(expected));
            assert_eq!(
                before,
                (
                    self.state.data,
                    self.state.ui_sequence,
                    self.state.pending,
                    self.account.data
                )
            );
        }
    }

    #[test]
    fn seventh_unit_has_no_payment_and_eighth_keeps_original_custody_identity() {
        let mut f = Fixture::new(1);
        let (job, units) = f.accept();
        assert_eq!(f.account.credits_tenths(), 7200); // Consent has no deposit.
        assert_eq!(f.state.snapshot().available_source_units, 8);
        f.account.debit_fuel(DEPARTURE_FUEL_TENTHS).unwrap();
        assert_eq!(f.account.fuel_tenths(), 96);
        f.receiving();
        for (index, unit) in units.iter().enumerate() {
            let event = f.receive(job, Choice::ReceiveOne(*unit));
            let Event::Received {
                units: received,
                remaining,
                completion,
                clue_awarded,
                ..
            } = event
            else {
                panic!("expected transfer");
            };
            assert_eq!(
                received.iter().flatten().copied().collect::<Vec<_>>(),
                vec![*unit]
            );
            assert_eq!(remaining, 7 - index as u8);
            assert_eq!(completion.is_some(), index == 7);
            assert_eq!(clue_awarded, index == 7);
            assert_eq!(
                f.account.credits_tenths(),
                if index == 7 { 14500 } else { 7200 }
            );
            assert_eq!(f.state.snapshot().clue.is_some(), index == 7);
        }
        let clue = f.state.snapshot().clue.unwrap();
        assert_eq!(clue.source_id(), SOURCE_ID);
        assert_eq!(clue.clue_id(), CLUE_ID);
        assert_eq!(clue.completion().job(), job);
        assert_eq!(f.state.job_status(job), Some(JobStatus::Delivered));
        assert_eq!(f.state.snapshot().held_tonnes, 0);
        let actor = f.actor();
        f.refuses(Command::ReviewReceiving { actor, job }, Error::NotActive);
    }

    #[test]
    fn second_allocation_is_disjoint_pays_once_and_preserves_first_clue() {
        let mut f = Fixture::new(1);
        let (first, a) = f.accept();
        f.receiving();
        f.receive(first, Choice::ReceiveAll);
        let clue = f.state.snapshot().clue;
        f.source();
        let (second, b) = f.accept();
        assert!(a.iter().all(|unit| !b.contains(unit)));
        assert_eq!(b[0].serial(), 8);
        f.receiving();
        assert!(matches!(
            f.receive(second, Choice::ReceiveAll),
            Event::Received {
                remaining: 0,
                completion: Some(_),
                clue_awarded: false,
                ..
            }
        ));
        assert_eq!(f.state.snapshot().clue, clue);
        assert_eq!(f.account.credits_tenths(), 21800);
        assert_eq!(f.state.snapshot().completed_jobs, 2);
        f.source();
        let actor = f.actor();
        f.refuses(Command::ReviewOffer { actor }, Error::SourceExhausted);
    }

    #[test]
    fn foreign_duplicate_and_out_of_order_units_cannot_mutate_custody() {
        let mut f = Fixture::new(1);
        let (job, units) = f.accept();
        let (_, foreign) = Fixture::new(7).accept();
        f.receiving();
        let review = f.review(Command::ReviewReceiving {
            actor: f.actor(),
            job,
        });
        f.refuses(
            Command::Confirm {
                review,
                choice: Choice::ReceiveOne(foreign[0]),
            },
            Error::ForeignUnit,
        );
        f.refuses(
            Command::Confirm {
                review,
                choice: Choice::ReceiveOne(units[1]),
            },
            Error::OutOfOrder,
        );
        f.apply(Command::Confirm {
            review,
            choice: Choice::ReceiveOne(units[0]),
        })
        .unwrap();
        let review = f.review(Command::ReviewReceiving {
            actor: f.actor(),
            job,
        });
        f.refuses(
            Command::Confirm {
                review,
                choice: Choice::ReceiveOne(units[0]),
            },
            Error::AlreadyTransferred,
        );
        assert_eq!(f.state.snapshot().held_tonnes, 7);
        assert_eq!(f.account.credits_tenths(), 7200);
    }

    #[test]
    fn cancel_replay_and_changed_world_or_account_invalidate_consent() {
        let mut f = Fixture::new(1);
        let review = f.review(Command::ReviewOffer { actor: f.actor() });
        f.apply(Command::Back).unwrap();
        f.refuses(
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            Error::StaleReview,
        );
        let review = f.review(Command::ReviewOffer { actor: f.actor() });
        f.account.debit(1).unwrap();
        f.refuses(
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            Error::StaleReview,
        );
        let review = f.review(Command::ReviewOffer { actor: f.actor() });
        f.world.contact.as_mut().unwrap().roster_revision += 1;
        f.refuses(
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            Error::StaleReview,
        );
        let review = f.review(Command::ReviewOffer { actor: f.actor() });
        f.apply(Command::Confirm {
            review,
            choice: Choice::Accept,
        })
        .unwrap();
        f.refuses(
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            Error::StaleReview,
        );
    }

    #[test]
    fn wrong_location_actor_equipment_and_capacity_refuse_without_allocation() {
        let mut f = Fixture::new(1);
        let actor = f.actor();
        let command = Command::ReviewOffer { actor };
        f.refuses(
            Command::ReviewOffer {
                actor: ActorId {
                    entity: 99,
                    generation: 1,
                },
            },
            Error::WrongActor,
        );
        f.world.dock.as_mut().unwrap().kind = StationKind::Other;
        f.refuses(command, Error::WrongStation);
        f.world.dock.as_mut().unwrap().kind = StationKind::MainOrbital;
        f.world.dock.as_mut().unwrap().owner = 99;
        f.refuses(command, Error::WrongOwner);
        f.world.dock.as_mut().unwrap().owner = 1;
        f.receiving();
        f.refuses(command, Error::WrongStation);
        f.source();
        let fit = f.account.data.fit;
        f.account.data.fit = None;
        f.refuses(command, Error::MissingEquipment);
        f.account.data.fit = fit;
        f.account.data.fit.as_mut().unwrap().items[3].origin = EquipmentOrigin::Unpaid;
        f.refuses(command, Error::UnpaidEquipment);
        f.account.data.fit = fit;
        f.account.data.other_tonnes = 85;
        f.refuses(command, Error::HoldFull);
        f.account.data.other_tonnes = 0;
        f.account.data.fuel_tenths = 33;
        f.refuses(command, Error::InsufficientFuel);
        assert_eq!(f.state.snapshot().available_source_units, 16);
    }

    #[test]
    fn failed_or_stale_publication_changes_no_domain_state() {
        let mut f = Fixture::new(1);
        let review = f.review(Command::ReviewOffer { actor: f.actor() });
        let before = (f.state.data, f.account.data);
        let command = Command::Confirm {
            review,
            choice: Choice::Accept,
        };
        {
            // Domain rollback boundary only; no disk writer is exercised.
            let _failed_write = f.state.prepare(&f.account, &f.world, command).unwrap();
        }
        assert_eq!((f.state.data, f.account.data), before);
        let plan = f.state.prepare(&f.account, &f.world, command).unwrap();
        f.world.epoch += 1;
        assert_eq!(
            f.state.commit(&mut f.account, &f.world, plan),
            Err(Error::StalePublication)
        );
        assert_eq!((f.state.data, f.account.data), before);
    }

    #[test]
    fn final_payment_overflow_preserves_last_unit_pending_consent_and_clue() {
        let mut f = Fixture::new(1);
        let (job, units) = f.accept();
        f.receiving();
        for unit in &units[..7] {
            f.receive(job, Choice::ReceiveOne(*unit));
        }
        f.account.data.credits = u32::MAX - REWARD_TENTHS + 1;
        let review = f.review(Command::ReviewReceiving {
            actor: f.actor(),
            job,
        });
        f.refuses(
            Command::Confirm {
                review,
                choice: Choice::ReceiveOne(units[7]),
            },
            Error::CreditOverflow,
        );
        assert_eq!(f.state.snapshot().held_tonnes, 1);
        assert!(f.state.snapshot().clue.is_none());
        f.account.debit(1).unwrap();
        f.receive(job, Choice::ReceiveOne(units[7]));
        assert_eq!(f.account.credits_tenths(), u32::MAX);
    }

    #[test]
    fn abandonment_removes_only_held_units_without_restock_reward_or_clue() {
        let mut f = Fixture::new(1);
        let (job, _) = f.accept();
        let actor = f.actor();
        f.refuses(Command::ReviewOffer { actor }, Error::ActiveJob);
        let review = f.review(Command::ReviewAbandonment { actor, job });
        assert_eq!(
            f.apply(Command::Confirm {
                review,
                choice: Choice::Abandon
            }),
            Ok(Event::Abandoned { job, removed: 8 })
        );
        assert_eq!(f.state.snapshot().available_source_units, 8);
        assert_eq!(f.state.snapshot().held_tonnes, 0);
        assert_eq!(f.account.credits_tenths(), 7200);
        assert!(f.state.snapshot().clue.is_none());
        assert_eq!(f.state.job_status(job), Some(JobStatus::Abandoned));
        let (_, units) = f.accept();
        assert_eq!(units[0].serial(), 8);
    }

    #[test]
    fn suspended_content_keeps_custody_and_receiving_needs_no_future_trip_fuel() {
        let mut f = Fixture::new(1);
        let (job, _) = f.accept();
        f.receiving();
        let actor = f.actor();
        f.world.content_digest = None;
        f.refuses(
            Command::ReviewReceiving { actor, job },
            Error::ContentUnavailable,
        );
        assert_eq!(f.state.snapshot().held_tonnes, 8);
        f.world.content_digest = Some(CONTENT_DIGEST);
        f.account.debit_fuel(130).unwrap();
        f.receive(job, Choice::ReceiveAll);
        assert_eq!(f.account.credits_tenths(), 14500);
    }

    #[test]
    fn fresh_profile_creates_exact_owned_standard_items_and_rejects_bad_ids() {
        let ids = FreshCommanderIds {
            owner: 1,
            hull: 2,
            inventory: 3,
            items: [4, 5, 6, 7, 8, 9, 10, 11],
        };
        let account = CommanderAccount::born_sh025(ids).unwrap();
        let items = account.starter_items().unwrap();
        for (index, item) in items.iter().enumerate() {
            assert_eq!(item.id(), ids.items[index]);
            assert_eq!(item.owner(), ids.owner);
            assert_eq!(item.component(), STARTER_COMPONENTS[index]);
            assert_eq!(item.fitted_hull(), Some(ids.hull));
            assert_eq!(item.origin, EquipmentOrigin::StandardBirth);
        }
        assert_eq!(account.credits_tenths(), 7200);
        assert_eq!(account.fuel_tenths(), 130);
        let mut bad = ids;
        bad.items[7] = bad.items[0];
        assert_eq!(CommanderAccount::born_sh025(bad), Err(Error::InvalidFit));
        bad = ids;
        bad.items[0] = 0;
        assert_eq!(CommanderAccount::born_sh025(bad), Err(Error::InvalidFit));
        bad = ids;
        bad.items[0] = bad.hull;
        assert_eq!(CommanderAccount::born_sh025(bad), Err(Error::InvalidFit));
    }

    #[test]
    fn forged_inventory_and_unpaid_spending_refuse_without_allocating_or_debiting() {
        let mut f = Fixture::new(1);
        let original = f.account.data;
        assert_eq!(f.account.debit(7201), Err(Error::InsufficientFunds));
        assert_eq!(f.account.data, original);
        assert_eq!(f.account.debit_fuel(131), Err(Error::InsufficientFuel));
        assert_eq!(f.account.data, original);
        let actor = f.actor();
        let command = Command::ReviewOffer { actor };
        f.account.data.fit.as_mut().unwrap().items[3].fitted_hull = None;
        f.refuses(command, Error::MissingEquipment);
        f.account.data = original;
        f.account.data.fit.as_mut().unwrap().items[0].owner = 99;
        f.refuses(command, Error::WrongOwner);
        f.account.data = original;
        f.account.data.fit.as_mut().unwrap().items[0].fitted_hull = Some(99);
        f.refuses(command, Error::InvalidFit);
        f.account.data = original;
        f.account.data.fit.as_mut().unwrap().items[1].id = original.fit.unwrap().items[0].id;
        f.refuses(command, Error::InvalidFit);
        f.account.data = original;
        f.account.data.fit.as_mut().unwrap().items[0].component = StarterComponent::BasicDrive;
        f.refuses(command, Error::InvalidFit);
        assert_eq!(f.state.snapshot().available_source_units, 16);
    }

    fn service_facts(f: &Fixture) -> services::ServiceFacts {
        services::ServiceFacts {
            owner: f.account.data.owner,
            hull: f.account.data.hull,
            site: services::MarketSite::Avenor,
            station_entity: 1,
            visit: 1,
            dock_sequence: 1,
            epoch: f.world.epoch,
            paused: false,
        }
    }

    #[test]
    fn service_hold_reservations_and_fit_preserve_actual_account_and_eight_unit_custody() {
        // Synthetic boundary fixture: no public cargo/credit/equipment grant is introduced.
        let mut f = Fixture::new(1);
        let (job, units) = f.accept();
        f.account.data.food_tonnes = 84;
        let mut market = services::StationMarkets::new_commander(1).unwrap();
        let facts = service_facts(&f);
        let before = f.account.data;
        let freight_before = f.state.data;
        let quote = market.quote(facts.site);
        assert_eq!(
            f.account.service_projection(&f.state).unwrap().used_tonnes,
            92
        );
        assert_eq!(
            f.account.execute_station_service(
                &f.state,
                &mut market,
                facts,
                services::Command::BuyFood
            ),
            Err(services::Error::HoldFull)
        );
        assert_eq!(
            (f.account.data, f.state.data, market.quote(facts.site)),
            (before, freight_before, quote)
        );
        f.account
            .execute_station_service(&f.state, &mut market, facts, services::Command::SellFood)
            .unwrap();
        f.account
            .execute_station_service(&f.state, &mut market, facts, services::Command::BuyFood)
            .unwrap();
        assert_eq!(f.account.data.food_tonnes, 84);
        assert_eq!(f.account.data.fit, before.fit);
        assert_eq!(f.state.data, freight_before);
        assert_eq!(f.state.issued_units(job), Some(units));
        f.account.data.reserved_food_tonnes = 84;
        let before = f.account.data;
        let quote = market.quote(facts.site);
        assert_eq!(
            f.account.execute_station_service(
                &f.state,
                &mut market,
                facts,
                services::Command::SellFood
            ),
            Err(services::Error::NoSellableCargo)
        );
        assert_eq!(
            (f.account.data, f.state.data, market.quote(facts.site)),
            (before, freight_before, quote)
        );
        f.account.data.fit.as_mut().unwrap().items[3].fitted_hull = None;
        let before = f.account.data;
        assert_eq!(
            f.account.execute_station_service(
                &f.state,
                &mut market,
                facts,
                services::Command::RefillFuel
            ),
            Err(services::Error::InvalidAccount)
        );
        assert_eq!(
            (f.account.data, f.state.data, market.quote(facts.site)),
            (before, freight_before, quote)
        );
    }

    #[test]
    fn real_food_property_counts_for_freight_acceptance_and_service_failure_is_jointly_atomic() {
        let mut f = Fixture::new(1);
        f.account.data.food_tonnes = 85;
        f.refuses(Command::ReviewOffer { actor: f.actor() }, Error::HoldFull);
        f.account.data.food_tonnes = 0;
        f.account.data.fuel_tenths = 96;
        f.account.data.credits = 67;
        let mut market = services::StationMarkets::new_commander(1).unwrap();
        let facts = service_facts(&f);
        let before = (f.account.data, f.state.data, market.quote(facts.site));
        assert_eq!(
            f.account.execute_station_service(
                &f.state,
                &mut market,
                facts,
                services::Command::RefillFuel
            ),
            Err(services::Error::InsufficientFunds)
        );
        assert_eq!(
            (f.account.data, f.state.data, market.quote(facts.site)),
            before
        );
        f.account.data.credits = 7200;
        f.account.data.revision = u64::MAX;
        let before = (f.account.data, f.state.data, market.quote(facts.site));
        assert_eq!(
            f.account.execute_station_service(
                &f.state,
                &mut market,
                facts,
                services::Command::BuyFood
            ),
            Err(services::Error::SequenceExhausted)
        );
        assert_eq!(
            (f.account.data, f.state.data, market.quote(facts.site)),
            before
        );
    }
}

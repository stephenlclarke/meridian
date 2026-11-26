//! Original T04. World-owned ordinary transactions create identified tonne and journey evidence.
use super::World;
use crate::freight::g02::CommanderAccount;
use crate::navigation::{DockContext, SystemId};
use crate::save::{Reader, SaveError, Writer};
use crate::station_services::{self as services, CommodityId, StationMarkets};

mod actions;
mod market;
mod persistence;

const MAX_RECORDS: usize = 128;
const MAX_ACTIVE: usize = 3;
const MAX_RECEIPTS: usize = 64;
const INVALID: SaveError =
    SaveError::InvalidState("T04 plan, identified tonne or transaction chronology");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationTradeAction {
    Accept,
    Plan {
        destination: SystemId,
        commodity: CommodityId,
    },
    Receive,
    Cancel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationTradeOutcome {
    Complete,
    Abandoned,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationTradeError {
    NoCommander,
    StaleReview,
    Paused,
    BerthRequired,
    AlreadyActive,
    NoActiveAttempt,
    AlreadyPlanned,
    PlanRequired,
    UnsupportedRoute,
    CommodityRequired,
    CargoPresent,
    Capacity,
    Stock,
    Credits,
    Fuel,
    TestFit,
    FitOrStockChanged,
    NotReady,
    ReceiptCapacity,
    CreditOverflow,
    Service(services::Error),
    Save(SaveError),
}
impl core::fmt::Display for NavigationTradeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "navigation trade practice refused: {self:?}")
    }
}
impl std::error::Error for NavigationTradeError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationTradeId {
    owner: u64,
    ordinal: u8,
}
impl NavigationTradeId {
    pub fn owner(self) -> u64 {
        self.owner
    }
    pub fn ordinal(self) -> u8 {
        self.ordinal
    }
}
/// Identity is the actual original purchase transaction, never an aggregate cargo label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationTonneId {
    owner: u64,
    attempt: u8,
    transaction: u64,
}
impl NavigationTonneId {
    pub fn owner(self) -> u64 {
        self.owner
    }
    pub fn attempt(self) -> u8 {
        self.attempt
    }
    pub fn transaction(self) -> u64 {
        self.transaction
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavigationTradeReview {
    action: NavigationTradeAction,
    dock: DockContext,
    tick: u64,
    epoch: u64,
    fingerprint: u64,
    plan: Option<Plan>,
}
impl NavigationTradeReview {
    pub fn action(self) -> NavigationTradeAction {
        self.action
    }
    pub fn plan(self) -> Option<NavigationTradePlanSnapshot> {
        self.plan.map(Plan::snapshot)
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavigationTradePlanSnapshot {
    pub destination: SystemId,
    pub commodity: CommodityId,
    pub price_tenths: u32,
    pub stock: u64,
    pub distance_ly: f64,
    pub fuel_debit: u16,
    pub confirmed_tick: u64,
    pub capacity_tonnes: u16,
    pub cargo_tonnes: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationTradeTrace {
    pub source_id: &'static str,
    pub clue_id: &'static str,
    pub receipt_id: &'static str,
    pub attempt: NavigationTradeId,
    pub confirmed_tick: u64,
    pub dock_sequence: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationTradeMarketSnapshot {
    pub tonne: NavigationTonneId,
    pub dock: DockContext,
    pub tick: u64,
    pub transaction: u64,
    pub price_tenths: u32,
    pub stock_before: u64,
    pub stock_after: u64,
    pub cargo_before: u64,
    pub cargo_after: u64,
    pub credits_before: u32,
    pub credits_after: u32,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationTradeAttemptSnapshot {
    pub id: NavigationTradeId,
    pub issuing_dock: DockContext,
    pub accepted_tick: u64,
    pub plan: Option<NavigationTradePlanSnapshot>,
    pub buy: Option<NavigationTradeMarketSnapshot>,
    pub held_tonne: Option<NavigationTonneId>,
    pub sale: Option<NavigationTradeMarketSnapshot>,
    pub journeys: usize,
    pub fuel_units_used: u32,
    pub fuel_paid_tenths: u32,
    pub cash_difference_tenths: Option<i64>,
    pub outcome: Option<NavigationTradeOutcome>,
    pub terminal_tick: Option<u64>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationTradeSnapshot {
    pub attempts: Vec<NavigationTradeAttemptSnapshot>,
    pub active: Option<NavigationTradeId>,
    pub reserved_capacity_tonnes: u16,
    pub ready: bool,
    pub trace: Option<NavigationTradeTrace>,
    pub active_services: usize,
    pub retained_mission_records: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fit {
    inventory: u64,
    items: [Option<u64>; 8],
    equipment_revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Plan {
    destination: SystemId,
    commodity: CommodityId,
    confirmed_tick: u64,
    fit: Fit,
    capacity: u16,
    cargo: u16,
    price: u32,
    stock: u64,
    distance: f64,
    fuel_debit: u16,
    fuel_at_plan: u16,
    refilled_at_plan: u64,
    credits_at_plan: u32,
    account_revision: u64,
    market_revision: u64,
    bought: u64,
    sold: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Balance {
    stock: u64,
    cargo: u64,
    credits: u32,
    bought: u64,
    sold: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Trade {
    tonne: NavigationTonneId,
    dock: DockContext,
    tick: u64,
    transaction: u64,
    account_revision: u64,
    jump_sequence: u64,
    fuel_at_trade: u16,
    price: u32,
    before: Balance,
    after: Balance,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Journey {
    tick: u64,
    account_revision: u64,
    jump: u64,
    origin: SystemId,
    destination: SystemId,
    source_visit: u64,
    arrival_visit: u64,
    fuel_before: u16,
    fuel_after: u16,
    debit: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fuel {
    tick: u64,
    dock: DockContext,
    transaction: u64,
    account_revision: u64,
    ordinal: u64,
    units_before: u16,
    units_after: u16,
    credits_before: u32,
    credits_after: u32,
    paid: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FuelAnchor {
    refilled_units: u64,
    fuel: u16,
    jump: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Terminal {
    tick: u64,
    dock: DockContext,
    outcome: NavigationTradeOutcome,
    fuel: FuelAnchor,
    account_revision: Option<u64>,
}
#[derive(Debug, Clone, PartialEq)]
struct Attempt {
    dock: DockContext,
    accepted_tick: u64,
    accepted_account_revision: Option<u64>,
    plan: Option<Plan>,
    buy: Option<Trade>,
    journeys: Vec<Journey>,
    fuel: Vec<Fuel>,
    sale: Option<Trade>,
    terminal: Option<Terminal>,
}
#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct State {
    attempts: Vec<Attempt>,
    trace: Option<u8>,
}
impl State {
    fn active(&self) -> Option<&Attempt> {
        self.attempts.last().filter(|a| a.terminal.is_none())
    }
    pub(super) fn occupancy(&self) -> (usize, usize) {
        (self.attempts.len(), usize::from(self.active().is_some()))
    }
    /// T04 consent/closure do not advance the account or simulation tick; only
    /// a strictly interior retained tick proves a prior active interval.
    pub(super) fn known_active_at(&self, tick: u64, revision: u64) -> bool {
        self.attempts.iter().any(|attempt| {
            if let Some(accepted) = attempt.accepted_account_revision {
                accepted < revision
                    && attempt.terminal.is_none_or(|terminal| {
                        terminal
                            .account_revision
                            .map_or(terminal.tick > tick, |closed| closed >= revision)
                    })
            } else {
                attempt.accepted_tick < tick
                    && attempt.terminal.is_none_or(|terminal| terminal.tick > tick)
            }
        })
    }
    pub(super) fn known_acceptance_revisions(&self) -> impl Iterator<Item = u64> + '_ {
        self.attempts
            .iter()
            .filter_map(|attempt| attempt.accepted_account_revision)
    }
    pub(super) fn reservations(&self) -> services::Reservations {
        let Some(a) = self.active() else {
            return services::Reservations::default();
        };
        let Some(plan) = a.plan else {
            return services::Reservations::default();
        };
        services::Reservations {
            capacity: u16::from(a.buy.is_none()),
            commodity: a.held().then_some(plan.commodity),
        }
    }
    fn id(&self, ordinal: usize) -> NavigationTradeId {
        NavigationTradeId {
            owner: self.attempts[ordinal].dock.owner().seed(),
            ordinal: ordinal as u8,
        }
    }
    fn trace_snapshot(&self) -> Option<NavigationTradeTrace> {
        let i = usize::from(self.trace?);
        let t = self.attempts.get(i)?.terminal?;
        Some(NavigationTradeTrace {
            source_id: "l337.training/navigation-trade",
            clue_id: "navigation-trade-family-trace",
            receipt_id: "l337.unwritten-route/clue/navigation-trade-family-trace",
            attempt: self.id(i),
            confirmed_tick: t.tick,
            dock_sequence: t.dock.sequence(),
        })
    }
}
impl Attempt {
    fn held(&self) -> bool {
        self.buy.is_some() && self.sale.is_none() && self.terminal.is_none()
    }
    fn selected_journey(&self) -> Option<Journey> {
        let plan = self.plan?;
        let buy = self.buy?;
        self.journeys.iter().copied().find(|j| {
            j.origin == self.dock.system()
                && j.destination == plan.destination
                && j.debit == plan.fuel_debit
                && j.tick > buy.tick
        })
    }
    fn ready(&self, dock: DockContext) -> bool {
        self.terminal.is_none() && self.return_receipt_ready(dock)
    }
    fn return_receipt_ready(&self, dock: DockContext) -> bool {
        let Some(sale) = self.sale else {
            return false;
        };
        let Some(last) = self.journeys.last() else {
            return false;
        };
        same_carrier(self.dock, dock)
            && dock.system() == self.dock.system()
            && dock.origin() == crate::navigation::DockOrigin::PhysicalEntry
            && dock.sequence() > self.dock.sequence()
            && last.destination == self.dock.system()
            && last.tick > sale.tick
            && last.arrival_visit == dock.visit()
            && dock.tick() >= last.tick
    }
    fn fuel_paid(&self) -> u32 {
        self.fuel.iter().map(|f| f.paid).sum()
    }
    fn fuel_used(&self) -> u32 {
        self.journeys.iter().map(|j| u32::from(j.debit)).sum()
    }
}
impl Plan {
    fn snapshot(self) -> NavigationTradePlanSnapshot {
        NavigationTradePlanSnapshot {
            destination: self.destination,
            commodity: self.commodity,
            price_tenths: self.price,
            stock: self.stock,
            distance_ly: self.distance,
            fuel_debit: self.fuel_debit,
            confirmed_tick: self.confirmed_tick,
            capacity_tonnes: self.capacity,
            cargo_tonnes: self.cargo,
        }
    }
}
impl Trade {
    fn snapshot(self) -> NavigationTradeMarketSnapshot {
        NavigationTradeMarketSnapshot {
            tonne: self.tonne,
            dock: self.dock,
            tick: self.tick,
            transaction: self.transaction,
            price_tenths: self.price,
            stock_before: self.before.stock,
            stock_after: self.after.stock,
            cargo_before: self.before.cargo,
            cargo_after: self.after.cargo,
            credits_before: self.before.credits,
            credits_after: self.after.credits,
        }
    }
}
fn same_carrier(a: DockContext, b: DockContext) -> bool {
    a.owner() == b.owner() && a.ship() == b.ship()
}
fn legal_tonne(id: CommodityId) -> bool {
    id.unit() == services::CargoUnit::Tonne
        && ![
            CommodityId::Slaves,
            CommodityId::Narcotics,
            CommodityId::Weapons,
            CommodityId::AlienItems,
        ]
        .contains(&id)
}
fn quantity(p: services::AccountProjection, id: CommodityId) -> u64 {
    if id == CommodityId::Food {
        u64::from(p.food_tonnes)
    } else {
        p.other_commodities[id.index() - 1]
    }
}
fn site(system: SystemId) -> Option<services::MarketSite> {
    super::services::service_site(system)
}

/// Complete lesson transition. The host atomically publishes bytes before committing live state.
pub struct PreparedNavigationTrade<'a> {
    world: &'a mut World,
    candidate: State,
    bytes: Vec<u8>,
}
impl PreparedNavigationTrade<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn commit(self) {
        self.world.navigation_trade = self.candidate;
    }
}
/// Ordinary commerce plus its exact lesson receipt, prepared under one exclusive World borrow.
pub struct PreparedStationService<'a> {
    world: &'a mut World,
    account: CommanderAccount,
    markets: StationMarkets,
    lesson: State,
    event: services::Event,
    bytes: Vec<u8>,
}
impl PreparedStationService<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn event(&self) -> services::Event {
        self.event
    }
    pub fn commit(self) -> services::Event {
        self.world.commander_account = Some(self.account);
        self.world.station_markets = Some(self.markets);
        self.world.navigation_trade = self.lesson;
        self.event
    }
}
#[cfg(test)]
mod tests;

impl World {
    fn navigation_trade_fuel_anchor(&self) -> Option<FuelAnchor> {
        Some(FuelAnchor {
            refilled_units: self.station_markets.as_ref()?.fuel_accounting().0,
            fuel: self.commander_account.as_ref()?.fuel_tenths(),
            jump: self.navigation?.jump_sequence(),
        })
    }
}

//! Canonical standard Missile: original paid main-berth offer and finite owned custody.
//! The original static shop has no mutable stock quantity. A purchase creates one item.
use super::{
    World,
    missiles::{MissileOrigin, MissileRound},
};
use crate::{
    freight::g02::CommanderAccount,
    navigation::{DockContext, DockOrigin},
    save::{Reader, SaveError, Writer},
};
use core::fmt;
pub const PRICE_TENTHS: u32 = 1800;
pub const REQUIRED_TECHNOLOGY: u8 = 2;
pub const PYLONS: usize = 9;
const INVALID: SaveError = SaveError::InvalidState("paid missile ownership, fitting or chronology");
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmunitionAction {
    Purchase,
    Store { item: u64 },
    Refit { item: u64, pylon: u8 },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmunitionError {
    StaleReview,
    Paused,
    BerthRequired,
    NoCommander,
    Full,
    MissingItem,
    Occupied,
    NoChange,
    InsufficientCredits,
    SequenceExhausted,
    Save(SaveError),
}
impl fmt::Display for AmmunitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Missile fitting refused: {self:?}")
    }
}
impl std::error::Error for AmmunitionError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmmunitionItem {
    pub item: u64,
    pub pylon: Option<u8>,
    pub purchase: Option<u32>,
    pub consumed: bool,
    pub xyzzy: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissilePurchase {
    pub transaction: u32,
    pub item: u64,
    pub dock: DockContext,
    pub price_tenths: u32,
    pub offer_revision: u64,
    pub account_revision: u64,
    pub tick: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmmunitionSnapshot {
    pub node: &'static str,
    pub name: &'static str,
    pub purchases: Vec<MissilePurchase>,
    pub items: Vec<AmmunitionItem>,
    pub capacity: u8,
    pub price_tenths: u32,
    pub required_technology: u8,
    pub fit_revision: u64,
    pub purchase_available: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmmunitionReview {
    action: AmmunitionAction,
    dock: DockContext,
    tick: u64,
    epoch: u64,
    revision: u64,
    fit_revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct State {
    events: Vec<Event>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Event {
    xyzzy: bool,
    action: AmmunitionAction,
    dock: DockContext,
    tick: u64,
    revision: u64,
    item: u64,
}
pub struct PreparedAmmunition<'a> {
    world: &'a mut World,
    account: CommanderAccount,
    state: State,
    bytes: Vec<u8>,
    item: u64,
}
impl PreparedAmmunition<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn item(&self) -> u64 {
        self.item
    }
    /// The host's atomic writer must succeed before the World can publish this candidate.
    pub fn publish_with<E>(self, publish: impl FnOnce(&[u8]) -> Result<(), E>) -> Result<u64, E> {
        publish(&self.bytes)?;
        Ok(self.commit())
    }
    pub fn commit(self) -> u64 {
        self.world.commander_account = Some(self.account);
        self.world.ammunition = self.state;
        self.item
    }
}
#[derive(Default)]
pub(super) struct Ledger {
    pub(super) items: Vec<AmmunitionItem>,
    pub(super) paid: u32,
    pub(super) extra_revisions: u64,
}
impl Ledger {
    fn rounds(&self) -> [Option<MissileRound>; PYLONS] {
        let mut rounds = [None; PYLONS];
        for item in &self.items {
            if let Some(pylon) = item.pylon {
                rounds[usize::from(pylon)] = Some(MissileRound {
                    item: item.item,
                    pylon,
                });
            }
        }
        rounds
    }
    fn apply(
        &mut self,
        action: AmmunitionAction,
        item: u64,
        purchase: Option<u32>,
    ) -> Result<(), AmmunitionError> {
        match action {
            AmmunitionAction::Purchase => {
                // The ordinary inventory bound excludes the one reserved stock drive.
                if self.items.iter().filter(|i| !i.consumed).count() >= 252 {
                    return Err(AmmunitionError::Full);
                }
                let pylon = self
                    .rounds()
                    .iter()
                    .position(Option::is_none)
                    .ok_or(AmmunitionError::Full)? as u8;
                self.items.push(AmmunitionItem {
                    item,
                    pylon: Some(pylon),
                    purchase,
                    consumed: false,
                    xyzzy: purchase.is_none(),
                });
                self.paid = self
                    .paid
                    .checked_add(if purchase.is_some() { PRICE_TENTHS } else { 0 })
                    .ok_or(AmmunitionError::SequenceExhausted)?;
            }
            AmmunitionAction::Store { item } => {
                let i = self
                    .items
                    .iter_mut()
                    .find(|i| i.item == item && !i.consumed)
                    .ok_or(AmmunitionError::MissingItem)?;
                if i.pylon.is_none() {
                    return Err(AmmunitionError::NoChange);
                }
                i.pylon = None;
            }
            AmmunitionAction::Refit { item, pylon } => {
                if usize::from(pylon) >= PYLONS {
                    return Err(AmmunitionError::Full);
                }
                if self
                    .items
                    .iter()
                    .any(|i| i.pylon == Some(pylon) && i.item != item)
                {
                    return Err(AmmunitionError::Occupied);
                }
                let i = self
                    .items
                    .iter_mut()
                    .find(|i| i.item == item && !i.consumed)
                    .ok_or(AmmunitionError::MissingItem)?;
                if i.pylon == Some(pylon) {
                    return Err(AmmunitionError::NoChange);
                }
                i.pylon = Some(pylon);
            }
        }
        self.extra_revisions = self
            .extra_revisions
            .checked_add(1)
            .ok_or(AmmunitionError::SequenceExhausted)?;
        Ok(())
    }
    fn launch(&mut self, origin: MissileOrigin) -> Result<(), SaveError> {
        let round = self
            .rounds()
            .into_iter()
            .rev()
            .flatten()
            .next()
            .ok_or(INVALID)?;
        if round.item != origin.item {
            return Err(INVALID);
        }
        let item = self
            .items
            .iter_mut()
            .find(|i| i.item == round.item)
            .ok_or(INVALID)?;
        item.pylon = None;
        item.consumed = true;
        self.extra_revisions += 1;
        Ok(())
    }
}
fn validate_ammunition_identity(
    e: &Event,
    world: &World,
    owner: u64,
    mut purchases: u32,
    mut grants: u32,
) -> Result<(u32, u32), SaveError> {
    if e.xyzzy {
        if e.action != AmmunitionAction::Purchase
            || !world.xyzzy.authorizes(e.dock, e.tick, e.revision)
        {
            return Err(INVALID);
        }
        grants = grants.checked_add(1).ok_or(INVALID)?;
        if grants > 100000
            || e.item
                != owner
                    .checked_add(300010 + u64::from(grants))
                    .ok_or(INVALID)?
        {
            return Err(INVALID);
        }
    } else if e.action == AmmunitionAction::Purchase {
        purchases = purchases.checked_add(1).ok_or(INVALID)?;
        if purchases > 100000
            || e.item
                != owner
                    .checked_add(10 + u64::from(purchases))
                    .ok_or(INVALID)?
        {
            return Err(INVALID);
        }
    } else if match e.action {
        AmmunitionAction::Store { item } | AmmunitionAction::Refit { item, .. } => item != e.item,
        _ => false,
    } {
        return Err(INVALID);
    }
    Ok((purchases, grants))
}

impl State {
    pub(super) fn account_events(&self) -> Vec<super::AccountEvent> {
        self.events
            .iter()
            .filter_map(|e| {
                (e.action == AmmunitionAction::Purchase).then_some(super::AccountEvent {
                    revision: e.revision,
                    effect: super::AccountEffect::Debit(if e.xyzzy { 0 } else { PRICE_TENTHS }),
                })
            })
            .collect()
    }

    pub(super) fn ledger(
        &self,
        world: &World,
        account: &CommanderAccount,
    ) -> Result<Ledger, SaveError> {
        self.ledger_with_module_count(world, account, world.modular.paid_count())
    }
    pub(super) fn ledger_with_module_count(
        &self,
        world: &World,
        account: &CommanderAccount,
        modules: usize,
    ) -> Result<Ledger, SaveError> {
        let owner = world.combat_identity().ok_or(INVALID)?.owner;
        let mut ledger = Ledger::default();
        for pylon in 0..3 {
            ledger.items.push(AmmunitionItem {
                item: owner.checked_add(8 + u64::from(pylon)).ok_or(INVALID)?,
                pylon: Some(pylon),
                purchase: None,
                consumed: false,
                xyzzy: false,
            });
        }
        let mut launches = world
            .missiles
            .projectiles
            .iter()
            .flatten()
            .map(|p| p.origin)
            .peekable();
        let mut purchases = 0_u32;
        let mut grants = 0_u32;
        let mut previous: Option<Event> = None;
        for e in &self.events {
            validate_berth_event(
                world,
                account,
                e.dock,
                e.tick,
                e.revision,
                previous.map(|p| (p.dock.sequence(), p.tick, p.revision)),
            )?;
            while launches.peek().is_some_and(|o| o.launched_tick <= e.tick) {
                let o = launches.next().ok_or(INVALID)?;
                if o.launched_tick >= e.dock.tick() && o.owner.visit >= e.dock.visit() {
                    return Err(INVALID);
                }
                ledger.launch(o)?;
            }
            (purchases, grants) = validate_ammunition_identity(e, world, owner, purchases, grants)?;
            ledger
                .apply(e.action, e.item, (!e.xyzzy).then_some(purchases))
                .map_err(|_| INVALID)?;
            previous = Some(*e);
        }
        for o in launches {
            ledger.launch(o)?;
        }
        for (pylon, item) in ledger.items.iter().take(3).enumerate() {
            let present = account
                .starter_items()
                .ok_or(INVALID)?
                .iter()
                .any(|i| i.id() == item.item);
            if present == item.consumed || item.item != owner + 8 + pylon as u64 {
                return Err(INVALID);
            }
        }
        if ledger.extra_revisions != account.ammunition_revision() {
            return Err(INVALID);
        }
        let foundation = 4
            + usize::from(account.equipment_state().owned().is_some())
            + modules
            + usize::from(world.emp.owned().ok().flatten().is_some());
        // The one reserved stock drive is exempt from the original ordinary-item bound.
        if ledger.items.iter().filter(|i| !i.consumed).count() + foundation > 256 {
            return Err(INVALID);
        }
        Ok(ledger)
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.u32(self.events.len() as u32);
        for e in &self.events {
            match e.action {
                AmmunitionAction::Purchase => w.u8(0),
                AmmunitionAction::Store { .. } => w.u8(1),
                AmmunitionAction::Refit { pylon, .. } => {
                    w.u8(2);
                    w.u8(pylon);
                }
            }
            e.dock.encode_save(w);
            w.u64(e.tick);
            w.u64(e.revision);
            w.u64(e.item);
            w.bool(e.xyzzy);
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let count = r.u32()? as usize;
        // Every event uses at least 64 bytes; the existing complete-save byte cap remains authoritative.
        if count > crate::save::MAX_SAVE_BYTES / 64 {
            return Err(SaveError::TooLarge);
        }
        let mut events = Vec::new();
        for _ in 0..count {
            let tag = r.u8()?;
            let pylon = if tag == 2 { r.u8()? } else { 0 };
            let dock = DockContext::decode_save(r)?;
            let tick = r.u64()?;
            let revision = r.u64()?;
            let item = r.u64()?;
            let xyzzy = r.bool()?;
            let action = match tag {
                0 => AmmunitionAction::Purchase,
                1 => AmmunitionAction::Store { item },
                2 => AmmunitionAction::Refit { item, pylon },
                _ => return Err(SaveError::InvalidEncoding),
            };
            events.push(Event {
                xyzzy,
                action,
                dock,
                tick,
                revision,
                item,
            });
        }
        Ok(Self { events })
    }
}
impl World {
    pub(super) fn ammunition_rounds(&self) -> Option<[Option<MissileRound>; PYLONS]> {
        Some(
            self.ammunition
                .ledger(self, self.commander_account.as_ref()?)
                .ok()?
                .rounds(),
        )
    }
    pub fn ammunition_snapshot(&self) -> Option<AmmunitionSnapshot> {
        let account = self.commander_account.as_ref()?;
        let ledger = self.ammunition.ledger(self, account).ok()?;
        let purchase_available = self
            .review_ammunition(AmmunitionAction::Purchase, self.state.tick)
            .is_ok();
        let purchases = self
            .ammunition
            .events
            .iter()
            .filter(|e| !e.xyzzy && e.action == AmmunitionAction::Purchase)
            .enumerate()
            .map(|(i, e)| MissilePurchase {
                transaction: (i + 1) as u32,
                item: e.item,
                dock: e.dock,
                price_tenths: PRICE_TENTHS,
                offer_revision: 1,
                account_revision: e.revision,
                tick: e.tick,
            })
            .collect();
        Some(AmmunitionSnapshot {
            node: "missile-standard",
            name: "Missile",
            purchases,
            items: ledger.items,
            capacity: PYLONS as u8,
            price_tenths: PRICE_TENTHS,
            required_technology: REQUIRED_TECHNOLOGY,
            fit_revision: account.ammunition_revision(),
            purchase_available,
        })
    }
    pub fn review_ammunition(
        &self,
        action: AmmunitionAction,
        tick: u64,
    ) -> Result<AmmunitionReview, AmmunitionError> {
        if tick != self.state.tick {
            return Err(AmmunitionError::StaleReview);
        }
        if self.is_paused() {
            return Err(AmmunitionError::Paused);
        }
        let account = self
            .commander_account
            .as_ref()
            .ok_or(AmmunitionError::NoCommander)?;
        let facts = self
            .current_service_facts(tick)
            .map_err(|_| AmmunitionError::BerthRequired)?;
        let dock = self
            .current_dock_context()
            .ok_or(AmmunitionError::BerthRequired)?;
        if self.state.mode != super::FlightMode::Docked
            || self.combat_snapshot().is_none_or(|s| s.player.destroyed)
            || crate::equipment::definition()
                .map_err(|_| AmmunitionError::BerthRequired)?
                .offer(facts.site)
                .technology
                < REQUIRED_TECHNOLOGY
        {
            return Err(AmmunitionError::BerthRequired);
        }
        let mut ledger = self
            .ammunition
            .ledger(self, account)
            .map_err(AmmunitionError::Save)?;
        if action == AmmunitionAction::Purchase && account.credits_tenths() < PRICE_TENTHS {
            return Err(AmmunitionError::InsufficientCredits);
        }
        ledger.apply(action, 0, Some(0))?;
        if ledger.items.iter().filter(|i| !i.consumed).count()
            + 4
            + usize::from(account.equipment_state().owned().is_some())
            + self.modular.paid_count()
            + usize::from(self.emp.owned().ok().flatten().is_some())
            > 256
        {
            return Err(AmmunitionError::Full);
        }
        Ok(AmmunitionReview {
            action,
            dock,
            tick,
            epoch: self
                .freight_session
                .as_ref()
                .ok_or(AmmunitionError::NoCommander)?
                .epoch(),
            revision: account.revision(),
            fit_revision: account.ammunition_revision(),
        })
    }
    pub fn prepare_ammunition(
        &mut self,
        review: AmmunitionReview,
        tick: u64,
    ) -> Result<PreparedAmmunition<'_>, AmmunitionError> {
        if self.review_ammunition(review.action, tick)? != review {
            return Err(AmmunitionError::StaleReview);
        }
        let current = self
            .commander_account
            .as_ref()
            .ok_or(AmmunitionError::NoCommander)?;
        let count = self
            .ammunition
            .events
            .iter()
            .filter(|e| !e.xyzzy && e.action == AmmunitionAction::Purchase)
            .count();
        let item = match review.action {
            AmmunitionAction::Purchase => review
                .dock
                .owner()
                .seed()
                .checked_add(11 + count as u64)
                .ok_or(AmmunitionError::SequenceExhausted)?,
            AmmunitionAction::Store { item } | AmmunitionAction::Refit { item, .. } => item,
        };
        if count >= 100000 {
            return Err(AmmunitionError::Full);
        }
        let paid = if review.action == AmmunitionAction::Purchase {
            PRICE_TENTHS
        } else {
            0
        };
        let account = current
            .prepare_ammunition(paid, None, false)
            .map_err(|_| AmmunitionError::SequenceExhausted)?;
        let mut state = self.ammunition.clone();
        state.events.push(Event {
            xyzzy: false,
            action: review.action,
            dock: review.dock,
            tick,
            revision: account.revision(),
            item,
        });
        let bytes = self
            .save_bytes_with_ammunition(&account, &state)
            .map_err(AmmunitionError::Save)?;
        Ok(PreparedAmmunition {
            world: self,
            account,
            state,
            bytes,
            item,
        })
    }
}

#[cfg(test)]
mod tests;

pub(super) fn validate_berth_event(
    world: &World,
    account: &CommanderAccount,
    dock: DockContext,
    tick: u64,
    revision: u64,
    previous: Option<(u64, u64, u64)>,
) -> Result<(), SaveError> {
    let owner = world.combat_identity().ok_or(INVALID)?;
    let nav = world.navigation.ok_or(INVALID)?.validate_save(
        world.navigation_catalogue.as_ref().ok_or(INVALID)?,
        world.state.mode,
        world.state.tick,
        account.fuel_tenths(),
    )?;
    if dock.ship().owner() != dock.owner()
        || dock.owner().seed() != owner.owner
        || dock.ship().ordinal() != world.combat_identity().ok_or(INVALID)?.hull
        || super::services::service_site(dock.system()).is_none()
        || dock.sequence() == 0
        || dock.sequence() > nav.dock_sequence
        || dock.visit() < dock.sequence()
        || dock.visit() > nav.visit
        || dock.tick() > tick
        || tick > world.state.tick
        || revision == 0
        || revision > account.revision()
        || previous.is_some_and(|p| tick < p.1 || revision <= p.2 || dock.sequence() < p.0)
        || world
            .current_dock_context()
            .is_some_and(|d| d.sequence() == dock.sequence() && d != dock)
    {
        return Err(INVALID);
    }
    match dock.origin() {
        DockOrigin::InitialStart
            if dock.sequence() == 1
                && dock.visit() == 1
                && dock.tick() == 0
                && dock.system() == crate::navigation::SystemId::AVENOR => {}
        DockOrigin::PhysicalEntry if dock.sequence() > 1 && dock.tick() > 0 => {}
        _ => return Err(INVALID),
    }
    Ok(())
}

impl World {
    pub(super) fn grant_xyzzy_missiles(
        &mut self,
        dock: DockContext,
        tick: u64,
    ) -> Result<(), SaveError> {
        loop {
            let current = self.commander_account.as_ref().ok_or(INVALID)?;
            let ledger = self.ammunition.ledger(self, current)?;
            let Some(pylon) = ledger.rounds().iter().position(Option::is_none) else {
                return Ok(());
            };
            let held = ledger
                .items
                .iter()
                .find(|i| !i.consumed && i.pylon.is_none());
            let (action, item, xyzzy) = if let Some(held) = held {
                (
                    AmmunitionAction::Refit {
                        item: held.item,
                        pylon: pylon as u8,
                    },
                    held.item,
                    false,
                )
            } else {
                let n = self.ammunition.events.iter().filter(|e| e.xyzzy).count();
                if n >= 100000 {
                    return Err(INVALID);
                }
                (
                    AmmunitionAction::Purchase,
                    dock.owner()
                        .seed()
                        .checked_add(300011 + n as u64)
                        .ok_or(INVALID)?,
                    true,
                )
            };
            let account = current
                .prepare_ammunition(0, None, false)
                .map_err(|_| INVALID)?;
            let event = Event {
                xyzzy,
                action,
                dock,
                tick,
                revision: account.revision(),
                item,
            };
            self.commander_account = Some(account);
            self.ammunition.events.push(event);
        }
    }
}

#[cfg(test)]
mod xyzzy_tests;

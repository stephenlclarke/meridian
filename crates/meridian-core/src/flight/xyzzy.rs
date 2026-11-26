//! Explicit secret transactions. No getter, normal purchase or save decoder grants property.
mod history;
use super::{FlightMode, ShieldGrade, World};
use crate::navigation::DockContext;
use crate::save::{Reader, SaveError, Writer};
pub(crate) use history::{AccountEffect, AccountEvent};

pub const STARTING_CREDITS_TENTHS: u32 = 10_000_000;
pub const COMMANDER_NAME: &str = "CLARKE";
const INVALID: SaveError = SaveError::InvalidState("XYZZY activation, grants or supply chronology");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyzzyAction {
    /// Host invokes only after the explicit xyzzy sequence or labelled secret activation.
    Enter,
    MaximumSupportedHardware,
    RestoreStarter,
    ToggleUnlimitedFuel,
    /// Meridian disposable-preview test access, separate from the original maximum outfit.
    FitShield(ShieldGrade),
    /// Separately labelled maximum test fit, including the original four-tank exception.
    MaximumFuelStorage,
}
impl XyzzyAction {
    fn outfit(self) -> bool {
        matches!(
            self,
            Self::Enter
                | Self::MaximumSupportedHardware
                | Self::RestoreStarter
                | Self::MaximumFuelStorage
        )
    }
    fn changes_equipment(self) -> bool {
        self.outfit() || matches!(self, Self::FitShield(_))
    }
    fn refills(self, mode: bool) -> bool {
        !matches!(self, Self::FitShield(_)) && (self.outfit() || mode)
    }
    fn tag(self) -> u8 {
        match self {
            Self::Enter => 0,
            Self::MaximumSupportedHardware => 1,
            Self::RestoreStarter => 2,
            Self::ToggleUnlimitedFuel => 3,
            Self::FitShield(grade) => 4 + grade as u8,
            Self::MaximumFuelStorage => 11,
        }
    }
    fn from_tag(tag: u8) -> Result<Self, SaveError> {
        match tag {
            0 => Ok(Self::Enter),
            1 => Ok(Self::MaximumSupportedHardware),
            2 => Ok(Self::RestoreStarter),
            3 => Ok(Self::ToggleUnlimitedFuel),
            4..=10 => Ok(Self::FitShield(ShieldGrade::from_tag(tag - 4)?)),
            11 => Ok(Self::MaximumFuelStorage),
            _ => Err(SaveError::InvalidEncoding),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyzzyError {
    NoCommander,
    StaleReview,
    Paused,
    BerthRequired,
    Inactive,
    ActiveFuelPractice,
    ActiveContractCustody,
    Save(SaveError),
}
impl core::fmt::Display for XyzzyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "XYZZY action refused: {self:?}")
    }
}
impl std::error::Error for XyzzyError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XyzzyReview {
    action: XyzzyAction,
    dock: DockContext,
    tick: u64,
    epoch: u64,
    revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XyzzySnapshot {
    pub active: bool,
    pub commander_name: &'static str,
    pub label: &'static str,
    pub unlimited_fuel: bool,
    pub cash_granted_once: bool,
    pub cash_tenths: u32,
    pub jump_limit_ly: f64,
    /// Full original outfit still needs broader targeting, utility and hull owners.
    pub full_legacy_outfit_available: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Record {
    action: XyzzyAction,
    dock: DockContext,
    tick: u64,
    revision: u64,
    end_revision: u64,
    cash_before_reset: Option<u32>,
    fuel_added: u16,
    fuel_spent: u64,
    unlimited_fuel: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct State {
    records: Vec<Record>,
}
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Terms {
    pub(super) cash_adjustment: i128,
    pub(super) fuel_supplied: u128,
    pub(super) revisions: u64,
}
impl State {
    pub(super) fn active(&self) -> bool {
        !self.records.is_empty()
    }
    fn unlimited_fuel(&self) -> bool {
        self.records.last().is_some_and(|r| r.unlimited_fuel)
    }
    pub(super) fn authorizes(&self, dock: DockContext, tick: u64, revision: u64) -> bool {
        self.records.iter().any(|r| {
            r.action.outfit()
                && r.dock == dock
                && r.tick == tick
                && revision > r.revision
                && revision <= r.end_revision
        })
    }
    pub(super) fn authorizes_shield(
        &self,
        grade: ShieldGrade,
        dock: DockContext,
        tick: u64,
        revision: u64,
    ) -> bool {
        self.records.iter().any(|record| {
            (record.action == XyzzyAction::FitShield(grade)
                || (record.action.outfit() && grade == ShieldGrade::Elite))
                && record.dock == dock
                && record.tick == tick
                && revision > record.revision
                && revision <= record.end_revision
        })
    }
    pub(super) fn authorizes_max_tanks(&self, dock: DockContext, tick: u64, revision: u64) -> bool {
        self.records.iter().any(|record| {
            record.action == XyzzyAction::MaximumFuelStorage
                && record.dock == dock
                && record.tick == tick
                && revision > record.revision
                && revision <= record.end_revision
        })
    }
    pub(super) fn max_tank_intervals(
        &self,
    ) -> impl Iterator<Item = (u64, u64, DockContext, u64)> + '_ {
        self.records
            .iter()
            .filter(|record| record.action == XyzzyAction::MaximumFuelStorage)
            .map(|record| {
                (
                    record.revision,
                    record.end_revision,
                    record.dock,
                    record.tick,
                )
            })
    }
    /// The exact retained fuel prefix at a T04 terminal, independent of later
    /// explicit supply. The full account validator still checks every receipt.
    pub(super) fn historical_fuel(
        &self,
        world: &World,
        markets: &crate::station_services::StationMarkets,
        dock: DockContext,
        tick: u64,
        revision: u64,
    ) -> Result<(u64, u16), SaveError> {
        let spent = world.jump_journal.spent_at(dock, tick)?;
        let mut journal_spent = 0_u64;
        let mut jumps = 0_u64;
        for event in world
            .jump_journal
            .account_events()
            .into_iter()
            .filter(|event| event.revision <= revision)
        {
            if let AccountEffect::Jump(debit) = event.effect {
                journal_spent = journal_spent.checked_add(u64::from(debit)).ok_or(INVALID)?;
                jumps = jumps.checked_add(1).ok_or(INVALID)?;
            }
        }
        if journal_spent != spent || dock.visit().checked_sub(dock.sequence()) != Some(jumps) {
            return Err(INVALID);
        }
        let mut refilled = 0_u64;
        for event in markets
            .account_events()
            .into_iter()
            .filter(|event| event.revision <= revision)
        {
            if let AccountEffect::Refill { units, .. } = event.effect {
                refilled = refilled.checked_add(u64::from(units)).ok_or(INVALID)?;
            }
        }
        let mut supplied = 0_u64;
        let mut previous: Option<Record> = None;
        for record in self
            .records
            .iter()
            .take_while(|record| record.revision <= revision)
        {
            if record.end_revision > revision || record.fuel_spent > spent {
                return Err(INVALID);
            }
            if let Some(prior) = previous
                && prior.unlimited_fuel
            {
                supplied = supplied
                    .checked_add(
                        record
                            .fuel_spent
                            .checked_sub(prior.fuel_spent)
                            .ok_or(INVALID)?,
                    )
                    .ok_or(INVALID)?;
            }
            supplied = supplied
                .checked_add(u64::from(record.fuel_added))
                .ok_or(INVALID)?;
            previous = Some(*record);
        }
        if let Some(prior) = previous
            && prior.unlimited_fuel
        {
            supplied = supplied
                .checked_add(spent.checked_sub(prior.fuel_spent).ok_or(INVALID)?)
                .ok_or(INVALID)?;
        }
        let fuel = u64::from(crate::navigation::FUEL_CAPACITY)
            .checked_add(refilled)
            .and_then(|value| value.checked_add(supplied))
            .and_then(|value| value.checked_sub(spent))
            .ok_or(INVALID)?;
        Ok((refilled, u16::try_from(fuel).map_err(|_| INVALID)?))
    }
    pub(super) fn refills_after_terminal(
        &self,
        dock: DockContext,
        tick: u64,
        revision: Option<u64>,
    ) -> u64 {
        self.records
            .iter()
            .filter(|record| match revision {
                Some(revision) => record.revision > revision,
                None => {
                    record.dock.sequence() > dock.sequence()
                        || (record.dock.sequence() == dock.sequence()
                            && (record.tick > tick
                                || (record.tick == tick
                                    && record.action == XyzzyAction::MaximumFuelStorage)))
                }
            })
            .map(|record| u64::from(record.fuel_added))
            .sum()
    }
    pub(super) fn validate(
        &self,
        world: &World,
        account: &crate::freight::g02::CommanderAccount,
        commerce: &super::persistence::Commerce<'_>,
    ) -> Result<Terms, SaveError> {
        if account.xyzzy_flags() != (self.active(), self.unlimited_fuel()) {
            return Err(INVALID);
        }
        let spent = world.navigation.ok_or(INVALID)?.fuel_spent();
        let mut terms = Terms::default();
        let mut previous: Option<Record> = None;
        for (i, r) in self.records.iter().enumerate() {
            super::ammunition::validate_berth_event(
                world,
                account,
                r.dock,
                r.tick,
                r.revision,
                previous.map(|p| (p.dock.sequence(), p.tick, p.end_revision)),
            )?;
            let old_mode = previous.is_some_and(|p| p.unlimited_fuel);
            let expected_mode = if i == 0 {
                true
            } else if r.action == XyzzyAction::ToggleUnlimitedFuel {
                !old_mode
            } else {
                old_mode
            };
            if (i == 0
                && !matches!(
                    r.action,
                    XyzzyAction::Enter | XyzzyAction::MaximumFuelStorage
                ))
                || r.cash_before_reset.is_some() != (i == 0)
                || r.unlimited_fuel != expected_mode
                || r.revision > r.end_revision
                || r.end_revision > account.revision()
                || r.fuel_spent > spent
                || previous.is_some_and(|p| r.fuel_spent < p.fuel_spent)
                || r.fuel_added > crate::navigation::MAX_FUEL_CAPACITY
                || (!r.action.refills(r.unlimited_fuel) && r.fuel_added != 0)
            {
                return Err(INVALID);
            }
            if let XyzzyAction::FitShield(grade) = r.action {
                commerce.modular.validate_shield_test_record(
                    r.dock.owner().seed(),
                    r.revision,
                    r.end_revision,
                    grade,
                )?;
            }
            if r.action == XyzzyAction::MaximumFuelStorage {
                if r.end_revision <= r.revision
                    || !world
                        .freight_session
                        .as_ref()
                        .ok_or(INVALID)?
                        .state
                        .custody_clear_at(r.revision, r.dock.sequence())
                    || commerce.lesson.known_active_at(r.tick, r.revision)
                    || commerce
                        .modular
                        .fuel_capacity_at_revision(r.dock.owner().seed(), r.end_revision)?
                        != crate::navigation::MAX_FUEL_CAPACITY
                {
                    return Err(INVALID);
                }
                terms.revisions = terms.revisions.checked_add(1).ok_or(INVALID)?;
            }
            if let Some(p) = previous
                && p.unlimited_fuel
            {
                terms.fuel_supplied += u128::from(r.fuel_spent - p.fuel_spent);
            }
            terms.fuel_supplied += u128::from(r.fuel_added);
            if let Some(before) = r.cash_before_reset {
                terms.cash_adjustment = i128::from(STARTING_CREDITS_TENTHS) - i128::from(before);
            }
            previous = Some(*r);
        }
        if let Some(p) = previous
            && p.unlimited_fuel
        {
            terms.fuel_supplied += u128::from(spent - p.fuel_spent);
        }
        terms.revisions = terms
            .revisions
            .checked_add(self.records.len() as u64)
            .ok_or(INVALID)?;
        if let Some(revision) = account.equipment_state().xyzzy_acquisition_revision() {
            let item = account.equipment_state().owned().ok_or(INVALID)?;
            if !self.records.iter().any(|r| {
                r.action.outfit()
                    && revision > r.revision
                    && revision <= r.end_revision
                    && super::services::service_site(r.dock.system()) == Some(item.purchase_site)
            }) {
                return Err(INVALID);
            }
        }
        self.validate_account_prefix(world, account, commerce)?;
        Ok(terms)
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.u32(self.records.len() as u32);
        for r in &self.records {
            w.u8(r.action.tag());
            r.dock.encode_save(w);
            w.u64(r.tick);
            w.u64(r.revision);
            w.u64(r.end_revision);
            w.option(r.cash_before_reset.as_ref(), |w, cash| w.u32(*cash));
            w.u16(r.fuel_added);
            w.u64(r.fuel_spent);
            w.bool(r.unlimited_fuel);
        }
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let n = r.u32()? as usize;
        if n > crate::save::MAX_SAVE_BYTES / 64 {
            return Err(SaveError::TooLarge);
        }
        let mut records = Vec::new();
        for _ in 0..n {
            records.push(Record {
                action: XyzzyAction::from_tag(r.u8()?)?,
                dock: DockContext::decode_save(r)?,
                tick: r.u64()?,
                revision: r.u64()?,
                end_revision: r.u64()?,
                cash_before_reset: r.option(|r| r.u32())?,
                fuel_added: r.u16()?,
                fuel_spent: r.u64()?,
                unlimited_fuel: r.bool()?,
            });
        }
        Ok(Self { records })
    }
}

/// Entire candidate remains unpublished until the host atomically writes these exact bytes.
pub struct PreparedXyzzy<'a> {
    world: &'a mut World,
    candidate: World,
    bytes: Vec<u8>,
}
impl PreparedXyzzy<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn snapshot(&self) -> Option<XyzzySnapshot> {
        self.candidate.xyzzy_snapshot()
    }
    pub fn publish_with<E>(self, writer: impl FnOnce(&[u8]) -> Result<(), E>) -> Result<(), E> {
        writer(&self.bytes)?;
        self.commit();
        Ok(())
    }
    pub fn commit(self) {
        *self.world = self.candidate;
    }
}
impl World {
    pub(super) fn test_tank_custody_clear(&self) -> bool {
        self.active_freight_job().is_none() && self.navigation_trade.occupancy().1 == 0
    }
    fn xyzzy_has_free_ordinary_utility_mount(&self) -> Result<bool, SaveError> {
        let fit = self.fuel_scoops_snapshot().ok_or(INVALID)?;
        Ok(fit.utility_mounts_used < fit.utility_mounts_capacity)
    }
    pub fn commander_name(&self) -> Option<&'static str> {
        self.commander_account.as_ref()?;
        Some(if self.xyzzy.active() {
            COMMANDER_NAME
        } else {
            "Rowan Calder"
        })
    }
    pub fn xyzzy_snapshot(&self) -> Option<XyzzySnapshot> {
        Some(XyzzySnapshot {
            active: self.xyzzy.active(),
            commander_name: self.commander_name()?,
            label: if self.xyzzy.active() {
                "XYZZY / COMMANDER CLARKE / SUPPORTED OUTFIT ONLY"
            } else {
                "NORMAL COMMANDER"
            },
            unlimited_fuel: self.xyzzy.unlimited_fuel(),
            cash_granted_once: self.xyzzy.active(),
            cash_tenths: self.commander_account.as_ref()?.credits_tenths(),
            jump_limit_ly: self
                .jump_reach_ly()?
                .min(f64::from(self.commander_account.as_ref()?.fuel_tenths()) / 10.0),
            full_legacy_outfit_available: false,
        })
    }
    pub fn review_xyzzy(&self, action: XyzzyAction, tick: u64) -> Result<XyzzyReview, XyzzyError> {
        if tick != self.state.tick {
            return Err(XyzzyError::StaleReview);
        }
        if self.is_paused() {
            return Err(XyzzyError::Paused);
        }
        let account = self
            .commander_account
            .as_ref()
            .ok_or(XyzzyError::NoCommander)?;
        if !matches!(action, XyzzyAction::Enter | XyzzyAction::MaximumFuelStorage)
            && !self.xyzzy.active()
        {
            return Err(XyzzyError::Inactive);
        }
        if self
            .navigation_trade_snapshot()
            .is_some_and(|s| s.active.is_some())
            && (action.changes_equipment() || !self.xyzzy.unlimited_fuel())
        {
            return Err(XyzzyError::ActiveFuelPractice);
        }
        if action == XyzzyAction::MaximumFuelStorage && !self.test_tank_custody_clear() {
            return Err(XyzzyError::ActiveContractCustody);
        }
        let dock = self
            .current_dock_context()
            .filter(|_| self.state.mode == FlightMode::Docked)
            .ok_or(XyzzyError::BerthRequired)?;
        if action == XyzzyAction::MaximumFuelStorage
            && !self
                .freight_session
                .as_ref()
                .ok_or(XyzzyError::NoCommander)?
                .state
                .custody_clear_at(
                    account
                        .revision()
                        .checked_add(1)
                        .ok_or(XyzzyError::Save(INVALID))?,
                    dock.sequence(),
                )
        {
            return Err(XyzzyError::ActiveContractCustody);
        }
        self.current_service_facts(tick)
            .map_err(|_| XyzzyError::BerthRequired)?;
        Ok(XyzzyReview {
            action,
            dock,
            tick,
            epoch: self
                .freight_session
                .as_ref()
                .ok_or(XyzzyError::NoCommander)?
                .epoch(),
            revision: account.revision(),
        })
    }
    pub fn prepare_xyzzy(
        &mut self,
        review: XyzzyReview,
        tick: u64,
    ) -> Result<PreparedXyzzy<'_>, XyzzyError> {
        if self.review_xyzzy(review.action, tick)? != review {
            return Err(XyzzyError::StaleReview);
        }
        // Existing validated reconstruction provides a private prospective World and advances its
        // session epoch, invalidating old authorities. It performs no filesystem access or grants.
        let mut candidate = World::from_save_bytes(&self.save_bytes().map_err(XyzzyError::Save)?)
            .map_err(XyzzyError::Save)?;
        candidate.apply_xyzzy(review).map_err(XyzzyError::Save)?;
        let bytes = candidate.save_bytes().map_err(XyzzyError::Save)?;
        Ok(PreparedXyzzy {
            world: self,
            candidate,
            bytes,
        })
    }
    fn apply_xyzzy(&mut self, review: XyzzyReview) -> Result<(), SaveError> {
        let first = !self.xyzzy.active();
        let mode = if first {
            true
        } else if review.action == XyzzyAction::ToggleUnlimitedFuel {
            !self.xyzzy.unlimited_fuel()
        } else {
            self.xyzzy.unlimited_fuel()
        };
        let refill = review.action.refills(mode);
        let max_tanks = review.action == XyzzyAction::MaximumFuelStorage;
        let current = self.commander_account.as_ref().ok_or(INVALID)?;
        let record = Record {
            action: review.action,
            dock: review.dock,
            tick: review.tick,
            revision: current.revision().checked_add(1).ok_or(INVALID)?,
            end_revision: u64::MAX,
            cash_before_reset: first.then_some(current.credits_tenths()),
            fuel_added: if refill {
                (if max_tanks {
                    crate::navigation::MAX_FUEL_CAPACITY
                } else {
                    current.fuel_capacity()
                })
                .checked_sub(current.fuel_tenths())
                .ok_or(INVALID)?
            } else {
                0
            },
            fuel_spent: self.navigation.ok_or(INVALID)?.fuel_spent(),
            unlimited_fuel: mode,
        };
        let account = current
            .prepare_xyzzy(first, refill && !max_tanks, mode)
            .map_err(|_| INVALID)?;
        self.xyzzy.records.push(record);
        self.commander_account = Some(account);
        if max_tanks {
            for _ in 0..crate::navigation::MAX_FUEL_STORAGE_UNITS {
                self.grant_xyzzy_module(
                    review.dock,
                    review.tick,
                    super::modular::ModuleKind::FuelStorage,
                )?;
            }
        }
        if review.action.outfit() {
            for kind in [
                super::modular::ModuleKind::EliteEnergyUnit,
                super::modular::ModuleKind::NavalLaser,
                super::modular::ModuleKind::AdvancedNavigation,
                super::modular::ModuleKind::FuelScoops,
                super::modular::ModuleKind::Targeting(crate::equipment::TargetingTier::Military),
                super::modular::ModuleKind::MiningTargeting,
                super::modular::ModuleKind::SalvageTargeting,
                super::modular::ModuleKind::Drive(crate::navigation::DriveGrade::Advanced),
                super::modular::ModuleKind::Shield(ShieldGrade::Elite),
            ] {
                if kind == super::modular::ModuleKind::FuelScoops
                    && !max_tanks
                    && self.fitted_fuel_scoops().is_none()
                    && !self.xyzzy_has_free_ordinary_utility_mount()?
                {
                    continue;
                }
                self.grant_xyzzy_module(review.dock, review.tick, kind)?;
            }
            if max_tanks
                || self.owned_emp().is_some_and(|item| item.fitted)
                || self.xyzzy_has_free_ordinary_utility_mount()?
            {
                self.grant_xyzzy_emp(review.dock, review.tick)?;
            }
            self.grant_xyzzy_missiles(review.dock, review.tick)?;
            let cargo_fitted = self
                .commander_account
                .as_ref()
                .ok_or(INVALID)?
                .equipment_state()
                .units_used()
                != 0;
            if max_tanks || cargo_fitted || self.xyzzy_has_free_ordinary_utility_mount()? {
                let prior = self.commander_account.as_ref().ok_or(INVALID)?;
                let previous_cargo_revision = prior.equipment_state().revision();
                let account = prior
                    .grant_xyzzy_cargo_bay(
                        super::services::service_site(review.dock.system()).ok_or(INVALID)?,
                    )
                    .map_err(|_| INVALID)?;
                if self.modular.has_test_cargo_history()
                    && account.equipment_state().revision() != previous_cargo_revision
                {
                    self.modular
                        .record_test_cargo_fit(&account, review.dock, review.tick)?;
                }
                self.commander_account = Some(account);
            }
            // The present supported mutation boundary is docked: ordinary docking already reset
            // energy/shields. Do not clear heat/cooldown or erase retained combat/mining evidence.
        } else if let XyzzyAction::FitShield(grade) = review.action {
            self.grant_xyzzy_module(
                review.dock,
                review.tick,
                super::modular::ModuleKind::Shield(grade),
            )?;
        }
        if max_tanks {
            let account = self
                .commander_account
                .as_ref()
                .ok_or(INVALID)?
                .prepare_xyzzy(false, true, mode)
                .map_err(|_| INVALID)?;
            self.commander_account = Some(account);
        }
        let revision = self.commander_account.as_ref().ok_or(INVALID)?.revision();
        self.xyzzy.records.last_mut().ok_or(INVALID)?.end_revision = revision;
        if max_tanks && !self.modular.has_test_cargo_history() {
            self.modular.record_test_cargo_fit(
                self.commander_account.as_ref().ok_or(INVALID)?,
                review.dock,
                review.tick,
            )?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests;

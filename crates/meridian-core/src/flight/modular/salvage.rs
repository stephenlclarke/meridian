//! Finite external recovery from terminal ordinary cargo. No live hull or crew effects.
use super::{
    CommanderAccount, Event, Ledger, ModuleAction, ModuleError, ModuleKind, ModuleSlot,
    OwnedModule, World,
};
use crate::flight::emp::SalvageEmpLink;
use crate::{
    docking::Vec3,
    equipment::WorkTargetId,
    save::{Reader, SaveError, Writer},
    station_services::{CargoIntake, CommodityId},
    traffic::{NpcId, NpcPhase, WreckManifest},
};

const INVALID: SaveError = SaveError::InvalidState("external salvage custody");
const MAX_SOURCES: usize = 2;
const MAX_UNITS: usize = 24;
const RECOVERY_TICKS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Exchange {
    source: NpcId,
    pub(super) cutter: u64,
    displaced_emp: Option<u64>,
    visit: u64,
    clock: u64,
    restoring: bool,
}

impl Exchange {
    pub(super) fn from_action(action: ModuleAction) -> Option<Self> {
        let ModuleAction::FlightSalvageExchange {
            source,
            cutter,
            displaced_emp,
            visit,
            clock,
            restoring,
        } = action
        else {
            return None;
        };
        Some(Self {
            source,
            cutter,
            displaced_emp,
            visit,
            clock,
            restoring,
        })
    }

    fn action(self) -> ModuleAction {
        ModuleAction::FlightSalvageExchange {
            source: self.source,
            cutter: self.cutter,
            displaced_emp: self.displaced_emp,
            visit: self.visit,
            clock: self.clock,
            restoring: self.restoring,
        }
    }

    fn paired_emp_revision(self, revision: u64) -> Option<u64> {
        if self.restoring {
            revision.checked_add(1)
        } else {
            revision.checked_sub(1)
        }
    }
}

/// The finite ordinary source is established independently of the player's cargo account.
/// Only its detached cargo is declared recoverable, never the source hull or its crew.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Claim {
    source: NpcId,
    terminal_tick: u64,
    commodity: CommodityId,
    quantity: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Receipt {
    source: NpcId,
    ordinal: u8,
    revision: u64,
    tick: u64,
    physical: u64,
    visit: u64,
    cutter: u64,
    scoops: u64,
    position: [u32; 3],
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct State {
    claims: Vec<Claim>,
    receipts: Vec<Receipt>,
}

fn claim_for(manifest: WreckManifest) -> Option<Claim> {
    (manifest.source.serial() == 1
        && (1..=12).contains(&manifest.quantity)
        && manifest.commodity.departure_legal_weight() == 0)
        .then_some(Claim {
            source: manifest.source,
            terminal_tick: manifest.destroyed_at_tick,
            commodity: manifest.commodity,
            quantity: manifest.quantity,
        })
}

fn collection_envelope(world: &World, source: Vec3, position: [f32; 3]) -> bool {
    let Some(station) = world.station_data.as_ref() else {
        return false;
    };
    // Detached external cargo is collected from the near, non-overlap work envelope.
    // The hull itself is not a capture volume; ordinary collision remains authoritative.
    let offset = Vec3::new(
        f64::from(position[0]) - source.x,
        f64::from(position[1]) - source.y,
        f64::from(position[2]) - source.z,
    );
    let distance = offset.dot(offset).sqrt();
    let clearance = station.player_clearance_radius() * 2.0;
    (clearance + 1.5..=clearance + 2.5).contains(&distance)
}

fn historical_claim(world: &World, id: NpcId) -> Option<Claim> {
    let manifest = world.traffic.wreck_manifest(id).ok()?;
    let actor = world
        .traffic
        .snapshot(id.system())?
        .actors
        .into_iter()
        .find(|actor| actor.id == id)?;
    if actor.phase != NpcPhase::Destroyed || actor.emp_crew_aboard.is_some() {
        return None;
    }
    claim_for(manifest)
}

impl Ledger {
    pub(super) fn apply_salvage_exchange(&mut self, event: Event) -> Result<(), ModuleError> {
        let exchange = Exchange::from_action(event.action).ok_or(ModuleError::NotForSale)?;
        if event.xyzzy || exchange.cutter != event.item {
            return Err(ModuleError::NotForSale);
        }
        let index = self
            .items
            .iter()
            .position(|item| item.id == exchange.cutter && item.kind == ModuleKind::SalvageCutter)
            .ok_or(ModuleError::MissingItem)?;
        if exchange.restoring {
            let prior = self.salvage_return.ok_or(ModuleError::NoChange)?;
            if prior.source != exchange.source
                || prior.cutter != exchange.cutter
                || prior.displaced_emp != exchange.displaced_emp
                || self.items[index].slot != Some(ModuleSlot::UtilitySalvageCutter)
            {
                return Err(ModuleError::StaleOccupant);
            }
            self.items[index].slot = None;
            self.salvage_return = None;
        } else {
            if self.salvage_return.is_some()
                || self.items[index].slot.is_some()
                || self.fitted(ModuleSlot::UtilityFuelScoops).is_none()
                || self.fitted(ModuleSlot::Targeting).is_none()
                || self.fitted(ModuleSlot::SalvageTargeting).is_none()
            {
                return Err(ModuleError::MissingPrerequisite);
            }
            self.install(index, None)?;
            self.salvage_return = Some(exchange);
        }
        Ok(())
    }
}

impl State {
    pub(super) fn is_empty(&self) -> bool {
        self.claims.is_empty() && self.receipts.is_empty()
    }

    fn claim(&self, source: NpcId) -> Option<Claim> {
        self.claims
            .iter()
            .find(|claim| claim.source == source)
            .copied()
    }

    fn recovered(&self, source: NpcId) -> usize {
        self.receipts
            .iter()
            .filter(|receipt| receipt.source == source)
            .count()
    }

    fn available(&self, claim: Claim, physical: u64) -> bool {
        self.recovered(claim.source) < usize::from(claim.quantity)
            && self.receipts.last().is_none_or(|r| {
                r.physical
                    .checked_add(RECOVERY_TICKS)
                    .is_some_and(|next| physical >= next)
            })
    }

    pub(super) fn encode(&self, w: &mut Writer) {
        w.u8(self.claims.len() as u8);
        for claim in &self.claims {
            claim.source.encode(w);
            w.u64(claim.terminal_tick);
            w.u8(claim.commodity.index() as u8);
            w.u8(claim.quantity);
        }
        w.u8(self.receipts.len() as u8);
        for r in &self.receipts {
            r.source.encode(w);
            w.u8(r.ordinal);
            for value in [r.revision, r.tick, r.physical, r.visit, r.cutter, r.scoops] {
                w.u64(value);
            }
            for coordinate in r.position {
                w.u32(coordinate);
            }
        }
    }

    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let mut state = Self::default();
        let count = usize::from(r.u8()?);
        if count == 0 || count > MAX_SOURCES {
            return Err(INVALID);
        }
        for _ in 0..count {
            let source = NpcId::decode(r)?;
            let terminal_tick = r.u64()?;
            let commodity = CommodityId::from_index(usize::from(r.u8()?)).ok_or(INVALID)?;
            let quantity = r.u8()?;
            if state.claim(source).is_some() || !(1..=12).contains(&quantity) {
                return Err(INVALID);
            }
            state.claims.push(Claim {
                source,
                terminal_tick,
                commodity,
                quantity,
            });
        }
        let count = usize::from(r.u8()?);
        if count == 0 || count > MAX_UNITS {
            return Err(INVALID);
        }
        for _ in 0..count {
            state.receipts.push(Receipt {
                source: NpcId::decode(r)?,
                ordinal: r.u8()?,
                revision: r.u64()?,
                tick: r.u64()?,
                physical: r.u64()?,
                visit: r.u64()?,
                cutter: r.u64()?,
                scoops: r.u64()?,
                position: [r.u32()?, r.u32()?, r.u32()?],
            });
        }
        Ok(state)
    }

    fn intakes(&self) -> impl Iterator<Item = CargoIntake> + '_ {
        self.receipts.iter().filter_map(|r| {
            self.claim(r.source).map(|claim| CargoIntake {
                revision: r.revision,
                commodity: claim.commodity,
            })
        })
    }
}

impl super::State {
    pub(in crate::flight) fn salvage_exchange_pending(&self) -> bool {
        self.salvage_exchange_source().is_some()
    }

    pub(in crate::flight) fn salvage_exchange_source(&self) -> Option<NpcId> {
        self.events
            .iter()
            .rev()
            .find_map(|event| Exchange::from_action(event.action))
            .filter(|exchange| !exchange.restoring)
            .map(|exchange| exchange.source)
    }

    pub(in crate::flight) fn matches_salvage_emp_event(&self, link: SalvageEmpLink) -> bool {
        let SalvageEmpLink {
            item,
            cutter,
            revision,
            tick,
            visit,
            clock,
            restoring,
        } = link;
        self.events.iter().any(|event| {
            let Some(exchange) = Exchange::from_action(event.action) else {
                return false;
            };
            !event.xyzzy
                && event.tick == tick
                && exchange.cutter == cutter
                && exchange.displaced_emp == Some(item)
                && exchange.visit == visit
                && exchange.clock == clock
                && exchange.restoring == restoring
                && exchange.paired_emp_revision(event.revision) == Some(revision)
        })
    }

    pub(super) fn validate_salvage_exchange(
        &self,
        world: &World,
        account: &CommanderAccount,
        event: Event,
        previous: Option<Event>,
        exchange: Exchange,
    ) -> Result<(), SaveError> {
        let identity = world.combat_identity().ok_or(INVALID)?;
        if previous.is_none_or(|prior| prior.dock != event.dock)
            || exchange.source.profile() != identity.owner
            || exchange.visit <= event.dock.visit()
            || exchange.visit > identity.visit
            || exchange.clock == 0
            || exchange.clock > world.combat.physical_tick()
            || exchange.clock > event.tick
            || event.tick - exchange.clock
                > world
                    .state
                    .tick
                    .saturating_sub(world.combat.physical_tick())
            || world
                .jump_journal
                .system_at_visit(exchange.visit, event.tick)
                .is_err()
            || (!exchange.restoring
                && (world
                    .jump_journal
                    .system_at_visit(exchange.visit, event.tick)?
                    != exchange.source.system()
                    || historical_claim(world, exchange.source)
                        .is_none_or(|claim| claim.terminal_tick >= event.tick)))
        {
            return Err(INVALID);
        }
        if let Some(item) = exchange.displaced_emp {
            let revision = exchange
                .paired_emp_revision(event.revision)
                .ok_or(INVALID)?;
            if revision > account.revision()
                || !world.emp.matches_salvage_handover(SalvageEmpLink {
                    item,
                    cutter: exchange.cutter,
                    revision,
                    tick: event.tick,
                    visit: exchange.visit,
                    clock: exchange.clock,
                    restoring: exchange.restoring,
                })
            {
                return Err(INVALID);
            }
        }
        Ok(())
    }

    pub(in crate::flight) fn validate_salvage(
        &self,
        world: &World,
        account: &CommanderAccount,
    ) -> Result<(), SaveError> {
        let owner = world.combat_identity().ok_or(INVALID)?.owner;
        let state = &self.salvage;
        if state.claims.len() > MAX_SOURCES || state.receipts.len() > MAX_UNITS {
            return Err(INVALID);
        }
        for claim in &state.claims {
            let manifest = world
                .traffic
                .wreck_manifest(claim.source)
                .map_err(|_| INVALID)?;
            let actor = world
                .traffic
                .snapshot(claim.source.system())
                .ok_or(INVALID)?
                .actors
                .into_iter()
                .find(|actor| actor.id == claim.source)
                .ok_or(INVALID)?;
            let recovered = state.recovered(claim.source);
            if claim.source.profile() != owner
                || claim_for(manifest) != Some(*claim)
                || actor.phase != NpcPhase::Destroyed
                || actor.emp_crew_aboard.is_some()
                || recovered == 0
                || recovered > usize::from(claim.quantity)
            {
                return Err(INVALID);
            }
        }
        let mut previous: Option<&Receipt> = None;
        for (index, r) in state.receipts.iter().enumerate() {
            let claim = state.claim(r.source).ok_or(INVALID)?;
            let manifest = world
                .traffic
                .wreck_manifest(r.source)
                .map_err(|_| INVALID)?;
            let ordinal = state.receipts[..index]
                .iter()
                .filter(|prior| prior.source == r.source)
                .count();
            if usize::from(r.ordinal) != ordinal
                || r.revision == 0
                || r.revision > account.revision()
                || r.tick <= claim.terminal_tick
                || r.tick > world.state.tick
                || r.physical == 0
                || r.physical > world.combat.physical_tick()
                || r.tick < r.physical
                || r.tick - r.physical
                    > world
                        .state
                        .tick
                        .saturating_sub(world.combat.physical_tick())
                || r.visit > world.combat_identity().ok_or(INVALID)?.visit
                || world.jump_journal.system_at_visit(r.visit, r.tick)? != r.source.system()
                || !collection_envelope(
                    world,
                    manifest.pose.position,
                    r.position.map(f32::from_bits),
                )
                || previous.is_some_and(|prior| {
                    r.revision <= prior.revision
                        || r.physical
                            .checked_sub(prior.physical)
                            .is_none_or(|elapsed| elapsed < RECOVERY_TICKS)
                        || r.tick - r.physical < prior.tick - prior.physical
                })
            {
                return Err(INVALID);
            }
            let fit = self.ledger_at_revision(owner, r.revision - 1)?;
            if fit
                .fitted(ModuleSlot::UtilitySalvageCutter)
                .is_none_or(|item| item.id != r.cutter)
                || fit
                    .fitted(ModuleSlot::UtilityFuelScoops)
                    .is_none_or(|item| item.id != r.scoops)
                || fit.fitted(ModuleSlot::Targeting).is_none()
                || fit.fitted(ModuleSlot::SalvageTargeting).is_none()
            {
                return Err(INVALID);
            }
            previous = Some(r);
        }
        Ok(())
    }

    pub(in crate::flight) fn salvage_intakes(&self) -> impl Iterator<Item = CargoIntake> + '_ {
        self.salvage.intakes()
    }
}

impl World {
    pub fn fitted_salvage_cutter(&self) -> Option<OwnedModule> {
        let account = self.commander_account.as_ref()?;
        self.modular
            .ledger(self, account)
            .ok()?
            .fitted(ModuleSlot::UtilitySalvageCutter)
            .filter(|item| item.kind == ModuleKind::SalvageCutter)
    }

    pub fn auto_salvage_eligible(&self, id: NpcId) -> bool {
        self.external_claim(id)
            .is_some_and(|claim| self.modular.salvage.recovered(id) < usize::from(claim.quantity))
    }

    fn external_claim(&self, id: NpcId) -> Option<Claim> {
        self.wreck_manifest(id).ok()?;
        historical_claim(self, id)
    }

    /// Select an owned cutter solely for a selected, independently admitted terminal cargo source.
    pub(in crate::flight) fn select_auto_salvage_tool(
        &mut self,
        id: NpcId,
    ) -> Result<(), ModuleError> {
        if self.state.mode != crate::FlightMode::Flight
            || self.is_paused()
            || self.emp.input_blocked()
            || self.combat_snapshot().is_none_or(|s| s.player.destroyed)
            || !self.auto_salvage_eligible(id)
            || self
                .selected_work_target()
                .is_none_or(|target| target.id != WorkTargetId::Wreck(id))
        {
            return Err(ModuleError::MissingPrerequisite);
        }
        let account = self
            .commander_account
            .as_ref()
            .ok_or(ModuleError::NoCommander)?;
        let ledger = self
            .modular
            .ledger(self, account)
            .map_err(ModuleError::Save)?;
        if let Some(active) = ledger.salvage_return {
            return if active.source == id {
                Ok(())
            } else {
                Err(ModuleError::StaleOccupant)
            };
        }
        if ledger.fitted(ModuleSlot::UtilitySalvageCutter).is_some() {
            return Ok(());
        }
        let item = ledger
            .items
            .iter()
            .find(|item| item.kind == ModuleKind::SalvageCutter && item.slot.is_none())
            .copied()
            .ok_or(ModuleError::MissingItem)?;
        let definition = crate::equipment::definition().map_err(|_| ModuleError::Capacity)?;
        let (units, mounts) = ledger.budget_usage(account, self.emp.utility_mounts())?;
        if units >= definition.equipment_units_capacity
            || mounts > definition.utility_mounts_capacity
        {
            return Err(ModuleError::Capacity);
        }
        let displaced_emp = if mounts == definition.utility_mounts_capacity {
            let emp = self
                .emp
                .owned()
                .map_err(ModuleError::Save)?
                .filter(|emp| emp.fitted)
                .ok_or(ModuleError::Capacity)?;
            if self.emp.travel_blocked(&self.traffic) {
                return Err(ModuleError::Capacity);
            }
            Some(emp.item)
        } else {
            None
        };
        let identity = self.combat_identity().ok_or(ModuleError::NoCommander)?;
        self.exchange_salvage(
            ledger,
            Exchange {
                source: id,
                cutter: item.id,
                displaced_emp,
                visit: identity.visit,
                clock: self.combat.physical_tick(),
                restoring: false,
            },
        )
    }

    /// Release only the temporary fit, returning the exact displacement to its original slot.
    pub(in crate::flight) fn restore_auto_salvage_tool(&mut self) -> Result<(), ModuleError> {
        if !self.modular.salvage_exchange_pending() {
            return Ok(());
        }
        if self.is_paused() || self.emp.input_blocked() {
            return Err(ModuleError::Paused);
        }
        if self.combat_snapshot().is_none_or(|s| s.player.destroyed) {
            return Err(ModuleError::MissingPrerequisite);
        }
        let account = self
            .commander_account
            .as_ref()
            .ok_or(ModuleError::NoCommander)?;
        let ledger = self
            .modular
            .ledger(self, account)
            .map_err(ModuleError::Save)?;
        let prior = ledger.salvage_return.ok_or(ModuleError::StaleOccupant)?;
        let identity = self.combat_identity().ok_or(ModuleError::NoCommander)?;
        self.exchange_salvage(
            ledger,
            Exchange {
                visit: identity.visit,
                clock: self.combat.physical_tick(),
                restoring: true,
                ..prior
            },
        )
    }

    fn exchange_salvage(
        &mut self,
        mut ledger: Ledger,
        exchange: Exchange,
    ) -> Result<(), ModuleError> {
        let prior = self
            .modular
            .events
            .last()
            .ok_or(ModuleError::StaleOccupant)?;
        let dock = prior.dock;
        let tick = self.state.tick;
        let current = self
            .commander_account
            .as_ref()
            .ok_or(ModuleError::NoCommander)?;
        let emp_first = exchange.displaced_emp.is_some() && !exchange.restoring;
        let before_module = if emp_first {
            Some(
                current
                    .prepare_emp_equipment(0)
                    .map_err(|_| ModuleError::Capacity)?,
            )
        } else {
            None
        };
        let before_module = before_module.as_ref().unwrap_or(current);
        let revision = before_module
            .revision()
            .checked_add(1)
            .ok_or(ModuleError::SequenceExhausted)?;
        let event = Event {
            xyzzy: false,
            action: exchange.action(),
            item: exchange.cutter,
            dock,
            tick,
            revision,
        };
        ledger.apply(event, 0)?;
        let owner = self
            .combat_identity()
            .ok_or(ModuleError::NoCommander)?
            .owner;
        let born = |slot, offset| {
            ledger
                .fitted(slot)
                .is_some_and(|item| item.id == owner + offset)
        };
        let mut account = before_module
            .prepare_module(
                0,
                [
                    born(ModuleSlot::FrontLaser, 7),
                    born(ModuleSlot::Energy, 5),
                    born(ModuleSlot::Navigation, 4),
                    born(ModuleSlot::Drive, 6),
                    born(ModuleSlot::Shield, 3),
                ],
                ledger.fuel_storage_count(),
            )
            .map_err(|_| ModuleError::Capacity)?;
        if exchange.displaced_emp.is_some() && exchange.restoring {
            account = account
                .prepare_emp_equipment(0)
                .map_err(|_| ModuleError::Capacity)?;
        }
        let candidate_emp = if let Some(item) = exchange.displaced_emp {
            self.emp
                .prepare_salvage_handover(SalvageEmpLink {
                    item,
                    cutter: exchange.cutter,
                    revision: exchange
                        .paired_emp_revision(revision)
                        .ok_or(ModuleError::SequenceExhausted)?,
                    tick,
                    visit: exchange.visit,
                    clock: exchange.clock,
                    restoring: exchange.restoring,
                })
                .map_err(ModuleError::Save)?
        } else {
            self.emp.clone()
        };
        ledger.validate_budgets(&account, candidate_emp.utility_mounts(), 0)?;
        let mut modules = self.modular.clone();
        modules.events.push(event);

        // The full validation joins EMP and modular histories in both directions. Both candidates
        // must be visible to that read-only validation, and neither can escape on failure.
        let old_account = self.commander_account.replace(account);
        let old_emp = core::mem::replace(&mut self.emp, candidate_emp);
        let old_modules = core::mem::replace(&mut self.modular, modules);
        if let Err(error) = self.save_bytes() {
            self.commander_account = old_account;
            self.emp = old_emp;
            self.modular = old_modules;
            return Err(ModuleError::Save(error));
        }
        Ok(())
    }

    /// Called only at an accepted selected-target work tick. This transfer cannot damage
    /// a vessel, create a new source manifest or take intact cargo from a disabled actor.
    pub(in crate::flight) fn auto_salvage_tick(&mut self, id: NpcId) -> &'static str {
        if self.state.mode != crate::FlightMode::Flight
            || self.is_paused()
            || self.emp.electrical_disabled()
            || self.combat_snapshot().is_none_or(|s| s.player.destroyed)
            || self
                .selected_work_target()
                .is_none_or(|target| target.id != WorkTargetId::Wreck(id))
        {
            return "SALVAGE unavailable";
        }
        let Some(claim) = self.external_claim(id) else {
            return "Claim unavailable";
        };
        let Some(cutter) = self.fitted_salvage_cutter() else {
            return "Cutter not fitted";
        };
        let Some(scoops) = self.fitted_fuel_scoops() else {
            return "Scoops not fitted";
        };
        if !self.work_targeting_fit().salvage {
            return "Salvage module required";
        }
        let Ok(manifest) = self.wreck_manifest(id) else {
            return "Claim unavailable";
        };
        if self.state.speed.abs() > 0.01
            || !collection_envelope(self, manifest.pose.position, self.state.position)
        {
            return "Hold for recovery";
        }
        let tick = self.state.tick;
        if self.modular.salvage.recovered(id) >= usize::from(claim.quantity) {
            return "Salvage complete";
        }
        let physical = self.combat.physical_tick();
        if tick <= claim.terminal_tick || !self.modular.salvage.available(claim, physical) {
            return "Recovering salvage";
        }
        let Some(account) = self.commander_account.as_ref() else {
            return "Salvage unavailable";
        };
        let Some(session) = self.freight_session.as_ref() else {
            return "Salvage unavailable";
        };
        let Some(identity) = self.combat_identity() else {
            return "Salvage unavailable";
        };
        let Ok(cabins) = self.cabin_tonnes() else {
            return "Hold unavailable";
        };
        let mut reservations = self.navigation_trade.reservations();
        let Some(capacity) = reservations.capacity.checked_add(cabins) else {
            return "Hold unavailable";
        };
        reservations.capacity = capacity;
        let Ok(candidate) =
            account.prepare_cargo_intake(&session.state, claim.commodity, reservations)
        else {
            return "Hold full or reserved";
        };
        let mut modular = self.modular.clone();
        let salvage = &mut modular.salvage;
        if salvage.receipts.len() >= MAX_UNITS {
            return "Salvage history full";
        }
        if salvage.claim(id).is_none() {
            if salvage.claims.len() >= MAX_SOURCES {
                return "Salvage history full";
            }
            salvage.claims.push(claim);
        }
        salvage.receipts.push(Receipt {
            source: id,
            ordinal: salvage.recovered(id) as u8,
            revision: candidate.revision(),
            tick,
            physical,
            visit: identity.visit,
            cutter: cutter.id,
            scoops: scoops.item,
            position: self.state.position.map(f32::to_bits),
        });
        if self.save_bytes_with_modules(&candidate, &modular).is_err() {
            return "Salvage save refused";
        }
        self.modular = modular;
        self.commander_account = Some(candidate);
        if !self.auto_salvage_eligible(id) {
            self.cancel_targeting();
            if self.restore_auto_salvage_tool().is_err() {
                return "Cutter return unavailable";
            }
            return "Salvage complete";
        }
        "Salvage recovered"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{ModuleAction, ModuleError, ModuleProvenance, XyzzyAction};

    fn apply(world: &mut World, action: ModuleAction) -> u64 {
        let tick = world.snapshot().tick;
        let review = world.review_module(action, tick).unwrap();
        world.prepare_module(review, tick).unwrap().commit()
    }

    fn source_id(owner: u64) -> NpcId {
        let mut writer = Writer::new();
        writer.u64(owner);
        writer.u8(1);
        writer.u16(7);
        writer.u8(1);
        NpcId::decode(&mut Reader::new(&writer.bytes)).unwrap()
    }

    fn exchange_fixture(world: &World, cutter: u64, displaced_emp: Option<u64>) -> (Ledger, Event) {
        let account = world.commander_account.as_ref().unwrap();
        let ledger = world.modular.ledger(world, account).unwrap();
        let source = source_id(world.commander_id().unwrap().seed());
        let exchange = Exchange {
            source,
            cutter,
            displaced_emp,
            visit: 2,
            clock: 1,
            restoring: false,
        };
        let event = Event {
            xyzzy: false,
            action: exchange.action(),
            item: cutter,
            dock: world.current_dock_context().unwrap(),
            tick: world.state.tick,
            revision: account.revision() + if displaced_emp.is_some() { 2 } else { 1 },
        };
        (ledger, event)
    }

    #[test]
    fn real_maximum_owned_items_form_a_reversible_joint_candidate_without_changing_live_fit() {
        // The owned property and balance are real MAX state. This exercises the isolated
        // paired ledger transitions, not a fabricated accepted external source or live lock.
        let mut world = World::new_commander(31906).unwrap();
        let review = world.review_xyzzy(XyzzyAction::Enter, 0).unwrap();
        world.prepare_xyzzy(review, 0).unwrap().commit();
        let modules = world.modular_equipment_snapshot().unwrap();
        let cutter = modules
            .items
            .iter()
            .find(|item| item.kind == ModuleKind::SalvageCutter)
            .unwrap()
            .id;
        let emp = world.owned_emp().unwrap();
        let scoop = world.fitted_fuel_scoops().unwrap().item;
        let bay = world.equipment_snapshot().unwrap().owned;
        let before = world.save_bytes().unwrap();
        let (mut ledger, select) = exchange_fixture(&world, cutter, Some(emp.item));
        let opening = ledger.fitted(ModuleSlot::UtilitySalvageCutter);
        assert!(opening.is_none());
        let exchange = Exchange::from_action(select.action).unwrap();
        let link = SalvageEmpLink {
            item: emp.item,
            cutter,
            revision: select.revision - 1,
            tick: select.tick,
            visit: exchange.visit,
            clock: exchange.clock,
            restoring: false,
        };
        let suspended = world.emp.prepare_salvage_handover(link).unwrap();
        assert_eq!(
            suspended.owned().unwrap(),
            Some(crate::flight::emp::OwnedEmp {
                fitted: false,
                ..emp
            })
        );
        assert!(suspended.matches_salvage_handover(link));
        ledger.apply(select, 0).unwrap();
        assert_eq!(
            ledger.fitted(ModuleSlot::UtilitySalvageCutter).unwrap().id,
            cutter
        );
        assert_eq!(
            ledger.fitted(ModuleSlot::UtilityFuelScoops).unwrap().id,
            scoop
        );
        assert_eq!(
            ledger
                .budget_usage(
                    world.commander_account.as_ref().unwrap(),
                    suspended.utility_mounts()
                )
                .unwrap()
                .1,
            3
        );
        assert!(ledger.apply(select, 0).is_err());
        let restore = Event {
            action: Exchange {
                restoring: true,
                ..exchange
            }
            .action(),
            revision: select.revision + 1,
            ..select
        };
        ledger.apply(restore, 0).unwrap();
        let restored = suspended
            .prepare_salvage_handover(SalvageEmpLink {
                revision: restore.revision + 1,
                restoring: true,
                ..link
            })
            .unwrap();
        assert_eq!(restored.owned().unwrap(), Some(emp));
        assert!(ledger.fitted(ModuleSlot::UtilitySalvageCutter).is_none());
        assert_eq!(
            ledger.fitted(ModuleSlot::UtilityFuelScoops).unwrap().id,
            scoop
        );
        assert_eq!(world.equipment_snapshot().unwrap().owned, bay);
        assert_eq!(world.save_bytes().unwrap(), before);
        assert!(world.restore_auto_salvage_tool().is_ok());
        assert!(world.select_auto_salvage_tool(source_id(31906)).is_err());
        assert_eq!(world.save_bytes().unwrap(), before);
    }

    #[test]
    fn paid_stored_cutter_free_mount_candidate_keeps_actual_scoop_and_targeting() {
        let mut world = crate::flight::equipment::tests::earned_world(31907);
        let cutter = apply(
            &mut world,
            ModuleAction::Purchase {
                kind: ModuleKind::SalvageCutter,
            },
        );
        apply(&mut world, ModuleAction::Store { item: cutter });
        let tick = world.state.tick;
        let review = world.review_xyzzy(XyzzyAction::Enter, tick).unwrap();
        world.prepare_xyzzy(review, tick).unwrap().commit();
        let owned = world
            .modular_equipment_snapshot()
            .unwrap()
            .items
            .into_iter()
            .find(|item| item.kind == ModuleKind::SalvageCutter)
            .unwrap();
        assert_eq!(owned.id, cutter);
        assert!(matches!(owned.provenance, ModuleProvenance::Paid { .. }));
        assert!(owned.slot.is_none());
        // The actual paid item has not been re-granted; free a real non-scoop mount at berth.
        let emp = world.owned_emp().unwrap();
        let tick = world.state.tick;
        let review = world
            .review_emp_equipment(crate::EmpEquipmentAction::Store { item: emp.item }, tick)
            .unwrap();
        world.prepare_emp_equipment(review, tick).unwrap().commit();
        let (mut ledger, select) = exchange_fixture(&world, cutter, None);
        ledger.apply(select, 0).unwrap();
        assert_eq!(
            ledger.fitted(ModuleSlot::UtilitySalvageCutter).unwrap().id,
            cutter
        );
        assert!(ledger.fitted(ModuleSlot::UtilityFuelScoops).is_some());
        assert_eq!(
            ledger
                .budget_usage(world.commander_account.as_ref().unwrap(), 0)
                .unwrap()
                .1,
            3
        );
        let exchange = Exchange::from_action(select.action).unwrap();
        ledger
            .apply(
                Event {
                    action: Exchange {
                        restoring: true,
                        ..exchange
                    }
                    .action(),
                    revision: select.revision + 1,
                    ..select
                },
                0,
            )
            .unwrap();
        assert!(ledger.fitted(ModuleSlot::UtilitySalvageCutter).is_none());
        assert_eq!(
            ledger
                .budget_usage(world.commander_account.as_ref().unwrap(), 0)
                .unwrap()
                .1,
            2
        );
    }

    #[test]
    fn ordinary_salvage_cutter_is_unique_paid_separate_and_persistent() {
        let mut world = crate::flight::equipment::tests::earned_world(31902);
        let action = ModuleAction::Purchase {
            kind: ModuleKind::SalvageCutter,
        };
        let cash = world.credits_tenths().unwrap();
        let front = world.pulse_laser_snapshot();
        let offer = world
            .module_offers()
            .into_iter()
            .find(|offer| offer.kind == ModuleKind::SalvageCutter)
            .unwrap();
        assert_eq!(
            (
                offer.price_tenths,
                offer.required_technology,
                offer.equipment_units
            ),
            (25_000, 7, 1)
        );
        let id = apply(&mut world, action);
        let item = world.fitted_salvage_cutter().unwrap();
        assert_eq!(item.id, id);
        assert!(matches!(
            item.provenance,
            ModuleProvenance::Paid {
                paid_tenths: 25_000,
                ..
            }
        ));
        assert_eq!(world.credits_tenths(), Some(cash - 25_000));
        assert_eq!(world.pulse_laser_snapshot(), front);
        assert_eq!(
            world
                .modular_equipment_snapshot()
                .unwrap()
                .utility_mounts_used,
            1
        );
        assert_eq!(
            world.review_module(action, world.snapshot().tick),
            Err(ModuleError::Capacity)
        );
        apply(&mut world, ModuleAction::Store { item: id });
        assert!(world.fitted_salvage_cutter().is_none());
        apply(
            &mut world,
            ModuleAction::Install {
                item: id,
                replacing: None,
            },
        );
        let cold = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
        assert_eq!(cold.fitted_salvage_cutter(), Some(item));
        assert_eq!(cold.pulse_laser_snapshot(), front);
    }

    #[test]
    fn compatible_maximum_stores_cutter_without_displacing_existing_utility() {
        let mut world = World::new_commander(31903).unwrap();
        assert_eq!(
            world.review_module(ModuleAction::GrantStoredSalvageCutter, 0),
            Err(ModuleError::NotForSale)
        );
        let review = world.review_xyzzy(XyzzyAction::Enter, 0).unwrap();
        world.prepare_xyzzy(review, 0).unwrap().commit();
        let modules = world.modular_equipment_snapshot().unwrap();
        assert_eq!(modules.utility_mounts_used, modules.utility_mounts_capacity);
        let owned = modules
            .items
            .into_iter()
            .find(|item| item.kind == ModuleKind::SalvageCutter)
            .unwrap();
        assert!(owned.slot.is_none() && matches!(owned.provenance, ModuleProvenance::XYZzy { .. }));
        assert!(world.fitted_fuel_scoops().is_some());
        assert!(world.equipment_snapshot().unwrap().owned.unwrap().fitted);
        let mut cold = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
        let scoop = cold.fitted_fuel_scoops().unwrap().item;
        apply(&mut cold, ModuleAction::Store { item: scoop });
        apply(
            &mut cold,
            ModuleAction::Install {
                item: owned.id,
                replacing: None,
            },
        );
        assert_eq!(cold.fitted_salvage_cutter().unwrap().id, owned.id);
        assert_eq!(
            cold.modular_equipment_snapshot()
                .unwrap()
                .utility_mounts_used,
            3
        );
    }

    #[test]
    fn empty_salvage_extension_stays_absent_and_live_cargo_is_not_eligible() {
        let mut world = World::new_commander(31904).unwrap();
        assert!(world.modular.salvage.is_empty());
        let baseline = world.save_bytes().unwrap();
        let mut writer = Writer::new();
        writer.u64(31904);
        writer.u8(1);
        writer.u16(7);
        writer.u8(1);
        let source = NpcId::decode(&mut Reader::new(&writer.bytes)).unwrap();
        assert!(!world.auto_salvage_eligible(source));
        assert_eq!(world.auto_salvage_tick(source), "SALVAGE unavailable");
        assert_eq!(world.save_bytes().unwrap(), baseline);
    }

    #[test]
    fn bounded_receipt_codec_retains_exact_source_ordinal_and_revision() {
        let mut writer = Writer::new();
        writer.u64(31904);
        writer.u8(1);
        writer.u16(7);
        writer.u8(1);
        let source = NpcId::decode(&mut Reader::new(&writer.bytes)).unwrap();
        let claim = Claim {
            source,
            terminal_tick: 10,
            commodity: CommodityId::Alloys,
            quantity: 2,
        };
        let mut state = State {
            claims: vec![claim],
            receipts: vec![Receipt {
                source,
                ordinal: 0,
                revision: 20,
                tick: 80,
                physical: 70,
                visit: 1,
                cutter: 1,
                scoops: 2,
                position: [0; 3],
            }],
        };
        assert_eq!(state.recovered(source), 1);
        assert!(!state.available(claim, 129));
        assert!(state.available(claim, 130));
        let mut second = state.receipts[0];
        second.ordinal = 1;
        second.revision += 1;
        second.tick += RECOVERY_TICKS;
        second.physical += RECOVERY_TICKS;
        state.receipts.push(second);
        assert!(!state.available(claim, 1000));
        let mut writer = Writer::new();
        state.encode(&mut writer);
        let mut reader = Reader::new(&writer.bytes);
        let cold = State::decode(&mut reader).unwrap();
        assert_eq!(cold, state);
        assert!(reader.finished());
        assert_eq!(
            cold.intakes().collect::<Vec<_>>(),
            vec![
                CargoIntake {
                    revision: 20,
                    commodity: CommodityId::Alloys
                },
                CargoIntake {
                    revision: 21,
                    commodity: CommodityId::Alloys
                }
            ]
        );
        writer.bytes.pop();
        assert!(State::decode(&mut Reader::new(&writer.bytes)).is_err());
    }
}

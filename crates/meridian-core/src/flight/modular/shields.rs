//! The stock shield fit is owned by the existing module ledger, not by the HUD or bank codec.
use super::{INVALID, Ledger, ModuleAction, ModuleKind, ModuleSlot, OwnedModule, State, World};
use crate::flight::{ShieldGrade, combat};
use crate::freight::g02::CommanderAccount;
use crate::save::SaveError;

pub(in crate::flight) enum ResourceFitChange {
    Energy(u16),
    Shield(ShieldGrade),
}

impl Ledger {
    pub(in crate::flight) fn shield_grade(&self) -> Option<ShieldGrade> {
        match self.fitted(ModuleSlot::Shield)?.kind {
            ModuleKind::Shield(grade) => Some(grade),
            _ => None,
        }
    }
}

impl State {
    pub(in crate::flight) fn resource_fit_changes(
        &self,
        world: &World,
    ) -> Result<Vec<(u64, ResourceFitChange)>, SaveError> {
        let owner = world.combat_identity().ok_or(INVALID)?.owner;
        let mut ledger = self.born_ledger(owner)?;
        let mut changes = Vec::new();
        let mut counters = [0; 2];
        let mut grade = ShieldGrade::Basic;
        let mut rate = 7;
        for event in &self.events {
            ledger.replay(*event, &mut counters).map_err(|_| INVALID)?;
            let next = ledger.shield_grade().ok_or(INVALID)?;
            let next_rate = ledger
                .fitted(ModuleSlot::Energy)
                .and_then(|item| item.kind.energy_recharge_multiplier())
                .unwrap_or(1)
                * 7;
            if next == grade && next_rate == rate {
                continue;
            }
            let clock = world
                .damage
                .berth_clock(event.dock.sequence())
                .ok_or(INVALID)?;
            if next != grade {
                changes.push((clock, ResourceFitChange::Shield(next)));
            }
            if next_rate != rate {
                changes.push((clock, ResourceFitChange::Energy(next_rate)));
            }
            grade = next;
            rate = next_rate;
        }
        Ok(changes)
    }

    fn shield_at_revision(&self, owner: u64, revision: u64) -> Result<OwnedModule, SaveError> {
        let mut ledger = self.born_ledger(owner)?;
        let mut counters = [0; 2];
        for event in self
            .events
            .iter()
            .take_while(|event| event.revision <= revision)
        {
            ledger.replay(*event, &mut counters).map_err(|_| INVALID)?;
        }
        ledger.fitted(ModuleSlot::Shield).ok_or(INVALID)
    }

    /// One explicit shield test record can change at most one module, and never issue a paid
    /// purchase. Reuse of an existing item is a real fit; an already fitted grade is a no-op.
    pub(in crate::flight) fn validate_shield_test_record(
        &self,
        owner: u64,
        start: u64,
        end: u64,
        grade: ShieldGrade,
    ) -> Result<(), SaveError> {
        let before = self.shield_at_revision(owner, start)?;
        let after = self.shield_at_revision(owner, end)?;
        if after.kind != ModuleKind::Shield(grade) {
            return Err(INVALID);
        }
        if start == end {
            return if before == after {
                Ok(())
            } else {
                Err(INVALID)
            };
        }
        let event = self
            .events
            .iter()
            .find(|event| event.revision == end)
            .ok_or(INVALID)?;
        if start.checked_add(1) != Some(end) || before.id == after.id || event.item != after.id {
            return Err(INVALID);
        }
        match event.action {
            ModuleAction::Purchase {
                kind: ModuleKind::Shield(next),
            } if event.xyzzy && next == grade => Ok(()),
            ModuleAction::Install {
                item,
                replacing: Some(previous),
            } if !event.xyzzy && item == after.id && previous == before.id => Ok(()),
            _ => Err(INVALID),
        }
    }
}

impl World {
    /// Stage the exact candidate account, fitted ledger and scaled bank values together. The
    /// existing save validators can then replay the actual candidate history. Restore the live
    /// World before returning on both success and refusal; only PreparedModule can publish it.
    pub(super) fn shield_fit_save(
        &mut self,
        account: CommanderAccount,
        state: State,
        combat: combat::State,
    ) -> Result<(CommanderAccount, State, Vec<u8>), SaveError> {
        let opening_account = self.commander_account.replace(account);
        let opening_state = std::mem::replace(&mut self.modular, state);
        let opening_combat = std::mem::replace(&mut self.combat, combat);
        let encoded = self.save_bytes();
        let account =
            std::mem::replace(&mut self.commander_account, opening_account).ok_or(INVALID);
        let state = std::mem::replace(&mut self.modular, opening_state);
        self.combat = opening_combat;
        Ok((account?, state, encoded?))
    }
}

//! World-bound paid equipment commands; immutable snapshots grant no ownership or fit.
use super::{MissionError, World};
use crate::equipment::{self, Command, Event, Snapshot};
impl World {
    pub fn equipment_snapshot(&self) -> Option<Snapshot> {
        let account = self.commander_account.as_ref()?;
        let session = self.freight_session.as_ref()?;
        let projection = account.service_projection(&session.state).ok()?;
        let definition = equipment::definition().ok()?;
        let site = self
            .current_dock_context()
            .and_then(|dock| super::services::service_site(dock.system()));
        Some(Snapshot {
            definition,
            owned: account.equipment_state().owned(),
            hold_capacity_tonnes: account.hold_capacity(),
            used_tonnes: projection.used_tonnes,
            equipment_units_used: account.equipment_state().units_used(),
            utility_mounts_used: account.equipment_state().units_used(),
            current_offer: site.map(|site| definition.offer(site)),
            service_available: site.is_some() && !self.is_paused(),
        })
    }
    pub fn request_equipment(
        &mut self,
        command: Command,
        expected_tick: u64,
    ) -> Result<Event, MissionError> {
        let facts = self.current_service_facts(expected_tick)?;
        let account = self
            .commander_account
            .as_mut()
            .ok_or(MissionError::NoCommander)?;
        let session = self
            .freight_session
            .as_ref()
            .ok_or(MissionError::NoCommander)?;
        account
            .execute_equipment(&session.state, facts, command)
            .map_err(MissionError::Equipment)
    }
}

#[cfg(test)]
mod tests;

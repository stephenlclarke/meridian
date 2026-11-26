//! World-bound paid equipment commands; immutable snapshots grant no ownership or fit.
use super::{MissionError, World};
use crate::equipment::{self, Command, Event, Snapshot};
impl World {
    pub fn equipment_snapshot(&self) -> Option<Snapshot> {
        let account = self.commander_account.as_ref()?;
        let projection = self.reserved_service_projection().ok()?;
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
        let reserved_capacity = self.navigation_trade.reservations().capacity
            + self.cabin_tonnes().map_err(MissionError::Service)?;
        let account = self
            .commander_account
            .as_mut()
            .ok_or(MissionError::NoCommander)?;
        let session = self
            .freight_session
            .as_ref()
            .ok_or(MissionError::NoCommander)?;
        account
            .execute_equipment(&session.state, facts, command, reserved_capacity)
            .map_err(MissionError::Equipment)
    }
}

#[cfg(test)]
pub(super) mod tests;

/// Prospective cargo fitting keeps the live account unchanged until durable publication.
pub struct PreparedEquipment<'a> {
    world: &'a mut World,
    account: crate::freight::g02::CommanderAccount,
    event: Event,
    bytes: Vec<u8>,
}
#[derive(Debug)]
pub enum EquipmentPreparationError {
    Command(MissionError),
    Save(crate::save::SaveError),
}
impl core::fmt::Display for EquipmentPreparationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Command(error) => write!(f, "{error}"),
            Self::Save(error) => write!(f, "{error}"),
        }
    }
}
impl std::error::Error for EquipmentPreparationError {}
impl PreparedEquipment<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn event(&self) -> Event {
        self.event
    }
    pub fn commit(self) -> Event {
        self.world.commander_account = Some(self.account);
        self.event
    }
}
impl World {
    pub fn prepare_equipment(
        &mut self,
        command: Command,
        tick: u64,
    ) -> Result<PreparedEquipment<'_>, EquipmentPreparationError> {
        let command_error = EquipmentPreparationError::Command;
        let facts = self.current_service_facts(tick).map_err(command_error)?;
        let account = self
            .commander_account
            .as_ref()
            .ok_or_else(|| command_error(MissionError::NoCommander))?;
        let session = self
            .freight_session
            .as_ref()
            .ok_or_else(|| command_error(MissionError::NoCommander))?;
        let (account, event) = account
            .prepare_equipment(
                &session.state,
                facts,
                command,
                self.navigation_trade.reservations().capacity
                    + self
                        .cabin_tonnes()
                        .map_err(|error| command_error(MissionError::Service(error)))?,
            )
            .map_err(|error| command_error(MissionError::Equipment(error)))?;
        let bytes = self
            .save_bytes_with_modules(&account, &self.modular)
            .map_err(EquipmentPreparationError::Save)?;
        Ok(PreparedEquipment {
            world: self,
            account,
            event,
            bytes,
        })
    }
}

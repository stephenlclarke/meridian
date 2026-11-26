//! Physical finite fragment intake. Selection and flight remain owned by World.
use super::{FlightMode, MiningError, World, flight_pose};
use crate::equipment::WorkTargetId;
use crate::freight::g02::CommanderAccount;
use crate::mining::{AsteroidId, Collection, OreId, OreSnapshot};
use crate::save::{MAX_SAVE_BYTES, Writer};
use crate::station_services::{self as services, CargoIntake, CommodityId};

impl World {
    pub(in crate::flight) fn cargo_intakes(
        &self,
        modular: &super::super::modular::State,
    ) -> Vec<CargoIntake> {
        self.mining
            .cargo_intakes()
            .chain(modular.salvage_intakes())
            .collect()
    }

    pub(super) fn mining_selected(&self, id: AsteroidId) -> bool {
        self.selected_work_target()
            .is_some_and(|target| match target.id {
                WorkTargetId::Asteroid(selected) => selected == id,
                WorkTargetId::Ore(selected) => selected.asteroid() == id,
                _ => false,
            })
    }

    pub fn mining_collection_target(&self, id: AsteroidId) -> Option<OreSnapshot> {
        if self.combat_identity()?.system != id.system() {
            return None;
        }
        let pose = flight_pose(self.state.position, self.state.orientation)?;
        let distance = |ore: &OreSnapshot| {
            let p = ore.position;
            let d = crate::docking::Vec3::new(
                p.x - pose.position.x,
                p.y - pose.position.y,
                p.z - pose.position.z,
            );
            d.dot(d)
        };
        self.mining
            .pending_ore(self.combat.physical_tick())
            .into_iter()
            .filter(|ore| ore.id.asteroid() == id)
            .min_by(|a, b| {
                distance(a)
                    .total_cmp(&distance(b))
                    .then_with(|| a.id.fragment().cmp(&b.id.fragment()))
            })
    }

    pub fn mining_ore_target(&self, id: OreId) -> Option<OreSnapshot> {
        if self.combat_identity()?.system != id.asteroid().system() {
            return None;
        }
        self.mining
            .pending_ore(self.combat.physical_tick())
            .into_iter()
            .find(|ore| ore.id == id)
    }

    pub(in crate::flight) fn auto_scoop_mined_ore_tick(&mut self, id: AsteroidId) -> &'static str {
        if !self.mining_selected(id) {
            return "Target released";
        }
        let target = self
            .selected_work_target()
            .and_then(|target| match target.id {
                WorkTargetId::Ore(ore) => self.mining_ore_target(ore),
                WorkTargetId::Asteroid(_) => self.mining_collection_target(id),
                _ => None,
            });
        let Some(ore) = target else {
            return "Mining complete";
        };
        match self.collect_mined_ore(self.state.tick, Some(ore.id), true) {
            Ok(()) if self.mining_collection_target(id).is_none() => "Mining complete",
            Ok(()) => "Ore collected",
            Err(MiningError::NoScoops) => "Fit fuel scoops",
            Err(MiningError::HoldFull) => "Cargo hold full",
            Err(MiningError::NoEligibleOreBody) => "Approach scoop aperture",
            Err(MiningError::ProtectedActivity) => "Protected activity",
            Err(_) => "Collection unavailable",
        }
    }

    /// Designated collection uses the full aperture and relative-velocity checks.
    pub fn request_manual_ore_collection(&mut self) -> Result<(), MiningError> {
        let Some(WorkTargetId::Ore(id)) = self.selected_work_target().map(|work| work.id) else {
            return Err(MiningError::NoEligibleOreBody);
        };
        if !self.pilot_operates_selected_tool() {
            return Err(MiningError::NoEligibleOreBody);
        }
        self.collect_mined_ore(self.state.tick, Some(id), true)
    }

    pub fn request_collect_mined_ore(&mut self, tick: u64) -> Result<(), MiningError> {
        self.collect_mined_ore(tick, None, false)
    }

    pub(super) fn prepare_mining_intake_account(
        &self,
        commodity: CommodityId,
    ) -> Result<CommanderAccount, MiningError> {
        let account = self
            .commander_account
            .as_ref()
            .ok_or(MiningError::NoCommander)?;
        let freight = &self
            .freight_session
            .as_ref()
            .ok_or(MiningError::NoCommander)?
            .state;
        let mut reservations = self.navigation_trade.reservations();
        reservations.capacity = reservations
            .capacity
            .checked_add(self.cabin_tonnes().map_err(|_| MiningError::InvalidState)?)
            .ok_or(MiningError::HoldFull)?;
        account
            .prepare_cargo_intake(freight, commodity, reservations)
            .map_err(|error| match error {
                services::Error::HoldFull | services::Error::QuantityLimit => MiningError::HoldFull,
                _ => MiningError::InvalidState,
            })
    }

    fn collect_mined_ore(
        &mut self,
        tick: u64,
        selected: Option<OreId>,
        automatic: bool,
    ) -> Result<(), MiningError> {
        if tick != self.state.tick {
            return Err(MiningError::StaleCommand);
        }
        if self.is_paused() {
            return Err(MiningError::Paused);
        }
        if self.state.mode != FlightMode::Flight || !self.pulse.alive() {
            return Err(MiningError::NotInFlight);
        }
        if self.emp.input_blocked() {
            return Err(MiningError::Resources);
        }
        let scoop = self.fitted_fuel_scoops().ok_or(MiningError::NoScoops)?;
        let owner = self.combat_identity().ok_or(MiningError::NoCommander)?;
        if owner.system != crate::navigation::SystemId::AVENOR {
            return Err(MiningError::NoEligibleOreBody);
        }
        if self.flight_reception.occupancy().1 > 0
            || self.navigation_trade.occupancy().1 > 0
            || self.practice.is_some_and(|p| !p.reception_is_terminal())
            || self
                .mission_snapshot()
                .is_some_and(|s| s.freight.held_tonnes > 0)
        {
            return Err(MiningError::ProtectedActivity);
        }
        if self
            .snapshot()
            .navigation
            .is_none_or(|n| n.travel.is_some())
        {
            return Err(MiningError::NotInFlight);
        }
        let pose = flight_pose(self.state.position, self.state.orientation)
            .ok_or(MiningError::InvalidState)?;
        if !self.mining_motion_clear(pose, pose, 0.0) {
            return Err(MiningError::Blocked);
        }
        let ore = self
            .mining
            .pending_ore(self.combat.physical_tick())
            .into_iter()
            .filter(|ore| selected.is_none_or(|id| ore.id == id))
            .find(|ore| ore.scoop_admissible(pose, self.state.speed, automatic))
            .ok_or(MiningError::NoEligibleOreBody)?;
        let account = self.prepare_mining_intake_account(ore.commodity)?;
        let receipt = Collection {
            id: ore.id,
            owner,
            item: scoop.item,
            revision: account.revision(),
            world_tick: tick,
            physical_tick: self.combat.physical_tick(),
            pose,
            speed: self.state.speed,
            automatic,
        };
        let mining = self.mining.prepare_collection(receipt)?;
        let mut before = Writer::new();
        let mut after = Writer::new();
        if !self.mining.collections().is_empty() {
            self.mining.encode_collections(&mut before);
        } else {
            after.bytes.extend([0_u8; 8]);
        }
        mining.encode_collections(&mut after);
        let size = self
            .encoded_current_size()
            .ok()
            .and_then(|n| n.checked_sub(before.bytes.len()))
            .and_then(|n| n.checked_add(after.bytes.len()))
            .ok_or(MiningError::ProofCapacity)?;
        if size > MAX_SAVE_BYTES {
            return Err(MiningError::ProofCapacity);
        }
        self.commander_account = Some(account);
        self.mining = mining;
        Ok(())
    }
}

//! World owns the real belt, work admission and account/resource/depletion commit.
use super::{FlightMode, World, flight_pose};
use crate::docking::RigidPose;
use crate::mining::{
    BeltSnapshot, MINING_RANGE, MiningContact, MiningError, MiningShot, MiningToolAuthority,
    belt_destination,
};
use crate::navigation::SystemId;
mod collection;
mod persistence;

impl World {
    /// Chart selection queries the actual profile/provider, including while another system is
    /// current. Old explicit native formats without a belt return no destination or marker.
    pub fn mining_destination(&self, system: SystemId) -> Option<crate::mining::BeltDestination> {
        if !self.mining.enabled() {
            return None;
        }
        self.navigation_catalogue.as_ref()?.system(system).ok()?;
        belt_destination(system)
    }
    pub fn mining_belt_snapshot(&self) -> Option<BeltSnapshot> {
        if !self.mining.enabled() {
            return None;
        }
        let identity = self.combat_identity()?;
        let destination = belt_destination(identity.system)?;
        let pose = flight_pose(self.state.position, self.state.orientation)?;
        let delta = crate::docking::Vec3::new(
            destination.position.x - pose.position.x,
            destination.position.y - pose.position.y,
            destination.position.z - pose.position.z,
        );
        Some(BeltSnapshot {
            destination,
            bodies: self.mining.bodies(),
            distance_from_player: delta.dot(delta).sqrt(),
            pending_ore: self.mining.pending_ore(self.combat.physical_tick()),
            front_contact: self
                .mining_beam_contact(pose, MINING_RANGE)
                .filter(|contact| self.mining_contact_visible(pose, *contact)),
        })
    }
    pub(super) fn mining_beam_contact(&self, pose: RigidPose, range: f64) -> Option<MiningContact> {
        self.asteroid_beam_in_system(self.combat_identity()?.system, pose, range)
    }
    pub(super) fn asteroid_beam_in_system(
        &self,
        system: SystemId,
        pose: RigidPose,
        range: f64,
    ) -> Option<MiningContact> {
        (system == SystemId::AVENOR)
            .then(|| self.mining.beam(pose.position, pose.orientation.z, range))
            .flatten()
    }
    pub(super) fn mining_step_contact(
        &self,
        system: SystemId,
        tick: u64,
        ray: RigidPose,
        range: f64,
    ) -> Option<MiningContact> {
        if system != SystemId::AVENOR {
            return None;
        }
        self.mining
            .at_step(tick)
            .beam(ray.position, ray.orientation.z, range)
    }
    pub(super) fn mining_segment_at_step(
        &self,
        system: SystemId,
        tick: u64,
        a: crate::docking::Vec3,
        b: crate::docking::Vec3,
        radius: f64,
    ) -> bool {
        system != SystemId::AVENOR || self.mining.at_step(tick).segment_clear(a, b, radius)
    }
    pub(super) fn mining_motion_clear(
        &self,
        before: RigidPose,
        after: RigidPose,
        angular: f64,
    ) -> bool {
        let Some(identity) = self.combat_identity() else {
            return true;
        };
        Self::mining_motion_clear_with(
            self.station_data.as_ref(),
            &self.mining,
            identity.system,
            before,
            after,
            angular,
        )
    }
    pub(super) fn mining_motion_clear_at_step(
        &self,
        data: &crate::station::OriginalStationData,
        system: SystemId,
        tick: u64,
        before: RigidPose,
        after: RigidPose,
        angular: f64,
    ) -> bool {
        if system != SystemId::AVENOR || !self.mining.enabled() {
            return true;
        }
        Self::mining_motion_clear_with(
            Some(data),
            &self.mining.at_step(tick),
            system,
            before,
            after,
            angular,
        )
    }
    fn mining_motion_clear_with(
        data: Option<&crate::station::OriginalStationData>,
        mining: &crate::mining::State,
        system: SystemId,
        before: RigidPose,
        after: RigidPose,
        angular: f64,
    ) -> bool {
        if system != SystemId::AVENOR || !mining.enabled() {
            return true;
        }
        data.is_some_and(|data| {
            mining
                .bodies()
                .into_iter()
                .filter(|body| body.remaining_energy > 0.0)
                .all(|body| {
                    let delta = crate::docking::Vec3::new(
                        after.position.x - before.position.x,
                        after.position.y - before.position.y,
                        after.position.z - before.position.z,
                    );
                    let offset = crate::docking::Vec3::new(
                        body.position.x - before.position.x,
                        body.position.y - before.position.y,
                        body.position.z - before.position.z,
                    );
                    let denominator = delta.dot(delta);
                    let t = if denominator > 0.0 {
                        (offset.dot(delta) / denominator).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let nearest = crate::docking::Vec3::new(
                        offset.x - delta.x * t,
                        offset.y - delta.y * t,
                        offset.z - delta.z * t,
                    );
                    nearest.dot(nearest).sqrt()
                        > body.bounding_radius() + data.player_clearance_radius() + 1.0
                        || (!body.contains_point(before.position)
                            && !body.contains_point(after.position)
                            && data.asteroid_motion_clear(before, after, angular, body))
                })
        })
    }
    fn mining_contact_visible(&self, pose: RigidPose, contact: MiningContact) -> bool {
        self.pulse_scene(pose).is_ok_and(|(_, scene, distance)| {
            matches!(scene, super::PulseContact::AsteroidBlocked { id, .. } if id == contact.asteroid)
                || distance > contact.distance
        })
    }
    pub fn mining_tool_authority(&self) -> Result<MiningToolAuthority, MiningError> {
        let actual = self
            .fitted_mining_tool()
            .ok_or(MiningError::NoCompatibleFittedTool)?;
        Ok(MiningToolAuthority {
            owner: actual.owner,
            hull: actual.hull,
            account_revision: actual.account_revision,
            item_id: actual.item,
            epoch: actual.epoch,
            opening_emission: self.combat.emissions,
            scope: std::sync::Arc::downgrade(&self.mining_scope),
        })
    }
    /// A manual designation never bends the beam to the selected rock.
    pub fn request_manual_mining(&mut self) -> Result<MiningShot, MiningError> {
        let tool = self.mining_tool_authority()?;
        let Some(crate::equipment::WorkTargetId::Asteroid(id)) =
            self.selected_work_target().map(|work| work.id)
        else {
            return Err(MiningError::NoTarget);
        };
        if !self.pilot_operates_selected_tool() {
            return Err(MiningError::NoTarget);
        }
        self.mine(tool, self.state.tick, Some(id))
    }

    /// An actual owned fitted tool emits only after the native first-contact query.
    /// Damage releases finite pending ore; it does not credit cargo or a wallet.
    pub fn request_mining(
        &mut self,
        tool: MiningToolAuthority,
        tick: u64,
    ) -> Result<MiningShot, MiningError> {
        self.mine(tool, tick, None)
    }

    fn mine(
        &mut self,
        tool: MiningToolAuthority,
        tick: u64,
        selected: Option<crate::mining::AsteroidId>,
    ) -> Result<MiningShot, MiningError> {
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
        let owner = self.combat_identity().ok_or(MiningError::NoCommander)?;
        let actual = self
            .fitted_mining_tool()
            .ok_or(MiningError::NoCompatibleFittedTool)?;
        if (
            actual.owner,
            actual.hull,
            actual.account_revision,
            actual.item,
            actual.epoch,
        ) != (
            tool.owner,
            tool.hull,
            tool.account_revision,
            tool.item_id,
            tool.epoch,
        ) || tool.opening_emission != self.combat.emissions
            || !tool
                .scope
                .upgrade()
                .is_some_and(|scope| std::sync::Arc::ptr_eq(&scope, &self.mining_scope))
        {
            return Err(MiningError::NoCompatibleFittedTool);
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
        let contact = self
            .mining_beam_contact(pose, MINING_RANGE)
            .filter(|hit| selected.is_none_or(|id| id == hit.asteroid))
            .ok_or(MiningError::NoTarget)?;
        if !self.mining_contact_visible(pose, contact) {
            return Err(MiningError::Blocked);
        }
        let pulse = self
            .prepare_mining_resources(actual.item, tick)
            .map_err(|_| MiningError::Resources)?;
        let (combat, emission) = self
            .combat
            .reserve_mining_emission()
            .map_err(|_| MiningError::Resources)?;
        let damage = self
            .damage
            .mining(self.combat.physical_tick(), contact.asteroid.ordinal())
            .map_err(|_| MiningError::ProofCapacity)?;
        let stroke = crate::mining::MiningStroke {
            owner,
            item: actual.item,
            account_revision: actual.account_revision,
            world_tick: tick,
            physical_tick: self.combat.physical_tick(),
            emission,
            pose,
            speed: self.state.speed,
            contact,
            peers: self.traffic.missile_scene(
                owner.system,
                tick,
                crate::traffic::SceneStage::Command(emission),
            ),
            missiles: std::sync::Arc::new(self.missiles.beam_scene(owner.visit)),
        };
        let (mining, released) = self.mining.prepare_work(stroke)?;
        self.check_mining_capacity(&mining, &damage)
            .map_err(|_| MiningError::ProofCapacity)?;
        let body = mining
            .body(usize::from(contact.asteroid.ordinal()))
            .ok_or(MiningError::InvalidState)?;
        let remaining_energy = body.remaining_energy;
        let commodity = body.commodity;
        // No fallible operation follows the first live assignment. Cargo, cash and account
        // revision cannot change until a separate genuine scoop/intake producer is admitted.
        self.mining = mining;
        self.pulse = pulse;
        self.combat = combat;
        self.damage = damage;
        Ok(MiningShot {
            contact,
            item_id: actual.item,
            commodity,
            extracted_units: released,
            remaining_energy,
            tick,
        })
    }
    pub(in crate::flight) fn auto_mine_tick(
        &mut self,
        id: crate::mining::AsteroidId,
    ) -> &'static str {
        if !self.mining_selected(id) {
            return "Target released";
        }
        let Some(body) = self
            .mining
            .body(usize::from(id.ordinal()))
            .filter(|b| b.id == id)
        else {
            return "Target unavailable";
        };
        if body.remaining_energy <= 0.0 {
            return if self.mining_collection_target(id).is_some() {
                "Ore ready"
            } else {
                "Mining complete"
            };
        }
        if self.fitted_fuel_scoops().is_none() {
            return "Fit fuel scoops";
        }
        match self.prepare_mining_intake_account(body.commodity) {
            Err(MiningError::HoldFull) => return "Cargo hold full",
            Err(_) => return "Cargo unavailable",
            Ok(_) => {}
        }
        if self
            .mining_belt_snapshot()
            .and_then(|belt| belt.front_contact)
            .is_none_or(|contact| contact.asteroid != id)
        {
            return "Align mining laser";
        }
        match self.select_auto_mining_tool() {
            Ok(()) => {}
            Err(MiningError::NoCompatibleFittedTool) => return "Fit mining laser",
            Err(MiningError::ProtectedActivity) => return "Protected activity",
            Err(MiningError::Resources) => return "Mining recharging",
            Err(_) => return "Mining unavailable",
        }
        let Ok(tool) = self.mining_tool_authority() else {
            return "Fit mining laser";
        };
        match self.request_mining(tool, self.state.tick) {
            Ok(shot) if shot.extracted_units > 0 => "Ore ready",
            Ok(_) => "Mining pulse",
            Err(MiningError::Resources) => "Mining recharging",
            Err(MiningError::ProtectedActivity) => "Protected activity",
            Err(_) => "Mining unavailable",
        }
    }
}
#[cfg(test)]
pub(super) mod tests;

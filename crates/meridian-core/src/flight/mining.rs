//! World owns the real belt, work admission and account/resource/depletion commit.
use super::{FlightMode, World, flight_pose};
use crate::docking::RigidPose;
use crate::mining::{
    BeltSnapshot, MINING_MAX_SPEED, MINING_RANGE, MiningContact, MiningError, MiningShot,
    MiningToolAuthority, belt_destination,
};
use crate::navigation::SystemId;

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
    pub(super) fn mining_motion_clear(
        &self,
        before: RigidPose,
        after: RigidPose,
        angular: f64,
    ) -> bool {
        if !self.mining.enabled() {
            return true;
        }
        if self
            .combat_identity()
            .is_none_or(|id| id.system != SystemId::AVENOR)
        {
            return true;
        }
        self.station_data.as_ref().is_some_and(|data| {
            self.mining.bodies().into_iter().all(|body| {
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
    /// Tool capability must come from the genuine fitted-account owner. Researcher/chart selection
    /// supplies navigation intent only. The work ray and material transfer are derived here.
    pub fn request_mining(
        &mut self,
        tool: MiningToolAuthority,
        tick: u64,
    ) -> Result<MiningShot, MiningError> {
        if tick != self.state.tick {
            return Err(MiningError::StaleCommand);
        }
        if self.is_paused() {
            return Err(MiningError::Paused);
        }
        if self.state.mode != FlightMode::Flight {
            return Err(MiningError::NotInFlight);
        }
        let account = self
            .commander_account
            .as_ref()
            .ok_or(MiningError::NoCommander)?;
        let session = self
            .freight_session
            .as_ref()
            .ok_or(MiningError::NoCommander)?;
        let projection = account
            .service_projection(&session.state)
            .map_err(|_| MiningError::InvalidState)?;
        if projection.owner != tool.owner
            || projection.hull != tool.hull
            || projection.revision != tool.account_revision
            || tool.item_id == 0
            || session.epoch() != tool.epoch
            || !tool
                .scope
                .upgrade()
                .is_some_and(|scope| std::sync::Arc::ptr_eq(&scope, &self.mining_scope))
            || (tool.current_item)(self) != Some(tool.item_id)
        {
            return Err(MiningError::NoCompatibleFittedTool);
        }
        if !(0.0..=MINING_MAX_SPEED).contains(&self.state.speed) {
            return Err(MiningError::TooFast);
        }
        let pose = flight_pose(self.state.position, self.state.orientation)
            .ok_or(MiningError::InvalidState)?;
        if self
            .navigation
            .is_none_or(|nav| nav.snapshot(account.fuel_tenths()).travel.is_some())
        {
            return Err(MiningError::NotInFlight);
        }
        if !self.mining_motion_clear(pose, pose, 0.0) {
            return Err(MiningError::Blocked);
        }
        let contact = self
            .mining_beam_contact(pose, MINING_RANGE)
            .ok_or(MiningError::NoTarget)?;
        if !self.mining_contact_visible(pose, contact) {
            return Err(MiningError::Blocked);
        }
        let (mining, commodity) = self.mining.prepare_extract(contact.asteroid)?;
        let candidate = account
            .prepare_mining_yield(
                &session.state,
                commodity,
                self.navigation_trade.reservations(),
            )
            .map_err(|error| {
                if matches!(error, crate::station_services::Error::HoldFull) {
                    MiningError::HoldFull
                } else {
                    MiningError::InvalidState
                }
            })?;
        let pulse = self
            .pulse
            .prepare_mining_discharge()
            .map_err(|_| MiningError::Resources)?;
        let (combat, _) = self
            .combat
            .reserve_missile_emission()
            .map_err(|_| MiningError::Resources)?;
        // All checks and candidate construction precede these assignments. No item, cash, market
        // stock, motion or additional cargo owner is created by work.
        self.commander_account = Some(candidate);
        self.mining = mining;
        self.pulse = pulse;
        self.combat = combat;
        if let Some(practice) = &mut self.practice {
            practice.observe_discharge(tick);
        }
        Ok(MiningShot {
            contact,
            item_id: tool.item_id,
            commodity,
            extracted_units: 1,
            tick,
        })
    }
}

#[cfg(test)]
mod tests;

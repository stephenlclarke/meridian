//! Closed work/source/resource boundary. Loading never mines or collects anything.
use super::{FlightMode, MINING_RANGE, RigidPose, World, flight_pose};
use crate::save::{MAX_SAVE_BYTES, SaveError, Writer};
impl World {
    pub(super) fn check_mining_capacity(
        &self,
        mining: &crate::mining::State,
        damage: &super::super::damage::State,
    ) -> Result<(), SaveError> {
        let mut old = Writer::new();
        self.mining.encode(&mut old);
        self.damage.encode(&mut old);
        let mut new = Writer::new();
        mining.encode(&mut new);
        damage.encode(&mut new);
        let size = self
            .encoded_current_size()?
            .checked_sub(old.bytes.len())
            .and_then(|n| n.checked_add(new.bytes.len()))
            .ok_or(SaveError::TooLarge)?;
        if size > MAX_SAVE_BYTES {
            return Err(SaveError::TooLarge);
        }
        Ok(())
    }
    pub(in crate::flight) fn validate_mining_work(
        &self,
        modules: &super::super::modular::State,
    ) -> Result<(), SaveError> {
        let invalid = SaveError::InvalidState("mining source, contact or discharge provenance");
        self.mining.validate()?;
        let owner = self.combat_identity().ok_or(invalid)?;
        if self
            .mining
            .expiry_receipts()
            .any(|(tick, visit)| tick > self.state.tick || visit >= owner.visit)
        {
            return Err(invalid);
        }
        for hit in self.mining.strokes() {
            self.validate_mining_visit(hit.owner, hit.world_tick, hit.physical_tick)?;
            let data = self.station_at_visit(hit.owner.visit).ok_or(invalid)?;
            if hit.owner.owner != owner.owner
                || hit.owner.hull != owner.hull
                || hit.owner.visit > owner.visit
                || hit.speed > self.hull_at_visit(hit.owner.visit).flight_rates().0
                || hit.emission > self.combat.emissions
                || self
                    .commander_account
                    .as_ref()
                    .is_none_or(|a| hit.account_revision > a.revision())
                || !modules.mining_was_fitted(hit)
                || hit.peers.system != hit.owner.system
                || hit.peers.world_tick != hit.world_tick
                || hit.peers.stage != crate::traffic::SceneStage::Command(hit.emission)
                || !self.mining_motion_at_source(data, hit.pose, hit.emission)
                || (hit.world_tick == self.state.tick
                    && (flight_pose(self.state.position, self.state.orientation) != Some(hit.pose)
                        || self.state.speed != hit.speed))
            {
                return Err(invalid);
            }
            self.traffic
                .validate_missile_scene(&hit.peers, self.state.tick, data)?;
            if hit
                .peers
                .contact(
                    data,
                    hit.pose.position,
                    hit.pose.orientation.z,
                    MINING_RANGE,
                )
                .is_some_and(|c| c.distance <= hit.contact.distance)
                || data
                    .beam_block_distance(
                        data.station_pose(),
                        hit.pose.position,
                        hit.pose.orientation.z,
                        MINING_RANGE,
                    )
                    .map_err(|_| invalid)?
                    .is_some_and(|d| d <= hit.contact.distance)
            {
                return Err(invalid);
            }
            let system = self
                .navigation_catalogue
                .as_ref()
                .ok_or(invalid)?
                .system(hit.owner.system)
                .map_err(|_| invalid)?;
            let scene = crate::celestial::scene(
                system,
                hit.owner.visit,
                hit.world_tick,
                data.station_pose().pose.position,
            );
            let a = hit.pose.position;
            let b = hit.contact.point;
            if [scene.planet, scene.sun].into_iter().any(|body| {
                body.blocks_motion(
                    [a.x as f32, a.y as f32, a.z as f32],
                    [b.x as f32, b.y as f32, b.z as f32],
                    0.0,
                )
            }) {
                return Err(invalid);
            }
            self.missiles.validate_retained_beam(
                &self.ecm,
                super::super::missiles::RetainedBeam {
                    system: hit.owner.system,
                    visit: hit.owner.visit,
                    world_tick: hit.world_tick,
                    physical_tick: hit.physical_tick,
                    command_emission: Some(hit.emission),
                    ecm_after_scene: false,
                    ray: hit.pose,
                    point: hit.contact.point,
                    bodies: &hit.missiles,
                },
            )?;
        }
        for receipt in self.mining.collections() {
            self.validate_mining_visit(receipt.owner, receipt.world_tick, receipt.physical_tick)?;
            if receipt.owner.owner != owner.owner
                || receipt.owner.hull != owner.hull
                || receipt.owner.visit > owner.visit
                || receipt.speed > self.hull_at_visit(receipt.owner.visit).flight_rates().0
                || self
                    .commander_account
                    .as_ref()
                    .is_none_or(|a| receipt.revision > a.revision())
                || !modules.scoop_was_fitted_for_collection(receipt)
                || (receipt.world_tick == self.state.tick
                    && (flight_pose(self.state.position, self.state.orientation)
                        != Some(receipt.pose)
                        || self.state.speed != receipt.speed))
            {
                return Err(invalid);
            }
        }
        if self.state.mode != FlightMode::Docked
            && !self.mining_motion_clear(
                flight_pose(self.state.position, self.state.orientation).ok_or(invalid)?,
                flight_pose(self.state.position, self.state.orientation).ok_or(invalid)?,
                0.0,
            )
        {
            return Err(invalid);
        }
        Ok(())
    }
    pub(super) fn validate_mining_visit(
        &self,
        owner: crate::CombatIdentity,
        tick: u64,
        clock: u64,
    ) -> Result<(), SaveError> {
        let invalid = SaveError::InvalidState("mining visit timing");
        if tick > self.state.tick
            || clock > self.combat.physical_tick()
            || tick.checked_sub(clock).is_none_or(|offset| {
                self.state
                    .tick
                    .checked_sub(self.combat.physical_tick())
                    .is_none_or(|end| offset > end)
            })
            || self.jump_journal.system_at_visit(owner.visit, tick)? != owner.system
            || self
                .xyzzy_medusa_issue()
                .is_some_and(|(dock, issue, _)| (owner.visit > dock.visit()) != (tick > issue))
        {
            return Err(invalid);
        }
        let (jumps, arrival) = self.jump_journal.visit_origin(owner.visit);
        let launch = owner
            .visit
            .checked_sub(1)
            .and_then(|v| v.checked_sub(jumps))
            .ok_or(invalid)?;
        let berth = self.damage.berth_clock(launch).ok_or(invalid)?;
        if arrival.map_or(clock <= berth, |arrival| clock < arrival)
            || self
                .damage
                .berth_clock(launch.checked_add(1).ok_or(invalid)?)
                .is_some_and(|next| clock >= next)
        {
            return Err(invalid);
        }
        // Both clocks advance together in flight, including through hyperspace.
        // The neighbouring jump seals their exact offset on this flight.
        if let Some(next) = self.jump_journal.completed_timing(owner.visit)?
            && next
                .departure_tick
                .checked_sub(tick)
                .zip(next.departure_clock.checked_sub(clock))
                .is_none_or(|(elapsed, physical)| elapsed != physical)
        {
            return Err(invalid);
        }
        if arrival.is_some() {
            let prev = self
                .jump_journal
                .completed_timing(owner.visit - 1)?
                .ok_or(invalid)?;
            if tick
                .checked_sub(prev.arrival_tick)
                .zip(clock.checked_sub(prev.arrival_clock))
                .is_none_or(|(elapsed, physical)| elapsed != physical)
            {
                return Err(invalid);
            }
        }
        Ok(())
    }
    fn mining_motion_at_source(
        &self,
        data: &crate::station::OriginalStationData,
        pose: RigidPose,
        emission: u64,
    ) -> bool {
        // Historical native contact lies outside the actual source mesh. A later terminal
        // tombstone must not legitimize having fired from inside an intact original rock.
        let original = self.mining.before_emission(emission);
        Self::mining_motion_clear_with(
            Some(data),
            &original,
            crate::navigation::SystemId::AVENOR,
            pose,
            pose,
            0.0,
        )
    }
}

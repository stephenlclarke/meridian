//! Work-target selection from current Meridian geometry and custody only.
use super::{ModuleSlot, World};
use crate::equipment::{WorkTarget, WorkTargetId, WorkTargetRole, WorkTargetingFit};
use crate::navigation::SystemId;
use crate::traffic::NpcPhase;

impl World {
    pub fn work_targeting_fit(&self) -> WorkTargetingFit {
        let Some(ledger) = self
            .commander_account
            .as_ref()
            .and_then(|account| self.modular.ledger(self, account).ok())
        else {
            return WorkTargetingFit::default();
        };
        if ledger.fitted(ModuleSlot::Targeting).is_none() {
            return WorkTargetingFit::default();
        }
        WorkTargetingFit {
            mining: ledger.fitted(ModuleSlot::MiningTargeting).is_some(),
            salvage: ledger.fitted(ModuleSlot::SalvageTargeting).is_some(),
        }
    }

    pub fn work_targets(&self) -> Vec<WorkTarget> {
        if self.state.mode != crate::FlightMode::Flight
            || self.emp.electrical_disabled()
            || self
                .combat_snapshot()
                .is_some_and(|combat| combat.player.destroyed)
        {
            return Vec::new();
        }
        let Some(identity) = self.combat_identity() else {
            return Vec::new();
        };
        if self
            .snapshot()
            .navigation
            .is_some_and(|nav| nav.travel.is_some())
        {
            return Vec::new();
        }
        let fit = self.work_targeting_fit();
        let mut targets = Vec::new();
        if fit.mining && self.mining.enabled() && identity.system == SystemId::AVENOR {
            targets.extend(
                self.mining
                    .bodies()
                    .into_iter()
                    .filter(|body| body.remaining_energy > 0.0 && body.remaining_units > 0)
                    .map(|body| WorkTarget {
                        id: WorkTargetId::Asteroid(body.id),
                        role: WorkTargetRole::RockHold,
                        position: body.position,
                    }),
            );
            // Pending ore is real; its current projection grants tracking only.
            targets.extend(
                self.mining
                    .pending_ore(self.combat.physical_tick())
                    .into_iter()
                    .map(|ore| WorkTarget {
                        id: WorkTargetId::Ore(ore.id),
                        role: WorkTargetRole::OreTracking,
                        position: ore.position,
                    }),
            );
        }
        if fit.salvage
            && let Some(traffic) = self.traffic_snapshot()
        {
            targets.extend(
                traffic
                    .actors
                    .into_iter()
                    .filter(|actor| {
                        actor.id.profile() == identity.owner
                            && actor.id.system() == identity.system
                            && actor.phase == NpcPhase::Destroyed
                            && self.wreck_manifest(actor.id).is_ok()
                    })
                    .map(|actor| WorkTarget {
                        id: WorkTargetId::Wreck(actor.id),
                        role: WorkTargetRole::WreckTracking,
                        position: actor.pose.position,
                    }),
            );
        }
        targets
    }

    /// Longitudinal guidance for a fixed, live rock. Steering and ordinary swept motion
    /// validation still run through the flight controller.
    pub(in crate::flight) fn asteroid_hold_input(
        &self,
        target: WorkTarget,
        input: crate::FlightInput,
    ) -> crate::FlightInput {
        let WorkTargetId::Asteroid(id) = target.id else {
            return input;
        };
        let Some(body) = self
            .mining
            .body(usize::from(id.ordinal()))
            .filter(|body| body.id == id && body.remaining_energy > 0.0)
        else {
            return input;
        };
        let Some(station) = &self.station_data else {
            return input;
        };
        let Some(player) = crate::flight::flight_pose(self.state.position, self.state.orientation)
        else {
            return input;
        };
        let offset = |point: crate::docking::Vec3| {
            crate::docking::Vec3::new(
                point.x - body.position.x,
                point.y - body.position.y,
                point.z - body.position.z,
            )
        };
        let radius = body
            .vertices
            .into_iter()
            .map(|point| {
                let delta = offset(point);
                delta.dot(delta).sqrt()
            })
            .fold(0.0, f64::max);
        let to_rock = offset(player.position);
        let distance = to_rock.dot(to_rock).sqrt();
        // Frozen RelativeWorkController.workMargin plus both hull radii.
        let stand_off = radius + station.player_clearance_radius() + 1.0;
        let room = (distance - stand_off).max(0.0);
        let facing = -to_rock.dot(player.orientation.z) > distance * 0.995;
        let units = f64::from(crate::flight::DISTANCE_PER_SPEED_UNIT);
        let acceleration = f64::from(crate::flight::ACCELERATION) * units;
        let delta_t = f64::from(crate::FIXED_DELTA_SECONDS);
        let desired = if facing {
            // Reserve one additional tick of travel alongside continuous braking distance.
            let tick_brake = acceleration * delta_t;
            (tick_brake.hypot((2.0 * acceleration * room).sqrt()) - tick_brake) / units
        } else {
            0.0
        };
        let throttle = ((desired - f64::from(self.state.speed)) / (acceleration * delta_t / units))
            .clamp(-1.0, 1.0) as f32;
        crate::FlightInput { throttle, ..input }
    }
}

#[cfg(test)]
mod tests;

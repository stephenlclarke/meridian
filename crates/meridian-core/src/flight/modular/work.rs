//! Work-target selection from current Meridian geometry and custody only.
use super::{ModuleSlot, World};
use crate::equipment::{WorkTarget, WorkTargetId, WorkTargetRole, WorkTargetingFit};
use crate::navigation::SystemId;
use crate::traffic::NpcPhase;

use crate::docking::Vec3;

fn difference(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

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
        let fit = self.work_targeting_fit();
        if !fit.mining && !fit.salvage {
            return Vec::new();
        }
        self.available_work_contacts(true)
            .into_iter()
            .filter(|work| match work.id {
                WorkTargetId::Wreck(_) => fit.salvage,
                WorkTargetId::Asteroid(_) | WorkTargetId::Ore(_) => fit.mining,
            })
            .collect()
    }

    pub(in crate::flight) fn available_work_contacts(
        &self,
        include_released_sources: bool,
    ) -> Vec<WorkTarget> {
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
        let mut targets = Vec::new();
        if self.mining.enabled() && identity.system == SystemId::AVENOR {
            let pending = self.mining.pending_ore(self.combat.physical_tick());
            targets.extend(
                self.mining
                    .bodies()
                    .into_iter()
                    .filter(|body| {
                        (body.remaining_energy > 0.0 && body.remaining_units > 0)
                            || include_released_sources
                                && pending.iter().any(|ore| ore.id.asteroid() == body.id)
                    })
                    .map(|body| WorkTarget {
                        id: WorkTargetId::Asteroid(body.id),
                        role: WorkTargetRole::RockHold,
                        position: body.position,
                    }),
            );
            targets.extend(pending.into_iter().map(|ore| WorkTarget {
                id: WorkTargetId::Ore(ore.id),
                role: WorkTargetRole::OreTracking,
                position: ore.position,
            }));
        }
        if let Some(traffic) = self.traffic_snapshot() {
            targets.extend(
                traffic
                    .actors
                    .into_iter()
                    .filter(|actor| {
                        actor.id.profile() == identity.owner
                            && actor.id.system() == identity.system
                            && (actor.electrical_disabled
                                || (actor.phase == NpcPhase::Destroyed
                                    && self.wreck_manifest(actor.id).is_ok()))
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

    /// Desired centre-to-centre separation, using the same hull bounds as swept motion.
    pub(in crate::flight) fn work_standoff(&self, target: WorkTarget) -> Option<f64> {
        let player_radius = self.station_data.as_ref()?.player_clearance_radius();
        match target.id {
            WorkTargetId::Asteroid(id) => {
                let rock = self
                    .mining
                    .body(usize::from(id.ordinal()))
                    .filter(|rock| rock.id == id)?;
                // Leave the settled hold outside the swept collision margin even at
                // the inner edge of its half-unit position tolerance.
                Some(rock.bounding_radius() + player_radius + 2.0)
            }
            // Traffic rejects centre distances <= 2 * radius + 1.
            WorkTargetId::Wreck(_) => Some(2.0 * player_radius + 2.0),
            // Original local scoop aperture is 45 below and 55 forward.
            WorkTargetId::Ore(_) => Some(45.0_f64.hypot(55.0)),
        }
    }

    /// A disabled ship can coast at its last velocity. Fly behind it, matching its
    /// velocity so that it remains in front of the cutter at the working distance.
    pub(in crate::flight) fn moving_work_course(
        &self,
        target: WorkTarget,
        player: crate::docking::RigidPose,
    ) -> Option<(Vec3, f64, f64)> {
        let WorkTargetId::Wreck(id) = target.id else {
            return None;
        };
        let ship = self.traffic_snapshot()?.actors.into_iter().find(|actor| {
            actor.id == id
                && actor.electrical_disabled
                && actor.phase != NpcPhase::Destroyed
                && actor.speed_units > 0.0
        })?;
        let separation = self.work_standoff(target)?;
        let scale = f64::from(crate::flight::DISTANCE_PER_SPEED_UNIT);
        let forward = ship.pose.orientation.z;
        let speed = ship.speed_units * scale;
        let error = difference(
            difference(ship.pose.position, player.position),
            Vec3::new(
                forward.x * separation,
                forward.y * separation,
                forward.z * separation,
            ),
        );
        let distance = error.dot(error).sqrt();
        let acceleration = f64::from(crate::flight::ACCELERATION) * scale;
        let gain = if distance > 0.0 {
            (acceleration / distance).sqrt().min(0.75)
        } else {
            0.75
        };
        let velocity = Vec3::new(
            forward.x * speed + error.x * gain,
            forward.y * speed + error.y * gain,
            forward.z * speed + error.z * gain,
        );
        let look_at = Vec3::new(
            player.position.x + velocity.x,
            player.position.y + velocity.y,
            player.position.z + velocity.z,
        );
        let desired_speed = velocity.dot(velocity).sqrt() / scale;
        let player_speed = f64::from(self.state.speed) * scale;
        let relative = Vec3::new(
            player.orientation.z.x * player_speed - forward.x * speed,
            player.orientation.z.y * player_speed - forward.y * speed,
            player.orientation.z.z * player_speed - forward.z * speed,
        );
        Some((look_at, desired_speed, relative.dot(relative).sqrt()))
    }

    pub(in crate::flight) fn work_hold_input(
        &self,
        target: WorkTarget,
        input: crate::FlightInput,
    ) -> crate::FlightInput {
        let Some(stand_off) = self.work_standoff(target) else {
            return input;
        };
        let Some(player) = crate::flight::flight_pose(self.state.position, self.state.orientation)
        else {
            return input;
        };
        let to_target = difference(target.position, player.position);
        let distance = to_target.dot(to_target).sqrt();
        let separation_error = distance - stand_off;
        let collecting = matches!(target.id, WorkTargetId::Ore(_));
        let retreating = !collecting && separation_error < -0.25;
        let room = if retreating {
            -separation_error
        } else {
            separation_error.max(0.0)
        };
        let direction = if retreating { -1.0 } else { 1.0 };
        let facing = direction * to_target.dot(player.orientation.z) > distance * 0.995;
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
        // Match the original intake speed gate as we enter the fragment envelope.
        let desired = if collecting && distance < 150.0 {
            desired.min(8.0 / units)
        } else {
            desired
        };
        let desired = if let Some((point, speed, _)) = self.moving_work_course(target, player) {
            let direction = difference(point, player.position);
            if direction.dot(player.orientation.z) >= direction.dot(direction).sqrt() * 0.98 {
                speed
            } else {
                0.0
            }
        } else {
            desired
        };
        let throttle = ((desired - f64::from(self.state.speed)) / (acceleration * delta_t / units))
            .clamp(-1.0, 1.0) as f32;
        crate::FlightInput { throttle, ..input }
    }
}

#[cfg(test)]
mod tests;

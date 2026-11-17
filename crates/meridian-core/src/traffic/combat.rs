use super::*;
impl Resources {
    pub(super) fn advance_many(&mut self, ticks: u64) -> Result<(), TrafficError> {
        let energy = (38000 - u64::from(self.energy)).min(ticks.saturating_mul(7));
        let front = (30000 - u64::from(self.front)).min(ticks.saturating_mul(3));
        let rear = (30000 - u64::from(self.rear)).min(ticks.saturating_mul(3));
        let cool = u64::from(self.heat).min(ticks);
        self.energy += energy as u16;
        self.front += front as u16;
        self.rear += rear as u16;
        self.heat -= cool as u16;
        self.cooldown = u64::from(self.cooldown).saturating_sub(ticks) as u8;
        for (total, delta) in [
            (&mut self.energy_regen, energy),
            (&mut self.front_regen, front),
            (&mut self.rear_regen, rear),
            (&mut self.heat_cooled, cool),
        ] {
            *total = total
                .checked_add(delta)
                .ok_or(TrafficError::SequenceExhausted)?;
        }
        Ok(())
    }
    pub(super) fn advance(&mut self) -> Result<(), TrafficError> {
        self.advance_many(1)
    }
    pub(super) fn fire(&mut self) -> Result<(), TrafficError> {
        if self.energy <= 100 || self.heat > 1150 || self.cooldown != 0 {
            return Err(TrafficError::InvalidState);
        }
        self.energy -= 100;
        self.heat += 50;
        self.cooldown = 18;
        self.shots = self
            .shots
            .checked_add(1)
            .ok_or(TrafficError::SequenceExhausted)?;
        Ok(())
    }
    pub(super) fn hit(&mut self, front: bool) -> Result<(), TrafficError> {
        let shield = if front {
            &mut self.front
        } else {
            &mut self.rear
        };
        let absorbed = (*shield).min(1500);
        *shield -= absorbed;
        let total = if front {
            &mut self.absorbed_front
        } else {
            &mut self.absorbed_rear
        };
        *total = total
            .checked_add(u64::from(absorbed))
            .ok_or(TrafficError::SequenceExhausted)?;
        let remainder = 1500 - absorbed;
        let damage = self.energy.min(remainder);
        self.energy -= damage;
        self.hull_damage = self
            .hull_damage
            .checked_add(u64::from(damage))
            .ok_or(TrafficError::SequenceExhausted)?;
        self.overkill = self
            .overkill
            .checked_add(u64::from(remainder - damage))
            .ok_or(TrafficError::SequenceExhausted)?;
        self.incoming = self
            .incoming
            .checked_add(1)
            .ok_or(TrafficError::SequenceExhausted)?;
        Ok(())
    }
    pub(super) fn validate(self, age: u64, shots: usize, incoming: usize) -> Result<(), SaveError> {
        if self.energy > 38_000
            || self.front > 30_000
            || self.rear > 30_000
            || self.heat > 1200
            || self.cooldown > 18
            || usize::from(self.shots) != shots
            || usize::from(self.incoming) != incoming
            || u128::from(self.energy) + u128::from(self.shots) * 100 + u128::from(self.hull_damage)
                != 38_000 + u128::from(self.energy_regen)
            || u128::from(self.front) + u128::from(self.absorbed_front)
                != 30_000 + u128::from(self.front_regen)
            || u128::from(self.rear) + u128::from(self.absorbed_rear)
                != 30_000 + u128::from(self.rear_regen)
            || u128::from(self.heat) + u128::from(self.heat_cooled) != u128::from(self.shots) * 50
            || u128::from(self.absorbed_front)
                + u128::from(self.absorbed_rear)
                + u128::from(self.hull_damage)
                + u128::from(self.overkill)
                != u128::from(self.incoming) * 1500 + u128::from(self.incoming_missiles) * 16000
            || u128::from(self.energy_regen) > u128::from(age) * 7
            || u128::from(self.front_regen) > u128::from(age) * 3
            || u128::from(self.rear_regen) > u128::from(age) * 3
            || self.heat_cooled > age
        {
            return Err(INVALID);
        }
        Ok(())
    }
}
impl State {
    pub(super) fn fire_npc_beams(
        &mut self,
        system: SystemId,
        data: &OriginalStationData,
        player: RigidPose,
        world_tick: u64,
        beam_clear: &dyn Fn(Vec3, Vec3, f64) -> bool,
        player_target: Option<PlayerTarget>,
    ) -> Result<(), TrafficError> {
        let Some(si) = system_index(system) else {
            return Ok(());
        };
        for index in 0..MAX_TRAFFIC_ACTORS {
            if self.receipt_count() >= MAX_TRAFFIC_SHOTS {
                break;
            }
            let Some(actor) = self.actors[index].filter(|a| {
                a.id.system == system
                    && a.id.serial == 2
                    && a.phase == NpcPhase::Hunting
                    && a.resources.cooldown == 0
                    && a.resources.energy > 100
                    && a.resources.heat <= 1150
            }) else {
                continue;
            };
            if let Some(target) = player_target
                && self.player_goal(actor, player.position, target).is_some()
            {
                self.fire_at_player(
                    index,
                    actor,
                    super::player_hits::PlayerEmission {
                        data,
                        player,
                        target,
                        world_tick,
                        beam_clear,
                    },
                )?;
                continue;
            }
            let Some(target) = super::motion::target(actor, &self.actors) else {
                continue;
            };
            let direction = actor.pose().orientation.z;
            let delta = sub(target.position, actor.position);
            if unit(delta).is_none_or(|aim| aim.dot(direction) < 0.995) {
                continue;
            }
            let Some(contact) = self
                .beam_except(
                    data,
                    system,
                    actor.position,
                    direction,
                    6000.0,
                    Some(actor.id),
                )
                .filter(|c| c.id == target.id)
            else {
                continue;
            };
            let station = data
                .beam_block_distance(data.station_pose(), actor.position, direction, 6000.0)
                .map_err(|_| TrafficError::InvalidState)?;
            let player_hit = data
                .player_beam_distance(player, actor.position, direction, 6000.0)
                .map_err(|_| TrafficError::InvalidState)?;
            if station.is_some_and(|d| d <= contact.distance)
                || player_hit.is_some_and(|d| d <= contact.distance)
                || !beam_clear(actor.position, direction, contact.distance)
            {
                continue;
            }
            let front = target
                .pose()
                .orientation
                .z
                .dot(sub(actor.position, target.position))
                >= 0.0;
            let beam = TrafficBeam {
                source: BeamSource::Npc(actor.id),
                victim: target.id,
                world_tick,
                local_tick: self.clocks[si],
                origin: actor.position,
                direction,
                point: contact.point,
                victim_pose: target.pose(),
                front,
            };
            let mut attacker = actor;
            attacker.resources.fire()?;
            self.actors[index] = Some(attacker);
            self.apply_hit(beam)?;
        }
        Ok(())
    }
    pub(crate) fn prepare_player_hit(
        &self,
        data: &OriginalStationData,
        input: PlayerBeamInput,
    ) -> Result<Self, TrafficError> {
        let PlayerBeamInput {
            system,
            source,
            origin,
            direction,
            contact,
            world_tick,
        } = input;
        let mut next = self.clone();
        let index = next
            .actor_index(contact.id)
            .ok_or(TrafficError::InvalidState)?;
        let victim = next.actors[index].ok_or(TrafficError::InvalidState)?;
        if victim.phase == NpcPhase::Destroyed {
            return Ok(next);
        }
        if self.receipt_count() >= MAX_TRAFFIC_SHOTS {
            return Err(TrafficError::ReceiptCapacity);
        }
        if !matches!(source, BeamSource::Player { .. })
            || contact.id.system != system
            || self.beam(data, system, origin, direction, 6000.0) != Some(contact)
        {
            return Err(TrafficError::InvalidState);
        }
        let i = system_index(system).ok_or(TrafficError::InvalidState)?;
        let front = victim
            .pose()
            .orientation
            .z
            .dot(sub(origin, victim.position))
            >= 0.0;
        next.apply_hit(TrafficBeam {
            source,
            victim: contact.id,
            world_tick,
            local_tick: self.clocks[i],
            origin,
            direction,
            point: contact.point,
            victim_pose: victim.pose(),
            front,
        })?;
        Ok(next)
    }
    fn apply_hit(&mut self, beam: TrafficBeam) -> Result<(), TrafficError> {
        if self.receipt_count() >= MAX_TRAFFIC_SHOTS {
            return Err(TrafficError::ReceiptCapacity);
        }
        let index = self
            .actor_index(beam.victim)
            .ok_or(TrafficError::InvalidState)?;
        let mut actor = self.actors[index].ok_or(TrafficError::InvalidState)?;
        if !actor.airborne() {
            return Err(TrafficError::InvalidState);
        }
        actor.resources.hit(beam.front)?;
        if let BeamSource::Npc(id) = beam.source {
            actor.aggressor = Some(id);
        }
        if actor.resources.energy == 0 {
            actor.phase = NpcPhase::Destroyed;
            actor.speed = 0.0;
        }
        self.actors[index] = Some(actor);
        Arc::make_mut(&mut self.shots).push(beam);
        Ok(())
    }
}

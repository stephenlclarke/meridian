use super::{
    Actor, BeamSource, BerthReceipt, MAX_TRAFFIC_ACTORS, NpcPhase, State, StationMarkets, SystemId,
    TickContext, TrafficBeam, TrafficError, Vec3, add, length, pose, scale, segment_distance, sub,
    system_index, unit,
};
impl State {
    /// One local physical tick. Inactive systems and docked/paused player time do not simulate
    /// unobserved intersystem journeys. Completed/dead identities remain, with no respawn.
    pub(crate) fn prepare_step(
        &self,
        context: TickContext<'_>,
        markets: &StationMarkets,
    ) -> Result<(Self, StationMarkets), TrafficError> {
        let TickContext {
            system,
            data,
            world_tick,
            protected,
            beam_clear,
            ..
        } = context;
        let mut next = self.clone();
        let mut market = markets.traffic_candidate();
        let Some(i) = system_index(system).filter(|_| self.enabled) else {
            return Ok((next, market));
        };
        if protected {
            return Ok((next, market));
        }
        next.clocks[i] = next.clocks[i]
            .checked_add(1)
            .ok_or(TrafficError::SequenceExhausted)?;
        next.birth(&context, &mut market)?;
        let opening = next.actors;
        for index in 0..MAX_TRAFFIC_ACTORS {
            let Some(actor) = opening[index].filter(|a| a.id.system == system && a.airborne())
            else {
                continue;
            };
            let actor = move_actor(
                actor,
                &opening,
                &context,
                &next.shots,
                next.clocks[i],
                next.missile_hits.iter().any(|h| h.victim == actor.id),
            )?;
            next.actors[index] = Some(actor);
        }
        resolve_peer_paths(
            &opening,
            &mut next.actors,
            system,
            data.player_clearance_radius(),
        );
        for actor in
            next.actors.iter_mut().flatten().filter(|a| {
                a.id.system == system && a.phase == NpcPhase::Inbound && a.berth.is_some()
            })
        {
            State::finish_trade(actor, &mut market, world_tick)?;
        }
        next.fire_npc_beams(
            system,
            data,
            context.player_after,
            world_tick,
            beam_clear,
            context.player_target,
        )?;
        Ok((next, market))
    }
}
fn move_actor(
    mut actor: Actor,
    opening: &[Option<Actor>; MAX_TRAFFIC_ACTORS],
    context: &TickContext<'_>,
    shots: &[TrafficBeam],
    local_tick: u64,
    missile_aggression: bool,
) -> Result<Actor, TrafficError> {
    let opening_actor = actor;
    let TickContext {
        system,
        data,
        player_before,
        player_after,
        celestial_clear,
        ..
    } = *context;
    actor.resources.advance()?;
    actor.resource_clock = local_tick;
    if !actor.resources.emp_disabled()
        && actor.id.serial == 1
        && (threat_position(
            actor,
            opening,
            player_after.position,
            shots,
            missile_aggression,
        )
        .is_some()
            || (actor.phase == NpcPhase::FinalAlign
                && length(sub(actor.position, Vec3::new(0.0, 0.0, 3500.0))) > 1.0))
    {
        actor.phase = NpcPhase::Approach;
    }
    let player_goal = context.player_target.and_then(|target| {
        State::player_goal_in(
            actor,
            opening,
            player_after.position,
            target,
            shots,
            missile_aggression,
        )
    });
    let (mut goal, mut speed) = if actor.resources.emp_disabled() {
        (
            add(actor.position, scale(actor.pose().orientation.z, 100000.0)),
            actor.speed,
        )
    } else {
        player_goal.map_or_else(
            || {
                intent(
                    actor,
                    opening,
                    player_after.position,
                    shots,
                    missile_aggression,
                )
            },
            |goal| {
                (
                    goal,
                    if length(sub(goal, actor.position)) < 800.0 {
                        0.0
                    } else {
                        34.3
                    },
                )
            },
        )
    };
    let yielding = if actor.resources.emp_disabled() {
        None
    } else {
        super::admission::yield_goal(actor, goal, context)
    };
    if let Some(detour) = yielding {
        goal = detour;
        speed = speed.min(length(sub(goal, actor.position)) * 0.65 / (18.0 * 4.0));
    } else if !actor.resources.emp_disabled()
        && actor.id.serial == 2
        && actor.resources.energy >= 9500
    {
        let target_pose = if player_goal.is_some() {
            Some(player_after)
        } else {
            target(actor, opening).map(Actor::pose)
        };
        if let Some(target_pose) = target_pose
            && let Some(point) = data
                .player_planar_aimpoint(target_pose, actor.position)
                .map_err(|_| TrafficError::InvalidState)?
        {
            goal = point;
        }
    }
    let delta = sub(goal, actor.position);
    let desired = delta.x.atan2(delta.z);
    let error = (desired - actor.yaw)
        .sin()
        .atan2((desired - actor.yaw).cos());
    if error.abs() > 0.25 {
        speed = speed.min(8.0);
    }
    if !actor.resources.emp_disabled() && actor.id.serial == 1 && actor.phase != NpcPhase::Inbound {
        speed = speed.min(length(delta) * 0.65 / (18.0 * 4.0));
    }
    let turn = if actor.resources.emp_disabled() {
        0.0
    } else {
        error.clamp(-0.65 / 60.0, 0.65 / 60.0)
    };
    let yaw = actor.yaw + turn;
    let yaw = yaw.sin().atan2(yaw.cos());
    let rate = 9.1 / 60.0;
    let velocity = if actor.resources.emp_disabled() {
        actor.speed
    } else {
        (actor.speed + (speed - actor.speed).clamp(-rate, rate)).clamp(0.0, 34.3)
    };
    let after = pose(
        add(
            actor.position,
            scale(
                pose(actor.position, yaw).orientation.z,
                velocity * 18.0 / 60.0,
            ),
        ),
        yaw,
    );
    let before = actor.pose();
    let radius = data.player_clearance_radius();
    let peers_clear = opening
        .iter()
        .flatten()
        .filter(|b| b.id != actor.id && b.id.system == system && b.solid())
        .all(|b| {
            segment_distance(before.position, after.position, b.position) > radius * 2.0 + 1.0
        });
    let relative_clear = segment_distance(
        sub(before.position, player_before.position),
        sub(after.position, player_after.position),
        Vec3::new(0.0, 0.0, 0.0),
    ) > radius * 2.0 + 1.0;
    let entry = actor.id.serial == 1
        && actor.phase == NpcPhase::Inbound
        && data.crosses_aperture(before, after);
    let station_clear = if entry {
        data.entry_clear(before, after, velocity * 18.0, 0.0, turn.abs())
    } else {
        data.motion_clear(before, after, turn.abs())
    };
    let arrival_clear = segment_distance(
        before.position,
        after.position,
        Vec3::new(0.0, 0.0, 16000.0),
    ) > radius * 2.0 + 64.0;
    if peers_clear
        && relative_clear
        && arrival_clear
        && super::admission::lane_clear(before.position, after.position, context)
        && station_clear
        && celestial_clear(before.position, after.position, radius)
    {
        actor.position = after.position;
        actor.yaw = yaw;
        actor.speed = velocity;
        if entry {
            actor.berth = Some(BerthReceipt {
                before: before.position,
                yaw: opening_actor.yaw,
                speed: velocity,
                angular: turn.abs(),
            });
        }
    } else {
        actor.speed = 0.0;
    }
    if !actor.resources.emp_disabled() {
        advance_route(&mut actor);
    }
    Ok(actor)
}
fn resolve_peer_paths(
    opening: &[Option<Actor>; MAX_TRAFFIC_ACTORS],
    actors: &mut [Option<Actor>; MAX_TRAFFIC_ACTORS],
    system: SystemId,
    radius: f64,
) {
    // Proposed peer paths may cross even when both endpoints clear the opening bodies.
    for a in 0..MAX_TRAFFIC_ACTORS {
        for b in a + 1..MAX_TRAFFIC_ACTORS {
            if let (Some(oa), Some(ob), Some(na), Some(nb)) =
                (opening[a], opening[b], actors[a], actors[b])
                && oa.id.system == system
                && ob.id.system == system
                && oa.airborne()
                && ob.airborne()
                && na.airborne()
                && nb.airborne()
                && segment_distance(
                    sub(oa.position, ob.position),
                    sub(na.position, nb.position),
                    Vec3::new(0.0, 0.0, 0.0),
                ) <= radius * 2.0 + 1.0
            {
                let mut na = na;
                let mut nb = nb;
                na.phase = oa.phase;
                na.berth = oa.berth;
                na.position = oa.position;
                na.yaw = oa.yaw;
                na.speed = 0.0;
                nb.phase = ob.phase;
                nb.berth = ob.berth;
                nb.position = ob.position;
                nb.yaw = ob.yaw;
                nb.speed = 0.0;
                actors[a] = Some(na);
                actors[b] = Some(nb);
            }
        }
    }
}
fn intent(
    a: Actor,
    actors: &[Option<Actor>; MAX_TRAFFIC_ACTORS],
    player: Vec3,
    shots: &[TrafficBeam],
    missile_aggression: bool,
) -> (Vec3, f64) {
    if a.id.serial == 1 {
        if let Some(threat) = threat_position(a, actors, player, shots, missile_aggression) {
            let away = unit(Vec3::new(
                a.position.x - threat.x,
                0.0,
                a.position.z - threat.z,
            ))
            .unwrap_or(Vec3::new(1.0, 0.0, 0.0));
            return (add(a.position, scale(away, 6000.0)), 34.3);
        }
        match a.phase {
            NpcPhase::Departing => (Vec3::new(0.0, 0.0, 3500.0), 24.0),
            NpcPhase::Outbound => (Vec3::new(2400.0, 0.0, 9000.0), 34.3),
            NpcPhase::ReturnAlign => (Vec3::new(0.0, 0.0, 9000.0), 24.0),
            NpcPhase::Approach => (Vec3::new(0.0, 0.0, 3500.0), 24.0),
            NpcPhase::FinalAlign => (add(a.position, Vec3::new(0.0, 0.0, -1000.0)), 0.0),
            NpcPhase::Inbound => (
                Vec3::new(0.0, 0.0, -300.0),
                if a.position.z < 3500.0 { 4.0 } else { 24.0 },
            ),
            _ => (a.position, 0.0),
        }
    } else if a.resources.energy < 9500 {
        let goal = Vec3::new(2400.0, 0.0, 16000.0);
        (
            goal,
            if length(sub(goal, a.position)) < 300.0 {
                0.0
            } else {
                34.3
            },
        )
    } else if let Some(target) = target(a, actors) {
        (
            target.position,
            if length(sub(target.position, a.position)) < 800.0 {
                0.0
            } else {
                34.3
            },
        )
    } else {
        let goal = Vec3::new(2400.0, 0.0, 16000.0);
        (
            goal,
            if length(sub(goal, a.position)) < 300.0 {
                0.0
            } else {
                24.0
            },
        )
    }
}
fn threat_position(
    a: Actor,
    actors: &[Option<Actor>; MAX_TRAFFIC_ACTORS],
    player: Vec3,
    shots: &[TrafficBeam],
    missile_aggression: bool,
) -> Option<Vec3> {
    if let Some(threat) = a.aggressor.and_then(|id| {
        actors.iter().flatten().find(|b| {
            b.id == id
                && b.airborne()
                && !b.resources.emp_disabled()
                && length(sub(b.position, a.position)) <= 6000.0
        })
    }) {
        return Some(threat.position);
    }
    if missile_aggression && length(sub(player, a.position)) <= 6000.0 {
        return Some(player);
    }
    shots
        .iter()
        .rfind(|s| s.victim == a.id)
        .filter(|s| {
            matches!(s.source, BeamSource::Player { .. })
                && length(sub(player, a.position)) <= 6000.0
        })
        .map(|_| player)
}
fn advance_route(a: &mut Actor) {
    if a.id.serial == 2 {
        a.phase = if a.resources.energy < 9500 {
            NpcPhase::Retreating
        } else {
            NpcPhase::Hunting
        };
        return;
    }
    a.phase = match a.phase {
        NpcPhase::Departing if a.position.z >= 3300.0 => NpcPhase::Outbound,
        NpcPhase::Outbound if length(sub(a.position, Vec3::new(2400.0, 0.0, 9000.0))) < 350.0 => {
            NpcPhase::ReturnAlign
        }
        NpcPhase::ReturnAlign if length(sub(a.position, Vec3::new(0.0, 0.0, 9000.0))) < 200.0 => {
            NpcPhase::Approach
        }
        NpcPhase::Approach if length(sub(a.position, Vec3::new(0.0, 0.0, 3500.0))) < 0.5 => {
            NpcPhase::FinalAlign
        }
        NpcPhase::FinalAlign if a.yaw.sin().abs() < 1e-6 && a.yaw.cos() < 0.0 && a.speed == 0.0 => {
            NpcPhase::Inbound
        }
        phase => phase,
    };
}
pub(super) fn target(a: Actor, actors: &[Option<Actor>; MAX_TRAFFIC_ACTORS]) -> Option<Actor> {
    actors
        .iter()
        .flatten()
        .filter(|b| {
            b.id != a.id
                && b.id.system == a.id.system
                && b.id.serial == 1
                && b.airborne()
                && !b.resources.emp_disabled()
                && !b.sold
                && b.units > 0
                && length(sub(b.position, a.position)) <= 12000.0
        })
        .min_by(|a, b| a.id.serial.cmp(&b.id.serial))
        .copied()
}

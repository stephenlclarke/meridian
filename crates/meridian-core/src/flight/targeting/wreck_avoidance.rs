//! Ordinary brake-and-turn collision avoidance for a stationary, authorised wreck.
//! This predicts only clearance from the passive recovery obstacle. The standard
//! swept traffic, asteroid, station and celestial gates still own every real movement.
use super::super::{
    ACCELERATION, DISTANCE_PER_SPEED_UNIT, FIXED_DELTA_SECONDS, MAX_SPEED, PITCH_RATE, ROLL_RATE,
    YAW_RATE, axis_rotation, forward, multiply, normalize,
};
use crate::docking::Vec3;
use crate::{FlightInput, Snapshot};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Steering {
    yaw: f32,
    pitch: f32,
}

impl Steering {
    const STRAIGHT: Self = Self {
        yaw: 0.0,
        pitch: 0.0,
    };

    fn brake(self) -> FlightInput {
        FlightInput {
            yaw: self.yaw,
            pitch: self.pitch,
            brake: true,
            ..FlightInput::default()
        }
    }
}

#[derive(Clone, Copy)]
struct Motion {
    position: [f32; 3],
    orientation: [f32; 4],
    speed: f32,
}

impl From<Snapshot> for Motion {
    fn from(snapshot: Snapshot) -> Self {
        Self {
            position: snapshot.position,
            orientation: snapshot.orientation,
            speed: snapshot.speed,
        }
    }
}

fn point(position: [f32; 3]) -> Vec3 {
    Vec3::new(
        f64::from(position[0]),
        f64::from(position[1]),
        f64::from(position[2]),
    )
}

fn minus(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn segment_clearance(start: Vec3, finish: Vec3, centre: Vec3) -> f64 {
    let travel = minus(finish, start);
    let from_centre = minus(start, centre);
    let fraction = if travel.dot(travel) == 0.0 {
        0.0
    } else {
        (-from_centre.dot(travel) / travel.dot(travel)).clamp(0.0, 1.0)
    };
    let nearest = Vec3::new(
        from_centre.x + fraction * travel.x,
        from_centre.y + fraction * travel.y,
        from_centre.z + fraction * travel.z,
    );
    nearest.dot(nearest).sqrt()
}

impl Motion {
    fn advance(self, input: FlightInput) -> Option<Self> {
        // Identical normal flight axis order, decrement and integration. No special
        // speed cap, emergency deceleration, collision exemption or position write.
        let yaw = axis_rotation(1, input.yaw * YAW_RATE * FIXED_DELTA_SECONDS);
        let pitch = axis_rotation(0, -input.pitch * PITCH_RATE * FIXED_DELTA_SECONDS);
        let roll = axis_rotation(2, input.roll * ROLL_RATE * FIXED_DELTA_SECONDS);
        let orientation = normalize(multiply(
            multiply(multiply(self.orientation, yaw), pitch),
            roll,
        ))?;
        let throttle = if input.brake { -1.0 } else { input.throttle };
        let speed =
            (self.speed + throttle * ACCELERATION * FIXED_DELTA_SECONDS).clamp(0.0, MAX_SPEED);
        let mut position = self.position;
        for (axis, direction) in position.iter_mut().zip(forward(orientation)) {
            *axis += direction * speed * FIXED_DELTA_SECONDS * DISTANCE_PER_SPEED_UNIT;
        }
        Some(Self {
            position,
            orientation,
            speed,
        })
    }

    fn straight_stop_clearance(self, centre: Vec3) -> f64 {
        let start = point(self.position);
        let direction = point(forward(self.orientation));
        // Continuous braking distance is a conservative upper bound for the real
        // discrete integrator (which applies the decrement before translation).
        let distance = f64::from(self.speed).powi(2) * f64::from(DISTANCE_PER_SPEED_UNIT)
            / (2.0 * f64::from(ACCELERATION));
        let end = Vec3::new(
            start.x + direction.x * distance,
            start.y + direction.y * distance,
            start.z + direction.z * distance,
        );
        segment_clearance(start, end, centre)
    }
}

fn normal_clearance(motion: Motion, centre: Vec3, desired: FlightInput) -> Option<f64> {
    let next = motion.advance(desired)?;
    Some(
        segment_clearance(point(motion.position), point(next.position), centre)
            .min(next.straight_stop_clearance(centre)),
    )
}

fn braking_input(motion: Motion, centre: Vec3, boundary: f64, turn: Steering) -> FlightInput {
    // Once the remaining straight brake is clear, freeze the attitude until stopped.
    // Do not turn back towards the obstacle while still carrying the overspeed.
    if motion.straight_stop_clearance(centre) > boundary {
        Steering::STRAIGHT.brake()
    } else {
        turn.brake()
    }
}

fn brake_forecast(mut motion: Motion, centre: Vec3, boundary: f64, turn: Steering) -> Option<f64> {
    let mut clearance = segment_clearance(point(motion.position), point(motion.position), centre);
    // At the fixed reference speed and decrement, even maximum-speed braking is below
    // this bound. Exhausting the bound refuses a claim of a clear forecast.
    let limit = (MAX_SPEED / (ACCELERATION * FIXED_DELTA_SECONDS)).ceil() as usize + 2;
    for _ in 0..limit {
        if motion.speed <= 0.0 {
            return Some(clearance);
        }
        let next = motion.advance(braking_input(motion, centre, boundary, turn))?;
        clearance = clearance.min(segment_clearance(
            point(motion.position),
            point(next.position),
            centre,
        ));
        motion = next;
    }
    None
}

fn choose_turn(motion: Motion, centre: Vec3, boundary: f64) -> Steering {
    // Use only ordinary yaw/pitch inputs. If no manoeuvre can avoid the obstacle,
    // still brake immediately and leave final admission to the swept collision owner.
    let mut best = Steering::STRAIGHT;
    let mut best_clearance = f64::NEG_INFINITY;
    for (yaw, pitch) in [
        (0.0, 0.0),
        (-1.0, 0.0),
        (1.0, 0.0),
        (0.0, -1.0),
        (0.0, 1.0),
        (-1.0, -1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (1.0, 1.0),
    ] {
        let turn = Steering { yaw, pitch };
        let clearance = brake_forecast(motion, centre, boundary, turn).unwrap_or(f64::NEG_INFINITY);
        if clearance > best_clearance {
            best = turn;
            best_clearance = clearance;
        }
    }
    best
}

fn decide(
    latched: &mut Option<Steering>,
    motion: Motion,
    centre: Vec3,
    standoff: f64,
    desired: FlightInput,
) -> FlightInput {
    // Authoritative swept traffic requires distance strictly greater than standoff-1.
    // Keep half the ordinary 1-unit margin for conservative prediction and float rounding.
    let opening = segment_clearance(point(motion.position), point(motion.position), centre);
    let boundary = (standoff - 0.5).min((opening - 0.001).max(standoff - 1.0));
    if motion.speed <= 0.0 {
        *latched = None;
    }
    if let Some(turn) = *latched {
        return braking_input(motion, centre, boundary, turn);
    }
    if normal_clearance(motion, centre, desired).is_some_and(|clearance| clearance > boundary) {
        return desired;
    }
    if motion.speed <= 0.0 {
        // Already stopped: allow normal attitude correction, but never accelerate into
        // an uncleared predicted segment. This avoids trapping a stationary return pose.
        return FlightInput {
            throttle: 0.0,
            brake: true,
            ..desired
        };
    }
    let turn = choose_turn(motion, centre, boundary);
    *latched = Some(turn);
    braking_input(motion, centre, boundary, turn)
}

pub(super) fn protect(
    latched: &mut Option<Steering>,
    snapshot: Snapshot,
    centre: Vec3,
    standoff: f64,
    desired: FlightInput,
) -> FlightInput {
    decide(latched, snapshot.into(), centre, standoff, desired)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn incident_motion() -> (Motion, Vec3) {
        // Exact Dell earned recovery trace at tick 39716 (xyzw quaternion).
        let position = [1572.6443, 0.0, 6524.8066];
        let centre = Vec3::new(1815.1940, 0.0, 7101.6332);
        let orientation = [0.0, 0.19771095, 0.0, 0.9802604];
        (
            Motion {
                position,
                orientation,
                speed: MAX_SPEED,
            },
            centre,
        )
    }

    #[test]
    fn inward_overspeed_brakes_and_turns_clear_using_only_ordinary_motion() {
        let (mut motion, centre) = incident_motion();
        let standoff = 2.0 * 72.5470172564567 + 2.0;
        assert!(motion.straight_stop_clearance(centre) < standoff - 1.0);
        let mut latched = None;
        let desired = FlightInput {
            brake: true,
            ..FlightInput::default()
        };
        let initial = decide(&mut latched, motion, centre, standoff, desired);
        assert!(initial.brake && (initial.yaw != 0.0 || initial.pitch != 0.0));
        for _ in 0..300 {
            if motion.speed <= 0.0 {
                break;
            }
            let command = decide(&mut latched, motion, centre, standoff, desired);
            assert!(command.brake);
            let next = motion.advance(command).unwrap();
            assert_eq!(
                next.speed,
                (motion.speed - ACCELERATION * FIXED_DELTA_SECONDS).max(0.0)
            );
            assert!(
                segment_clearance(point(motion.position), point(next.position), centre)
                    > standoff - 1.0
            );
            motion = next;
        }
        assert_eq!(motion.speed, 0.0);
        assert_eq!(
            decide(&mut latched, motion, centre, standoff, desired),
            desired
        );
        assert!(latched.is_none());
    }

    #[test]
    fn already_unavoidable_next_sweep_does_not_manufacture_a_stop() {
        let centre = Vec3::new(0.0, 0.0, 154.0);
        let motion = Motion {
            position: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0],
            speed: MAX_SPEED,
        };
        let mut latched = None;
        let command = decide(&mut latched, motion, centre, 154.0, FlightInput::default());
        let next = motion.advance(command).unwrap();
        assert!(command.brake);
        assert!(next.speed > 0.0);
        assert!(segment_clearance(point(motion.position), point(next.position), centre) < 153.0);
    }

    #[test]
    fn normal_slow_clear_approach_keeps_ordinary_controller_input() {
        let (mut motion, centre) = incident_motion();
        motion.speed = 1.0;
        let desired = FlightInput {
            throttle: 0.1,
            ..FlightInput::default()
        };
        let mut latched = None;
        assert_eq!(
            decide(&mut latched, motion, centre, 154.0, desired),
            desired
        );
        assert!(latched.is_none());
    }

    #[test]
    fn cancelling_targeting_clears_transient_avoidance() {
        let mut world = crate::World::new_commander(41001).unwrap();
        world.targeting.wreck_avoidance = Some(Steering {
            yaw: 1.0,
            pitch: 0.0,
        });
        world.cancel_targeting();
        assert!(world.targeting.wreck_avoidance.is_none());
        assert!(world.automatic_work_status().is_none());
    }
}

use core::fmt;

pub const FIXED_DELTA_SECONDS: f32 = 1.0 / 60.0;

// SH025 [game], immutable legacy a11fe16924d87d06021816f9037952afcbac08db.
// These are reference gameplay units, not an asserted SI or renderer-space calibration.
const MAX_SPEED: f32 = 34.3;
const ACCELERATION: f32 = 9.1;
const PITCH_RATE: f32 = 0.7;
const YAW_RATE: f32 = 0.65;
const ROLL_RATE: f32 = 0.79;

/// Held controls. Zero throttle coasts; negative throttle slows without reversing.
/// Positive pitch raises the nose, positive yaw turns toward local +X, and roll is about +Z.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct FlightInput {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub throttle: f32,
    pub brake: bool,
}

impl FlightInput {
    fn validate(self) -> Result<(), StepError> {
        let controls = [self.pitch, self.yaw, self.roll, self.throttle];
        if controls.iter().any(|value| !value.is_finite()) {
            return Err(StepError::NonFiniteInput);
        }
        if controls.iter().any(|value| !(-1.0..=1.0).contains(value)) {
            return Err(StepError::InputOutOfRange);
        }
        Ok(())
    }
}

/// Presentation value only. Quaternion order is xyzw, local forward +Z and up +Y.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
    pub position: [f32; 3],
    pub orientation: [f32; 4],
    pub speed: f32,
    pub tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepError {
    NonFiniteInput,
    InputOutOfRange,
    TickOverflow,
    NonFiniteState,
}

impl fmt::Display for StepError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFiniteInput => "flight input must be finite",
            Self::InputOutOfRange => "held flight controls must be within [-1, 1]",
            Self::TickOverflow => "simulation tick exhausted",
            Self::NonFiniteState => "flight step cannot produce a valid finite pose",
        })
    }
}

impl std::error::Error for StepError {}

/// One owner mutates World; callers receive small immutable presentation values.
#[derive(Debug)]
pub struct World {
    state: Snapshot,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            state: Snapshot {
                position: [0.0, 0.0, -600.0],
                orientation: [0.0, 0.0, 0.0, 1.0],
                speed: 0.0,
                tick: 0,
            },
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.state
    }

    /// Advance exactly one 1/60-second tick. Any rejection leaves World unchanged.
    /// Rotation is local yaw, then pitch, then roll; translation uses the new pose/speed.
    /// Brake overrides throttle and uses the same reference deceleration magnitude.
    pub fn step(&mut self, input: FlightInput) -> Result<(), StepError> {
        input.validate()?;
        let mut next = self.state;
        next.tick = next.tick.checked_add(1).ok_or(StepError::TickOverflow)?;

        let yaw = axis_rotation(1, input.yaw * YAW_RATE * FIXED_DELTA_SECONDS);
        let pitch = axis_rotation(0, -input.pitch * PITCH_RATE * FIXED_DELTA_SECONDS);
        let roll = axis_rotation(2, input.roll * ROLL_RATE * FIXED_DELTA_SECONDS);
        next.orientation = normalize(multiply(
            multiply(multiply(next.orientation, yaw), pitch),
            roll,
        ))
        .ok_or(StepError::NonFiniteState)?;

        let throttle = if input.brake { -1.0 } else { input.throttle };
        next.speed =
            (next.speed + throttle * ACCELERATION * FIXED_DELTA_SECONDS).clamp(0.0, MAX_SPEED);
        let forward = forward(next.orientation);
        for (position, direction) in next.position.iter_mut().zip(forward) {
            *position += direction * next.speed * FIXED_DELTA_SECONDS;
        }
        if !next.speed.is_finite() || next.position.iter().any(|value| !value.is_finite()) {
            return Err(StepError::NonFiniteState);
        }
        self.state = next;
        Ok(())
    }
}

fn axis_rotation(axis: usize, angle: f32) -> [f32; 4] {
    let (sin, cos) = (angle * 0.5).sin_cos();
    let mut quaternion = [0.0, 0.0, 0.0, cos];
    quaternion[axis] = sin;
    quaternion
}

fn multiply(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

fn normalize(quaternion: [f32; 4]) -> Option<[f32; 4]> {
    let squared_length: f32 = quaternion.iter().map(|value| value * value).sum();
    if !squared_length.is_finite() || squared_length <= f32::MIN_POSITIVE {
        return None;
    }
    let inverse_length = squared_length.sqrt().recip();
    let result = quaternion.map(|value| value * inverse_length);
    result
        .iter()
        .all(|value| value.is_finite())
        .then_some(result)
}

fn forward([x, y, z, w]: [f32; 4]) -> [f32; 3] {
    [
        2.0 * (x * z + w * y),
        2.0 * (y * z - w * x),
        1.0 - 2.0 * (x * x + y * y),
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn close(actual: f32, expected: f32, tolerance: f32) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} != {expected}"
        );
    }

    #[test]
    fn starts_stationary_facing_positive_z() {
        let mut world = World::new();
        let initial = world.snapshot();
        assert_eq!(initial.position, [0.0, 0.0, -600.0]);
        assert_eq!(initial.orientation, [0.0, 0.0, 0.0, 1.0]);
        world.step(FlightInput::default()).unwrap();
        assert_eq!(world.snapshot().position, initial.position);
        assert_eq!(world.snapshot().tick, 1);
    }

    #[test]
    fn invalid_controls_reject_the_whole_step_without_mutation() {
        let mut world = World::new();
        world
            .step(FlightInput {
                throttle: 1.0,
                ..FlightInput::default()
            })
            .unwrap();
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.01, -1.01] {
            for field in 0..4 {
                let mut input = FlightInput::default();
                match field {
                    0 => input.pitch = invalid,
                    1 => input.yaw = invalid,
                    2 => input.roll = invalid,
                    _ => input.throttle = invalid,
                }
                let before = world.snapshot();
                let expected = if invalid.is_finite() {
                    StepError::InputOutOfRange
                } else {
                    StepError::NonFiniteInput
                };
                assert_eq!(world.step(input), Err(expected));
                assert_eq!(world.snapshot(), before);
            }
        }
    }

    #[test]
    fn repeated_input_trace_is_deterministic() {
        let mut first = World::new();
        let mut second = World::new();
        for tick in 0..3600 {
            let input = FlightInput {
                pitch: if tick % 120 < 60 { 0.5 } else { -0.5 },
                yaw: 0.25,
                roll: -0.75,
                throttle: 0.8,
                brake: tick % 180 > 150,
            };
            first.step(input).unwrap();
            second.step(input).unwrap();
            assert_eq!(first.snapshot(), second.snapshot());
        }
    }

    #[test]
    fn combined_rotation_stays_normalized_and_finite() {
        let mut world = World::new();
        for _ in 0..10000 {
            world
                .step(FlightInput {
                    pitch: 1.0,
                    yaw: -1.0,
                    roll: 1.0,
                    throttle: 1.0,
                    brake: false,
                })
                .unwrap();
            let snapshot = world.snapshot();
            let norm: f32 = snapshot.orientation.iter().map(|v| v * v).sum();
            close(norm, 1.0, 0.000002);
            assert!(snapshot.position.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn reference_acceleration_reaches_bounded_speed_and_idle_coasts() {
        let mut world = World::new();
        let accelerate = FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        };
        for _ in 0..60 {
            world.step(accelerate).unwrap();
        }
        close(world.snapshot().speed, 9.1, 0.00002);
        assert_eq!(world.snapshot().position[0], 0.0);
        assert_eq!(world.snapshot().position[1], 0.0);
        assert!(world.snapshot().position[2] > -600.0);
        for _ in 0..600 {
            world.step(accelerate).unwrap();
        }
        close(world.snapshot().speed, 34.3, 0.00001);
        let cruising = world.snapshot();
        world.step(FlightInput::default()).unwrap();
        assert_eq!(world.snapshot().speed, cruising.speed);
        assert!(world.snapshot().position[2] > cruising.position[2]);
    }

    #[test]
    fn brake_overrides_throttle_stops_and_cannot_reverse() {
        let mut world = World::new();
        for _ in 0..120 {
            world
                .step(FlightInput {
                    throttle: 1.0,
                    ..FlightInput::default()
                })
                .unwrap();
        }
        let before = world.snapshot().speed;
        let brake = FlightInput {
            throttle: 1.0,
            brake: true,
            ..FlightInput::default()
        };
        world.step(brake).unwrap();
        close(world.snapshot().speed, before - 9.1 / 60.0, 0.00001);
        for _ in 0..300 {
            world.step(brake).unwrap();
        }
        assert_eq!(world.snapshot().speed, 0.0);
        let stopped = world.snapshot().position;
        world.step(brake).unwrap();
        assert_eq!(world.snapshot().position, stopped);
        world
            .step(FlightInput {
                throttle: -1.0,
                ..FlightInput::default()
            })
            .unwrap();
        assert_eq!(world.snapshot().speed, 0.0);
    }

    #[test]
    fn local_pitch_and_yaw_steer_forward_with_declared_signs() {
        let mut pitch = World::new();
        let mut yaw = World::new();
        for _ in 0..60 {
            pitch
                .step(FlightInput {
                    pitch: 1.0,
                    throttle: 1.0,
                    ..FlightInput::default()
                })
                .unwrap();
            yaw.step(FlightInput {
                yaw: 1.0,
                throttle: 1.0,
                ..FlightInput::default()
            })
            .unwrap();
        }
        assert!(pitch.snapshot().position[1] > 0.0);
        assert!(yaw.snapshot().position[0] > 0.0);
        assert_eq!(pitch.snapshot().position[0], 0.0);
        assert_eq!(yaw.snapshot().position[1], 0.0);
        close(
            pitch.snapshot().orientation[0],
            -(0.7_f32 / 2.0).sin(),
            0.000002,
        );
        close(
            yaw.snapshot().orientation[1],
            (0.65_f32 / 2.0).sin(),
            0.000002,
        );
    }

    #[test]
    fn overflow_or_nonfinite_candidate_does_not_publish() {
        let mut world = World::new();
        world.state.tick = u64::MAX;
        let before = world.snapshot();
        assert_eq!(
            world.step(FlightInput::default()),
            Err(StepError::TickOverflow)
        );
        assert_eq!(world.snapshot(), before);
        world.state.tick = 0;
        world.state.orientation = [0.0; 4];
        let before = world.snapshot();
        assert_eq!(
            world.step(FlightInput::default()),
            Err(StepError::NonFiniteState)
        );
        assert_eq!(world.snapshot(), before);
    }
}

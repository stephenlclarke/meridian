//! Explicit internal entry, no default directory/profile/save access or new actor authority.
use crate::profile::Profile;
use meridian_core::{FlightStart, OriginalStationData, World, mule::MuleHullSource};
use std::{ffi::OsString, io, path::PathBuf, time::Duration};

const DOCKING_TICK_LIMIT: u64 = 160;

/// Wall time inside renderer.draw for internal frames whose present call returned.
#[derive(Default)]
pub(super) struct FrameTiming {
    frames: u64,
    sum_ns: u128,
    range_ns: Option<(u128, u128)>,
}

impl FrameTiming {
    pub(super) fn observe(
        &mut self,
        result: &Result<crate::renderer::Presentation, String>,
        elapsed: Duration,
    ) {
        if !matches!(result, Ok(crate::renderer::Presentation::Presented)) {
            return;
        }
        let ns = elapsed.as_nanos();
        self.frames += 1;
        self.sum_ns += ns;
        let (min, max) = self.range_ns.unwrap_or((ns, ns));
        self.range_ns = Some((min.min(ns), max.max(ns)));
    }

    pub(super) fn report(&self) -> String {
        let (min, max) = self.range_ns.map_or_else(
            || ("null".into(), "null".into()),
            |(min, max)| (min.to_string(), max.to_string()),
        );
        format!(
            "MULE_PRESENTED_DRAW_WALL {{\"presentedFrames\":{},\"totalNanoseconds\":{},\"minNanoseconds\":{min},\"maxNanoseconds\":{max}}}",
            self.frames, self.sum_ns
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum View {
    Front,
    Rear,
    Far,
    Side,
    Dock,
    DockObstructed,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Launch {
    Profile(PathBuf),
    Mule(View),
}
impl Launch {
    pub(super) fn parse(args: impl Iterator<Item = OsString>) -> io::Result<Self> {
        let args: Vec<_> = args.collect();
        if args
            .first()
            .is_some_and(|a| a == "--internal-mule-validation")
        {
            let view = match args.get(1).and_then(|v| v.to_str()) {
                Some("front") => View::Front,
                Some("rear") => View::Rear,
                Some("far") => View::Far,
                Some("side") => View::Side,
                Some("dock") => View::Dock,
                Some("dock-obstructed") => View::DockObstructed,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Expected --internal-mule-validation front|rear|far|side|dock|dock-obstructed; no profile/save arguments",
                    ));
                }
            };
            if args.len() != 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Internal Mule validation cannot use a save directory or extra arguments",
                ));
            }
            return Ok(Self::Mule(view));
        }
        crate::profile::directory_from_args(args.into_iter()).map(Self::Profile)
    }
    pub(super) fn camera_view(&self) -> meridian_core::FlightView {
        if matches!(self, Self::Mule(View::Side)) {
            meridian_core::FlightView::Left
        } else {
            meridian_core::FlightView::Front
        }
    }
    pub(super) fn open(self) -> Result<Profile, Box<dyn std::error::Error>> {
        match self {
            Self::Profile(directory) => Profile::open(directory),
            Self::Mule(view) => {
                const SEED: u64 = 0x4d55_4c45;
                let world = match view {
                    View::Dock | View::DockObstructed => {
                        let (observer, approach) = docking_starts(view == View::DockObstructed)?;
                        World::mule_docking_validation_scenario(SEED, observer, approach)?
                    }
                    _ => World::mule_validation_scenario(SEED, observer(view)?)?,
                };
                Profile::mule_validation(world).map_err(|e| io::Error::other(e).into())
            }
        }
    }
}

/// Read the exact bundled SH025 native point radius, not a visual mesh surrogate.
/// Core's bundled station loader authenticates the same file before this is used.
fn observer_source() -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let bytes = include_bytes!("../../../../assets/legacy/sh025.physics");
    if bytes.get(..8) != Some(b"MPHYS001") {
        return Err(io::Error::other("Invalid observer source").into());
    }
    let count = u32::from_le_bytes(
        bytes
            .get(8..12)
            .ok_or_else(|| io::Error::other("Incomplete observer header"))?
            .try_into()?,
    ) as usize;
    let end = 16usize
        .checked_add(
            count
                .checked_mul(24)
                .ok_or_else(|| io::Error::other("Observer point overflow"))?,
        )
        .ok_or_else(|| io::Error::other("Observer point overflow"))?;
    let points = bytes
        .get(16..end)
        .ok_or_else(|| io::Error::other("Incomplete observer source"))?;
    let mut radius = 0.0f64;
    let mut left = 0.0f64;
    for point in points.as_chunks::<24>().0 {
        left = left.min(f64::from_le_bytes(point[..8].try_into()?));
        let mut squared = 0.0;
        for value in point.as_chunks::<8>().0 {
            let v = f64::from_le_bytes(*value);
            if !v.is_finite() {
                return Err(io::Error::other("Nonfinite observer source").into());
            }
            squared += v * v;
        }
        radius = radius.max(squared.sqrt());
    }
    if !radius.is_finite() || radius <= 0.0 {
        return Err(io::Error::other("Invalid observer radius").into());
    }
    Ok((radius, left))
}

/// Place the physical observer outside the station front and the inbound hull's lateral sweep.
fn docking_starts(
    obstructed: bool,
) -> Result<(FlightStart, FlightStart), Box<dyn std::error::Error>> {
    let station = OriginalStationData::bundled()?;
    let port = station.port().center();
    let source = MuleHullSource::bundled()?;
    let (observer_radius, _) = observer_source()?;
    let reserve = observer_radius * 1.001 + 2.0;
    // The +69 m lane meets the original SH028 front lip. The +100 m lane
    // clears the wall and is only an entry-conditions refusal.
    let approach_x = port.x + if obstructed { 69.0 } else { 0.0 };
    let approach = FlightStart {
        position: [approach_x as f32, port.y as f32, 260.0],
        orientation: [0.0, 1.0, 0.0, 0.0],
        speed: if obstructed { 33.2 } else { 4.0 },
    };
    let observer_position = [
        (f64::from(approach.position[0]) - source.maximum_origin_radius() - reserve) as f32,
        approach.position[1],
        (port.z + reserve) as f32,
    ];
    // Frame the full inbound lane from the opening pose to the port plane. An aim
    // fixed on the port alone can crop the entire hull at the start of the approach.
    let lateral = approach_x - f64::from(observer_position[0]);
    let bearing = |z: f64| lateral.atan2(z - f64::from(observer_position[2]));
    let yaw = (bearing(f64::from(approach.position[2])) + bearing(port.z)) * 0.5;
    let (sine, cosine) = (yaw * 0.5).sin_cos();
    let observer = FlightStart {
        position: observer_position,
        orientation: [0.0, sine as f32, 0.0, cosine as f32],
        speed: 0.0,
    };
    Ok((observer, approach))
}

/// The existing World constructor alone validates station/observer clearance and owns the actor.
/// No fallback, reposition retry, caller-supplied actor, speed or shield state is accepted.
fn observer(view: View) -> Result<FlightStart, Box<dyn std::error::Error>> {
    let station = OriginalStationData::bundled()?;
    let source = MuleHullSource::bundled()?;
    let launch = station.mule_launch_start(&source)?;
    let (radius, left) = observer_source()?;
    let gap = source.maximum_origin_radius() + radius * 1.001 + 1.0;
    let ahead = view != View::Rear;
    let distance = gap + if view == View::Far { 128.0 } else { 0.0 };
    let z = f64::from(launch.position[2]) + if ahead { distance } else { -distance };
    if !z.is_finite() || z.abs() > 1e6 {
        return Err(io::Error::other("Observer placement exceeds internal scope").into());
    }
    if view == View::Side {
        let right = source
            .pieces()
            .iter()
            .try_fold(f64::NEG_INFINITY, |largest, p| {
                p.support_bound(meridian_core::docking::Vec3::new(1.0, 0.0, 0.0))
                    .map(|v| largest.max(v))
            })?;
        // Start well ahead, outside the required enclosing spheres, looking through the
        // existing left view. Half a metre of side-plane overlap intentionally allows
        // the real World sweep to decide contact; it never grants a clear trajectory.
        return Ok(FlightStart {
            position: [
                (f64::from(launch.position[0]) + right - left - 0.5) as f32,
                launch.position[1],
                (f64::from(launch.position[2]) + gap + 128.0) as f32,
            ],
            orientation: [0.0, 0.0, 0.0, 1.0],
            speed: 0.0,
        });
    }
    Ok(FlightStart {
        position: [launch.position[0], launch.position[1], z as f32],
        orientation: if ahead {
            [0.0, 1.0, 0.0, 0.0]
        } else {
            [0.0, 0.0, 0.0, 1.0]
        },
        speed: 0.0,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Control {
    Quit,
    Updated,
    Ignored,
}
impl super::App {
    fn docking_hold(&mut self) -> bool {
        use meridian_core::traffic::NpcPhase;
        let Some(frame) = self.game.world.mule_actor_snapshot() else {
            return false;
        };
        let actor = frame.actor();
        let reason = match actor.phase {
            NpcPhase::Complete => "CROSSING ACCEPTED",
            NpcPhase::Approach if frame.motion_refusal().is_some() => "MULE STOPPED",
            NpcPhase::Approach if frame.world_tick() >= DOCKING_TICK_LIMIT => {
                "VALIDATION TICK LIMIT REACHED"
            }
            _ => return false,
        };
        self.paused = true;
        self.notice.set(reason.into());
        true
    }

    pub(super) fn advance_validation(
        &mut self,
        ticks: usize,
    ) -> Result<Option<meridian_core::MotionRefusal>, meridian_core::StepError> {
        for _ in 0..ticks {
            if self.docking_hold() {
                break;
            }
            self.game
                .world
                .step(meridian_core::FlightInput::default())?;
            if self.docking_hold() {
                break;
            }
        }
        Ok(None)
    }

    /// Host controls only. Every accepted step still calls the real World reducer.
    pub(super) fn validation_control(
        &mut self,
        code: winit::keyboard::KeyCode,
    ) -> Result<Control, meridian_core::StepError> {
        use winit::keyboard::KeyCode;
        if !self.game.is_internal_validation() {
            return Ok(Control::Ignored);
        }
        match code {
            KeyCode::Escape | KeyCode::KeyQ => Ok(Control::Quit),
            KeyCode::KeyP if !self.docking_hold() => {
                self.paused = !self.paused;
                self.reset_input();
                Ok(Control::Updated)
            }
            KeyCode::Period if self.paused && !self.docking_hold() => {
                match self.advance_frame(true, 1) {
                    super::FrameAdvance::Fatal(error) => Err(error),
                    super::FrameAdvance::Continue(_) => Ok(Control::Updated),
                }
            }
            _ => Ok(Control::Ignored),
        }
    }
}

#[cfg(test)]
mod tests;

//! Explicit internal entry, no default directory/profile/save access or new actor authority.
use crate::profile::Profile;
use meridian_core::{FlightStart, OriginalStationData, World, mule::MuleHullSource};
use std::{ffi::OsString, io, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum View {
    Front,
    Rear,
    Far,
    Side,
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
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Expected --internal-mule-validation front|rear|far|side; no profile/save arguments",
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
                let world = World::mule_validation_scenario(0x4d55_4c45, observer(view)?)?;
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
            KeyCode::KeyP => {
                self.paused = !self.paused;
                self.reset_input();
                Ok(Control::Updated)
            }
            KeyCode::Period if self.paused => match self.advance_frame(true, 1) {
                super::FrameAdvance::Fatal(error) => Err(error),
                super::FrameAdvance::Continue(_) => Ok(Control::Updated),
            },
            _ => Ok(Control::Ignored),
        }
    }
}

#[cfg(test)]
mod tests;

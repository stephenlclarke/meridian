//! Isolated SH171 preview custody. It is not the PLUGH quest or an ordinary owned hull.
use super::{
    FlightMode, FlightStart, OriginalStationData, Snapshot, StationError, StepError, World,
};
use crate::save::{Reader, SAVE_VERSION, SaveError, Writer, checksum};

const MAGIC: &[u8; 8] = b"MEDPRV01";
const INVALID: SaveError = SaveError::InvalidState("Medusa preview hull, fit or pose");
const MAX_SAVE: usize = 128;
const CABIN_CAPACITY: u8 = 4;
const HOLD_TONNES: u16 = 22;
const EXTRA_TONNES: u16 = 10;
const CABIN_TONNES: u16 = 5;
const EQUIPMENT_UNITS: u16 = 13;
const UTILITY_MOUNTS: u16 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerHullKind {
    Peregrine,
    Medusa,
}

impl PlayerHullKind {
    pub const fn source_id(self) -> &'static str {
        match self {
            Self::Peregrine => "SH025",
            Self::Medusa => "SH171",
        }
    }

    /// Speed, acceleration, pitch, yaw and roll in original gameplay units.
    pub(super) const fn flight_rates(self) -> (f32, f32, f32, f32, f32) {
        match self {
            Self::Peregrine => (
                super::MAX_SPEED,
                super::ACCELERATION,
                super::PITCH_RATE,
                super::YAW_RATE,
                super::ROLL_RATE,
            ),
            Self::Medusa => (50.3, 18.0, 1.21, 1.12, 1.37),
        }
    }
}

/// A non-commercial fixture fit, never issued to the ordinary commander ledger.
/// Each cabin consumes the source's 5 t and one equipment unit. A cargo expansion
/// consumes one equipment unit and one utility mount; no paid or quest item is granted.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MedusaPreviewFit {
    pub cargo_expansion: bool,
    pub cabins: u8,
}

impl MedusaPreviewFit {
    pub const fn cargo_capacity_tonnes(self) -> u16 {
        HOLD_TONNES
            + if self.cargo_expansion {
                EXTRA_TONNES
            } else {
                0
            }
    }
    pub const fn cargo_used_tonnes(self) -> u16 {
        CABIN_TONNES * self.cabins as u16
    }
    pub const fn equipment_used(self) -> u16 {
        self.cabins as u16 + self.cargo_expansion as u16
    }
    pub const fn utility_used(self) -> u16 {
        self.cargo_expansion as u16
    }
    pub const fn valid(self) -> bool {
        self.cabins <= CABIN_CAPACITY
            && self.cargo_used_tonnes() <= self.cargo_capacity_tonnes()
            && self.equipment_used() <= EQUIPMENT_UNITS
            && self.utility_used() <= UTILITY_MOUNTS
    }
}

impl World {
    pub fn player_hull_kind(&self) -> PlayerHullKind {
        if self
            .station_data
            .as_ref()
            .and_then(OriginalStationData::medusa_pieces)
            .is_some()
        {
            PlayerHullKind::Medusa
        } else {
            PlayerHullKind::Peregrine
        }
    }

    /// Geometry for retained flight evidence predating a hull change at its recorded berth.
    pub(super) fn station_at_visit(&self, visit: u64) -> Option<&OriginalStationData> {
        Some(self.station_data.as_ref()?.player_geometry_at(
            visit,
            self.xyzzy_medusa_issue().map(|(dock, _, _)| dock.visit()),
        ))
    }

    pub(super) fn hull_at_visit(&self, visit: u64) -> PlayerHullKind {
        if self
            .xyzzy_medusa_issue()
            .is_some_and(|(dock, _, _)| visit > dock.visit())
        {
            PlayerHullKind::Medusa
        } else {
            PlayerHullKind::Peregrine
        }
    }

    /// Explicit, separate preview World. No ordinary commander, wallet, quest or trade exists.
    pub fn new_medusa_preview() -> Result<Self, StationError> {
        let mut world = Self::at_station(OriginalStationData::medusa_bundled()?)?;
        world.medusa_preview_fit = Some(MedusaPreviewFit::default());
        Ok(world)
    }

    pub const fn medusa_preview_fit(&self) -> Option<MedusaPreviewFit> {
        self.medusa_preview_fit
    }

    /// Preview equipment changes remain local and dock-only. No hidden acquisition authority.
    pub fn set_medusa_preview_fit(&mut self, fit: MedusaPreviewFit) -> Result<(), StepError> {
        if self.medusa_preview_fit.is_none()
            || self.state.mode != FlightMode::Docked
            || !fit.valid()
        {
            return Err(StepError::InvalidMode);
        }
        self.medusa_preview_fit = Some(fit);
        Ok(())
    }

    pub fn medusa_preview_save_bytes(&self) -> Result<Vec<u8>, SaveError> {
        let fit = self.medusa_preview_fit.ok_or(INVALID)?;
        if self.commander_id().is_some()
            || self.player_hull_kind() != PlayerHullKind::Medusa
            || !fit.valid()
        {
            return Err(INVALID);
        }
        let station = self.station_data.as_ref().ok_or(INVALID)?;
        validate_pose(station, &self.state)?;
        let mut writer = Writer::new();
        writer.bytes.extend(MAGIC);
        writer.u64(source_fingerprint());
        writer.bool(fit.cargo_expansion);
        writer.u8(fit.cabins);
        writer.u8(match self.flight_view() {
            super::FlightView::Front => 0,
            super::FlightView::Rear => 1,
            super::FlightView::Left => 2,
            super::FlightView::Right => 3,
        });
        super::persistence::write_flight(&mut writer, &self.state);
        let digest = checksum(&writer.bytes);
        writer.u64(digest);
        Ok(writer.bytes)
    }

    pub fn from_medusa_preview_save(bytes: &[u8]) -> Result<Self, SaveError> {
        if bytes.len() > MAX_SAVE {
            return Err(SaveError::TooLarge);
        }
        let end = bytes.len().checked_sub(8).ok_or(SaveError::Truncated)?;
        let (body, tail) = bytes.split_at(end);
        if checksum(body) != Reader::new(tail).u64()? {
            return Err(SaveError::ChecksumMismatch);
        }
        let mut reader = Reader::new(body);
        if &reader.fixed::<8>()? != MAGIC || reader.u64()? != source_fingerprint() {
            return Err(SaveError::ContentMismatch);
        }
        let fit = MedusaPreviewFit {
            cargo_expansion: reader.bool()?,
            cabins: reader.u8()?,
        };
        if !fit.valid() {
            return Err(INVALID);
        }
        let view = match reader.u8()? {
            0 => super::FlightView::Front,
            1 => super::FlightView::Rear,
            2 => super::FlightView::Left,
            3 => super::FlightView::Right,
            _ => return Err(SaveError::InvalidEncoding),
        };
        let mut state = super::persistence::read_flight(&mut reader, SAVE_VERSION)?;
        if !reader.finished() {
            return Err(SaveError::InvalidEncoding);
        }
        let mut candidate = Self::new_medusa_preview().map_err(|_| SaveError::ContentMismatch)?;
        let station = candidate.station_data.as_ref().ok_or(INVALID)?;
        validate_pose(station, &state)?;
        state.station = Some(station.station_pose());
        candidate.state = state;
        candidate.medusa_preview_fit = Some(fit);
        candidate
            .request_flight_view(view, state.tick)
            .map_err(|_| INVALID)?;
        Ok(candidate)
    }
}

fn source_fingerprint() -> u64 {
    let mut bytes = b"SH171/stock-preview/flight/v1".to_vec();
    bytes.extend(include_bytes!("../../../../assets/legacy/sh028.physics"));
    bytes.extend(include_bytes!("../../../../assets/legacy/sh028.port"));
    bytes.extend(include_bytes!(
        "../../../../assets/fleet/sh171/flight.physics"
    ));
    bytes.extend(include_bytes!(
        "../../../../assets/fleet/sh171/flight.supports"
    ));
    bytes.extend(include_bytes!(
        "../../../../integration/sh171-medusa/MEDUSA-FLAGSHIP.json"
    ));
    let (speed, acceleration, pitch, yaw, roll) = PlayerHullKind::Medusa.flight_rates();
    for rate in [speed, acceleration, pitch, yaw, roll] {
        bytes.extend(rate.to_le_bytes());
    }
    for amount in [
        HOLD_TONNES,
        EXTRA_TONNES,
        CABIN_TONNES,
        EQUIPMENT_UNITS,
        UTILITY_MOUNTS,
    ] {
        bytes.extend(amount.to_le_bytes());
    }
    bytes.push(CABIN_CAPACITY);
    checksum(&bytes)
}

fn validate_pose(station: &OriginalStationData, state: &Snapshot) -> Result<(), SaveError> {
    let norm = state
        .orientation
        .iter()
        .map(|value| value * value)
        .sum::<f32>();
    let bound = 16_000.0
        + state.tick as f64
            * 50.3
            * f64::from(super::DISTANCE_PER_SPEED_UNIT * super::FIXED_DELTA_SECONDS)
            * 1.1;
    if !state.speed.is_finite()
        || !(0.0..=50.3).contains(&state.speed)
        || state
            .position
            .iter()
            .any(|v| !v.is_finite() || f64::from(v.abs()) > bound)
        || !norm.is_finite()
        || (norm - 1.0).abs() > 1e-5
        || (state.motion_refusal.is_some() && state.speed != 0.0)
        || matches!(
            state.motion_refusal,
            Some(
                super::MotionRefusal::NpcContact
                    | super::MotionRefusal::AsteroidContact
                    | super::MotionRefusal::CelestialContact
            )
        )
    {
        return Err(INVALID);
    }
    if state.mode == FlightMode::Docked {
        return if state.speed == 0.0
            && state.position == [0.0; 3]
            && state.orientation == [0.0, 0.0, 0.0, 1.0]
            && state.motion_refusal.is_none()
        {
            Ok(())
        } else {
            Err(INVALID)
        };
    }
    let start = FlightStart {
        position: state.position,
        orientation: state.orientation,
        speed: state.speed,
    };
    if state.mode == FlightMode::Launching {
        if station.launch_start().map_err(|_| INVALID)? != start
            || !station.initial_flight_is_clear(start)
        {
            return Err(INVALID);
        }
    } else {
        let pose = crate::station::flight_pose(start.position, start.orientation).ok_or(INVALID)?;
        // A normal flight may lawfully pass behind the port plane while outside
        // the aperture. Initial launch clearance is not the saved-flight gate.
        if !station.motion_clear(pose, pose, 0.0) {
            return Err(INVALID);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FlightInput;

    #[test]
    fn stock_preview_fit_save_and_flight_are_separate_from_ordinary_custody()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut world = World::new_medusa_preview()?;
        assert_eq!(world.player_hull_kind(), PlayerHullKind::Medusa);
        assert_eq!(world.commander_id(), None);
        assert!(world.save_bytes().is_err());
        assert!(
            World::new_commander(551)?
                .medusa_preview_save_bytes()
                .is_err()
        );
        let fit = MedusaPreviewFit {
            cargo_expansion: true,
            cabins: 4,
        };
        world.set_medusa_preview_fit(fit)?;
        assert!(
            world
                .set_medusa_preview_fit(MedusaPreviewFit { cabins: 5, ..fit })
                .is_err()
        );
        let docked = world.medusa_preview_save_bytes()?;
        assert!(World::from_save_bytes(&docked).is_err());
        let mut world = World::from_medusa_preview_save(&docked)?;
        assert_eq!(world.medusa_preview_fit(), Some(fit));
        world.request_launch(0)?;
        assert!(
            world
                .set_medusa_preview_fit(MedusaPreviewFit::default())
                .is_err()
        );
        let launching = world.medusa_preview_save_bytes()?;
        let mut world = World::from_medusa_preview_save(&launching)?;
        for _ in 0..8 {
            world.step(FlightInput {
                throttle: 1.0,
                ..FlightInput::default()
            })?;
        }
        let snapshot = world.snapshot();
        assert_eq!(snapshot.mode, FlightMode::Flight);
        let flight = world.medusa_preview_save_bytes()?;
        let mut restored = World::from_medusa_preview_save(&flight)?;
        assert_eq!(restored.snapshot(), snapshot);
        for _ in 0..8 {
            world.step(FlightInput::default())?;
            restored.step(FlightInput::default())?;
        }
        assert_eq!(world.snapshot(), restored.snapshot());
        let mut broken = flight;
        broken[17] = 5;
        let end = broken.len() - 8;
        let digest = checksum(&broken[..end]);
        broken[end..].copy_from_slice(&digest.to_le_bytes());
        assert!(World::from_medusa_preview_save(&broken).is_err());
        Ok(())
    }

    #[test]
    fn medusa_preview_actual_stock_rates_and_physical_return_survive_continue()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut world = World::new_medusa_preview()?;
        world.request_launch(world.snapshot().tick)?;
        let launch_speed = world.snapshot().speed;
        world.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        })?;
        assert!(
            (world.snapshot().speed - (launch_speed + 18.0 * super::super::FIXED_DELTA_SECONDS))
                .abs()
                < 1e-6
        );
        // Move farther down the clear exit line before turning the real long hull.
        for _ in 0..60 {
            world.step(FlightInput::default())?;
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        for _ in 0..80 {
            world.step(FlightInput {
                brake: true,
                ..FlightInput::default()
            })?;
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        assert_eq!(world.snapshot().speed, 0.0);
        for _ in 0..220 {
            let orientation = world.snapshot().orientation;
            let remaining = core::f64::consts::PI
                - 2.0 * f64::from(orientation[1]).atan2(f64::from(orientation[3]));
            if remaining <= 1e-6 {
                break;
            }
            let yaw = (remaining / (1.12 * f64::from(super::super::FIXED_DELTA_SECONDS)))
                .clamp(0.0, 1.0) as f32;
            world.step(FlightInput {
                yaw,
                ..FlightInput::default()
            })?;
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        let mut world = World::from_medusa_preview_save(&world.medusa_preview_save_bytes()?)?;
        for _ in 0..4000 {
            let throttle = if world.snapshot().speed < 4.0 {
                1.0
            } else {
                0.0
            };
            world.step(FlightInput {
                throttle,
                ..FlightInput::default()
            })?;
            assert_eq!(world.snapshot().motion_refusal, None);
            if world.snapshot().mode == FlightMode::Docked {
                assert_eq!(world.snapshot().position, [0.0; 3]);
                let continued =
                    World::from_medusa_preview_save(&world.medusa_preview_save_bytes()?)?;
                assert_eq!(continued.snapshot(), world.snapshot());
                assert_eq!(continued.commander_id(), None);
                return Ok(());
            }
        }
        Err("Medusa preview did not return through SH028".into())
    }

    #[test]
    fn medusa_preview_legal_flight_behind_port_can_save_and_continue()
    -> Result<(), Box<dyn std::error::Error>> {
        fn action(world: &mut World, input: FlightInput) -> Result<(), StepError> {
            world.step(input)?;
            assert_eq!(world.snapshot().motion_refusal, None);
            Ok(())
        }
        fn turn(world: &mut World, end: f64) -> Result<(), StepError> {
            for _ in 0..220 {
                let q = world.snapshot().orientation;
                let remaining = end - 2.0 * f64::from(q[1]).atan2(f64::from(q[3]));
                if remaining <= 1e-6 {
                    break;
                }
                action(
                    world,
                    FlightInput {
                        yaw: (remaining / (1.12 * f64::from(super::super::FIXED_DELTA_SECONDS)))
                            .clamp(0.0, 1.0) as f32,
                        ..FlightInput::default()
                    },
                )?;
            }
            Ok(())
        }
        fn brake(world: &mut World) -> Result<(), StepError> {
            for _ in 0..200 {
                if world.snapshot().speed == 0.0 {
                    break;
                }
                action(
                    world,
                    FlightInput {
                        brake: true,
                        ..FlightInput::default()
                    },
                )?;
            }
            assert_eq!(world.snapshot().speed, 0.0);
            Ok(())
        }
        let mut world = World::new_medusa_preview()?;
        world.request_launch(world.snapshot().tick)?;
        brake(&mut world)?;
        turn(&mut world, core::f64::consts::FRAC_PI_2)?;
        for _ in 0..400 {
            action(
                &mut world,
                FlightInput {
                    throttle: 1.0,
                    ..FlightInput::default()
                },
            )?;
        }
        brake(&mut world)?;
        assert!(world.snapshot().position[0] > 2000.0);
        turn(&mut world, core::f64::consts::PI)?;
        for _ in 0..400 {
            action(
                &mut world,
                FlightInput {
                    throttle: 1.0,
                    ..FlightInput::default()
                },
            )?;
        }
        assert!(world.snapshot().position[2] < 0.0);
        assert_eq!(world.snapshot().mode, FlightMode::Flight);
        let saved = world.medusa_preview_save_bytes()?;
        let continued = World::from_medusa_preview_save(&saved)?;
        assert_eq!(continued.snapshot(), world.snapshot());
        Ok(())
    }
}

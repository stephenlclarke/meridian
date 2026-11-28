//! Authenticated SH171 flight geometry using the existing SH028 station adapter.
use super::{CollisionMesh, OriginalStationData, PORT_BYTES, STATION_BYTES, StationError};
use crate::mule::{MULE_CLEARANCE_METRES, SmoothPiece, rounded_maximum_origin_radius};

const PHYSICS: &[u8] = include_bytes!("../../../../assets/fleet/sh171/flight.physics");
const SUPPORTS: &[u8] = include_bytes!("../../../../assets/fleet/sh171/flight.supports");
const SUPPORT_COUNT: usize = 70;

impl OriginalStationData {
    /// Source-bound Medusa geometry for a World which has already authorised SH171.
    /// This constructor grants no player identity, equipment or acquisition rights.
    pub fn medusa_bundled() -> Result<Self, StationError> {
        let mut hull = CollisionMesh::decode(PHYSICS)?;
        if hull.vertices.len() != 372 || hull.faces.len() != 512 {
            return Err(StationError::InvalidMesh);
        }
        let pieces = load_supports(SUPPORTS)?;
        hull.radius = hull.radius.max(rounded_maximum_origin_radius(&pieces));
        let mut data = Self::from_pinned_parts(STATION_BYTES, hull, PORT_BYTES, Some(pieces))?;
        data.npc_geometry = Some(Box::new(Self::bundled()?));
        Ok(data)
    }

    /// The same rounded pieces used by the authenticated SH028 launch and entry checks.
    pub fn medusa_pieces(&self) -> Option<&[SmoothPiece]> {
        self.medusa.as_deref()
    }
}

fn load_supports(mut bytes: &[u8]) -> Result<Box<[SmoothPiece]>, StationError> {
    if take::<8>(&mut bytes)? != *b"MDU512S1"
        || u32::from_le_bytes(take(&mut bytes)?) as usize != SUPPORT_COUNT
    {
        return Err(StationError::InvalidMesh);
    }
    let mut pieces = Vec::with_capacity(SUPPORT_COUNT);
    for _ in 0..SUPPORT_COUNT {
        let count = u32::from_le_bytes(take(&mut bytes)?) as usize;
        if !(4..=8).contains(&count) {
            return Err(StationError::InvalidMesh);
        }
        let mut points = Vec::with_capacity(count);
        for _ in 0..count {
            let point = super::vertex_at(bytes, 0)?;
            bytes = bytes.get(24..).ok_or(StationError::InvalidMesh)?;
            points.push(point);
        }
        pieces.push(
            SmoothPiece::new(points, MULE_CLEARANCE_METRES)
                .map_err(|_| StationError::InvalidMesh)?,
        );
    }
    if !bytes.is_empty() {
        return Err(StationError::InvalidMesh);
    }
    Ok(pieces.into_boxed_slice())
}

fn take<const N: usize>(bytes: &mut &[u8]) -> Result<[u8; N], StationError> {
    let (head, rest) = bytes.split_first_chunk().ok_or(StationError::InvalidMesh)?;
    *bytes = rest;
    Ok(*head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docking::RigidPose;
    use crate::{FIXED_DELTA_SECONDS, FlightInput, FlightMode, World, station::flight_pose};

    fn inbound(x: f32, z: f32) -> RigidPose {
        flight_pose([x, 0.0, z], [0.0, 1.0, 0.0, 0.0])
            .unwrap_or_else(|| panic!("invalid Medusa approach fixture at ({x}, {z})"))
    }

    #[test]
    fn medusa_geometry_is_bound_without_changing_the_ordinary_hull() -> Result<(), StationError> {
        let ordinary = OriginalStationData::bundled()?;
        let medusa = OriginalStationData::medusa_bundled()?;
        assert_eq!(ordinary.hull_triangle_count(), 964);
        assert!(ordinary.medusa_pieces().is_none());
        assert_eq!(medusa.hull_triangle_count(), 512);
        assert_eq!(medusa.medusa_pieces().map(<[_]>::len), Some(70));
        assert_eq!(
            medusa.station_triangle_count(),
            ordinary.station_triangle_count()
        );
        assert_eq!(medusa.port().center(), ordinary.port().center());
        assert!(medusa.initial_flight_is_clear(medusa.launch_start()?));
        assert!(OriginalStationData::from_bytes(STATION_BYTES, PHYSICS, PORT_BYTES).is_err());
        assert!(load_supports(&SUPPORTS[..SUPPORTS.len() - 1]).is_err());
        Ok(())
    }

    #[test]
    fn medusa_swept_station_entry_accepts_routes_and_rejects_contact_and_invalid_entry()
    -> Result<(), StationError> {
        let station = OriginalStationData::medusa_bundled()?;
        for x in [0.0, 25.0] {
            let approach = inbound(x, 198.32);
            let outside = inbound(x, 137.0);
            let inside = inbound(x, 136.0);
            assert!(station.motion_clear(approach, outside, 0.0), "{x}");
            assert!(station.entry_clear(outside, inside, 72.0, 0.0, 0.0), "{x}");
            assert!(!station.entry_clear(outside, inside, 144.0, 0.0, 0.0));
            assert!(!station.entry_clear(outside, inside, 72.0, 0.1, 0.0));
        }
        assert!(!station.motion_clear(
            inbound(60.809_685, 198.32),
            inbound(60.809_685, 136.0),
            0.0
        ));
        assert!(!station.entry_clear(inbound(100.0, 137.0), inbound(100.0, 136.0), 72.0, 0.0, 0.0));
        Ok(())
    }

    #[test]
    fn medusa_world_launch_and_return_use_ordinary_commands()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut world = World::at_station(OriginalStationData::medusa_bundled()?)?;
        assert_eq!(world.snapshot().mode, FlightMode::Docked);
        world.request_launch(world.snapshot().tick)?;
        assert_eq!(world.snapshot().mode, FlightMode::Launching);
        for _ in 0..80 {
            world.step(FlightInput {
                brake: true,
                ..FlightInput::default()
            })?;
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        assert_eq!(world.snapshot().speed, 0.0);
        for _ in 0..400 {
            let orientation = world.snapshot().orientation;
            let remaining = core::f64::consts::PI
                - 2.0 * f64::from(orientation[1]).atan2(f64::from(orientation[3]));
            if remaining <= 1e-6 {
                break;
            }
            let yaw = (remaining / (0.65 * f64::from(FIXED_DELTA_SECONDS))).clamp(0.0, 1.0) as f32;
            world.step(FlightInput {
                yaw,
                ..FlightInput::default()
            })?;
            assert_eq!(world.snapshot().motion_refusal, None);
        }
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
            let snapshot = world.snapshot();
            assert_eq!(snapshot.motion_refusal, None, "{:?}", snapshot.position);
            if snapshot.mode == FlightMode::Docked {
                assert_eq!(snapshot.speed, 0.0);
                assert!(world.commander_id().is_none());
                return Ok(());
            }
        }
        panic!("Medusa did not return through the authenticated station port");
    }
}

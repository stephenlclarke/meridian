//! SH001 smooth-envelope queries against the authenticated SH028 station.
//! These are geometric checks; only World can issue arrival, launch or owned-body commands.
use super::{
    FlightStart, LAUNCH_SPEED, OriginalStationData, StationError, add, finite, flight_pose, sub,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::mule::MuleHullSource;

impl OriginalStationData {
    pub fn mule_motion_clear(
        &self,
        source: &MuleHullSource,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
    ) -> bool {
        self.station.faces.iter().all(|face| {
            let triangle = [
                self.station.vertices[face[0]],
                self.station.vertices[face[1]],
                self.station.vertices[face[2]],
            ];
            source.sweep_clears_triangles(start, end, angular_travel, &[triangle]) == Ok(true)
        })
    }
    pub fn mule_launch_start(&self, source: &MuleHullSource) -> Result<FlightStart, StationError> {
        let aligned = RigidPose {
            position: Vec3::new(0.0, 0.0, 0.0),
            orientation: Basis3::IDENTITY,
        };
        if source.fits_port(&self.port, aligned, self.station_pose(), 0.0) != Ok(true) {
            return Err(StationError::UnsafeLaunch);
        }
        let rear = source
            .pieces()
            .iter()
            .try_fold(f64::NEG_INFINITY, |largest, piece| {
                piece
                    .support_bound(Vec3::new(0.0, 0.0, -1.0))
                    .map(|distance| largest.max(distance))
            })
            .map_err(|_| StationError::InvalidMesh)?;
        let z = self.station.maximum.z + rear + 1000.0;
        let rounded = z as f32;
        let rounded = if f64::from(rounded) < z {
            rounded.next_up()
        } else {
            rounded
        };
        let start = FlightStart {
            position: [0.0, 0.0, rounded],
            orientation: [0.0, 0.0, 0.0, 1.0],
            speed: LAUNCH_SPEED,
        };
        let pose =
            flight_pose(start.position, start.orientation).ok_or(StationError::InvalidPose)?;
        let end = RigidPose {
            position: add(pose.position, Vec3::new(0.0, 0.0, 1000.0)),
            orientation: pose.orientation,
        };
        if !self.mule_motion_clear(source, pose, end, 0.0) {
            return Err(StationError::UnsafeLaunch);
        }
        Ok(start)
    }
    pub fn mule_entry_clear(
        &self,
        source: &MuleHullSource,
        start: RigidPose,
        end: RigidPose,
        physical_speed: f64,
        roll_rate: f64,
        angular_travel: f64,
    ) -> bool {
        self.mule_entry_conditions(
            source,
            start,
            end,
            physical_speed,
            roll_rate,
            angular_travel,
        ) && self.mule_motion_clear(source, start, end, angular_travel)
    }

    /// Entry policy and aperture fit, independent of station mesh clearance.
    pub(crate) fn mule_entry_conditions(
        &self,
        source: &MuleHullSource,
        start: RigidPose,
        end: RigidPose,
        physical_speed: f64,
        roll_rate: f64,
        angular_travel: f64,
    ) -> bool {
        let old = self.depth(start);
        let depth = self.depth(end);
        if !(old > 0.0 && depth <= 0.0 && depth > -self.port.tunnel_depth())
            || !physical_speed.is_finite()
            || !(0.0..144.0).contains(&physical_speed)
            || !roll_rate.is_finite()
            || roll_rate.abs() >= 0.1
            || end.orientation.z.z >= -0.995
            || end.orientation.y.y <= 0.995
            || !angular_travel.is_finite()
            || angular_travel < 0.0
        {
            return false;
        }
        let radius = source.maximum_origin_radius();
        let a = sub(start.position, self.port.center());
        let b = sub(end.position, self.port.center());
        let coordinate_scale = start
            .position
            .x
            .abs()
            .max(start.position.y.abs())
            .max(start.position.z.abs())
            .max(end.position.x.abs())
            .max(end.position.y.abs())
            .max(end.position.z.abs());
        let numeric = 64.0 * f64::from(f32::EPSILON) * (coordinate_scale + radius + 1.0);
        let margin = (radius + a.dot(a).sqrt().max(b.dot(b).sqrt())) * angular_travel + numeric;
        let station = self.station_pose();
        margin.is_finite()
            && source.fits_port(&self.port, start, station, margin) == Ok(true)
            && source.fits_port(&self.port, end, station, margin) == Ok(true)
    }
}

impl OriginalStationData {
    /// Controlled validation only: the stationary observer retains its real SH025 mesh.
    /// Initial noncontainment is established by the World constructor, then every
    /// continuous Mule step checks the same frozen observer's complete native surface.
    pub(crate) fn mule_observer_clear(
        &self,
        source: &MuleHullSource,
        start: RigidPose,
        end: RigidPose,
        observer: RigidPose,
    ) -> bool {
        if !finite(observer.position) || !observer.orientation.is_valid_rotation() {
            return false;
        }
        self.hull.faces.iter().all(|face| {
            let triangle = [face[0], face[1], face[2]].map(|i| {
                add(
                    observer.position,
                    observer.orientation.apply(self.hull.vertices[i]),
                )
            });
            source.sweep_clears_triangles(start, end, 0.0, &[triangle]) == Ok(true)
        })
    }
}

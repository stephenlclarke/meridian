//! Source-bound rounded hull queries against the authenticated SH028 station.
//! These are geometric checks; only World can issue arrival, launch or owned-body commands.
use super::{
    FlightStart, LAUNCH_SPEED, OriginalStationData, StationError, add, finite, flight_pose, sub,
};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::goshawk::GoshawkHullSource;
use crate::goshawk_mk2::GoshawkMk2HullSource;
use crate::mule::{MuleHullSource, SmoothPiece, rounded_fits_port, rounded_maximum_origin_radius};

impl OriginalStationData {
    pub fn mule_motion_clear(
        &self,
        source: &MuleHullSource,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
    ) -> bool {
        self.rounded_station_motion_clear(source.pieces().as_ref(), start, end, angular_travel)
    }

    /// Controlled SH009 motion against the original SH028 mesh.
    pub fn goshawk_mk2_motion_clear(
        &self,
        source: &GoshawkMk2HullSource,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
    ) -> bool {
        self.rounded_station_motion_clear(source.pieces().as_ref(), start, end, angular_travel)
    }

    fn rounded_station_motion_clear(
        &self,
        pieces: &[SmoothPiece],
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
            pieces.iter().all(|piece| {
                piece.sweep_clears_triangle(start, end, angular_travel, triangle) == Ok(true)
            })
        })
    }
    /// Controlled SH013 motion against the original SH028 mesh.
    pub fn buffalo_motion_clear(
        &self,
        source: &crate::buffalo::BuffaloHullSource,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
    ) -> bool {
        self.rounded_station_motion_clear(source.pieces().as_ref(), start, end, angular_travel)
    }

    pub fn mule_launch_start(&self, source: &MuleHullSource) -> Result<FlightStart, StationError> {
        self.rounded_station_launch_start(source.pieces().as_ref())
    }

    fn rounded_station_launch_start(
        &self,
        pieces: &[SmoothPiece],
    ) -> Result<FlightStart, StationError> {
        let aligned = RigidPose {
            position: Vec3::new(0.0, 0.0, 0.0),
            orientation: Basis3::IDENTITY,
        };
        if rounded_fits_port(pieces, &self.port, aligned, self.station_pose(), 0.0) != Ok(true) {
            return Err(StationError::UnsafeLaunch);
        }
        let rear = pieces
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
        if !self.rounded_station_motion_clear(pieces, pose, end, 0.0) {
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
        self.rounded_station_entry_conditions(
            source.pieces().as_ref(),
            start,
            end,
            physical_speed,
            roll_rate,
            angular_travel,
        )
    }

    fn rounded_station_entry_conditions(
        &self,
        pieces: &[SmoothPiece],
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
        let radius = rounded_maximum_origin_radius(pieces);
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
            && rounded_fits_port(pieces, &self.port, start, station, margin) == Ok(true)
            && rounded_fits_port(pieces, &self.port, end, station, margin) == Ok(true)
    }
}

impl OriginalStationData {
    /// Validate a source-relative exterior launch for the candidate SH008 envelope.
    pub fn goshawk_launch_start(
        &self,
        source: &GoshawkHullSource,
    ) -> Result<FlightStart, StationError> {
        self.rounded_station_launch_start(source.pieces().as_ref())
    }

    pub fn goshawk_motion_clear(
        &self,
        source: &GoshawkHullSource,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
    ) -> bool {
        self.rounded_station_motion_clear(source.pieces().as_ref(), start, end, angular_travel)
    }

    /// Validate the entire rounded SH008 crossing against the original physical SH028 port.
    pub fn goshawk_entry_clear(
        &self,
        source: &GoshawkHullSource,
        start: RigidPose,
        end: RigidPose,
        physical_speed: f64,
        roll_rate: f64,
        angular_travel: f64,
    ) -> bool {
        self.goshawk_entry_conditions(
            source,
            start,
            end,
            physical_speed,
            roll_rate,
            angular_travel,
        ) && self.goshawk_motion_clear(source, start, end, angular_travel)
    }

    pub(crate) fn goshawk_entry_conditions(
        &self,
        source: &GoshawkHullSource,
        start: RigidPose,
        end: RigidPose,
        physical_speed: f64,
        roll_rate: f64,
        angular_travel: f64,
    ) -> bool {
        self.rounded_station_entry_conditions(
            source.pieces().as_ref(),
            start,
            end,
            physical_speed,
            roll_rate,
            angular_travel,
        )
    }

    pub(crate) fn goshawk_observer_clear(
        &self,
        source: &GoshawkHullSource,
        start: RigidPose,
        end: RigidPose,
        observer: RigidPose,
    ) -> bool {
        self.rounded_observer_clear(source.pieces().as_ref(), start, end, observer)
    }

    fn rounded_observer_clear(
        &self,
        pieces: &[SmoothPiece],
        start: RigidPose,
        end: RigidPose,
        observer: RigidPose,
    ) -> bool {
        // Fixed orientations and linear translation bound every point by the two hull radii.
        let radius = rounded_maximum_origin_radius(pieces);
        let margin = (radius + self.player_radius()) * 1.001 + 0.001;
        let a = sub(start.position, observer.position);
        let b = sub(end.position, observer.position);
        if [(a.x, b.x), (a.y, b.y), (a.z, b.z)]
            .into_iter()
            .any(|(first, last)| first.min(last) > margin || first.max(last) < -margin)
        {
            return true;
        }
        self.hull.faces.iter().all(|face| {
            let triangle = [face[0], face[1], face[2]].map(|i| {
                add(
                    observer.position,
                    observer.orientation.apply(self.hull.vertices[i]),
                )
            });
            pieces
                .iter()
                .all(|piece| piece.sweep_clears_triangle(start, end, 0.0, triangle) == Ok(true))
        })
    }
}

impl OriginalStationData {
    pub fn goshawk_custom_launch_start(
        &self,
        source: &crate::goshawk_custom::GoshawkCustomHullSource,
    ) -> Result<FlightStart, StationError> {
        self.rounded_station_launch_start(source.pieces())
    }

    pub fn goshawk_custom_motion_clear(
        &self,
        source: &crate::goshawk_custom::GoshawkCustomHullSource,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
    ) -> bool {
        self.rounded_station_motion_clear(source.pieces(), start, end, angular_travel)
    }

    /// Test SH010 against the actual SH028 aperture, admission conditions and station triangles.
    pub fn goshawk_custom_entry_clear(
        &self,
        source: &crate::goshawk_custom::GoshawkCustomHullSource,
        start: RigidPose,
        end: RigidPose,
        physical_speed: f64,
        roll_rate: f64,
        angular_travel: f64,
    ) -> bool {
        self.goshawk_custom_entry_conditions(
            source,
            start,
            end,
            physical_speed,
            roll_rate,
            angular_travel,
        ) && self.goshawk_custom_motion_clear(source, start, end, angular_travel)
    }

    pub(crate) fn goshawk_custom_entry_conditions(
        &self,
        source: &crate::goshawk_custom::GoshawkCustomHullSource,
        start: RigidPose,
        end: RigidPose,
        physical_speed: f64,
        roll_rate: f64,
        angular_travel: f64,
    ) -> bool {
        self.rounded_station_entry_conditions(
            source.pieces(),
            start,
            end,
            physical_speed,
            roll_rate,
            angular_travel,
        )
    }

    /// Qualifies explicit SH035 motion against the original SH028 source triangles.
    pub fn stag_liner_motion_clear(
        &self,
        source: &crate::stag_liner::StagLinerHullSource,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
    ) -> bool {
        self.rounded_station_motion_clear(source.pieces(), start, end, angular_travel)
    }

    pub(crate) fn goshawk_custom_observer_clear(
        &self,
        source: &crate::goshawk_custom::GoshawkCustomHullSource,
        start: RigidPose,
        end: RigidPose,
        observer: RigidPose,
    ) -> bool {
        self.rounded_observer_clear(source.pieces(), start, end, observer)
    }
}

impl OriginalStationData {
    /// Qualifies explicit SH042 motion against the original SH028 source triangles.
    pub fn saker_motion_clear(
        &self,
        source: &crate::saker::SakerHullSource,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
    ) -> bool {
        self.rounded_station_motion_clear(source.pieces(), start, end, angular_travel)
    }

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

impl OriginalStationData {
    /// A stationary authored courier outside the station-facing launch corridor.
    pub(crate) fn renas_mule_berth(
        &self,
        source: &MuleHullSource,
    ) -> Result<RigidPose, StationError> {
        let station = self.station_pose().pose;
        let launch = self.launch_start()?;
        let at_launch =
            flight_pose(launch.position, launch.orientation).ok_or(StationError::InvalidPose)?;
        let side = source.maximum_origin_radius() + self.player_radius() * 1.001 + 16.0;
        let offset = station.orientation.apply(Vec3::new(side, 0.0, 88.0));
        let pose = RigidPose {
            position: add(at_launch.position, offset),
            orientation: station.orientation,
        };
        if !self.mule_motion_clear(source, pose, pose, 0.0) {
            return Err(StationError::UnsafeLaunch);
        }
        Ok(pose)
    }

    /// In the player's translating frame, the stationary Mule sweeps against the
    /// original SH025 triangles from the opening pose. Bound the player rotation
    /// by inflating the separating certificates with its maximum point travel.
    pub(crate) fn mule_ordinary_player_clear(
        &self,
        source: &MuleHullSource,
        mule: RigidPose,
        before: RigidPose,
        after: RigidPose,
        angular_travel: f64,
    ) -> bool {
        if !finite(before.position)
            || !finite(after.position)
            || !before.orientation.is_valid_rotation()
            || !after.orientation.is_valid_rotation()
            || !angular_travel.is_finite()
            || angular_travel < 0.0
        {
            return false;
        }
        let margin = (source.maximum_origin_radius() + self.player_radius()) * 1.001 + 0.001;
        let center = mule.position;
        let a = sub(before.position, center);
        let b = sub(after.position, center);
        if [(a.x, b.x), (a.y, b.y), (a.z, b.z)]
            .into_iter()
            .any(|(first, last)| first.min(last) > margin || first.max(last) < -margin)
        {
            return true;
        }
        let mut end = mule;
        end.position = add(mule.position, sub(before.position, after.position));
        let padding = self.player_radius() * (1.001 * (angular_travel + 1e-6).min(2.0) + 1e-9);
        self.hull.faces.iter().all(|face| {
            let triangle = [face[0], face[1], face[2]].map(|i| {
                add(
                    before.position,
                    before.orientation.apply(self.hull.vertices[i]),
                )
            });
            source.pieces().iter().all(|piece| {
                piece.sweep_clears_triangle_with_padding(mule, end, 0.0, triangle, padding)
                    == Ok(true)
            })
        })
    }
}

impl OriginalStationData {
    /// Controlled SH012 motion against the original SH028 mesh.
    pub fn sable_liner_motion_clear(
        &self,
        source: &crate::sable_liner::SableLinerHullSource,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
    ) -> bool {
        self.rounded_station_motion_clear(source.pieces().as_ref(), start, end, angular_travel)
    }
}

#[cfg(test)]
mod tests {
    use super::{MuleHullSource, OriginalStationData, StationError, flight_pose};

    #[test]
    fn authored_berth_clears_the_source_launch_but_blocks_a_swept_player_hull()
    -> Result<(), Box<dyn std::error::Error>> {
        let station = OriginalStationData::bundled()?;
        let source = MuleHullSource::bundled()?;
        let berth = station.renas_mule_berth(&source)?;
        let launch = station.launch_start()?;
        let before =
            flight_pose(launch.position, launch.orientation).ok_or(StationError::InvalidPose)?;
        assert!(station.mule_ordinary_player_clear(&source, berth, before, before, 0.0));

        let mut end = before;
        end.position.x = 2.0 * berth.position.x - before.position.x;
        end.position.z = 2.0 * berth.position.z - before.position.z;
        // Both endpoints are clear. The interpolated path crosses the berth.
        assert!(station.mule_ordinary_player_clear(&source, berth, end, end, 0.0));
        assert!(!station.mule_ordinary_player_clear(&source, berth, before, end, 0.0));
        Ok(())
    }
}

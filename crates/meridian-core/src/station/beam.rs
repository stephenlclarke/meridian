//! Finite ray against the identical native triangles used for the station physical scene.
use super::{OriginalStationData, StationError, cross, finite, sub};
use crate::docking::{StationPose, Vec3};

impl OriginalStationData {
    pub(crate) fn beam_block_distance(
        &self,
        station: StationPose,
        origin: Vec3,
        direction: Vec3,
        range: f64,
    ) -> Result<Option<f64>, StationError> {
        let pose = station
            .model_pose()
            .map_err(|_| StationError::InvalidPose)?;
        if !finite(origin)
            || !finite(direction)
            || (direction.dot(direction) - 1.0).abs() > 1e-9
            || !range.is_finite()
            || range <= 0.0
        {
            return Err(StationError::InvalidPose);
        }
        let local = |v: Vec3| {
            Vec3::new(
                v.dot(pose.orientation.x),
                v.dot(pose.orientation.y),
                v.dot(pose.orientation.z),
            )
        };
        let origin = local(sub(origin, pose.position));
        let direction = local(direction);
        let vertices = &self.station.vertices;
        let mut nearest = None;
        for face in &self.station.faces {
            let [a, b, c] = [vertices[face[0]], vertices[face[1]], vertices[face[2]]];
            if let Some(distance) =
                triangle_distance(origin, direction, a, b, c, nearest.unwrap_or(range))
            {
                nearest = Some(distance);
            }
        }
        Ok(nearest)
    }
}

// Matches the original two-sided GameplayRaycast triangle narrow phase and range endpoints.
pub(crate) fn triangle_distance(
    origin: Vec3,
    direction: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    limit: f64,
) -> Option<f64> {
    let edge = sub(b, a);
    let other = sub(c, a);
    let perpendicular = cross(direction, other);
    let determinant = edge.dot(perpendicular);
    if !determinant.is_finite()
        || determinant.abs() <= edge.dot(edge).sqrt() * other.dot(other).sqrt() * 1e-12
    {
        return None;
    }
    let reciprocal = determinant.recip();
    let offset = sub(origin, a);
    let u = offset.dot(perpendicular) * reciprocal;
    if !(-1e-10..=1.0 + 1e-10).contains(&u) {
        return None;
    }
    let perpendicular = cross(offset, edge);
    let v = direction.dot(perpendicular) * reciprocal;
    if v < -1e-10 || u + v > 1.0 + 1e-10 {
        return None;
    }
    let distance = other.dot(perpendicular) * reciprocal;
    (distance.is_finite() && (0.0..=limit).contains(&distance)).then_some(distance)
}

impl OriginalStationData {
    pub(crate) fn player_beam_distance(
        &self,
        pose: crate::docking::RigidPose,
        origin: Vec3,
        direction: Vec3,
        range: f64,
    ) -> Result<Option<f64>, StationError> {
        if !pose.orientation.is_valid_rotation()
            || !finite(pose.position)
            || !finite(pose.orientation.x)
            || !finite(pose.orientation.y)
            || !finite(pose.orientation.z)
            || !finite(origin)
            || !finite(direction)
            || (direction.dot(direction) - 1.0).abs() > 1e-9
            || !range.is_finite()
            || range <= 0.0
        {
            return Err(StationError::InvalidPose);
        }
        let local = |v: Vec3| {
            Vec3::new(
                v.dot(pose.orientation.x),
                v.dot(pose.orientation.y),
                v.dot(pose.orientation.z),
            )
        };
        let origin = local(sub(origin, pose.position));
        let direction = local(direction);
        let mut nearest = None;
        for face in &self.hull.faces {
            let [a, b, c] = [
                self.hull.vertices[face[0]],
                self.hull.vertices[face[1]],
                self.hull.vertices[face[2]],
            ];
            if let Some(distance) =
                triangle_distance(origin, direction, a, b, c, nearest.unwrap_or(range))
            {
                nearest = Some(distance);
            }
        }
        Ok(nearest)
    }

    /// A model origin can lie in a real opening. Aim at the nearest nondegenerate
    /// native triangle centroid instead; collision still uses the unmodified mesh.
    /// Face order breaks equal-distance ties, so runtime and retained replay agree.
    pub(crate) fn player_missile_aimpoint(
        &self,
        pose: crate::docking::RigidPose,
        origin: Vec3,
    ) -> Result<Vec3, StationError> {
        if !pose.orientation.is_valid_rotation() || !finite(pose.position) || !finite(origin) {
            return Err(StationError::InvalidPose);
        }
        let delta = sub(origin, pose.position);
        let local_origin = Vec3::new(
            delta.dot(pose.orientation.x),
            delta.dot(pose.orientation.y),
            delta.dot(pose.orientation.z),
        );
        let mut best: Option<(f64, Vec3)> = None;
        for face in &self.hull.faces {
            let [a, b, c] = [
                self.hull.vertices[face[0]],
                self.hull.vertices[face[1]],
                self.hull.vertices[face[2]],
            ];
            let point = super::scale(super::add(super::add(a, b), c), 1.0 / 3.0);
            let delta = sub(point, local_origin);
            let distance_squared = delta.dot(delta);
            if !distance_squared.is_finite()
                || distance_squared <= 1e-18
                || best.is_some_and(|(old, _)| distance_squared >= old)
            {
                continue;
            }
            let distance = distance_squared.sqrt();
            let direction = super::scale(delta, distance.recip());
            if triangle_distance(local_origin, direction, a, b, c, distance + 1e-7).is_some() {
                best = Some((distance_squared, point));
            }
        }
        best.map(|(_, point)| super::add(pose.position, pose.orientation.apply(point)))
            .ok_or(StationError::InvalidPose)
    }

    pub(crate) fn player_clearance_radius(&self) -> f64 {
        self.hull.radius
    }

    /// Enclose the complete admitted station collision representation before
    /// testing the source's strict 64 metre protected-volume separation.
    pub(crate) fn practice_sphere_clears_station(
        &self,
        station: StationPose,
        center: Vec3,
        radius: f64,
    ) -> bool {
        let Ok(pose) = station.model_pose() else {
            return false;
        };
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for vertex in &self.station.vertices {
            let oriented = pose.orientation.apply(*vertex);
            let point = super::add(pose.position, oriented);
            for (i, coord) in [point.x, point.y, point.z].into_iter().enumerate() {
                low[i] = low[i].min(coord);
                high[i] = high[i].max(coord);
            }
        }
        let [x, y, z] = [center.x, center.y, center.z];
        let nearest = Vec3::new(
            x.clamp(low[0], high[0]),
            y.clamp(low[1], high[1]),
            z.clamp(low[2], high[2]),
        );
        let delta = sub(center, nearest);
        let numerical =
            1e-8 * (center.dot(center).sqrt() + nearest.dot(nearest).sqrt() + radius).max(1.0);
        delta.dot(delta).sqrt() - radius > 64.0 + numerical
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_triangles_include_range_boundary_and_first_wall_without_filling_aperture() {
        let data = OriginalStationData::bundled().unwrap_or_else(|error| panic!("{error}"));
        let station = data.station_pose();
        let direction = Vec3::new(0.0, 0.0, -1.0);
        let wall = Vec3::new(70.0, 0.0, 500.0);
        let distance = data
            .beam_block_distance(station, wall, direction, 6000.0)
            .unwrap_or_else(|error| panic!("{error}"))
            .unwrap_or_else(|| panic!("missing native wall"));
        assert!((distance - 363.68).abs() < 1e-9);
        assert_eq!(
            data.beam_block_distance(station, wall, direction, distance),
            Ok(Some(distance))
        );
        assert_eq!(
            data.beam_block_distance(station, wall, direction, distance - 0.001),
            Ok(None)
        );
        assert_eq!(
            data.beam_block_distance(station, Vec3::new(0.0, 0.0, 500.0), direction, 6000.0),
            Ok(None)
        );
        assert_eq!(
            data.beam_block_distance(station, wall, Vec3::new(0.0, 0.0, 1.0), 6000.0),
            Ok(None)
        );
        assert_eq!(
            data.beam_block_distance(station, wall, Vec3::new(0.0, 0.0, 0.0), 6000.0),
            Err(StationError::InvalidPose)
        );
    }

    #[test]
    fn fore_aft_native_hull_gap_stays_open_while_guidance_selects_real_surface() {
        use crate::docking::{Basis3, RigidPose};
        let data = OriginalStationData::bundled().unwrap_or_else(|e| panic!("{e}"));
        // Geometry fixture only: no World pose, actor or gameplay receipt is installed.
        let body = RigidPose {
            position: Vec3::new(0.0, 0.0, 0.0),
            orientation: Basis3::IDENTITY,
        };
        for z in [-500.0, 500.0] {
            let origin = Vec3::new(0.0, 0.0, z);
            let centre_ray = Vec3::new(0.0, 0.0, -z.signum());
            assert_eq!(
                data.player_beam_distance(body, origin, centre_ray, 1000.0),
                Ok(None),
                "the authored central opening must stay empty"
            );
            let aim = data
                .player_missile_aimpoint(body, origin)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_ne!(aim, body.position);
            let delta = sub(aim, origin);
            let length = delta.dot(delta).sqrt();
            let direction = super::super::scale(delta, length.recip());
            let first = data
                .player_beam_distance(body, origin, direction, length + 1e-7)
                .unwrap_or_else(|e| panic!("{e}"))
                .unwrap_or_else(|| panic!("surface-guided ray missed actual hull"));
            assert!(first > 0.0 && first <= length + 1e-7);
            assert_eq!(
                data.player_beam_distance(body, origin, direction, first - 1e-4),
                Ok(None)
            );
        }
    }

    #[test]
    fn native_surface_guidance_transforms_actual_geometry_and_rejects_invalid_pose() {
        use crate::docking::{Basis3, RigidPose};
        let data = OriginalStationData::bundled().unwrap_or_else(|e| panic!("{e}"));
        let identity = RigidPose {
            position: Vec3::new(0.0, 0.0, 0.0),
            orientation: Basis3::IDENTITY,
        };
        let origin = Vec3::new(7.0, 3.0, -500.0);
        let local = data
            .player_missile_aimpoint(identity, origin)
            .unwrap_or_else(|e| panic!("{e}"));
        let (s, c) = 1.2_f64.sin_cos();
        let body = RigidPose {
            position: Vec3::new(700.0, 0.0, 5000.0),
            orientation: Basis3 {
                x: Vec3::new(c, 0.0, -s),
                y: Vec3::new(0.0, 1.0, 0.0),
                z: Vec3::new(s, 0.0, c),
            },
        };
        let origin = super::super::add(body.position, body.orientation.apply(origin));
        let point = data
            .player_missile_aimpoint(body, origin)
            .unwrap_or_else(|e| panic!("{e}"));
        let expected = super::super::add(body.position, body.orientation.apply(local));
        let error = sub(point, expected);
        assert!(error.dot(error) < 1e-16);
        let delta = sub(point, origin);
        let distance = delta.dot(delta).sqrt();
        assert!(
            data.player_beam_distance(
                body,
                origin,
                super::super::scale(delta, distance.recip()),
                distance + 1e-7
            )
            .unwrap_or_else(|e| panic!("{e}"))
            .is_some()
        );
        assert_eq!(
            data.player_missile_aimpoint(body, Vec3::new(f64::NAN, 0.0, 0.0)),
            Err(StationError::InvalidPose)
        );
        let invalid = RigidPose {
            orientation: Basis3 {
                z: Vec3::new(0.0, 0.0, 0.0),
                ..Basis3::IDENTITY
            },
            ..body
        };
        assert_eq!(
            data.player_missile_aimpoint(invalid, origin),
            Err(StationError::InvalidPose)
        );
    }
}

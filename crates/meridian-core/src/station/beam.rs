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
fn triangle_distance(
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
}

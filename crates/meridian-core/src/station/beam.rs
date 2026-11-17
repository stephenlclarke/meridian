//! Finite ray against the identical native triangles used for the station physical scene.
use super::{OriginalStationData, StationError, add, cross, finite, scale, sub};
use crate::docking::{StationPose, Vec3};
#[cfg(test)]
mod aim_tests;

impl OriginalStationData {
    /// G3's native triangle aim is shared by missiles, NPCs and contact fixtures.
    pub(crate) fn player_missile_aimpoint(
        &self,
        pose: crate::docking::RigidPose,
        origin: Vec3,
    ) -> Result<Vec3, StationError> {
        self.player_beam_aim_point(pose, origin, false)?
            .ok_or(StationError::InvalidPose)
    }
    pub(crate) fn player_planar_aimpoint(
        &self,
        pose: crate::docking::RigidPose,
        origin: Vec3,
    ) -> Result<Option<Vec3>, StationError> {
        self.player_beam_aim_point(pose, origin, true)
    }
    /// An actual admitted hull triangle supplies the aiming point. A model origin
    /// need not lie on a solid surface. Planar pilots can only aim in their own Y plane.
    fn player_beam_aim_point(
        &self,
        pose: crate::docking::RigidPose,
        origin: Vec3,
        planar: bool,
    ) -> Result<Option<Vec3>, StationError> {
        if !pose.orientation.is_valid_rotation() || !finite(pose.position) || !finite(origin) {
            return Err(StationError::InvalidPose);
        }
        let mut best: Option<(f64, Vec3)> = None;
        for face in &self.hull.faces {
            let [a, b, c] = [face[0], face[1], face[2]]
                .map(|i| add(pose.position, pose.orientation.apply(self.hull.vertices[i])));
            let candidate = if planar {
                let Some(point) = planar_triangle_point([a, b, c], origin.y) else {
                    continue;
                };
                point
            } else {
                scale(add(add(a, b), c), 1.0 / 3.0)
            };
            let delta = sub(candidate, origin);
            let score = delta.dot(delta);
            if !score.is_finite() || score <= 1e-18 || best.is_some_and(|(old, _)| score >= old) {
                continue;
            }
            let distance = score.sqrt();
            let direction = scale(delta, distance.recip());
            // Exclude a coplanar/tangent candidate; final admission still uses the
            // complete native first-contact query, including nearer surfaces.
            if triangle_distance(origin, direction, a, b, c, distance + 1e-6).is_none() {
                continue;
            }
            best = Some((score, candidate));
        }
        Ok(best.map(|(_, point)| point))
    }
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

// A segment inside the actual triangle and horizontal weapon plane, not an enclosing proxy.
fn planar_triangle_point([a, b, c]: [Vec3; 3], y: f64) -> Option<Vec3> {
    let mut points = [a; 3];
    let mut count = 0;
    for (a, b) in [(a, b), (b, c), (c, a)] {
        let dy = b.y - a.y;
        if dy.abs() <= 1e-12 {
            continue;
        }
        let fraction = (y - a.y) / dy;
        if (0.0..=1.0).contains(&fraction) {
            points[count] = add(a, scale(sub(b, a), fraction));
            count += 1;
        }
    }
    (count >= 2).then(|| scale(add(points[0], points[1]), 0.5))
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

//! Rounded convex supports. Only strict separating certificates grant clearance.
use super::EnvelopeError;
use crate::docking::{Basis3, RigidPose, Vec3};
use std::sync::Arc;

pub(super) fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
pub(super) fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
pub(super) fn scale(a: Vec3, k: f64) -> Vec3 {
    Vec3::new(a.x * k, a.y * k, a.z * k)
}
fn coordinates(a: Vec3) -> [f64; 3] {
    [a.x, a.y, a.z]
}
fn length(a: Vec3) -> f64 {
    a.dot(a).sqrt()
}
pub(super) fn finite(a: Vec3) -> bool {
    coordinates(a)
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1e12)
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
fn transpose(b: Basis3, d: Vec3) -> Vec3 {
    Vec3::new(b.x.dot(d), b.y.dot(d), b.z.dot(d))
}
pub(super) fn valid_pose(p: RigidPose) -> bool {
    let b = p.orientation;
    finite(p.position)
        && b.is_valid_rotation()
        && (b.x.dot(cross(b.y, b.z)) - 1.0).abs() <= 1e-10
        && [b.x, b.y, b.z]
            .iter()
            .all(|a| (a.dot(*a) - 1.0).abs() <= 1e-10)
        && b.x.dot(b.y).abs() <= 1e-10
        && b.x.dot(b.z).abs() <= 1e-10
        && b.y.dot(b.z).abs() <= 1e-10
}

/// Convex hull of authenticated source points plus a spherical2cm offset.
/// Its support is max(dot(source,n))+radius*length(n). The rounded boundary is C1.
#[derive(Debug, Clone, PartialEq)]
pub struct SmoothPiece {
    points: Arc<[Vec3]>,
    low: Vec3,
    high: Vec3,
    radius: f64,
    origin_radius: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvelopeRayContact {
    Clear,
    Surface { distance: f64, point: Vec3 },
    Uncertain { lower_bound: f64 },
}
impl EnvelopeRayContact {
    pub fn lower_bound(self) -> Option<f64> {
        match self {
            Self::Clear => None,
            Self::Surface { distance, .. } => Some(distance),
            Self::Uncertain { lower_bound } => Some(lower_bound),
        }
    }
}
struct DistanceBracket {
    lower: f64,
    upper: f64,
    direction: Vec3,
}
impl SmoothPiece {
    pub(super) fn new(points: Vec<Vec3>, radius: f64) -> Result<Self, EnvelopeError> {
        if points.is_empty()
            || points.len() > 5344
            || !points.iter().copied().all(finite)
            || !radius.is_finite()
            || radius <= 0.0
            || radius > 0.02
        {
            return Err(EnvelopeError::InvalidContent);
        }
        let low = Vec3::new(
            points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
            points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
            points.iter().map(|p| p.z).fold(f64::INFINITY, f64::min),
        );
        let high = Vec3::new(
            points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max),
            points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max),
            points.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max),
        );
        let origin_radius = points.iter().map(|p| length(*p)).fold(0.0, f64::max) + radius;
        Ok(Self {
            points: points.into(),
            low,
            high,
            radius,
            origin_radius,
        })
    }
    pub fn source_points(&self) -> &[Vec3] {
        &self.points
    }
    pub fn source_bounds(&self) -> (Vec3, Vec3) {
        (self.low, self.high)
    }
    pub fn rounding_metres(&self) -> f64 {
        self.radius
    }
    pub fn center(&self) -> Vec3 {
        add(self.low, scale(sub(self.high, self.low), 0.5))
    }
    pub(super) fn origin_radius(&self) -> f64 {
        self.origin_radius
    }
    fn raw_support(&self, d: Vec3) -> Vec3 {
        let mut best = self.points[0];
        let mut best_dot = best.dot(d);
        for point in self.points.iter().copied().skip(1) {
            let projection = point.dot(d);
            if projection > best_dot {
                best = point;
                best_dot = projection;
            }
        }
        best
    }
    fn support_point(&self, d: Vec3) -> Vec3 {
        let n = length(d);
        if n == 0.0 {
            self.points[0]
        } else {
            add(self.raw_support(d), scale(d, self.radius / n))
        }
    }
    pub fn support_bound(&self, d: Vec3) -> Result<f64, EnvelopeError> {
        if !finite(d) {
            return Err(EnvelopeError::InvalidGeometry);
        }
        let n = length(d);
        let value = self.raw_support(d).dot(d) + self.radius * n;
        let error = 2048.0 * f64::EPSILON * (1.0 + self.origin_radius) * n;
        if !value.is_finite() || !error.is_finite() {
            return Err(EnvelopeError::InvalidGeometry);
        }
        Ok(value + error)
    }
    /// Conservative closed containment; an unresolved distance stays contained/blocked.
    /// Source membership and convexity independently prove every original triangle is inside.
    pub fn contains(&self, p: Vec3) -> bool {
        finite(p) && self.distance_bracket(p).lower <= self.radius
    }
    fn distance_bracket(&self, p: Vec3) -> DistanceBracket {
        let error = 2048.0 * f64::EPSILON * (1.0 + length(p) + self.origin_radius);
        let mut simplex = vec![sub(self.points[0], p)];
        let mut closest = simplex[0];
        let mut result = DistanceBracket {
            lower: 0.0,
            upper: length(closest) + error,
            direction: Vec3::new(1.0, 0.0, 0.0),
        };
        for _ in 0..48 {
            let upper = length(closest);
            if upper <= error {
                return DistanceBracket {
                    lower: 0.0,
                    upper: upper + error,
                    direction: result.direction,
                };
            }
            let direction = scale(closest, -1.0 / upper);
            let support = self.raw_support(direction);
            let lower = (sub(p, support).dot(direction) - error).max(0.0);
            result = DistanceBracket {
                lower,
                upper: upper + error,
                direction,
            };
            if result.upper - result.lower <= error.max(1e-9)
                || result.upper < self.radius
                || simplex.len() == 4
            {
                return result;
            }
            let next = sub(support, p);
            if simplex.contains(&next) {
                return result;
            }
            simplex.push(next);
            let Some((point, keep)) = closest_simplex(&simplex) else {
                return result;
            };
            let old = simplex;
            simplex = keep.into_iter().map(|i| old[i]).collect();
            closest = point;
        }
        result
    }
    /// Continuous swept rounded support. Uncertainty and nonconvergence remain blocked.
    pub fn sweep_clears_triangle(
        &self,
        start: RigidPose,
        end: RigidPose,
        angular_travel: f64,
        triangle: [Vec3; 3],
    ) -> Result<bool, EnvelopeError> {
        if !valid_pose(start)
            || !valid_pose(end)
            || !angular_travel.is_finite()
            || angular_travel < 0.0
            || !triangle.into_iter().all(finite)
        {
            return Err(EnvelopeError::InvalidGeometry);
        }
        // Each accepted Gram-matrix entry differs from identity by at most 1e-10.
        // Its spectral error is <= 3e-10, so the proper polar rotation differs
        // from the basis by < 3e-10 in Frobenius norm. The relative matrix then
        // differs by < 1e-9. Its sine/cosine pair has error < 3e-9; a 1e-8
        // angular guard covers that residual and floating-point arithmetic.
        // atan2 retains small rotations even when trace/acos rounds to zero,
        // and stays well-conditioned near a half turn as well.
        let a = start.orientation;
        let b = end.orientation;
        let sine = 0.5
            * length(Vec3::new(
                a.y.dot(b.z) - a.z.dot(b.y),
                a.z.dot(b.x) - a.x.dot(b.z),
                a.x.dot(b.y) - a.y.dot(b.x),
            ));
        let cosine = 0.5 * (a.x.dot(b.x) + a.y.dot(b.y) + a.z.dot(b.z) - 1.0);
        let endpoint_angle = (sine.atan2(cosine) + 1e-8).min(std::f64::consts::PI);
        if endpoint_angle > angular_travel + 1e-6 {
            return Err(EnvelopeError::UnderstatedRotation);
        }
        let movement = sub(end.position, start.position);
        let center = add(start.position, start.orientation.apply(self.center()));
        let rotation_padding =
            self.origin_radius * (1.001 * (angular_travel + 1e-6).min(2.0) + 1e-9);
        let mut low = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut high = Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for x in [self.low.x, self.high.x] {
            for y in [self.low.y, self.high.y] {
                for z in [self.low.z, self.high.z] {
                    let q = add(start.position, start.orientation.apply(Vec3::new(x, y, z)));
                    low = Vec3::new(low.x.min(q.x), low.y.min(q.y), low.z.min(q.z));
                    high = Vec3::new(high.x.max(q.x), high.y.max(q.y), high.z.max(q.z));
                }
            }
        }
        let guard = 4096.0
            * f64::EPSILON
            * (1.0
                + length(start.position)
                + length(end.position)
                + self.origin_radius
                + triangle.iter().map(|p| length(*p)).fold(0.0, f64::max));
        let padding = self.radius * 1.001 + rotation_padding + guard;
        let low = coordinates(add(
            low,
            Vec3::new(
                movement.x.min(0.0) - padding,
                movement.y.min(0.0) - padding,
                movement.z.min(0.0) - padding,
            ),
        ));
        let high = coordinates(add(
            high,
            Vec3::new(
                movement.x.max(0.0) + padding,
                movement.y.max(0.0) + padding,
                movement.z.max(0.0) + padding,
            ),
        ));
        for (axis, lower) in low.into_iter().enumerate() {
            let first = triangle
                .iter()
                .map(|p| coordinates(*p)[axis])
                .fold(f64::INFINITY, f64::min);
            let last = triangle
                .iter()
                .map(|p| coordinates(*p)[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            if last < lower || first > high[axis] {
                return Ok(true);
            }
        }
        let mut direction = sub(
            scale(add(add(triangle[0], triangle[1]), triangle[2]), 1.0 / 3.0),
            center,
        );
        if length(direction) < 1e-12 {
            direction = Vec3::new(1.0, 0.0, 0.0);
        }
        let mut simplex = Vec::with_capacity(4);
        for _ in 0..48 {
            let magnitude = length(direction);
            if !magnitude.is_finite() || magnitude < 1e-12 {
                return Ok(false);
            }
            direction = scale(direction, 1.0 / magnitude);
            let local = transpose(start.orientation, direction);
            let displacement = if movement.dot(direction) > 0.0 {
                movement
            } else {
                Vec3::new(0.0, 0.0, 0.0)
            };
            let support = add(
                add(
                    add(
                        start.position,
                        start.orientation.apply(self.support_point(local)),
                    ),
                    displacement,
                ),
                scale(direction, rotation_padding),
            );
            let mut opposite = triangle[0];
            for p in triangle.into_iter().skip(1) {
                if p.dot(direction) < opposite.dot(direction) {
                    opposite = p;
                }
            }
            let value = start.position.dot(direction)
                + movement.dot(direction).max(0.0)
                + self.support_bound(local)?
                + rotation_padding
                - opposite.dot(direction);
            let error = 4096.0
                * f64::EPSILON
                * (1.0
                    + length(start.position)
                    + length(movement)
                    + length(opposite)
                    + self.origin_radius
                    + rotation_padding);
            if !value.is_finite() || !error.is_finite() {
                return Err(EnvelopeError::InvalidGeometry);
            }
            if value < -error {
                return Ok(true);
            }
            if simplex.len() == 4 {
                return Ok(false);
            }
            simplex.push(sub(support, opposite));
            let Some((closest, keep)) = closest_simplex(&simplex) else {
                return Ok(false);
            };
            let old = simplex;
            simplex = keep.into_iter().map(|i| old[i]).collect();
            direction = scale(closest, -1.0);
        }
        Ok(false)
    }
    /// Conservative advancement against the same surface, not the original detailed triangles.
    /// A Surface needs an inside witness; uncertainty is an occlusion/refusal, not an effect.
    pub fn ray_contact(
        &self,
        pose: RigidPose,
        origin: Vec3,
        direction: Vec3,
        range: f64,
    ) -> Result<EnvelopeRayContact, EnvelopeError> {
        if !valid_pose(pose)
            || !finite(origin)
            || !finite(direction)
            || (direction.dot(direction) - 1.0).abs() > 1e-8
            || !range.is_finite()
            || !(0.0..=1e12).contains(&range)
        {
            return Err(EnvelopeError::InvalidGeometry);
        }
        let b = pose.orientation;
        let det = b.x.dot(cross(b.y, b.z));
        let inverse = |d: Vec3| {
            Vec3::new(
                cross(b.y, b.z).dot(d) / det,
                cross(b.z, b.x).dot(d) / det,
                cross(b.x, b.y).dot(d) / det,
            )
        };
        let p = inverse(sub(origin, pose.position));
        let v = inverse(direction);
        let stretch = length(v);
        if !stretch.is_finite() || stretch <= 0.0 {
            return Err(EnvelopeError::InvalidGeometry);
        }
        let guard = 4096.0
            * f64::EPSILON
            * (1.0 + length(origin) + length(pose.position) + range + self.origin_radius);
        let low = coordinates(sub(
            self.low,
            Vec3::new(
                self.radius + guard,
                self.radius + guard,
                self.radius + guard,
            ),
        ));
        let high = coordinates(add(
            self.high,
            Vec3::new(
                self.radius + guard,
                self.radius + guard,
                self.radius + guard,
            ),
        ));
        let (mut begin, mut finish) = (0.0_f64, range);
        for (axis, (lower, upper)) in low.into_iter().zip(high).enumerate() {
            let q = coordinates(p)[axis];
            let d = coordinates(v)[axis];
            if d.abs() < 1e-300 {
                if q < lower || q > upper {
                    return Ok(EnvelopeRayContact::Clear);
                }
            } else {
                let a = (lower - q) / d;
                let b = (upper - q) / d;
                begin = begin.max(a.min(b));
                finish = finish.min(a.max(b));
            }
        }
        if begin > finish {
            return Ok(EnvelopeRayContact::Clear);
        }
        let range = finish;
        let tolerance = 1e-6;
        let mut distance = begin;
        for _ in 0..64 {
            let point = add(p, scale(v, distance));
            let bounds = self.distance_bracket(point);
            if bounds.upper <= self.radius {
                return Ok(EnvelopeRayContact::Surface {
                    distance,
                    point: add(origin, scale(direction, distance)),
                });
            }
            let closing = -bounds.direction.dot(v);
            if bounds.lower > self.radius && closing <= 0.0 {
                return Ok(EnvelopeRayContact::Clear);
            }
            if bounds.lower <= self.radius + tolerance {
                let probe = (distance + tolerance / stretch).min(range);
                let tested = self.distance_bracket(add(p, scale(v, probe)));
                if tested.upper <= self.radius {
                    return Ok(EnvelopeRayContact::Surface {
                        distance: probe,
                        point: add(origin, scale(direction, probe)),
                    });
                }
                return Ok(EnvelopeRayContact::Uncertain {
                    lower_bound: distance,
                });
            }
            let step = (bounds.lower - self.radius) / closing;
            if !step.is_finite() || step < 0.0 {
                return Ok(EnvelopeRayContact::Uncertain {
                    lower_bound: distance,
                });
            }
            if distance + step > range {
                return Ok(EnvelopeRayContact::Clear);
            }
            if step < tolerance / stretch {
                return Ok(EnvelopeRayContact::Uncertain {
                    lower_bound: distance,
                });
            }
            distance += step * 0.999999;
        }
        Ok(EnvelopeRayContact::Uncertain {
            lower_bound: distance,
        })
    }
}
fn closest_simplex(points: &[Vec3]) -> Option<(Vec3, Vec<usize>)> {
    let mut best: Option<(f64, Vec3, Vec<usize>)> = None;
    for mask in 1usize..(1usize << points.len()) {
        let ids: Vec<usize> = (0..points.len())
            .filter(|i| mask & (1usize << *i) != 0)
            .collect();
        let anchor = points[ids[0]];
        let k = ids.len() - 1;
        let mut weights = [0.0; 4];
        if k > 0 {
            let diffs: Vec<Vec3> = ids
                .iter()
                .skip(1)
                .map(|i| sub(points[*i], anchor))
                .collect();
            let mut equations = [[0.0; 4]; 3];
            let mut i = 0;
            while i < k {
                let mut j = 0;
                while j < k {
                    equations[i][j] = diffs[i].dot(diffs[j]);
                    j += 1;
                }
                equations[i][k] = -diffs[i].dot(anchor);
                i += 1;
            }
            let mut singular = false;
            let mut column = 0;
            while column < k {
                let mut pivot = column;
                let mut row = column + 1;
                while row < k {
                    if equations[row][column].abs() > equations[pivot][column].abs() {
                        pivot = row;
                    }
                    row += 1;
                }
                if !equations[pivot][column].is_finite() || equations[pivot][column].abs() < 1e-20 {
                    singular = true;
                    break;
                }
                equations.swap(column, pivot);
                let divisor = equations[column][column];
                let mut j = column;
                while j <= k {
                    equations[column][j] /= divisor;
                    j += 1;
                }
                let mut row = 0;
                while row < k {
                    if row != column {
                        let ratio = equations[row][column];
                        let mut j = column;
                        while j <= k {
                            equations[row][j] -= ratio * equations[column][j];
                            j += 1;
                        }
                    }
                    row += 1;
                }
                column += 1;
            }
            if singular {
                continue;
            }
            let mut i = 0;
            while i < k {
                weights[i + 1] = equations[i][k];
                i += 1;
            }
        }
        weights[0] = 1.0 - weights[1..k + 1].iter().sum::<f64>();
        if weights[..=k].iter().any(|v| !v.is_finite() || *v < 0.0) {
            continue;
        }
        let mut closest = Vec3::new(0.0, 0.0, 0.0);
        for (i, id) in ids.iter().enumerate() {
            closest = add(closest, scale(points[*id], weights[i]));
        }
        let distance = closest.dot(closest);
        if !distance.is_finite() {
            continue;
        }
        if best.as_ref().is_none_or(|previous| distance < previous.0) {
            let mut keep: Vec<usize> = ids
                .into_iter()
                .enumerate()
                .filter_map(|(i, id)| (weights[i] > 1.0e-12).then_some(id))
                .collect();
            if keep.is_empty() {
                keep.push(0);
            }
            best = Some((distance, closest, keep));
        }
    }
    best.map(|(_, point, ids)| (point, ids))
}

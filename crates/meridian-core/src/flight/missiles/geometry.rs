//! Exact programmatic Geometry.swift missile fallback, immutable a11fe169.
use crate::docking::{Basis3, RigidPose, Vec3};
pub const MISSILE_VERTICES: [Vec3; 7] = [
    Vec3::new(-8.0, 0.0, -25.0),
    Vec3::new(8.0, 0.0, -25.0),
    Vec3::new(30.0, 12.0, 30.0),
    Vec3::new(-30.0, 12.0, 30.0),
    Vec3::new(0.0, -18.0, -20.0),
    Vec3::new(22.0, -10.0, 30.0),
    Vec3::new(-22.0, -10.0, 30.0),
];
pub const MISSILE_FACES: [&[usize]; 8] = [
    &[0, 3, 2, 1],
    &[0, 4, 6],
    &[0, 6, 3],
    &[1, 2, 5],
    &[1, 5, 4],
    &[0, 1, 4],
    &[3, 6, 5, 2],
    &[4, 5, 6],
];
pub(super) fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
pub(super) fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
pub(super) fn scale(a: Vec3, b: f64) -> Vec3 {
    Vec3::new(a.x * b, a.y * b, a.z * b)
}
pub(super) fn length(a: Vec3) -> f64 {
    a.dot(a).sqrt()
}
pub(super) fn unit(a: Vec3) -> Option<Vec3> {
    let n = length(a);
    (n.is_finite() && n > 1e-9).then(|| scale(a, 1.0 / n))
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
pub(super) fn pose(position: Vec3, direction: Vec3) -> Option<RigidPose> {
    let z = unit(direction)?;
    let up = if z.y.abs() > 0.99 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let x = unit(cross(up, z))?;
    let y = cross(z, x);
    Some(RigidPose {
        position,
        orientation: Basis3 { x, y, z },
    })
}
pub(super) fn mesh_distance(projectile: RigidPose, ray: RigidPose, range: f64) -> Option<f64> {
    let v = MISSILE_VERTICES.map(|v| add(projectile.position, projectile.orientation.apply(v)));
    let mut best = None;
    for face in MISSILE_FACES {
        for i in 1..face.len() - 1 {
            if let Some(d) = crate::station::beam::triangle_distance(
                ray.position,
                ray.orientation.z,
                v[face[0]],
                v[face[i]],
                v[face[i + 1]],
                best.unwrap_or(range),
            ) {
                best = Some(d);
            }
        }
    }
    best
}

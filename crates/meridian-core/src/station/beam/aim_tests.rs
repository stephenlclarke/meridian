//! Native mesh regressions. Authored source; execution belongs to the existing owner lane.
use super::*;
use crate::docking::{Basis3, RigidPose};
fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn some<T>(v: Option<T>) -> T {
    v.unwrap_or_else(|| panic!("missing native surface"))
}
fn check(data: &OriginalStationData, pose: RigidPose, origin: Vec3, point: Vec3) {
    let delta = sub(point, origin);
    let distance = delta.dot(delta).sqrt();
    let direction = scale(delta, distance.recip());
    let contact = some(ok(data.player_beam_distance(
        pose,
        origin,
        direction,
        distance + 1e-5,
    )));
    assert!(contact >= 0.0 && contact <= distance + 1e-5);
}
#[test]
fn axial_native_void_stays_empty_and_triangle_derived_planar_aim_reaches_real_body() {
    let data = ok(OriginalStationData::bundled());
    let pose = RigidPose {
        position: Vec3::new(0.0, 0.0, 1000.0),
        orientation: Basis3::IDENTITY,
    };
    let origin = Vec3::new(0.0, 0.0, 0.0);
    assert_eq!(
        ok(data.player_beam_distance(pose, origin, Vec3::new(0.0, 0.0, 1.0), 2000.0)),
        None
    );
    let point = some(ok(data.player_planar_aimpoint(pose, origin)));
    assert!((point.y - origin.y).abs() < 1e-10);
    check(&data, pose, origin, point);
    check(
        &data,
        pose,
        origin,
        ok(data.player_missile_aimpoint(pose, origin)),
    );
}
#[test]
fn out_of_plane_hull_has_no_yaw_only_target_but_keeps_its_real_three_dimensional_surface() {
    let data = ok(OriginalStationData::bundled());
    let pose = RigidPose {
        position: Vec3::new(0.0, 1000.0, 1000.0),
        orientation: Basis3::IDENTITY,
    };
    let origin = Vec3::new(0.0, 0.0, 0.0);
    assert_eq!(ok(data.player_planar_aimpoint(pose, origin)), None);
    let point = ok(data.player_missile_aimpoint(pose, origin));
    assert!(point.y > 900.0);
    check(&data, pose, origin, point);
}
#[test]
fn rotated_native_surface_aim_is_deterministic_and_never_accepts_invalid_pose() {
    let data = ok(OriginalStationData::bundled());
    let (s, c) = 0.7_f64.sin_cos();
    let pose = RigidPose {
        position: Vec3::new(350.0, 0.0, 1000.0),
        orientation: Basis3 {
            x: Vec3::new(c, 0.0, -s),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(s, 0.0, c),
        },
    };
    let origin = Vec3::new(-600.0, 0.0, 1200.0);
    let point = some(ok(data.player_planar_aimpoint(pose, origin)));
    assert_eq!(some(ok(data.player_planar_aimpoint(pose, origin))), point);
    check(&data, pose, origin, point);
    assert_eq!(
        data.player_planar_aimpoint(pose, Vec3::new(f64::NAN, 0.0, 0.0)),
        Err(StationError::InvalidPose)
    );
}

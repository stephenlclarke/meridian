//! Native mesh regressions. Authored source; execution belongs to the existing owner lane.
use super::{OriginalStationData, StationError, Vec3, scale, sub};
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

#[test]
fn retained_emp_release_axial_miss_selects_a_native_player_surface_without_filling_the_gap() {
    // Exact retained 1a5fc9d geometry only; no World, attacker or effect is fabricated.
    let data = ok(OriginalStationData::bundled());
    let player = RigidPose {
        position: Vec3::new(-5659.5107421875, 0.0, 1261.367919921875),
        orientation: Basis3 {
            x: Vec3::new(-0.9959530185475471, 0.0, 0.089875385095304),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(-0.089875385095304, 0.0, -0.9959530185475471),
        },
    };
    let primary = Vec3::new(-5667.164627312388, 0.0, 1176.5514822992216);
    let centre_delta = sub(player.position, primary);
    let centre_distance = centre_delta.dot(centre_delta).sqrt();
    assert!(centre_distance < 2500.0);
    assert_eq!(
        ok(data.player_beam_distance(
            player,
            primary,
            scale(centre_delta, centre_distance.recip()),
            7500.0,
        )),
        None,
        "the observed native centre-ray miss must remain a miss"
    );
    let surface = ok(data.player_missile_aimpoint(player, primary));
    assert_ne!(surface, player.position);
    let delta = sub(surface, primary);
    let distance = delta.dot(delta).sqrt();
    let direction = scale(delta, distance.recip());
    let hit = some(ok(
        data.player_beam_distance(player, primary, direction, 2500.0)
    ));
    assert!(hit > 0.0 && hit < 2500.0);
    assert_eq!(
        ok(data.player_beam_distance(player, primary, direction, hit - 1e-4)),
        None,
        "native first contact and strict range remain authoritative"
    );
    check(&data, player, primary, surface);
}

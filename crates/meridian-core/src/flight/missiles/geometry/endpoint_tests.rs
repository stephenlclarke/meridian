//! Frozen independent binary64/rational native-plane oracle, not a World pose/history.
//! Reference SHA-256 6f4c32c3d8f822ec4bf6b5b9acde5a46b04d5d942f47e0e4413a744672623e42.
//! Generated before editing/evaluating the SUT, using exact rational plane intersection
//! and Gram barycentrics over admitted native triangle0 [11,10,9]. Actual Linux guard unknown.
use super::{Basis3, RigidPose, Vec3, add, pose, ray_point, scale};
fn vector(bits: [u64; 3]) -> Vec3 {
    let [x, y, z] = bits.map(f64::from_bits);
    Vec3::new(x, y, z)
}
fn bits(p: Vec3) -> [u64; 3] {
    [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
}
#[test]
fn frozen_native_counterexample_keeps_one_contact_ray_endpoint_without_approximate_equality() {
    let origin = vector([0xc03aafc3ba01e4c1, 0x4008dd4589f803dc, 0xc05021f315c5d5b8]);
    let once = vector([0x3fc0aa07bd7b7488, 0xbfed298d8b980bee, 0x3fd8ff0b9c392ecc]);
    let canonical = [0x3fc0aa07bd7b7489, 0xbfed298d8b980bf0, 0x3fd8ff0b9c392ece];
    let distance = f64::from_bits(0x4029fffffffffff7);
    let ray = pose(origin, once).unwrap_or_else(|| panic!("frozen finite ray"));
    assert_eq!(bits(ray.orientation.z), canonical);
    assert_ne!(
        once, ray.orientation.z,
        "unit normalization is not bit-idempotent"
    );
    assert_eq!(
        bits(add(origin, scale(once, distance))),
        [0xc038fe7ef0c35aec, 0xc0217a719eed88b2, 0xc04db9fefdaddcb0]
    );
    assert_eq!(
        bits(ray_point(ray, distance)),
        [0xc038fe7ef0c35aec, 0xc0217a719eed88b4, 0xc04db9fefdaddcb0]
    );
    let data = crate::station::OriginalStationData::bundled().unwrap_or_else(|e| panic!("{e}"));
    let body = RigidPose {
        position: Vec3::new(0.0, 0.0, 0.0),
        orientation: Basis3::IDENTITY,
    };
    let actual = data
        .player_beam_distance(
            body,
            origin,
            ray.orientation.z,
            super::super::MISSILE_SPEED / 60.0,
        )
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|| panic!("independent native-plane ray missed"));
    // Only the numerical raycast/reference comparison has an accuracy tolerance.
    // Runtime movement/receipt/replay equality remains exact, with no new epsilon.
    assert!((actual - distance).abs() < 1e-10);
    assert_ne!(add(origin, scale(once, actual)), ray_point(ray, actual));
}

//! Native geometry components, not injected World poses, emitted shots or gameplay acceptance.
use super::retained_solid_contact;
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::flight::MissileContact;
use crate::flight::missiles::geometry::pose;
use crate::save::SaveError;
use crate::station::OriginalStationData;
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"))
}
fn vector(bits: [u64; 3]) -> Vec3 {
    let [x, y, z] = bits.map(f64::from_bits);
    Vec3::new(x, y, z)
}
#[test]
fn frozen_native_plane_contact_keeps_rear_facing_and_both_player_eligibility_guards() {
    // Same independently frozen rational/IEEE triangle0 reference as endpoint_tests.
    // Reference SHA256 6f4c32c3d8f822ec4bf6b5b9acde5a46b04d5d942f47e0e4413a744672623e42.
    let data = ok(OriginalStationData::bundled());
    let mut station = data.station_pose();
    station.pose.position = Vec3::new(1_000_000.0, 1_000_000.0, 1_000_000.0);
    let player = RigidPose {
        position: Vec3::new(0.0, 0.0, 0.0),
        orientation: Basis3::IDENTITY,
    };
    let ray = ok(pose(
        vector([0xc03aafc3ba01e4c1, 0x4008dd4589f803dc, 0xc05021f315c5d5b8]),
        vector([0x3fc0aa07bd7b7488, 0xbfed298d8b980bee, 0x3fd8ff0b9c392ecc]),
    )
    .ok_or("frozen finite ray"));
    let invalid = SaveError::InvalidState("fixture NPC boundary");
    let (wall, contact) = ok(retained_solid_contact(
        &data,
        station,
        ray,
        13.2,
        player,
        (true, true),
        invalid,
    ));
    assert_eq!(wall, None);
    let Some((distance, kind)) = contact else {
        panic!("independently proven native surface missed");
    };
    assert!((distance - f64::from_bits(0x4029fffffffffff7)).abs() < 1e-10);
    assert_eq!(kind, MissileContact::Player { front: false });
    for eligibility in [(false, true), (true, false), (false, false)] {
        assert_eq!(
            ok(retained_solid_contact(
                &data,
                station,
                ray,
                13.2,
                player,
                eligibility,
                invalid
            )),
            (None, None)
        );
    }
}
#[test]
fn native_station_boundary_and_geometry_errors_keep_the_callers_exact_refusal() {
    let data = ok(OriginalStationData::bundled());
    let station = data.station_pose();
    let ray = RigidPose {
        position: Vec3::new(70.0, 0.0, 500.0),
        orientation: Basis3 {
            x: Vec3::new(-1.0, 0.0, 0.0),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(0.0, 0.0, -1.0),
        },
    };
    let player = RigidPose {
        position: Vec3::new(100000.0, 0.0, 0.0),
        orientation: Basis3::IDENTITY,
    };
    let invalid = SaveError::InvalidState("fixture practice boundary");
    let (wall, contact) = ok(retained_solid_contact(
        &data,
        station,
        ray,
        6000.0,
        player,
        (true, true),
        invalid,
    ));
    let Some(distance) = wall else {
        panic!("original native station wall missed");
    };
    assert!((distance - 363.68).abs() < 1e-9);
    assert_eq!(contact, Some((distance, MissileContact::Station)));
    assert_eq!(
        ok(retained_solid_contact(
            &data,
            station,
            ray,
            distance,
            player,
            (true, true),
            invalid
        )),
        (wall, contact)
    );
    let mut invalid_ray = ray;
    invalid_ray.orientation.z = Vec3::new(0.0, 0.0, 0.0);
    for refusal in [invalid, SaveError::InvalidState("fixture NPC boundary")] {
        assert_eq!(
            retained_solid_contact(
                &data,
                station,
                invalid_ray,
                6000.0,
                player,
                (false, false),
                refusal
            ),
            Err(refusal)
        );
    }
}

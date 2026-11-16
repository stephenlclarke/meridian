use super::{OFFSETS, PRACTICE_HALF_SIDE, VOLUME_OFFSET, VOLUME_RADIUS};
use crate::docking::{RigidPose, StationPose, Vec3};
use crate::station::OriginalStationData;

pub(super) fn beacon_pose(frame: RigidPose, ordinal: usize) -> RigidPose {
    RigidPose {
        position: add(frame.position, frame.orientation.apply(OFFSETS[ordinal])),
        orientation: frame.orientation,
    }
}

pub(super) fn range_clear(
    data: &OriginalStationData,
    station: StationPose,
    player: RigidPose,
) -> bool {
    if player.position.dot(player.position) > 1e18 {
        return false;
    }
    let center = add(player.position, player.orientation.apply(VOLUME_OFFSET));
    let distance = |other| {
        let delta = sub(center, other);
        delta.dot(delta).sqrt()
    };
    // The two canonical exclusion anchors are in the original station-centered
    // frame. They are used only for range admission in this frozen scene.
    distance(player.position) > VOLUME_RADIUS + data.player_clearance_radius() + 64.0
        && distance(Vec3::new(2600.0, -700.0, 7500.0)) > VOLUME_RADIUS + 1500.0
        && distance(Vec3::new(-5000.0, 1500.0, 12500.0)) > VOLUME_RADIUS + 4.0 * 1100.0
        && data.practice_sphere_clears_station(station, center, VOLUME_RADIUS)
}

pub(super) fn nearest(
    frame: RigidPose,
    ray: RigidPose,
    station_distance: Option<f64>,
) -> Option<(usize, f64)> {
    let cap = station_distance.unwrap_or(crate::PULSE_RANGE);
    let mut best = None;
    for ordinal in 0..OFFSETS.len() {
        let beacon = beacon_pose(frame, ordinal);
        if let Some(distance) = cube_surface(ray, beacon, cap)
            && station_distance.is_none_or(|station| distance < station)
            && best.is_none_or(|(_, nearest)| distance < nearest)
        {
            best = Some((ordinal, distance));
        }
    }
    best
}

// Slab intersection of the six source cube planes. Starting within a cube
// selects its exit surface. The face at the origin and exact range are included.
fn cube_surface(ray: RigidPose, beacon: RigidPose, limit: f64) -> Option<f64> {
    let offset = sub(ray.position, beacon.position);
    let mut enter = f64::NEG_INFINITY;
    let mut exit = f64::INFINITY;
    for axis in [
        beacon.orientation.x,
        beacon.orientation.y,
        beacon.orientation.z,
    ] {
        let p = offset.dot(axis);
        let d = ray.orientation.z.dot(axis);
        if d.abs() <= 1e-12 {
            if p.abs() > PRACTICE_HALF_SIDE {
                return None;
            }
            continue;
        }
        let a = (-PRACTICE_HALF_SIDE - p) / d;
        let b = (PRACTICE_HALF_SIDE - p) / d;
        enter = enter.max(a.min(b));
        exit = exit.min(a.max(b));
        if enter > exit {
            return None;
        }
    }
    let distance = if enter < 0.0 { exit } else { enter };
    (distance.is_finite() && (0.0..=limit).contains(&distance)).then_some(distance)
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

use super::{
    GoshawkApproachSnapshot, GoshawkApproachStatus as Status, GoshawkHullApproachSnapshot, World,
};
use crate::save::SaveError;
use crate::{FlightInput, FlightStart, StationError, StepError};
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

const OBSERVER: FlightStart = FlightStart {
    position: [0.0, 0.0, 16_000.0],
    orientation: [0.0, 0.0, 0.0, 1.0],
    speed: 0.0,
};

fn inbound(x: f32, z: f32, speed: f32) -> FlightStart {
    FlightStart {
        position: [x, 0.0, z],
        orientation: [0.0, 1.0, 0.0, 0.0],
        speed,
    }
}

fn frame(world: &World) -> Result<GoshawkApproachSnapshot, StepError> {
    world
        .goshawk_approach_snapshot()
        .ok_or(StepError::InvalidMode)
}

fn run(
    label: &str,
    observer: FlightStart,
    approach: FlightStart,
    expected: Status,
) -> Result<(), Box<dyn Error>> {
    let world = World::goshawk_docking_validation_scenario(observer, approach)?;
    run_world(label, world, expected, World::goshawk_approach_snapshot)
}

fn run_world<const N: usize, E>(
    label: &str,
    mut world: World,
    expected: Status,
    snapshot: fn(&World) -> Option<GoshawkHullApproachSnapshot<N, E>>,
) -> Result<(), Box<dyn Error>> {
    let frame = |world: &World| snapshot(world).ok_or(StepError::InvalidMode);
    let plane = world
        .station_port()
        .ok_or(StationError::InvalidPort)?
        .center()
        .z;
    let mut before = frame(&world)?;
    let mut samples = Vec::new();

    let terminal = loop {
        assert!(samples.len() < 160, "{label} exceeded its approach horizon");
        let started = Instant::now();
        let result = world.step(FlightInput::default());
        samples.push(started.elapsed().as_nanos());
        result?;
        let after = frame(&world)?;

        assert_eq!(after.world_tick(), before.world_tick() + 1);
        assert_eq!(after.pose().orientation, before.pose().orientation);
        assert_eq!(after.pose().position.x, before.pose().position.x);
        assert_eq!(after.pose().position.y, before.pose().position.y);
        assert!(Arc::ptr_eq(before.pieces(), after.pieces()));
        if after.status() != Status::Approaching {
            break after;
        }
        assert!(after.pose().position.z < before.pose().position.z);
        assert!(after.pose().position.z > plane);
        before = after;
    };

    assert_eq!(terminal.status(), expected, "{label}");
    assert_eq!(terminal.speed_units(), 0.0);
    assert!(before.pose().position.z > plane);
    if expected == Status::Entered {
        assert!(terminal.pose().position.z <= plane);
    } else {
        assert_eq!(terminal.pose(), before.pose());
    }

    world.step(FlightInput::default())?;
    let stopped = frame(&world)?;
    assert_eq!(stopped.pose(), terminal.pose());
    assert_eq!(stopped.status(), expected);

    let mut ordered = samples.clone();
    ordered.sort_unstable();
    let count = ordered.len();
    let median = ordered[(count - 1) / 2].midpoint(ordered[count / 2]);
    let p95 = (count * 95).div_ceil(100) - 1;
    eprintln!(
        "GOSHAWK_WORLD_COLLISION_COST route={label} status={expected:?} ticks={count} median_ns={median} p95_ns={} max_ns={} terminal_ns={}",
        ordered[p95],
        ordered[count - 1],
        samples[count - 1],
    );
    Ok(())
}

#[test]
fn goshawk_world_approach_reports_physical_contact_and_entry_clearance()
-> Result<(), Box<dyn Error>> {
    run(
        "aligned",
        OBSERVER,
        inbound(0.0, 220.0, 4.0),
        Status::Entered,
    )?;
    // Exact source maximum at the door is strictly below eight speed units.
    run(
        "overspeed",
        OBSERVER,
        inbound(0.0, 220.0, 9.0),
        Status::EntryConditions,
    )?;
    // Independent SH028 lip witness locates SH008 rail vertex 6 inside SH028 face 482.
    run(
        "lip",
        OBSERVER,
        inbound(60.809_685, 220.0, 12.0),
        Status::StationContact,
    )?;
    // Source-backed clear sweep outside the opening isolates entry policy from contact.
    run(
        "off-aperture",
        OBSERVER,
        inbound(100.0, 220.0, 4.0),
        Status::EntryConditions,
    )?;

    let closer_observer = FlightStart {
        position: [0.0, 0.0, 360.0],
        ..OBSERVER
    };
    run(
        "observer",
        closer_observer,
        inbound(0.0, 520.0, 8.0),
        Status::ObserverContact,
    )?;
    Ok(())
}

#[test]
fn goshawk_world_snapshots_keep_source_and_scope_without_a_commander() -> Result<(), Box<dyn Error>>
{
    assert!(World::new().goshawk_approach_snapshot().is_none());
    let start = inbound(0.0, 220.0, 4.0);
    let mut world = World::goshawk_docking_validation_scenario(OBSERVER, start)?;
    let other = World::goshawk_docking_validation_scenario(OBSERVER, start)?;
    let first = frame(&world)?;
    assert_eq!(first.pieces().len(), 69);
    assert_eq!(
        first.source_identity(),
        (
            "aafcb5dfec2cb5d141316756daae267888bf3541383f4f155c2561a4d7dd0bb7",
            "6815e92d5480692d18576ab5ed5e98cd1f99b1bff622c3c7eb346ed59bcf8d42",
        )
    );
    assert!(world.goshawk_snapshot_is_current(&first));
    assert!(!other.goshawk_snapshot_is_current(&first));
    assert!(other.goshawk_observer_pose(&first).is_none());
    assert!(world.commander_id().is_none());
    assert!(world.player_ship_id().is_none());
    assert!(world.current_dock_context().is_none());
    assert_eq!(world.save_bytes(), Err(SaveError::NoCommander));

    let observer = world
        .goshawk_observer_pose(&first)
        .ok_or(StepError::InvalidMode)?;
    let input = FlightInput {
        yaw: 0.5,
        ..FlightInput::default()
    };
    assert_eq!(world.step(input), Err(StepError::InvalidMode));
    assert_eq!(frame(&world)?.pose(), first.pose());
    assert!(world.goshawk_snapshot_is_current(&first));

    world.step(FlightInput::default())?;
    let second = frame(&world)?;
    assert!(!world.goshawk_snapshot_is_current(&first));
    assert!(world.goshawk_observer_pose(&first).is_none());
    assert_eq!(world.goshawk_observer_pose(&second), Some(observer));
    assert!(Arc::ptr_eq(first.pieces(), second.pieces()));
    Ok(())
}

// These independent bit patterns come from the adopted baseline in the retained
// MC1 proposal. The proposal also records the separately charged liner alternative.
const PROPOSAL_CENTER_BITS: [u64; 3] = [
    0xbeeb_7eef_cbd4_29e1,
    0x3fd3_f41e_356c_f7c6,
    0xc002_e69d_21d0_7f38,
];
const PROPOSAL_INERTIA_BITS: [[u64; 3]; 3] = [
    [
        0x4144_2315_a201_7f17,
        0x400b_f138_abb8_9cbf,
        0xc02d_3bf5_d7de_de95,
    ],
    [
        0x400b_f138_abb8_9cbf,
        0x414c_8a2d_eb7c_b2cb,
        0x40f2_b168_40d7_add0,
    ],
    [
        0xc02d_3bf5_d7de_de95,
        0x40f2_b168_40d7_add0,
        0x4133_eae6_dd58_4f58,
    ],
];

#[test]
fn goshawk_world_publishes_the_adopted_stock_mass_properties() -> Result<(), Box<dyn Error>> {
    let mut world = World::goshawk_docking_validation_scenario(OBSERVER, inbound(6.0, 220.0, 4.0))?;
    let first = frame(&world)?;
    let properties = first.engineering();
    assert_eq!(properties.dry_mass_kilograms, 33_719);
    assert_eq!(
        properties.center_of_mass_metres.map(f64::to_bits),
        PROPOSAL_CENTER_BITS
    );
    assert_eq!(
        properties
            .inertia_about_center_kg_m2
            .map(|row| row.map(f64::to_bits)),
        PROPOSAL_INERTIA_BITS,
    );
    assert_eq!(
        properties.proposal_sha256,
        "c086123c0e38c12189719bbbcb36120da8c3099e0e959f335520de2851eceb55"
    );

    // The inbound half-turn around Y reverses the local X and Z offsets only.
    let [x, y, z] = PROPOSAL_CENTER_BITS.map(f64::from_bits);
    let center = first.center_of_mass_world_metres();
    assert_eq!((center.x, center.y, center.z), (6.0 - x, y, 220.0 - z));
    world.step(FlightInput::default())?;
    let next = frame(&world)?;
    assert!(world.goshawk_snapshot_is_current(&next));
    assert!(core::ptr::eq(next.engineering(), properties));
    let position = next.pose().position;
    let center = next.center_of_mass_world_metres();
    assert!(position.z < 220.0);
    assert_eq!(
        (center.x, center.y, center.z),
        (position.x - x, position.y + y, position.z - z)
    );
    Ok(())
}

#[test]
fn goshawk_world_rotates_the_stock_center_forward_at_oblique_yaw() -> Result<(), Box<dyn Error>> {
    let approach = FlightStart {
        position: [6.0, 4.0, 520.0],
        orientation: [0.0, 0.8, 0.0, 0.6],
        speed: 4.0,
    };
    let world = World::goshawk_docking_validation_scenario(OBSERVER, approach)?;
    let center = frame(&world)?.center_of_mass_world_metres();
    let [x, y, z] = PROPOSAL_CENTER_BITS.map(f64::from_bits);
    // For this yaw, sine is 24/25 and cosine is -7/25. An inverse rotation
    // gives the opposite sign for both sine terms. Allow for the f32 input.
    assert!((center.x - (6.0 - 0.28 * x + 0.96 * z)).abs() < 1e-6);
    assert_eq!(center.y, 4.0 + y);
    assert!((center.z - (520.0 - 0.96 * x - 0.28 * z)).abs() < 1e-6);
    Ok(())
}

#[test]
fn goshawk_world_refuses_unbound_motion_and_overlapping_starts() {
    let starts = [
        inbound(0.0, 220.0, 41.0_f32.next_up()),
        inbound(0.0, 220.0, 0.0),
        inbound(0.0, 140.0, 4.0),
        inbound(0.0, 16_000.0, 4.0),
        FlightStart {
            orientation: OBSERVER.orientation,
            ..inbound(0.0, 220.0, 4.0)
        },
    ];
    for start in starts {
        assert!(matches!(
            World::goshawk_docking_validation_scenario(OBSERVER, start),
            Err(StationError::InvalidPose)
        ));
    }
    assert!(matches!(
        World::goshawk_docking_validation_scenario(
            FlightStart {
                speed: 1.0,
                ..OBSERVER
            },
            inbound(0.0, 220.0, 4.0),
        ),
        Err(StationError::InvalidPose)
    ));
}

#[test]
fn custom_world_approach_reports_its_own_physical_contacts_and_entry() -> Result<(), Box<dyn Error>>
{
    let closer_observer = FlightStart {
        position: [0.0, 0.0, 360.0],
        ..OBSERVER
    };
    let cases = [
        (
            "custom-aligned",
            OBSERVER,
            inbound(0.0, 220.0, 4.0),
            Status::Entered,
        ),
        (
            "custom-overspeed",
            OBSERVER,
            inbound(0.0, 220.0, 9.0),
            Status::EntryConditions,
        ),
        (
            "custom-lip",
            OBSERVER,
            inbound(60.883_217, 220.0, 12.0),
            Status::StationContact,
        ),
        (
            "custom-off-aperture",
            OBSERVER,
            inbound(100.0, 220.0, 4.0),
            Status::EntryConditions,
        ),
        (
            "custom-observer",
            closer_observer,
            inbound(0.0, 520.0, 8.0),
            Status::ObserverContact,
        ),
    ];
    for (label, observer, approach, expected) in cases {
        let world = World::goshawk_custom_docking_validation_scenario(observer, approach)?;
        assert!(world.goshawk_approach_snapshot().is_none());
        run_world(
            label,
            world,
            expected,
            World::goshawk_custom_approach_snapshot,
        )?;
    }
    Ok(())
}

#[test]
fn custom_world_enforces_own_r3_speed_and_snapshot_scope() -> Result<(), Box<dyn Error>> {
    use crate::goshawk_custom::{CUSTOM_CONTRACT_SHA256, CUSTOM_RESOURCE_SHA256};
    assert!(World::new().goshawk_custom_approach_snapshot().is_none());
    let start = inbound(0.0, 220.0, 4.0);
    let mut world = World::goshawk_custom_docking_validation_scenario(OBSERVER, start)?;
    let other = World::goshawk_custom_docking_validation_scenario(OBSERVER, start)?;
    let explorer = World::goshawk_docking_validation_scenario(OBSERVER, start)?;
    let first = world
        .goshawk_custom_approach_snapshot()
        .ok_or(StepError::InvalidMode)?;
    assert!(world.goshawk_approach_snapshot().is_none());
    assert!(explorer.goshawk_custom_approach_snapshot().is_none());
    assert!(!explorer.goshawk_custom_snapshot_is_current(&first));
    assert!(!other.goshawk_custom_snapshot_is_current(&first));
    assert!(world.goshawk_custom_snapshot_is_current(&first));
    assert_eq!(first.pieces().len(), 73);
    assert_eq!(
        first.source_identity(),
        (CUSTOM_RESOURCE_SHA256, CUSTOM_CONTRACT_SHA256)
    );
    assert!(world.commander_id().is_none() && world.player_ship_id().is_none());
    assert_eq!(world.save_bytes(), Err(SaveError::NoCommander));
    let observer = world
        .goshawk_custom_observer_pose(&first)
        .ok_or(StepError::InvalidMode)?;
    let input = FlightInput {
        yaw: 0.5,
        ..FlightInput::default()
    };
    assert_eq!(world.step(input), Err(StepError::InvalidMode));
    assert!(world.goshawk_custom_snapshot_is_current(&first));
    world.step(FlightInput::default())?;
    let second = world
        .goshawk_custom_approach_snapshot()
        .ok_or(StepError::InvalidMode)?;
    assert!(!world.goshawk_custom_snapshot_is_current(&first));
    assert!(world.goshawk_custom_observer_pose(&first).is_none());
    assert_eq!(world.goshawk_custom_observer_pose(&second), Some(observer));
    assert!(Arc::ptr_eq(first.pieces(), second.pieces()));
    // R3's inclusive 40.0 controlled speed comes from the pinned SH010 contract.
    // Exercise the actual constructor and step at the boundary, independent of
    // the ordinary station entry speed limit.
    let mut maximum =
        World::goshawk_custom_docking_validation_scenario(OBSERVER, inbound(0.0, 220.0, 40.0))?;
    let at_limit = maximum
        .goshawk_custom_approach_snapshot()
        .ok_or(StepError::InvalidMode)?;
    assert_eq!(at_limit.speed_units(), 40.0);
    assert_eq!(at_limit.status(), Status::Approaching);
    assert_eq!(
        at_limit.source_identity(),
        (CUSTOM_RESOURCE_SHA256, CUSTOM_CONTRACT_SHA256)
    );
    maximum.step(FlightInput::default())?;
    let advanced = maximum
        .goshawk_custom_approach_snapshot()
        .ok_or(StepError::InvalidMode)?;
    assert_eq!(advanced.speed_units(), 40.0);
    assert_eq!(advanced.status(), Status::Approaching);
    assert_eq!(advanced.world_tick(), at_limit.world_tick() + 1);
    assert!(advanced.pose().position.z < at_limit.pose().position.z);
    for speed in [0.0, 40.0_f32.next_up()] {
        assert!(matches!(
            World::goshawk_custom_docking_validation_scenario(OBSERVER, inbound(0.0, 220.0, speed)),
            Err(StationError::InvalidPose)
        ));
    }
    Ok(())
}

#[test]
fn raven_world_approach_reports_its_own_physical_contacts_and_entry() -> Result<(), Box<dyn Error>>
{
    let closer_observer = FlightStart {
        position: [0.0, 0.0, 360.0],
        ..OBSERVER
    };
    let cases = [
        (
            "raven-aligned",
            OBSERVER,
            inbound(0.0, 220.0, 4.0),
            Status::Entered,
        ),
        (
            "raven-overspeed",
            OBSERVER,
            inbound(0.0, 220.0, 9.0),
            Status::EntryConditions,
        ),
        (
            "raven-lip",
            OBSERVER,
            inbound(70.328_20, 220.0, 12.0),
            Status::StationContact,
        ),
        (
            "raven-off-aperture",
            OBSERVER,
            inbound(100.0, 220.0, 4.0),
            Status::EntryConditions,
        ),
        (
            "raven-observer",
            closer_observer,
            inbound(0.0, 520.0, 8.0),
            Status::ObserverContact,
        ),
    ];
    for (label, observer, approach, expected) in cases {
        let world = World::raven_docking_validation_scenario(observer, approach)?;
        assert!(world.goshawk_approach_snapshot().is_none());
        assert!(world.goshawk_custom_approach_snapshot().is_none());
        run_world(label, world, expected, World::raven_approach_snapshot)?;
    }
    Ok(())
}

#[test]
fn raven_world_enforces_own_r3_speed_and_snapshot_scope() -> Result<(), Box<dyn Error>> {
    use crate::raven::{RAVEN_CONTRACT_SHA256, RAVEN_RESOURCE_SHA256};
    assert!(World::new().raven_approach_snapshot().is_none());
    let start = inbound(0.0, 220.0, 4.0);
    let mut world = World::raven_docking_validation_scenario(OBSERVER, start)?;
    let other = World::raven_docking_validation_scenario(OBSERVER, start)?;
    let explorer = World::goshawk_docking_validation_scenario(OBSERVER, start)?;
    let custom = World::goshawk_custom_docking_validation_scenario(OBSERVER, start)?;
    let first = world
        .raven_approach_snapshot()
        .ok_or(StepError::InvalidMode)?;
    assert!(world.goshawk_approach_snapshot().is_none());
    assert!(world.goshawk_custom_approach_snapshot().is_none());
    assert!(explorer.raven_approach_snapshot().is_none());
    assert!(custom.raven_approach_snapshot().is_none());
    assert!(!custom.raven_snapshot_is_current(&first));
    let explorer_frame = explorer
        .goshawk_approach_snapshot()
        .ok_or(StepError::InvalidMode)?;
    let custom_frame = custom
        .goshawk_custom_approach_snapshot()
        .ok_or(StepError::InvalidMode)?;
    assert!(!world.goshawk_snapshot_is_current(&explorer_frame));
    assert!(!world.goshawk_custom_snapshot_is_current(&custom_frame));
    assert_eq!(explorer_frame.engineering().dry_mass_kilograms, 33_719);
    assert_eq!(custom_frame.pieces().len(), 73);
    assert!(!explorer.raven_snapshot_is_current(&first));
    assert!(!other.raven_snapshot_is_current(&first));
    assert!(world.raven_snapshot_is_current(&first));
    assert_eq!(first.pieces().len(), 70);
    assert_eq!(
        first.source_identity(),
        (RAVEN_RESOURCE_SHA256, RAVEN_CONTRACT_SHA256)
    );
    assert!(world.commander_id().is_none() && world.player_ship_id().is_none());
    assert_eq!(world.save_bytes(), Err(SaveError::NoCommander));
    let observer = world
        .raven_observer_pose(&first)
        .ok_or(StepError::InvalidMode)?;
    let input = FlightInput {
        yaw: 0.5,
        ..FlightInput::default()
    };
    assert_eq!(world.step(input), Err(StepError::InvalidMode));
    assert!(world.raven_snapshot_is_current(&first));
    world.step(FlightInput::default())?;
    let second = world
        .raven_approach_snapshot()
        .ok_or(StepError::InvalidMode)?;
    assert!(!world.raven_snapshot_is_current(&first));
    assert!(world.raven_observer_pose(&first).is_none());
    assert_eq!(world.raven_observer_pose(&second), Some(observer));
    assert!(Arc::ptr_eq(first.pieces(), second.pieces()));
    // R3's inclusive 46.0 controlled speed comes from the pinned SH015 contract.
    // Exercise the actual constructor and step at the boundary, independent of
    // the ordinary station entry speed limit.
    let mut maximum =
        World::raven_docking_validation_scenario(OBSERVER, inbound(0.0, 220.0, 46.0))?;
    let at_limit = maximum
        .raven_approach_snapshot()
        .ok_or(StepError::InvalidMode)?;
    assert_eq!(at_limit.speed_units(), 46.0);
    assert_eq!(at_limit.status(), Status::Approaching);
    assert_eq!(
        at_limit.source_identity(),
        (RAVEN_RESOURCE_SHA256, RAVEN_CONTRACT_SHA256)
    );
    maximum.step(FlightInput::default())?;
    let advanced = maximum
        .raven_approach_snapshot()
        .ok_or(StepError::InvalidMode)?;
    assert_eq!(advanced.speed_units(), 46.0);
    assert_eq!(advanced.status(), Status::Approaching);
    assert_eq!(advanced.world_tick(), at_limit.world_tick() + 1);
    assert!(advanced.pose().position.z < at_limit.pose().position.z);
    for speed in [0.0, 46.0_f32.next_up()] {
        assert!(matches!(
            World::raven_docking_validation_scenario(OBSERVER, inbound(0.0, 220.0, speed)),
            Err(StationError::InvalidPose)
        ));
    }
    Ok(())
}

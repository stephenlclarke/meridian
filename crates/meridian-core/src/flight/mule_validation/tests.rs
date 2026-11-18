use super::*;
use crate::traffic::MuleValidationRefusal;
use std::sync::Arc;

fn observer() -> FlightStart {
    FlightStart {
        position: [0.0, 0.0, 16000.0],
        orientation: [0.0, 1.0, 0.0, 0.0],
        speed: 0.0,
    }
}

#[test]
fn mule_world_publication_binds_actual_actor_pose_surface_scope_and_generation()
-> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::mule_validation_scenario(117, observer())?;
    let other = World::mule_validation_scenario(117, observer())?;
    let first = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
    let foreign = other.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
    assert_eq!(first.actor(), foreign.actor());
    assert!(world.mule_snapshot_is_current(&first));
    assert!(!world.mule_snapshot_is_current(&foreign));
    assert!(world.mule_observer_pose(&foreign).is_none());
    assert!(!other.mule_snapshot_is_current(&first));
    assert_eq!(first.actor().hull_id, "SH001");
    assert_eq!(first.actor().id.serial(), u8::MAX);
    assert_eq!(first.actor().front_shield_hundredths, 24000);
    assert_eq!(first.actor().rear_shield_hundredths, 24000);
    assert_eq!(first.presentation().front_shield_fraction, 1.0);
    assert_eq!(first.presentation().rear_shield_fraction, 1.0);
    assert_eq!(
        first.source_identity(),
        (
            crate::mule::MULE_RESOURCE_SHA256,
            crate::mule::MULE_CONTRACT_SHA256
        )
    );
    let frozen_observer = world.snapshot();
    assert_eq!(
        world.mule_observer_pose(&first),
        flight_pose(frozen_observer.position, frozen_observer.orientation)
    );
    world.step(FlightInput::default())?;
    let second = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
    assert_eq!(first.actor().id, second.actor().id);
    assert_eq!(second.generation(), first.generation() + 1);
    assert_eq!(second.world_tick(), first.world_tick() + 1);
    assert!(second.actor().pose.position.z > first.actor().pose.position.z);
    assert_eq!(world.snapshot().position, frozen_observer.position);
    assert_eq!(world.snapshot().orientation, frozen_observer.orientation);
    assert!(!world.mule_snapshot_is_current(&first));
    assert!(world.mule_observer_pose(&first).is_none());
    assert!(world.mule_snapshot_is_current(&second));
    assert!(Arc::ptr_eq(first.presentation(), second.presentation()));
    assert!(Arc::ptr_eq(
        &first.presentation().pieces,
        &second.presentation().pieces
    ));
    drop(world);
    assert!(!other.mule_snapshot_is_current(&second));
    Ok(())
}

#[test]
fn mule_validation_refuses_observer_commands_and_grants_no_profile_or_save()
-> Result<(), Box<dyn std::error::Error>> {
    assert!(World::mule_validation_scenario(0, observer()).is_err());
    let mut moving_observer = observer();
    moving_observer.speed = 1.0;
    assert!(World::mule_validation_scenario(117, moving_observer).is_err());
    let source = MuleHullSource::bundled()?;
    let data = crate::OriginalStationData::bundled()?;
    let mut overlapping_observer = data.mule_launch_start(&source)?;
    overlapping_observer.speed = 0.0;
    assert!(World::mule_validation_scenario(117, overlapping_observer).is_err());
    let mut world = World::mule_validation_scenario(117, observer())?;
    let before = world.snapshot();
    let frame = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
    assert_eq!(
        world.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        }),
        Err(StepError::InvalidMode)
    );
    assert_eq!(
        world.step(FlightInput {
            yaw: f32::NAN,
            ..FlightInput::default()
        }),
        Err(StepError::NonFiniteInput)
    );
    assert_eq!(world.snapshot(), before);
    assert!(world.mule_snapshot_is_current(&frame));
    assert!(world.commander_id().is_none());
    assert!(world.player_ship_id().is_none());
    assert!(world.traffic_snapshot().is_none());
    assert!(world.current_dock_context().is_none());
    assert_eq!(frame.actor().cash_tenths, 0);
    assert_eq!(frame.actor().cargo, None);
    assert_eq!(frame.actor().emp_crew_aboard, None);
    assert_eq!(world.save_bytes(), Err(crate::save::SaveError::NoCommander));
    Ok(())
}

#[test]
fn mule_world_sweep_stops_before_the_actual_stationary_observer_mesh()
-> Result<(), Box<dyn std::error::Error>> {
    let source = MuleHullSource::bundled()?;
    let data = crate::OriginalStationData::bundled()?;
    let launch = data.mule_launch_start(&source)?;
    let mut view = observer();
    view.position[2] = launch.position[2] + 1000.0;
    let mut world = World::mule_validation_scenario(119, view)?;
    let mut stopped = false;
    for _ in 0..512 {
        let before = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
        world.step(FlightInput::default())?;
        let after = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
        if after.motion_refusal().is_some() {
            assert_eq!(
                after.motion_refusal(),
                Some(MuleValidationRefusal::ObserverContact)
            );
            assert_eq!(after.actor().pose, before.actor().pose);
            assert_eq!(after.actor().speed_units, 0.0);
            world.step(FlightInput::default())?;
            let held = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
            assert_eq!(held.actor().pose, after.actor().pose);
            assert!(held.generation() > after.generation());
            assert!(!world.mule_snapshot_is_current(&after));
            stopped = true;
            break;
        }
    }
    assert!(stopped, "Mule must not pass through the physical observer");
    Ok(())
}

#[test]
fn ordinary_world_and_current_save_do_not_acquire_a_validation_mule()
-> Result<(), Box<dyn std::error::Error>> {
    assert!(World::new().mule_actor_snapshot().is_none());
    let world = World::new_commander(733)?;
    assert!(world.mule_actor_snapshot().is_none());
    let bytes = world.save_bytes()?;
    // Checked decode preserves the encoded domain. Public Continue deliberately
    // invalidates old UI reviews/session contact and therefore changes two counters.
    let restored = World::decode_saved_world(&bytes)?;
    assert!(restored.mule_actor_snapshot().is_none());
    assert_eq!(restored.save_bytes()?, bytes);

    let session = world
        .freight_session
        .as_ref()
        .ok_or(crate::save::SaveError::NoCommander)?;
    let (epoch, review_sequence) = session.continue_floor();
    let mut state_record = crate::save::Writer::new();
    session.state.encode_save(&mut state_record);
    let mut session_record = crate::save::Writer::new();
    session.encode_save(&mut session_record);
    let matches: Vec<_> = bytes
        .windows(session_record.bytes.len())
        .enumerate()
        .filter_map(|(offset, record)| {
            (record == session_record.bytes.as_slice()).then_some(offset)
        })
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "the complete authored freight session must be unique"
    );
    // State writes review_sequence last; the containing session writes epoch next.
    let review_offset = matches[0] + state_record.bytes.len() - 8;
    let epoch_offset = review_offset + 8;
    assert_eq!(
        &bytes[review_offset..review_offset + 8],
        &review_sequence.to_le_bytes()
    );
    assert_eq!(&bytes[epoch_offset..epoch_offset + 8], &epoch.to_le_bytes());
    let continued = World::from_save_bytes(&bytes)?;
    assert!(continued.mule_actor_snapshot().is_none());
    assert_eq!(continued.snapshot(), world.snapshot());
    assert_eq!(
        continued
            .freight_session
            .as_ref()
            .ok_or(crate::save::SaveError::NoCommander)?
            .continue_floor(),
        (epoch + 1, review_sequence + 1)
    );
    // Independent byte-delta expectation: no other domain/equipment/custody byte
    // may change. Do not normalize the actual result or call the invalidator here.
    let mut expected = bytes.clone();
    expected[review_offset..review_offset + 8]
        .copy_from_slice(&(review_sequence + 1).to_le_bytes());
    expected[epoch_offset..epoch_offset + 8].copy_from_slice(&(epoch + 1).to_le_bytes());
    let checksum_offset = expected.len() - 8;
    let checksum = crate::save::checksum(&expected[..checksum_offset]);
    expected[checksum_offset..].copy_from_slice(&checksum.to_le_bytes());
    assert_eq!(continued.save_bytes()?, expected);
    Ok(())
}

fn inbound(x: f32, speed: f32) -> FlightStart {
    FlightStart {
        position: [x, 0.0, 260.0],
        orientation: [0.0, 1.0, 0.0, 0.0],
        speed,
    }
}

#[test]
fn mule_docking_accepts_the_source_speed_cap_and_rejects_the_next_f32()
-> Result<(), Box<dyn std::error::Error>> {
    // Literal from the immutable SH001 performance contract, held independent of
    // the production constant so a cap change cannot move this boundary fixture.
    let advertised_cap = 33.2_f32;
    let approach = inbound(0.0, advertised_cap);
    let world = World::mule_docking_validation_scenario(733, observer(), approach)?;
    let frame = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
    assert_eq!(frame.actor().speed_units, f64::from(advertised_cap));

    let too_fast = FlightStart {
        speed: advertised_cap.next_up(),
        ..approach
    };
    assert!(matches!(
        World::mule_docking_validation_scenario(733, observer(), too_fast),
        Err(StationError::InvalidPose)
    ));
    Ok(())
}

#[test]
fn mule_world_docking_accepts_aligned_crossing_and_refuses_speed_and_wall_contact()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::traffic::NpcPhase;
    use std::time::Instant;

    let mut inside = inbound(0.0, 4.0);
    inside.position[2] = 136.0;
    assert!(World::mule_docking_validation_scenario(733, observer(), inside).is_err());

    for (case, start, expected_refusal) in [
        ("aligned", inbound(0.0, 4.0), None),
        (
            "overspeed",
            inbound(0.0, 9.0),
            Some(MuleValidationRefusal::EntryConditions),
        ),
        (
            "sidewall",
            inbound(100.0, 33.2),
            Some(MuleValidationRefusal::StationContact),
        ),
    ] {
        let mut world = World::mule_docking_validation_scenario(733, observer(), start)?;
        let port_z = world
            .station_port()
            .ok_or(StepError::InvalidMode)?
            .center()
            .z;
        let original = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
        let mut durations = Vec::new();
        let mut terminal = None;
        let mut terminal_nanos = 0;
        for _ in 0..160 {
            let before = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
            assert_eq!(before.actor().phase, NpcPhase::Approach);
            assert!(before.actor().pose.position.z > port_z);
            let clock = Instant::now();
            world.step(FlightInput::default())?;
            let elapsed = clock.elapsed().as_nanos();
            let after = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
            if after.actor().phase == NpcPhase::Complete || after.motion_refusal().is_some() {
                assert_eq!(after.motion_refusal(), expected_refusal);
                assert_eq!(after.actor().speed_units, 0.0);
                if expected_refusal.is_some() {
                    assert_eq!(after.actor().phase, NpcPhase::Approach);
                    assert_eq!(after.actor().pose, before.actor().pose);
                } else {
                    assert_eq!(after.actor().phase, NpcPhase::Complete);
                    assert!(after.actor().pose.position.z <= port_z);
                    assert_eq!(
                        after.actor().pose.orientation,
                        before.actor().pose.orientation
                    );
                }
                assert_eq!(after.actor().id, original.actor().id);
                assert!(Arc::ptr_eq(after.presentation(), original.presentation()));
                assert!(world.mule_snapshot_is_current(&after));
                assert!(!world.mule_snapshot_is_current(&before));
                assert!(world.current_dock_context().is_none());
                assert_eq!(world.save_bytes(), Err(crate::save::SaveError::NoCommander));
                terminal = Some(after);
                terminal_nanos = elapsed;
                break;
            }
            durations.push(elapsed);
            assert!(after.actor().pose.position.z < before.actor().pose.position.z);
        }
        let terminal = terminal.ok_or(StepError::InvalidMode)?;
        assert!(!durations.is_empty());
        world.step(FlightInput::default())?;
        let held = world.mule_actor_snapshot().ok_or(StepError::InvalidMode)?;
        assert_eq!(held.actor().pose, terminal.actor().pose);
        assert_eq!(held.actor().phase, terminal.actor().phase);
        assert_eq!(held.motion_refusal(), terminal.motion_refusal());
        assert!(!world.mule_snapshot_is_current(&terminal));
        durations.sort_unstable();
        let p95_index = (durations.len() * 95).div_ceil(100) - 1;
        eprintln!(
            "MULE_WORLD_DOCK_COST {{\"case\":\"{case}\",\"approachTicks\":{},\"medianNanoseconds\":{},\"p95Nanoseconds\":{},\"maxNanoseconds\":{},\"terminalNanoseconds\":{terminal_nanos}}}",
            durations.len(),
            durations[durations.len() / 2],
            durations[p95_index],
            durations[durations.len() - 1],
        );
    }
    Ok(())
}

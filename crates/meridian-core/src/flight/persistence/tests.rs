#![allow(clippy::unwrap_used)]
use super::*;
use crate::freight::g02::{self, ActorId, Choice, Command, Event, GoodsId, JobId, Review};
use crate::navigation::{BASIC_SPOOL_TICKS, DockOrigin};
use crate::station_services;

fn action(world: &mut World, value: Command) -> Result<Event, MissionError> {
    world.request_freight(value, world.snapshot().tick)
}
fn review(world: &mut World, value: Command) -> Review {
    match action(world, value).unwrap() {
        Event::Reviewed(r) => r,
        other => panic!("expected review {other:?}"),
    }
}
fn contact(world: &mut World) -> ActorId {
    let npc = world.station_contacts()[0];
    world.request_contact(npc, world.snapshot().tick).unwrap();
    npc.actor()
}
fn accept(world: &mut World) -> (JobId, [GoodsId; 8]) {
    let actor = contact(world);
    let r = review(world, Command::ReviewOffer { actor });
    match action(
        world,
        Command::Confirm {
            review: r,
            choice: Choice::Accept,
        },
    )
    .unwrap()
    {
        Event::Accepted { job, units } => (job, units),
        other => panic!("expected accepted {other:?}"),
    }
}
fn receive(world: &mut World, actor: ActorId, job: JobId, choice: Choice) -> Event {
    let r = review(world, Command::ReviewReceiving { actor, job });
    action(world, Command::Confirm { review: r, choice }).unwrap()
}
fn launch(world: &mut World) {
    world.request_launch(world.snapshot().tick).unwrap();
    world.step(FlightInput::default()).unwrap();
}
fn approach(world: &mut World) {
    for _ in 0..9000 {
        let s = world.snapshot();
        if s.mode == FlightMode::Docked {
            break;
        }
        let speed = if s.position[2] > 2500.0 { 34.3 } else { 4.0 };
        world
            .step(FlightInput {
                throttle: ((speed - s.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..FlightInput::default()
            })
            .unwrap();
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    assert_eq!(world.snapshot().mode, FlightMode::Docked);
    assert_eq!(
        world.current_dock_context().unwrap().origin(),
        DockOrigin::PhysicalEntry
    );
}
fn destination(world: &mut World) {
    launch(world);
    world
        .request_jump(SystemId::RENAS, world.snapshot().tick)
        .unwrap();
    for _ in 0..BASIC_SPOOL_TICKS {
        world.step(FlightInput::default()).unwrap();
    }
    assert!(world.current_dock_context().is_none());
    approach(world);
}
fn fingerprint(
    world: &World,
) -> (
    Snapshot,
    Option<MissionSnapshot>,
    Option<ServiceSnapshot>,
    Option<u32>,
    Vec<StationNpc>,
) {
    (
        world.snapshot(),
        world.mission_snapshot(),
        world.service_snapshot(),
        world.credits_tenths(),
        world.station_contacts(),
    )
}
fn roundtrip(world: &World) -> World {
    let restored = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(restored.snapshot(), world.snapshot());
    assert_eq!(restored.credits_tenths(), world.credits_tenths());
    assert_eq!(
        restored.mission_snapshot().unwrap().freight,
        world.mission_snapshot().unwrap().freight
    );
    assert_eq!(restored.service_snapshot(), world.service_snapshot());
    assert_eq!(
        restored.current_dock_context(),
        world.current_dock_context()
    );
    assert_eq!(restored.commander_id(), world.commander_id());
    assert_eq!(restored.player_ship_id(), world.player_ship_id());
    assert_eq!(restored.station_contacts(), world.station_contacts());
    assert!(restored.mission_snapshot().unwrap().contact.is_none());
    restored
}

#[test]
fn fresh_save_continue_preserves_birth_admission_and_revokes_ui_across_repeated_restores() {
    let mut world = World::new_commander(401).unwrap();
    let actor = contact(&mut world);
    let r = review(&mut world, Command::ReviewOffer { actor });
    let bytes = world.save_bytes().unwrap();
    let dock = world.current_dock_context();
    let mut loaded = roundtrip(&world);
    assert_eq!(
        action(
            &mut loaded,
            Command::Confirm {
                review: r,
                choice: Choice::Accept
            }
        ),
        Err(MissionError::Freight(g02::Error::StaleReview))
    );
    let actor = contact(&mut loaded);
    let next = review(&mut loaded, Command::ReviewOffer { actor });
    loaded.restore_bytes(&bytes).unwrap();
    assert_eq!(loaded.current_dock_context(), dock);
    let actor = contact(&mut loaded);
    let _current = review(&mut loaded, Command::ReviewOffer { actor });
    assert_eq!(
        action(
            &mut loaded,
            Command::Confirm {
                review: next,
                choice: Choice::Accept
            }
        ),
        Err(MissionError::Freight(g02::Error::StaleReview))
    );
    assert_eq!(loaded.credits_tenths(), Some(7200));
    assert_eq!(
        loaded
            .mission_snapshot()
            .unwrap()
            .freight
            .available_source_units,
        16
    );
    assert_eq!(World::new().save_bytes(), Err(SaveError::NoCommander));
    loaded.request_pause(true, loaded.snapshot().tick).unwrap();
    let mut paused = roundtrip(&loaded);
    assert!(paused.mission_snapshot().unwrap().paused);
    let before = fingerprint(&paused);
    assert_eq!(paused.step(FlightInput::default()), Err(StepError::Paused));
    assert_eq!(fingerprint(&paused), before);
}

#[test]
fn seven_unit_continue_pays_only_actual_eighth_and_completed_continue_cannot_repay() {
    let mut world = World::new_commander(501).unwrap();
    world
        .request_station_service(station_services::Command::BuyFood, 0)
        .unwrap();
    let (job, units) = accept(&mut world);
    let source_market = world.service_snapshot().unwrap().current_market;
    let mut world = roundtrip(&world);
    destination(&mut world);
    let actor = contact(&mut world);
    for unit in &units[..7] {
        receive(&mut world, actor, job, Choice::ReceiveOne(*unit));
    }
    let r = review(&mut world, Command::ReviewReceiving { actor, job });
    let balance = world.credits_tenths().unwrap();
    let mut world = roundtrip(&world);
    assert_eq!(world.mission_snapshot().unwrap().freight.held_tonnes, 1);
    assert!(world.mission_snapshot().unwrap().freight.clue.is_none());
    assert_eq!(
        action(
            &mut world,
            Command::Confirm {
                review: r,
                choice: Choice::ReceiveOne(units[7])
            }
        ),
        Err(MissionError::Freight(g02::Error::StaleReview))
    );
    let actor = contact(&mut world);
    let last = review(&mut world, Command::ReviewReceiving { actor, job });
    assert!(matches!(
        action(
            &mut world,
            Command::Confirm {
                review: last,
                choice: Choice::ReceiveOne(units[7])
            }
        ),
        Ok(Event::Received {
            completion: Some(_),
            clue_awarded: true,
            remaining: 0,
            ..
        })
    ));
    assert_eq!(world.credits_tenths(), Some(balance + 7300));
    let clue = world.mission_snapshot().unwrap().freight.clue;
    let done_bytes = world.save_bytes().unwrap();
    let mut world = roundtrip(&world);
    assert_eq!(world.mission_snapshot().unwrap().freight.clue, clue);
    assert_eq!(
        world
            .mission_snapshot()
            .unwrap()
            .freight
            .available_source_units,
        8
    );
    assert_eq!(world.service_snapshot().unwrap().food_tonnes, 1);
    let actor = contact(&mut world);
    assert_eq!(
        action(
            &mut world,
            Command::Confirm {
                review: last,
                choice: Choice::ReceiveAll
            }
        ),
        Err(MissionError::Freight(g02::Error::StaleReview))
    );
    assert_eq!(
        action(&mut world, Command::ReviewReceiving { actor, job }),
        Err(MissionError::Freight(g02::Error::NotActive))
    );
    assert_eq!(world.credits_tenths(), Some(balance + 7300));
    // Both source and destination stocks survive; no fresh-market constructor runs on continue.
    let mut w = Writer::new();
    world.station_markets.as_ref().unwrap().encode_save(&mut w);
    let decoded = StationMarkets::decode_save(&mut Reader::new(&w.bytes)).unwrap();
    assert_eq!(
        Some(decoded.quote(station_services::MarketSite::Avenor)),
        source_market
    );
    let mut bad = done_bytes;
    let end = domain_end(&world);
    let value = u64::from_le_bytes(bad[end - 16..end - 8].try_into().unwrap());
    bad[end - 16..end - 8].copy_from_slice(&(value + 1).to_le_bytes());
    reseal(&mut bad);
    assert_refused_unchanged(&mut world, &bad);
}

#[test]
fn in_flight_spool_continue_preserves_countdown_pose_and_once_only_fuel_charge() {
    let mut reference = World::new_commander(601).unwrap();
    accept(&mut reference);
    launch(&mut reference);
    reference
        .request_jump(SystemId::RENAS, reference.snapshot().tick)
        .unwrap();
    for _ in 0..123 {
        reference.step(FlightInput::default()).unwrap();
    }
    let mut loaded = roundtrip(&reference);
    for _ in 123..BASIC_SPOOL_TICKS {
        reference.step(FlightInput::default()).unwrap();
        loaded.step(FlightInput::default()).unwrap();
        assert_eq!(reference.snapshot(), loaded.snapshot());
    }
    assert_eq!(loaded.snapshot().position, [0.0, 0.0, 16000.0]);
    assert_eq!(loaded.snapshot().navigation.unwrap().fuel_units, 96);
    assert!(loaded.current_dock_context().is_none());
    let mut loaded = roundtrip(&loaded);
    loaded.step(FlightInput::default()).unwrap();
    assert_eq!(loaded.snapshot().navigation.unwrap().fuel_units, 96);
    approach(&mut loaded);
    let mut loaded = roundtrip(&loaded);
    assert_eq!(loaded.request_refuel(loaded.snapshot().tick), Ok(68));
    let loaded = roundtrip(&loaded);
    assert_eq!(loaded.snapshot().navigation.unwrap().fuel_units, 130);
    assert_eq!(loaded.credits_tenths(), Some(7132));
}

fn reseal(bytes: &mut [u8]) {
    let end = bytes.len() - 8;
    let digest = checksum(&bytes[..end]);
    bytes[end..].copy_from_slice(&digest.to_le_bytes());
}
fn assert_refused_unchanged(world: &mut World, bad: &[u8]) {
    let before = fingerprint(world);
    let before_bytes = world.save_bytes().unwrap();
    assert!(world.restore_bytes(bad).is_err());
    assert_eq!(fingerprint(world), before);
    assert_eq!(world.save_bytes().unwrap(), before_bytes);
}
fn account_start(world: &World) -> usize {
    let mut w = Writer::new();
    write_flight(&mut w, &world.state);
    world.navigation.unwrap().encode_save(&mut w);
    18 + CONTENT.len() + w.bytes.len()
}
fn domain_start(world: &World) -> usize {
    let mut w = Writer::new();
    world
        .commander_account
        .as_ref()
        .unwrap()
        .encode_save(&mut w);
    account_start(world) + w.bytes.len()
}
fn domain_end(world: &World) -> usize {
    let mut w = Writer::new();
    world
        .freight_session
        .as_ref()
        .unwrap()
        .state
        .encode_save(&mut w);
    domain_start(world) + w.bytes.len()
}
fn markets_start(world: &World) -> usize {
    let mut w = Writer::new();
    world.freight_session.as_ref().unwrap().encode_save(&mut w);
    domain_start(world) + w.bytes.len()
}

#[test]
fn malformed_oversized_wrong_version_and_changed_content_never_replace_live_world() {
    let mut world = World::new_commander(701).unwrap();
    let actor = contact(&mut world);
    let pending = review(&mut world, Command::ReviewOffer { actor });
    let valid = world.save_bytes().unwrap();
    for length in [0, 1, 7, valid.len() - 1] {
        assert_refused_unchanged(&mut world, &valid[..length]);
    }
    assert_refused_unchanged(&mut world, &vec![0; MAX_SAVE_BYTES + 1]);
    for offset in [0, 8, 10] {
        let mut bad = valid.clone();
        bad[offset] ^= 1;
        reseal(&mut bad);
        assert_refused_unchanged(&mut world, &bad);
    }
    let mut bad = valid.clone();
    bad[100] ^= 1;
    assert_refused_unchanged(&mut world, &bad);
    let mut trailing = valid.clone();
    trailing.splice(trailing.len() - 8..trailing.len() - 8, [0]);
    reseal(&mut trailing);
    assert_refused_unchanged(&mut world, &trailing);
    let mut nan_pose = valid;
    let first_position = 18 + CONTENT.len();
    nan_pose[first_position..first_position + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    reseal(&mut nan_pose);
    assert_refused_unchanged(&mut world, &nan_pose);
    assert!(matches!(
        action(
            &mut world,
            Command::Confirm {
                review: pending,
                choice: Choice::Accept
            }
        ),
        Ok(Event::Accepted { .. })
    ));
}

#[test]
fn semantically_tampered_wallet_cargo_fit_units_stock_and_completion_are_refused() {
    let mut world = World::new_commander(801).unwrap();
    accept(&mut world);
    let valid = world.save_bytes().unwrap();
    let account = account_start(&world);
    let domain = domain_start(&world);
    let market = markets_start(&world);
    // Version-one wire fields, with a valid replacement checksum: this exercises semantic checks.
    for (offset, replacement) in [
        (account + 24, 1_u32.to_le_bytes().to_vec()), // Unsupported extra credit/payment.
        (account + 30, 1_u16.to_le_bytes().to_vec()), // Food absent from the actual market ledger.
        (account + 34, 131_u16.to_le_bytes().to_vec()),
        (account + 37, 999_u64.to_le_bytes().to_vec()), // Foreign inventory owner.
        (account + 61 + 28, 804_u64.to_le_bytes().to_vec()), // Duplicate first item ID.
        (domain + 43, vec![1]),                         // Duplicate first/second goods serial.
        (domain + 109, vec![1]), // Delivered status with eight still-held units/no transfer.
        (market + 25, 0_u64.to_le_bytes().to_vec()), // Stock not explained by real trades.
    ] {
        let mut bad = valid.clone();
        bad[offset..offset + replacement.len()].copy_from_slice(&replacement);
        reseal(&mut bad);
        assert_refused_unchanged(&mut world, &bad);
    }
}

#[test]
fn abandoned_allocations_stay_exhausted_after_continue_without_replacement_stock() {
    let mut world = World::new_commander(901).unwrap();
    let (first, a) = accept(&mut world);
    let actor = world.station_contacts()[0].actor();
    let r = review(&mut world, Command::ReviewAbandonment { actor, job: first });
    action(
        &mut world,
        Command::Confirm {
            review: r,
            choice: Choice::Abandon,
        },
    )
    .unwrap();
    let mut world = roundtrip(&world);
    assert_eq!(
        world
            .mission_snapshot()
            .unwrap()
            .freight
            .available_source_units,
        8
    );
    let (second, b) = accept(&mut world);
    assert!(a.iter().all(|unit| !b.contains(unit)));
    let actor = world.station_contacts()[0].actor();
    let r = review(
        &mut world,
        Command::ReviewAbandonment { actor, job: second },
    );
    action(
        &mut world,
        Command::Confirm {
            review: r,
            choice: Choice::Abandon,
        },
    )
    .unwrap();
    let mut world = roundtrip(&world);
    let actor = contact(&mut world);
    assert_eq!(
        action(&mut world, Command::ReviewOffer { actor }),
        Err(MissionError::Freight(g02::Error::SourceExhausted))
    );
    assert_eq!(
        world
            .mission_snapshot()
            .unwrap()
            .freight
            .available_source_units,
        0
    );
    assert_eq!(world.mission_snapshot().unwrap().freight.held_tonnes, 0);
    assert_eq!(world.credits_tenths(), Some(7200));
    assert!(world.mission_snapshot().unwrap().freight.clue.is_none());
}

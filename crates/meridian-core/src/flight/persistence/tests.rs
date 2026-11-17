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
    assert_eq!(world.active_freight_job(), Some(job));
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
    assert_eq!(world.active_freight_job(), None);
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
    // Preserved Food/account prefix wire fields, with a valid replacement checksum: this exercises semantic checks.
    for (offset, replacement) in [
        (account + 24, 1_u32.to_le_bytes().to_vec()), // Unsupported extra credit/payment.
        (account + 30, 1_u16.to_le_bytes().to_vec()), // Food absent from the actual market ledger.
        (account + 34, 131_u16.to_le_bytes().to_vec()),
        (account + 37, 999_u64.to_le_bytes().to_vec()), // Foreign inventory owner.
        (account + 62 + 29, 804_u64.to_le_bytes().to_vec()), // Duplicate first item ID.
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

#[test]
fn native_commodity_cargo_stock_and_money_continue_together_and_old_version_refuses_atomically() {
    use station_services::{Command as Trade, CommodityId};
    let mut world = World::new_commander(1201).unwrap();
    for id in [
        CommodityId::Gold,
        CommodityId::GemStones,
        CommodityId::Machinery,
    ] {
        world
            .request_station_service(Trade::BuyCommodity(id), world.snapshot().tick)
            .unwrap();
    }
    assert_eq!(world.credits_tenths(), Some(646));
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 1);
    let restored = roundtrip(&world);
    for id in CommodityId::ALL {
        assert_eq!(
            restored.commodity_quantity(id),
            world.commodity_quantity(id)
        );
        assert_eq!(restored.commodity_quote(id), world.commodity_quote(id));
    }
    assert_eq!(restored.legal_value(), Some(0));
    let valid = world.save_bytes().unwrap();
    let mut old_version = valid.clone();
    old_version[8..10].copy_from_slice(&1_u16.to_le_bytes());
    reseal(&mut old_version);
    assert_eq!(
        world.restore_bytes(&old_version),
        Err(SaveError::UnsupportedVersion)
    );
    assert_eq!(world.save_bytes().unwrap(), valid);
    // Account extension follows the exact pre-existing owned-fit prefix.
    let account_extension = account_start(&world) + 61 + 8 * 29;
    let market_extension = markets_start(&world) + 16 + 2 * 17 + 4 * 8 + 4 * 8;
    for (offset, value) in [
        (account_extension + 7 * 8, 2_u64), // ordinary Machinery absent from purchases
        (account_extension + 12 * 8, 0),    // missing paid native kilogram
        (market_extension + 7 * 8, 54),     // unexplained stock reset after buying
        (market_extension + 2 * 16 * 8 + 7 * 8, 2), // forged Machinery buy count
    ] {
        let mut bad = valid.clone();
        bad[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        reseal(&mut bad);
        assert_refused_unchanged(&mut world, &bad);
        assert_eq!(world.save_bytes().unwrap(), valid);
    }
}

#[test]
fn paid_full_original_catalogue_transactions_continue_after_genuine_freight_income() {
    use station_services::{Command as Trade, CommodityId};
    let mut world = World::new_commander(1301).unwrap();
    let (job, _) = accept(&mut world);
    destination(&mut world);
    let actor = contact(&mut world);
    receive(&mut world, actor, job, Choice::ReceiveAll);
    assert_eq!(world.credits_tenths(), Some(14500));
    for id in CommodityId::ALL
        .into_iter()
        .filter(|id| *id != CommodityId::AlienItems)
    {
        world
            .request_station_service(Trade::BuyCommodity(id), world.snapshot().tick)
            .unwrap();
        let loaded = roundtrip(&world);
        assert_eq!(loaded.commodity_quantity(id), Some(1));
        assert_eq!(loaded.commodity_quote(id), world.commodity_quote(id));
        world
            .request_station_service(Trade::SellCommodity(id), world.snapshot().tick)
            .unwrap();
        let loaded = roundtrip(&world);
        assert_eq!(loaded.commodity_quantity(id), Some(0));
    }
    assert_eq!(
        world
            .commodity_quote(CommodityId::AlienItems)
            .unwrap()
            .stock,
        0
    );
}

#[test]
fn impossible_export_legal_bits_refuse_without_replacing_actual_cargo() {
    use station_services::{Command as Trade, CommodityId};
    let mut world = World::new_commander(1501).unwrap();
    world
        .request_station_service(Trade::BuyCommodity(CommodityId::Narcotics), 0)
        .unwrap();
    launch(&mut world);
    assert_eq!(world.legal_value(), Some(2));
    let valid = world.save_bytes().unwrap();
    let legal = account_start(&world) + 61 + 8 * 29 + 2 * 16 * 8;
    for value in [1, 3, 255] {
        // Only one Narcotics unit was ever bought: export bits can only include 2.
        let mut bad = valid.clone();
        bad[legal] = value;
        reseal(&mut bad);
        assert_refused_unchanged(&mut world, &bad);
        assert_eq!(world.save_bytes().unwrap(), valid);
    }
}

#[test]
fn public_tick_zero_launch_immediately_saves_and_continues_launching_without_minting_state() {
    use station_services::{Command as Trade, CommodityId};
    let mut world = World::new_commander(1601).unwrap();
    let initial = world.snapshot();
    let owner = world.commander_id();
    let hull = world.player_ship_id();
    let narcotics = world.commodity_quote(CommodityId::Narcotics).unwrap();
    let gems = world.commodity_quote(CommodityId::GemStones).unwrap();
    for id in [CommodityId::Narcotics, CommodityId::GemStones] {
        world
            .request_station_service(Trade::BuyCommodity(id), 0)
            .unwrap();
    }
    assert_eq!(world.snapshot().tick, 0);
    assert_eq!(world.legal_value(), Some(0));
    world.request_launch(0).unwrap(); // Deliberately no simulation step before save.
    let launched = world.snapshot();
    let expected = OriginalStationData::bundled()
        .unwrap()
        .launch_start()
        .unwrap();
    assert_eq!(launched.tick, 0);
    assert_eq!(launched.mode, FlightMode::Launching);
    assert_eq!(launched.position, expected.position);
    assert_eq!(launched.orientation, expected.orientation);
    assert_eq!(launched.speed, expected.speed);
    assert_ne!(launched.position, initial.position);
    assert_eq!(world.legal_value(), Some(2));
    let bytes = world.save_bytes().unwrap();
    let mut continued = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(continued.snapshot(), launched);
    assert_eq!(continued.commander_id(), owner);
    assert_eq!(continued.player_ship_id(), hull);
    assert_eq!(continued.current_dock_context(), None);
    assert_eq!(continued.legal_value(), Some(2));
    assert_eq!(continued.credits_tenths(), Some(645));
    assert_eq!(
        continued.commodity_quantity(CommodityId::Narcotics),
        Some(1)
    );
    assert_eq!(
        continued.commodity_quantity(CommodityId::GemStones),
        Some(1)
    );
    assert_eq!(continued.service_snapshot().unwrap().used_tonnes, 1);
    assert_eq!(continued.mission_snapshot().unwrap().freight.held_tonnes, 0);
    let before = continued.save_bytes().unwrap();
    assert_eq!(continued.request_launch(0), Err(StepError::InvalidMode));
    assert_eq!(
        continued.request_station_service(Trade::BuyCommodity(CommodityId::Food), 0),
        Err(MissionError::NotDocked)
    );
    assert_eq!(continued.save_bytes().unwrap(), before);
    continued.step(FlightInput::default()).unwrap();
    // Genuine public flight returns to the original finite station rows after Continue.
    for target in [SystemId::RENAS, SystemId::AVENOR] {
        if continued.snapshot().mode == FlightMode::Docked {
            launch(&mut continued);
        }
        continued
            .request_jump(target, continued.snapshot().tick)
            .unwrap();
        for _ in 0..BASIC_SPOOL_TICKS {
            continued.step(FlightInput::default()).unwrap();
        }
        approach(&mut continued);
    }
    assert_eq!(continued.snapshot().navigation.unwrap().fuel_units, 62);
    assert_eq!(continued.credits_tenths(), Some(645));
    assert_eq!(continued.legal_value(), Some(2));
    assert_eq!(
        continued.commodity_quote(CommodityId::Narcotics).unwrap(),
        station_services::CommodityQuote {
            stock: narcotics.stock - 1,
            ..narcotics
        }
    );
    assert_eq!(
        continued.commodity_quote(CommodityId::GemStones).unwrap(),
        station_services::CommodityQuote {
            stock: gems.stock - 1,
            ..gems
        }
    );
}

#[test]
fn forged_launch_admission_counts_and_wide_overflow_values_refuse_atomically() {
    let mut world = World::new_commander(1701).unwrap();
    world.request_launch(0).unwrap();
    let valid = world.save_bytes().unwrap();
    let mut flight = Writer::new();
    write_flight(&mut flight, &world.state);
    let nav = 18 + CONTENT.len() + flight.bytes.len();
    // Owner8 + ship16 + bound drive32 + system3 precede visit/dock sequence.
    let visit = nav + 8 + 16 + 32 + 3;
    let dock_sequence = visit + 8;
    // Navigation's final three fixed-width counters follow its two bounded options.
    let counts = account_start(&world) - 3 * 8;
    for (new_visit, new_dock, new_launches, new_jumps) in [
        (3, 1, 2, 0), // A second launch cannot consume the same initial admission.
        (3, 2, 2, 0), // Balanced admissions still require a physical-entry tick (tick remains zero).
        (2, 0, 1, 0), // No zero dock sequence/subtraction underflow.
        (1, 1, 1, 0), // Launch must advance visit.
        (u64::MAX, u64::MAX, u64::MAX, 0), // visit=1+launches cannot wrap.
        (u64::MAX, u64::MAX, u64::MAX, u64::MAX), // wide sums cannot wrap either.
    ] {
        let mut bad = valid.clone();
        for (offset, value) in [
            (visit, new_visit),
            (dock_sequence, new_dock),
            (counts, new_launches),
            (counts + 8, new_jumps),
        ] {
            bad[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }
        reseal(&mut bad);
        assert_refused_unchanged(&mut world, &bad);
        assert_eq!(world.save_bytes().unwrap(), valid);
    }
}

#[test]
fn another_real_public_branch_cannot_supply_a_practice_incident_this_world_never_emitted() {
    use crate::flight::missiles::tests::stop;
    use crate::flight::practice::tests::{aim_at, consent, launch, shoot, step};
    let mut world = World::new_commander(14701).unwrap();
    consent(&mut world);
    launch(&mut world);
    stop(&mut world);
    aim_at(&mut world, crate::docking::Vec3::new(70.0, 0.0, 136.32));
    assert!(matches!(
        shoot(&mut world).contact,
        crate::PulseContact::StationBlocked { .. }
    ));
    let opening = world.save_bytes().unwrap();
    let mut other_branch = World::from_save_bytes(&opening).unwrap();
    step(&mut world, 18);
    step(&mut other_branch, 18);
    assert!(matches!(
        shoot(&mut other_branch).contact,
        crate::PulseContact::StationBlocked { .. }
    ));
    let other_receipt = other_branch
        .combat_snapshot()
        .unwrap()
        .last_ordinary_impact
        .unwrap();
    assert_eq!(other_receipt.incident, 2);
    assert_eq!(
        world
            .combat_snapshot()
            .unwrap()
            .last_ordinary_impact
            .unwrap()
            .incident,
        1
    );
    // Both poses and ticks came from the same actual public history. Only the other
    // branch emitted this second incident. Its otherwise real receipt is not authority here.
    assert_eq!(world.snapshot(), other_branch.snapshot());
    let bytes = world.save_bytes().unwrap();
    let mut bad = World::from_save_bytes(&bytes).unwrap();
    bad.practice.as_mut().unwrap().ordinary_hit = Some(other_receipt);
    let poisoned = bad.unchecked_test_save().unwrap();
    let expected = SaveError::InvalidState("T02 ordinary hit has no accepted incident");
    assert_eq!(
        bad.validate_practice_incidents(bad.practice, &bad.practice_history),
        Err(expected)
    );
    assert_eq!(world.restore_bytes(&poisoned), Err(expected));
    assert_eq!(world.save_bytes().unwrap(), bytes);
}

#[test]
fn qualified_v15_opening_layout_keeps_real_birth_provenance_and_accepts_a_real_new_discharge() {
    // Opening codec fixture only. At actual tick zero there are no module events or traffic
    // shots, so the qualified15 body is byte-identical; only its frozen header differs.
    let world = World::new_commander(15001).unwrap();
    let current = world.save_bytes().unwrap();
    let body = 10 + CONTENT.len() + 8;
    let mut legacy = Writer::new();
    legacy.bytes.extend(MAGIC);
    legacy.u16(15);
    legacy.bytes.extend(V15_CONTENT);
    legacy.u64(compiled_content_fingerprint(V15_CONTENT));
    legacy.bytes.extend(&current[body..current.len() - 8]);
    legacy.u64(checksum(&legacy.bytes));
    let mut loaded = World::from_save_bytes(&legacy.bytes).unwrap();
    assert_eq!(loaded.snapshot(), world.snapshot());
    assert_eq!(
        loaded.modular_equipment_snapshot(),
        world.modular_equipment_snapshot()
    );
    assert_eq!(loaded.credits_tenths(), Some(7200));
    assert!(loaded.damage.enabled());
    crate::flight::practice::tests::launch(&mut loaded);
    loaded
        .request_fire_front_pulse(loaded.snapshot().tick)
        .unwrap();
    assert_eq!(
        loaded
            .flight_resources_snapshot()
            .unwrap()
            .energy_hundredths,
        37900
    );
    let bytes = loaded.save_bytes().unwrap();
    let cold = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(
        cold.flight_resources_snapshot(),
        loaded.flight_resources_snapshot()
    );
    assert_eq!(
        cold.modular_equipment_snapshot(),
        loaded.modular_equipment_snapshot()
    );
}
#[test]
fn qualified_v13_absence_does_not_mint_resource_birth_history_for_fast_or_energy_modules() {
    let world = World::new_commander(15002).unwrap();
    let bytes = v13_test_fixture(&world).unwrap();
    let loaded = World::from_save_bytes(&bytes).unwrap();
    assert!(!loaded.damage.enabled());
    let before = loaded.save_bytes().unwrap();
    for kind in [
        crate::ModuleKind::BeamLaser,
        crate::ModuleKind::ExtraEnergyUnit,
    ] {
        assert!(matches!(
            loaded.review_module(crate::ModuleAction::Purchase { kind }, 0),
            Err(crate::ModuleError::Save(_))
        ));
    }
    assert_eq!(loaded.save_bytes().unwrap(), before);
}

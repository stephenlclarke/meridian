#![allow(clippy::unwrap_used)]

use super::*;
use crate::flight::equipment::tests::{earned_world, journey};
use crate::navigation::SystemId;

fn buy(grade: CabinGrade) -> ModuleAction {
    ModuleAction::Purchase {
        kind: ModuleKind::Cabin(grade),
    }
}

fn act(world: &mut World, action: ModuleAction) -> u64 {
    let tick = world.snapshot().tick;
    let review = world.review_module(action, tick).unwrap();
    let prepared = world.prepare_module(review, tick).unwrap();
    let bytes = prepared.bytes().to_vec();
    let restored = World::from_save_bytes(&bytes).unwrap();
    let item = prepared.commit();
    assert_eq!(world.save_bytes().unwrap(), bytes);
    assert_eq!(world.service_snapshot(), restored.service_snapshot());
    assert_eq!(world.credits_tenths(), restored.credits_tenths());
    item
}

fn passage(world: &mut World, action: PassengerAction) {
    act(world, ModuleAction::Passenger(action));
}

fn review(world: &World, action: ModuleAction) -> Result<ModuleReview, ModuleError> {
    world.review_module(action, world.snapshot().tick)
}

fn funded_avenor(seed: u64) -> World {
    let mut world = earned_world(seed);
    world.request_refuel(world.snapshot().tick).unwrap();
    journey(&mut world, SystemId::AVENOR);
    world
}

fn journey_with_arrival(world: &mut World, target: SystemId) -> u64 {
    use crate::{FlightInput, FlightMode};
    world.request_launch(world.snapshot().tick).unwrap();
    world.step(FlightInput::default()).unwrap();
    world.request_jump(target, world.snapshot().tick).unwrap();
    let remaining = world
        .snapshot()
        .navigation
        .unwrap()
        .travel
        .unwrap()
        .remaining_ticks;
    for _ in 0..remaining {
        world.step(FlightInput::default()).unwrap();
    }
    let arrival = world.snapshot().tick;
    assert_eq!(world.current_dock_context(), None);
    for _ in 0..9000 {
        let snapshot = world.snapshot();
        if snapshot.mode == FlightMode::Docked {
            return arrival;
        }
        let speed = if snapshot.position[2] > 2500.0 {
            34.3
        } else {
            4.0
        };
        world
            .step(FlightInput {
                throttle: ((speed - snapshot.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..FlightInput::default()
            })
            .unwrap();
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    panic!("actual public flight did not dock");
}

fn reject_changed_history(world: &World, live: &mut World, retained: &[u8]) {
    assert!(world.save_bytes().is_err());
    assert!(
        live.restore_bytes(&world.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), retained);
}

#[test]
fn original_six_offers_empty_start_and_no_grant() {
    let mut world = World::new_commander(31001).unwrap();
    for (grade, places, price, awake) in [
        (CabinGrade::PackedStasis, 20, 10_000, false),
        (CabinGrade::Economy, 8, 25_000, true),
        (CabinGrade::Standard, 4, 50_000, true),
        (CabinGrade::Luxury, 2, 100_000, true),
        (CabinGrade::SingleStateroom, 1, 200_000, true),
        (CabinGrade::DoubleStateroom, 2, 200_000, true),
    ] {
        let kind = ModuleKind::Cabin(grade);
        let offer = world
            .module_offers()
            .into_iter()
            .find(|offer| offer.kind == kind)
            .unwrap();
        assert_eq!(
            (
                grade.places(),
                offer.price_tenths,
                offer.equipment_units,
                grade.supports_awake()
            ),
            (places, price, 1, awake)
        );
        assert_eq!(ModuleKind::from_tag(kind.tag()), Ok(kind));
        assert!(!kind.xyzzy_supported());
        assert_eq!(
            review(&world, buy(grade)),
            Err(ModuleError::InsufficientCredits)
        );
    }
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 0);
    let fit = world.modular_equipment_snapshot().unwrap().fit_revision;
    passage(&mut world, PassengerAction::RequestReplies);
    let journey = world.passenger_journey_snapshot().unwrap();
    assert_eq!(journey.status, PassageStatus::Refused);
    assert_eq!(journey.cabins, None);
    assert_eq!(
        review(&world, ModuleAction::Passenger(PassengerAction::Board)),
        Err(ModuleError::PassageConsent)
    );
    assert_eq!(world.credits_tenths(), Some(7200));
    assert_eq!(
        world.modular_equipment_snapshot().unwrap().fit_revision,
        fit
    );
}

#[test]
fn paid_repeatable_cabins_reserve_hold_and_units_and_failed_write_is_atomic() {
    let mut world = funded_avenor(31002);
    let cash = world.credits_tenths().unwrap();
    let tick = world.snapshot().tick;
    let before = world.save_bytes().unwrap();
    let r = world.review_module(buy(CabinGrade::Economy), tick).unwrap();
    assert_eq!(
        world
            .prepare_module(r, tick)
            .unwrap()
            .publish_with(|_| Err::<(), _>("disk")),
        Err("disk")
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    let first = act(&mut world, buy(CabinGrade::Economy));
    let second = act(&mut world, buy(CabinGrade::Economy));
    assert_ne!(first, second);
    assert_eq!(world.credits_tenths(), Some(cash - 50_000));
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 10);
    assert_eq!(
        world
            .modular_equipment_snapshot()
            .unwrap()
            .equipment_units_used,
        2
    );
    assert!(
        world
            .passenger_cabins()
            .unwrap()
            .iter()
            .all(|cabin| cabin.life_support_working
                && cabin.free_awake_places == 8
                && cabin.free_stasis_places == 8
                && cabin.passenger.is_none())
    );
    assert_eq!(
        world.passenger_journey_snapshot().unwrap().status,
        PassageStatus::Waiting
    );
    act(&mut world, ModuleAction::Store { item: first });
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 5);
    assert_eq!(
        review(
            &world,
            ModuleAction::Install {
                item: first,
                replacing: Some(second)
            }
        ),
        Err(ModuleError::StaleOccupant)
    );
    act(
        &mut world,
        ModuleAction::Install {
            item: first,
            replacing: None,
        },
    );
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 10);
    assert_eq!(world.credits_tenths(), Some(cash - 50_000));
    use crate::equipment::{Command as Bay, Event as BayEvent};
    use crate::freight::g02::{Choice, Command as Freight, Event as FreightEvent};
    let BayEvent::Purchased { item: bay, .. } = world
        .request_equipment(Bay::PurchaseCargoBay, world.snapshot().tick)
        .unwrap()
    else {
        panic!("paid cargo bay");
    };
    world
        .request_equipment(Bay::UnfitCargoBay(bay), world.snapshot().tick)
        .unwrap();
    world
        .request_equipment(Bay::FitCargoBay(bay), world.snapshot().tick)
        .unwrap();
    let actor = world.station_contacts()[0];
    world.request_contact(actor, world.snapshot().tick).unwrap();
    let FreightEvent::Reviewed(offer) = world
        .request_freight(
            Freight::ReviewOffer {
                actor: actor.actor(),
            },
            world.snapshot().tick,
        )
        .unwrap()
    else {
        panic!("freight offer");
    };
    world
        .request_freight(
            Freight::Confirm {
                review: offer,
                choice: Choice::Accept,
            },
            world.snapshot().tick,
        )
        .unwrap();
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 18);
    let bytes = world.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&bytes).unwrap();
    let retained = live.save_bytes().unwrap();
    world.modular.history.remove_last_bay_for_test();
    reject_changed_history(&world, &mut live, &retained);
    world = World::from_save_bytes(&bytes).unwrap();
    world.modular.history.remove_last_freight_for_test();
    reject_changed_history(&world, &mut live, &retained);
}

#[test]
fn original_two_private_cabins_board_travel_receive_once_and_continue() {
    let mut world = funded_avenor(31003);
    let packed = act(&mut world, buy(CabinGrade::PackedStasis));
    let first = act(&mut world, buy(CabinGrade::Economy));
    passage(&mut world, PassengerAction::RequestReplies);
    assert_eq!(
        world.passenger_journey_snapshot().unwrap().status,
        PassageStatus::Refused
    );
    let second = act(&mut world, buy(CabinGrade::Economy));
    assert_eq!(
        review(&world, ModuleAction::Passenger(PassengerAction::Board)),
        Err(ModuleError::PassageConsent)
    );
    passage(&mut world, PassengerAction::RequestReplies);
    let offer = world.passenger_journey_snapshot().unwrap();
    assert_eq!(
        (offer.status, offer.cabins, offer.reply_is_current),
        (PassageStatus::Accepted, Some([first, second]), true)
    );
    world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert!(!world.passenger_journey_snapshot().unwrap().reply_is_current);
    assert_eq!(
        review(&world, ModuleAction::Passenger(PassengerAction::Board)),
        Err(ModuleError::PassageConsent)
    );
    passage(&mut world, PassengerAction::RequestReplies);
    passage(&mut world, PassengerAction::Board);
    let aboard = world.passenger_journey_snapshot().unwrap();
    assert_eq!(aboard.status, PassageStatus::Aboard);
    assert!(
        world
            .unwritten_route_clue_progress()
            .unwrap()
            .earned
            .is_empty()
    );
    assert!(
        world
            .review_xyzzy(
                crate::XyzzyAction::MaximumFuelStorage,
                world.snapshot().tick
            )
            .is_err()
    );
    assert_eq!(aboard.cabins, Some([first, second]));
    assert_eq!(aboard.people.map(|p| p.person), ["mira-talen", "oren-vey"]);
    for cabin in world.passenger_cabins().unwrap() {
        if [first, second].contains(&cabin.item) {
            assert_eq!((cabin.free_awake_places, cabin.free_stasis_places), (0, 0));
            assert!(cabin.passenger.is_some());
        } else if cabin.item == packed {
            assert_eq!((cabin.free_awake_places, cabin.free_stasis_places), (0, 20));
        }
    }
    assert_eq!(
        review(&world, ModuleAction::Store { item: first }),
        Err(ModuleError::CabinOccupied)
    );
    assert_eq!(
        review(&world, ModuleAction::Store { item: second }),
        Err(ModuleError::CabinOccupied)
    );
    assert_eq!(
        review(&world, ModuleAction::Passenger(PassengerAction::Receive)),
        Err(ModuleError::BerthRequired)
    );
    act(&mut world, ModuleAction::Store { item: packed });
    let arrival = journey_with_arrival(&mut world, SystemId::RENAS);
    world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let before_receipt = world.passenger_journey_snapshot().unwrap();
    assert_eq!(before_receipt.people, aboard.people);
    assert_eq!(before_receipt.cabins, aboard.cabins);
    assert_eq!(before_receipt.status, PassageStatus::Aboard);
    let cash = world.credits_tenths().unwrap();
    let tick = world.snapshot().tick;
    let r = world
        .review_module(ModuleAction::Passenger(PassengerAction::Receive), tick)
        .unwrap();
    assert_eq!(
        world
            .prepare_module(r, tick)
            .unwrap()
            .publish_with(|_| Err::<(), _>("disk")),
        Err("disk")
    );
    assert_eq!(world.passenger_journey_snapshot().unwrap(), before_receipt);
    assert_eq!(world.credits_tenths(), Some(cash));
    passage(&mut world, PassengerAction::Receive);
    assert_eq!(world.credits_tenths(), Some(cash + 6100));
    world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let done = world.passenger_journey_snapshot().unwrap();
    assert_eq!(
        (
            done.people,
            done.cabins,
            done.status,
            done.clue.and_then(|receipt| receipt.clue_id())
        ),
        (
            aboard.people,
            aboard.cabins,
            PassageStatus::Delivered,
            Some(PASSAGE_CLUE_ID)
        )
    );
    let terminal = done.terminal.unwrap();
    assert_eq!(terminal.owner, aboard.people[0].owner);
    assert_eq!(terminal.cabins, [first, second]);
    assert_eq!(terminal.receiving.system(), SystemId::RENAS);
    assert_eq!(terminal.boarding.system(), SystemId::AVENOR);
    assert!(terminal.terminal_revision > terminal.boarding_revision);
    let journal = world.unwritten_route_clue_progress().unwrap();
    assert_eq!(journal.earned.len(), 1);
    assert_eq!(journal.earned[0].source_id, "l337.contract/cabin-journey");
    assert_eq!(journal.earned[0].clue_id, "cabin-journey-family-trace");
    assert_eq!(
        World::from_save_bytes(&world.save_bytes().unwrap())
            .unwrap()
            .unwritten_route_clue_progress(),
        Some(journal)
    );
    assert_eq!(
        review(&world, ModuleAction::Passenger(PassengerAction::Receive)),
        Err(ModuleError::PassageClosed)
    );
    act(&mut world, ModuleAction::Store { item: first });
    assert_eq!(world.credits_tenths(), Some(cash + 6100));
    journey(&mut world, SystemId::AVENOR);
    let valid = world.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let retained = live.save_bytes().unwrap();
    world.modular.history.remove_last_berth_for_test();
    reject_changed_history(&world, &mut live, &retained);
    world = World::from_save_bytes(&valid).unwrap();
    // Same historical Renas arrival visit, but before its accepted physical dock. Neither
    // a plausible visit/system prefix nor the eventual wallet balance authorizes reception.
    let receipt = world
        .modular
        .events
        .iter_mut()
        .find(|event| event.action == ModuleAction::Passenger(PassengerAction::Receive))
        .unwrap();
    let mut encoded = Writer::new();
    receipt.dock.encode_save(&mut encoded);
    let tick_offset = encoded.bytes.len() - 9;
    encoded.bytes[tick_offset..tick_offset + 8].copy_from_slice(&arrival.to_le_bytes());
    receipt.dock = DockContext::decode_save(&mut Reader::new(&encoded.bytes)).unwrap();
    reject_changed_history(&world, &mut live, &retained);
}

#[test]
fn safe_cancellation_releases_both_and_closes_finite_source() {
    let mut world = funded_avenor(31004);
    let first = act(&mut world, buy(CabinGrade::Economy));
    let second = act(&mut world, buy(CabinGrade::Economy));
    passage(&mut world, PassengerAction::RequestReplies);
    passage(&mut world, PassengerAction::Board);
    let cash = world.credits_tenths();
    passage(&mut world, PassengerAction::Cancel);
    world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let state = world.passenger_journey_snapshot().unwrap();
    assert_eq!(state.status, PassageStatus::Cancelled);
    assert_eq!(state.clue, None);
    assert!(
        world
            .unwritten_route_clue_progress()
            .unwrap()
            .earned
            .is_empty()
    );
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(
        review(
            &world,
            ModuleAction::Passenger(PassengerAction::RequestReplies)
        ),
        Err(ModuleError::PassageClosed)
    );
    act(&mut world, ModuleAction::Store { item: first });
    act(&mut world, ModuleAction::Store { item: second });
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 0);
}

#[test]
fn cabin_mass_is_shared_with_ordinary_market_and_refit() {
    use crate::station_services::{Command as Trade, CommodityId, Error as TradeError};
    let mut world = earned_world(31005);
    let cabin = act(&mut world, buy(CabinGrade::PackedStasis));
    let trade = Trade::BuyCommodity(CommodityId::Textiles);
    for _ in 0..87 {
        world
            .request_station_service(trade, world.snapshot().tick)
            .unwrap();
    }
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 92);
    assert_eq!(
        world.request_station_service(trade, world.snapshot().tick),
        Err(crate::MissionError::Service(TradeError::HoldFull))
    );
    assert_eq!(
        review(&world, buy(CabinGrade::PackedStasis)),
        Err(ModuleError::Capacity)
    );
    act(&mut world, ModuleAction::Store { item: cabin });
    world
        .request_station_service(trade, world.snapshot().tick)
        .unwrap();
    assert_eq!(
        review(
            &world,
            ModuleAction::Install {
                item: cabin,
                replacing: None
            }
        ),
        Err(ModuleError::Capacity)
    );
    world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 88);
    let valid = world.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let retained = live.save_bytes().unwrap();
    let market_revision = world.commander_account.as_ref().unwrap().revision();
    let store = world
        .modular
        .events
        .iter_mut()
        .find(|event| event.action == ModuleAction::Store { item: cabin })
        .unwrap();
    assert!(
        world
            .station_markets
            .as_mut()
            .unwrap()
            .move_cargo_revision_for_test(market_revision, store.revision)
    );
    store.revision = market_revision;
    // The same paid cabin is stored and the final 88 tonnes still fit, but the reordered
    // market purchase would transiently occupy 93t. The actual prefix must refuse it.
    reject_changed_history(&world, &mut live, &retained);
}

#[test]
fn forged_passage_receipt_cannot_change_historical_fuel_or_create_boarding() {
    let mut world = World::new_commander(31006).unwrap();
    passage(&mut world, PassengerAction::RequestReplies);
    let bytes = world.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&bytes).unwrap();
    let retained = live.save_bytes().unwrap();
    world.modular.events[0].item -= 1;
    assert!(world.save_bytes().is_err());
    assert!(
        live.restore_bytes(&world.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), retained);
    world = World::from_save_bytes(&bytes).unwrap();
    world.modular.events[0].action = ModuleAction::Passenger(PassengerAction::Board);
    assert!(world.save_bytes().is_err());
    assert!(
        live.restore_bytes(&world.unchecked_test_save().unwrap())
            .is_err()
    );
    assert_eq!(live.save_bytes().unwrap(), retained);
}

#[test]
fn extended_receipt_requires_marker_and_exact_bounded_sixteen_bit_fuel() {
    let packed = cabins::pack_receipt(17, 410).unwrap();
    assert_eq!(cabins::receipt_epoch(packed), 17);
    assert_eq!(cabins::receipt_fuel(packed), Some(410));
    assert_eq!(cabins::receipt_fuel((17 << 16) | 411), None);
    assert_eq!(cabins::receipt_fuel(410), None);
    assert_eq!(
        cabins::pack_receipt(u64::MAX, 410),
        Err(ModuleError::SequenceExhausted)
    );
    let mut world = World::new_commander(31007).unwrap();
    passage(&mut world, PassengerAction::RequestReplies);
    let mut writer = Writer::new();
    world.modular.encode(&mut writer);
    let complete = writer.bytes.clone();
    let mut reader = Reader::new(&complete);
    assert_eq!(
        State::decode(&mut reader, crate::save::SAVE_VERSION).unwrap(),
        world.modular
    );
    assert!(reader.finished());
    writer.bytes[3] &= 0xbf;
    assert!(State::decode(&mut Reader::new(&writer.bytes), crate::save::SAVE_VERSION).is_err());
    assert!(
        State::decode(
            &mut Reader::new(&complete[..complete.len() - 1]),
            crate::save::SAVE_VERSION
        )
        .is_err()
    );
}

#[test]
fn legacy_unrecorded_bay_refit_is_not_fabricated_into_an_anchor() {
    use crate::equipment::Command as Bay;
    let mut world = World::from_save_bytes(include_bytes!(
        "../../../tests/fixtures/xyzzy-paid-base-v17.bin"
    ))
    .unwrap();
    let mut equipment = world.equipment_snapshot().unwrap();
    if equipment.owned.is_none() {
        world
            .request_equipment(Bay::PurchaseCargoBay, world.snapshot().tick)
            .unwrap();
        equipment = world.equipment_snapshot().unwrap();
    }
    let item = equipment.owned.unwrap();
    if item.fitted {
        world
            .request_equipment(Bay::UnfitCargoBay(item.id), world.snapshot().tick)
            .unwrap();
    }
    let before = world.save_bytes().unwrap();
    assert_eq!(
        review(&world, buy(CabinGrade::PackedStasis)),
        Err(ModuleError::PassageHistory)
    );
    assert_eq!(world.save_bytes().unwrap(), before);
}

#[test]
fn cabin_capacity_respects_t04_reservation_purchase_and_cancel_at_every_prefix() {
    use crate::station_services::{Command as Trade, CommodityId, Error as TradeError};
    use crate::{MissionError, NavigationTradeAction as TradeAction};
    fn confirm(world: &mut World, action: TradeAction) {
        let tick = world.snapshot().tick;
        let review = world.review_navigation_trade(action, tick).unwrap();
        world
            .prepare_navigation_trade(review, tick)
            .unwrap()
            .commit();
    }
    let mut world = earned_world(31008);
    world.request_refuel(world.snapshot().tick).unwrap();
    let cabin = act(&mut world, buy(CabinGrade::PackedStasis));
    let ordinary = Trade::BuyCommodity(CommodityId::Textiles);
    for _ in 0..86 {
        world
            .request_station_service(ordinary, world.snapshot().tick)
            .unwrap();
    }
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 91);
    confirm(&mut world, TradeAction::Accept);
    confirm(
        &mut world,
        TradeAction::Plan {
            destination: SystemId::AVENOR,
            commodity: CommodityId::Food,
        },
    );
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 92);
    assert_eq!(
        world
            .navigation_trade_snapshot()
            .unwrap()
            .reserved_capacity_tonnes,
        1
    );
    assert_eq!(
        world.request_station_service(ordinary, world.snapshot().tick),
        Err(MissionError::Service(TradeError::HoldFull))
    );
    let planned = world.save_bytes().unwrap();
    let mut bought = World::from_save_bytes(&planned).unwrap();
    bought
        .request_station_service(Trade::BuyFood, bought.snapshot().tick)
        .unwrap();
    bought = World::from_save_bytes(&bought.save_bytes().unwrap()).unwrap();
    assert_eq!(
        bought
            .navigation_trade_snapshot()
            .unwrap()
            .reserved_capacity_tonnes,
        0
    );
    assert_eq!(bought.commodity_quantity(CommodityId::Food), Some(1));
    assert_eq!(bought.service_snapshot().unwrap().used_tonnes, 92);

    world = World::from_save_bytes(&planned).unwrap();
    act(&mut world, ModuleAction::Store { item: cabin });
    world
        .request_station_service(ordinary, world.snapshot().tick)
        .unwrap();
    confirm(&mut world, TradeAction::Cancel);
    world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(
        world
            .navigation_trade_snapshot()
            .unwrap()
            .reserved_capacity_tonnes,
        0
    );
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 87);
    let valid = world.save_bytes().unwrap();
    let mut live = World::from_save_bytes(&valid).unwrap();
    let retained = live.save_bytes().unwrap();
    assert!(
        world
            .navigation_trade
            .omit_last_reservation_release_for_test()
    );
    reject_changed_history(&world, &mut live, &retained);
    world = World::from_save_bytes(&valid).unwrap();
    let market_revision = world.commander_account.as_ref().unwrap().revision();
    let store = world
        .modular
        .events
        .iter_mut()
        .find(|event| event.action == ModuleAction::Store { item: cabin })
        .unwrap();
    assert!(
        world
            .station_markets
            .as_mut()
            .unwrap()
            .move_cargo_revision_for_test(market_revision, store.revision)
    );
    store.revision = market_revision;
    // Actual mass would have been 92t, not 93t, but the still-reserved T04 tonne
    // makes the reordered historical account impossible. Final 87t still fits.
    reject_changed_history(&world, &mut live, &retained);
}

#[test]
fn actual_four_tank_commander_records_410_fuel_refusal_and_cold_resume_without_passengers() {
    use crate::{XyzzyAction, navigation::MAX_FUEL_CAPACITY};
    let mut world = World::new_commander(31009).unwrap();
    let review = world
        .review_xyzzy(XyzzyAction::MaximumFuelStorage, 0)
        .unwrap();
    world.prepare_xyzzy(review, 0).unwrap().commit();
    let tanks = world
        .modular_equipment_snapshot()
        .unwrap()
        .items
        .into_iter()
        .filter(|item| matches!(item.slot, Some(ModuleSlot::FuelStorage(_))))
        .collect::<Vec<_>>();
    assert_eq!(tanks.len(), 4);
    assert_eq!(
        tanks
            .iter()
            .map(|item| item.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    let opening = world.snapshot().navigation.unwrap();
    assert_eq!(
        (opening.fuel_units, opening.fuel_capacity_units),
        (410, MAX_FUEL_CAPACITY)
    );
    assert!(world.test_tank_overfit());
    assert!(world.passenger_cabins().unwrap().is_empty());
    passage(&mut world, PassengerAction::RequestReplies);
    let request = world
        .modular
        .events
        .iter()
        .find(|event| event.action == ModuleAction::Passenger(PassengerAction::RequestReplies))
        .unwrap();
    assert_eq!(cabins::receipt_fuel(request.item), Some(410));
    assert_eq!(
        world.passenger_journey_snapshot().unwrap().status,
        PassageStatus::Refused
    );
    let bytes = world.save_bytes().unwrap();
    let cold = World::from_save_bytes(&bytes).unwrap();
    assert_eq!(cold.snapshot().navigation.unwrap(), opening);
    assert_eq!(
        cold.modular_equipment_snapshot()
            .unwrap()
            .items
            .into_iter()
            .filter(|item| matches!(item.slot, Some(ModuleSlot::FuelStorage(_))))
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        tanks.iter().map(|item| item.id).collect::<Vec<_>>()
    );
    assert_eq!(
        cold.passenger_journey_snapshot().unwrap().status,
        PassageStatus::Refused
    );
    assert!(!cold.passenger_journey_snapshot().unwrap().reply_is_current);
    assert!(cold.passenger_cabins().unwrap().is_empty());
    assert!(
        cold.unwritten_route_clue_progress()
            .unwrap()
            .earned
            .is_empty()
    );
    assert!(review(&cold, ModuleAction::Passenger(PassengerAction::Board)).is_err());
}

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
    journey(&mut world, SystemId::RENAS);
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
        (done.people, done.cabins, done.status, done.clue),
        (
            aboard.people,
            aboard.cabins,
            PassageStatus::Delivered,
            Some("cabin-journey-family-trace")
        )
    );
    assert_eq!(
        review(&world, ModuleAction::Passenger(PassengerAction::Receive)),
        Err(ModuleError::PassageClosed)
    );
    act(&mut world, ModuleAction::Store { item: first });
    assert_eq!(world.credits_tenths(), Some(cash + 6100));
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

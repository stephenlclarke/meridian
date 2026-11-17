#![allow(clippy::unwrap_used)]
use super::*;
use crate::equipment::{Error as EquipmentError, ItemId};
use crate::freight::g02::{self, Choice};
use crate::navigation::{BASIC_SPOOL_TICKS, SystemId};
use crate::station_services::{Command as Trade, CommodityId, Error as TradeError};
use crate::{FlightInput, FlightMode, StepError};

fn trade(world: &mut World, command: Trade) {
    world
        .request_station_service(command, world.snapshot().tick)
        .unwrap();
}
fn equipment(world: &mut World, command: Command) -> Result<Event, MissionError> {
    world.request_equipment(command, world.snapshot().tick)
}
pub(in crate::flight) fn journey(world: &mut World, target: SystemId) {
    world.request_launch(world.snapshot().tick).unwrap();
    world.step(FlightInput::default()).unwrap();
    world.request_jump(target, world.snapshot().tick).unwrap();
    for _ in 0..BASIC_SPOOL_TICKS {
        world.step(FlightInput::default()).unwrap();
    }
    for _ in 0..9000 {
        let snapshot = world.snapshot();
        if snapshot.mode == FlightMode::Docked {
            return;
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
fn buy(world: &mut World, id: CommodityId, quantity: usize) {
    for _ in 0..quantity {
        trade(world, Trade::BuyCommodity(id));
    }
}
fn sell(world: &mut World, id: CommodityId, quantity: usize) {
    for _ in 0..quantity {
        trade(world, Trade::SellCommodity(id));
    }
}
fn continued(world: &World) -> World {
    let loaded = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(loaded.snapshot(), world.snapshot());
    assert_eq!(loaded.credits_tenths(), world.credits_tenths());
    assert_eq!(loaded.equipment_snapshot(), world.equipment_snapshot());
    assert_eq!(loaded.service_snapshot(), world.service_snapshot());
    for id in CommodityId::ALL {
        assert_eq!(loaded.commodity_quantity(id), world.commodity_quantity(id));
        assert_eq!(loaded.commodity_quote(id), world.commodity_quote(id));
    }
    loaded
}
fn refuses(world: &mut World, command: Command, error: EquipmentError) {
    let before = world.save_bytes().unwrap();
    assert_eq!(
        equipment(world, command),
        Err(MissionError::Equipment(error))
    );
    assert_eq!(world.save_bytes().unwrap(), before);
}
// Funds come entirely from genuine original finite markets and charged public travel.
// The independent expected cash sequence is deliberately not computed from the implementation.
pub(in crate::flight) fn earned_world(seed: u64) -> World {
    earned_from(World::new_commander(seed).unwrap())
}
pub(in crate::flight) fn earned_from(mut world: World) -> World {
    for (quantity, expected) in [(29, 11434), (47, 18296), (75, 29042)] {
        if world.snapshot().navigation.unwrap().system != SystemId::AVENOR {
            if world.snapshot().navigation.unwrap().fuel_units < 68 {
                world.request_refuel(world.snapshot().tick).unwrap();
            }
            journey(&mut world, SystemId::AVENOR);
        }
        buy(&mut world, CommodityId::Minerals, quantity);
        journey(&mut world, SystemId::RENAS);
        sell(&mut world, CommodityId::Minerals, quantity);
        assert_eq!(world.credits_tenths(), Some(expected));
    }
    buy(&mut world, CommodityId::Alloys, 48);
    journey(&mut world, SystemId::AVENOR);
    sell(&mut world, CommodityId::Alloys, 48);
    assert_eq!(world.credits_tenths(), Some(33650));
    assert_eq!(world.request_refuel(world.snapshot().tick), Ok(204));
    buy(&mut world, CommodityId::GemStones, 37);
    journey(&mut world, SystemId::RENAS);
    sell(&mut world, CommodityId::GemStones, 37);
    assert_eq!(world.credits_tenths(), Some(49541));
    buy(&mut world, CommodityId::Alloys, 83);
    journey(&mut world, SystemId::AVENOR);
    sell(&mut world, CommodityId::Alloys, 83);
    assert_eq!(world.credits_tenths(), Some(57509));
    buy(&mut world, CommodityId::GemStones, 64);
    journey(&mut world, SystemId::RENAS);
    sell(&mut world, CommodityId::GemStones, 64);
    assert_eq!(world.credits_tenths(), Some(85349));
    assert_eq!(world.equipment_snapshot().unwrap().owned, None);
    world
}
fn purchase(world: &mut World) -> ItemId {
    match equipment(world, Command::PurchaseCargoBay).unwrap() {
        Event::Purchased { item, paid_tenths } => {
            assert_eq!(paid_tenths, 28000);
            item
        }
        event => panic!("expected actual paid purchase, got {event:?}"),
    }
}
#[test]
fn public_paid_identity_fit_capacity_native_units_and_continue_stay_one_transaction() {
    let mut world = earned_world(2101);
    let before = world.equipment_snapshot().unwrap();
    assert_eq!(
        (
            before.hold_capacity_tonnes,
            before.equipment_units_used,
            before.utility_mounts_used
        ),
        (92, 0, 0)
    );
    let item = purchase(&mut world);
    assert_eq!(world.credits_tenths(), Some(57349));
    let fitted = world.equipment_snapshot().unwrap();
    assert_eq!(
        (
            fitted.hold_capacity_tonnes,
            fitted.equipment_units_used,
            fitted.utility_mounts_used
        ),
        (127, 1, 1)
    );
    assert_eq!(fitted.owned.unwrap().id, item);
    assert_eq!(
        fitted.owned.unwrap().bound_hull,
        world.player_ship_id().unwrap().ordinal()
    );
    refuses(
        &mut world,
        Command::PurchaseCargoBay,
        EquipmentError::AlreadyOwned,
    );
    refuses(
        &mut world,
        Command::FitCargoBay(item),
        EquipmentError::AlreadyFitted,
    );
    assert_paid_save_guards(&mut world);
    world = continued(&world);
    buy(&mut world, CommodityId::Food, 112);
    buy(&mut world, CommodityId::Minerals, 15);
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 127);
    let before = world.save_bytes().unwrap();
    assert_eq!(
        world.request_station_service(
            Trade::BuyCommodity(CommodityId::Minerals),
            world.snapshot().tick
        ),
        Err(MissionError::Service(TradeError::HoldFull))
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    buy(&mut world, CommodityId::Gold, 1); // Native kg retains zero tonne cost even at expanded capacity.
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 127);
    refuses(
        &mut world,
        Command::UnfitCargoBay(item),
        EquipmentError::HoldFull,
    );
    world = continued(&world);
    sell(&mut world, CommodityId::Minerals, 15);
    sell(&mut world, CommodityId::Food, 20);
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 92);
    let balance = world.credits_tenths();
    assert_eq!(
        equipment(&mut world, Command::UnfitCargoBay(item)),
        Ok(Event::Stored { item })
    );
    assert_eq!(world.credits_tenths(), balance);
    assert_eq!(world.equipment_snapshot().unwrap().hold_capacity_tonnes, 92);
    assert_eq!(
        (
            world.equipment_snapshot().unwrap().equipment_units_used,
            world.equipment_snapshot().unwrap().utility_mounts_used
        ),
        (0, 0)
    );
    assert_eq!(world.equipment_snapshot().unwrap().owned.unwrap().id, item);
    refuses(
        &mut world,
        Command::PurchaseCargoBay,
        EquipmentError::AlreadyOwned,
    );
    refuses(
        &mut world,
        Command::UnfitCargoBay(item),
        EquipmentError::AlreadyStored,
    );
    world = continued(&world);
    assert_eq!(
        equipment(&mut world, Command::FitCargoBay(item)),
        Ok(Event::Fitted { item })
    );
    assert_eq!(world.credits_tenths(), balance);
    assert_eq!(world.equipment_snapshot().unwrap().owned.unwrap().id, item);
    continued(&world);
}
#[test]
fn actual_mission_custody_prevents_unfit_capacity_bypass_and_wrong_owner_item_never_fits() {
    let mut world = earned_world(2201);
    let item = purchase(&mut world);
    world.request_refuel(world.snapshot().tick).unwrap();
    journey(&mut world, SystemId::AVENOR);
    buy(&mut world, CommodityId::Food, 85);
    let actor = world.station_contacts()[0];
    world.request_contact(actor, world.snapshot().tick).unwrap();
    let reviewed = world
        .request_freight(
            g02::Command::ReviewOffer {
                actor: actor.actor(),
            },
            world.snapshot().tick,
        )
        .unwrap();
    let g02::Event::Reviewed(review) = reviewed else {
        panic!("expected actual offer");
    };
    equipment(&mut world, Command::UnfitCargoBay(item)).unwrap();
    equipment(&mut world, Command::FitCargoBay(item)).unwrap();
    let before = world.save_bytes().unwrap();
    assert_eq!(
        world.request_freight(
            g02::Command::Confirm {
                review,
                choice: Choice::Accept
            },
            world.snapshot().tick
        ),
        Err(MissionError::Freight(g02::Error::StaleReview))
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    let renewed = world
        .request_freight(
            g02::Command::ReviewOffer {
                actor: actor.actor(),
            },
            world.snapshot().tick,
        )
        .unwrap();
    let g02::Event::Reviewed(review) = renewed else {
        panic!("expected refreshed actual offer");
    };
    world
        .request_freight(
            g02::Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            world.snapshot().tick,
        )
        .unwrap();
    assert_eq!(world.mission_snapshot().unwrap().freight.held_tonnes, 8);
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 93);
    refuses(
        &mut world,
        Command::UnfitCargoBay(item),
        EquipmentError::HoldFull,
    );
    sell(&mut world, CommodityId::Food, 1);
    equipment(&mut world, Command::UnfitCargoBay(item)).unwrap();
    assert_eq!(world.service_snapshot().unwrap().used_tonnes, 92);
    assert_eq!(world.mission_snapshot().unwrap().freight.held_tonnes, 8);
    let mut other = World::new_commander(2301).unwrap();
    refuses(
        &mut other,
        Command::FitCargoBay(item),
        EquipmentError::NotOwned,
    );
    let mut forged_display = other.equipment_snapshot().unwrap();
    forged_display.owned = world.equipment_snapshot().unwrap().owned;
    forged_display.hold_capacity_tonnes = 127;
    assert_eq!(other.equipment_snapshot().unwrap().owned, None);
    assert_eq!(forged_display.hold_capacity_tonnes, 127);
    continued(&world);
}
#[test]
fn purchase_uses_current_public_berth_tick_pause_cash_and_preserves_failed_bytes() {
    let mut world = World::new_commander(2401).unwrap();
    refuses(
        &mut world,
        Command::PurchaseCargoBay,
        EquipmentError::InsufficientFunds,
    );
    world.step(FlightInput::default()).unwrap();
    let before = world.save_bytes().unwrap();
    assert_eq!(
        world.request_equipment(Command::PurchaseCargoBay, 0),
        Err(MissionError::StaleCommand)
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    world.request_pause(true, world.snapshot().tick).unwrap();
    let before = world.save_bytes().unwrap();
    assert_eq!(
        equipment(&mut world, Command::PurchaseCargoBay),
        Err(MissionError::Paused)
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    world.request_pause(false, world.snapshot().tick).unwrap();
    world.request_launch(world.snapshot().tick).unwrap();
    let before = world.save_bytes().unwrap();
    assert_eq!(
        equipment(&mut world, Command::PurchaseCargoBay),
        Err(MissionError::NotDocked)
    );
    assert_eq!(world.save_bytes().unwrap(), before);
    let mut prototype = World::new();
    assert_eq!(prototype.equipment_snapshot(), None);
    assert_eq!(
        equipment(&mut prototype, Command::PurchaseCargoBay),
        Err(MissionError::NoCommander)
    );
    assert_eq!(
        world.request_launch(world.snapshot().tick),
        Err(StepError::InvalidMode)
    );
    let mut unsupported = World::new_commander(2501).unwrap();
    journey(&mut unsupported, SystemId::RENAS);
    journey(
        &mut unsupported,
        SystemId {
            galaxy: 0,
            system: 0,
        },
    );
    let before = unsupported.save_bytes().unwrap();
    assert_eq!(
        equipment(&mut unsupported, Command::PurchaseCargoBay),
        Err(MissionError::Service(TradeError::UnsupportedStation))
    );
    assert_eq!(unsupported.save_bytes().unwrap(), before);
}

fn assert_paid_save_guards(world: &mut World) {
    use crate::save::{SaveError, Writer, checksum};
    let valid = world.save_bytes().unwrap();
    let mut account = Writer::new();
    world
        .commander_account
        .as_ref()
        .unwrap()
        .encode_save(&mut account);
    let matches: Vec<_> = valid
        .windows(account.bytes.len())
        .enumerate()
        .filter_map(|(i, b)| (b == account.bytes.as_slice()).then_some(i))
        .collect();
    assert_eq!(matches.len(), 1);
    let mut equipment = Writer::new();
    world
        .commander_account
        .as_ref()
        .unwrap()
        .equipment_state()
        .encode(&mut equipment, crate::save::SAVE_VERSION);
    // Paid ammunition and modular revisions follow equipment in the current account codec.
    // Locate the actual encoded component, rather than treating it as the account suffix.
    let equipment_offsets: Vec<_> = account
        .bytes
        .windows(equipment.bytes.len())
        .enumerate()
        .filter_map(|(i, b)| (b == equipment.bytes.as_slice()).then_some(i))
        .collect();
    assert_eq!(equipment_offsets.len(), 1);
    let start = matches[0] + equipment_offsets[0];
    assert_eq!(
        &valid[start..start + equipment.bytes.len()],
        equipment.bytes.as_slice()
    );
    for (offset, replacement) in [
        (0, 0_u64.to_le_bytes().to_vec()), // paid record cannot have no equipment transaction
        (9, 999_u64.to_le_bytes().to_vec()), // item belonging to another commander
        (17, vec![2]),                     // nonexistent second purchase identity
        (18, 999_u64.to_le_bytes().to_vec()), // foreign paid owner
        (26, 999_u64.to_le_bytes().to_vec()), // different bound hull
        (34, 2_u64.to_le_bytes().to_vec()), // unsupported receipt/transaction
        (42, u64::MAX.to_le_bytes().to_vec()), // future purchase revision
        (50, 999_u16.to_le_bytes().to_vec()), // unsupported purchase site
        (52, 0_u32.to_le_bytes().to_vec()), // unpaid fitted module
        (57, vec![0]),                     // fitted flag follows the XYZZY provenance byte
    ] {
        let mut bad = valid.clone();
        assert_ne!(
            &valid[start + offset..start + offset + replacement.len()],
            replacement.as_slice(),
            "corruption at equipment offset {offset} must change the target field"
        );
        bad[start + offset..start + offset + replacement.len()].copy_from_slice(&replacement);
        let end = bad.len() - 8;
        let digest = checksum(&bad[..end]);
        bad[end..].copy_from_slice(&digest.to_le_bytes());
        assert!(
            world.restore_bytes(&bad).is_err(),
            "equipment offset {offset} must be refused"
        );
        assert_eq!(world.save_bytes().unwrap(), valid);
    }
    // A coherently encoded stored foundation cannot keep the fitted intrinsic
    // zero-unit budget exemption underneath the paid utility.
    let mut stored_foundation = valid.clone();
    let shield = matches[0] + 61; // unchanged supported starter-fit prefix
    stored_foundation[shield + 19] = 0;
    stored_foundation.drain(shield + 20..shield + 28);
    let end = stored_foundation.len() - 8;
    let digest = checksum(&stored_foundation[..end]);
    stored_foundation[end..].copy_from_slice(&digest.to_le_bytes());
    assert!(world.restore_bytes(&stored_foundation).is_err());
    assert_eq!(world.save_bytes().unwrap(), valid);
    for version in [1_u16, 2] {
        let mut old = valid.clone();
        old[8..10].copy_from_slice(&version.to_le_bytes());
        let end = old.len() - 8;
        let digest = checksum(&old[..end]);
        old[end..].copy_from_slice(&digest.to_le_bytes());
        assert_eq!(
            world.restore_bytes(&old),
            Err(SaveError::UnsupportedVersion)
        );
        assert_eq!(world.save_bytes().unwrap(), valid);
    }
}

#[test]
fn current_paid_and_hot_world_continues_without_t02_and_refuses_an_original_v4_opening() {
    use crate::PracticeStatus;
    let mut world = earned_world(2411);
    let _paid = purchase(&mut world);
    world.request_launch(world.snapshot().tick).unwrap();
    world.step(FlightInput::default()).unwrap();
    world
        .request_fire_front_pulse(world.snapshot().tick)
        .unwrap();
    // The current scenario has real physical NPC history and retains it on Continue.
    let current = world.save_bytes().unwrap();
    let labelled = crate::flight::persistence::unsupported_version_test_input(&world, 4).unwrap();
    assert_eq!(
        World::from_save_bytes(&labelled).unwrap_err(),
        crate::save::SaveError::UnsupportedVersion
    );
    assert_eq!(world.save_bytes().unwrap(), current);
    let continued = World::from_save_bytes(&current).unwrap();
    assert_eq!(continued.traffic_snapshot(), world.traffic_snapshot());
    assert_eq!(continued.equipment_snapshot(), world.equipment_snapshot());
    assert_eq!(
        continued.pulse_laser_snapshot(),
        world.pulse_laser_snapshot()
    );

    let opening = World::new_commander(2411).unwrap();
    let old_opening =
        crate::flight::persistence::unsupported_version_test_input(&opening, 4).unwrap();
    assert_eq!(u16::from_le_bytes([old_opening[8], old_opening[9]]), 4);
    assert_eq!(
        World::from_save_bytes(&old_opening).unwrap_err(),
        crate::save::SaveError::UnsupportedVersion
    );
    assert_eq!(continued.snapshot(), world.snapshot());
    assert_eq!(continued.credits_tenths(), world.credits_tenths());
    assert_eq!(
        continued.practice_snapshot().unwrap().status,
        PracticeStatus::Unaccepted
    );
    assert_eq!(
        &continued.save_bytes().unwrap()[8..10],
        &crate::save::SAVE_VERSION.to_le_bytes()
    );
}

#[test]
fn prospective_cargo_equipment_drop_and_commit_preserve_full_account_chronology() {
    let mut world = earned_world(89801);
    let before = world.save_bytes().unwrap();
    let credit = world.credits_tenths().unwrap();
    let tick = world.snapshot().tick;
    let staged = {
        let prepared = world
            .prepare_equipment(Command::PurchaseCargoBay, tick)
            .unwrap();
        let bytes = prepared.bytes().to_vec();
        let cold = World::from_save_bytes(&bytes).unwrap();
        assert!(cold.equipment_snapshot().unwrap().owned.unwrap().fitted);
        assert_eq!(cold.credits_tenths(), Some(credit - 28000));
        bytes
    };
    assert_eq!(world.save_bytes().unwrap(), before);
    world
        .prepare_equipment(Command::PurchaseCargoBay, tick)
        .unwrap()
        .commit();
    assert_eq!(world.save_bytes().unwrap(), staged);
    assert_eq!(world.credits_tenths(), Some(credit - 28000));
    let item = world.equipment_snapshot().unwrap().owned.unwrap().id;
    let before = world.save_bytes().unwrap();
    drop(
        world
            .prepare_equipment(Command::UnfitCargoBay(item), tick)
            .unwrap(),
    );
    assert_eq!(world.save_bytes().unwrap(), before);
}

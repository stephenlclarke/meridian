//! Real public commands create all positive evidence. Direct edits are negative codec fixtures only.
use super::{
    NavigationTradeAction as Action, NavigationTradeError as Error,
    NavigationTradeOutcome as Outcome, NavigationTradeSnapshot, World,
};
use crate::freight::g02::{Choice, Command as Freight, Event as FreightEvent};
use crate::navigation::{DockOrigin, SystemId};
use crate::station_services::{Command, CommodityId as Id, Error as ServiceError, Event};
use crate::{FlightInput, FlightReceptionAction, MissionError};

fn ok<T, E: core::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("unexpected refusal: {e:?}"))
}
fn some<T>(v: Option<T>) -> T {
    v.unwrap_or_else(|| panic!("missing actual state"))
}
fn commander(seed: u64) -> World {
    ok(World::new_commander(seed))
}
fn funded(seed: u64) -> World {
    let mut world = crate::flight::equipment::tests::earned_world(seed);
    service(&mut world, Command::RefillFuel);
    world
}
fn snap(w: &World) -> NavigationTradeSnapshot {
    some(w.navigation_trade_snapshot())
}
fn steps(w: &mut World, n: usize) {
    for _ in 0..n {
        ok(w.step(FlightInput::default()));
    }
}
fn confirm(w: &mut World, action: Action) {
    let tick = w.snapshot().tick;
    let review = ok(w.review_navigation_trade(action, tick));
    steps(w, 3);
    let tick = w.snapshot().tick;
    let prepared = ok(w.prepare_navigation_trade(review, tick));
    let bytes = prepared.bytes().to_vec();
    let staged = ok(World::from_save_bytes(&bytes));
    let expected = snap(&staged);
    prepared.commit();
    assert_eq!(snap(w), expected);
    assert_eq!(ok(w.save_bytes()), bytes);
}
fn plan(w: &mut World, id: Id) {
    confirm(w, Action::Accept);
    let source = some(w.current_dock_context()).system();
    let destination = if source == SystemId::AVENOR {
        SystemId::RENAS
    } else {
        SystemId::AVENOR
    };
    confirm(
        w,
        Action::Plan {
            destination,
            commodity: id,
        },
    );
}
fn service(w: &mut World, command: Command) -> Event {
    let tick = w.snapshot().tick;
    ok(w.request_station_service(command, tick))
}
fn journey(w: &mut World, to: SystemId) {
    crate::flight::equipment::tests::journey(w, to);
    assert_eq!(some(w.current_dock_context()).system(), to);
    assert_eq!(
        some(w.current_dock_context()).origin(),
        DockOrigin::PhysicalEntry
    );
}
fn cold(w: &World) -> World {
    let bytes = ok(w.save_bytes());
    assert_eq!(
        u16::from_le_bytes([bytes[8], bytes[9]]),
        crate::save::SAVE_VERSION
    );
    let loaded = ok(World::from_save_bytes(&bytes));
    assert_eq!(loaded.snapshot(), w.snapshot());
    assert_eq!(snap(&loaded), snap(w));
    assert_eq!(loaded.credits_tenths(), w.credits_tenths());
    assert_eq!(loaded.service_snapshot(), w.service_snapshot());
    assert_eq!(loaded.missile_snapshot(), w.missile_snapshot());
    assert_eq!(loaded.pulse_laser_snapshot(), w.pulse_laser_snapshot());
    assert_eq!(loaded.combat_snapshot(), w.combat_snapshot());
    assert_eq!(
        loaded.practice_ledger_snapshot(),
        w.practice_ledger_snapshot()
    );
    assert_eq!(
        loaded.flight_reception_snapshot(),
        w.flight_reception_snapshot()
    );
    for id in Id::ALL {
        assert_eq!(loaded.commodity_quantity(id), w.commodity_quantity(id));
        assert_eq!(loaded.commodity_quote(id), w.commodity_quote(id));
    }
    loaded
}
fn reject(w: &mut World, change: impl FnOnce(&mut World)) {
    let before = ok(w.save_bytes());
    let snapshot = snap(w);
    let mut invalid = cold(w);
    change(&mut invalid);
    let bytes = ok(invalid.unchecked_test_save());
    assert!(World::from_save_bytes(&bytes).is_err());
    assert!(w.restore_bytes(&bytes).is_err());
    assert_eq!(ok(w.save_bytes()), before);
    assert_eq!(snap(w), snapshot);
}
fn complete(w: &mut World, id: Id) -> super::NavigationTradeTrace {
    let before = some(w.credits_tenths());
    let prior_trace = snap(w).trace;
    plan(w, id);
    assert_eq!(some(w.credits_tenths()), before);
    assert_eq!(snap(w).reserved_capacity_tonnes, 1);
    assert_eq!(w.commodity_quantity(id), Some(0));
    let quote = some(w.commodity_quote(id));
    service(w, Command::BuyCommodity(id));
    let buy = some(snap(w).attempts.last()).buy;
    let buy = some(buy);
    assert_eq!(buy.price_tenths, quote.ask_tenths);
    assert_eq!(buy.cargo_before, 0);
    assert_eq!(buy.cargo_after, 1);
    assert_eq!(snap(w).reserved_capacity_tonnes, 0);
    assert_eq!(w.reserved_commodity_quantity(id), Some(1));
    assert_eq!(snap(w).trace, prior_trace);
    let held = some(snap(w).attempts.last()).held_tonne;
    assert_eq!(held, Some(buy.tonne));
    *w = cold(w);
    journey(w, SystemId::RENAS);
    let sold_quote = some(w.commodity_quote(id));
    service(w, Command::SellCommodity(id));
    let sale = some(some(snap(w).attempts.last()).sale);
    assert_eq!(sale.tonne, buy.tonne);
    assert_eq!(sale.price_tenths, sold_quote.bid_tenths);
    assert_eq!(sale.cargo_after + 1, sale.cargo_before);
    assert_eq!(w.reserved_commodity_quantity(id), Some(0));
    assert_eq!(snap(w).trace, prior_trace);
    assert_eq!(
        w.review_navigation_trade(Action::Receive, w.snapshot().tick),
        Err(Error::NotReady)
    );
    let cash = some(w.credits_tenths());
    let fuel_before = some(w.snapshot().navigation).fuel_units;
    let paid = service(w, Command::RefillFuel).paid_tenths();
    assert_eq!(paid, u32::from(130 - fuel_before) * 2);
    assert!(paid > 0);
    assert_eq!(some(w.credits_tenths()), cash - paid);
    *w = cold(w);
    journey(w, SystemId::AVENOR);
    assert!(snap(w).ready);
    let a = some(snap(w).attempts.last()).clone();
    assert_eq!(a.journeys, 2);
    assert_eq!(a.fuel_units_used, 68);
    assert_eq!(a.fuel_paid_tenths, paid);
    let difference = i64::from(sale.price_tenths) - i64::from(buy.price_tenths) - i64::from(paid);
    assert_eq!(a.cash_difference_tenths, Some(difference));
    assert_eq!(
        i64::from(some(w.credits_tenths())) - i64::from(before),
        difference
    );
    let cash = some(w.credits_tenths());
    confirm(w, Action::Receive);
    assert_eq!(some(w.credits_tenths()), cash);
    assert_eq!(
        some(snap(w).attempts.last()).outcome,
        Some(Outcome::Complete)
    );
    let trace = some(snap(w).trace);
    assert_eq!(trace.source_id, "l337.training/navigation-trade");
    assert_eq!(
        trace.receipt_id,
        "l337.unwritten-route/clue/navigation-trade-family-trace"
    );
    let _continued = cold(w);
    trace
}

#[test]
fn genuine_food_loss_and_mineral_profit_keep_exact_tonne_paid_fuel_and_zero_lesson_cash() {
    for (i, id, expected) in [(0, Id::Food, -98_i64), (1, Id::Minerals, 78_i64)] {
        let mut w = commander(6101 + i);
        let rounds = some(w.missile_snapshot()).rounds;
        let trace = complete(&mut w, id);
        assert_eq!(snap(&w).attempts[0].cash_difference_tenths, Some(expected));
        assert_eq!(some(w.missile_snapshot()).rounds, rounds);
        assert_eq!(
            w.review_navigation_trade(Action::Receive, w.snapshot().tick),
            Err(Error::NoActiveAttempt)
        );
        assert_eq!(trace.clue_id, "navigation-trade-family-trace");
    }
}
#[test]
fn repeated_success_retains_first_trace_and_all_old_family_resources() {
    let mut w = commander(6201);
    let trace = complete(&mut w, Id::Minerals);
    let old = snap(&w).attempts[0].clone();
    let repeated = complete(&mut w, Id::Food);
    assert_eq!(repeated, trace);
    assert_eq!(snap(&w).attempts[0], old);
    assert_eq!(snap(&w).attempts.len(), 2);
    assert_eq!(some(w.practice_ledger_snapshot()).trace, None);
    assert_eq!(some(w.flight_reception_snapshot()).trace, None);
    assert_eq!(some(w.mission_snapshot()).freight.clue, None);
}
#[test]
fn separate_accept_plan_back_and_market_fuel_terminal_drop_preserve_live_bytes() {
    let mut w = commander(6301);
    let review = ok(w.review_navigation_trade(Action::Accept, 0));
    let before = ok(w.save_bytes());
    drop(ok(w.prepare_navigation_trade(review, 0)));
    assert_eq!(ok(w.save_bytes()), before);
    confirm(&mut w, Action::Accept);
    assert_eq!(snap(&w).reserved_capacity_tonnes, 0);
    assert!(snap(&w).attempts[0].plan.is_none());
    let tick = w.snapshot().tick;
    let action = Action::Plan {
        destination: SystemId::RENAS,
        commodity: Id::Food,
    };
    let review = ok(w.review_navigation_trade(action, tick));
    let before = ok(w.save_bytes());
    let prepared = ok(w.prepare_navigation_trade(review, tick));
    let staged = ok(World::from_save_bytes(prepared.bytes()));
    assert_eq!(snap(&staged).reserved_capacity_tonnes, 1);
    drop(prepared);
    assert_eq!(ok(w.save_bytes()), before);
    confirm(&mut w, action);
    for command in [Command::BuyFood, Command::SellFood, Command::RefillFuel] {
        if command == Command::SellFood {
            service(&mut w, Command::BuyFood);
            journey(&mut w, SystemId::RENAS);
        }
        let tick = w.snapshot().tick;
        let before = ok(w.save_bytes());
        let prepared = ok(w.prepare_station_service(command, tick));
        let event = prepared.event();
        let candidate = ok(World::from_save_bytes(prepared.bytes()));
        assert_ne!(prepared.bytes(), before.as_slice());
        match event {
            Event::BoughtFood { .. } => assert_eq!(candidate.commodity_quantity(Id::Food), Some(1)),
            Event::SoldFood { .. } => assert!(snap(&candidate).attempts[0].sale.is_some()),
            Event::Refuelled { .. } => {
                assert_eq!(some(candidate.snapshot().navigation).fuel_units, 130)
            }
            _ => panic!("expected actual tutorial market boundary"),
        }
        drop(prepared);
        assert_eq!(ok(w.save_bytes()), before);
        if command == Command::SellFood {
            assert!(matches!(event, Event::SoldFood { .. }));
            service(&mut w, command);
        }
    }
    journey(&mut w, SystemId::AVENOR);
    let tick = w.snapshot().tick;
    let review = ok(w.review_navigation_trade(Action::Receive, tick));
    let before = ok(w.save_bytes());
    let prepared = ok(w.prepare_navigation_trade(review, tick));
    assert!(
        snap(&ok(World::from_save_bytes(prepared.bytes())))
            .trace
            .is_some()
    );
    drop(prepared);
    assert_eq!(ok(w.save_bytes()), before);
    assert_eq!(snap(&w).trace, None);
    confirm(&mut w, Action::Receive);
}
#[test]
fn stale_foreign_continue_pause_future_and_actual_plan_changes_refuse_without_mutation() {
    let mut w = commander(6401);
    let mut foreign = commander(6402);
    let review = ok(w.review_navigation_trade(Action::Accept, 0));
    let before = ok(foreign.save_bytes());
    assert!(foreign.prepare_navigation_trade(review, 0).is_err());
    assert_eq!(ok(foreign.save_bytes()), before);
    steps(&mut w, 4);
    assert!(w.prepare_navigation_trade(review, 0).is_err());
    let future = super::NavigationTradeReview { tick: 5, ..review };
    assert!(w.prepare_navigation_trade(future, 4).is_err());
    drop(ok(w.prepare_navigation_trade(review, 4)));
    let bytes = ok(w.save_bytes());
    ok(w.restore_bytes(&bytes));
    assert!(w.prepare_navigation_trade(review, 4).is_err());
    ok(w.request_pause(true, 4));
    assert_eq!(
        w.review_navigation_trade(Action::Accept, 4),
        Err(Error::Paused)
    );
    ok(w.request_pause(false, 4));
    confirm(&mut w, Action::Accept);
    let action = Action::Plan {
        destination: SystemId::RENAS,
        commodity: Id::Food,
    };
    let tick = w.snapshot().tick;
    let review = ok(w.review_navigation_trade(action, tick));
    service(&mut w, Command::BuyCommodity(Id::Textiles));
    let before = ok(w.save_bytes());
    assert!(
        w.prepare_navigation_trade(review, w.snapshot().tick)
            .is_err()
    );
    assert_eq!(ok(w.save_bytes()), before);
    confirm(&mut w, action);
    let tick = w.snapshot().tick;
    let review = ok(w.review_navigation_trade(Action::Cancel, tick));
    let before = ok(w.save_bytes());
    drop(ok(w.prepare_navigation_trade(review, tick)));
    assert_eq!(ok(w.save_bytes()), before);
    confirm(&mut w, Action::Cancel);
    assert_eq!(snap(&w).reserved_capacity_tonnes, 0);
}
#[test]
fn chosen_routes_units_legal_tonnes_existing_cargo_capacity_and_actual_fuel_are_required() {
    let mut w = commander(6501);
    confirm(&mut w, Action::Accept);
    let tick = w.snapshot().tick;
    for destination in [
        SystemId::AVENOR,
        SystemId {
            galaxy: 0,
            system: 0,
        },
        SystemId {
            galaxy: 1,
            system: 185,
        },
        SystemId {
            galaxy: 0,
            system: 256,
        },
    ] {
        assert_eq!(
            w.review_navigation_trade(
                Action::Plan {
                    destination,
                    commodity: Id::Food
                },
                tick
            ),
            Err(Error::UnsupportedRoute)
        );
    }
    for commodity in [
        Id::Gold,
        Id::Platinum,
        Id::GemStones,
        Id::Slaves,
        Id::Narcotics,
        Id::Weapons,
        Id::AlienItems,
    ] {
        assert_eq!(
            w.review_navigation_trade(
                Action::Plan {
                    destination: SystemId::RENAS,
                    commodity
                },
                tick
            ),
            Err(Error::CommodityRequired)
        );
    }
    service(&mut w, Command::BuyFood);
    assert_eq!(
        w.review_navigation_trade(
            Action::Plan {
                destination: SystemId::RENAS,
                commodity: Id::Food
            },
            tick
        ),
        Err(Error::CargoPresent)
    );
    let mut fuel = commander(6502);
    for to in [SystemId::RENAS, SystemId::AVENOR, SystemId::RENAS] {
        journey(&mut fuel, to);
    }
    assert_eq!(some(fuel.snapshot().navigation).fuel_units, 28);
    confirm(&mut fuel, Action::Accept);
    assert_eq!(
        fuel.review_navigation_trade(
            Action::Plan {
                destination: SystemId::AVENOR,
                commodity: Id::Food
            },
            fuel.snapshot().tick
        ),
        Err(Error::Fuel)
    );
    assert!(service(&mut fuel, Command::RefillFuel).paid_tenths() > 0);
    confirm(
        &mut fuel,
        Action::Plan {
            destination: SystemId::AVENOR,
            commodity: Id::Food,
        },
    );
}
#[test]
fn genuine_depletion_and_no_purchase_credits_refuse_without_restock_or_fake_stock() {
    let mut w = funded(6601);
    assert_eq!(some(w.current_dock_context()).system(), SystemId::RENAS);
    assert_eq!(some(w.commodity_quote(Id::Radioactives)).stock, 15);
    for _ in 0..15 {
        service(&mut w, Command::BuyCommodity(Id::Radioactives));
    }
    assert_eq!(some(w.commodity_quote(Id::Radioactives)).stock, 0);
    journey(&mut w, SystemId::AVENOR);
    for _ in 0..15 {
        service(&mut w, Command::SellCommodity(Id::Radioactives));
    }
    service(&mut w, Command::RefillFuel);
    journey(&mut w, SystemId::RENAS);
    confirm(&mut w, Action::Accept);
    let before = ok(w.save_bytes());
    assert_eq!(
        w.review_navigation_trade(
            Action::Plan {
                destination: SystemId::AVENOR,
                commodity: Id::Radioactives
            },
            w.snapshot().tick
        ),
        Err(Error::Stock)
    );
    assert_eq!(ok(w.save_bytes()), before);
    let mut poor = commander(6602);
    confirm(&mut poor, Action::Accept);
    assert_eq!(
        poor.review_navigation_trade(
            Action::Plan {
                destination: SystemId::RENAS,
                commodity: Id::Luxuries
            },
            poor.snapshot().tick
        ),
        Err(Error::Credits)
    );
}
#[test]
fn real_reserved_slot_transfers_to_owned_tonne_and_blocks_other_capacity_consumers() {
    let mut w = funded(6701);
    for _ in 0..92 {
        service(&mut w, Command::BuyCommodity(Id::Textiles));
    }
    confirm(&mut w, Action::Accept);
    let action = Action::Plan {
        destination: SystemId::AVENOR,
        commodity: Id::Food,
    };
    assert_eq!(
        w.review_navigation_trade(action, w.snapshot().tick),
        Err(Error::Capacity)
    );
    service(&mut w, Command::SellCommodity(Id::Textiles));
    confirm(&mut w, action);
    assert_eq!(some(w.service_snapshot()).used_tonnes, 92);
    let before = ok(w.save_bytes());
    assert_eq!(
        w.request_station_service(Command::BuyCommodity(Id::Minerals), w.snapshot().tick),
        Err(MissionError::Service(ServiceError::HoldFull))
    );
    assert_eq!(ok(w.save_bytes()), before);
    service(&mut w, Command::BuyFood);
    assert_eq!(some(w.service_snapshot()).used_tonnes, 92);
    assert_eq!(w.commodity_quantity(Id::Food), Some(1));
    assert_eq!(w.reserved_commodity_quantity(Id::Food), Some(1));
    let _continued = cold(&w);
}
#[test]
fn wrong_station_unrelated_extra_or_replacement_sale_never_substitutes_identified_original() {
    let mut w = commander(6801);
    plan(&mut w, Id::Food);
    service(&mut w, Command::BuyFood);
    let original = some(snap(&w).attempts[0].buy).tonne;
    service(&mut w, Command::BuyFood);
    assert_eq!(w.commodity_quantity(Id::Food), Some(2));
    service(&mut w, Command::SellFood);
    assert_eq!(snap(&w).attempts[0].held_tonne, Some(original));
    assert_eq!(snap(&w).attempts[0].sale, None);
    let before = ok(w.save_bytes());
    assert_eq!(
        w.request_station_service(Command::SellFood, w.snapshot().tick),
        Err(MissionError::Service(ServiceError::NoSellableCargo))
    );
    assert_eq!(ok(w.save_bytes()), before);
    service(&mut w, Command::BuyCommodity(Id::Textiles));
    service(&mut w, Command::SellCommodity(Id::Textiles));
    assert_eq!(snap(&w).attempts[0].sale, None);
    journey(&mut w, SystemId::RENAS);
    service(&mut w, Command::SellFood);
    let sale = some(snap(&w).attempts[0].sale);
    assert_eq!(sale.tonne, original);
    service(&mut w, Command::BuyFood);
    service(&mut w, Command::SellFood);
    assert_eq!(snap(&w).attempts[0].sale, Some(sale));
    journey(&mut w, SystemId::AVENOR);
    confirm(&mut w, Action::Receive);
    assert_eq!(snap(&w).attempts[0].sale, Some(sale));
}
#[test]
fn cancellation_releases_original_without_refunds_restock_or_replacement_revival() {
    let mut w = commander(6901);
    plan(&mut w, Id::Food);
    service(&mut w, Command::BuyFood);
    let before_cash = w.credits_tenths();
    let before_quote = w.commodity_quote(Id::Food);
    let buy = some(snap(&w).attempts[0].buy);
    confirm(&mut w, Action::Cancel);
    assert_eq!(w.credits_tenths(), before_cash);
    assert_eq!(w.commodity_quote(Id::Food), before_quote);
    assert_eq!(w.commodity_quantity(Id::Food), Some(1));
    assert_eq!(w.reserved_commodity_quantity(Id::Food), Some(0));
    assert_eq!(snap(&w).trace, None);
    service(&mut w, Command::SellFood);
    plan(&mut w, Id::Food);
    service(&mut w, Command::BuyFood);
    let replacement = some(snap(&w).attempts[1].buy);
    assert_ne!(buy.tonne, replacement.tonne);
    assert_eq!(snap(&w).attempts[0].outcome, Some(Outcome::Abandoned));
    assert_eq!(snap(&w).attempts[0].buy, Some(buy));
    let _continued = cold(&w);
}
#[test]
fn actual_three_owner_reservations_refuse_a_fourth_without_fabricated_occupancy() {
    let mut w = commander(7001);
    let review = ok(w.review_flight_reception(FlightReceptionAction::Accept, 0));
    ok(w.prepare_flight_reception(review, 0)).commit();
    crate::flight::practice::tests::consent(&mut w);
    let npc = w.station_contacts()[0];
    ok(w.request_contact(npc, 0));
    let FreightEvent::Reviewed(review) =
        ok(w.request_freight(Freight::ReviewOffer { actor: npc.actor() }, 0))
    else {
        panic!("real offer required")
    };
    ok(w.request_freight(
        Freight::Confirm {
            review,
            choice: Choice::Accept,
        },
        0,
    ));
    assert_eq!(snap(&w).active_services, 3);
    let before = ok(w.save_bytes());
    assert_eq!(
        w.review_navigation_trade(Action::Accept, 0),
        Err(Error::Capacity)
    );
    assert_eq!(ok(w.save_bytes()), before);
    let review = ok(w.review_flight_reception(FlightReceptionAction::Cancel, 0));
    ok(w.prepare_flight_reception(review, 0)).commit();
    confirm(&mut w, Action::Accept);
    assert_eq!(snap(&w).active_services, 3);
    assert_eq!(some(w.practice_ledger_snapshot()).active_services, 3);
    assert_eq!(
        w.review_flight_reception(FlightReceptionAction::Accept, w.snapshot().tick),
        Err(crate::FlightReceptionError::SharedCapacity)
    );
    let mut w = commander(7002);
    confirm(&mut w, Action::Accept);
    let tick = w.snapshot().tick;
    let review = ok(w.review_flight_reception(FlightReceptionAction::Accept, tick));
    ok(w.prepare_flight_reception(review, tick)).commit();
    crate::flight::practice::tests::consent(&mut w);
    let npc = w.station_contacts()[0];
    let tick = w.snapshot().tick;
    ok(w.request_contact(npc, tick));
    let FreightEvent::Reviewed(review) =
        ok(w.request_freight(Freight::ReviewOffer { actor: npc.actor() }, tick))
    else {
        panic!("real offer required")
    };
    let before = ok(w.save_bytes());
    assert_eq!(
        w.request_freight(
            Freight::Confirm {
                review,
                choice: Choice::Accept
            },
            tick
        ),
        Err(MissionError::SharedCapacity)
    );
    assert_eq!(ok(w.save_bytes()), before);
}
#[test]
fn current_fit_revision_changed_after_plan_refuses_actual_purchase_atomically() {
    let mut w = funded(7101);
    let tick = w.snapshot().tick;
    ok(w.request_equipment(crate::equipment::Command::PurchaseCargoBay, tick));
    let item = some(some(w.equipment_snapshot()).owned).id;
    plan(&mut w, Id::Food);
    let tick = w.snapshot().tick;
    ok(w.request_equipment(crate::equipment::Command::UnfitCargoBay(item), tick));
    let before = ok(w.save_bytes());
    assert_eq!(
        w.request_station_service(Command::BuyFood, tick),
        Err(MissionError::NavigationTrade(Error::FitOrStockChanged))
    );
    assert_eq!(ok(w.save_bytes()), before);
    assert_eq!(w.commodity_quantity(Id::Food), Some(0));
    confirm(&mut w, Action::Cancel);
    plan(&mut w, Id::Food);
    service(&mut w, Command::BuyFood);
    assert!(snap(&w).attempts[1].held_tonne.is_some());
}
#[test]
fn checked_receipts_plan_marker_and_chronology_tamper_never_replace_live_cold_state() {
    let mut w = commander(7201);
    plan(&mut w, Id::Food);
    service(&mut w, Command::BuyFood);
    reject(&mut w, |w| w.navigation_trade.attempts[0].buy = None);
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].buy.as_mut())
            .tonne
            .transaction += 1
    });
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].buy.as_mut()).price += 1
    });
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].plan.as_mut()).commodity = Id::Gold
    });
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].plan.as_mut()).fuel_debit += 1
    });
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].buy.as_mut()).transaction = u64::MAX
    });
    journey(&mut w, SystemId::RENAS);
    service(&mut w, Command::SellFood);
    service(&mut w, Command::RefillFuel);
    journey(&mut w, SystemId::AVENOR);
    confirm(&mut w, Action::Receive);
    reject(&mut w, |w| w.navigation_trade.trace = None);
    reject(&mut w, |w| w.navigation_trade.trace = Some(127));
    reject(&mut w, |w| {
        w.navigation_trade.attempts[0].journeys[0].debit += 1
    });
    reject(&mut w, |w| {
        w.navigation_trade.attempts[0].journeys[0].jump += 1
    });
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].sale.as_mut())
            .tonne
            .attempt += 1
    });
    reject(&mut w, |w| w.navigation_trade.attempts[0].fuel[0].paid += 1);
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].terminal.as_mut()).dock =
            w.navigation_trade.attempts[0].dock
    });
    reject(&mut w, |w| {
        let j = w.navigation_trade.attempts[0].journeys[0];
        w.navigation_trade.attempts[0].journeys.resize(65, j);
    });
    confirm(&mut w, Action::Accept);
    steps(&mut w, 10);
    reject(&mut w, |w| {
        let tick = w.navigation_trade.attempts[1].accepted_tick + 1;
        some(w.navigation_trade.attempts[0].terminal.as_mut()).tick = tick;
    });
}
#[test]
fn exact_v10_reader_retains_completed_t01_without_inventing_t04_plan_or_tonne() {
    let mut w = commander(7301);
    let review = ok(w.review_flight_reception(FlightReceptionAction::Accept, 0));
    ok(w.prepare_flight_reception(review, 0)).commit();
    crate::flight::practice::tests::launch(&mut w);
    crate::flight::missiles::tests::stop(&mut w);
    crate::flight::practice::tests::aim_at(&mut w, crate::docking::Vec3::new(0.0, 0.0, 0.0));
    for _ in 0..2 {
        ok(w.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        }));
    }
    crate::flight::missiles::tests::dock(&mut w);
    let tick = w.snapshot().tick;
    let review = ok(w.review_flight_reception(FlightReceptionAction::Receive, tick));
    ok(w.prepare_flight_reception(review, tick)).commit();
    let t01 = some(w.flight_reception_snapshot());
    let bytes = ok(crate::flight::persistence::v10_test_fixture(&w));
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 10);
    let mut loaded = ok(World::from_save_bytes(&bytes));
    assert_eq!(some(loaded.flight_reception_snapshot()), t01);
    assert!(snap(&loaded).attempts.is_empty());
    assert_eq!(snap(&loaded).trace, None);
    let _upgraded = cold(&loaded);
    complete(&mut loaded, Id::Minerals);
    assert_eq!(some(loaded.flight_reception_snapshot()).trace, t01.trace);
    assert_eq!(
        some(loaded.flight_reception_snapshot()).attempts,
        t01.attempts
    );
    confirm(&mut w, Action::Accept);
    assert!(crate::flight::persistence::v10_test_fixture(&w).is_err());
}

#[test]
fn actual_trade_round_trip_keeps_eight_original_g02_units_and_gunnery_owner() {
    let mut w = commander(7401);
    let npc = w.station_contacts()[0];
    ok(w.request_contact(npc, 0));
    let FreightEvent::Reviewed(review) =
        ok(w.request_freight(Freight::ReviewOffer { actor: npc.actor() }, 0))
    else {
        panic!("real freight offer required")
    };
    let FreightEvent::Accepted { job, units } = ok(w.request_freight(
        Freight::Confirm {
            review,
            choice: Choice::Accept,
        },
        0,
    )) else {
        panic!("real original allocation required")
    };
    crate::flight::practice::tests::consent(&mut w);
    let freight = some(w.mission_snapshot()).freight;
    assert_eq!(freight.held_tonnes, 8);
    complete(&mut w, Id::Food);
    assert_eq!(w.active_freight_job(), Some(job));
    assert_eq!(some(w.mission_snapshot()).freight, freight);
    assert_eq!(some(w.service_snapshot()).used_tonnes, 8);
    assert_eq!(snap(&w).active_services, 2);
    assert_eq!(some(w.practice_ledger_snapshot()).attempts.len(), 1);
    assert_eq!(some(w.practice_ledger_snapshot()).trace, None);
    assert!(
        units
            .iter()
            .enumerate()
            .all(|(i, unit)| !units[..i].contains(unit))
    );
    let _continued = cold(&w);
}

#[test]
fn omitted_actual_fuel_receipt_refuses_active_and_completed_food_journeys_atomically() {
    let mut active = commander(7501);
    plan(&mut active, Id::Food);
    service(&mut active, Command::BuyFood);
    journey(&mut active, SystemId::RENAS);
    assert_eq!(service(&mut active, Command::RefillFuel).paid_tenths(), 68);
    assert_eq!(snap(&active).attempts[0].fuel_paid_tenths, 68);
    // Coherent-checksum negative fixture: only the retained receipt list is erased.
    // Actual wallet, shop counters, tank and journey evidence stay byte-for-byte unchanged.
    reject(&mut active, |w| w.navigation_trade.attempts[0].fuel.clear());

    let mut completed = commander(7502);
    complete(&mut completed, Id::Food);
    assert_eq!(
        snap(&completed).attempts[0].cash_difference_tenths,
        Some(-98)
    );
    reject(&mut completed, |w| {
        w.navigation_trade.attempts[0].fuel.clear()
    });
    assert_eq!(snap(&completed).attempts[0].fuel_paid_tenths, 68);
    assert_eq!(
        snap(&completed).attempts[0].cash_difference_tenths,
        Some(-98)
    );
    assert!(snap(&completed).trace.is_some());
}

#[test]
fn fuel_continuity_refuses_changed_tank_revision_and_terminal_anchors() {
    let mut w = commander(7601);
    complete(&mut w, Id::Food);
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].buy.as_mut()).fuel_at_trade -= 1
    });
    reject(&mut w, |w| {
        let j = &mut w.navigation_trade.attempts[0].journeys[1];
        j.fuel_before -= 1;
        j.fuel_after -= 1; // Debit arithmetic still balances; tank chain does not.
    });
    reject(&mut w, |w| {
        let revision = w.navigation_trade.attempts[0].fuel[0].account_revision;
        w.navigation_trade.attempts[0].journeys[1].account_revision = revision;
    });
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].terminal.as_mut())
            .fuel
            .fuel -= 1
    });
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].terminal.as_mut())
            .fuel
            .refilled_units = 0
    });
    reject(&mut w, |w| {
        some(w.navigation_trade.attempts[0].plan.as_mut()).refilled_at_plan += 1
    });
}

#[test]
fn ordinary_fuel_before_plan_and_after_terminal_stays_outside_lesson_cost() {
    let mut w = commander(7701);
    journey(&mut w, SystemId::RENAS);
    assert_eq!(service(&mut w, Command::RefillFuel).paid_tenths(), 68);
    journey(&mut w, SystemId::AVENOR);
    assert_eq!(some(w.snapshot().navigation).fuel_units, 96);
    complete(&mut w, Id::Food);
    let before = snap(&w).attempts[0].clone();
    assert_eq!(before.fuel_paid_tenths, 136);
    assert_eq!(before.cash_difference_tenths, Some(-166));
    // A genuine later ordinary purchase changes global refill counters and the wallet,
    // while the completed lesson retains its own actual end anchor and cost.
    assert_eq!(service(&mut w, Command::RefillFuel).paid_tenths(), 68);
    assert_eq!(snap(&w).attempts[0], before);
    let _continued = cold(&w);
}

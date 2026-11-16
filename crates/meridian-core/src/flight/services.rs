//! Actual World station admission and the sole account/market transaction.
use super::{MissionError, World};
use crate::navigation::SystemId;
use crate::station_services::{
    self as services, CommodityId, CommodityQuote, FoodQuote, MarketSite, ServiceFacts,
};

/// Presentation only. Editing this copy cannot create cargo, a price or service admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceSnapshot {
    pub food_tonnes: u16,
    pub reserved_food_tonnes: u16,
    pub used_tonnes: u16,
    pub current_market: Option<FoodQuote>,
}

impl World {
    pub fn commodity_quote(&self, id: CommodityId) -> Option<CommodityQuote> {
        let site = service_site(self.current_dock_context()?.system())?;
        Some(self.station_markets.as_ref()?.commodity_quote(site, id))
    }
    pub fn commodity_quantity(&self, id: CommodityId) -> Option<u64> {
        let account = self.commander_account.as_ref()?;
        let projection = account
            .service_projection(&self.freight_session.as_ref()?.state)
            .ok()?;
        Some(if id == CommodityId::Food {
            u64::from(projection.food_tonnes)
        } else {
            projection.other_commodities[id.index() - 1]
        })
    }
    pub fn reserved_commodity_quantity(&self, id: CommodityId) -> Option<u64> {
        let account = self.commander_account.as_ref()?;
        let projection = account
            .service_projection(&self.freight_session.as_ref()?.state)
            .ok()?;
        Some(if id == CommodityId::Food {
            u64::from(projection.reserved_food_tonnes)
        } else {
            projection.reserved_commodities[id.index() - 1]
        })
    }
    pub fn legal_value(&self) -> Option<u8> {
        Some(self.commander_account.as_ref()?.legal_value())
    }

    pub fn service_snapshot(&self) -> Option<ServiceSnapshot> {
        let session = self.freight_session.as_ref()?;
        let account = self.commander_account.as_ref()?;
        let projection = account.service_projection(&session.state).ok()?;
        let site = self
            .current_dock_context()
            .and_then(|dock| service_site(dock.system()));
        Some(ServiceSnapshot {
            food_tonnes: projection.food_tonnes,
            reserved_food_tonnes: projection.reserved_food_tonnes,
            used_tonnes: projection.used_tonnes,
            current_market: self
                .station_markets
                .as_ref()
                .zip(site)
                .map(|(market, site)| market.quote(site)),
        })
    }

    /// One explicit ordinary action at the actual current supported main-orbital station.
    /// Neither a chosen site, a copied DockContext nor a caller price is a command argument.
    pub fn request_station_service(
        &mut self,
        command: services::Command,
        expected_tick: u64,
    ) -> Result<services::Event, MissionError> {
        let session = self.station_command_admission(expected_tick)?;
        let dock = self.current_dock_context().ok_or(MissionError::NotDocked)?;
        let site = service_site(dock.system())
            .ok_or(MissionError::Service(services::Error::UnsupportedStation))?;
        let facts = ServiceFacts {
            owner: dock.owner().seed(),
            hull: dock.ship().ordinal(),
            site,
            station_entity: dock.actor_entity(),
            visit: dock.visit(),
            dock_sequence: dock.sequence(),
            epoch: session.epoch(),
            paused: self.is_paused(),
        };
        let account = self
            .commander_account
            .as_mut()
            .ok_or(MissionError::NoCommander)?;
        let session = self
            .freight_session
            .as_ref()
            .ok_or(MissionError::NoCommander)?;
        let markets = self
            .station_markets
            .as_mut()
            .ok_or(MissionError::NoCommander)?;
        account
            .execute_station_service(&session.state, markets, facts, command)
            .map_err(MissionError::Service)
    }

    /// Compatibility entry point; the single shared service rule computes and applies payment.
    pub fn request_refuel(&mut self, expected_tick: u64) -> Result<u32, MissionError> {
        self.request_station_service(services::Command::RefillFuel, expected_tick)
            .map(services::Event::paid_tenths)
    }
}

fn service_site(system: SystemId) -> Option<MarketSite> {
    match system {
        SystemId::AVENOR => Some(MarketSite::Avenor),
        SystemId::RENAS => Some(MarketSite::Renas),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::freight::g02::{self, Choice, GoodsId, JobId, Review};
    use crate::navigation::BASIC_SPOOL_TICKS;
    use crate::{FlightInput, FlightMode, MissionSnapshot, ServiceSnapshot, Snapshot, StationNpc};
    use services::{Command, Error, Event};

    fn action(world: &mut World, command: Command) -> Result<Event, MissionError> {
        world.request_station_service(command, world.snapshot().tick)
    }
    fn freight(world: &mut World, command: g02::Command) -> Result<g02::Event, MissionError> {
        world.request_freight(command, world.snapshot().tick)
    }
    fn review(world: &mut World, command: g02::Command) -> Review {
        match freight(world, command).unwrap() {
            g02::Event::Reviewed(review) => review,
            event => panic!("expected freight review, got {event:?}"),
        }
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
    fn refuses(world: &mut World, command: Command, error: Error) {
        let before = fingerprint(world);
        assert_eq!(action(world, command), Err(MissionError::Service(error)));
        assert_eq!(fingerprint(world), before);
    }
    fn jump(world: &mut World, system: SystemId) {
        world.request_jump(system, world.snapshot().tick).unwrap();
        for _ in 0..BASIC_SPOOL_TICKS {
            world.step(FlightInput::default()).unwrap();
        }
        assert_eq!(world.snapshot().mode, FlightMode::Flight);
        assert!(world.current_dock_context().is_none());
        assert_eq!(world.service_snapshot().unwrap().current_market, None);
    }
    fn dock(world: &mut World) {
        for _ in 0..9000 {
            let before = world.snapshot();
            if before.mode == FlightMode::Docked {
                return;
            }
            let target_speed = if before.position[2] > 2500.0 {
                34.3
            } else {
                4.0
            };
            world
                .step(FlightInput {
                    throttle: ((target_speed - before.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                    ..FlightInput::default()
                })
                .unwrap();
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        panic!("public controls did not admit actual station entry");
    }
    fn launch(world: &mut World) {
        world.request_launch(world.snapshot().tick).unwrap();
        world.step(FlightInput::default()).unwrap();
    }
    fn accept(world: &mut World) -> (JobId, [GoodsId; 8]) {
        let npc = world.station_contacts()[0];
        world.request_contact(npc, world.snapshot().tick).unwrap();
        let offer = g02::Command::ReviewOffer { actor: npc.actor() };
        let pending = review(world, offer);
        // An actual ordinary purchase changes the same account and revokes this old review.
        action(world, Command::BuyFood).unwrap();
        let before = fingerprint(world);
        assert_eq!(
            freight(
                world,
                g02::Command::Confirm {
                    review: pending,
                    choice: Choice::Accept
                }
            ),
            Err(MissionError::Freight(g02::Error::StaleReview))
        );
        assert_eq!(fingerprint(world), before);
        let pending = review(world, offer);
        match freight(
            world,
            g02::Command::Confirm {
                review: pending,
                choice: Choice::Accept,
            },
        )
        .unwrap()
        {
            g02::Event::Accepted { job, units } => (job, units),
            event => panic!("expected original allocation, got {event:?}"),
        }
    }

    #[test]
    fn public_buy_sell_paid_refill_and_return_preserve_eight_original_units_and_station_stock() {
        let mut world = World::new_commander(301).unwrap();
        assert_eq!(world.service_snapshot().unwrap().food_tonnes, 0);
        let initial = world.current_dock_context().unwrap();
        let (job, units) = accept(&mut world);
        assert_eq!(world.credits_tenths(), Some(6974));
        assert_eq!(
            (
                world.service_snapshot().unwrap().food_tonnes,
                world.service_snapshot().unwrap().used_tonnes
            ),
            (1, 9)
        );
        assert_eq!(
            world
                .service_snapshot()
                .unwrap()
                .current_market
                .unwrap()
                .stock,
            127
        );
        assert_eq!(world.mission_snapshot().unwrap().freight.held_tonnes, 8);
        assert!(
            units
                .iter()
                .enumerate()
                .all(|(i, unit)| !units[..i].contains(unit))
        );
        launch(&mut world);
        let airborne = fingerprint(&world);
        assert_eq!(
            action(&mut world, Command::SellFood),
            Err(MissionError::NotDocked)
        );
        assert_eq!(fingerprint(&world), airborne);
        jump(&mut world, SystemId::RENAS);
        dock(&mut world);
        assert!(!world.dock_context_is_current(&initial));
        assert_eq!(
            world.current_dock_context().unwrap().system(),
            SystemId::RENAS
        );
        assert_eq!(
            world
                .service_snapshot()
                .unwrap()
                .current_market
                .unwrap()
                .stock,
            112
        );
        assert_eq!(
            action(&mut world, Command::SellFood).unwrap(),
            Event::SoldFood {
                site: MarketSite::Renas,
                bid_tenths: 196,
                credited_tenths: 196
            }
        );
        assert_eq!(world.credits_tenths(), Some(7170));
        assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 96);
        assert_eq!(world.request_refuel(world.snapshot().tick), Ok(68));
        assert_eq!(world.credits_tenths(), Some(7102));
        assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 130);
        assert_eq!(world.request_refuel(world.snapshot().tick), Ok(0));
        assert_eq!(world.credits_tenths(), Some(7102));
        action(&mut world, Command::BuyFood).unwrap();
        action(&mut world, Command::SellFood).unwrap();
        assert_eq!(world.credits_tenths(), Some(7082));
        assert_eq!(
            world
                .service_snapshot()
                .unwrap()
                .current_market
                .unwrap()
                .stock,
            113
        );
        assert_eq!(world.service_snapshot().unwrap().used_tonnes, 8);
        refuses(&mut world, Command::SellFood, Error::NoSellableCargo);
        let crew = world.station_contacts()[0];
        world.request_contact(crew, world.snapshot().tick).unwrap();
        for (index, unit) in units.iter().enumerate() {
            let pending = review(
                &mut world,
                g02::Command::ReviewReceiving {
                    actor: crew.actor(),
                    job,
                },
            );
            assert_eq!(pending.next_unit(), Some(*unit));
            let event = freight(
                &mut world,
                g02::Command::Confirm {
                    review: pending,
                    choice: Choice::ReceiveOne(*unit),
                },
            )
            .unwrap();
            let g02::Event::Received {
                units: received,
                completion,
                ..
            } = event
            else {
                panic!("expected transfer");
            };
            assert_eq!(
                received.iter().flatten().copied().collect::<Vec<_>>(),
                vec![*unit]
            );
            assert_eq!(completion.is_some(), index == 7);
            assert_eq!(
                world.credits_tenths(),
                Some(if index == 7 { 14382 } else { 7082 })
            );
            assert_eq!(
                world.service_snapshot().unwrap().used_tonnes,
                7 - index as u16
            );
        }
        assert!(world.mission_snapshot().unwrap().freight.clue.is_some());
        launch(&mut world);
        jump(&mut world, SystemId::AVENOR);
        dock(&mut world);
        assert_eq!(
            world
                .service_snapshot()
                .unwrap()
                .current_market
                .unwrap()
                .stock,
            127
        );
        assert_eq!(world.service_snapshot().unwrap().food_tonnes, 0);
        assert_eq!(world.mission_snapshot().unwrap().freight.completed_jobs, 1);
    }

    #[test]
    fn public_spending_and_unaffordable_refill_refuse_with_all_account_stock_and_cargo_unchanged() {
        let mut world = World::new_commander(401).unwrap();
        refuses(&mut world, Command::SellFood, Error::NoSellableCargo);
        for _ in 0..31 {
            action(&mut world, Command::BuyFood).unwrap();
        }
        assert_eq!(world.credits_tenths(), Some(194));
        assert_eq!(world.service_snapshot().unwrap().food_tonnes, 31);
        assert_eq!(
            world
                .service_snapshot()
                .unwrap()
                .current_market
                .unwrap()
                .stock,
            97
        );
        refuses(&mut world, Command::BuyFood, Error::InsufficientFunds);
        launch(&mut world);
        for system in [SystemId::RENAS, SystemId::AVENOR, SystemId::RENAS] {
            jump(&mut world, system);
        }
        dock(&mut world);
        assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 28);
        refuses(&mut world, Command::RefillFuel, Error::InsufficientFunds);
        assert_eq!(world.service_snapshot().unwrap().food_tonnes, 31);
        assert_eq!(
            world
                .service_snapshot()
                .unwrap()
                .current_market
                .unwrap()
                .stock,
            112
        );
        assert_eq!(world.credits_tenths(), Some(194));
    }

    #[test]
    fn public_stale_paused_prototype_and_descriptive_values_cannot_authorize_services() {
        let mut world = World::new_commander(601).unwrap();
        let mut displayed = world.service_snapshot().unwrap();
        displayed.food_tonnes = 92;
        displayed.current_market.as_mut().unwrap().ask_tenths = 0;
        assert_eq!(displayed.food_tonnes, 92);
        assert_eq!(world.service_snapshot().unwrap().food_tonnes, 0);
        assert_eq!(
            world
                .service_snapshot()
                .unwrap()
                .current_market
                .unwrap()
                .ask_tenths,
            226
        );
        let mut presentation = world.snapshot();
        presentation.navigation.as_mut().unwrap().system = SystemId::RENAS;
        assert_eq!(presentation.navigation.unwrap().system, SystemId::RENAS);
        assert_eq!(
            world.current_dock_context().unwrap().system(),
            SystemId::AVENOR
        );
        world.step(FlightInput::default()).unwrap();
        let before = fingerprint(&world);
        assert_eq!(
            world.request_station_service(Command::BuyFood, 0),
            Err(MissionError::StaleCommand)
        );
        assert_eq!(fingerprint(&world), before);
        world.request_pause(true, world.snapshot().tick).unwrap();
        let before = fingerprint(&world);
        assert_eq!(
            action(&mut world, Command::BuyFood),
            Err(MissionError::Paused)
        );
        assert_eq!(
            world.request_refuel(world.snapshot().tick),
            Err(MissionError::Paused)
        );
        assert_eq!(fingerprint(&world), before);
        for mut prototype in [World::new(), World::at_original_station().unwrap()] {
            assert_eq!(prototype.service_snapshot(), None);
            let before = fingerprint(&prototype);
            assert_eq!(
                action(&mut prototype, Command::BuyFood),
                Err(MissionError::NoCommander)
            );
            assert_eq!(fingerprint(&prototype), before);
        }
        world.request_pause(false, world.snapshot().tick).unwrap();
        launch(&mut world);
        jump(&mut world, SystemId::RENAS);
        jump(
            &mut world,
            SystemId {
                galaxy: 0,
                system: 0,
            },
        );
        dock(&mut world);
        assert_eq!(world.current_dock_context().unwrap().system().system, 0);
        refuses(&mut world, Command::BuyFood, Error::UnsupportedStation);
    }
    #[test]
    fn ordinary_machinery_never_sells_or_replaces_the_eight_identified_freight_units() {
        let mut world = World::new_commander(901).unwrap();
        let (job, units) = accept(&mut world);
        let held = world.mission_snapshot().unwrap().freight;
        assert_eq!(world.commodity_quantity(CommodityId::Machinery), Some(0));
        refuses(
            &mut world,
            Command::SellCommodity(CommodityId::Machinery),
            Error::NoSellableCargo,
        );
        action(&mut world, Command::BuyCommodity(CommodityId::Machinery)).unwrap();
        action(&mut world, Command::SellCommodity(CommodityId::Machinery)).unwrap();
        assert_eq!(world.mission_snapshot().unwrap().freight, held);
        action(&mut world, Command::BuyCommodity(CommodityId::Machinery)).unwrap();
        assert_eq!(world.service_snapshot().unwrap().used_tonnes, 10); // Food + actual ordinary unit + eight mission units.
        launch(&mut world);
        jump(&mut world, SystemId::RENAS);
        dock(&mut world);
        let npc = world.station_contacts()[0];
        world.request_contact(npc, world.snapshot().tick).unwrap();
        for unit in units {
            let pending = review(
                &mut world,
                g02::Command::ReviewReceiving {
                    actor: npc.actor(),
                    job,
                },
            );
            assert_eq!(pending.next_unit(), Some(unit));
            freight(
                &mut world,
                g02::Command::Confirm {
                    review: pending,
                    choice: Choice::ReceiveOne(unit),
                },
            )
            .unwrap();
        }
        assert_eq!(world.mission_snapshot().unwrap().freight.held_tonnes, 0);
        assert_eq!(world.commodity_quantity(CommodityId::Machinery), Some(1));
        assert_eq!(world.service_snapshot().unwrap().used_tonnes, 2);
        action(&mut world, Command::SellCommodity(CommodityId::Machinery)).unwrap();
        assert_eq!(world.commodity_quantity(CommodityId::Machinery), Some(0));
        refuses(
            &mut world,
            Command::SellCommodity(CommodityId::Machinery),
            Error::NoSellableCargo,
        );
    }

    #[test]
    fn legal_record_changes_only_on_successful_real_launch_and_survives_sale_and_continue() {
        let mut world = World::new_commander(1001).unwrap();
        action(&mut world, Command::BuyCommodity(CommodityId::Slaves)).unwrap();
        assert_eq!(world.legal_value(), Some(0));
        world.step(FlightInput::default()).unwrap();
        assert_eq!(world.request_launch(0), Err(crate::StepError::StaleCommand));
        assert_eq!(world.legal_value(), Some(0));
        world.request_pause(true, world.snapshot().tick).unwrap();
        assert_eq!(
            world.request_launch(world.snapshot().tick),
            Err(crate::StepError::Paused)
        );
        assert_eq!(world.legal_value(), Some(0));
        world.request_pause(false, world.snapshot().tick).unwrap();
        launch(&mut world);
        assert_eq!(world.legal_value(), Some(1));
        let mut world = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
        assert_eq!(world.legal_value(), Some(1));
        jump(&mut world, SystemId::RENAS);
        dock(&mut world);
        action(&mut world, Command::SellCommodity(CommodityId::Slaves)).unwrap();
        assert_eq!(world.legal_value(), Some(1));
        action(&mut world, Command::BuyCommodity(CommodityId::Narcotics)).unwrap();
        launch(&mut world);
        assert_eq!(world.legal_value(), Some(3)); // 1 OR (2 * one Narcotics); not a trade ban or fine.
        let continued = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
        assert_eq!(continued.legal_value(), Some(3));
        let mut sold_before_departure = World::new_commander(1101).unwrap();
        action(
            &mut sold_before_departure,
            Command::BuyCommodity(CommodityId::Weapons),
        )
        .unwrap();
        action(
            &mut sold_before_departure,
            Command::SellCommodity(CommodityId::Weapons),
        )
        .unwrap();
        launch(&mut sold_before_departure);
        assert_eq!(sold_before_departure.legal_value(), Some(0));
    }
    #[test]
    fn every_typed_commodity_uses_actual_stale_paused_and_airborne_admission() {
        let mut world = World::new_commander(1401).unwrap();
        world.step(FlightInput::default()).unwrap();
        let before = world.save_bytes().unwrap();
        for id in CommodityId::ALL {
            assert_eq!(
                world.request_station_service(Command::BuyCommodity(id), 0),
                Err(MissionError::StaleCommand)
            );
            assert_eq!(world.save_bytes().unwrap(), before);
        }
        world.request_pause(true, world.snapshot().tick).unwrap();
        let before = world.save_bytes().unwrap();
        for id in CommodityId::ALL {
            assert_eq!(
                action(&mut world, Command::BuyCommodity(id)),
                Err(MissionError::Paused)
            );
            assert_eq!(world.save_bytes().unwrap(), before);
        }
        world.request_pause(false, world.snapshot().tick).unwrap();
        launch(&mut world);
        let before = world.save_bytes().unwrap();
        for id in CommodityId::ALL {
            assert_eq!(
                action(&mut world, Command::SellCommodity(id)),
                Err(MissionError::NotDocked)
            );
            assert_eq!(world.save_bytes().unwrap(), before);
        }
    }
}

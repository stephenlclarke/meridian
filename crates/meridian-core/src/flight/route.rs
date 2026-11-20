//! World-owned route review, publication, and progress on real jumps and docks.
use super::{FlightMode, World};
use crate::navigation::{
    FUEL_CAPACITY, NavigationGrade, ROUTE_SYSTEMS, RouteAlternative, RouteEconomy, RouteError,
    RoutePlan, RouteProgress, RouteStatus, SystemId, route_alternatives, route_feasible, route_hop,
};
use crate::save::SaveError;
use crate::station_services::{CargoUnit, CommodityId};

/// Publish these candidate bytes before applying the route to the live World.
pub struct PreparedNavigationRoute<'a> {
    world: &'a mut World,
    route: Option<RoutePlan>,
    bytes: Vec<u8>,
}
impl PreparedNavigationRoute<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn commit(self) {
        self.world.navigation_route = self.route;
    }
}

impl World {
    pub fn navigation_computer_grade(&self) -> NavigationGrade {
        let Some((account, navigation)) = self.commander_account.as_ref().zip(self.navigation)
        else {
            return NavigationGrade::Empty;
        };
        if !Self::has_bound_drive(navigation, account) {
            return NavigationGrade::Empty;
        }
        self.modular
            .ledger(self, account)
            .map_or(NavigationGrade::Empty, |ledger| ledger.navigation_grade())
    }

    pub fn route_alternatives(
        &self,
        destination: SystemId,
    ) -> Result<Vec<RouteAlternative>, RouteError> {
        let catalogue = self
            .navigation_catalogue
            .as_ref()
            .ok_or(RouteError::ComputerRequired)?;
        let nav = self.navigation.ok_or(RouteError::ComputerRequired)?;
        let account = self
            .commander_account
            .as_ref()
            .ok_or(RouteError::ComputerRequired)?;
        let grade = self.navigation_computer_grade();
        if grade == NavigationGrade::Empty {
            return Err(RouteError::ComputerRequired);
        }
        let current = nav.snapshot(account.fuel_tenths());
        let ship = nav.ship();
        let template = RoutePlan {
            owner: ship.owner().seed(),
            hull: ship.ordinal(),
            origin_visit: current.visit,
            origin_jumps: nav.jump_sequence(),
            grade,
            origin_fuel: current.fuel_units,
            stop: 0,
            systems: Vec::new(),
        };
        route_alternatives(
            catalogue,
            &template,
            current.system,
            destination,
            &self.route_economy(),
        )
    }

    fn route_economy(&self) -> RouteEconomy {
        let mut economy = RouteEconomy::default();
        let (Some(markets), Ok(cargo)) =
            (&self.station_markets, self.reserved_service_projection())
        else {
            return economy;
        };
        let open_room = u64::from(cargo.hold_capacity_tonnes.saturating_sub(cargo.used_tonnes));
        for good in CommodityId::ALL {
            let sellable = if good == CommodityId::Food {
                u64::from(cargo.food_tonnes.saturating_sub(cargo.reserved_food_tonnes))
            } else {
                let i = good.index() - 1;
                cargo.other_commodities[i].saturating_sub(cargo.reserved_commodities[i])
            };
            let room = if good.unit() == CargoUnit::Tonne {
                open_room
            } else {
                63
            };
            let quotes = ROUTE_SYSTEMS.map(|system| {
                super::services::service_site(system)
                    .map(|site| markets.commodity_quote(site, good))
            });
            for (to, destination) in quotes.iter().enumerate() {
                let Some(destination) = destination else {
                    continue;
                };
                economy.sale[to] = economy.sale[to]
                    .saturating_add(sellable.saturating_mul(u64::from(destination.bid_tenths)));
                for (from, source) in quotes.iter().enumerate() {
                    let Some(source) = source else { continue };
                    let profit =
                        u64::from(destination.bid_tenths.saturating_sub(source.ask_tenths));
                    let potential = profit * source.stock.min(room).min(63);
                    economy.trade[from][to] = economy.trade[from][to].max(potential);
                }
            }
        }
        economy
    }

    pub fn active_route_progress(&self) -> Option<RouteProgress> {
        let plan = self.navigation_route.as_ref()?;
        let catalogue = self.navigation_catalogue.as_ref()?;
        let nav = self
            .navigation?
            .snapshot(self.commander_account.as_ref()?.fuel_tenths());
        let mut result = RouteProgress {
            plan: plan.clone(),
            completed_legs: plan.stop,
            next_system: None,
            next_fuel_units: None,
            status: RouteStatus::Complete,
        };
        let Some(next) = plan.next_system() else {
            return Some(result);
        };
        if nav.system != plan.current_system() {
            result.status = RouteStatus::AwaitingDock;
            return Some(result);
        }
        if !route_feasible(
            catalogue,
            &plan.systems[usize::from(plan.stop)..],
            FUEL_CAPACITY,
            self.navigation_computer_grade(),
        ) {
            result.status = RouteStatus::ComputerUnavailable;
            return Some(result);
        }
        let (distance, debit) = route_hop(catalogue, nav.system, next).ok()?;
        let required_here = (distance * 10.0).ceil() as u16;
        result.next_fuel_units = Some(required_here.max(debit));
        result.status = if distance <= f64::from(nav.fuel_units) / 10.0 && debit <= nav.fuel_units {
            result.next_system = Some(next);
            RouteStatus::Ready
        } else {
            RouteStatus::RefuelRequired
        };
        Some(result)
    }

    pub fn set_navigation_route(
        &mut self,
        plan: &RoutePlan,
        expected_tick: u64,
    ) -> Result<(), RouteError> {
        self.prepare_navigation_route(Some(plan), expected_tick)?
            .commit();
        Ok(())
    }

    pub fn clear_navigation_route(&mut self, expected_tick: u64) -> Result<(), RouteError> {
        self.prepare_navigation_route(None, expected_tick)?.commit();
        Ok(())
    }

    pub fn prepare_navigation_route(
        &mut self,
        plan: Option<&RoutePlan>,
        expected_tick: u64,
    ) -> Result<PreparedNavigationRoute<'_>, RouteError> {
        if expected_tick != self.state.tick {
            return Err(RouteError::StalePlan);
        }
        if self.is_paused() {
            return Err(RouteError::Paused);
        }
        let dock = self
            .current_dock_context()
            .ok_or(RouteError::DockRequired)?;
        if let Some(plan) = plan
            && (!plan.agrees_with_ship(dock.ship())
                || plan.stop != 0
                || plan.origin_visit != dock.visit()
                || self
                    .navigation
                    .is_none_or(|nav| plan.origin_jumps != nav.jump_sequence())
                || !self
                    .route_alternatives(plan.destination())?
                    .iter()
                    .any(|candidate| &candidate.plan == plan))
        {
            return Err(RouteError::StalePlan);
        }
        let route = plan.cloned();
        let prior = std::mem::replace(&mut self.navigation_route, route.clone());
        let encoded = self.save_bytes();
        self.navigation_route = prior;
        Ok(PreparedNavigationRoute {
            world: self,
            route,
            bytes: encoded.map_err(RouteError::Save)?,
        })
    }

    pub(super) fn cancel_route_on_manual_jump(&mut self, origin: SystemId, target: SystemId) {
        if self.navigation_route.as_ref().is_some_and(|plan| {
            plan.current_system() != origin || plan.next_system() != Some(target)
        }) {
            self.navigation_route = None;
        }
    }

    pub(super) fn reconcile_navigation_route(&mut self, physical_entry: bool) {
        let Some(plan) = self.navigation_route.as_mut() else {
            return;
        };
        let Some(nav) = self.navigation else { return };
        let current = nav.snapshot(0).system;
        if plan.current_system() == current {
            return;
        }
        if plan.next_system() == Some(current) {
            if physical_entry {
                plan.stop += 1;
            }
        } else {
            self.navigation_route = None;
        }
    }

    pub(super) fn validate_saved_navigation_route(&self) -> Result<(), SaveError> {
        const INVALID: SaveError = SaveError::InvalidState("navigation computer plan");
        let Some(plan) = &self.navigation_route else {
            return Ok(());
        };
        let nav = self.navigation.ok_or(INVALID)?;
        let catalogue = self.navigation_catalogue.as_ref().ok_or(INVALID)?;
        let current = nav.snapshot(0);
        if !plan.agrees_with_ship(nav.ship())
            || !route_feasible(catalogue, plan.systems(), plan.origin_fuel, plan.grade)
            || plan.origin_visit > current.visit
        {
            return Err(INVALID);
        }
        let awaiting_dock = current.system != plan.current_system();
        if awaiting_dock
            && (self.state.mode != FlightMode::Flight
                || plan.next_system() != Some(current.system)
                || current.travel.is_some())
        {
            return Err(INVALID);
        }
        if current
            .travel
            .is_some_and(|travel| Some(travel.destination) != plan.next_system())
        {
            return Err(INVALID);
        }
        let completed = usize::from(plan.stop) + usize::from(awaiting_dock);
        if !self
            .jump_journal
            .matches_route_prefix(plan.origin_jumps, &plan.systems()[..=completed])
        {
            return Err(INVALID);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

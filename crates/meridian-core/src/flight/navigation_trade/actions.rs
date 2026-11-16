use super::{
    Attempt, Fit, MAX_ACTIVE, MAX_RECORDS, NavigationTradeAction as Action,
    NavigationTradeAttemptSnapshot, NavigationTradeError as Error,
    NavigationTradeOutcome as Outcome, NavigationTradeReview, NavigationTradeSnapshot, Plan,
    PreparedNavigationTrade, Terminal, World, legal_tonne, quantity, same_carrier, site,
};
use crate::navigation::{DockContext, STOCK_JUMP_REACH_LY, SystemId, stock_jump_fuel_cost};
use crate::station_services::CommodityId;

impl World {
    fn navigation_trade_berth(&self, tick: u64) -> Result<DockContext, Error> {
        if tick != self.state.tick {
            return Err(Error::StaleReview);
        }
        if self.commander_account.is_none() {
            return Err(Error::NoCommander);
        }
        if self.is_paused() {
            return Err(Error::Paused);
        }
        let dock = self
            .current_dock_context()
            .filter(|d| site(d.system()).is_some())
            .ok_or(Error::BerthRequired)?;
        if self
            .station_data
            .as_ref()
            .is_none_or(|s| self.state.station != Some(s.station_pose()))
            || self.combat_snapshot().is_none_or(|s| s.player.destroyed)
        {
            return Err(Error::BerthRequired);
        }
        Ok(dock)
    }
    pub(super) fn navigation_trade_fit(&self) -> Option<Fit> {
        let (inventory, items, equipment_revision) =
            self.commander_account.as_ref()?.owned_fit_identity()?;
        Some(Fit {
            inventory,
            items,
            equipment_revision,
        })
    }
    pub fn review_navigation_trade(
        &self,
        action: Action,
        tick: u64,
    ) -> Result<NavigationTradeReview, Error> {
        let dock = self.navigation_trade_berth(tick)?;
        let plan = match action {
            Action::Accept => {
                let (records, active) = self.original_service_occupancy();
                if self.navigation_trade.active().is_some() {
                    return Err(Error::AlreadyActive);
                }
                if records >= MAX_RECORDS || active >= MAX_ACTIVE {
                    return Err(Error::Capacity);
                }
                None
            }
            Action::Plan {
                destination,
                commodity,
            } => {
                let a = self
                    .navigation_trade
                    .active()
                    .ok_or(Error::NoActiveAttempt)?;
                if a.plan.is_some() {
                    return Err(Error::AlreadyPlanned);
                }
                if !same_carrier(a.dock, dock) || dock.system() != a.dock.system() {
                    return Err(Error::BerthRequired);
                }
                Some(self.navigation_trade_plan(destination, commodity, tick)?)
            }
            Action::Receive | Action::Cancel => {
                let a = self
                    .navigation_trade
                    .active()
                    .ok_or(Error::NoActiveAttempt)?;
                if !same_carrier(a.dock, dock) {
                    return Err(Error::BerthRequired);
                }
                if action == Action::Receive && !a.ready(dock) {
                    return Err(Error::NotReady);
                }
                None
            }
        };
        Ok(NavigationTradeReview {
            action,
            dock,
            tick,
            epoch: self
                .freight_session
                .as_ref()
                .ok_or(Error::NoCommander)?
                .epoch(),
            fingerprint: self
                .flight_reception_material_fingerprint()
                .map_err(Error::Save)?,
            plan,
        })
    }
    fn navigation_trade_plan(
        &self,
        destination: SystemId,
        commodity: CommodityId,
        tick: u64,
    ) -> Result<Plan, Error> {
        let dock = self.current_dock_context().ok_or(Error::BerthRequired)?;
        if destination == dock.system() || site(destination).is_none() {
            return Err(Error::UnsupportedRoute);
        }
        if !legal_tonne(commodity) {
            return Err(Error::CommodityRequired);
        }
        let account = self.commander_account.as_ref().ok_or(Error::NoCommander)?;
        let p = account
            .service_projection(
                &self
                    .freight_session
                    .as_ref()
                    .ok_or(Error::NoCommander)?
                    .state,
            )
            .map_err(Error::Service)?;
        if quantity(p, commodity) != 0 {
            return Err(Error::CargoPresent);
        }
        if p.used_tonnes >= p.hold_capacity_tonnes {
            return Err(Error::Capacity);
        }
        let quote = self
            .commodity_quote(commodity)
            .ok_or(Error::BerthRequired)?;
        if quote.stock == 0 {
            return Err(Error::Stock);
        }
        if p.credits_tenths < quote.ask_tenths {
            return Err(Error::Credits);
        }
        let distance = self
            .navigation_catalogue
            .as_ref()
            .ok_or(Error::NoCommander)?
            .distance_ly(dock.system(), destination)
            .map_err(|_| Error::UnsupportedRoute)?;
        let fuel = stock_jump_fuel_cost(distance).map_err(|_| Error::UnsupportedRoute)?;
        if distance <= 0.0 || distance > STOCK_JUMP_REACH_LY {
            return Err(Error::UnsupportedRoute);
        }
        if distance > f64::from(p.fuel_tenths) / 10.0 || fuel > p.fuel_tenths {
            return Err(Error::Fuel);
        }
        let markets = self.station_markets.as_ref().ok_or(Error::NoCommander)?;
        let (bought, sold) =
            markets.commodity_counts(site(dock.system()).ok_or(Error::BerthRequired)?, commodity);
        Ok(Plan {
            destination,
            commodity,
            confirmed_tick: tick,
            fit: self.navigation_trade_fit().ok_or(Error::NoCommander)?,
            capacity: p.hold_capacity_tonnes,
            cargo: p.used_tonnes,
            price: quote.ask_tenths,
            stock: quote.stock,
            distance,
            fuel_debit: fuel,
            fuel_at_plan: p.fuel_tenths,
            credits_at_plan: p.credits_tenths,
            account_revision: p.revision,
            market_revision: markets.transaction_revision(),
            bought,
            sold,
        })
    }
    pub fn prepare_navigation_trade(
        &mut self,
        review: NavigationTradeReview,
        tick: u64,
    ) -> Result<PreparedNavigationTrade<'_>, Error> {
        let current = self.review_navigation_trade(review.action, tick)?;
        if review.dock != current.dock
            || review.epoch != current.epoch
            || review.tick > current.tick
            || review.fingerprint != current.fingerprint
            || review.plan.map(|mut p| {
                p.confirmed_tick = current.tick;
                p
            }) != current.plan
        {
            return Err(Error::StaleReview);
        }
        let mut candidate = self.navigation_trade.clone();
        match review.action {
            Action::Accept => candidate.attempts.push(Attempt {
                dock: current.dock,
                accepted_tick: current.tick,
                plan: None,
                buy: None,
                journeys: Vec::new(),
                fuel: Vec::new(),
                sale: None,
                terminal: None,
            }),
            Action::Plan { .. } => {
                let last = candidate
                    .attempts
                    .last_mut()
                    .ok_or(Error::NoActiveAttempt)?;
                last.plan = current.plan;
            }
            Action::Cancel | Action::Receive => {
                let i = candidate.attempts.len() - 1;
                let outcome = if review.action == Action::Receive {
                    Outcome::Complete
                } else {
                    Outcome::Abandoned
                };
                candidate.attempts[i].terminal = Some(Terminal {
                    dock: current.dock,
                    tick: current.tick,
                    outcome,
                });
                if outcome == Outcome::Complete && candidate.trace.is_none() {
                    candidate.trace = Some(i as u8);
                }
            }
        }
        let bytes = self
            .save_bytes_with_navigation_trade(
                &candidate,
                self.commander_account.as_ref().ok_or(Error::NoCommander)?,
                self.station_markets.as_ref().ok_or(Error::NoCommander)?,
            )
            .map_err(Error::Save)?;
        Ok(PreparedNavigationTrade {
            world: self,
            candidate,
            bytes,
        })
    }
    pub fn navigation_trade_snapshot(&self) -> Option<NavigationTradeSnapshot> {
        self.commander_account.as_ref()?;
        let state = &self.navigation_trade;
        let (records, active) = self.original_service_occupancy();
        Some(NavigationTradeSnapshot {
            attempts: state
                .attempts
                .iter()
                .enumerate()
                .map(|(i, a)| NavigationTradeAttemptSnapshot {
                    id: state.id(i),
                    issuing_dock: a.dock,
                    accepted_tick: a.accepted_tick,
                    plan: a.plan.map(Plan::snapshot),
                    buy: a.buy.map(super::Trade::snapshot),
                    held_tonne: a.buy.filter(|_| a.held()).map(|b| b.tonne),
                    sale: a.sale.map(super::Trade::snapshot),
                    journeys: a.journeys.len(),
                    fuel_units_used: a.fuel_used(),
                    fuel_paid_tenths: a.fuel_paid(),
                    cash_difference_tenths: a.sale.zip(a.buy).map(|(s, b)| {
                        i64::from(s.price) - i64::from(b.price) - i64::from(a.fuel_paid())
                    }),
                    outcome: a.terminal.map(|t| t.outcome),
                    terminal_tick: a.terminal.map(|t| t.tick),
                })
                .collect(),
            active: state.active().map(|_| state.id(state.attempts.len() - 1)),
            reserved_capacity_tonnes: state.reservations().capacity,
            ready: state
                .active()
                .zip(self.current_dock_context())
                .is_some_and(|(a, d)| a.ready(d)),
            trace: state.trace_snapshot(),
            active_services: active,
            retained_mission_records: records,
        })
    }
}

//! Informational original trade scoring over three genuine providers. No transaction authority.
use super::{NavigationRouteError, World};
use crate::{
    navigation::{FUEL_CAPACITY, RouteKind, SystemId},
    station_services::{self, CargoUnit, CommodityId, CommodityQuote},
};
use core::fmt;
const SITES: [SystemId; 3] = [SystemId::AVENOR, SystemId::LARERIA, SystemId::RENAS];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradePlanningError {
    Navigation(NavigationRouteError),
    MarketUnavailable,
    Arithmetic,
}
impl fmt::Display for TradePlanningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Navigation(error) => fmt::Display::fmt(error, f),
            Self::MarketUnavailable => f.write_str("Admitted market or ordinary cargo unavailable"),
            Self::Arithmetic => f.write_str("Trade estimate exceeds exact arithmetic bounds"),
        }
    }
}
impl std::error::Error for TradePlanningError {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeExportEstimate {
    pub commodity: CommodityId,
    /// Native units: tonnes, kilograms or grams, never converted to tonnes.
    pub quantity: u64,
    pub ask_tenths: u32,
    pub benchmark_bid_tenths: u32,
    pub gross_tenths: i128,
}
#[derive(Debug, Clone, PartialEq)]
pub struct TradeProfitEstimate {
    pub destination: SystemId,
    pub distance_ly: f64,
    /// Compared with selling sellable carried cargo here, plus cash/hold-limited exports.
    pub estimated_delta_tenths: i128,
    pub carried_delta_tenths: i128,
    pub exports: Vec<TradeExportEstimate>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeLegEstimate {
    pub source: SystemId,
    pub destination: SystemId,
    pub opportunity: Option<TradeExportEstimate>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct TradePathEstimate {
    pub systems: Vec<SystemId>,
    pub legs: Vec<TradeLegEstimate>,
    pub labels: Vec<&'static str>,
    pub distance_ly: f64,
    pub arrival_exposure: f64,
    /// Sum of independent best-commodity opportunities; NOT a funded multi-leg itinerary.
    pub independent_leg_gross_tenths: u128,
    pub best_carried_sale_tenths: u128,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationTradePreview {
    pub as_of_tick: u64,
    pub origin: SystemId,
    pub destination: SystemId,
    pub paths: Vec<TradePathEstimate>,
    /// Original positive direct-reach ranking, exact signed arithmetic and ID ties, at most five.
    pub direct_profits: Vec<TradeProfitEstimate>,
    /// Actual current World-owned quotes at only the three admitted providers, sorted by system ID.
    pub quotes: [[CommodityQuote; 17]; 3],
    /// Original immutable initial bids. They are benchmarks, not live/future sale promises.
    pub benchmark_bids: [[u32; 17]; 3],
    pub ordinary_quantities: [u64; 17],
    pub reserved_quantities: [u64; 17],
    pub cash_tenths: u32,
    pub used_tonnes: u16,
    pub capacity_tonnes: u16,
    pub fuel_tenths: u16,
    pub fitted_reach_ly: f64,
}
impl NavigationTradePreview {
    pub fn assumptions(&self) -> &'static str {
        "As-of quotes and original initial bid benchmarks; gross estimates exclude fuel, fees and credit saturation. Later stock, cash, refuelling, travel and sale are not guaranteed. Reopen after changes."
    }
}
fn index(id: SystemId) -> Result<usize, TradePlanningError> {
    SITES
        .iter()
        .position(|site| *site == id)
        .ok_or(TradePlanningError::MarketUnavailable)
}
fn sum(a: i128, b: i128) -> Result<i128, TradePlanningError> {
    a.checked_add(b).ok_or(TradePlanningError::Arithmetic)
}
impl NavigationTradePreview {
    fn sellable(&self, id: usize) -> Result<u64, TradePlanningError> {
        self.ordinary_quantities[id]
            .checked_sub(self.reserved_quantities[id])
            .ok_or(TradePlanningError::MarketUnavailable)
    }
    fn cargo_sale(&self, destination: usize) -> Result<u128, TradePlanningError> {
        let mut total = 0_u128;
        for commodity in CommodityId::ALL {
            let id = commodity.index();
            let value =
                u128::from(self.sellable(id)?) * u128::from(self.benchmark_bids[destination][id]);
            total = total
                .checked_add(value)
                .ok_or(TradePlanningError::Arithmetic)?;
        }
        Ok(total)
    }
    fn leg_detail(
        &self,
        source: usize,
        destination: usize,
    ) -> Result<TradeLegEstimate, TradePlanningError> {
        let mut sellable_tonnes = 0_u64;
        for id in CommodityId::ALL {
            if id.unit() == CargoUnit::Tonne {
                sellable_tonnes = sellable_tonnes
                    .checked_add(self.sellable(id.index())?)
                    .ok_or(TradePlanningError::Arithmetic)?;
            }
        }
        let reserved_tonnes = u64::from(self.used_tonnes)
            .checked_sub(sellable_tonnes)
            .ok_or(TradePlanningError::MarketUnavailable)?;
        let room = u64::from(self.capacity_tonnes)
            .checked_sub(reserved_tonnes)
            .ok_or(TradePlanningError::MarketUnavailable)?;
        let mut best: Option<TradeExportEstimate> = None;
        for id in CommodityId::ALL {
            let quote = self.quotes[source][id.index()];
            let gain =
                self.benchmark_bids[destination][id.index()].saturating_sub(quote.ask_tenths);
            let quantity = quote.stock.min(63);
            let load = if id.unit() == CargoUnit::Tonne {
                quantity.min(room)
            } else {
                quantity
            };
            let gross = i128::from(gain) * i128::from(load);
            if gross > 0 && best.as_ref().is_none_or(|old| gross > old.gross_tenths) {
                best = Some(TradeExportEstimate {
                    commodity: id,
                    quantity: load,
                    ask_tenths: quote.ask_tenths,
                    benchmark_bid_tenths: self.benchmark_bids[destination][id.index()],
                    gross_tenths: gross,
                });
            }
        }
        Ok(TradeLegEstimate {
            source: SITES[source],
            destination: SITES[destination],
            opportunity: best,
        })
    }
    fn leg_opportunity(
        &self,
        source: usize,
        destination: usize,
    ) -> Result<u128, TradePlanningError> {
        let detail = self.leg_detail(source, destination)?;
        u128::try_from(detail.opportunity.map_or(0, |value| value.gross_tenths))
            .map_err(|_| TradePlanningError::Arithmetic)
    }
    fn direct_profit(
        &self,
        target: usize,
        distance_ly: f64,
    ) -> Result<TradeProfitEstimate, TradePlanningError> {
        let source = index(self.origin)?;
        let mut carried = 0;
        for commodity in CommodityId::ALL {
            let id = commodity.index();
            let gain = i128::from(self.benchmark_bids[target][id])
                - i128::from(self.quotes[source][id].bid_tenths);
            carried = sum(
                carried,
                i128::from(self.sellable(id)?)
                    .checked_mul(gain)
                    .ok_or(TradePlanningError::Arithmetic)?,
            )?;
        }
        let mut exports: Vec<_> = CommodityId::ALL
            .into_iter()
            .filter(|id| {
                self.benchmark_bids[target][id.index()] > self.quotes[source][id.index()].ask_tenths
                    && self.quotes[source][id.index()].stock > 0
            })
            .collect();
        exports.sort_by(|a, b| {
            let gain = |id: CommodityId| {
                self.benchmark_bids[target][id.index()] - self.quotes[source][id.index()].ask_tenths
            };
            gain(*b).cmp(&gain(*a)).then(a.index().cmp(&b.index()))
        });
        let mut room = u64::from(
            self.capacity_tonnes
                .checked_sub(self.used_tonnes)
                .ok_or(TradePlanningError::MarketUnavailable)?,
        );
        let mut cash = u64::from(self.cash_tenths);
        let mut estimate = carried;
        let mut purchases = Vec::new();
        for id in exports {
            let i = id.index();
            let quote = self.quotes[source][i];
            let storage = if id.unit() == CargoUnit::Tonne {
                room
            } else {
                63_u64.saturating_sub(self.ordinary_quantities[i])
            };
            let affordable = if quote.ask_tenths == 0 {
                quote.stock
            } else {
                cash / u64::from(quote.ask_tenths)
            };
            let quantity = storage.min(affordable).min(quote.stock);
            if quantity == 0 {
                continue;
            }
            let gross = i128::from(quantity)
                * i128::from(self.benchmark_bids[target][i] - quote.ask_tenths);
            estimate = sum(estimate, gross)?;
            if id.unit() == CargoUnit::Tonne {
                room -= quantity;
            }
            cash = cash
                .checked_sub(
                    quantity
                        .checked_mul(u64::from(quote.ask_tenths))
                        .ok_or(TradePlanningError::Arithmetic)?,
                )
                .ok_or(TradePlanningError::Arithmetic)?;
            purchases.push(TradeExportEstimate {
                commodity: id,
                quantity,
                ask_tenths: quote.ask_tenths,
                benchmark_bid_tenths: self.benchmark_bids[target][i],
                gross_tenths: gross,
            });
        }
        Ok(TradeProfitEstimate {
            destination: SITES[target],
            distance_ly,
            estimated_delta_tenths: estimate,
            carried_delta_tenths: carried,
            exports: purchases,
        })
    }
}
impl World {
    /// Read-only source-grounded estimates. No returned value is accepted by a transaction API.
    pub fn navigation_trade_preview(
        &self,
        target: SystemId,
        tick: u64,
    ) -> Result<NavigationTradePreview, TradePlanningError> {
        use TradePlanningError as E;
        // Reuse actual known/provider/fit/tank/planning-radius/current-berth refusal unchanged.
        self.review_navigation_route(target, RouteKind::FewestJumps, tick)
            .map_err(E::Navigation)?;
        let nav = self.snapshot().navigation.ok_or(E::MarketUnavailable)?;
        let projection = self
            .reserved_service_projection()
            .map_err(|_| E::MarketUnavailable)?;
        let markets = self.station_markets.as_ref().ok_or(E::MarketUnavailable)?;
        let catalogue = self
            .navigation_catalogue
            .as_ref()
            .ok_or(E::MarketUnavailable)?;
        let benchmark =
            station_services::navigation_bid_benchmarks().map_err(|_| E::MarketUnavailable)?;
        // Source content order is Avenor/Renas/Lareria; presentation/search order is original ID.
        let order = [0, 2, 1];
        let quotes = core::array::from_fn(|i| {
            core::array::from_fn(|id| {
                let site = [
                    station_services::MarketSite::Avenor,
                    station_services::MarketSite::Lareria,
                    station_services::MarketSite::Renas,
                ][i];
                markets.commodity_quote(site, CommodityId::ALL[id])
            })
        });
        let mut result = NavigationTradePreview {
            as_of_tick: tick,
            origin: nav.system,
            destination: target,
            paths: Vec::new(),
            direct_profits: Vec::new(),
            quotes,
            benchmark_bids: core::array::from_fn(|i| benchmark[order[i]]),
            ordinary_quantities: core::array::from_fn(|i| {
                if i == 0 {
                    u64::from(projection.food_tonnes)
                } else {
                    projection.other_commodities[i - 1]
                }
            }),
            reserved_quantities: core::array::from_fn(|i| {
                if i == 0 {
                    u64::from(projection.reserved_food_tonnes)
                } else {
                    projection.reserved_commodities[i - 1]
                }
            }),
            cash_tenths: projection.credits_tenths,
            used_tonnes: projection.used_tonnes,
            capacity_tonnes: projection.hold_capacity_tonnes,
            fuel_tenths: nav.fuel_units,
            fitted_reach_ly: self.jump_reach_ly().ok_or(E::MarketUnavailable)?,
        };
        let first = result
            .fitted_reach_ly
            .min(f64::from(result.fuel_tenths) / 10.0);
        let full = result.fitted_reach_ly.min(f64::from(FUEL_CAPACITY) / 10.0);
        let distance = |a, b| {
            catalogue
                .distance_ly(a, b)
                .map_err(|_| E::MarketUnavailable)
        };
        for (i, id) in SITES
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, id)| *id != nav.system)
        {
            let d = distance(nav.system, id)?;
            if d <= first {
                let profit = result.direct_profit(i, d)?;
                if profit.estimated_delta_tenths > 0 {
                    result.direct_profits.push(profit);
                }
            }
        }
        result.direct_profits.sort_by(|a, b| {
            b.estimated_delta_tenths
                .cmp(&a.estimated_delta_tenths)
                .then(a.destination.system.cmp(&b.destination.system))
        });
        result.direct_profits.truncate(5);
        let mut candidates = vec![vec![nav.system, target]];
        for via in SITES
            .into_iter()
            .filter(|id| *id != nav.system && *id != target)
        {
            candidates.push(vec![nav.system, via, target]);
        }
        let mut feasible = Vec::new();
        for path in candidates {
            let mut distances = Vec::new();
            let mut legs = Vec::new();
            let mut exposure = 0.0;
            let mut trade = 0_u128;
            let mut sale = 0_u128;
            for (leg, pair) in path.windows(2).enumerate() {
                let d = distance(pair[0], pair[1])?;
                if d > if leg == 0 { first } else { full } {
                    break;
                }
                distances.push(d);
                // Existing two-node pure provider supplies original arrival exposure, not invented government risk.
                let direct = catalogue
                    .route_alternatives(pair[0], pair[1], pair, full, full)
                    .map_err(|_| E::MarketUnavailable)?;
                exposure += direct
                    .first()
                    .ok_or(E::MarketUnavailable)?
                    .arrival_hostile_fraction_sum;
                trade = trade
                    .checked_add(result.leg_opportunity(index(pair[0])?, index(pair[1])?)?)
                    .ok_or(E::Arithmetic)?;
                sale = sale.max(result.cargo_sale(index(pair[1])?)?);
                legs.push(result.leg_detail(index(pair[0])?, index(pair[1])?)?);
            }
            if distances.len() + 1 == path.len() {
                let value = TradePathEstimate {
                    systems: path,
                    legs,
                    labels: Vec::new(),
                    distance_ly: distances.iter().sum(),
                    arrival_exposure: exposure,
                    independent_leg_gross_tenths: trade,
                    best_carried_sale_tenths: sale,
                };
                feasible.push(value);
            }
        }
        // At three admitted systems these are every simple feasible path. Strict positive
        // original scores cannot improve with a cycle; ID order resolves equal costs.
        let mut choices: Vec<(usize, &'static str)> = Vec::new();
        let source = index(nav.system)?;
        let destination = index(target)?;
        for (mode, label) in [
            (0, "Fewest (original distance tie)"),
            (1, "Safer"),
            (2, "Trade opportunities"),
        ] {
            let mut weights = vec![vec![None; 3]; 3];
            for (a, row) in weights.iter_mut().enumerate() {
                for (b, weight) in row.iter_mut().enumerate() {
                    if a == b {
                        continue;
                    }
                    let d = distance(SITES[a], SITES[b])?;
                    if d > if a == source { first } else { full } {
                        continue;
                    }
                    let pair = [SITES[a], SITES[b]];
                    let direct = catalogue
                        .route_alternatives(pair[0], pair[1], &pair, full, full)
                        .map_err(|_| E::MarketUnavailable)?;
                    let risk = direct
                        .first()
                        .ok_or(E::MarketUnavailable)?
                        .arrival_hostile_fraction_sum;
                    *weight = Some(match mode {
                        0 => 1.0 + d * 0.0001,
                        1 => 1.0 + risk,
                        _ => (1.0 + risk * 0.2 - result.leg_opportunity(a, b)? as f64 / 10000.0)
                            .max(0.25),
                    });
                }
            }
            if let Some(path) = crate::navigation::scored_route_path(&weights, source, destination)
                .map_err(|_| E::Arithmetic)?
            {
                let systems: Vec<_> = path.into_iter().map(|i| SITES[i]).collect();
                let i = feasible
                    .iter()
                    .position(|p| p.systems == systems)
                    .ok_or(E::MarketUnavailable)?;
                if mode != 2 || feasible[i].independent_leg_gross_tenths > 0 {
                    choices.push((i, label));
                }
            }
        }
        // With exactly three sites, only one eligible unique-path cargo-sale waypoint exists.
        let destination_sale = result.cargo_sale(index(target)?)?;
        if let Some((i, _)) = feasible.iter().enumerate().find(|(_, path)| {
            path.systems.len() == 3 && path.best_carried_sale_tenths > destination_sale
        }) {
            choices.push((i, "Best carried cargo sale"));
        }
        for (i, label) in choices {
            if let Some(existing) = result
                .paths
                .iter_mut()
                .find(|p| p.systems == feasible[i].systems)
            {
                existing.labels.push(label);
            } else {
                let mut path = feasible[i].clone();
                path.labels.push(label);
                result.paths.push(path);
            }
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests;

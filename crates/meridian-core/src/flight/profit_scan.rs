//! Read-only, bounded trade estimates for the actual station and directly reachable markets.
use super::World;
use crate::navigation::{ROUTE_SYSTEMS, SystemId, route_hop};
use crate::station_services::{
    AccountProjection, CargoUnit, CommodityId, CommodityQuote, MAX_MARKET_QUANTITY,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfitDestination {
    pub system: SystemId,
    /// Extra credits (tenths) over selling existing unreserved cargo at the current station.
    /// Current quotes, before fuel expense or risk; never a guaranteed future settlement.
    pub profit_tenths: u64,
}

impl World {
    /// At most five positive results. No hypothetical refuel, purchase, sale or save mutation.
    pub fn profit_destinations(&self) -> Vec<ProfitDestination> {
        let Some((dock, markets)) = self
            .current_dock_context()
            .zip(self.station_markets.as_ref())
        else {
            return Vec::new();
        };
        let Some(source) = super::services::service_site(dock.system()) else {
            return Vec::new();
        };
        let Some((catalogue, reach)) = self.navigation_catalogue.as_ref().zip(self.jump_reach_ly())
        else {
            return Vec::new();
        };
        let Ok(cargo) = self.reserved_service_projection() else {
            return Vec::new();
        };
        let origin = CommodityId::ALL.map(|id| markets.commodity_quote(source, id));
        let mut destinations = Vec::new();
        for system in ROUTE_SYSTEMS {
            if system == dock.system()
                || !route_hop(catalogue, dock.system(), system).is_ok_and(|(distance, debit)| {
                    distance <= reach.min(f64::from(cargo.fuel_tenths) / 10.0)
                        && debit <= cargo.fuel_tenths
                })
            {
                continue;
            }
            let Some(site) = super::services::service_site(system) else {
                continue;
            };
            let target = CommodityId::ALL.map(|id| markets.commodity_quote(site, id));
            let profit_tenths = estimate(cargo, &origin, &target);
            if profit_tenths > 0 {
                destinations.push(ProfitDestination {
                    system,
                    profit_tenths,
                });
            }
        }
        rank(&mut destinations);
        destinations
    }
}

fn rank(destinations: &mut Vec<ProfitDestination>) {
    destinations.sort_by_key(|result| {
        (
            core::cmp::Reverse(result.profit_tenths),
            result.system.galaxy,
            result.system.system,
        )
    });
    destinations.truncate(5);
}

fn cargo_quantity(cargo: AccountProjection, id: CommodityId) -> (u64, u64) {
    if id == CommodityId::Food {
        (
            u64::from(cargo.food_tonnes),
            u64::from(cargo.reserved_food_tonnes),
        )
    } else {
        (
            cargo.other_commodities[id.index() - 1],
            cargo.reserved_commodities[id.index() - 1],
        )
    }
}

fn estimate(
    cargo: AccountProjection,
    origin: &[CommodityQuote; 17],
    target: &[CommodityQuote; 17],
) -> u64 {
    let mut room = u64::from(cargo.hold_capacity_tonnes.saturating_sub(cargo.used_tonnes));
    let mut cash = u64::from(cargo.credits_tenths);
    let mut direct_sale = 0_u128;
    let mut destination_sale = 0_u128;
    let mut destination_capacity = [0; 17];
    for id in CommodityId::ALL {
        let i = id.index();
        let (held, reserved) = cargo_quantity(cargo, id);
        let sellable = held.saturating_sub(reserved);
        let local_qty = sellable.min(MAX_MARKET_QUANTITY.saturating_sub(origin[i].stock));
        let capacity = MAX_MARKET_QUANTITY.saturating_sub(target[i].stock);
        let target_qty = sellable.min(capacity);
        destination_capacity[i] = capacity - target_qty;
        direct_sale += u128::from(local_qty) * u128::from(origin[i].bid_tenths);
        destination_sale += u128::from(target_qty) * u128::from(target[i].bid_tenths);
    }
    // Original scan buys highest positive unit margin first. Share the same wallet and hold;
    // the three non-tonne goods retain the legacy scan's independent 63-unit purchase cap.
    let mut exports = CommodityId::ALL;
    exports.sort_by_key(|id| {
        let i = id.index();
        (
            core::cmp::Reverse(target[i].bid_tenths.saturating_sub(origin[i].ask_tenths)),
            i,
        )
    });
    for id in exports {
        let i = id.index();
        let price = u64::from(origin[i].ask_tenths);
        if u64::from(target[i].bid_tenths) <= price {
            continue;
        }
        let (held, _) = cargo_quantity(cargo, id);
        let tonnes = id.unit() == CargoUnit::Tonne;
        let storage = if tonnes {
            room
        } else {
            63_u64.saturating_sub(held)
        };
        let quantity = storage
            .min(origin[i].stock)
            .min(destination_capacity[i])
            .min(cash.checked_div(price).unwrap_or(u64::MAX));
        cash -= quantity * price;
        if tonnes {
            room -= quantity;
        }
        destination_sale += u128::from(quantity) * u128::from(target[i].bid_tenths);
    }
    // Native sales saturate cash at u32::MAX. Compare final wealth on the two paths.
    let direct = (u128::from(cargo.credits_tenths) + direct_sale).min(u128::from(u32::MAX));
    let remote = (u128::from(cash) + destination_sale).min(u128::from(u32::MAX));
    u64::try_from(remote.saturating_sub(direct)).unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::flight::equipment::tests::journey;
    use crate::station_services::{MarketSite, StationMarkets};

    #[test]
    fn real_station_scan_is_read_only_and_rejects_unreachable_or_unserviced_worlds() {
        let mut world = World::new_commander(24101).unwrap();
        let before = world.save_bytes().unwrap();
        let result = world.profit_destinations();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].system, SystemId::RENAS);
        assert!(result[0].profit_tenths > 0);
        assert_eq!(world.save_bytes().unwrap(), before);
        assert_eq!(
            World::from_save_bytes(&before)
                .unwrap()
                .profit_destinations(),
            result
        );
        for system in [SystemId::RENAS, SystemId::AVENOR, SystemId::RENAS] {
            journey(&mut world, system);
        }
        assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 28);
        assert!(world.profit_destinations().is_empty());
        let before = world.save_bytes().unwrap();
        assert!(world.profit_destinations().is_empty());
        assert_eq!(world.save_bytes().unwrap(), before);
        world.request_launch(world.snapshot().tick).unwrap();
        assert!(world.profit_destinations().is_empty());
        assert!(World::new().profit_destinations().is_empty());
    }

    #[test]
    fn estimate_limits_shared_wallet_hold_and_finite_stock_excluding_reserved_cargo() {
        let world = World::new_commander(24102).unwrap();
        let mut cargo = world.reserved_service_projection().unwrap();
        let markets = StationMarkets::new_commander(24102).unwrap();
        let mut origin = CommodityId::ALL.map(|id| markets.commodity_quote(MarketSite::Avenor, id));
        let mut target = CommodityId::ALL.map(|id| markets.commodity_quote(MarketSite::Renas, id));
        for quote in origin.iter_mut().chain(target.iter_mut()) {
            quote.ask_tenths = 0;
            quote.bid_tenths = 0;
            quote.stock = 0;
        }
        cargo.credits_tenths = 100;
        cargo.hold_capacity_tonnes = 3;
        cargo.used_tonnes = 1;
        cargo.food_tonnes = 1;
        cargo.reserved_food_tonnes = 1;
        let gold = CommodityId::Gold.index();
        cargo.other_commodities[gold - 1] = 62;
        cargo.reserved_commodities[gold - 1] = 62;
        for (id, ask, bid, stock) in [
            (CommodityId::Food, 10, 25, 10),
            (CommodityId::Textiles, 20, 45, 1),
            (CommodityId::Gold, 5, 15, 10),
        ] {
            let i = id.index();
            origin[i].ask_tenths = ask;
            origin[i].bid_tenths = ask - 2;
            origin[i].stock = stock;
            target[i].bid_tenths = bid;
        }
        target[gold].stock = MAX_MARKET_QUANTITY - 1;
        assert_eq!(estimate(cargo, &origin, &target), 50);
        cargo.credits_tenths = 20;
        assert_eq!(estimate(cargo, &origin, &target), 25);
        cargo.credits_tenths = 0;
        assert_eq!(estimate(cargo, &origin, &target), 0);
        cargo.reserved_food_tonnes = 0;
        assert_eq!(estimate(cargo, &origin, &target), 17);
        target[0].bid_tenths = 1;
        assert_eq!(estimate(cargo, &origin, &target), 0);
    }

    #[test]
    fn stable_ties_and_five_result_cap() {
        let mut results = (0..7)
            .rev()
            .map(|system| ProfitDestination {
                system: SystemId { galaxy: 0, system },
                profit_tenths: if system == 6 { 2 } else { 1 },
            })
            .collect::<Vec<_>>();
        rank(&mut results);
        assert_eq!(
            results
                .iter()
                .map(|result| result.system.system)
                .collect::<Vec<_>>(),
            [6, 0, 1, 2, 3]
        );
    }
}

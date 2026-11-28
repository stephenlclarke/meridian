//! Route planning over G01 stations with implemented travel, docking and refuelling.
use super::{
    NavigationError, NavigationGrade, OriginalNavigation, PlayerShipId, SystemId,
    persistence::{read_system, write_system},
    stock_jump_fuel_cost,
};
use crate::save::{Reader, SaveError, Writer};
use crate::station_services::CommodityId;

pub const ROUTE_SYSTEMS: [SystemId; 3] = [SystemId::AVENOR, SystemId::LARERIA, SystemId::RENAS];
const INVALID: SaveError = SaveError::InvalidState("navigation computer plan");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteError {
    ComputerRequired,
    UnsupportedSystem,
    AlreadyThere,
    OutsidePlanningRadius,
    RefuelBeforePlanning,
    NoFeasibleRoute,
    StalePlan,
    DockRequired,
    Paused,
    Save(SaveError),
}
impl core::fmt::Display for RouteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::ComputerRequired => "Fit a navigational computer",
            Self::UnsupportedSystem => "Route planning is unavailable for this destination",
            Self::AlreadyThere => "Already at this destination",
            Self::OutsidePlanningRadius => "Destination is outside this computer's planning radius",
            Self::RefuelBeforePlanning => "Refuel before planning",
            Self::NoFeasibleRoute => "No feasible route with the current fuel and fitted drive",
            Self::StalePlan => "Route has changed; reopen the planner",
            Self::DockRequired => "Dock before setting or clearing a route",
            Self::Paused => "Resume before changing the route",
            Self::Save(error) => return error.fmt(f),
        })
    }
}
impl std::error::Error for RouteError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutePreference {
    FewestJumps,
    Safest,
    TradeOpportunities,
    BestCargoSale,
}
impl RoutePreference {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FewestJumps => "Fewest Jumps",
            Self::Safest => "Safest",
            Self::TradeOpportunities => "Trade Opportunities",
            Self::BestCargoSale => "Best Cargo Sale",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutePlan {
    pub(crate) owner: u64,
    pub(crate) hull: u64,
    pub(crate) origin_visit: u64,
    pub(crate) origin_jumps: u64,
    pub(crate) grade: NavigationGrade,
    pub(crate) origin_fuel: u16,
    pub(crate) origin_storage_count: u8,
    pub(crate) stop: u8,
    pub(crate) systems: Vec<SystemId>,
}
impl RoutePlan {
    pub fn grade(&self) -> NavigationGrade {
        self.grade
    }

    pub fn systems(&self) -> &[SystemId] {
        &self.systems
    }

    pub fn destination(&self) -> SystemId {
        self.systems[self.systems.len() - 1]
    }

    pub(crate) fn current_system(&self) -> SystemId {
        self.systems[usize::from(self.stop)]
    }

    pub(crate) fn next_system(&self) -> Option<SystemId> {
        self.systems.get(usize::from(self.stop) + 1).copied()
    }

    pub(crate) fn agrees_with_ship(&self, ship: PlayerShipId) -> bool {
        self.owner == ship.owner().seed() && self.hull == ship.ordinal()
    }

    pub(crate) fn encode_save(&self, writer: &mut Writer) {
        writer.u64(self.owner);
        writer.u64(self.hull);
        writer.u64(self.origin_visit);
        writer.u64(self.origin_jumps);
        writer.u8(self.grade.tag());
        writer.u16(super::pack_fuel(
            self.origin_fuel,
            self.origin_storage_count,
        ));
        writer.u8(self.stop);
        writer.u8(self.systems.len() as u8);
        for &system in &self.systems {
            write_system(writer, system);
        }
    }

    pub(crate) fn decode_save(reader: &mut Reader<'_>) -> Result<Self, SaveError> {
        let owner = reader.u64()?;
        let hull = reader.u64()?;
        let origin_visit = reader.u64()?;
        let origin_jumps = reader.u64()?;
        let grade = NavigationGrade::decode(reader)?;
        let (origin_fuel, origin_storage_count) = super::unpack_fuel(reader.u16()?)?;
        let stop = reader.u8()?;
        let count = usize::from(reader.u8()?);
        if !(2..=ROUTE_SYSTEMS.len()).contains(&count) || usize::from(stop) >= count {
            return Err(INVALID);
        }
        let mut systems = Vec::with_capacity(count);
        for _ in 0..count {
            let system = read_system(reader)?;
            if !ROUTE_SYSTEMS.contains(&system) || systems.contains(&system) {
                return Err(INVALID);
            }
            systems.push(system);
        }
        if grade == NavigationGrade::Empty || origin_fuel == 0 || origin_visit <= origin_jumps {
            return Err(INVALID);
        }
        Ok(Self {
            owner,
            hull,
            origin_visit,
            origin_jumps,
            grade,
            origin_fuel,
            origin_storage_count,
            stop,
            systems,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteAlternative {
    pub preferences: Vec<RoutePreference>,
    pub plan: RoutePlan,
    pub distance_ly: f64,
    pub fuel_units: u16,
    pub risk: f64,
    pub trade_tenths: u64,
    pub cargo_sale_tenths: u64,
    /// Read-only estimates from the same live quote snapshot as the route score.
    pub trade_legs: Vec<RouteTradeLeg>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeExportEstimate {
    pub commodity: CommodityId,
    pub quantity: u64,
    pub ask_tenths: u32,
    pub bid_tenths: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTradeLeg {
    pub from: SystemId,
    pub to: SystemId,
    pub profit_tenths: u64,
    pub exports: Vec<TradeExportEstimate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteStatus {
    Ready,
    RefuelRequired,
    AwaitingDock,
    ComputerUnavailable,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteProgress {
    pub plan: RoutePlan,
    pub completed_legs: u8,
    /// A jump target only when departing a confirmed stop.
    pub next_system: Option<SystemId>,
    pub next_fuel_units: Option<u16>,
    pub status: RouteStatus,
}

/// Existing World market quotes and unreserved cargo, converted to estimates only.
#[derive(Default)]
pub(crate) struct RouteEconomy {
    pub(crate) trade: [[u64; ROUTE_SYSTEMS.len()]; ROUTE_SYSTEMS.len()],
    pub(crate) sale: [u64; ROUTE_SYSTEMS.len()],
    pub(crate) exports: [[Vec<TradeExportEstimate>; ROUTE_SYSTEMS.len()]; ROUTE_SYSTEMS.len()],
}

pub(crate) fn route_station_index(system: SystemId) -> Option<usize> {
    ROUTE_SYSTEMS
        .iter()
        .position(|&candidate| candidate == system)
}

// Frozen L337 OriginalWorlds/catalogue.json at a11fe169, sorted like ROUTE_SYSTEMS.
const HOSTILITY: [f64; ROUTE_SYSTEMS.len()] = [0.0, 0.1, 0.2];

pub(crate) fn route_hop(
    catalogue: &OriginalNavigation,
    from: SystemId,
    to: SystemId,
) -> Result<(f64, u16), NavigationError> {
    let distance = catalogue.distance_ly(from, to)?;
    Ok((distance, stock_jump_fuel_cost(distance)?))
}

pub(crate) fn route_feasible(
    catalogue: &OriginalNavigation,
    path: &[SystemId],
    first_fuel: u16,
    tank_capacity: u16,
    grade: NavigationGrade,
) -> bool {
    grade != NavigationGrade::Empty
        && path.windows(2).enumerate().all(|(index, hop)| {
            let fuel = if index == 0 {
                first_fuel
            } else {
                tank_capacity
            };
            route_hop(catalogue, hop[0], hop[1]).is_ok_and(|(distance, debit)| {
                distance <= grade.reach_ly().min(f64::from(fuel) / 10.0) && debit <= fuel
            })
        })
}

pub(crate) fn route_alternatives(
    catalogue: &OriginalNavigation,
    template: &RoutePlan,
    origin: SystemId,
    destination: SystemId,
    economy: &RouteEconomy,
) -> Result<Vec<RouteAlternative>, RouteError> {
    if !ROUTE_SYSTEMS.contains(&origin) || !ROUTE_SYSTEMS.contains(&destination) {
        return Err(RouteError::UnsupportedSystem);
    }
    if destination == origin {
        return Err(RouteError::AlreadyThere);
    }
    let distance = catalogue
        .distance_ly(origin, destination)
        .map_err(|_| RouteError::UnsupportedSystem)?;
    if template
        .grade
        .planning_radius_with_capacity(super::fuel_capacity(template.origin_storage_count))
        .is_some_and(|radius| distance > radius)
    {
        return Err(RouteError::OutsidePlanningRadius);
    }
    if template.origin_fuel == 0 {
        return Err(RouteError::RefuelBeforePlanning);
    }
    let paths = std::iter::once(vec![origin, destination]).chain(
        ROUTE_SYSTEMS
            .into_iter()
            .filter(|&via| via != origin && via != destination)
            .map(|via| vec![origin, via, destination]),
    );
    let mut candidates = Vec::new();
    for path in paths.filter(|path| {
        route_feasible(
            catalogue,
            path,
            template.origin_fuel,
            super::fuel_capacity(template.origin_storage_count),
            template.grade,
        )
    }) {
        candidates.push(candidate(catalogue, template, path, economy)?);
    }
    if candidates.is_empty() {
        return Err(RouteError::NoFeasibleRoute);
    }
    let mut result = Vec::<RouteAlternative>::new();
    for preference in [
        RoutePreference::FewestJumps,
        RoutePreference::Safest,
        RoutePreference::TradeOpportunities,
        RoutePreference::BestCargoSale,
    ] {
        let eligible = candidates.iter().filter(|c| match preference {
            RoutePreference::TradeOpportunities => c.trade_tenths > 0,
            RoutePreference::BestCargoSale => route_station_index(destination)
                .is_some_and(|index| c.cargo_sale_tenths > economy.sale[index]),
            _ => true,
        });
        let Some(chosen) = eligible
            .min_by(|a, b| score(a, preference, economy).total_cmp(&score(b, preference, economy)))
        else {
            continue;
        };
        if let Some(existing) = result
            .iter_mut()
            .find(|candidate| candidate.plan == chosen.plan)
        {
            existing.preferences.push(preference);
        } else {
            let mut chosen = chosen.clone();
            chosen.preferences.push(preference);
            result.push(chosen);
        }
    }
    Ok(result)
}

fn candidate(
    catalogue: &OriginalNavigation,
    template: &RoutePlan,
    systems: Vec<SystemId>,
    economy: &RouteEconomy,
) -> Result<RouteAlternative, RouteError> {
    let mut distance_ly = 0.0;
    let mut fuel_units = 0;
    let mut risk = 0.0;
    let mut trade_tenths = 0u64;
    let mut cargo_sale_tenths = 0;
    let mut trade_legs = Vec::new();
    for hop in systems.windows(2) {
        let from = route_station_index(hop[0]).ok_or(RouteError::UnsupportedSystem)?;
        let to = route_station_index(hop[1]).ok_or(RouteError::UnsupportedSystem)?;
        let (distance, debit) =
            route_hop(catalogue, hop[0], hop[1]).map_err(|_| RouteError::NoFeasibleRoute)?;
        distance_ly += distance;
        fuel_units += debit;
        risk += HOSTILITY[to];
        trade_tenths = trade_tenths.saturating_add(economy.trade[from][to]);
        cargo_sale_tenths = cargo_sale_tenths.max(economy.sale[to]);
        trade_legs.push(RouteTradeLeg {
            from: hop[0],
            to: hop[1],
            profit_tenths: economy.trade[from][to],
            exports: economy.exports[from][to].clone(),
        });
    }
    Ok(RouteAlternative {
        preferences: Vec::new(),
        plan: RoutePlan {
            systems,
            ..template.clone()
        },
        distance_ly,
        fuel_units,
        risk,
        trade_tenths,
        cargo_sale_tenths,
        trade_legs,
    })
}

fn score(route: &RouteAlternative, preference: RoutePreference, economy: &RouteEconomy) -> f64 {
    let jumps = (route.plan.systems.len() - 1) as f64;
    match preference {
        RoutePreference::FewestJumps => jumps + route.distance_ly * 0.0001,
        RoutePreference::Safest => jumps + route.risk,
        RoutePreference::TradeOpportunities => route
            .plan
            .systems
            .windows(2)
            .map(|hop| {
                route_station_index(hop[0])
                    .zip(route_station_index(hop[1]))
                    .map_or(1.0, |(a, b)| {
                        (1.0 + HOSTILITY[b] * 0.2 - economy.trade[a][b] as f64 / 10_000.0).max(0.25)
                    })
            })
            .sum(),
        RoutePreference::BestCargoSale => -(route.cargo_sale_tenths as f64),
    }
}

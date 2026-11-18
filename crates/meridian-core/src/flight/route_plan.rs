//! Installed current-hull route bookmarks. World still owns every jump, fuel debit and berth.
use super::{ModuleKind, ModuleSlot, World};
use crate::navigation::{DockContext, FUEL_CAPACITY, RouteKind, SystemId};
use crate::save::{Reader, SaveError, Writer};
use std::{fmt, sync::Arc};

const SITES: [SystemId; 3] = [SystemId::AVENOR, SystemId::RENAS, SystemId::LARERIA];
const INVALID: SaveError = SaveError::InvalidState("installed navigation route");
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationRoutePhase {
    Ready,
    AwaitingDock,
    Complete,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationRouteSnapshot {
    pub systems: Vec<SystemId>,
    pub kind: RouteKind,
    pub phase: NavigationRoutePhase,
    pub next: Option<SystemId>,
    pub stop_index: usize,
    pub fuel_required: Option<u16>,
    pub refuel_required: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationRouteError {
    Unavailable,
    StaleReview,
    NoComputer,
    UnsupportedDestination,
    NoRoute,
    Busy,
    AwaitingDock,
    Complete,
    Cancelled,
    Save(SaveError),
    Jump(crate::navigation::NavigationError),
}
impl fmt::Display for NavigationRouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => {
                f.write_str("Route planning requires an active current main berth")
            }
            Self::StaleReview => f.write_str("Route values changed; review again"),
            Self::NoComputer => f.write_str("Fit an owned navigation computer before planning"),
            Self::UnsupportedDestination => {
                f.write_str("This destination has no admitted arrival service")
            }
            Self::NoRoute => f.write_str("No feasible route with current fuel and fitted range"),
            Self::Busy => f.write_str("Finish the current jump before changing the route"),
            Self::AwaitingDock => {
                f.write_str("Physically dock at this route stop before its next leg")
            }
            Self::Complete => f.write_str("Navigation destination reached"),
            Self::Cancelled => f.write_str("Route cancelled; dock and review a new route"),
            Self::Save(error) => fmt::Display::fmt(error, f),
            Self::Jump(error) => fmt::Display::fmt(error, f),
        }
    }
}
impl std::error::Error for NavigationRouteError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Plan {
    path: [SystemId; 3],
    len: u8,
    kind: RouteKind,
    dock: DockContext,
    tick: u64,
    jumps: u64,
    account_revision: u64,
    item: u64,
    tier: u8,
    fuel: u16,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct State {
    revision: u64,
    cancelled: bool,
    plan: Option<Plan>,
}
#[derive(Debug, Clone)]
pub struct NavigationRouteReview {
    plan: Plan,
    revision: u64,
    epoch: u64,
    scope: Arc<()>,
    preview: crate::navigation::RouteAlternative,
}
impl NavigationRouteReview {
    pub fn preview(&self) -> &crate::navigation::RouteAlternative {
        &self.preview
    }
}
pub struct PreparedNavigationRoute<'a> {
    world: &'a mut World,
    state: State,
    bytes: Vec<u8>,
}
impl PreparedNavigationRoute<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn commit(self) {
        self.world.route_plan = self.state;
    }
    pub fn publish_with<E>(self, writer: impl FnOnce(&[u8]) -> Result<(), E>) -> Result<(), E> {
        writer(&self.bytes)?;
        self.commit();
        Ok(())
    }
}
fn tier(kind: ModuleKind) -> Option<u8> {
    match kind {
        ModuleKind::BasicNavigation => Some(1),
        ModuleKind::IntermediateNavigation => Some(2),
        ModuleKind::AdvancedNavigation => Some(3),
        _ => None,
    }
}
fn reach(tier: u8) -> f64 {
    f64::from(tier) * 7.0
}
fn actual_reach(tier: u8) -> f64 {
    reach(tier).max(crate::navigation::STOCK_JUMP_REACH_LY)
}
fn kind_tag(kind: RouteKind) -> u8 {
    match kind {
        RouteKind::FewestJumps => 0,
        RouteKind::Safer => 1,
    }
}
fn read_kind(r: &mut Reader<'_>) -> Result<RouteKind, SaveError> {
    match r.u8()? {
        0 => Ok(RouteKind::FewestJumps),
        1 => Ok(RouteKind::Safer),
        _ => Err(INVALID),
    }
}
impl World {
    /// Original Plugh.knownSystems exposes ordinary charts without a visit/discovery grant.
    /// The hidden synthetic port 256 is not in the admitted original G01 catalogue.
    pub fn known_navigation_systems(&self) -> Option<Vec<SystemId>> {
        let nav = self.snapshot().navigation?;
        (nav.system.galaxy == 0).then(|| {
            (0..256)
                .map(|system| SystemId { galaxy: 0, system })
                .collect()
        })
    }
    pub fn review_navigation_route(
        &self,
        target: SystemId,
        kind: RouteKind,
        tick: u64,
    ) -> Result<NavigationRouteReview, NavigationRouteError> {
        use NavigationRouteError as E;
        if tick != self.state.tick || self.is_paused() {
            return Err(E::StaleReview);
        }
        let dock = self
            .current_dock_context()
            .filter(|dock| SITES.contains(&dock.system()))
            .ok_or(E::Unavailable)?;
        if !SITES.contains(&target) || target == dock.system() {
            return Err(E::UnsupportedDestination);
        }
        let account = self.commander_account.as_ref().ok_or(E::Unavailable)?;
        let fit = self
            .modular_equipment_snapshot()
            .ok_or(E::NoComputer)?
            .items
            .into_iter()
            .find(|item| item.slot == Some(ModuleSlot::Navigation))
            .ok_or(E::NoComputer)?;
        let tier = tier(fit.kind).ok_or(E::NoComputer)?;
        let nav = self.navigation.ok_or(E::Unavailable)?;
        let fuel = account.fuel_tenths();
        let maximum = self.jump_reach_ly().ok_or(E::NoComputer)?;
        let catalogue = self.navigation_catalogue.as_ref().ok_or(E::Unavailable)?;
        let distance = catalogue
            .distance_ly(dock.system(), target)
            .map_err(E::Jump)?;
        if tier != 3 && distance > f64::from(FUEL_CAPACITY) / 10.0 + f64::from(tier) * 7.0 {
            return Err(E::NoRoute);
        }
        // Visibility and executable geometry are separate. Do not discover a world
        // or approve an unsupported arrival merely because its chart record exists.
        let visible = self.known_navigation_systems().ok_or(E::Unavailable)?;
        let eligible: Vec<_> = SITES
            .into_iter()
            .filter(|id| visible.contains(id))
            .collect();
        let alternatives = catalogue
            .route_alternatives(
                dock.system(),
                target,
                &eligible,
                maximum.min(f64::from(fuel) / 10.0),
                maximum.min(f64::from(FUEL_CAPACITY) / 10.0),
            )
            .map_err(|_| E::NoRoute)?;
        let route = alternatives
            .into_iter()
            .find(|route| route.labels.contains(&kind))
            .ok_or(E::NoRoute)?;
        let mut path = [dock.system(); 3];
        if !(2..=3).contains(&route.systems.len()) {
            return Err(E::NoRoute);
        }
        path[..route.systems.len()].copy_from_slice(&route.systems);
        Ok(NavigationRouteReview {
            plan: Plan {
                path,
                len: route.systems.len() as u8,
                kind,
                dock,
                tick,
                jumps: nav.jump_sequence(),
                account_revision: account.revision(),
                item: fit.id,
                tier,
                fuel,
            },
            revision: self.route_plan.revision,
            epoch: self.freight_session.as_ref().ok_or(E::Unavailable)?.epoch(),
            scope: self.mining_scope.clone(),
            preview: route,
        })
    }
    pub fn prepare_navigation_route(
        &mut self,
        review: NavigationRouteReview,
        tick: u64,
    ) -> Result<PreparedNavigationRoute<'_>, NavigationRouteError> {
        use NavigationRouteError as E;
        if !Arc::ptr_eq(&review.scope, &self.mining_scope) {
            return Err(E::StaleReview);
        }
        let fresh = self.review_navigation_route(
            review.plan.path[usize::from(review.plan.len) - 1],
            review.plan.kind,
            tick,
        )?;
        if fresh.plan != review.plan
            || fresh.revision != review.revision
            || fresh.epoch != review.epoch
        {
            return Err(E::StaleReview);
        }
        let state = State {
            revision: self
                .route_plan
                .revision
                .checked_add(1)
                .ok_or(E::StaleReview)?,
            cancelled: false,
            plan: Some(review.plan),
        };
        let bytes = self
            .save_bytes_with_navigation_route(&state)
            .map_err(E::Save)?;
        Ok(PreparedNavigationRoute {
            world: self,
            state,
            bytes,
        })
    }
    pub fn prepare_clear_navigation_route(
        &mut self,
        tick: u64,
    ) -> Result<PreparedNavigationRoute<'_>, NavigationRouteError> {
        use NavigationRouteError as E;
        if tick != self.state.tick || self.is_paused() || self.commander_account.is_none() {
            return Err(E::StaleReview);
        }
        if self
            .snapshot()
            .navigation
            .is_some_and(|nav| nav.travel.is_some())
        {
            return Err(E::Busy);
        }
        let state = State {
            revision: self
                .route_plan
                .revision
                .checked_add(1)
                .ok_or(E::StaleReview)?,
            cancelled: false,
            plan: None,
        };
        let bytes = self
            .save_bytes_with_navigation_route(&state)
            .map_err(E::Save)?;
        Ok(PreparedNavigationRoute {
            world: self,
            state,
            bytes,
        })
    }
    pub fn navigation_route_snapshot(&self) -> Option<NavigationRouteSnapshot> {
        let plan = self.route_plan.plan?;
        let path = plan.path[..usize::from(plan.len)].to_vec();
        let nav = self.snapshot().navigation?;
        let fit = self
            .modular_equipment_snapshot()?
            .items
            .into_iter()
            .find(|item| item.slot == Some(ModuleSlot::Navigation));
        let history = self.jump_journal.route_legs(plan.jumps)?;
        let mut cancelled =
            self.route_plan.cancelled || fit.and_then(|item| tier(item.kind)).is_none();
        let count = history.len();
        if count >= path.len() {
            cancelled = true;
        }
        for (i, leg) in history.iter().enumerate() {
            if path.get(i) != Some(&leg.source)
                || path.get(i + 1) != Some(&leg.destination)
                || (i > 0 && leg.source_visit <= history[i - 1].source_visit.saturating_add(1))
            {
                cancelled = true;
            }
        }
        if path.get(count) != Some(&nav.system) {
            cancelled = true;
        }
        let mut stop = count.min(path.len() - 1);
        let waiting = history.last().is_some_and(|leg| {
            self.current_dock_context().is_none() && nav.visit == leg.source_visit.saturating_add(1)
        });
        if waiting {
            stop = stop.saturating_sub(1);
        }
        let phase = if cancelled {
            NavigationRoutePhase::Cancelled
        } else if waiting {
            NavigationRoutePhase::AwaitingDock
        } else if stop + 1 == path.len() {
            NavigationRoutePhase::Complete
        } else {
            NavigationRoutePhase::Ready
        };
        let next = (phase == NavigationRoutePhase::Ready).then(|| path[stop + 1]);
        let distance = next.and_then(|target| {
            self.navigation_catalogue
                .as_ref()?
                .distance_ly(nav.system, target)
                .ok()
        });
        let fuel_required = distance.and_then(|d| crate::navigation::stock_jump_fuel_cost(d).ok());
        Some(NavigationRouteSnapshot {
            systems: path,
            kind: plan.kind,
            phase,
            next,
            stop_index: stop,
            fuel_required,
            refuel_required: fuel_required.is_some_and(|cost| cost > nav.fuel_units)
                || distance.is_some_and(|distance| distance > f64::from(nav.fuel_units) / 10.0),
        })
    }
    pub(super) fn observe_navigation_route_dock(&mut self) {
        if self
            .navigation_route_snapshot()
            .is_some_and(|route| route.phase == NavigationRoutePhase::Cancelled)
        {
            self.route_plan.cancelled = true;
        }
    }
    pub fn request_navigation_route_jump(&mut self, tick: u64) -> Result<(), NavigationRouteError> {
        use NavigationRouteError as E;
        let route = self.navigation_route_snapshot().ok_or(E::NoRoute)?;
        match route.phase {
            NavigationRoutePhase::Ready => {}
            NavigationRoutePhase::AwaitingDock => return Err(E::AwaitingDock),
            NavigationRoutePhase::Complete => return Err(E::Complete),
            NavigationRoutePhase::Cancelled => return Err(E::Cancelled),
        }
        self.request_jump(route.next.ok_or(E::NoRoute)?, tick)
            .map_err(E::Jump)
    }
}
impl State {
    pub(super) fn manual_jump(&mut self, target: SystemId, expected: Option<SystemId>) {
        if self.plan.is_some() && expected != Some(target) {
            self.cancelled = true;
        }
    }
    pub(super) fn encode(&self, w: &mut Writer) {
        w.u64(self.revision);
        w.bool(self.cancelled);
        w.option(self.plan.as_ref(), |w, p| {
            w.u8(p.len);
            for id in p.path {
                w.u8(id.galaxy);
                w.u16(id.system);
            }
            w.u8(kind_tag(p.kind));
            p.dock.encode_save(w);
            w.u64(p.tick);
            w.u64(p.jumps);
            w.u64(p.account_revision);
            w.u64(p.item);
            w.u8(p.tier);
            w.u16(p.fuel);
        });
    }
    pub(super) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let revision = r.u64()?;
        let cancelled = r.bool()?;
        let plan = r.option(|r| {
            let len = r.u8()?;
            let path = r.array(|r| {
                Ok(SystemId {
                    galaxy: r.u8()?,
                    system: r.u16()?,
                })
            })?;
            Ok(Plan {
                len,
                path,
                kind: read_kind(r)?,
                dock: DockContext::decode_save(r)?,
                tick: r.u64()?,
                jumps: r.u64()?,
                account_revision: r.u64()?,
                item: r.u64()?,
                tier: r.u8()?,
                fuel: r.u16()?,
            })
        })?;
        Ok(Self {
            revision,
            cancelled,
            plan,
        })
    }
    pub(super) fn validate(&self, world: &World) -> Result<(), SaveError> {
        let Some(p) = self.plan else {
            return if self.cancelled { Err(INVALID) } else { Ok(()) };
        };
        let current = world.navigation.ok_or(INVALID)?;
        let account = world.commander_account.as_ref().ok_or(INVALID)?;
        if self.revision == 0
            || !(2..=3).contains(&p.len)
            || !(1..=3).contains(&p.tier)
            || p.fuel == 0
            || p.fuel > FUEL_CAPACITY
            || p.tick > world.state.tick
            || p.tick < p.dock.tick()
            || p.account_revision > account.revision()
            || p.dock.owner() != current.owner()
            || p.dock.ship() != current.ship()
            || p.dock.visit() > world.snapshot().navigation.ok_or(INVALID)?.visit
            || p.dock.system() != p.path[0]
        {
            return Err(INVALID);
        }
        let prefix = p
            .dock
            .visit()
            .checked_sub(p.dock.sequence())
            .ok_or(INVALID)?;
        if prefix != p.jumps {
            return Err(INVALID);
        }
        world.jump_journal.spent_at(p.dock, p.tick)?;
        let fit = world
            .modular
            .route_navigation_item(p.dock, p.tick, p.account_revision)
            .ok_or(INVALID)?;
        if fit.id != p.item || tier(fit.kind) != Some(p.tier) {
            return Err(INVALID);
        }
        let path = &p.path[..usize::from(p.len)];
        if p.path[usize::from(p.len)..]
            .iter()
            .any(|id| *id != p.path[0])
        {
            return Err(INVALID);
        }
        if path.iter().any(|id| !SITES.contains(id))
            || path
                .iter()
                .enumerate()
                .any(|(i, id)| path[..i].contains(id))
        {
            return Err(INVALID);
        }
        let catalogue = world.navigation_catalogue.as_ref().ok_or(INVALID)?;
        let alternatives = catalogue
            .route_alternatives(
                p.path[0],
                *path.last().ok_or(INVALID)?,
                &SITES,
                actual_reach(p.tier).min(f64::from(p.fuel) / 10.0),
                actual_reach(p.tier).min(f64::from(FUEL_CAPACITY) / 10.0),
            )
            .map_err(|_| INVALID)?;
        if !alternatives
            .iter()
            .any(|route| route.labels.contains(&p.kind) && route.systems == path)
        {
            return Err(INVALID);
        }
        let future = world.jump_journal.route_legs(p.jumps).ok_or(INVALID)?;
        if future.first().is_some_and(|leg| {
            leg.source != p.path[0]
                || leg.source_visit < p.dock.visit()
                || leg.departure_tick < p.tick
        }) {
            return Err(INVALID);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests;

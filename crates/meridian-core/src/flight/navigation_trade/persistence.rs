use super::{
    Attempt, Balance, Fit, Fuel, FuelAnchor, INVALID, Journey, MAX_ACTIVE, MAX_RECEIPTS,
    MAX_RECORDS, NavigationTonneId, NavigationTradeOutcome as Outcome, Plan, Reader, SaveError,
    State, Terminal, Trade, World, Writer, legal_tonne, same_carrier, site,
};
use crate::flight::persistence::Commerce;
use crate::navigation::{
    DockContext, DockOrigin, SavedNavigationSummary, SystemId, stock_jump_fuel_cost,
};
use crate::station_services::{CommodityId, MAX_MARKET_QUANTITY, StationMarkets};

struct Context<'a> {
    world: &'a World,
    commerce: &'a Commerce<'a>,
    nav: SavedNavigationSummary,
}
impl State {
    pub(in crate::flight) fn validate(
        &self,
        world: &World,
        commerce: &Commerce<'_>,
        nav: SavedNavigationSummary,
        t01: (usize, usize),
        t02: (usize, usize),
    ) -> Result<(), SaveError> {
        let (passenger_records, passenger_active) = commerce.modular.passenger_occupancy();
        let records = self.attempts.len()
            + passenger_records
            + t01.0
            + t02.0
            + world
                .freight_session
                .as_ref()
                .map_or(0, |s| s.state.retained_mission_records());
        let active = usize::from(self.active().is_some())
            + passenger_active
            + t01.1
            + t02.1
            + usize::from(world.active_freight_job().is_some());
        if records > MAX_RECORDS || active > MAX_ACTIVE {
            return Err(INVALID);
        }
        let context = Context {
            world,
            commerce,
            nav,
        };
        let account = commerce.account.ok_or(INVALID)?;
        let projection = account
            .service_projection(&world.freight_session.as_ref().ok_or(INVALID)?.state)
            .map_err(|_| INVALID)?;
        let cabin_tonnes = commerce.modular.cabin_tonnes().map_err(|_| INVALID)?;
        let projection = crate::flight::modular::add_cabin_mass(projection, cabin_tonnes)
            .map_err(|_| INVALID)?;
        self.reservations().apply(projection).map_err(|_| INVALID)?;
        if self
            .active()
            .and_then(|a| a.plan.filter(|_| a.buy.is_none()))
            .is_some_and(|p| super::quantity(projection, p.commodity) != 0)
        {
            return Err(INVALID);
        }
        let mut first = None;
        let mut operations = Vec::new();
        let mut jump = 0;
        let mut fuel_units = 0_u64;
        for (i, a) in self.attempts.iter().enumerate() {
            a.validate(i, &context)?;
            if i + 1 < self.attempts.len() && a.terminal.is_none() {
                return Err(INVALID);
            }
            if i > 0 {
                validate_next(&self.attempts[i - 1], a)?;
            }
            if a.terminal.is_some_and(|t| t.outcome == Outcome::Complete) && first.is_none() {
                first = Some(i as u8);
            }
            for j in &a.journeys {
                if j.jump <= jump {
                    return Err(INVALID);
                }
                jump = j.jump;
                fuel_units += u64::from(j.debit);
            }
            for t in [a.buy, a.sale].into_iter().flatten() {
                operations.push((t.transaction, t.account_revision, t.tick));
            }
            for f in &a.fuel {
                operations.push((f.transaction, f.account_revision, f.tick));
            }
        }
        if self.trace != first || fuel_units > nav.fuel_spent {
            return Err(INVALID);
        }
        operations.sort_unstable();
        if operations
            .windows(2)
            .any(|p| p[0].0 >= p[1].0 || p[0].1 >= p[1].1 || p[0].2 > p[1].2)
        {
            return Err(INVALID);
        }
        Ok(())
    }
    pub(in crate::flight) fn encode(&self, w: &mut Writer) {
        w.u8(self.attempts.len() as u8);
        for a in &self.attempts {
            a.encode(w);
        }
        w.option(self.trace.as_ref(), |w, n| w.u8(*n));
    }
    pub(in crate::flight) fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let count = usize::from(r.u8()?);
        if count > MAX_RECORDS {
            return Err(INVALID);
        }
        let mut attempts = Vec::with_capacity(count);
        for _ in 0..count {
            attempts.push(Attempt::decode(r)?);
        }
        Ok(Self {
            attempts,
            trace: r.option(|r| r.u8())?,
        })
    }
}
fn validate_next(old: &Attempt, next: &Attempt) -> Result<(), SaveError> {
    let t = old.terminal.ok_or(INVALID)?;
    if t.tick > next.accepted_tick
        || t.account_revision
            .zip(next.accepted_account_revision)
            .is_some_and(|(closed, accepted)| closed > accepted)
        || t.dock.sequence() > next.dock.sequence()
        || t.dock.visit() > next.dock.visit()
        || t.dock.tick() > next.dock.tick()
        || (t.dock.sequence() == next.dock.sequence() && t.dock != next.dock)
        || (t.dock.sequence() < next.dock.sequence() && t.tick >= next.dock.tick())
    {
        return Err(INVALID);
    }
    Ok(())
}
impl Attempt {
    fn validate(&self, ordinal: usize, c: &Context<'_>) -> Result<(), SaveError> {
        validate_dock(self.dock, c)?;
        let current_revision = c.commerce.account.ok_or(INVALID)?.revision();
        if self.accepted_tick >= 1 << 63
            || self.accepted_tick < self.dock.tick()
            || self.accepted_tick > c.world.state.tick
            || self.accepted_account_revision.is_some_and(|revision| {
                revision > current_revision
                    || self
                        .plan
                        .is_some_and(|plan| plan.account_revision < revision)
            })
        {
            return Err(INVALID);
        }
        let Some(plan) = self.plan else {
            return if self.buy.is_none()
                && self.sale.is_none()
                && self.journeys.is_empty()
                && self.fuel.is_empty()
            {
                self.validate_terminal(c)
            } else {
                Err(INVALID)
            };
        };
        plan.validate(self, c)?;
        if let Some(buy) = self.buy {
            buy.validate_buy(self, plan, ordinal, c)?;
        } else if self.sale.is_some() || !self.journeys.is_empty() {
            return Err(INVALID);
        }
        self.validate_journeys(c)?;
        if self.terminal.is_none()
            && let Some(buy) = self.buy
        {
            let (jump, system) = self
                .journeys
                .last()
                .map_or((buy.jump_sequence, buy.dock.system()), |j| {
                    (j.jump, j.destination)
                });
            if jump != c.nav.jumps
                || c.world
                    .navigation
                    .is_none_or(|n| n.snapshot(0).system != system)
            {
                return Err(INVALID);
            }
        }
        for f in &self.fuel {
            f.validate(plan, c)?;
            if self.terminal.is_some_and(|t| f.tick > t.tick) {
                return Err(INVALID);
            }
        }
        if self.fuel.len() > MAX_RECEIPTS
            || self
                .fuel
                .windows(2)
                .any(|p| p[0].transaction >= p[1].transaction || p[0].ordinal >= p[1].ordinal)
        {
            return Err(INVALID);
        }
        self.validate_fuel_ledger(plan, c)?;
        if let Some(sale) = self.sale {
            sale.validate_sale(self, plan, c)?;
        }
        self.validate_terminal(c)
    }
    fn validate_fuel_ledger(&self, plan: Plan, c: &Context<'_>) -> Result<(), SaveError> {
        let markets = c.commerce.markets.ok_or(INVALID)?;
        let current = FuelAnchor {
            refilled_units: markets.fuel_accounting().0,
            fuel: c.commerce.account.ok_or(INVALID)?.fuel_tenths(),
            jump: c.nav.jumps,
        };
        let end = self.terminal.map_or(current, |t| t.fuel);
        let added = self
            .fuel
            .iter()
            .map(|f| u64::from(f.units_after - f.units_before))
            .sum::<u64>();
        if end.refilled_units > current.refilled_units
            || end.jump > current.jump
            || end.fuel > crate::navigation::MAX_FUEL_CAPACITY
            || end.refilled_units.checked_sub(plan.refilled_at_plan) != Some(added)
        {
            return Err(INVALID);
        }
        if end.refilled_units == current.refilled_units
            && end.jump == current.jump
            && end.fuel != current.fuel
        {
            let terminal = self.terminal.ok_or(INVALID)?;
            let later = c.world.xyzzy.refills_after_terminal(
                terminal.dock,
                terminal.tick,
                terminal.account_revision,
            ) + c
                .world
                .solar
                .receipts()
                .filter(|receipt| {
                    terminal
                        .account_revision
                        .map_or(receipt.tick > terminal.tick, |revision| {
                            receipt.revision > revision
                        })
                })
                .map(|receipt| u64::from(receipt.added))
                .sum::<u64>();
            if u64::from(end.fuel).checked_add(later) != Some(u64::from(current.fuel)) {
                return Err(INVALID);
            }
        }
        if let Some(buy) = self.buy {
            self.validate_fuel_continuity(buy, end, c)?;
        }
        Ok(())
    }
    fn validate_fuel_continuity(
        &self,
        buy: Trade,
        end: FuelAnchor,
        c: &Context<'_>,
    ) -> Result<(), SaveError> {
        let mut events = Vec::new();
        for f in self.fuel.iter().filter(|f| f.transaction > buy.transaction) {
            events.push((f.account_revision, f.tick, f.units_before, f.units_after));
        }
        for j in &self.journeys {
            events.push((j.account_revision, j.tick, j.fuel_before, j.fuel_after));
        }
        if let Some(sale) = self.sale {
            events.push((
                sale.account_revision,
                sale.tick,
                sale.fuel_at_trade,
                sale.fuel_at_trade,
            ));
        }
        for receipt in c.world.solar.receipts().filter(|receipt| {
            receipt.revision > buy.account_revision
                && self.terminal.is_none_or(|terminal| {
                    terminal
                        .account_revision
                        .map_or(receipt.tick <= terminal.tick, |revision| {
                            receipt.revision <= revision
                        })
                })
        }) {
            events.push((
                receipt.revision,
                receipt.tick,
                receipt.before,
                receipt.before.checked_add(receipt.added).ok_or(INVALID)?,
            ));
        }
        events.sort_unstable();
        let mut revision = buy.account_revision;
        let mut tick = buy.tick;
        let mut fuel = buy.fuel_at_trade;
        for (next_revision, next_tick, before, after) in events {
            if next_revision <= revision || next_tick < tick || before != fuel {
                return Err(INVALID);
            }
            revision = next_revision;
            tick = next_tick;
            fuel = after;
        }
        if fuel != end.fuel {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_journeys(&self, c: &Context<'_>) -> Result<(), SaveError> {
        if self.journeys.len() > MAX_RECEIPTS {
            return Err(INVALID);
        }
        if let Some(first) = self.journeys.first() {
            let buy = self.buy.ok_or(INVALID)?;
            if buy.jump_sequence.checked_add(1) != Some(first.jump)
                || first.origin != buy.dock.system()
            {
                return Err(INVALID);
            }
        }
        for j in &self.journeys {
            j.validate(self.buy.ok_or(INVALID)?, c)?;
            if self.terminal.is_some_and(|t| j.tick > t.tick) {
                return Err(INVALID);
            }
        }
        if self.journeys.windows(2).any(|p| {
            p[0].tick >= p[1].tick
                || p[0].jump.checked_add(1) != Some(p[1].jump)
                || p[0].destination != p[1].origin
                || p[0].arrival_visit > p[1].source_visit
        }) {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_terminal(&self, c: &Context<'_>) -> Result<(), SaveError> {
        let Some(t) = self.terminal else {
            return Ok(());
        };
        validate_dock(t.dock, c)?;
        if t.fuel.fuel > crate::navigation::MAX_FUEL_CAPACITY
            || t.fuel.jump > c.nav.jumps
            || t.fuel.refilled_units > c.commerce.markets.ok_or(INVALID)?.fuel_accounting().0
        {
            return Err(INVALID);
        }
        if let Some(revision) = t.account_revision {
            if revision > c.commerce.account.ok_or(INVALID)?.revision()
                || self
                    .accepted_account_revision
                    .is_some_and(|accepted| accepted > revision)
                || self
                    .plan
                    .is_some_and(|plan| plan.account_revision > revision)
                || self.buy.is_some_and(|buy| buy.account_revision > revision)
                || self
                    .sale
                    .is_some_and(|sale| sale.account_revision > revision)
                || self
                    .fuel
                    .iter()
                    .any(|fuel| fuel.account_revision > revision)
                || self
                    .journeys
                    .iter()
                    .any(|journey| journey.account_revision > revision)
                || t.dock.visit().checked_sub(t.dock.sequence()) != Some(t.fuel.jump)
                || c.world.xyzzy.historical_fuel(
                    c.world,
                    c.commerce.markets.ok_or(INVALID)?,
                    t.dock,
                    t.tick,
                    revision,
                )? != (t.fuel.refilled_units, t.fuel.fuel)
            {
                return Err(INVALID);
            }
        } else if self.accepted_account_revision.is_some() {
            return Err(INVALID);
        }
        if !same_carrier(self.dock, t.dock)
            || t.tick < self.accepted_tick
            || t.tick < t.dock.tick()
            || t.tick > c.world.state.tick
            || t.dock.sequence() < self.dock.sequence()
            || t.dock.visit() < self.dock.visit()
            || (t.dock.sequence() == self.dock.sequence() && t.dock != self.dock)
            || self.plan.is_some_and(|p| p.confirmed_tick > t.tick)
            || self.sale.is_some_and(|s| s.tick > t.tick)
            || self.buy.is_some_and(|b| b.tick > t.tick)
        {
            return Err(INVALID);
        }
        if t.outcome == Outcome::Complete && !self.return_receipt_ready(t.dock) {
            return Err(INVALID);
        }
        Ok(())
    }
    fn encode(&self, w: &mut Writer) {
        self.dock.encode_save(w);
        w.u64(
            self.accepted_tick
                | if self.accepted_account_revision.is_some() {
                    1 << 63
                } else {
                    0
                },
        );
        if let Some(revision) = self.accepted_account_revision {
            w.u64(revision);
        }
        w.option(self.plan.as_ref(), |w, p| p.encode(w));
        w.option(self.buy.as_ref(), |w, t| t.encode(w));
        w.u8(self.journeys.len() as u8);
        for j in &self.journeys {
            j.encode(w);
        }
        w.u8(self.fuel.len() as u8);
        for f in &self.fuel {
            f.encode(w);
        }
        w.option(self.sale.as_ref(), |w, t| t.encode(w));
        w.option(self.terminal.as_ref(), |w, t| {
            w.u64(t.tick);
            t.dock.encode_save(w);
            w.u8(match t.outcome {
                Outcome::Complete => 0,
                Outcome::Abandoned => 1,
            } | if t.account_revision.is_some() { 2 } else { 0 });
            t.fuel.encode(w);
            if let Some(revision) = t.account_revision {
                w.u64(revision);
            }
        });
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        let dock = DockContext::decode_save(r)?;
        let raw_tick = r.u64()?;
        let accepted_tick = raw_tick & !(1 << 63);
        let accepted_account_revision = if raw_tick & (1 << 63) != 0 {
            Some(r.u64()?)
        } else {
            None
        };
        let plan = r.option(Plan::decode)?;
        let buy = r.option(Trade::decode)?;
        let journeys = bounded(r, Journey::decode)?;
        let fuel = bounded(r, Fuel::decode)?;
        let sale = r.option(Trade::decode)?;
        let terminal = r.option(|r| {
            let tick = r.u64()?;
            let dock = DockContext::decode_save(r)?;
            let tag = r.u8()?;
            let outcome = match tag {
                0 | 2 => Outcome::Complete,
                1 | 3 => Outcome::Abandoned,
                _ => return Err(INVALID),
            };
            let fuel = FuelAnchor::decode(r)?;
            Ok(Terminal {
                tick,
                dock,
                outcome,
                fuel,
                account_revision: if tag & 2 != 0 { Some(r.u64()?) } else { None },
            })
        })?;
        Ok(Self {
            dock,
            accepted_tick,
            accepted_account_revision,
            plan,
            buy,
            journeys,
            fuel,
            sale,
            terminal,
        })
    }
}
fn bounded<T>(
    r: &mut Reader<'_>,
    mut read: impl FnMut(&mut Reader<'_>) -> Result<T, SaveError>,
) -> Result<Vec<T>, SaveError> {
    let count = usize::from(r.u8()?);
    if count > MAX_RECEIPTS {
        return Err(INVALID);
    }
    let mut v = Vec::with_capacity(count);
    for _ in 0..count {
        v.push(read(r)?);
    }
    Ok(v)
}
fn validate_dock(d: DockContext, c: &Context<'_>) -> Result<(), SaveError> {
    if d.owner().seed() != c.nav.owner
        || d.ship().owner() != d.owner()
        || d.ship().ordinal() != c.nav.hull
        || site(d.system()).is_none()
        || d.sequence() == 0
        || d.sequence() > c.nav.dock_sequence
        || d.visit() < d.sequence()
        || d.visit() > c.nav.visit
        || d.tick() > c.world.state.tick
        || c.world
            .current_dock_context()
            .is_some_and(|now| now.sequence() == d.sequence() && now != d)
    {
        return Err(INVALID);
    }
    match d.origin() {
        DockOrigin::InitialStart
            if d.sequence() == 1
                && d.visit() == 1
                && d.tick() == 0
                && d.system() == SystemId::AVENOR =>
        {
            Ok(())
        }
        DockOrigin::PhysicalEntry if d.sequence() > 1 && d.tick() > 0 => Ok(()),
        _ => Err(INVALID),
    }
}
impl Fit {
    fn validate(self, c: &Context<'_>) -> Result<(), SaveError> {
        let (inventory, items, revision) = c
            .commerce
            .account
            .ok_or(INVALID)?
            .owned_fit_identity()
            .ok_or(INVALID)?;
        if Some(self.inventory) != c.nav.owner.checked_add(2)
            || self.inventory != inventory
            || self.equipment_revision > revision
        {
            return Err(INVALID);
        }
        for (i, item) in self.items.into_iter().enumerate() {
            if item.is_some_and(|id| Some(id) != c.nav.owner.checked_add(3 + i as u64))
                || (i < 5 && item.is_none())
                || (item.is_none() && items[i].is_some())
            {
                return Err(INVALID);
            }
        }
        Ok(())
    }
    fn encode(self, w: &mut Writer) {
        w.u64(self.inventory);
        w.u64(self.equipment_revision);
        for id in self.items {
            w.option(id.as_ref(), |w, id| w.u64(*id));
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            inventory: r.u64()?,
            equipment_revision: r.u64()?,
            items: r.array(|r| r.option(|r| r.u64()))?,
        })
    }
}
impl Plan {
    fn validate(self, a: &Attempt, c: &Context<'_>) -> Result<(), SaveError> {
        self.fit.validate(c)?;
        let markets = c.commerce.markets.ok_or(INVALID)?;
        let train = site(a.dock.system()).ok_or(INVALID)?;
        let p = c
            .commerce
            .account
            .ok_or(INVALID)?
            .service_projection(&c.world.freight_session.as_ref().ok_or(INVALID)?.state)
            .map_err(|_| INVALID)?;
        let distance = c
            .world
            .navigation_catalogue
            .as_ref()
            .ok_or(INVALID)?
            .distance_ly(a.dock.system(), self.destination)
            .map_err(|_| INVALID)?;
        if site(self.destination).is_none()
            || self.destination == a.dock.system()
            || !legal_tonne(self.commodity)
            || self.confirmed_tick < a.accepted_tick
            || self.confirmed_tick > c.world.state.tick
            || ![92, 127].contains(&self.capacity)
            || self.cargo >= self.capacity
            || self.stock == 0
            || self.stock > MAX_MARKET_QUANTITY
            || self.credits_at_plan < self.price
            || self.distance != distance
            || distance <= 0.0
            || distance
                > c.commerce
                    .modular
                    .jump_reach_at_revision(c.nav.owner, self.account_revision)?
            || distance > f64::from(self.fuel_at_plan) / 10.0
            || self.fuel_at_plan
                > c.commerce
                    .modular
                    .fuel_capacity_at_revision(c.nav.owner, self.account_revision)?
            || self.refilled_at_plan > markets.fuel_accounting().0
            || stock_jump_fuel_cost(distance).map_err(|_| INVALID)? != self.fuel_debit
            || self.fuel_debit > self.fuel_at_plan
            || self.account_revision > p.revision
            || self.market_revision > markets.transaction_revision()
            || self.bought.saturating_add(self.sold) > self.market_revision
            || self.price != markets.commodity_quote(train, self.commodity).ask_tenths
            || stock_at(
                markets,
                &c.world.traffic,
                self.confirmed_tick,
                train,
                self.commodity,
                self.bought,
                self.sold,
            ) != Some(self.stock)
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn encode(self, w: &mut Writer) {
        write_system(w, self.destination);
        w.u8(self.commodity.index() as u8);
        w.u64(self.confirmed_tick);
        self.fit.encode(w);
        w.u16(self.capacity);
        w.u16(self.cargo);
        w.u32(self.price);
        w.u64(self.stock);
        w.u64(self.distance.to_bits());
        w.u16(self.fuel_debit);
        w.u16(self.fuel_at_plan);
        w.u64(self.refilled_at_plan);
        w.u32(self.credits_at_plan);
        w.u64(self.account_revision);
        w.u64(self.market_revision);
        w.u64(self.bought);
        w.u64(self.sold);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            destination: read_system(r)?,
            commodity: CommodityId::from_index(usize::from(r.u8()?)).ok_or(INVALID)?,
            confirmed_tick: r.u64()?,
            fit: Fit::decode(r)?,
            capacity: r.u16()?,
            cargo: r.u16()?,
            price: r.u32()?,
            stock: r.u64()?,
            distance: f64::from_bits(r.u64()?),
            fuel_debit: r.u16()?,
            fuel_at_plan: r.u16()?,
            refilled_at_plan: r.u64()?,
            credits_at_plan: r.u32()?,
            account_revision: r.u64()?,
            market_revision: r.u64()?,
            bought: r.u64()?,
            sold: r.u64()?,
        })
    }
}
fn stock_at(
    markets: &StationMarkets,
    traffic: &crate::traffic::State,
    tick: u64,
    site: crate::station_services::MarketSite,
    id: CommodityId,
    bought: u64,
    sold: u64,
) -> Option<u64> {
    let (now_bought, now_sold) = markets.commodity_counts(site, id);
    if bought > now_bought || sold > now_sold {
        return None;
    }
    let si = match site {
        crate::station_services::MarketSite::Avenor => 0,
        crate::station_services::MarketSite::Renas => 1,
        crate::station_services::MarketSite::Lareria => 2,
    };
    let now = traffic.flows();
    let past = traffic.flows_at(tick);
    let seed = u128::from(markets.commodity_quote(site, id).stock)
        .checked_add(u128::from(now_bought))?
        .checked_add(u128::from(now.bought[si][id.index()]))?
        .checked_sub(u128::from(now_sold))?
        .checked_sub(u128::from(now.sold[si][id.index()]))?;
    u64::try_from(
        seed.checked_add(u128::from(sold))?
            .checked_add(u128::from(past.sold[si][id.index()]))?
            .checked_sub(u128::from(bought))?
            .checked_sub(u128::from(past.bought[si][id.index()]))?,
    )
    .ok()
}
impl Trade {
    fn validate_common(self, plan: Plan, c: &Context<'_>) -> Result<(), SaveError> {
        validate_dock(self.dock, c)?;
        let markets = c.commerce.markets.ok_or(INVALID)?;
        let site = site(self.dock.system()).ok_or(INVALID)?;
        let account = c
            .commerce
            .account
            .ok_or(INVALID)?
            .service_projection(&c.world.freight_session.as_ref().ok_or(INVALID)?.state)
            .map_err(|_| INVALID)?;
        if self.tick < plan.confirmed_tick
            || self.tick < self.dock.tick()
            || self.tick > c.world.state.tick
            || self.transaction <= plan.market_revision
            || self.transaction > markets.transaction_revision()
            || self.account_revision <= plan.account_revision
            || self.account_revision > account.revision
            || self.jump_sequence > c.nav.jumps
            || self.fuel_at_trade
                > c.commerce
                    .modular
                    .fuel_capacity_at_revision(c.nav.owner, self.account_revision)?
            || self.after.bought.saturating_add(self.after.sold) > self.transaction
            || stock_at(
                markets,
                &c.world.traffic,
                self.tick,
                site,
                plan.commodity,
                self.before.bought,
                self.before.sold,
            ) != Some(self.before.stock)
            || stock_at(
                markets,
                &c.world.traffic,
                self.tick,
                site,
                plan.commodity,
                self.after.bought,
                self.after.sold,
            ) != Some(self.after.stock)
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_buy(
        self,
        a: &Attempt,
        plan: Plan,
        ordinal: usize,
        c: &Context<'_>,
    ) -> Result<(), SaveError> {
        self.validate_common(plan, c)?;
        if self.tonne
            != (NavigationTonneId {
                owner: a.dock.owner().seed(),
                attempt: ordinal as u8,
                transaction: self.transaction,
            })
            || !same_carrier(a.dock, self.dock)
            || self.dock.system() != a.dock.system()
            || self.dock.sequence() < a.dock.sequence()
            || self.price != plan.price
            || self.before.stock != plan.stock
            || self.before.bought != plan.bought
            || self.before.sold != plan.sold
            || self.before.stock.checked_sub(1) != Some(self.after.stock)
            || self.before.cargo != 0
            || self.after.cargo != 1
            || self.before.credits.checked_sub(self.price) != Some(self.after.credits)
            || self.before.bought.checked_add(1) != Some(self.after.bought)
            || self.before.sold != self.after.sold
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn validate_sale(self, a: &Attempt, plan: Plan, c: &Context<'_>) -> Result<(), SaveError> {
        self.validate_common(plan, c)?;
        let buy = a.buy.ok_or(INVALID)?;
        let selected = a.selected_journey().ok_or(INVALID)?;
        let arrived = a
            .journeys
            .iter()
            .rfind(|j| j.tick <= self.tick)
            .ok_or(INVALID)?;
        let quote = c
            .commerce
            .markets
            .ok_or(INVALID)?
            .commodity_quote(site(plan.destination).ok_or(INVALID)?, plan.commodity);
        if self.tonne != buy.tonne
            || self.transaction <= buy.transaction
            || self.account_revision <= buy.account_revision
            || self.tick < selected.tick
            || !same_carrier(a.dock, self.dock)
            || self.dock.system() != plan.destination
            || self.dock.origin() != DockOrigin::PhysicalEntry
            || self.dock.visit() != arrived.arrival_visit
            || arrived.destination != plan.destination
            || self.jump_sequence != arrived.jump
            || self.price != quote.bid_tenths
            || self.before.stock.checked_add(1) != Some(self.after.stock)
            || self.before.cargo.checked_sub(1) != Some(self.after.cargo)
            || self.before.credits.checked_add(self.price) != Some(self.after.credits)
            || self.before.sold.checked_add(1) != Some(self.after.sold)
            || self.before.bought != self.after.bought
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn encode(self, w: &mut Writer) {
        self.tonne.encode(w);
        self.dock.encode_save(w);
        w.u64(self.tick);
        w.u64(self.transaction);
        w.u64(self.account_revision);
        w.u64(self.jump_sequence);
        w.u16(self.fuel_at_trade);
        w.u32(self.price);
        self.before.encode(w);
        self.after.encode(w);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            tonne: NavigationTonneId::decode(r)?,
            dock: DockContext::decode_save(r)?,
            tick: r.u64()?,
            transaction: r.u64()?,
            account_revision: r.u64()?,
            jump_sequence: r.u64()?,
            fuel_at_trade: r.u16()?,
            price: r.u32()?,
            before: Balance::decode(r)?,
            after: Balance::decode(r)?,
        })
    }
}
impl Journey {
    fn validate(self, buy: Trade, c: &Context<'_>) -> Result<(), SaveError> {
        let distance = c
            .world
            .navigation_catalogue
            .as_ref()
            .ok_or(INVALID)?
            .distance_ly(self.origin, self.destination)
            .map_err(|_| INVALID)?;
        let account_revision = c
            .commerce
            .account
            .ok_or(INVALID)?
            .service_projection(&c.world.freight_session.as_ref().ok_or(INVALID)?.state)
            .map_err(|_| INVALID)?
            .revision;
        if self.account_revision <= buy.account_revision
            || self.account_revision > account_revision
            || self.origin == self.destination
            || distance
                > c.commerce
                    .modular
                    .jump_reach_at_revision(c.nav.owner, self.account_revision)?
            || distance > f64::from(self.fuel_before) / 10.0
            || self.tick <= buy.tick
            || self.tick > c.world.state.tick
            || self.jump == 0
            || self.jump > c.nav.jumps
            || self.source_visit <= buy.dock.visit()
            || self.source_visit.checked_add(1) != Some(self.arrival_visit)
            || self.arrival_visit > c.nav.visit
            || self.fuel_before
                > c.commerce
                    .modular
                    .fuel_capacity_at_revision(c.nav.owner, self.account_revision)?
            || self.fuel_before.checked_sub(self.debit) != Some(self.fuel_after)
            || self.debit == 0
            || stock_jump_fuel_cost(distance).map_err(|_| INVALID)? != self.debit
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn encode(self, w: &mut Writer) {
        w.u64(self.tick);
        w.u64(self.account_revision);
        w.u64(self.jump);
        write_system(w, self.origin);
        write_system(w, self.destination);
        w.u64(self.source_visit);
        w.u64(self.arrival_visit);
        w.u16(self.fuel_before);
        w.u16(self.fuel_after);
        w.u16(self.debit);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            tick: r.u64()?,
            account_revision: r.u64()?,
            jump: r.u64()?,
            origin: read_system(r)?,
            destination: read_system(r)?,
            source_visit: r.u64()?,
            arrival_visit: r.u64()?,
            fuel_before: r.u16()?,
            fuel_after: r.u16()?,
            debit: r.u16()?,
        })
    }
}
impl Fuel {
    fn validate(self, plan: Plan, c: &Context<'_>) -> Result<(), SaveError> {
        validate_dock(self.dock, c)?;
        let markets = c.commerce.markets.ok_or(INVALID)?;
        let account = c
            .commerce
            .account
            .ok_or(INVALID)?
            .service_projection(&c.world.freight_session.as_ref().ok_or(INVALID)?.state)
            .map_err(|_| INVALID)?;
        let capacity = c
            .commerce
            .modular
            .fuel_capacity_at_revision(c.nav.owner, self.account_revision)?;
        if self.tick < plan.confirmed_tick
            || self.tick < self.dock.tick()
            || self.tick > c.world.state.tick
            || self.transaction <= plan.market_revision
            || self.transaction > markets.transaction_revision()
            || self.account_revision <= plan.account_revision
            || self.account_revision > account.revision
            || self.ordinal == 0
            || self.ordinal > markets.fuel_accounting().1
            || self.units_before >= capacity
            || self.units_after != capacity
            || self.paid != u32::from(capacity - self.units_before) * 2
            || self.credits_before.checked_sub(self.paid) != Some(self.credits_after)
        {
            return Err(INVALID);
        }
        Ok(())
    }
    fn encode(self, w: &mut Writer) {
        w.u64(self.tick);
        self.dock.encode_save(w);
        w.u64(self.transaction);
        w.u64(self.account_revision);
        w.u64(self.ordinal);
        w.u16(self.units_before);
        w.u16(self.units_after);
        w.u32(self.credits_before);
        w.u32(self.credits_after);
        w.u32(self.paid);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            tick: r.u64()?,
            dock: DockContext::decode_save(r)?,
            transaction: r.u64()?,
            account_revision: r.u64()?,
            ordinal: r.u64()?,
            units_before: r.u16()?,
            units_after: r.u16()?,
            credits_before: r.u32()?,
            credits_after: r.u32()?,
            paid: r.u32()?,
        })
    }
}
impl NavigationTonneId {
    fn encode(self, w: &mut Writer) {
        w.u64(self.owner);
        w.u8(self.attempt);
        w.u64(self.transaction);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            owner: r.u64()?,
            attempt: r.u8()?,
            transaction: r.u64()?,
        })
    }
}
impl Balance {
    fn encode(self, w: &mut Writer) {
        w.u64(self.stock);
        w.u64(self.cargo);
        w.u32(self.credits);
        w.u64(self.bought);
        w.u64(self.sold);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            stock: r.u64()?,
            cargo: r.u64()?,
            credits: r.u32()?,
            bought: r.u64()?,
            sold: r.u64()?,
        })
    }
}
fn write_system(w: &mut Writer, s: SystemId) {
    w.u8(s.galaxy);
    w.u16(s.system);
}
fn read_system(r: &mut Reader<'_>) -> Result<SystemId, SaveError> {
    Ok(SystemId {
        galaxy: r.u8()?,
        system: r.u16()?,
    })
}

impl FuelAnchor {
    fn encode(self, w: &mut Writer) {
        w.u64(self.refilled_units);
        w.u16(self.fuel);
        w.u64(self.jump);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, SaveError> {
        Ok(Self {
            refilled_units: r.u64()?,
            fuel: r.u16()?,
            jump: r.u64()?,
        })
    }
}

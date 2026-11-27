//! Presentation of opaque T04 reviews and immutable World snapshots.
use meridian_core::{
    NavigationTradeAction, NavigationTradeReview, NavigationTradeSnapshot, World,
    navigation::SystemId, station_services::CommodityId,
};

fn system_name(id: SystemId) -> &'static str {
    match id {
        SystemId::AVENOR => "AVENOR",
        SystemId::RENAS => "RENAS",
        SystemId::LARERIA => "LARERIA",
        _ => "UNAVAILABLE",
    }
}

pub fn review_title(action: NavigationTradeAction) -> &'static str {
    match action {
        NavigationTradeAction::Accept => "TRADE LESSON CONSENT",
        NavigationTradeAction::Plan { .. } => "TRADE LESSON PLAN",
        NavigationTradeAction::Receive => "TRADE LESSON RECEPTION",
        NavigationTradeAction::Cancel => "CANCEL TRADE LESSON",
    }
}

pub fn append_review(review: NavigationTradeReview, lines: &mut Vec<String>) {
    let terms = match review.action() {
        NavigationTradeAction::Accept => {
            "Accept trade practice here. On the galaxy chart, 7 chooses a destination. In the lesson, [ / ] selects goods and R reviews the route, stock, price, hold and fuel. No lesson fee or cash reward."
        }
        NavigationTradeAction::Plan { .. } => {
            "Confirm this selected direct route and one tonne of this commodity using the current owned fit and stock. Reserve one hold tonne without buying it. G makes the ordinary purchase; travel, sell the same original tonne at the destination and return here. Trade may lose money."
        }
        NavigationTradeAction::Receive => {
            "Receive the actual original-tonne trade and return. Settle zero lesson cash and retain the first Navigation Trade Family Trace once. Ordinary purchases, sales and fuel payments remain charged."
        }
        NavigationTradeAction::Cancel => {
            "Cancel this trade lesson. Release its remaining reservation or held original tonne for ordinary use. Keep all history and actual expenses; no refund, restock or clue. A new attempt requires fresh consent."
        }
    };
    crate::session::append_paragraph(lines, terms);
    if let Some(plan) = review.plan() {
        let name = plan
            .commodity
            .definition()
            .map(|d| d.name)
            .unwrap_or("UNAVAILABLE");
        lines.push(format!(
            "SELECTED {name} | DESTINATION {} | {:.2} LY",
            system_name(plan.destination),
            plan.distance_ly
        ));
        lines.push(format!(
            "BUY {}.{} CR | STOCK {} | HOLD {}/{}T | FUEL {}.{}",
            plan.price_tenths / 10,
            plan.price_tenths % 10,
            plan.stock,
            plan.cargo_tonnes,
            plan.capacity_tonnes,
            plan.fuel_debit / 10,
            plan.fuel_debit % 10
        ));
    }
    lines.push("ENTER CONFIRM AND SAVE | B BACK | 7 DETAILS".into());
}

fn objective(
    state: &NavigationTradeSnapshot,
    berth: Option<SystemId>,
    selected: CommodityId,
    destination: Option<SystemId>,
) -> String {
    let Some(attempt) = state
        .attempts
        .last()
        .filter(|attempt| Some(attempt.id) == state.active)
    else {
        return "At Avenor, Renas or Lareria main orbital, press R to review trade practice."
            .into();
    };
    let issuer = attempt.issuing_dock.system();
    let home = system_name(issuer);
    let name = |id: CommodityId| id.definition().map_or("UNAVAILABLE", |d| d.name);
    if state.ready {
        return "Press R for the debrief. Enter confirms the trade and saves.".into();
    }
    let Some(plan) = attempt.plan else {
        if berth != Some(issuer) {
            return format!("Return to {home} to review the trade plan.");
        }
        return format!(
            "At {home}, select a legal tonne with [ / ]. R reviews the plan for {} to {}.",
            name(selected),
            destination.map_or("UNAVAILABLE", system_name)
        );
    };
    let goods = name(plan.commodity);
    if attempt.buy.is_none() {
        return format!("At {home}, select {goods} and buy one tonne with G.");
    }
    if attempt.sale.is_some() {
        return format!("Return to {home} main orbital for the trade debrief.");
    }
    if berth == Some(plan.destination) {
        format!("Select {goods} and use Shift+G to sell the original tonne here.")
    } else {
        format!(
            "Deliver the original tonne of {goods} to {} main orbital.",
            system_name(plan.destination)
        )
    }
}

pub fn travel_target(world: &World, current: SystemId) -> Option<SystemId> {
    let state = world.navigation_trade_snapshot()?;
    let attempt = state
        .attempts
        .last()
        .filter(|a| Some(a.id) == state.active)?;
    let plan = attempt.plan?;
    let issuer = attempt.issuing_dock.system();
    if attempt.held_tonne.is_some() && current == issuer {
        Some(plan.destination)
    } else if attempt.sale.is_some() && current == plan.destination {
        Some(issuer)
    } else {
        None
    }
}

pub fn append_details(
    world: &World,
    selected: CommodityId,
    destination: Option<SystemId>,
    lines: &mut Vec<String>,
) {
    let Some(state) = world.navigation_trade_snapshot() else {
        return;
    };
    let berth = world.current_dock_context().map(|dock| dock.system());
    crate::session::append_paragraph(lines, &objective(&state, berth, selected, destination));
    if state.active.is_none()
        && let Some(target) = destination
    {
        lines.push(format!(
            "DESTINATION {} | GALAXY CHART: 7 CHOOSES ANOTHER",
            system_name(target)
        ));
    }
    lines.push("7 TRADE GOAL | R REVIEW | 6 REVIEW CANCELLATION | F REFUEL".into());
    lines.push(format!(
        "RECORDS {} | ACTIVE SERVICES {} | RESERVED HOLD {}T",
        state.retained_mission_records, state.active_services, state.reserved_capacity_tonnes
    ));
    if let Some(attempt) = state.attempts.last() {
        lines.push(format!(
            "ATTEMPT {} | ISSUING {} | OUTCOME {:?}",
            attempt.id.ordinal(),
            system_name(attempt.issuing_dock.system()),
            attempt.outcome
        ));
        if let Some(plan) = attempt.plan {
            let name = plan
                .commodity
                .definition()
                .map(|d| d.name)
                .unwrap_or("UNAVAILABLE");
            lines.push(format!(
                "{name} | DESTINATION {} | BUY QUOTE {}.{} CR",
                system_name(plan.destination),
                plan.price_tenths / 10,
                plan.price_tenths % 10
            ));
        }
        lines.push(format!(
            "ORIGINAL PURCHASE {} | HELD {} | SELECTED SALE {}",
            attempt.buy.is_some(),
            attempt.held_tonne.is_some(),
            attempt.sale.is_some()
        ));
        lines.push(format!(
            "ADMITTED JOURNEYS {} | FUEL USED {} | FUEL PAID {}.{} CR",
            attempt.journeys,
            attempt.fuel_units_used,
            attempt.fuel_paid_tenths / 10,
            attempt.fuel_paid_tenths % 10
        ));
        if let Some(net) = attempt.cash_difference_tenths {
            lines.push(format!(
                "NET ORDINARY CASH {net} TENTHS CR | LESSON PAYMENT ZERO"
            ));
        }
    }
    lines.push(
        if state.trace.is_some() {
            "NAVIGATION TRADE FAMILY TRACE RETAINED ONCE"
        } else {
            "NAVIGATION TRADE FAMILY TRACE NOT YET RECORDED"
        }
        .into(),
    );
}

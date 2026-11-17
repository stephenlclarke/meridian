//! Presentation of opaque T04 reviews and immutable World snapshots.
use meridian_core::{NavigationTradeAction, NavigationTradeReview, World, navigation::SystemId};

fn system_name(id: SystemId) -> &'static str {
    match id {
        SystemId::AVENOR => "AVENOR",
        SystemId::RENAS => "RENAS",
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
            "Consent to ordinary navigation and trade. This does not buy cargo or choose a plan. After confirmation, use [ / ] to select a legal tonne commodity, then 5 reviews its direct route and current fit, stock, price, hold and fuel. No lesson fee or cash reward."
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

pub fn append_details(world: &World, lines: &mut Vec<String>) {
    lines
        .push("TRADE LESSON | 4 CONSENT / RECEIVE | 5 PLAN SELECTED | 6 CANCEL | 7 DETAILS".into());
    lines.push("[ / ] SELECT COMMODITY | G BUY | SHIFT+G SELL | J DIRECT TRAVEL | F REFUEL".into());
    let Some(state) = world.navigation_trade_snapshot() else {
        return;
    };
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
        } else {
            lines.push("NO PLAN CONFIRMED | SELECT WITH [ / ] THEN 5 REVIEWS".into());
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
        if state.ready {
            "4 REVIEW RECEPTION AT THE ISSUING BERTH"
        } else {
            "COMPLETE THE ACTUAL PLAN, ORIGINAL SALE AND RETURN BEFORE RECEPTION"
        }
        .into(),
    );
    lines.push(
        if state.trace.is_some() {
            "NAVIGATION TRADE FAMILY TRACE RETAINED ONCE"
        } else {
            "NAVIGATION TRADE FAMILY TRACE NOT YET RECORDED"
        }
        .into(),
    );
}

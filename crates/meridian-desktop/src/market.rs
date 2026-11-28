//! Read-only ordinary market presentation. Prices, cargo and admission come from World.
use meridian_core::World;
use meridian_core::station_services::{self, CommodityId};

pub fn content_error(_: station_services::Error) -> String {
    "Original commodity data is unavailable; keep the current commander and report the error."
        .into()
}

pub fn append_panel(
    world: &World,
    selected_commodity: CommodityId,
    lines: &mut Vec<String>,
) -> Result<(), String> {
    let order = CommodityId::display_order().map_err(content_error)?;
    let selected = order
        .iter()
        .position(|id| *id == selected_commodity)
        .ok_or("Selected commodity is absent from the original market order")?;
    let service = world
        .service_snapshot()
        .ok_or("No commander market account")?;
    let credit = world.credits_tenths().ok_or("No commander wallet")?;
    lines.push(format!(
        "CREDITS {}.{} CR | HOLD {}/{}T | LEGAL VALUE {}",
        credit / 10,
        credit % 10,
        service.used_tonnes,
        service.hold_capacity_tonnes,
        world.legal_value().ok_or("No commander legal record")?
    ));
    let definition = selected_commodity.definition().map_err(content_error)?;
    lines.push(format!(
        "SELECTED: {} | NATIVE UNIT {} | {}/17",
        definition.name,
        definition.unit.symbol(),
        selected + 1
    ));
    lines.push("  COMMODITY                UNIT | BUY CR / SELL CR | STOCK".into());
    // Five source-ordered rows keep selection, actual quotes and controls visible at 640x360.
    let start = selected.saturating_sub(2).min(order.len() - 5);
    for &id in &order[start..start + 5] {
        let definition = id.definition().map_err(content_error)?;
        let quote = world.commodity_quote(id);
        let prices = quote
            .map(|q| {
                format!(
                    "{}.{:01} / {}.{:01} | {}",
                    q.ask_tenths / 10,
                    q.ask_tenths % 10,
                    q.bid_tenths / 10,
                    q.bid_tenths % 10,
                    q.stock
                )
            })
            .unwrap_or_else(|| "BERTH REQUIRED".into());
        lines.push(format!(
            "{} {:22} {:4} | {prices}",
            if id == selected_commodity { ">" } else { " " },
            definition.name,
            definition.unit.symbol()
        ));
    }
    let held = world
        .commodity_quantity(selected_commodity)
        .ok_or("No commander cargo account")?;
    let reserved = world
        .reserved_commodity_quantity(selected_commodity)
        .ok_or("No commander cargo reservations")?;
    lines.push(format!(
        "HELD {held} {} | RESERVED {reserved} {} | KG/G USE ZERO HOLD TONNES",
        definition.unit.symbol(),
        definition.unit.symbol()
    ));
    lines.push("M CLOSE | I MISSION | Ctrl+S SAVE | Ctrl+R RELOAD | ESC SAVE AND QUIT".into());
    Ok(())
}

pub fn service_message(event: station_services::Event) -> Result<String, String> {
    let (verb, commodity, cash) = match event {
        station_services::Event::BoughtFood { paid_tenths, .. } => {
            ("Bought", CommodityId::Food, paid_tenths)
        }
        station_services::Event::SoldFood {
            credited_tenths, ..
        } => ("Sold", CommodityId::Food, credited_tenths),
        station_services::Event::BoughtCommodity {
            commodity,
            paid_tenths,
            ..
        } => ("Bought", commodity, paid_tenths),
        station_services::Event::SoldCommodity {
            commodity,
            credited_tenths,
            ..
        } => ("Sold", commodity, credited_tenths),
        station_services::Event::Refuelled { paid_tenths, .. } => {
            return Ok(format!(
                "Paid {}.{} CR for fuel.",
                paid_tenths / 10,
                paid_tenths % 10
            ));
        }
    };
    let definition = commodity.definition().map_err(content_error)?;
    Ok(format!(
        "{verb} 1 {} {} for {}.{} CR.",
        definition.unit.symbol(),
        definition.name,
        cash / 10,
        cash % 10
    ))
}

pub fn commodity_trade_message(
    event: station_services::CommodityTradeEvent,
) -> Result<String, String> {
    let definition = event.commodity.definition().map_err(content_error)?;
    let (verb, amount) = match event.side {
        station_services::TradeSide::Buy => ("Bought", event.paid_tenths),
        station_services::TradeSide::Sell => ("Sold", event.credited_tenths),
    };
    Ok(format!(
        "{verb} {} {} {} for {}.{} CR.",
        event.quantity,
        definition.unit.symbol(),
        definition.name,
        amount / 10,
        amount % 10
    ))
}

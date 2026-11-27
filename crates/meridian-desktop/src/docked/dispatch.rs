use super::{base_row, ui};
use meridian_core::{ModuleAction, World, freight::DispatchAction, navigation::SystemId};

pub(super) const BOARD: &str = "sealed-dispatch";
pub(super) const TITLE: &str = "G01 sealed dispatch board";
const ACTIONS: [(&str, DispatchAction); 5] = [
    ("g01-package-1", DispatchAction::Collect(1)),
    ("g01-package-2", DispatchAction::Collect(3)),
    ("g01-package-3", DispatchAction::Collect(5)),
    ("g01-receive", DispatchAction::Receive),
    ("g01-abandon", DispatchAction::Abandon),
];
pub(super) fn action(node: &str) -> Option<DispatchAction> {
    ACTIONS
        .iter()
        .find(|(id, _)| *id == node)
        .map(|(_, action)| *action)
}
pub(super) fn name(system: SystemId) -> &'static str {
    match system {
        SystemId::AVENOR => "Avenor",
        SystemId::LARERIA => "Lareria",
        SystemId::RENAS => "Renas",
        _ => "Unavailable",
    }
}
pub(super) fn rows(world: &World) -> Vec<ui::EquipmentRow> {
    let records = world.dispatch_records();
    let held = records
        .last()
        .filter(|r| r.terminal.is_none() && !r.lost_with_carrier);
    let offers = world.dispatch_offers();
    let mut rows = Vec::new();
    for (node, action) in ACTIONS {
        let (terms, label) = match action {
            DispatchAction::Collect(row) => {
                let Some(offer) = offers.iter().find(|o| o.row == row) else {
                    continue;
                };
                (offer, format!("Collect sealed package {}", row.div_ceil(2)))
            }
            DispatchAction::Receive | DispatchAction::Abandon => {
                let Some(record) = held else { continue };
                (
                    &record.offer,
                    if action == DispatchAction::Receive {
                        "Receiving officer / hand over original".into()
                    } else {
                        "Sender / abandon for zero fee".into()
                    },
                )
            }
        };
        let path = terms
            .path
            .iter()
            .map(|s| name(*s))
            .collect::<Vec<_>>()
            .join(" > ");
        let mut row = base_row(node, &label, TITLE);
        row.readout
            .push(format!("Route: {path} | Fee: {} CR", terms.fee_tenths / 10));
        row.readout
            .push("One sealed tonne. No deposit or time limit.".into());
        row.readout
            .push("Fly the quoted legs. Visit the Sealed Dispatch Receiving Officer.".into());
        if let Some(record) = held {
            row.readout.push(format!(
                "Original record {} | sender {}",
                record.issue_revision,
                name(record.issuer.system())
            ));
        }
        if let Some(previous) = records.iter().rev().find(|r| r.terminal.is_some()) {
            let paid = if previous
                .terminal
                .is_some_and(|(action, _)| action == DispatchAction::Receive)
            {
                previous.offer.fee_tenths / 10
            } else {
                0
            };
            row.readout.push(format!(
                "Last record {} closed | {} CR paid",
                previous.issue_revision, paid
            ));
        }
        let review = world.review_module(ModuleAction::Dispatch(action), world.snapshot().tick);
        row.can_buy = review.is_ok();
        row.refusal = review.err().map(|e| e.to_string());
        rows.push(row);
    }
    if rows.is_empty() {
        let mut row = base_row("g01-unavailable", "No reachable dispatch route", TITLE);
        row.refusal = Some("Check the fitted navigation range and available fuel.".into());
        rows.push(row);
    }
    rows
}

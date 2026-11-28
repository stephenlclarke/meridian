//! TOREN's Contract Board and workshop choices.
use super::{base_row, dispatch::name, ui};
use meridian_core::{ModuleAction, World, freight::BondedAction};

pub(super) const BOARD: &str = "bonded-freight";
pub(super) const TITLE: &str = "G03 TOREN bonded freight";
const ACTIONS: [(&str, &str, BondedAction); 6] = [
    (
        "g03-h0",
        "Bonded machinery / premium H0",
        BondedAction::Accept(0),
    ),
    (
        "g03-h1",
        "Bonded machinery / premium H1",
        BondedAction::Accept(1),
    ),
    (
        "g03-h2",
        "Bonded machinery / premium H2",
        BondedAction::Accept(2),
    ),
    (
        "g03-one",
        "Receive one original tonne",
        BondedAction::ReceiveOne,
    ),
    (
        "g03-all",
        "Receive all remaining original tonnes",
        BondedAction::ReceiveAll,
    ),
    (
        "g03-abandon",
        "Return consignment at sender",
        BondedAction::Abandon,
    ),
];
pub(super) fn action(node: &str) -> Option<BondedAction> {
    ACTIONS
        .iter()
        .find(|(id, _, _)| *id == node)
        .map(|(_, _, action)| *action)
}
pub(super) fn rows(world: &World) -> Vec<ui::EquipmentRow> {
    let records = world.bonded_records();
    let active = records.last().filter(|r| r.remaining() > 0);
    let offers = world.bonded_offers();
    let mut rows = Vec::new();
    for (node, label, action) in ACTIONS {
        let offer = if let BondedAction::Accept(tier) = action {
            let Some(offer) = offers.iter().find(|o| o.tier == tier) else {
                continue;
            };
            offer
        } else {
            let Some(record) = active else {
                continue;
            };
            &record.offer
        };
        let path = offer
            .path
            .iter()
            .map(|s| name(*s))
            .collect::<Vec<_>>()
            .join(" > ");
        let mut row = base_row(node, label, TITLE);
        row.readout.push(format!("TOREN workshop route: {path}"));
        row.readout.push(format!(
            "H{} | Fee {} CR + 75 CR bond refund on final receipt",
            offer.tier,
            offer.fee_tenths / 10
        ));
        if let Some(record) = active {
            row.readout.push(format!(
                "Issue {} | {} of 3 original tonnes aboard",
                record.issue_revision,
                record.remaining()
            ));
            if action == BondedAction::Abandon {
                row.readout.push(
                    "Return the remaining tonnes at the sender. The 75 CR bond is forfeited."
                        .into(),
                );
            }
        } else {
            row.readout
                .push("Accept: 3 t free hold and 75 CR bond. No deadline.".into());
        }
        let review = world.review_module(ModuleAction::Bonded(action), world.snapshot().tick);
        row.can_buy = review.is_ok();
        row.refusal = review.err().map(|e| e.to_string());
        rows.push(row);
    }
    if rows.is_empty() {
        let mut row = base_row("g03-unavailable", "No TOREN offer available", TITLE);
        row.refusal =
            Some("Check the active freight consignment, fitted range and available fuel.".into());
        rows.push(row);
    }
    if let Some(previous) = records.last().filter(|r| r.remaining() == 0) {
        let settlement = if previous.complete() {
            format!(
                "Last delivery: {} CR fee and 75 CR bond returned.",
                previous.offer.fee_tenths / 10
            )
        } else {
            "Last consignment closed. Bond forfeited.".into()
        };
        for row in &mut rows {
            row.readout.push(settlement.clone());
        }
    }
    rows
}

//! A successful visible Store may defer one read-only Fit review until normal frame preparation.
use super::*;
pub(super) struct Handoff {
    item: u64,
    node: String,
    slot: String,
    anchor: Anchor,
    requested: bool,
}
impl Docked {
    pub(super) fn remember_stored_fit(&mut self, world: &World, action: Action) {
        self.stored_fit = None;
        let Action::Module(ModuleAction::Store { item }) = action else {
            return;
        };
        if self.screen() != ui::Screen::Equipment {
            return;
        }
        let Some(modules) = world.modular_equipment_snapshot() else {
            return;
        };
        let Some(stored) = modules
            .items
            .iter()
            .find(|m| m.id == item && m.slot.is_none())
        else {
            return;
        };
        let Ok(anchor) = Anchor::capture(world) else {
            return;
        };
        self.stored_fit = Some(Handoff {
            item,
            node: stored.kind.node().into(),
            slot: self.slot.clone(),
            anchor,
            requested: false,
        });
    }
    fn stored_fit_current(&self, world: &World, handoff: &Handoff) -> bool {
        self.active(world)
            && self.screen() == ui::Screen::Equipment
            && self.pending.is_none()
            && self.slot == handoff.slot
            && Anchor::capture(world).is_ok_and(|anchor| anchor == handoff.anchor)
    }
    pub(super) fn queue_stored_fit(&mut self, world: &World, input: &ui::Input) -> bool {
        let Some(mut handoff) = self.stored_fit.take() else {
            return false;
        };
        // This queues no confirmation or world command. Any other input discards the handoff.
        if !matches!(input, ui::Input::Key(ui::Key::Fit))
            || !self.stored_fit_current(world, &handoff)
        {
            return false;
        }
        handoff.requested = true;
        self.stored_fit = Some(handoff);
        true
    }
    pub(super) fn prepare_stored_fit(&mut self, world: &World) {
        // The handoff expires at the next normal build, even if no Y arrived.
        let Some(handoff) = self.stored_fit.take().filter(|h| h.requested) else {
            return;
        };
        if !self.stored_fit_current(world, &handoff) {
            self.feedback = "Values changed; open a fresh review".into();
            return;
        }
        if let Err(error) = self.review_equipment(
            world,
            handoff.node,
            Some(handoff.item),
            handoff.slot,
            ui::EquipmentOperation::Fit,
            None,
        ) {
            self.feedback = error;
        }
    }
}

#[cfg(test)]
mod tests;

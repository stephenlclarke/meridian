//! Fit the three additional Fleet Medusa mounts from the original hull-issue receipt.
use super::*;

impl ModuleSlot {
    pub fn laser(facing: FlightView) -> Self {
        match facing {
            FlightView::Front => Self::FrontLaser,
            FlightView::Left => Self::LeftLaser,
            FlightView::Right => Self::RightLaser,
            FlightView::Rear => Self::RearLaser,
        }
    }

    pub fn laser_facing(self) -> Option<FlightView> {
        match self {
            Self::FrontLaser => Some(FlightView::Front),
            Self::LeftLaser => Some(FlightView::Left),
            Self::RightLaser => Some(FlightView::Right),
            Self::RearLaser => Some(FlightView::Rear),
            _ => None,
        }
    }
}

pub(super) fn tag(facing: FlightView) -> u8 {
    match facing {
        FlightView::Front => 0,
        FlightView::Left => 1,
        FlightView::Right => 2,
        FlightView::Rear => 3,
    }
}

pub(super) fn facing(tag: u8) -> Result<FlightView, SaveError> {
    match tag {
        0 => Ok(FlightView::Front),
        1 => Ok(FlightView::Left),
        2 => Ok(FlightView::Right),
        3 => Ok(FlightView::Rear),
        _ => Err(SaveError::InvalidEncoding),
    }
}

pub(in crate::flight) fn issued_laser_id(
    dock: DockContext,
    revision: u64,
    facing: FlightView,
) -> Option<u64> {
    let index = tag(facing).checked_sub(1)?;
    // Preserve the identities used in existing Medusa emission receipts.
    let offset = revision
        .checked_mul(3)?
        .checked_add(500_011 + u64::from(index))?;
    let owner = dock.owner().seed();
    owner
        .checked_add(offset)
        .or_else(|| owner.checked_sub(offset))
}

impl State {
    pub(in crate::flight) fn bind_mount_issue(&mut self, issue: Option<(DockContext, u64, u64)>) {
        self.mount_issue = issue;
    }

    pub(super) fn born_ledger(&self, owner: u64) -> Result<Ledger, SaveError> {
        let mut ledger = Ledger::born(owner)?;
        ledger.pending_mount_issue = self.mount_issue;
        Ok(ledger)
    }
}

impl Ledger {
    pub(super) fn issue_mounts_through(&mut self, revision: u64) -> Result<(), ModuleError> {
        let Some((dock, tick, account_revision)) = self.pending_mount_issue else {
            return Ok(());
        };
        if revision < account_revision {
            return Ok(());
        }
        for facing in [FlightView::Left, FlightView::Right, FlightView::Rear] {
            let id = issued_laser_id(dock, account_revision, facing)
                .ok_or(ModuleError::SequenceExhausted)?;
            self.items.push(OwnedModule {
                id,
                kind: ModuleKind::NavalLaser,
                slot: Some(ModuleSlot::laser(facing)),
                provenance: ModuleProvenance::MedusaIssue {
                    dock,
                    tick,
                    account_revision,
                },
            });
        }
        self.pending_mount_issue = None;
        self.has_side_mounts = true;
        Ok(())
    }

    pub(super) fn laser_slot(
        &self,
        kind: ModuleKind,
        facing: FlightView,
    ) -> Result<ModuleSlot, ModuleError> {
        if kind.laser_profile().is_none() || (facing != FlightView::Front && !self.has_side_mounts)
        {
            return Err(ModuleError::Capacity);
        }
        Ok(ModuleSlot::laser(facing))
    }

    pub(super) fn item_is_fitted(&self, id: u64, slot: ModuleSlot) -> bool {
        self.items.iter().any(|item| {
            item.id == id
                && if slot == ModuleSlot::FrontLaser {
                    item.slot.and_then(ModuleSlot::laser_facing).is_some()
                } else {
                    item.slot == Some(slot)
                }
        })
    }
}

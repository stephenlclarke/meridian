//! G01 sealed dispatch at the currently admitted original main orbitals.
use super::{Event, Ledger, ModuleAction, ModuleError, State, World};
use crate::navigation::{DockContext, DockOrigin, OriginalNavigation, SystemId, fuel_capacity};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

// The saved collect tag selects this immutable route graph. New issuers require a new generation.
const MARKETS_V1: [SystemId; 3] = [SystemId::AVENOR, SystemId::LARERIA, SystemId::RENAS];

pub const DISPATCH_DIGEST: &str =
    "8cd3f7b667d3842cbe2c0f4a6acb9d304fc4bede2fce66573169569e2ed4fcb3";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchAction {
    Collect(u8),
    Receive,
    Abandon,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchOffer {
    pub row: u8,
    pub path: Vec<SystemId>,
    pub fee_tenths: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchRecord {
    pub issuer: DockContext,
    pub issue_revision: u64,
    pub offer: DispatchOffer,
    pub terminal: Option<(DispatchAction, DockContext)>,
}

#[derive(Default)]
pub(super) struct Dispatch {
    pub(super) records: Vec<DispatchRecord>,
    pub(super) events: u64,
    pub(super) earnings: u32,
}

impl Dispatch {
    pub(super) fn active(&self) -> Option<&DispatchRecord> {
        self.records
            .last()
            .filter(|record| record.terminal.is_none())
    }
    fn offers(&self, ledger: &Ledger, dock: DockContext, fuel: u16) -> Vec<DispatchOffer> {
        if self.active().is_some()
            || !MARKETS_V1.contains(&dock.system())
            || fuel > fuel_capacity(ledger.fuel_storage_count())
        {
            return Vec::new();
        }
        let revision = self
            .records
            .iter()
            .filter(|r| r.issuer.system() == dock.system())
            .count()
            * 2;
        let key = format!(
            "contract-board-v1:{DISPATCH_DIGEST}/sealed-dispatch/main-orbital:0:{}:{revision}",
            dock.system().system
        );
        let hash = Sha256::digest(key.as_bytes());
        let mut seed = u32::from_be_bytes([hash[4], hash[5], hash[6], hash[7]]);
        if seed == 0 {
            seed = 0x6d2b79f5;
        }
        static CATALOGUE: OnceLock<Option<OriginalNavigation>> = OnceLock::new();
        let Some(nav) = CATALOGUE
            .get_or_init(|| OriginalNavigation::bundled().ok())
            .as_ref()
        else {
            return Vec::new();
        };
        let mut paths = vec![vec![dock.system()]];
        let mut cursor = 0;
        while cursor < paths.len() {
            let path = paths[cursor].clone();
            cursor += 1;
            let Some(&from) = path.last() else { continue };
            let available = if path.len() == 1 {
                fuel
            } else {
                fuel_capacity(ledger.fuel_storage_count())
            };
            let reach = ledger.jump_reach_ly().min(f64::from(available) / 10.0);
            if path.len() >= 4 {
                continue;
            }
            for target in MARKETS_V1 {
                if paths.iter().any(|p| p.last() == Some(&target)) {
                    continue;
                }
                if nav
                    .distance_ly(from, target)
                    .is_ok_and(|distance| distance <= reach)
                {
                    let mut next = path.clone();
                    next.push(target);
                    paths.push(next);
                }
            }
        }
        paths.remove(0);
        paths.sort_by_key(|path| path.last().map(|s| s.system));
        if paths.is_empty() {
            return Vec::new();
        }
        let mut offers = Vec::new();
        for row in 0..6 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            if row % 2 == 1 {
                let index = ((u64::from(seed) * paths.len() as u64) >> 32) as usize;
                let path = paths[index].clone();
                offers.push(DispatchOffer {
                    row,
                    fee_tenths: 3200 + 900 * (path.len() as u32 - 1),
                    path,
                });
            }
        }
        offers
    }
}

impl Ledger {
    pub(super) fn apply_dispatch(
        &mut self,
        event: Event,
        action: DispatchAction,
    ) -> Result<(), ModuleError> {
        match action {
            DispatchAction::Collect(row) => {
                let fuel =
                    u16::try_from(event.item).map_err(|_| ModuleError::DispatchUnavailable)?;
                let offer = self
                    .dispatch
                    .offers(self, event.dock, fuel)
                    .into_iter()
                    .find(|o| o.row == row)
                    .ok_or(ModuleError::DispatchUnavailable)?;
                self.dispatch.records.push(DispatchRecord {
                    issuer: event.dock,
                    issue_revision: event.revision,
                    offer,
                    terminal: None,
                });
            }
            DispatchAction::Receive | DispatchAction::Abandon => {
                let record = self
                    .dispatch
                    .records
                    .last_mut()
                    .filter(|r| r.terminal.is_none())
                    .ok_or(ModuleError::DispatchUnavailable)?;
                if record.issuer.ship() != event.dock.ship() || event.item != record.issue_revision
                {
                    return Err(ModuleError::DispatchUnavailable);
                }
                if action == DispatchAction::Receive {
                    if event.dock.origin() != DockOrigin::PhysicalEntry
                        || event.dock.sequence() <= record.issuer.sequence()
                        || record.offer.path.last() != Some(&event.dock.system())
                    {
                        return Err(ModuleError::BerthRequired);
                    }
                    self.dispatch.earnings = self
                        .dispatch
                        .earnings
                        .checked_add(record.offer.fee_tenths)
                        .ok_or(ModuleError::SequenceExhausted)?;
                } else if event.dock.system() != record.issuer.system() {
                    return Err(ModuleError::BerthRequired);
                }
                record.terminal = Some((action, event.dock));
            }
        }
        self.dispatch.events += 1;
        Ok(())
    }
}

impl State {
    pub(super) fn dispatch_occupancy(&self) -> (usize, usize) {
        let mut total = 0;
        let mut active = 0;
        for event in &self.events {
            if let ModuleAction::Dispatch(action) = event.action {
                active = usize::from(matches!(action, DispatchAction::Collect(_)));
                total += active;
            }
        }
        (total, active)
    }
}

impl World {
    /// Closed G01 package rows from the current sender and fitted, fuelled route graph.
    pub fn dispatch_offers(&self) -> Vec<DispatchOffer> {
        let Some(dock) = self.current_dock_context() else {
            return Vec::new();
        };
        let Some(account) = &self.commander_account else {
            return Vec::new();
        };
        let Ok(ledger) = self.modular.ledger(self, account) else {
            return Vec::new();
        };
        ledger.dispatch.offers(&ledger, dock, account.fuel_tenths())
    }
    pub fn dispatch_records(&self) -> Vec<DispatchRecord> {
        if self.modular.dispatch_occupancy().0 == 0 {
            return Vec::new();
        }
        self.commander_account
            .as_ref()
            .and_then(|account| self.modular.ledger(self, account).ok())
            .map_or_else(Vec::new, |ledger| ledger.dispatch.records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight::equipment::tests::journey;

    fn confirm(world: &mut World, action: DispatchAction) {
        let tick = world.snapshot().tick;
        let review = world
            .review_module(ModuleAction::Dispatch(action), tick)
            .unwrap();
        world.prepare_module(review, tick).unwrap().commit();
    }
    fn cold(world: &World) -> World {
        World::from_save_bytes(&world.save_bytes().unwrap()).unwrap()
    }

    #[test]
    fn sealed_dispatch_two_leg_original_survives_cold_and_pays_only_at_lareria() {
        let mut world = World::new_commander(41001).unwrap();
        let offers = world.dispatch_offers();
        assert_eq!(
            offers.iter().map(|o| o.row).collect::<Vec<_>>(),
            vec![1, 3, 5]
        );
        assert_eq!(
            offers[2].path,
            vec![SystemId::AVENOR, SystemId::RENAS, SystemId::LARERIA]
        );
        assert_eq!(offers[2].fee_tenths, 5000);
        let cash = world.credits_tenths().unwrap();
        confirm(&mut world, DispatchAction::Collect(5));
        let held = world.dispatch_records();
        assert_eq!(world.credits_tenths(), Some(cash));
        assert_eq!(world.service_snapshot().unwrap().used_tonnes, 1);
        let mut world = cold(&world);
        assert_eq!(world.dispatch_records(), held);
        assert!(world.dispatch_offers().is_empty());
        journey(&mut world, SystemId::RENAS);
        let before = world.save_bytes().unwrap();
        for action in [
            DispatchAction::Collect(1),
            DispatchAction::Receive,
            DispatchAction::Abandon,
        ] {
            assert!(
                world
                    .review_module(ModuleAction::Dispatch(action), world.snapshot().tick)
                    .is_err()
            );
        }
        assert_eq!(world.save_bytes().unwrap(), before);
        journey(&mut world, SystemId::LARERIA);
        let mut world = cold(&world);
        let tick = world.snapshot().tick;
        let review = world
            .review_module(ModuleAction::Dispatch(DispatchAction::Receive), tick)
            .unwrap();
        let before = world.save_bytes().unwrap();
        assert!(
            world
                .prepare_module(review, tick)
                .unwrap()
                .publish_with(|_| Err(()))
                .is_err()
        );
        assert_eq!(world.save_bytes().unwrap(), before);
        confirm(&mut world, DispatchAction::Receive);
        let world = cold(&world);
        assert_eq!(world.credits_tenths(), Some(cash + 5000));
        assert_eq!(world.service_snapshot().unwrap().used_tonnes, 0);
        let record = world.dispatch_records().pop().unwrap();
        assert_eq!(
            (record.issue_revision, record.issuer, record.offer),
            (held[0].issue_revision, held[0].issuer, offers[2].clone())
        );
        assert_eq!(
            record.terminal,
            Some((
                DispatchAction::Receive,
                world.current_dock_context().unwrap()
            ))
        );
        assert_eq!(
            world
                .unwritten_route_clue_progress()
                .unwrap()
                .earned
                .iter()
                .filter(|c| c.clue_id == "sealed-dispatch-family-trace")
                .count(),
            1
        );
        assert!(
            world
                .review_module(
                    ModuleAction::Dispatch(DispatchAction::Receive),
                    world.snapshot().tick
                )
                .is_err()
        );
    }

    #[test]
    fn sealed_dispatch_sender_abandonment_releases_only_the_original_tonne() {
        let mut world = World::new_commander(41002).unwrap();
        let cash = world.credits_tenths();
        confirm(&mut world, DispatchAction::Collect(1));
        let first = world.dispatch_records()[0].issue_revision;
        confirm(&mut world, DispatchAction::Abandon);
        let mut world = cold(&world);
        assert_eq!(world.credits_tenths(), cash);
        assert_eq!(world.service_snapshot().unwrap().used_tonnes, 0);
        assert!(
            world
                .unwritten_route_clue_progress()
                .unwrap()
                .earned
                .is_empty()
        );
        confirm(&mut world, DispatchAction::Collect(1));
        let records = cold(&world).dispatch_records();
        assert_eq!(records.len(), 2);
        assert!(records[1].issue_revision > first);
        assert!(records[0].terminal.is_some());
    }
}

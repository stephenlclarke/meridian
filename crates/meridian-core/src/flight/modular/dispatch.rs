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
    pub lost_with_carrier: bool,
}

#[derive(Default)]
pub(super) struct Dispatch {
    pub(super) records: Vec<DispatchRecord>,
    pub(super) events: u64,
    pub(super) earnings: u32,
}

pub(super) fn board_draws(
    ledger: &Ledger,
    dock: DockContext,
    fuel: u16,
    digest: &str,
    board: &str,
    revision: usize,
) -> Vec<Vec<SystemId>> {
    if !MARKETS_V1.contains(&dock.system()) || fuel > fuel_capacity(ledger.fuel_storage_count()) {
        return Vec::new();
    }
    let key = format!(
        "contract-board-v1:{digest}/{board}/main-orbital:0:{}:{revision}",
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
    (0..6)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            paths[((u64::from(seed) * paths.len() as u64) >> 32) as usize].clone()
        })
        .collect()
}

impl Dispatch {
    pub(super) fn active(&self) -> Option<&DispatchRecord> {
        self.records
            .last()
            .filter(|record| record.terminal.is_none())
    }
    fn offers(&self, ledger: &Ledger, dock: DockContext, fuel: u16) -> Vec<DispatchOffer> {
        if self.active().is_some() {
            return Vec::new();
        }
        let revision = self
            .records
            .iter()
            .filter(|r| r.issuer.system() == dock.system())
            .count()
            * 2;
        board_draws(
            ledger,
            dock,
            fuel,
            DISPATCH_DIGEST,
            "sealed-dispatch",
            revision,
        )
        .into_iter()
        .enumerate()
        .skip(1)
        .step_by(2)
        .map(|(row, path)| DispatchOffer {
            row: row as u8,
            fee_tenths: 3200 + 900 * (path.len() as u32 - 1),
            path,
        })
        .collect()
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
                    lost_with_carrier: false,
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
    pub(in crate::flight) fn dispatch_occupancy(&self) -> (usize, usize) {
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
        let mut records = self
            .commander_account
            .as_ref()
            .and_then(|account| self.modular.ledger(self, account).ok())
            .map_or_else(Vec::new, |ledger| ledger.dispatch.records);
        if self
            .combat_snapshot()
            .is_some_and(|combat| combat.player.destroyed)
            && let Some(record) = records.last_mut().filter(|r| r.terminal.is_none())
        {
            record.lost_with_carrier = true;
        }
        records
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
            .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"));
        world
            .prepare_module(review, tick)
            .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"))
            .commit();
    }
    fn cold(world: &World) -> World {
        World::from_save_bytes(
            &world
                .save_bytes()
                .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}")),
        )
        .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"))
    }

    #[test]
    fn sealed_dispatch_two_leg_original_survives_cold_and_pays_only_at_lareria() {
        let mut world = World::new_commander(41001)
            .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"));
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
        let cash = world
            .credits_tenths()
            .unwrap_or_else(|| panic!("missing credits_tenths"));
        confirm(&mut world, DispatchAction::Collect(5));
        let held = world.dispatch_records();
        assert_eq!(world.credits_tenths(), Some(cash));
        assert_eq!(
            world
                .service_snapshot()
                .unwrap_or_else(|| panic!("missing service_snapshot"))
                .used_tonnes,
            1
        );
        let mut world = cold(&world);
        assert_eq!(world.dispatch_records(), held);
        assert!(world.dispatch_offers().is_empty());
        journey(&mut world, SystemId::RENAS);
        let before = world
            .save_bytes()
            .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"));
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
        assert_eq!(
            world
                .save_bytes()
                .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}")),
            before
        );
        journey(&mut world, SystemId::LARERIA);
        let mut world = cold(&world);
        let tick = world.snapshot().tick;
        let review = world
            .review_module(ModuleAction::Dispatch(DispatchAction::Receive), tick)
            .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"));
        let before = world
            .save_bytes()
            .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"));
        assert!(
            world
                .prepare_module(review, tick)
                .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"))
                .publish_with(|_| Err(()))
                .is_err()
        );
        assert_eq!(
            world
                .save_bytes()
                .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}")),
            before
        );
        confirm(&mut world, DispatchAction::Receive);
        let world = cold(&world);
        assert_eq!(world.credits_tenths(), Some(cash + 5000));
        assert_eq!(
            world
                .service_snapshot()
                .unwrap_or_else(|| panic!("missing service_snapshot"))
                .used_tonnes,
            0
        );
        let record = world
            .dispatch_records()
            .pop()
            .unwrap_or_else(|| panic!("missing pop"));
        assert_eq!(
            (record.issue_revision, record.issuer, record.offer),
            (held[0].issue_revision, held[0].issuer, offers[2].clone())
        );
        assert_eq!(
            record.terminal,
            Some((
                DispatchAction::Receive,
                world
                    .current_dock_context()
                    .unwrap_or_else(|| panic!("missing current_dock_context"))
            ))
        );
        assert_eq!(
            world
                .unwritten_route_clue_progress()
                .unwrap_or_else(|| panic!("missing unwritten_route_clue_progress"))
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
    fn sealed_dispatch_refuses_test_refills_while_the_original_is_aboard() {
        let mut world = World::new_commander(41004).unwrap_or_else(|error| panic!("{error:?}"));
        confirm(&mut world, DispatchAction::Collect(1));
        let before = world
            .save_bytes()
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(
            world.review_xyzzy(crate::XyzzyAction::Enter, world.snapshot().tick),
            Err(crate::XyzzyError::ActiveFuelPractice)
        );
        assert_eq!(
            world
                .save_bytes()
                .unwrap_or_else(|error| panic!("{error:?}")),
            before
        );
    }

    #[test]
    fn sealed_dispatch_second_delivery_pays_its_own_fee_and_keeps_one_clue() {
        let mut world = World::new_commander(41003)
            .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"));
        for _ in 0..2 {
            let offer = world.dispatch_offers().remove(0);
            let before = world
                .credits_tenths()
                .unwrap_or_else(|| panic!("missing credits_tenths"));
            confirm(&mut world, DispatchAction::Collect(offer.row));
            for &stop in &offer.path[1..] {
                journey(&mut world, stop);
            }
            confirm(&mut world, DispatchAction::Receive);
            assert_eq!(world.credits_tenths(), Some(before + offer.fee_tenths));
            world = cold(&world);
            assert_eq!(
                world
                    .unwritten_route_clue_progress()
                    .unwrap_or_else(|| panic!("missing unwritten_route_clue_progress"))
                    .earned
                    .len(),
                1
            );
        }
        assert_eq!(world.dispatch_records().len(), 2);
    }

    #[test]
    fn sealed_dispatch_sender_abandonment_releases_only_the_original_tonne() {
        let mut world = World::new_commander(41002)
            .unwrap_or_else(|error| panic!("unexpected refusal: {error:?}"));
        let cash = world.credits_tenths();
        confirm(&mut world, DispatchAction::Collect(1));
        let first = world.dispatch_records()[0].issue_revision;
        confirm(&mut world, DispatchAction::Abandon);
        let mut world = cold(&world);
        assert_eq!(world.credits_tenths(), cash);
        assert_eq!(
            world
                .service_snapshot()
                .unwrap_or_else(|| panic!("missing service_snapshot"))
                .used_tonnes,
            0
        );
        assert!(
            world
                .unwritten_route_clue_progress()
                .unwrap_or_else(|| panic!("missing unwritten_route_clue_progress"))
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

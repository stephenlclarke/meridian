//! Original G03 TOREN bonded machinery and workshop receipts.
use super::{Event, Ledger, ModuleAction, ModuleError, State, World, dispatch::board_draws};
use crate::navigation::{DockContext, DockOrigin, SystemId};

const PACK: &str = "08dfba917d8f630b0fb39a4458ec1fa06245eb1347352a172a7576c0967682ca";
pub(super) const BOND: u32 = 750;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BondedAction {
    Accept(u8),
    ReceiveOne,
    ReceiveAll,
    Abandon,
}

impl BondedAction {
    pub(super) fn receiving(self) -> bool {
        matches!(self, Self::ReceiveOne | Self::ReceiveAll)
    }
    fn remaining_after(self, count: u16) -> u16 {
        match self {
            Self::Accept(_) => 3,
            Self::ReceiveOne => count.saturating_sub(1),
            Self::ReceiveAll | Self::Abandon => 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BondedOffer {
    pub tier: u8,
    pub path: Vec<SystemId>,
    pub fee_tenths: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BondedUnit {
    Aboard,
    Received(DockContext),
    Returned(DockContext),
    Lost,
}

/// Each original tonne is identified by issuer, account issue revision and array position.
/// A workshop stamp stays with that original even if the carrier later abandons or is lost.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BondedRecord {
    pub issuer: DockContext,
    pub issue_revision: u64,
    pub offer: BondedOffer,
    pub units: [BondedUnit; 3],
}
impl BondedRecord {
    pub fn remaining(&self) -> u16 {
        self.units
            .iter()
            .filter(|u| **u == BondedUnit::Aboard)
            .count() as u16
    }
    pub fn complete(&self) -> bool {
        self.units
            .iter()
            .all(|u| matches!(u, BondedUnit::Received(_)))
    }
}

#[derive(Default)]
pub(super) struct Bonded {
    pub(super) records: Vec<BondedRecord>,
    pub(super) credited: u32,
    pub(super) events: u64,
}
impl Bonded {
    pub(super) fn active(&self) -> Option<&BondedRecord> {
        self.records.last().filter(|r| r.remaining() > 0)
    }
    fn offers(&self, ledger: &Ledger, dock: DockContext, fuel: u16) -> Vec<BondedOffer> {
        if self.active().is_some() {
            return Vec::new();
        }
        let revision = 2 * self
            .records
            .iter()
            .filter(|r| r.issuer.system() == dock.system())
            .count();
        board_draws(ledger, dock, fuel, PACK, "bonded-freight", revision)
            .into_iter()
            .step_by(2)
            .enumerate()
            .map(|(tier, path)| BondedOffer {
                tier: tier as u8,
                fee_tenths: 4800 + 950 * (path.len() as u32 - 1) + 800 * tier as u32,
                path,
            })
            .collect()
    }
}
impl Ledger {
    pub(super) fn apply_bonded(
        &mut self,
        event: Event,
        action: BondedAction,
    ) -> Result<(), ModuleError> {
        if let BondedAction::Accept(tier) = action {
            let fuel = u16::try_from(event.item).map_err(|_| ModuleError::BondedUnavailable)?;
            let offer = self
                .bonded
                .offers(self, event.dock, fuel)
                .into_iter()
                .find(|offer| offer.tier == tier)
                .ok_or(ModuleError::BondedUnavailable)?;
            self.bonded.records.push(BondedRecord {
                issuer: event.dock,
                issue_revision: event.revision,
                offer,
                units: [BondedUnit::Aboard; 3],
            });
        } else {
            let record = self
                .bonded
                .records
                .last_mut()
                .filter(|r| {
                    r.remaining() > 0
                        && r.issue_revision == event.item
                        && r.issuer.ship() == event.dock.ship()
                        && r.issuer.owner() == event.dock.owner()
                })
                .ok_or(ModuleError::BondedUnavailable)?;
            let stamp = if action.receiving() {
                if event.dock.origin() != DockOrigin::PhysicalEntry
                    || event.dock.sequence() <= record.issuer.sequence()
                    || event.dock.visit() <= record.issuer.visit()
                    || record.offer.path.last() != Some(&event.dock.system())
                {
                    return Err(ModuleError::BerthRequired);
                }
                BondedUnit::Received(event.dock)
            } else {
                if event.dock.system() != record.issuer.system() {
                    return Err(ModuleError::BerthRequired);
                }
                BondedUnit::Returned(event.dock)
            };
            for unit in record
                .units
                .iter_mut()
                .filter(|u| **u == BondedUnit::Aboard)
            {
                *unit = stamp;
                if action == BondedAction::ReceiveOne {
                    break;
                }
            }
            if record.complete() {
                self.bonded.credited = self
                    .bonded
                    .credited
                    .checked_add(record.offer.fee_tenths + BOND)
                    .ok_or(ModuleError::SequenceExhausted)?;
            }
        }
        self.bonded.events += 1;
        Ok(())
    }
}
impl State {
    pub(in crate::flight) fn bonded_occupancy(&self) -> (usize, u16) {
        self.events.iter().fold((0, 0), |(issued, remaining), e| {
            if let ModuleAction::Bonded(action) = e.action {
                (
                    issued + usize::from(matches!(action, BondedAction::Accept(_))),
                    action.remaining_after(remaining),
                )
            } else {
                (issued, remaining)
            }
        })
    }
}
impl World {
    pub fn bonded_offers(&self) -> Vec<BondedOffer> {
        let Some(dock) = self.current_dock_context() else {
            return Vec::new();
        };
        let Some(account) = &self.commander_account else {
            return Vec::new();
        };
        if self.active_freight_job().is_some() {
            return Vec::new();
        }
        let Ok(ledger) = self.modular.ledger(self, account) else {
            return Vec::new();
        };
        ledger.bonded.offers(&ledger, dock, account.fuel_tenths())
    }
    pub fn bonded_records(&self) -> Vec<BondedRecord> {
        if self.modular.bonded_occupancy().0 == 0 {
            return Vec::new();
        }
        let mut records = self
            .commander_account
            .as_ref()
            .and_then(|a| self.modular.ledger(self, a).ok())
            .map_or_else(Vec::new, |l| l.bonded.records);
        if self.combat_snapshot().is_some_and(|c| c.player.destroyed)
            && let Some(r) = records.last_mut()
        {
            for unit in r.units.iter_mut().filter(|u| **u == BondedUnit::Aboard) {
                *unit = BondedUnit::Lost;
            }
        }
        records
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight::equipment::tests::journey;

    fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|e| panic!("unexpected refusal: {e:?}"))
    }
    fn confirm(world: &mut World, action: BondedAction) {
        let tick = world.snapshot().tick;
        let review = ok(world.review_module(ModuleAction::Bonded(action), tick));
        ok(world.prepare_module(review, tick)).commit();
    }
    fn cold(world: &World) -> World {
        ok(World::from_save_bytes(&ok(world.save_bytes())))
    }
    #[test]
    fn bonded_freight_three_tiers_and_partial_receipts() {
        let mut world = ok(World::new_commander(41003));
        let rows = world.bonded_offers();
        assert_eq!(
            rows.iter()
                .map(|o| (o.tier, o.fee_tenths))
                .collect::<Vec<_>>(),
            vec![(0, 5750), (1, 6550), (2, 8300)]
        );
        assert_eq!(rows[0].path, vec![SystemId::AVENOR, SystemId::RENAS]);
        confirm(&mut world, BondedAction::Accept(0));
        let mut world = cold(&world);
        let original = world.bonded_records().remove(0);
        assert_eq!(original.remaining(), 3);
        assert_eq!(world.credits_tenths(), Some(6450));
        assert!(
            world
                .review_module(
                    ModuleAction::Bonded(BondedAction::ReceiveOne),
                    world.snapshot().tick
                )
                .is_err()
        );
        journey(&mut world, SystemId::RENAS);
        for received in 1..=3 {
            confirm(&mut world, BondedAction::ReceiveOne);
            world = cold(&world);
            let record = world.bonded_records().remove(0);
            assert_eq!(record.issue_revision, original.issue_revision);
            assert_eq!(record.issuer, original.issuer);
            assert!(record.units[..received].iter().all(|u|matches!(u,BondedUnit::Received(d) if d.sequence()>original.issuer.sequence() && d.visit()>original.issuer.visit())));
            assert!(
                record.units[received..]
                    .iter()
                    .all(|u| *u == BondedUnit::Aboard)
            );
            assert_eq!(
                world.service_snapshot().map(|s| s.used_tonnes),
                Some(3 - received as u16)
            );
            assert_eq!(
                world.credits_tenths(),
                Some(if received == 3 { 12950 } else { 6450 })
            );
            assert_eq!(
                world
                    .unwritten_route_clue_progress()
                    .map(|s| s.earned.len()),
                Some(usize::from(received == 3))
            );
        }
        assert!(
            world
                .review_module(
                    ModuleAction::Bonded(BondedAction::ReceiveAll),
                    world.snapshot().tick
                )
                .is_err()
        );
    }

    #[test]
    fn bonded_freight_earlier_workshop_cannot_authorise_a_partial_lot() {
        let mut world = ok(World::new_commander(41004));
        journey(&mut world, SystemId::RENAS);
        let earlier = world
            .current_dock_context()
            .unwrap_or_else(|| panic!("missing earlier berth"));
        journey(&mut world, SystemId::AVENOR);
        let refuel = ok(world.request_refuel(world.snapshot().tick));
        confirm(&mut world, BondedAction::Accept(0));
        journey(&mut world, SystemId::RENAS);
        confirm(&mut world, BondedAction::ReceiveOne);
        let valid = ok(world.save_bytes());
        let last = world
            .modular
            .events
            .last_mut()
            .unwrap_or_else(|| panic!("missing workshop receipt"));
        last.dock = earlier;
        assert!(
            world
                .save_bytes()
                .and_then(|bytes| World::from_save_bytes(&bytes))
                .is_err()
        );
        let mut world = ok(World::from_save_bytes(&valid));
        journey(&mut world, SystemId::AVENOR);
        confirm(&mut world, BondedAction::Abandon);
        let mut world = cold(&world);
        let previous = world.bonded_records().remove(0);
        assert!(matches!(
            previous.units,
            [
                BondedUnit::Received(_),
                BondedUnit::Returned(_),
                BondedUnit::Returned(_)
            ]
        ));
        assert_eq!(world.credits_tenths(), Some(6450 - refuel));
        assert!(!previous.complete());
        confirm(&mut world, BondedAction::Accept(0));
        assert_ne!(
            world.bonded_records()[1].issue_revision,
            previous.issue_revision
        );
    }
}

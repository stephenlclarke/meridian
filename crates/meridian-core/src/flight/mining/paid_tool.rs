//! Actual paid-equipment join. Apply only with G3 b1bed892ba4b37a5996b2c964e0c7fc532c1c2fb.
use crate::World;
use crate::mining::{MiningError, MiningToolAuthority};

impl World {
    pub fn mining_tool_authority(&self) -> Result<MiningToolAuthority, MiningError> {
        let actual = self
            .fitted_mining_tool()
            .ok_or(MiningError::NoCompatibleFittedTool)?;
        Ok(MiningToolAuthority {
            owner: actual.owner,
            hull: actual.hull,
            account_revision: actual.account_revision,
            item_id: actual.item,
            epoch: actual.epoch,
            scope: std::sync::Arc::downgrade(&self.mining_scope),
            current_item: World::paid_fitted_mining_item,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight::mining::tests::{approach, ok, some};
    use crate::flight::missiles::tests::{dock, fly_axis};
    use crate::station_services::{CargoUnit, Command, CommodityId};
    use crate::{FlightInput, ModuleAction};

    fn module(w: &mut World, action: ModuleAction) -> u64 {
        let tick = w.snapshot().tick;
        let review = ok(w.review_module(action, tick));
        ok(w.prepare_module(review, tick)).commit()
    }
    fn funded_tool(seed: u64) -> (World, u64) {
        let mut w = crate::flight::equipment::tests::earned_world(seed);
        if some(w.snapshot().navigation).fuel_units < 34 {
            let tick = w.snapshot().tick;
            ok(w.request_refuel(tick));
        }
        crate::flight::equipment::tests::journey(&mut w, crate::navigation::SystemId::AVENOR);
        let tick = w.snapshot().tick;
        let review = ok(w.review_module(ModuleAction::PurchaseMiningLaser, tick));
        let prepared = ok(w.prepare_module(review, tick));
        let staged = ok(World::from_save_bytes(prepared.bytes()));
        assert!(staged.fitted_mining_tool().is_some());
        let item = prepared.commit();
        (w, item)
    }
    fn steps(w: &mut World, count: usize) {
        for _ in 0..count {
            ok(w.step(FlightInput::default()));
        }
    }
    fn mine(w: &mut World) -> crate::mining::MiningShot {
        let authority = ok(w.mining_tool_authority());
        let tick = w.snapshot().tick;
        ok(w.request_mining(authority, tick))
    }
    fn return_to_berth(w: &mut World) {
        // Reach the clear station approach axis before aligning for physical entry.
        fly_axis(w, 0, 0.0, 10.0);
        dock(w);
    }
    #[test]
    fn real_paid_tool_mines_with_shared_resource_debit_and_cold_custody() {
        let (mut w, item) = funded_tool(10401);
        let prelaunch = ok(w.mining_tool_authority());
        approach(&mut w);
        let tick = w.snapshot().tick;
        assert_eq!(
            w.request_mining(prelaunch, tick),
            Err(MiningError::NoCompatibleFittedTool)
        );
        let old = ok(w.mining_tool_authority());
        let before = some(w.mining_belt_snapshot()).bodies;
        let cash = w.credits_tenths();
        let shot = mine(&mut w);
        assert_eq!((shot.item_id, shot.extracted_units), (item, 1));
        assert_eq!(w.credits_tenths(), cash);
        let after = some(w.mining_belt_snapshot()).bodies;
        assert_eq!(before[0].remaining_units - after[0].remaining_units, 1);
        assert_eq!(&after[1..], &before[1..]);
        let resources = some(w.flight_resources_snapshot());
        assert_eq!(
            (
                resources.energy_hundredths,
                resources.heat_fifths,
                resources.cooldown_ticks
            ),
            (37_900, 50, 18)
        );
        assert!(w.pulse_laser_snapshot().is_none());
        let authority = ok(w.mining_tool_authority());
        let before_refusal = ok(w.save_bytes());
        assert_eq!(
            w.request_mining(authority, tick),
            Err(MiningError::Resources)
        );
        assert_eq!(ok(w.save_bytes()), before_refusal);
        steps(&mut w, 18);
        let bytes = ok(w.save_bytes());
        let tick = w.snapshot().tick;
        assert_eq!(
            w.request_mining(old, tick),
            Err(MiningError::NoCompatibleFittedTool)
        );
        assert_eq!(ok(w.save_bytes()), bytes);
        let loaded = ok(World::from_save_bytes(&bytes));
        assert_eq!(
            some(loaded.mining_belt_snapshot()).bodies,
            some(w.mining_belt_snapshot()).bodies
        );
        assert_eq!(
            loaded.commodity_quantity(shot.commodity),
            w.commodity_quantity(shot.commodity)
        );
        assert_eq!(some(loaded.fitted_mining_tool()).item, item);
    }
    #[test]
    fn genuine_authority_refuses_foreign_continue_restore_pause_and_unfit() {
        let (mut w, item) = funded_tool(10501);
        approach(&mut w);
        let bytes = ok(w.save_bytes());
        let mut independent = ok(World::from_save_bytes(&bytes));
        let independent_before = ok(independent.save_bytes());
        let cross_world = ok(w.mining_tool_authority());
        let tick = independent.snapshot().tick;
        assert_eq!(
            independent.request_mining(cross_world, tick),
            Err(MiningError::NoCompatibleFittedTool)
        );
        assert_eq!(ok(independent.save_bytes()), independent_before);
        let old = ok(w.mining_tool_authority());
        ok(w.restore_bytes(&bytes));
        let restored_before = ok(w.save_bytes());
        let tick = w.snapshot().tick;
        assert_eq!(
            w.request_mining(old, tick),
            Err(MiningError::NoCompatibleFittedTool)
        );
        assert_eq!(ok(w.save_bytes()), restored_before);
        let paused = ok(w.mining_tool_authority());
        ok(w.request_pause(true, tick));
        let before = ok(w.save_bytes());
        assert_eq!(w.request_mining(paused, tick), Err(MiningError::Paused));
        assert_eq!(ok(w.save_bytes()), before);
        ok(w.request_pause(false, tick));
        return_to_berth(&mut w);
        module(&mut w, ModuleAction::Store { item });
        assert_eq!(
            w.mining_tool_authority().err(),
            Some(MiningError::NoCompatibleFittedTool)
        );
    }
    #[test]
    fn genuine_refit_and_second_paid_item_change_authority_without_granting_stock() {
        let (mut w, first) = funded_tool(10601);
        let old = ok(w.mining_tool_authority());
        module(&mut w, ModuleAction::Store { item: first });
        assert!(w.mining_tool_authority().is_err());
        module(
            &mut w,
            ModuleAction::Install {
                item: first,
                replacing: None,
            },
        );
        assert_eq!(some(w.fitted_mining_tool()).item, first);
        let cash = some(w.credits_tenths());
        let second = module(&mut w, ModuleAction::PurchaseMiningLaser);
        assert_ne!(first, second);
        assert_eq!(some(w.credits_tenths()), cash - 33_000);
        assert_eq!(some(w.fitted_mining_tool()).item, second);
        let stock = some(w.mining_belt_snapshot()).bodies;
        assert_eq!(stock[0].extracted_units, 0);
        approach(&mut w);
        let tick = w.snapshot().tick;
        let before = ok(w.save_bytes());
        assert_eq!(
            w.request_mining(old, tick),
            Err(MiningError::NoCompatibleFittedTool)
        );
        assert_eq!(ok(w.save_bytes()), before);
        assert_eq!(mine(&mut w).item_id, second);
        let loaded = ok(World::from_save_bytes(&ok(w.save_bytes())));
        assert_eq!(some(loaded.fitted_mining_tool()).item, second);
    }
    #[test]
    fn paid_mining_depletes_across_every_continue_and_sale_and_return_cannot_regenerate_it() {
        let (mut w, item) = funded_tool(10701);
        approach(&mut w);
        let body = some(w.mining_belt_snapshot()).bodies[0];
        let cash = w.credits_tenths();
        let opening = some(w.commodity_quantity(body.commodity));
        for count in 1..=body.initial_units {
            let shot = mine(&mut w);
            assert_eq!(shot.contact.asteroid, body.id);
            assert_eq!(shot.item_id, item);
            assert_eq!(
                some(w.commodity_quantity(body.commodity)),
                opening + u64::from(count)
            );
            w = ok(World::from_save_bytes(&ok(w.save_bytes())));
            let actual = some(w.mining_belt_snapshot()).bodies[0];
            assert_eq!(actual.vertices, body.vertices);
            assert_eq!(actual.remaining_units, body.initial_units - count);
            steps(&mut w, 18);
        }
        assert_eq!(w.credits_tenths(), cash);
        let authority = ok(w.mining_tool_authority());
        let tick = w.snapshot().tick;
        let before = ok(w.save_bytes());
        assert_eq!(
            w.request_mining(authority, tick),
            Err(MiningError::Depleted)
        );
        assert_eq!(ok(w.save_bytes()), before);
        return_to_berth(&mut w);
        let bid = some(w.commodity_quote(body.commodity)).bid_tenths;
        let cash = some(w.credits_tenths());
        for _ in 0..body.initial_units {
            let tick = w.snapshot().tick;
            ok(w.request_station_service(Command::SellCommodity(body.commodity), tick));
        }
        assert_eq!(
            some(w.credits_tenths()),
            cash + u32::from(body.initial_units) * bid
        );
        assert_eq!(some(w.commodity_quantity(body.commodity)), opening);
        w = ok(World::from_save_bytes(&ok(w.save_bytes())));
        approach(&mut w);
        let current = some(w.mining_belt_snapshot()).bodies[0];
        assert_eq!(
            (current.remaining_units, current.extracted_units),
            (0, body.initial_units)
        );
        let authority = ok(w.mining_tool_authority());
        let tick = w.snapshot().tick;
        let before = ok(w.save_bytes());
        assert_eq!(
            w.request_mining(authority, tick),
            Err(MiningError::Depleted)
        );
        assert_eq!(ok(w.save_bytes()), before);
    }
    #[test]
    fn genuine_full_hold_refuses_paid_work_before_resources_or_depletion_change() {
        let (mut w, _) = funded_tool(10801);
        // Only public, paid purchases into the actual hold. No account/capacity fixture edits.
        for id in CommodityId::ALL {
            if id.unit() != CargoUnit::Tonne {
                continue;
            }
            for _ in 0..92 {
                let tick = w.snapshot().tick;
                if w.request_station_service(Command::BuyCommodity(id), tick)
                    .is_err()
                {
                    break;
                }
            }
        }
        let hold = some(w.service_snapshot());
        assert_eq!(hold.used_tonnes, hold.hold_capacity_tonnes);
        approach(&mut w);
        let authority = ok(w.mining_tool_authority());
        let tick = w.snapshot().tick;
        let before = ok(w.save_bytes());
        assert_eq!(
            w.request_mining(authority, tick),
            Err(MiningError::HoldFull)
        );
        assert_eq!(ok(w.save_bytes()), before);
    }
}

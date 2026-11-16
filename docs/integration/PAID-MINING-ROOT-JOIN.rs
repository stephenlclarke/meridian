//! Root-only serial composition with MC1's actual mining module. Not compiled by this
//! standalone equipment branch. The two integration cases are authored and UNRUN.
//! Add this under flight/mining and replace request_mining's direct pulse preparation
//! with self.prepare_mining_resources(tool.item_id, tick), preserving its error mapping.
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
#[allow(clippy::unwrap_used)]
mod actual_paid_tool_tests {
    use super::*;
    use crate::flight::missiles::tests::{dock, stop};
    use crate::flight::practice::tests::{aim_at, launch};
    use crate::{FlightInput, ModuleAction};

    fn funded_tool(seed: u64) -> (World, u64) {
        let mut w = crate::flight::equipment::tests::earned_world(seed);
        if w.snapshot().navigation.unwrap().fuel_units < 34 {
            w.request_refuel(w.snapshot().tick).unwrap();
        }
        crate::flight::equipment::tests::journey(&mut w, crate::navigation::SystemId::AVENOR);
        let tick = w.snapshot().tick;
        let review = w
            .review_module(ModuleAction::PurchaseMiningLaser, tick)
            .unwrap();
        let prepared = w.prepare_module(review, tick).unwrap();
        let staged = World::from_save_bytes(prepared.bytes()).unwrap();
        assert!(staged.fitted_mining_tool().is_some());
        let item = prepared.commit();
        (w, item)
    }
    fn approach(w: &mut World) {
        launch(w);
        stop(w);
        let target = w.mining_belt_snapshot().unwrap().bodies[0].position;
        aim_at(w, target);
        for _ in 0..1800 {
            let p = w.snapshot().position;
            let distance = (f64::from(p[0]) - target.x).hypot(f64::from(p[2]) - target.z);
            if distance < 1900.0 {
                stop(w);
                assert!(w.mining_belt_snapshot().unwrap().front_contact.is_some());
                return;
            }
            w.step(FlightInput {
                throttle: 1.0,
                ..FlightInput::default()
            })
            .unwrap();
            assert!(w.snapshot().motion_refusal.is_none());
        }
        panic!("ordinary flight did not approach the real belt");
    }
    fn steps(w: &mut World, count: usize) {
        for _ in 0..count {
            w.step(FlightInput::default()).unwrap();
        }
    }
    #[test]
    fn real_paid_fitted_tool_extracts_one_unit_with_exact_shared_resource_debit_and_cold_custody() {
        let (mut w, item) = funded_tool(10401);
        let prelaunch = w.mining_tool_authority().unwrap();
        approach(&mut w);
        assert_eq!(
            w.request_mining(prelaunch, w.snapshot().tick),
            Err(MiningError::NoCompatibleFittedTool)
        );
        let old = w.mining_tool_authority().unwrap();
        let valid = w.mining_tool_authority().unwrap();
        let before = w.mining_belt_snapshot().unwrap().bodies;
        let cash = w.credits_tenths();
        let shot = w.request_mining(valid, w.snapshot().tick).unwrap();
        assert_eq!(shot.item_id, item);
        assert_eq!(shot.extracted_units, 1);
        assert_eq!(w.credits_tenths(), cash);
        let after = w.mining_belt_snapshot().unwrap().bodies;
        assert_eq!(
            before
                .iter()
                .map(|b| u64::from(b.remaining_units))
                .sum::<u64>()
                - after
                    .iter()
                    .map(|b| u64::from(b.remaining_units))
                    .sum::<u64>(),
            1
        );
        let resources = w.flight_resources_snapshot().unwrap();
        assert_eq!(
            (
                resources.energy_hundredths,
                resources.heat_fifths,
                resources.cooldown_ticks
            ),
            (37_900, 50, 18)
        );
        steps(&mut w, 18);
        let bytes = w.save_bytes().unwrap();
        assert_eq!(
            w.request_mining(old, w.snapshot().tick),
            Err(MiningError::NoCompatibleFittedTool)
        );
        assert_eq!(w.save_bytes().unwrap(), bytes);
        let loaded = World::from_save_bytes(&bytes).unwrap();
        assert_eq!(
            loaded.mining_belt_snapshot().unwrap().bodies,
            w.mining_belt_snapshot().unwrap().bodies
        );
        assert_eq!(
            loaded.commodity_quantity(shot.commodity),
            w.commodity_quantity(shot.commodity)
        );
        assert_eq!(loaded.fitted_mining_tool().unwrap().item, item);
    }
    #[test]
    fn genuine_tool_authority_refuses_foreign_continue_restore_pause_and_unfit_without_yield() {
        let (mut w, item) = funded_tool(10501);
        approach(&mut w);
        let bytes = w.save_bytes().unwrap();
        let mut independent = World::from_save_bytes(&bytes).unwrap();
        let independent_before = independent.save_bytes().unwrap();
        let cross_world = w.mining_tool_authority().unwrap();
        assert_eq!(
            independent.request_mining(cross_world, independent.snapshot().tick),
            Err(MiningError::NoCompatibleFittedTool)
        );
        assert_eq!(independent.save_bytes().unwrap(), independent_before);
        let old = w.mining_tool_authority().unwrap();
        w.restore_bytes(&bytes).unwrap();
        assert_eq!(
            w.request_mining(old, w.snapshot().tick),
            Err(MiningError::NoCompatibleFittedTool)
        );
        let paused = w.mining_tool_authority().unwrap();
        w.request_pause(true, w.snapshot().tick).unwrap();
        let paused_before = w.save_bytes().unwrap();
        assert_eq!(
            w.request_mining(paused, w.snapshot().tick),
            Err(MiningError::Paused)
        );
        assert_eq!(w.save_bytes().unwrap(), paused_before);
        w.request_pause(false, w.snapshot().tick).unwrap();
        dock(&mut w);
        let tick = w.snapshot().tick;
        let review = w.review_module(ModuleAction::Store { item }, tick).unwrap();
        w.prepare_module(review, tick).unwrap().commit();
        assert!(matches!(
            w.mining_tool_authority(),
            Err(MiningError::NoCompatibleFittedTool)
        ));
        let before = w.save_bytes().unwrap();
        assert_eq!(
            w.mining_tool_authority().err(),
            Some(MiningError::NoCompatibleFittedTool)
        );
        assert_eq!(w.save_bytes().unwrap(), before);
    }
}

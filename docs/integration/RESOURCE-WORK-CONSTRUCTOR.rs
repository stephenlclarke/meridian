// Root-only integration; not compiled in the isolated fast-laser crate.
impl World {
    pub(super) fn prepare_resource_work(
        &self,
        item: u64,
        tick: u64,
        emission: u64,
        asteroid: u8,
    ) -> Result<History, SaveError> {
        if self.paid_fitted_mining_item() != Some(item)
            || tick != self.state.tick
            || self.is_paused()
            || self.state.mode != FlightMode::Flight
        {
            return Err(INVALID);
        }
        let owner = self.combat_identity().ok_or(INVALID)?;
        let mut history = self.resource_history.clone().ok_or(INVALID)?;
        history.push(Event::Work {
            shot: Shot {
                tick,
                physical: self.combat.physical_tick(),
                system: owner.system,
                visit: owner.visit,
                item,
                weapon: OrdinaryWeapon::FrontMiningLaser,
                emission,
            },
            asteroid,
        });
        Ok(history)
    }
}

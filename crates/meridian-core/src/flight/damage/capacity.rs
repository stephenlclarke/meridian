//! Preflight the existing whole-save byte limit before coupled combat publication.
use super::{FlightMode, MAX_SAVE_BYTES, PulseResources, SaveError, State, World, Writer};
use crate::flight::{NavigationState, Snapshot, combat, missiles, navigation_trade, practice};
use crate::station_services::StationMarkets;
use crate::traffic;
fn size(encode: impl FnOnce(&mut Writer)) -> usize {
    let mut w = Writer::new();
    encode(&mut w);
    w.bytes.len()
}
fn parts(
    traffic: &traffic::State,
    damage: &State,
    combat: &combat::State,
    pulse: &PulseResources,
    missiles: &missiles::State,
    practice: Option<practice::Practice>,
) -> usize {
    size(|w| {
        traffic.encode(w);
        damage.encode(w);
        combat.encode(w);
        pulse.encode_save(w);
        missiles.encode(w);
        w.option(practice.as_ref(), |w, p| p.encode_reception(w));
        w.option(practice.as_ref(), |w, p| p.encode(w));
        w.option(practice.and_then(|p| p.ordinary_hit).as_ref(), |w, h| {
            h.encode(w)
        });
    })
}
impl World {
    pub(in crate::flight) fn check_fire_combat_capacity(
        &self,
        traffic: &traffic::State,
        damage: &State,
        combat: &combat::State,
        pulse: &PulseResources,
        missiles: &missiles::State,
        practice: Option<practice::Practice>,
    ) -> Result<(), SaveError> {
        if self.commander_account.is_none() {
            return Ok(());
        }
        let old = parts(
            &self.traffic,
            &self.damage,
            &self.combat,
            &self.pulse,
            &self.missiles,
            self.practice,
        );
        let new = parts(traffic, damage, combat, pulse, missiles, practice);
        let total = self
            .encoded_current_size()?
            .checked_sub(old)
            .and_then(|n| n.checked_add(new))
            .ok_or(SaveError::TooLarge)?;
        if total > MAX_SAVE_BYTES {
            return Err(SaveError::TooLarge);
        }
        Ok(())
    }
    pub(in crate::flight) fn check_step_combat_capacity(
        &self,
        next: Snapshot,
        navigation: Option<NavigationState>,
        prepared: &missiles::PreparedStep,
        traffic: Option<&(traffic::State, StationMarkets)>,
        resources: (
            &State,
            &crate::mining::State,
            &crate::flight::emp::State,
            &crate::flight::ecm::State,
        ),
        trade: Option<&navigation_trade::State>,
    ) -> Result<(), SaveError> {
        let (damage, mining, emp, ecm) = resources;
        if self.commander_account.is_none() {
            return Ok(());
        }
        let (traffic, markets) = traffic
            .map_or((&self.traffic, self.station_markets.as_ref()), |(t, m)| {
                (t, Some(m))
            });
        // Retained missile motion/loss/interception scenes can grow without another hit.
        // Compare their encoded size as well as domain events before skipping the envelope.
        let missile_growth =
            size(|w| prepared.missiles.encode(w)) > size(|w| self.missiles.encode(w));
        let journal_grows = navigation.map(NavigationState::jump_sequence)
            != self.navigation.map(NavigationState::jump_sequence);
        if *damage == self.damage
            && mining.unchanged_since(&self.mining)
            && !traffic.proof_grows_from(&self.traffic)
            && !missile_growth
            && !journal_grows
            && !emp.proof_grows_from(&self.emp)
            && ecm.unchanged_since(&self.ecm)
        {
            return Ok(());
        }
        let old = parts(
            &self.traffic,
            &self.damage,
            &self.combat,
            &self.pulse,
            &self.missiles,
            self.practice,
        );
        let new = parts(
            traffic,
            damage,
            &prepared.combat,
            &prepared.pulse,
            &prepared.missiles,
            prepared.practice,
        );
        let old_mining = size(|w| self.mining.encode(w));
        let new_mining = size(|w| mining.encode(w));
        let mut total = self
            .encoded_current_size()?
            .checked_sub(old)
            .and_then(|n| n.checked_add(new))
            .and_then(|n| n.checked_sub(old_mining))
            .and_then(|n| n.checked_add(new_mining))
            .and_then(|n| n.checked_sub(size(|w| self.emp.encode(w))))
            .and_then(|n| n.checked_add(size(|w| emp.encode(w))))
            .and_then(|n| n.checked_sub(self.ecm.encoded_len()))
            .and_then(|n| n.checked_add(ecm.encoded_len()))
            .ok_or(SaveError::TooLarge)?;
        let old = size(|w| {
            if let Some(n) = self.navigation {
                n.encode_save(w);
            }
            if let Some(m) = &self.station_markets {
                m.encode_save(w);
            }
            self.navigation_trade.encode(w);
        });
        let new = size(|w| {
            if let Some(n) = navigation {
                n.encode_save(w);
            }
            if let Some(m) = markets {
                m.encode_save(w);
            }
            trade.unwrap_or(&self.navigation_trade).encode(w);
        });
        total = total
            .checked_sub(old)
            .and_then(|n| n.checked_add(new))
            .ok_or(SaveError::TooLarge)?;
        // A physical entry adds the actual supported main-station roster. The existing
        // airborne roster is absent. Rebinding changes no freight state or variable ID size.
        if self.state.mode != FlightMode::Docked
            && next.mode == FlightMode::Docked
            && let Some(dock) = navigation.and_then(NavigationState::dock)
        {
            total = total
                .checked_add(size(|w| dock.encode_save(w)) + 8 + 8 + 1 + 8)
                .ok_or(SaveError::TooLarge)?;
        }
        if journal_grows {
            total = total
                .checked_add(crate::navigation::CompletedJump::ENCODED_SIZE)
                .ok_or(SaveError::TooLarge)?;
        }
        if total > MAX_SAVE_BYTES {
            return Err(SaveError::TooLarge);
        }
        Ok(())
    }
}

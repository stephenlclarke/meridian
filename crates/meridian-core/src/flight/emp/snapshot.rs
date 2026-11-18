//! Immutable readouts of actual property and electrical state.
use super::*;
#[derive(Debug, Clone, PartialEq)]
pub struct EmpSnapshot {
    pub item: OwnedEmp,
    pub main_energy: f64,
    pub bank_energy: f64,
    pub bank_capacity: f64,
    pub transfer_loss: f64,
    pub electrically_disabled: bool,
    pub charge_phase: Option<EmpChargePhase>,
    pub charge_release: Option<EmpOrigin>,
    pub charge_ticks: u64,
    pub target: Option<crate::traffic::NpcId>,
    pub releases: u16,
    pub live_disabled_targets: usize,
    pub travel_blocked: bool,
}
impl World {
    pub fn emp_snapshot(&self) -> Option<EmpSnapshot> {
        let item = self.emp.owned().ok()??;
        let energy = |amount: u64| amount as f64 / f64::from(policy::SCALE);
        Some(EmpSnapshot {
            item,
            main_energy: energy(u64::from(self.emp.meter.main)),
            bank_energy: energy(u64::from(self.emp.meter.bank)),
            bank_capacity: energy(u64::from(item.grades.bank_capacity())),
            transfer_loss: energy(
                self.emp
                    .meter
                    .main_debited
                    .saturating_sub(self.emp.meter.bank_gained),
            ),
            electrically_disabled: self.emp.electrical_disabled(),
            charge_phase: self.emp.charge.as_ref().map(|c| c.phase),
            charge_release: self
                .emp
                .charge
                .as_ref()
                .and_then(|c| c.pulse)
                .and_then(|ordinal| self.emp.pulses.get(usize::from(ordinal) - 1))
                .map(|p| p.origin),
            charge_ticks: self.emp.charge.as_ref().map_or(0, |c| {
                c.ended_at
                    .unwrap_or(self.combat.physical_tick())
                    .saturating_sub(c.start.physical_tick)
            }),
            target: self.emp.charge.as_ref().map(|c| c.target),
            releases: self.emp.pulses.len() as u16,
            live_disabled_targets: self
                .emp
                .episodes
                .iter()
                .filter(|episode| {
                    episode.recovered_at.is_none()
                        && self
                            .traffic
                            .emp_resources(episode.victim)
                            .is_some_and(|(h, _, _, p)| h > 0 && p.is_some_and(|p| p.disabled))
                })
                .count(),
            travel_blocked: self.emp.travel_blocked(&self.traffic),
        })
    }
}

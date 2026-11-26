//! Reconstruct charge from accepted physical intervals and actual berth/arrival history.
use super::{
    EmpEquipmentAction, EmpGrades, EquipmentEvent, INVALID, SaveError, State, World, meter, policy,
    scene,
};
#[derive(Clone, Copy)]
enum Change {
    FinalDeath,
    Boundary,
    Equipment(EquipmentEvent),
    Energy(Option<u8>),
    Pulse(usize),
    Start,
    Ecm,
}
struct Replay {
    meter: meter::Meter,
    clock: u64,
    bank_clock: u64,
    present: bool,
    acquired: bool,
    fitted: bool,
    grades: EmpGrades,
    energy: Option<u8>,
}
impl Replay {
    fn event(
        &mut self,
        clock: u64,
        change: Change,
        pulses: &[super::runtime::Pulse],
        last_special: &mut Option<u64>,
    ) -> Result<(), SaveError> {
        match change {
            Change::Boundary | Change::FinalDeath | Change::Pulse(_) => {
                if clock == 0 || *last_special == Some(clock) {
                    return Err(INVALID);
                }
                self.advance(clock - 1)?;
                self.post(clock)?;
                *last_special = Some(clock);
                if matches!(change, Change::FinalDeath) && self.fitted {
                    self.bank_clock = clock;
                }
                if let Change::Pulse(index) = change {
                    self.apply_pulse(pulses, index, clock)?;
                }
            }
            other => {
                self.advance(clock)?;
                match other {
                    Change::Equipment(event) => self.equipment(event)?,
                    Change::Energy(stage) => self.energy = stage,
                    Change::Start => self.admission()?,
                    Change::Ecm if self.meter.disabled_at.is_none() => {}
                    Change::Ecm => return Err(INVALID),
                    _ => return Err(INVALID),
                }
            }
        }
        Ok(())
    }
    fn apply_pulse(
        &mut self,
        pulses: &[super::runtime::Pulse],
        index: usize,
        clock: u64,
    ) -> Result<(), SaveError> {
        self.admission()?;
        self.meter.discharge()?;
        self.bank_clock = clock;
        let pulse = &pulses[index];
        let expected = if let Some(contact) = pulse
            .contacts
            .iter()
            .find(|c| c.victim == scene::Victim::Player)
        {
            let power = contact
                .power_hundredths
                .checked_sub(pulse.player_shield_absorbed)
                .ok_or(INVALID)?;
            self.meter
                .drain(u32::from(power) * (policy::SCALE / 100), clock)?
        } else {
            0
        };
        if expected != pulse.player_electrical_drained {
            return Err(INVALID);
        }
        Ok(())
    }
    fn rate(&self) -> u32 {
        210 * (1 + u32::from(self.energy.unwrap_or(0)))
    }
    fn post(&mut self, end: u64) -> Result<(), SaveError> {
        if self.acquired && (self.present || self.fitted) {
            self.present = true;
            self.meter.post_main(end, self.rate())?;
        }
        self.clock = end;
        Ok(())
    }
    fn advance(&mut self, end: u64) -> Result<(), SaveError> {
        if end < self.clock {
            return Err(INVALID);
        }
        if end == self.clock {
            return Ok(());
        }
        if !self.acquired || (!self.present && !self.fitted) {
            self.clock = end;
            return Ok(());
        }
        self.present = true;
        if self.fitted && self.bank_clock != self.clock {
            self.post(self.clock.checked_add(1).ok_or(INVALID)?)?;
            self.bank_clock = self.clock;
        }
        self.meter.advance(
            self.clock,
            end,
            self.rate(),
            self.fitted.then_some(self.grades),
        )?;
        if self.fitted {
            self.bank_clock = end;
        }
        self.clock = end;
        Ok(())
    }
    fn equipment(&mut self, event: EquipmentEvent) -> Result<(), SaveError> {
        match event.action {
            EmpEquipmentAction::Purchase => {
                if self.acquired {
                    return Err(INVALID);
                }
                self.acquired = true;
                self.fitted = true;
                self.bank_clock = self.clock;
            }
            EmpEquipmentAction::Fit { .. } => self.fitted = true,
            EmpEquipmentAction::Store { .. } => self.fitted = false,
            EmpEquipmentAction::Upgrade { axis, .. } => {
                self.grades = self.grades.advancing(axis).map_err(|_| INVALID)?;
                self.bank_clock = self.clock;
            }
        }
        Ok(())
    }
    fn admission(&self) -> Result<(), SaveError> {
        if !self.acquired
            || !self.fitted
            || !self.present
            || self.energy.is_none()
            || self.meter.main == 0
            || self.meter.disabled_at.is_some()
            || self.meter.bank < policy::PULSE_COST
        {
            return Err(INVALID);
        }
        Ok(())
    }
}
impl State {
    fn meter_changes(
        &self,
        world: &World,
        modular: &super::super::modular::State,
    ) -> Result<Vec<(u64, u8, Change)>, SaveError> {
        let end = world.combat.physical_tick();
        let mut events = Vec::new();
        for clock in world
            .damage
            .emp_berths()
            .into_iter()
            .chain(world.jump_journal.emp_arrival_clocks())
        {
            events.push((clock, 0_u8, Change::Boundary));
        }
        for event in self.equipment.iter() {
            let clock = world
                .damage
                .berth_clock(event.dock.sequence())
                .ok_or(INVALID)?;
            events.push((clock, 1, Change::Equipment(*event)));
        }
        for (clock, stage) in modular.emp_energy_changes(world)? {
            events.push((clock, 2, Change::Energy(stage)));
        }
        for (index, pulse) in self.pulses.iter().enumerate() {
            events.push((pulse.charge.start.physical_tick, 3, Change::Start));
            events.push((pulse.release.physical_tick, 0, Change::Pulse(index)));
        }
        if let Some(charge) = &self.charge
            && charge.pulse.is_none()
        {
            events.push((charge.start.physical_tick, 3, Change::Start));
        }
        // ECM is a command after the accepted step, including any same-clock
        // EMP pulse/recovery. Check every historical activation, not only the tail.
        for activation in world.ecm.activations() {
            events.push((activation.physical_tick, 4, Change::Ecm));
        }
        // An incoming fatal hit suppresses only the final bank transfer, after main posting.
        // A later command that destroys the player cannot erase the already accepted transfer.
        if end > 0 && !world.damage.alive_before_emp_step(world)? {
            events.push((end, 0, Change::FinalDeath));
        }
        events.sort_by_key(|&(clock, phase, _)| (clock, phase));
        Ok(events)
    }
    pub(super) fn replay_meter(
        &self,
        world: &World,
        modular: &super::super::modular::State,
    ) -> Result<(), SaveError> {
        let end = world.combat.physical_tick();
        let events = self.meter_changes(world, modular)?;
        let mut replay = Replay {
            meter: meter::Meter::default(),
            clock: 0,
            bank_clock: 0,
            present: false,
            acquired: false,
            fitted: false,
            grades: EmpGrades::default(),
            energy: Some(0),
        };
        let mut last_special = None;
        for (clock, _, change) in events {
            if clock > end {
                return Err(INVALID);
            }
            replay.event(clock, change, &self.pulses, &mut last_special)?;
        }
        replay.advance(end)?;
        let expected_clock = if replay.acquired { end } else { 0 };
        if self.meter != replay.meter
            || self.meter_clock != expected_clock
            || self.bank_clock != replay.bank_clock
            || self.main_present != replay.present
        {
            return Err(INVALID);
        }
        Ok(())
    }
}

/// Isolated numerical fixture uses a real captured pulse identity but supplies
/// a small, balanced main reserve to exercise the disabling edge deterministically.
/// It is not a fabricated live encounter or a complete saved World.
#[cfg(test)]
pub(super) fn verify_ecm_disabled_fixture(
    mut pulse: super::runtime::Pulse,
) -> Result<(), SaveError> {
    let release = pulse.release.physical_tick;
    let main = 1;
    let paid = u64::from(policy::PULSE_COST / 4 * 5);
    let mut replay = Replay {
        meter: meter::Meter {
            main,
            bank: policy::PULSE_COST,
            generated: u64::from(main) + paid,
            main_debited: paid,
            bank_gained: u64::from(policy::PULSE_COST),
            ..meter::Meter::default()
        },
        clock: release - 1,
        bank_clock: release - 1,
        present: true,
        acquired: true,
        fitted: true,
        grades: EmpGrades::default(),
        energy: Some(0),
    };
    replay.meter.validate()?;
    let contact = pulse
        .contacts
        .iter_mut()
        .find(|c| c.victim == scene::Victim::Player)
        .ok_or(INVALID)?;
    contact.power_hundredths = 100;
    pulse.player_shield_absorbed = 0;
    pulse.player_electrical_drained = main + replay.rate();
    let pulses = [pulse];
    let mut special = None;
    replay.event(release, Change::Pulse(0), &pulses, &mut special)?;
    assert_eq!(replay.meter.disabled_at, Some(release));
    assert_eq!(
        replay.event(release, Change::Ecm, &pulses, &mut special),
        Err(INVALID)
    );
    assert_eq!(
        replay.event(release + 1, Change::Ecm, &pulses, &mut special),
        Err(INVALID)
    );
    assert_eq!(
        replay.event(
            release + policy::RECOVERY_TICKS,
            Change::Ecm,
            &pulses,
            &mut special
        ),
        Err(INVALID)
    );
    assert_eq!(replay.meter.disabled_at, Some(release));
    replay.event(
        release + policy::RECOVERY_TICKS + 1,
        Change::Ecm,
        &pulses,
        &mut special,
    )?;
    assert_eq!(replay.meter.disabled_at, None);
    replay.meter.validate()
}

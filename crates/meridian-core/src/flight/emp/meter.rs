//! Finite electrical reserve and item bank, separate from fatal hull energy.
use super::policy::{EmpGrades, MAIN_CAPACITY, PULSE_COST, RECOVERY_TICKS};
use crate::save::SaveError;
const INVALID: SaveError = SaveError::InvalidState("EMP electrical transfer conservation");
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Meter {
    pub(super) main: u32,
    pub(super) bank: u32,
    pub(super) disabled_at: Option<u64>,
    pub(super) generated: u64,
    pub(super) discarded: u64,
    pub(super) main_debited: u64,
    pub(super) bank_gained: u64,
    pub(super) bank_spent: u64,
    pub(super) pulse_drained: u64,
}
impl Meter {
    /// Fold a constant-term sequence of accepted flight intervals without replaying wall time.
    pub(super) fn advance(
        &mut self,
        start: u64,
        end: u64,
        recharge: u32,
        fitted: Option<EmpGrades>,
    ) -> Result<(), SaveError> {
        if end < start || !(210..=840).contains(&recharge) {
            return Err(INVALID);
        }
        let begin = self
            .disabled_at
            .map_or(start, |at| start.max(at.saturating_add(RECOVERY_TICKS)));
        let mut ticks = end.saturating_sub(begin);
        if ticks == 0 {
            return self.validate();
        }
        self.disabled_at = None;
        if let Some(grades) = fitted {
            let rate = grades.gain_per_tick();
            let missing = grades
                .bank_capacity()
                .checked_sub(self.bank)
                .ok_or(INVALID)?;
            let full = ticks.min(u64::from(missing / rate));
            self.transfer_span(full, recharge, rate)?;
            ticks -= full;
            let remaining = grades
                .bank_capacity()
                .checked_sub(self.bank)
                .ok_or(INVALID)?;
            if ticks > 0 && remaining > 0 {
                self.transfer_span(1, recharge, remaining)?;
                ticks -= 1;
            }
        }
        self.transfer_span(ticks, recharge, 0)?;
        self.validate()
    }
    fn transfer_span(&mut self, ticks: u64, recharge: u32, gain: u32) -> Result<(), SaveError> {
        if ticks == 0 {
            return Ok(());
        }
        if !gain.is_multiple_of(4) {
            return Err(INVALID);
        }
        let debit = gain / 4 * 5;
        if debit >= recharge {
            return Err(INVALID);
        }
        let generated = u128::from(ticks) * u128::from(recharge);
        let paid = u128::from(ticks) * u128::from(debit);
        let gained = u128::from(ticks) * u128::from(gain);
        let before = self.main;
        self.main =
            (u128::from(before) + generated - paid).min(u128::from(MAIN_CAPACITY - debit)) as u32;
        let discarded = u128::from(before) + generated - paid - u128::from(self.main);
        for (total, amount) in [
            (&mut self.generated, generated),
            (&mut self.discarded, discarded),
            (&mut self.main_debited, paid),
            (&mut self.bank_gained, gained),
        ] {
            *total = total
                .checked_add(u64::try_from(amount).map_err(|_| INVALID)?)
                .ok_or(INVALID)?;
        }
        self.bank = self
            .bank
            .checked_add(u32::try_from(gained).map_err(|_| INVALID)?)
            .ok_or(INVALID)?;
        Ok(())
    }
    /// Only an accepted flight interval calls this; docked/paused time earns nothing.
    pub(super) fn post_main(&mut self, clock: u64, recharge: u32) -> Result<(), SaveError> {
        if self
            .disabled_at
            .is_some_and(|at| clock <= at.saturating_add(RECOVERY_TICKS))
        {
            return Ok(());
        }
        let gained = recharge.min(MAIN_CAPACITY.checked_sub(self.main).ok_or(INVALID)?);
        self.main += gained;
        self.generated = self
            .generated
            .checked_add(u64::from(recharge))
            .ok_or(INVALID)?;
        self.discarded = self
            .discarded
            .checked_add(u64::from(recharge - gained))
            .ok_or(INVALID)?;
        if self.main > 0 {
            self.disabled_at = None;
        }
        self.validate()
    }
    pub(super) fn transfer(&mut self, grades: EmpGrades) -> Result<(), SaveError> {
        if self.disabled_at.is_some() || self.main == 0 {
            return Ok(());
        }
        let gain = grades
            .gain_per_tick()
            .min(
                grades
                    .bank_capacity()
                    .checked_sub(self.bank)
                    .ok_or(INVALID)?,
            )
            .min(((self.main - 1) / 5) * 4);
        // Every bank amount is a multiple of four subunits. Keep represented payment exact.
        if !gain.is_multiple_of(4) {
            return Err(INVALID);
        }
        let debit = gain / 4 * 5;
        self.main = self.main.checked_sub(debit).ok_or(INVALID)?;
        self.bank += gain;
        self.main_debited = self
            .main_debited
            .checked_add(u64::from(debit))
            .ok_or(INVALID)?;
        self.bank_gained = self
            .bank_gained
            .checked_add(u64::from(gain))
            .ok_or(INVALID)?;
        self.validate()
    }
    pub(super) fn discharge(&mut self) -> Result<(), SaveError> {
        self.bank = self.bank.checked_sub(PULSE_COST).ok_or(INVALID)?;
        self.bank_spent = self
            .bank_spent
            .checked_add(u64::from(PULSE_COST))
            .ok_or(INVALID)?;
        self.validate()
    }
    pub(super) fn drain(&mut self, power: u32, clock: u64) -> Result<u32, SaveError> {
        let debit = self.main.min(power);
        self.main -= debit;
        self.pulse_drained = self
            .pulse_drained
            .checked_add(u64::from(debit))
            .ok_or(INVALID)?;
        if self.main == 0 {
            self.disabled_at = Some(clock);
        }
        self.validate()?;
        Ok(debit)
    }
    pub(super) fn validate(self) -> Result<(), SaveError> {
        if self.main > MAIN_CAPACITY
            || self.bank > 64 * super::policy::SCALE
            || u128::from(self.generated)
                != u128::from(self.main)
                    + u128::from(self.discarded)
                    + u128::from(self.main_debited)
                    + u128::from(self.pulse_drained)
            || u128::from(self.bank_gained) != u128::from(self.bank) + u128::from(self.bank_spent)
            || u128::from(self.bank_gained) * 5 != u128::from(self.main_debited) * 4
            || self
                .disabled_at
                .is_some_and(|at| at.checked_add(RECOVERY_TICKS).is_none())
            || (self.disabled_at.is_some() && self.main != 0)
        {
            return Err(INVALID);
        }
        Ok(())
    }
}

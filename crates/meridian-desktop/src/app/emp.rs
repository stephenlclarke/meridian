//! Actual per-physical-tick held control; the core alone owns release and electrical effects.
use super::{
    App, DESTROYED_NOTICE, FlightInput, KeyCode, MotionRefusal, StepError, player_destroyed,
    step_frame,
};
impl App {
    pub(super) fn cancel_emp_input(&mut self) {
        if player_destroyed(&self.game.world) {
            return;
        }
        let tick = self.game.world.snapshot().tick;
        // A paused core cannot accept a command; no held host key survives, and the next
        // accepted step submits false before it can advance the restored charge.
        if self.game.world.emp_snapshot().is_some() {
            let _ = self.game.world.request_emp_trigger(false, tick);
        }
    }
    pub(super) fn step_emp_frame(
        &mut self,
        input: FlightInput,
        ticks: usize,
    ) -> Result<Option<MotionRefusal>, StepError> {
        let mut refusal = None;
        for _ in 0..ticks {
            if player_destroyed(&self.game.world) {
                self.clear_held_input();
                break;
            }
            let previous = self.game.world.emp_snapshot().and_then(|s| s.charge_phase);
            let held = self.keys.contains(&KeyCode::KeyU);
            if held || self.game.world.emp_snapshot().is_some() {
                let tick = self.game.world.snapshot().tick;
                if let Err(error) = self.game.world.request_emp_trigger(held, tick)
                    && held
                {
                    self.notice.set(error.to_string());
                }
            }
            let mut fired = false;
            refusal = step_frame(
                &mut self.game.world,
                input,
                1,
                self.keys.contains(&KeyCode::KeyZ),
                &mut self.pulse,
                &mut self.mining,
                || fired = true,
            )?
            .or(refusal);
            self.emp_feedback(previous);
            self.publish_audio(fired);
            if player_destroyed(&self.game.world) {
                self.clear_held_input();
                self.notice.set(DESTROYED_NOTICE.into());
                break;
            }
            // Stop catch-up at the first accepted physical hold that needs consent.
            if self.maybe_open_occupied_cut() {
                break;
            }
        }
        Ok(refusal)
    }
    fn emp_feedback(&mut self, previous: Option<meridian_core::EmpChargePhase>) {
        use meridian_core::EmpChargePhase::{Cancelled, Charging, Released};
        let Some(s) = self.game.world.emp_snapshot() else {
            return;
        };
        if s.charge_phase == previous {
            return;
        }
        let message = match s.charge_phase {
            Some(Charging) => "EMP charging; hold U for ten seconds; release cancels",
            Some(Cancelled) => "EMP charge cancelled; no pulse spent",
            Some(Released) if s.charge_release.is_some() => {
                "EMP pulse released; shield/electrical effects only"
            }
            Some(Released) => {
                "EMP release refused; no pulse spent; release U before another attempt"
            }
            None => return,
        };
        self.notice.set(message.into());
    }
}

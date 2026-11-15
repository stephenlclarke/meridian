#[derive(Default)]
pub struct FixedClock {
    remainder: f64,
    discard_next_elapsed: bool,
}

impl FixedClock {
    pub fn reset(&mut self) {
        self.remainder = 0.0;
        self.discard_next_elapsed = true;
    }

    pub fn ticks(&mut self, elapsed: f64, active: bool) -> usize {
        if !active || !elapsed.is_finite() || elapsed < 0.0 {
            self.reset();
            return 0;
        }
        if self.discard_next_elapsed {
            self.discard_next_elapsed = false;
            return 0;
        }
        // A stalled desktop must not build an unbounded simulation backlog.
        self.remainder = (self.remainder + elapsed).min(0.1);
        let tick = 1.0 / 60.0;
        let count = (self.remainder / tick).floor() as usize;
        self.remainder -= count as f64 * tick;
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_frames_retain_fractional_ticks() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.ticks(1.0 / 120.0, true), 0);
        assert_eq!(clock.ticks(1.0 / 120.0, true), 1);
    }

    #[test]
    fn stall_is_bounded_and_pause_does_not_catch_up() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.ticks(60.0, true), 6);
        assert_eq!(clock.ticks(60.0, false), 0);
        assert_eq!(clock.ticks(0.0, true), 0);
        assert_eq!(clock.ticks(f64::NAN, true), 0);
        assert_eq!(clock.ticks(-1.0, true), 0);
    }

    #[test]
    fn pause_without_redraw_discards_time_on_first_resumed_frame() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.ticks(1.0 / 120.0, true), 0);
        // A paused window can receive no redraws at all before the resume event.
        clock.reset();
        assert_eq!(clock.ticks(600.0, true), 0);
        assert_eq!(clock.ticks(1.0 / 60.0, true), 1);
    }
}

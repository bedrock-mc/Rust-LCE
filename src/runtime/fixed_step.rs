use std::time::Duration;

use crate::core::timing::{MAX_TICKS_PER_UPDATE, tick_duration};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickDispatch {
    pub ticks_to_run: u32,
    pub alpha: f64,
    pub dropped_ticks: u32,
}

#[derive(Debug, Clone)]
pub struct FixedStepLoop {
    step: Duration,
    max_ticks_per_update: u32,
    accumulated_nanos: u128,
}

impl FixedStepLoop {
    pub fn new(step: Duration, max_ticks_per_update: u32) -> Self {
        assert!(!step.is_zero(), "step duration must be greater than zero");
        assert!(
            max_ticks_per_update > 0,
            "max_ticks_per_update must be at least one"
        );

        Self {
            step,
            max_ticks_per_update,
            accumulated_nanos: 0,
        }
    }

    pub fn lce_default() -> Self {
        Self::new(tick_duration(), MAX_TICKS_PER_UPDATE)
    }

    pub fn step(&self) -> Duration {
        self.step
    }

    pub fn max_ticks_per_update(&self) -> u32 {
        self.max_ticks_per_update
    }

    pub fn pending_fraction(&self) -> f64 {
        self.accumulated_nanos as f64 / self.step.as_nanos() as f64
    }

    pub fn reset(&mut self) {
        self.accumulated_nanos = 0;
    }

    pub fn update(&mut self, frame_delta: Duration) -> TickDispatch {
        let step_nanos = self.step.as_nanos();
        self.accumulated_nanos = self
            .accumulated_nanos
            .saturating_add(frame_delta.as_nanos());

        let available_ticks = self.accumulated_nanos / step_nanos;
        let ticks_to_run_u128 = available_ticks.min(u128::from(self.max_ticks_per_update));
        let dropped_ticks_u128 = available_ticks.saturating_sub(ticks_to_run_u128);

        self.accumulated_nanos %= step_nanos;

        let ticks_to_run = u32::try_from(ticks_to_run_u128).unwrap_or(self.max_ticks_per_update);
        let dropped_ticks = u32::try_from(dropped_ticks_u128).unwrap_or(u32::MAX);
        let alpha = self.pending_fraction();

        TickDispatch {
            ticks_to_run,
            alpha,
            dropped_ticks,
        }
    }
}

impl Default for FixedStepLoop {
    fn default() -> Self {
        Self::lce_default()
    }
}

use std::time::Duration;

use lce_rust::core::timing::{MAX_TICKS_PER_UPDATE, TICKS_PER_SECOND, tick_duration};
use lce_rust::runtime::FixedStepLoop;

const EPSILON: f64 = 1e-9;

#[test]
fn uses_lce_timing_defaults() {
    let runtime = FixedStepLoop::default();

    assert_eq!(TICKS_PER_SECOND, 20);
    assert_eq!(tick_duration(), Duration::from_millis(50));
    assert_eq!(runtime.step(), Duration::from_millis(50));
    assert_eq!(runtime.max_ticks_per_update(), MAX_TICKS_PER_UPDATE);
}

#[test]
fn accumulates_fractional_frame_time_across_updates() {
    let mut runtime = FixedStepLoop::default();

    let first = runtime.update(Duration::from_millis(30));
    assert_eq!(first.ticks_to_run, 0);
    assert_eq!(first.dropped_ticks, 0);
    assert!((first.alpha - 0.6).abs() < EPSILON);

    let second = runtime.update(Duration::from_millis(30));
    assert_eq!(second.ticks_to_run, 1);
    assert_eq!(second.dropped_ticks, 0);
    assert!((second.alpha - 0.2).abs() < EPSILON);
}

#[test]
fn clamps_large_frame_spikes_and_reports_dropped_ticks() {
    let mut runtime = FixedStepLoop::default();

    let dispatch = runtime.update(Duration::from_secs(2));

    assert_eq!(dispatch.ticks_to_run, MAX_TICKS_PER_UPDATE);
    assert_eq!(dispatch.dropped_ticks, 30);
    assert!(dispatch.alpha.abs() < EPSILON);
}

#[test]
fn reset_clears_pending_fraction() {
    let mut runtime = FixedStepLoop::default();

    runtime.update(Duration::from_millis(35));
    assert!(runtime.pending_fraction() > 0.0);

    runtime.reset();

    assert!(runtime.pending_fraction().abs() < EPSILON);
}

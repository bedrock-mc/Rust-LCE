use std::time::Duration;

pub const TICKS_PER_SECOND: u32 = 20;
pub const MAX_TICKS_PER_UPDATE: u32 = 10;
pub const MILLIS_PER_SECOND: u64 = 1_000;
pub const MILLIS_PER_TICK: u64 = MILLIS_PER_SECOND / TICKS_PER_SECOND as u64;

pub fn tick_duration() -> Duration {
    Duration::from_millis(MILLIS_PER_TICK)
}

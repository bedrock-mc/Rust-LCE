use std::collections::BTreeSet;

use crate::world::{
    ChunkLifecycleEvent, ChunkPos, DAY_LENGTH_TICKS, ScheduledTick, ScheduledTickKind, WeatherKind,
};

pub const DEFAULT_BOOT_DAY_TIME: u64 = 6_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeEnvironment {
    pub weather: WeatherKind,
    pub day_time: u64,
    pub sky_brightness: f32,
}

impl Default for RuntimeEnvironment {
    fn default() -> Self {
        let weather = WeatherKind::Clear;
        let day_time = 0;

        Self {
            weather,
            day_time,
            sky_brightness: sky_brightness_for(day_time, weather),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LifecycleHookBatch {
    pub relight_chunks: BTreeSet<ChunkPos>,
    pub triggered_block_ticks: Vec<ScheduledTick>,
    pub triggered_tile_ticks: Vec<ScheduledTick>,
    pub weather_changed: bool,
    pub time_advanced: bool,
}

pub fn consume_lifecycle_events(
    environment: &mut RuntimeEnvironment,
    events: &[ChunkLifecycleEvent],
) -> LifecycleHookBatch {
    let mut batch = LifecycleHookBatch::default();

    for event in events {
        match *event {
            ChunkLifecycleEvent::ChunkLoaded { chunk }
            | ChunkLifecycleEvent::ChunkActivated { chunk }
            | ChunkLifecycleEvent::ChunkDeactivated { chunk }
            | ChunkLifecycleEvent::ChunkUnloaded { chunk } => {
                let _ = chunk;
            }
            ChunkLifecycleEvent::ChunkTicked { chunk, .. } => {
                let _ = chunk;
            }
            ChunkLifecycleEvent::TimeAdvanced { day_time, .. } => {
                environment.day_time = day_time;
                batch.time_advanced = true;
            }
            ChunkLifecycleEvent::WeatherChanged { to, .. } => {
                environment.weather = to;
                batch.weather_changed = true;
            }
            ChunkLifecycleEvent::TickScheduled { .. } => {}
            ChunkLifecycleEvent::TickTriggered { tick } => match tick.kind {
                ScheduledTickKind::Block => batch.triggered_block_ticks.push(tick),
                ScheduledTickKind::Tile => batch.triggered_tile_ticks.push(tick),
            },
        }
    }

    if batch.time_advanced || batch.weather_changed {
        environment.sky_brightness = sky_brightness_for(environment.day_time, environment.weather);
    }

    batch
}

pub fn sky_brightness_for(day_time: u64, weather: WeatherKind) -> f32 {
    let wrapped_day = day_time % DAY_LENGTH_TICKS;
    let day_fraction = wrapped_day as f32 / DAY_LENGTH_TICKS as f32;
    let daylight =
        (((day_fraction - 0.25) * std::f32::consts::TAU).cos() * 0.5 + 0.5).clamp(0.0, 1.0);

    let weather_attenuation = match weather {
        WeatherKind::Clear => 1.0,
        WeatherKind::Rain => 0.78,
        WeatherKind::Thunder => 0.55,
    };

    (0.12 + daylight * 0.88 * weather_attenuation).clamp(0.05, 1.0)
}

pub fn align_total_ticks_to_day_time(total_ticks: u64, day_time: u64) -> u64 {
    let day_index = total_ticks / DAY_LENGTH_TICKS;
    let target_day_time = day_time % DAY_LENGTH_TICKS;
    day_index
        .saturating_mul(DAY_LENGTH_TICKS)
        .saturating_add(target_day_time)
}

pub fn sky_color_from_brightness(brightness: f32) -> (f32, f32, f32) {
    let value = brightness.clamp(0.05, 1.0);
    let red = 0.07 + value * 0.38;
    let green = 0.10 + value * 0.56;
    let blue = 0.16 + value * 0.76;

    (
        red.clamp(0.0, 1.0),
        green.clamp(0.0, 1.0),
        blue.clamp(0.0, 1.0),
    )
}

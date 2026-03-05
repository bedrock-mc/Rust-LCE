use lce_rust::client::lifecycle_hooks::{
    DEFAULT_BOOT_DAY_TIME, RuntimeEnvironment, align_total_ticks_to_day_time,
    consume_lifecycle_events, sky_brightness_for,
};
use lce_rust::world::{
    BlockPos, ChunkLifecycleEvent, ChunkPos, DAY_LENGTH_TICKS, ScheduledTick, ScheduledTickKind,
    WeatherKind,
};

#[test]
fn consumes_time_and_weather_events_into_runtime_environment() {
    let mut environment = RuntimeEnvironment::default();

    let events = [
        ChunkLifecycleEvent::TimeAdvanced {
            total_ticks: 6_000,
            day_time: 6_000,
        },
        ChunkLifecycleEvent::WeatherChanged {
            from: WeatherKind::Clear,
            to: WeatherKind::Rain,
        },
    ];

    let batch = consume_lifecycle_events(&mut environment, &events);

    assert!(batch.time_advanced);
    assert!(batch.weather_changed);
    assert_eq!(environment.day_time, 6_000);
    assert_eq!(environment.weather, WeatherKind::Rain);
    assert_eq!(
        environment.sky_brightness,
        sky_brightness_for(6_000, WeatherKind::Rain)
    );
}

#[test]
fn classifies_triggered_ticks_without_forcing_chunk_relight() {
    let mut environment = RuntimeEnvironment::default();
    let shared_chunk = ChunkPos::new(1, -2);
    let other_chunk = ChunkPos::new(3, 4);

    let events = [
        ChunkLifecycleEvent::ChunkLoaded {
            chunk: shared_chunk,
        },
        ChunkLifecycleEvent::TickTriggered {
            tick: ScheduledTick {
                id: 2,
                kind: ScheduledTickKind::Block,
                block: BlockPos::new(16, 70, -17),
                chunk: shared_chunk,
                payload_id: 5,
                execute_at: 40,
            },
        },
        ChunkLifecycleEvent::TickTriggered {
            tick: ScheduledTick {
                id: 3,
                kind: ScheduledTickKind::Tile,
                block: BlockPos::new(49, 30, 64),
                chunk: other_chunk,
                payload_id: 44,
                execute_at: 40,
            },
        },
    ];

    let batch = consume_lifecycle_events(&mut environment, &events);

    assert_eq!(batch.triggered_block_ticks.len(), 1);
    assert_eq!(batch.triggered_tile_ticks.len(), 1);
    assert!(batch.relight_chunks.is_empty());
}

#[test]
fn sky_brightness_reflects_weather_and_day_cycle() {
    let clear_midday = sky_brightness_for(6_000, WeatherKind::Clear);
    let clear_midnight = sky_brightness_for(18_000, WeatherKind::Clear);
    let rainy_midday = sky_brightness_for(6_000, WeatherKind::Rain);
    let thunder_midday = sky_brightness_for(6_000, WeatherKind::Thunder);

    assert!(clear_midday > clear_midnight);
    assert!(rainy_midday < clear_midday);
    assert!(thunder_midday < rainy_midday);
}

#[test]
fn boot_time_alignment_forces_default_daytime_without_losing_day_index() {
    let total_ticks = DAY_LENGTH_TICKS * 4 + 137;
    let aligned = align_total_ticks_to_day_time(total_ticks, DEFAULT_BOOT_DAY_TIME);
    assert_eq!(aligned, DAY_LENGTH_TICKS * 4 + DEFAULT_BOOT_DAY_TIME);

    let wrapped_target = align_total_ticks_to_day_time(32, DAY_LENGTH_TICKS + 15);
    assert_eq!(wrapped_target, 15);
}

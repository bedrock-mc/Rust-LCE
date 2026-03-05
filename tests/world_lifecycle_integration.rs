use lce_rust::world::{
    BlockPos, ChunkLifecycleController, ChunkLifecycleEvent, ChunkPos, DAY_LENGTH_TICKS,
    ScheduledTick, ScheduledTickKind, WeatherKind,
};

#[test]
fn load_activate_tick_and_unload_emit_expected_events() {
    let chunk = ChunkPos::new(2, -3);
    let mut lifecycle = ChunkLifecycleController::new();

    assert!(lifecycle.load_chunk(chunk));
    assert!(lifecycle.set_chunk_active(chunk, true));
    lifecycle.tick_once();
    assert!(lifecycle.unload_chunk(chunk));

    let events = lifecycle.drain_events();
    assert_eq!(events.len(), 6);

    assert_eq!(events[0], ChunkLifecycleEvent::ChunkLoaded { chunk });
    assert_eq!(events[1], ChunkLifecycleEvent::ChunkActivated { chunk });
    assert_eq!(
        events[2],
        ChunkLifecycleEvent::TimeAdvanced {
            total_ticks: 1,
            day_time: 1,
        }
    );
    assert_eq!(
        events[3],
        ChunkLifecycleEvent::ChunkTicked {
            chunk,
            world_tick: 1,
            chunk_tick_count: 1,
        }
    );
    assert_eq!(events[4], ChunkLifecycleEvent::ChunkDeactivated { chunk });
    assert_eq!(events[5], ChunkLifecycleEvent::ChunkUnloaded { chunk });
}

#[test]
fn ticking_only_advances_active_chunks() {
    let chunk_a = ChunkPos::new(0, 0);
    let chunk_b = ChunkPos::new(1, 0);
    let mut lifecycle = ChunkLifecycleController::new();

    lifecycle.load_chunk(chunk_a);
    lifecycle.load_chunk(chunk_b);
    lifecycle.set_chunk_active(chunk_a, true);
    lifecycle.tick_many(3);

    assert_eq!(lifecycle.chunk_tick_count(chunk_a), 3);
    assert_eq!(lifecycle.chunk_tick_count(chunk_b), 0);

    lifecycle.set_chunk_active(chunk_b, true);
    lifecycle.tick_many(2);

    assert_eq!(lifecycle.chunk_tick_count(chunk_a), 5);
    assert_eq!(lifecycle.chunk_tick_count(chunk_b), 2);
}

#[test]
fn cannot_activate_unloaded_chunk() {
    let chunk = ChunkPos::new(-2, 4);
    let mut lifecycle = ChunkLifecycleController::new();

    assert!(!lifecycle.set_chunk_active(chunk, true));
    assert!(lifecycle.drain_events().is_empty());
}

#[test]
fn unload_cancels_pending_ticks_for_that_chunk_only() {
    let chunk_a = ChunkPos::new(0, 0);
    let chunk_b = ChunkPos::new(1, 0);
    let block_a = BlockPos::new(1, 64, 1);
    let block_b = BlockPos::new(17, 64, 1);

    let mut lifecycle = ChunkLifecycleController::new();
    lifecycle.load_chunk(chunk_a);
    lifecycle.load_chunk(chunk_b);

    lifecycle.schedule_block_tick(block_a, 11, 2);
    lifecycle.schedule_block_tick(block_b, 22, 2);
    assert_eq!(lifecycle.pending_scheduled_tick_count(), 2);

    assert!(lifecycle.unload_chunk(chunk_a));
    assert_eq!(lifecycle.pending_scheduled_tick_count(), 1);

    lifecycle.tick_many(2);
    let due = lifecycle.drain_triggered_ticks();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].chunk, chunk_b);
}

#[test]
fn unload_drops_triggered_ticks_waiting_for_runtime_consumption() {
    let chunk = ChunkPos::new(-3, 2);
    let block = BlockPos::new(-48, 70, 33);

    let mut lifecycle = ChunkLifecycleController::new();
    lifecycle.load_chunk(chunk);
    lifecycle.schedule_block_tick(block, 1, 1);

    lifecycle.tick_once();
    assert_eq!(lifecycle.drain_triggered_ticks().len(), 1);

    lifecycle.schedule_block_tick(block, 1, 1);
    lifecycle.tick_once();
    assert_eq!(lifecycle.pending_scheduled_tick_count(), 0);

    assert!(lifecycle.unload_chunk(chunk));
    assert!(lifecycle.drain_triggered_ticks().is_empty());
}

#[test]
fn weather_change_and_day_time_rollover_are_tracked() {
    let mut lifecycle = ChunkLifecycleController::new();

    assert!(lifecycle.set_weather(WeatherKind::Rain));
    assert!(!lifecycle.set_weather(WeatherKind::Rain));
    assert!(lifecycle.set_weather(WeatherKind::Thunder));

    lifecycle.tick_many((DAY_LENGTH_TICKS + 5) as u32);
    let time = lifecycle.time();

    assert_eq!(time.total_ticks, DAY_LENGTH_TICKS + 5);
    assert_eq!(time.day_time, 5);
    assert_eq!(lifecycle.weather().kind, WeatherKind::Thunder);

    let weather_events: Vec<_> = lifecycle
        .drain_events()
        .into_iter()
        .filter(|event| matches!(event, ChunkLifecycleEvent::WeatherChanged { .. }))
        .collect();

    assert_eq!(weather_events.len(), 2);
    assert_eq!(
        weather_events[0],
        ChunkLifecycleEvent::WeatherChanged {
            from: WeatherKind::Clear,
            to: WeatherKind::Rain,
        }
    );
    assert_eq!(
        weather_events[1],
        ChunkLifecycleEvent::WeatherChanged {
            from: WeatherKind::Rain,
            to: WeatherKind::Thunder,
        }
    );
}

#[test]
fn can_bootstrap_lifecycle_time_from_existing_world_tick() {
    let mut lifecycle = ChunkLifecycleController::with_total_ticks(DAY_LENGTH_TICKS * 2 + 123);
    assert_eq!(lifecycle.time().total_ticks, DAY_LENGTH_TICKS * 2 + 123);
    assert_eq!(lifecycle.time().day_time, 123);

    lifecycle.tick_once();
    assert_eq!(lifecycle.time().total_ticks, DAY_LENGTH_TICKS * 2 + 124);
    assert_eq!(lifecycle.time().day_time, 124);
}

#[test]
fn scheduled_ticks_fire_at_due_tick_with_stable_ordering() {
    let mut lifecycle = ChunkLifecycleController::with_total_ticks(100);

    let block_b = BlockPos::new(4, 64, 4);
    let block_c = BlockPos::new(5, 64, 4);
    let block_a = BlockPos::new(6, 64, 4);

    let id_b = lifecycle.schedule_tile_tick(block_b, 88, 1);
    let id_c = lifecycle.schedule_block_tick(block_c, 1, 1);
    let id_a = lifecycle.schedule_block_tick(block_a, 2, 2);

    assert_eq!(lifecycle.pending_scheduled_tick_count(), 3);

    lifecycle.tick_once();
    let first_due = lifecycle.drain_triggered_ticks();
    assert_eq!(first_due.len(), 2);
    assert_eq!(
        first_due[0],
        ScheduledTick {
            id: id_b,
            kind: ScheduledTickKind::Tile,
            block: block_b,
            chunk: ChunkPos::new(0, 0),
            payload_id: 88,
            execute_at: 101,
        }
    );
    assert_eq!(
        first_due[1],
        ScheduledTick {
            id: id_c,
            kind: ScheduledTickKind::Block,
            block: block_c,
            chunk: ChunkPos::new(0, 0),
            payload_id: 1,
            execute_at: 101,
        }
    );

    lifecycle.tick_once();
    let second_due = lifecycle.drain_triggered_ticks();
    assert_eq!(second_due.len(), 1);
    assert_eq!(
        second_due[0],
        ScheduledTick {
            id: id_a,
            kind: ScheduledTickKind::Block,
            block: block_a,
            chunk: ChunkPos::new(0, 0),
            payload_id: 2,
            execute_at: 102,
        }
    );

    assert_eq!(lifecycle.pending_scheduled_tick_count(), 0);
}

#[test]
fn zero_delay_tick_schedules_for_next_tick_not_immediate() {
    let mut lifecycle = ChunkLifecycleController::new();
    lifecycle.schedule_block_tick(BlockPos::new(16, 64, 16), 9, 0);

    assert!(lifecycle.drain_triggered_ticks().is_empty());

    lifecycle.tick_once();
    let due = lifecycle.drain_triggered_ticks();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].execute_at, 1);
}

#[test]
fn duplicate_tick_requests_for_same_target_do_not_accumulate() {
    let mut lifecycle = ChunkLifecycleController::new();
    let block = BlockPos::new(5, 70, 5);

    let first_id = lifecycle.schedule_block_tick(block, 42, 3);
    let second_id = lifecycle.schedule_block_tick(block, 42, 8);

    assert_eq!(first_id, second_id);
    assert_eq!(lifecycle.pending_scheduled_tick_count(), 1);

    lifecycle.tick_many(3);
    let due = lifecycle.drain_triggered_ticks();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id, first_id);
    assert_eq!(due[0].execute_at, 3);
}

#[test]
fn earlier_tick_request_reschedules_existing_entry_sooner() {
    let mut lifecycle = ChunkLifecycleController::new();
    let block = BlockPos::new(-4, 66, 12);

    let original_id = lifecycle.schedule_block_tick(block, 7, 8);
    let rescheduled_id = lifecycle.schedule_block_tick(block, 7, 2);

    assert_eq!(original_id, rescheduled_id);
    assert_eq!(lifecycle.pending_scheduled_tick_count(), 1);

    lifecycle.tick_once();
    assert!(lifecycle.drain_triggered_ticks().is_empty());

    lifecycle.tick_once();
    let due = lifecycle.drain_triggered_ticks();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id, original_id);
    assert_eq!(due[0].execute_at, 2);
}

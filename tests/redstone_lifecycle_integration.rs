use lce_rust::world::{
    BlockPos, BlockWorld, ChunkLifecycleController, LEVER_BLOCK_ID, REDSTONE_TORCH_OFF_BLOCK_ID,
    REDSTONE_TORCH_ON_BLOCK_ID, REDSTONE_WIRE_BLOCK_ID, REDSTONE_WIRE_POWERED_BLOCK_ID,
    REDSTONE_WIRE_TICK_DELAY, REPEATER_OFF_BLOCK_ID, REPEATER_ON_BLOCK_ID,
    process_scheduled_redstone_tick, redstone_tick_for_placement, redstone_ticks_for_block_change,
};

#[test]
fn placement_tick_metadata_is_emitted_for_redstone_components() {
    let wire_block = BlockPos::new(1, 64, 1);
    let torch_block = BlockPos::new(2, 64, 1);
    let repeater_block = BlockPos::new(3, 64, 1);

    let wire_tick =
        redstone_tick_for_placement(wire_block, REDSTONE_WIRE_BLOCK_ID).expect("wire tick");
    assert_eq!(wire_tick.block, wire_block);
    assert_eq!(wire_tick.payload_id, REDSTONE_WIRE_BLOCK_ID);
    assert_eq!(wire_tick.delay_ticks, REDSTONE_WIRE_TICK_DELAY);

    let torch_tick =
        redstone_tick_for_placement(torch_block, REDSTONE_TORCH_ON_BLOCK_ID).expect("torch tick");
    assert_eq!(torch_tick.block, torch_block);
    assert_eq!(torch_tick.payload_id, REDSTONE_TORCH_ON_BLOCK_ID);

    let repeater_tick =
        redstone_tick_for_placement(repeater_block, REPEATER_OFF_BLOCK_ID).expect("repeater tick");
    assert_eq!(repeater_tick.block, repeater_block);
    assert_eq!(repeater_tick.payload_id, REPEATER_OFF_BLOCK_ID);

    assert!(redstone_tick_for_placement(BlockPos::new(0, 64, 0), LEVER_BLOCK_ID).is_none());
}

#[test]
fn powered_wire_uses_internal_non_vanilla_block_id() {
    assert!(REDSTONE_WIRE_POWERED_BLOCK_ID > 255);
}

#[test]
fn wire_turns_on_from_adjacent_lever_and_turns_off_after_source_break() {
    let mut world = BlockWorld::new();
    let mut lifecycle = ChunkLifecycleController::new();

    let lever = BlockPos::new(0, 64, 0);
    let wire = BlockPos::new(1, 64, 0);

    world.place_block(lever, LEVER_BLOCK_ID);
    world.place_block(wire, REDSTONE_WIRE_BLOCK_ID);

    lifecycle.schedule_block_tick(wire, REDSTONE_WIRE_BLOCK_ID, REDSTONE_WIRE_TICK_DELAY);
    run_redstone_ticks(&mut world, &mut lifecycle, 1);
    assert_eq!(world.block_id(wire), REDSTONE_WIRE_POWERED_BLOCK_ID);

    world.break_block(lever);
    for scheduled in redstone_ticks_for_block_change(&world, lever, None) {
        lifecycle.schedule_block_tick(scheduled.block, scheduled.payload_id, scheduled.delay_ticks);
    }

    run_redstone_ticks(&mut world, &mut lifecycle, 3);
    assert_eq!(world.block_id(wire), REDSTONE_WIRE_BLOCK_ID);
}

#[test]
fn repeater_tracks_neighbor_wire_power_state() {
    let mut world = BlockWorld::new();
    let mut lifecycle = ChunkLifecycleController::new();

    let lever = BlockPos::new(0, 64, 0);
    let wire = BlockPos::new(1, 64, 0);
    let repeater = BlockPos::new(2, 64, 0);

    world.place_block(lever, LEVER_BLOCK_ID);
    world.place_block(wire, REDSTONE_WIRE_BLOCK_ID);
    world.place_block(repeater, REPEATER_OFF_BLOCK_ID);

    for scheduled in redstone_ticks_for_block_change(&world, lever, Some(LEVER_BLOCK_ID)) {
        lifecycle.schedule_block_tick(scheduled.block, scheduled.payload_id, scheduled.delay_ticks);
    }

    run_redstone_ticks(&mut world, &mut lifecycle, 4);
    assert_eq!(world.block_id(wire), REDSTONE_WIRE_POWERED_BLOCK_ID);
    assert_eq!(world.block_id(repeater), REPEATER_ON_BLOCK_ID);

    world.break_block(lever);
    for scheduled in redstone_ticks_for_block_change(&world, lever, None) {
        lifecycle.schedule_block_tick(scheduled.block, scheduled.payload_id, scheduled.delay_ticks);
    }

    run_redstone_ticks(&mut world, &mut lifecycle, 6);
    assert_eq!(world.block_id(wire), REDSTONE_WIRE_BLOCK_ID);
    assert_eq!(world.block_id(repeater), REPEATER_OFF_BLOCK_ID);
}

#[test]
fn torch_turns_off_when_powered_and_back_on_when_power_removed() {
    let mut world = BlockWorld::new();
    let mut lifecycle = ChunkLifecycleController::new();

    let lever = BlockPos::new(0, 64, 0);
    let torch = BlockPos::new(1, 64, 0);

    world.place_block(lever, LEVER_BLOCK_ID);
    world.place_block(torch, REDSTONE_TORCH_ON_BLOCK_ID);

    for scheduled in redstone_ticks_for_block_change(&world, lever, Some(LEVER_BLOCK_ID)) {
        lifecycle.schedule_block_tick(scheduled.block, scheduled.payload_id, scheduled.delay_ticks);
    }

    run_redstone_ticks(&mut world, &mut lifecycle, 3);
    assert_eq!(world.block_id(torch), REDSTONE_TORCH_OFF_BLOCK_ID);

    world.break_block(lever);
    for scheduled in redstone_ticks_for_block_change(&world, lever, None) {
        lifecycle.schedule_block_tick(scheduled.block, scheduled.payload_id, scheduled.delay_ticks);
    }

    run_redstone_ticks(&mut world, &mut lifecycle, 3);
    assert_eq!(world.block_id(torch), REDSTONE_TORCH_ON_BLOCK_ID);
}

fn run_redstone_ticks(
    world: &mut BlockWorld,
    lifecycle: &mut ChunkLifecycleController,
    ticks: u32,
) {
    for _ in 0..ticks {
        lifecycle.tick_once();

        let due_ticks = lifecycle.drain_triggered_ticks();
        for tick in due_ticks {
            if let Some(outcome) = process_scheduled_redstone_tick(world, tick) {
                for scheduled in outcome.scheduled_ticks {
                    lifecycle.schedule_block_tick(
                        scheduled.block,
                        scheduled.payload_id,
                        scheduled.delay_ticks,
                    );
                }
            }
        }
    }
}

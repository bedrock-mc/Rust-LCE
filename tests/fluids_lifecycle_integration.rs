use lce_rust::world::{
    BlockPos, BlockWorld, ChunkLifecycleController, LAVA_SOURCE_BLOCK_ID, LAVA_TICK_DELAY,
    WATER_FLOWING_BLOCK_ID, WATER_SOURCE_BLOCK_ID, WATER_TICK_DELAY, fluid_tick_for_placement,
    fluid_ticks_for_block_change, process_fluid_tick, process_scheduled_fluid_tick,
};

#[test]
fn placing_fluid_blocks_emits_expected_schedule_metadata() {
    let water_block = BlockPos::new(2, 70, -1);
    let lava_block = BlockPos::new(-3, 65, 9);

    let water_tick = fluid_tick_for_placement(water_block, WATER_SOURCE_BLOCK_ID)
        .expect("water should schedule");
    assert_eq!(water_tick.block, water_block);
    assert_eq!(water_tick.payload_id, WATER_SOURCE_BLOCK_ID);
    assert_eq!(water_tick.delay_ticks, WATER_TICK_DELAY);

    let lava_tick =
        fluid_tick_for_placement(lava_block, LAVA_SOURCE_BLOCK_ID).expect("lava should schedule");
    assert_eq!(lava_tick.block, lava_block);
    assert_eq!(lava_tick.payload_id, LAVA_SOURCE_BLOCK_ID);
    assert_eq!(lava_tick.delay_ticks, LAVA_TICK_DELAY);

    assert!(fluid_tick_for_placement(BlockPos::new(0, 64, 0), 1).is_none());
}

#[test]
fn scheduled_water_tick_spreads_downward_with_falling_depth() {
    let mut world = BlockWorld::new();
    let source = BlockPos::new(0, 64, 0);
    let below = BlockPos::new(0, 63, 0);

    world.place_block(source, WATER_SOURCE_BLOCK_ID);
    world.set_block_data(source, 0);

    let mut lifecycle = ChunkLifecycleController::new();
    lifecycle.schedule_block_tick(source, WATER_SOURCE_BLOCK_ID, WATER_TICK_DELAY);
    lifecycle.tick_many(WATER_TICK_DELAY);

    let due = lifecycle.drain_triggered_ticks();
    assert_eq!(due.len(), 1);

    let outcome =
        process_scheduled_fluid_tick(&mut world, due[0]).expect("water tick should process");

    assert_eq!(world.block_id(below), WATER_SOURCE_BLOCK_ID);
    assert_eq!(world.block_data(below), 8);
    assert!(outcome.changed_blocks.contains(&below));
    assert!(
        outcome
            .scheduled_ticks
            .iter()
            .any(|scheduled| scheduled.block == below)
    );
}

#[test]
fn flowing_water_without_source_support_evaporates() {
    let mut world = BlockWorld::new();
    let flowing = BlockPos::new(3, 70, 3);

    world.place_block(flowing, WATER_SOURCE_BLOCK_ID);
    world.set_block_data(flowing, 3);
    world.place_block(BlockPos::new(3, 69, 3), 1);

    let outcome = process_fluid_tick(&mut world, flowing, WATER_SOURCE_BLOCK_ID)
        .expect("tick should process");

    assert_eq!(world.block_id(flowing), 0);
    assert!(outcome.changed_blocks.contains(&flowing));
}

#[test]
fn static_water_reacts_to_neighbor_change_and_spreads_horizontally() {
    let mut world = BlockWorld::new();
    let source = BlockPos::new(10, 70, 10);

    world.place_block(source, WATER_FLOWING_BLOCK_ID);
    world.set_block_data(source, 0);
    world.place_block(BlockPos::new(10, 69, 10), 1);

    let outcome = process_fluid_tick(&mut world, source, WATER_FLOWING_BLOCK_ID)
        .expect("tick should process");

    let expected_neighbors = [
        BlockPos::new(9, 70, 10),
        BlockPos::new(11, 70, 10),
        BlockPos::new(10, 70, 9),
        BlockPos::new(10, 70, 11),
    ];

    for neighbor in expected_neighbors {
        assert_eq!(world.block_id(neighbor), WATER_SOURCE_BLOCK_ID);
        assert_eq!(world.block_data(neighbor), 1);
        assert!(outcome.changed_blocks.contains(&neighbor));
    }
}

#[test]
fn lava_flowing_into_water_turns_target_to_stone() {
    let mut world = BlockWorld::new();
    let lava = BlockPos::new(-4, 80, 3);
    let below = BlockPos::new(-4, 79, 3);

    world.place_block(lava, LAVA_SOURCE_BLOCK_ID);
    world.set_block_data(lava, 0);
    world.place_block(below, WATER_FLOWING_BLOCK_ID);
    world.set_block_data(below, 0);

    let outcome =
        process_fluid_tick(&mut world, lava, LAVA_SOURCE_BLOCK_ID).expect("lava should flow");

    assert_eq!(world.block_id(below), 1);
    assert!(outcome.changed_blocks.contains(&below));
}

#[test]
fn fluid_ticks_for_block_change_captures_adjacent_and_placed_fluids() {
    let mut world = BlockWorld::new();
    let changed = BlockPos::new(0, 64, 0);
    let adjacent_water = BlockPos::new(1, 64, 0);
    world.place_block(adjacent_water, WATER_FLOWING_BLOCK_ID);

    let scheduled = fluid_ticks_for_block_change(&world, changed, Some(WATER_SOURCE_BLOCK_ID));

    assert!(
        scheduled
            .iter()
            .any(|tick| tick.block == adjacent_water && tick.payload_id == WATER_SOURCE_BLOCK_ID)
    );
    assert!(
        scheduled
            .iter()
            .any(|tick| tick.block == changed && tick.payload_id == WATER_SOURCE_BLOCK_ID)
    );
}

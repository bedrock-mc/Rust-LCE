use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use lce_rust::world::{BlockPos, BlockWorld, ChunkPos, WATER_SOURCE_BLOCK_ID};

#[test]
fn placed_blocks_round_trip_through_chunk_storage() {
    let root = unique_temp_directory("block_round_trip");

    let first = BlockPos::new(1, 64, 1);
    let second = BlockPos::new(20, 70, -3);

    let mut world = BlockWorld::new();
    world.place_block(first, 1);
    world.place_block(second, 5);
    assert_eq!(world.block_id(first), 1);
    assert_eq!(world.block_id(second), 5);

    world
        .save_all_touched_chunks(&root)
        .expect("chunks should save");

    let mut loaded = BlockWorld::new();
    loaded
        .load_chunk(&root, ChunkPos::from_block(first))
        .expect("first chunk should load");
    loaded
        .load_chunk(&root, ChunkPos::from_block(second))
        .expect("second chunk should load");

    assert_eq!(loaded.block_id(first), 1);
    assert_eq!(loaded.block_id(second), 5);

    cleanup_dir(&root);
}

#[test]
fn break_and_resave_persists_air_state() {
    let root = unique_temp_directory("block_break_persist");

    let target = BlockPos::new(-2, 67, 33);
    let chunk = ChunkPos::from_block(target);

    let mut world = BlockWorld::new();
    world.place_block(target, 24);
    world
        .save_chunk(&root, chunk)
        .expect("initial chunk save should succeed");
    assert_eq!(world.block_id(target), 24);

    world.break_block(target);
    world
        .save_chunk(&root, chunk)
        .expect("chunk rewrite should succeed");

    let mut loaded = BlockWorld::new();
    loaded
        .load_chunk(&root, chunk)
        .expect("chunk should load after rewrite");
    assert_eq!(loaded.block_id(target), 0);

    cleanup_dir(&root);
}

#[test]
fn replace_chunk_blocks_swaps_chunk_contents_atomically() {
    let mut world = BlockWorld::new();
    let chunk = ChunkPos::new(0, 0);

    let old_block = BlockPos::new(0, 64, 0);
    let replacement_block = BlockPos::new(2, 64, 2);
    let wrong_chunk_block = BlockPos::new(20, 64, 20);

    world.place_block(old_block, 1);
    assert_eq!(world.block_id(old_block), 1);

    world.replace_chunk_blocks(
        chunk,
        vec![
            (replacement_block, 5),
            (wrong_chunk_block, 9),
            (BlockPos::new(1, 64, 1), 0),
        ],
    );

    assert_eq!(world.block_id(old_block), 0);
    assert_eq!(world.block_id(replacement_block), 5);
    assert_eq!(world.block_id(wrong_chunk_block), 0);
}

#[test]
fn fluid_block_data_round_trips_through_chunk_storage() {
    let root = unique_temp_directory("fluid_data_round_trip");
    let block = BlockPos::new(6, 62, -5);
    let chunk = ChunkPos::from_block(block);

    let mut world = BlockWorld::new();
    world.place_block(block, WATER_SOURCE_BLOCK_ID);
    world.set_block_data(block, 8);
    world
        .save_chunk(&root, chunk)
        .expect("fluid chunk should save");

    let mut loaded = BlockWorld::new();
    loaded
        .load_chunk(&root, chunk)
        .expect("fluid chunk should load");

    assert_eq!(loaded.block_id(block), WATER_SOURCE_BLOCK_ID);
    assert_eq!(loaded.block_data(block), 8);

    cleanup_dir(&root);
}

fn unique_temp_directory(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();

    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "lce_rust_{test_name}_{}_{}",
        std::process::id(),
        nanos
    ));
    dir
}

fn cleanup_dir(path: &PathBuf) {
    let _ = std::fs::remove_dir_all(path);
}

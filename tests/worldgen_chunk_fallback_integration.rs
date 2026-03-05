use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use lce_rust::world::worldgen::RandomLevelSource;
use lce_rust::world::{BlockPos, BlockWorld, ChunkLoadOutcome, ChunkPos};

#[test]
fn missing_chunk_payload_triggers_generation_and_persistence() {
    let root = unique_temp_directory("worldgen_chunk_fallback");
    let source = RandomLevelSource::new(12_345);
    let chunk = ChunkPos::new(2, -1);

    let mut world = BlockWorld::new();
    let outcome = world
        .load_chunk_or_generate(&root, chunk, |target_chunk| {
            source.generate_chunk(target_chunk).blocks
        })
        .expect("load/generate should succeed");

    assert_eq!(outcome, ChunkLoadOutcome::GeneratedFallback);
    assert!(!world.blocks_in_chunk(chunk).is_empty());

    let expected_generated_len = source.generate_chunk(chunk).blocks.len();
    assert_eq!(world.blocks_in_chunk(chunk).len(), expected_generated_len);

    let mut second_world = BlockWorld::new();
    let mut fallback_calls = 0usize;
    let second_outcome = second_world
        .load_chunk_or_generate(&root, chunk, |_| {
            fallback_calls = fallback_calls.saturating_add(1);
            Vec::new()
        })
        .expect("loading persisted generated chunk should succeed");

    assert_eq!(second_outcome, ChunkLoadOutcome::LoadedFromStorage);
    assert_eq!(fallback_calls, 0);
    assert_eq!(
        second_world.blocks_in_chunk(chunk).len(),
        world.blocks_in_chunk(chunk).len()
    );

    cleanup_dir(&root);
}

#[test]
fn fallback_generation_ignores_blocks_outside_target_chunk() {
    let root = unique_temp_directory("worldgen_chunk_guard");
    let chunk = ChunkPos::new(0, 0);
    let mut world = BlockWorld::new();

    let outcome = world
        .load_chunk_or_generate(&root, chunk, |_| {
            vec![
                (BlockPos::new(1, 70, 1), 1),
                (BlockPos::new(16, 70, 1), 2),
                (BlockPos::new(-1, 70, 1), 3),
            ]
        })
        .expect("fallback generation should succeed");

    assert_eq!(outcome, ChunkLoadOutcome::GeneratedFallback);
    assert_eq!(world.block_id(BlockPos::new(1, 70, 1)), 1);
    assert_eq!(world.block_id(BlockPos::new(16, 70, 1)), 0);
    assert_eq!(world.block_id(BlockPos::new(-1, 70, 1)), 0);

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

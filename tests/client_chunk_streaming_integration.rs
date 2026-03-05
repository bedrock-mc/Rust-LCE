use std::collections::BTreeSet;

use lce_rust::client::chunk_streaming::{
    chunk_diff, desired_chunk_window, lifecycle_note_chunk_loaded, lifecycle_note_chunk_unloaded,
    parse_boolean_flag, parse_perf_logging_flag, perf_logging_enabled_with_default,
    player_chunk_from_position,
};
use lce_rust::world::{BlockPos, ChunkLifecycleController, ChunkPos, Vec3};

#[test]
fn maps_player_position_to_chunk_coordinates() {
    let chunk = player_chunk_from_position(Vec3::new(-0.1, 65.0, 31.9));

    assert_eq!(chunk, ChunkPos::new(-1, 1));
}

#[test]
fn builds_square_chunk_window_around_center() {
    let window = desired_chunk_window(ChunkPos::new(10, -4), 1);

    assert_eq!(window.len(), 9);
    assert!(window.contains(&ChunkPos::new(9, -5)));
    assert!(window.contains(&ChunkPos::new(10, -4)));
    assert!(window.contains(&ChunkPos::new(11, -3)));
}

#[test]
fn computes_chunk_load_and_unload_sets() {
    let current: BTreeSet<_> = [
        ChunkPos::new(0, 0),
        ChunkPos::new(1, 0),
        ChunkPos::new(2, 0),
    ]
    .into_iter()
    .collect();
    let desired: BTreeSet<_> = [
        ChunkPos::new(1, 0),
        ChunkPos::new(2, 0),
        ChunkPos::new(3, 0),
    ]
    .into_iter()
    .collect();

    let (to_load, to_unload) = chunk_diff(&current, &desired);

    assert_eq!(to_load, vec![ChunkPos::new(3, 0)]);
    assert_eq!(to_unload, vec![ChunkPos::new(0, 0)]);
}

#[test]
fn chunk_mapping_matches_block_grid_math() {
    let block = BlockPos::new(47, 70, -17);
    assert_eq!(ChunkPos::from_block(block), ChunkPos::new(2, -2));
}

#[test]
fn lifecycle_hooks_follow_chunk_window_transition() {
    let current: BTreeSet<_> = [ChunkPos::new(0, 0), ChunkPos::new(1, 0)]
        .into_iter()
        .collect();
    let desired: BTreeSet<_> = [ChunkPos::new(1, 0), ChunkPos::new(2, 0)]
        .into_iter()
        .collect();

    let mut lifecycle = ChunkLifecycleController::new();
    for chunk in &current {
        lifecycle_note_chunk_loaded(&mut lifecycle, *chunk);
    }

    let (to_load, to_unload) = chunk_diff(&current, &desired);
    for chunk in to_load {
        lifecycle_note_chunk_loaded(&mut lifecycle, chunk);
    }
    for chunk in to_unload {
        lifecycle_note_chunk_unloaded(&mut lifecycle, chunk);
    }

    assert_eq!(lifecycle.loaded_chunks(), &desired);
    assert_eq!(lifecycle.active_chunks(), &desired);
}

#[test]
fn shared_boolean_flag_parser_handles_truthy_and_falsy_values() {
    assert!(!parse_boolean_flag(None));
    assert!(parse_boolean_flag(Some("1")));
    assert!(parse_boolean_flag(Some("true")));
    assert!(parse_boolean_flag(Some("YES")));
    assert!(parse_boolean_flag(Some(" on ")));
    assert!(!parse_boolean_flag(Some("0")));
    assert!(!parse_boolean_flag(Some("false")));
    assert!(!parse_boolean_flag(Some("no")));
    assert!(!parse_boolean_flag(Some("")));
    assert!(!parse_boolean_flag(Some("maybe")));
}

#[test]
fn perf_logging_flag_matches_async_flag_parsing_rules() {
    assert!(!parse_perf_logging_flag(None));
    assert!(parse_perf_logging_flag(Some("true")));
    assert!(parse_perf_logging_flag(Some("On")));
    assert!(!parse_perf_logging_flag(Some("off")));
    assert!(!parse_perf_logging_flag(Some("0")));
}

#[test]
fn perf_logging_defaults_to_disabled_when_env_missing() {
    assert!(!perf_logging_enabled_with_default(None));
    assert!(perf_logging_enabled_with_default(Some("true")));
    assert!(!perf_logging_enabled_with_default(Some("false")));
}

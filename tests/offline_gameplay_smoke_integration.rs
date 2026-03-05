use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use lce_rust::save::world_io::{load_world_snapshot, save_world_snapshot};
use lce_rust::world::{
    BlockPos, BlockWorld, MovementInput, OfflineGameSession, OfflineWorldBootstrap,
};

#[test]
fn offline_gameplay_smoke_round_trip() {
    let world_root = unique_temp_directory("offline_smoke");

    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("SmokeWorld", 123_456)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);
    for tick in 0..40 {
        game.tick(MovementInput {
            strafe: 0.0,
            forward: 1.0,
            jump: tick == 5,
            sneak: false,
        });
    }

    let place_pos = BlockPos::new(2, 64, 2);
    let broken_pos = BlockPos::new(18, 65, -3);
    let mut blocks = BlockWorld::new();
    blocks.place_block(place_pos, 4);
    blocks.place_block(broken_pos, 7);
    blocks.break_block(broken_pos);

    let snapshot = game.world_snapshot();
    save_world_snapshot(&world_root, &snapshot).expect("world snapshot should save");
    blocks
        .save_chunk(
            &world_root,
            lce_rust::world::ChunkPos::from_block(place_pos),
        )
        .expect("placed chunk should save");
    blocks
        .save_chunk(
            &world_root,
            lce_rust::world::ChunkPos::from_block(broken_pos),
        )
        .expect("broken chunk should save");

    let reloaded_snapshot = load_world_snapshot(&world_root).expect("snapshot should reload");
    assert_eq!(reloaded_snapshot.name, "SmokeWorld");
    assert_eq!(reloaded_snapshot.seed, 123_456);
    assert_eq!(reloaded_snapshot.tick_count, 40);

    let mut reloaded_world = OfflineWorldBootstrap::new();
    reloaded_world.load_world(reloaded_snapshot);

    let mut reloaded_blocks = BlockWorld::new();
    reloaded_blocks
        .load_chunk(
            &world_root,
            lce_rust::world::ChunkPos::from_block(place_pos),
        )
        .expect("placed chunk should load");
    reloaded_blocks
        .load_chunk(
            &world_root,
            lce_rust::world::ChunkPos::from_block(broken_pos),
        )
        .expect("broken chunk should load");

    assert_eq!(reloaded_blocks.block_id(place_pos), 4);
    assert_eq!(reloaded_blocks.block_id(broken_pos), 0);

    cleanup_dir(&world_root);
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

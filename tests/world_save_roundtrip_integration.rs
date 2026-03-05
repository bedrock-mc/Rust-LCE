use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use lce_rust::save::nbt::{NbtCompound, NbtRoot, NbtTag, write_root_to_bytes};
use lce_rust::save::world_io::{
    WorldIoError, load_chunk_payload, load_world_snapshot, save_chunk_payload, save_world_snapshot,
};
use lce_rust::world::OfflineWorldBootstrap;

#[test]
fn saves_and_loads_world_snapshot_and_chunks() {
    let root = unique_temp_directory("world_roundtrip");

    let mut bootstrap = OfflineWorldBootstrap::new();
    bootstrap
        .create_world("Survival", 42)
        .expect("world should be created");
    bootstrap
        .tick_active_world(77)
        .expect("world should advance");

    let snapshot = bootstrap
        .save_active_world()
        .expect("snapshot should be produced");
    save_world_snapshot(&root, &snapshot).expect("world metadata should save");

    save_chunk_payload(&root, 0, 0, b"chunk_zero").expect("chunk should save");
    save_chunk_payload(&root, 33, -1, b"chunk_cross_region")
        .expect("cross-region chunk should save");

    let loaded_snapshot = load_world_snapshot(&root).expect("world metadata should load");
    assert_eq!(loaded_snapshot, snapshot);

    let loaded_chunk_a = load_chunk_payload(&root, 0, 0)
        .expect("chunk load should succeed")
        .expect("chunk should exist");
    let loaded_chunk_b = load_chunk_payload(&root, 33, -1)
        .expect("chunk load should succeed")
        .expect("chunk should exist");

    assert_eq!(loaded_chunk_a, b"chunk_zero");
    assert_eq!(loaded_chunk_b, b"chunk_cross_region");

    let mut reloaded = OfflineWorldBootstrap::new();
    let world = reloaded.load_world(loaded_snapshot);
    assert_eq!(world.name, "Survival");
    assert_eq!(world.seed, 42);
    assert_eq!(world.tick_count, 77);

    cleanup_dir(&root);
}

#[test]
fn loading_world_without_required_fields_returns_error() {
    let root = unique_temp_directory("world_missing_field");
    std::fs::create_dir_all(&root).expect("temp world dir should be created");

    let mut compound = NbtCompound::new();
    compound.insert("LevelName", NbtTag::String("BrokenWorld".to_string()));
    compound.insert("RandomSeed", NbtTag::Long(99));

    let bytes = write_root_to_bytes(&NbtRoot::new("Data", compound))
        .expect("manual metadata payload should encode");
    std::fs::write(root.join("level.dat"), bytes).expect("metadata file should be written");

    let error = load_world_snapshot(&root).expect_err("missing tick count should fail");
    assert!(matches!(error, WorldIoError::MissingField("TickCount")));

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

use std::collections::BTreeSet;

use crate::world::{BlockPos, ChunkLifecycleController, ChunkPos, Vec3};

pub fn parse_boolean_flag(value: Option<&str>) -> bool {
    value.is_some_and(|raw| {
        let normalized = raw.trim().to_ascii_lowercase();
        matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
    })
}

pub fn parse_perf_logging_flag(value: Option<&str>) -> bool {
    parse_boolean_flag(value)
}

pub fn perf_logging_enabled_with_default(value: Option<&str>) -> bool {
    value.map_or(false, |raw| parse_perf_logging_flag(Some(raw)))
}

pub fn performance_logging_enabled() -> bool {
    perf_logging_enabled_with_default(std::env::var("LCE_PERF_LOG").ok().as_deref())
}

pub fn player_chunk_from_position(position: Vec3) -> ChunkPos {
    ChunkPos::from_block(BlockPos::new(
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    ))
}

pub fn desired_chunk_window(center: ChunkPos, radius: i32) -> BTreeSet<ChunkPos> {
    let mut chunks = BTreeSet::new();
    let bounded_radius = radius.max(0);

    for x in (center.x - bounded_radius)..=(center.x + bounded_radius) {
        for z in (center.z - bounded_radius)..=(center.z + bounded_radius) {
            chunks.insert(ChunkPos::new(x, z));
        }
    }

    chunks
}

pub fn chunk_diff(
    current: &BTreeSet<ChunkPos>,
    desired: &BTreeSet<ChunkPos>,
) -> (Vec<ChunkPos>, Vec<ChunkPos>) {
    let to_load = desired
        .iter()
        .copied()
        .filter(|chunk| !current.contains(chunk))
        .collect();

    let to_unload = current
        .iter()
        .copied()
        .filter(|chunk| !desired.contains(chunk))
        .collect();

    (to_load, to_unload)
}

pub fn lifecycle_note_chunk_loaded(lifecycle: &mut ChunkLifecycleController, chunk: ChunkPos) {
    let _ = lifecycle.load_chunk(chunk);
    let _ = lifecycle.set_chunk_active(chunk, true);
}

pub fn lifecycle_note_chunk_unloaded(lifecycle: &mut ChunkLifecycleController, chunk: ChunkPos) {
    let _ = lifecycle.set_chunk_active(chunk, false);
    let _ = lifecycle.unload_chunk(chunk);
}

use lce_rust::world::ChunkPos;
use lce_rust::world::worldgen::{BiomeId, BiomeSource};

const CACHE_DECAY_DRIVE_STEPS: i32 = 900;

#[test]
fn aligned_chunk_requests_populate_and_hit_biome_cache() {
    let source = BiomeSource::new(90_001);
    let before = source.cache_stats();
    assert_eq!(before.cached_blocks, 0);

    let first = source.chunk_biomes(ChunkPos::new(0, 0));
    let after_first = source.cache_stats();
    assert_eq!(after_first.cached_blocks, 1);
    assert_eq!(after_first.misses, before.misses + 1);
    assert_eq!(after_first.hits, before.hits);

    let second = source.chunk_biomes(ChunkPos::new(0, 0));
    let after_second = source.cache_stats();
    assert_eq!(first, second);
    assert_eq!(after_second.misses, after_first.misses);
    assert_eq!(after_second.hits, after_first.hits + 1);
}

#[test]
fn non_aligned_biome_queries_bypass_chunk_cache() {
    let source = BiomeSource::new(-123_456);
    let before = source.cache_stats();

    let indices = source.biome_index_block(3, 5, 8, 8, true);
    let after = source.cache_stats();

    assert_eq!(indices.len(), 64);
    assert_eq!(before, after);
}

#[test]
fn stale_cache_blocks_are_evicted_after_decay_window() {
    let source = BiomeSource::new(44_321);

    let _ = source.chunk_biomes(ChunkPos::new(0, 0));
    let seeded = source.cache_stats();
    assert_eq!(seeded.cached_blocks, 1);

    for zone in 1..=CACHE_DECAY_DRIVE_STEPS {
        let world_x = zone * 16;
        let _ = source.biome_at(world_x, 0);
    }

    let before_revisit = source.cache_stats();
    let _ = source.chunk_biomes(ChunkPos::new(0, 0));
    let after_revisit = source.cache_stats();

    assert_eq!(after_revisit.misses, before_revisit.misses + 1);
}

#[test]
fn chunk_biome_map_matches_cached_biome_indices() {
    let source = BiomeSource::new(77_311);
    let chunk = ChunkPos::new(-2, 3);
    let biomes = source.chunk_biomes(chunk);
    let indices = source.biome_index_block(chunk.x * 16, chunk.z * 16, 16, 16, true);

    for local_z in 0..16usize {
        for local_x in 0..16usize {
            let index = local_z * 16 + local_x;
            let biome_from_map = biomes.at(local_x, local_z);
            let biome_from_index =
                BiomeId::from_index(indices[index]).expect("cached biome index should be valid");
            assert_eq!(biome_from_map, biome_from_index);
        }
    }
}

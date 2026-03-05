use std::collections::HashMap;

use super::{CHUNK_AREA, CHUNK_SIDE_I32, CHUNK_SIDE_USIZE};

const DECAY_ACCESS_WINDOW: u64 = 600;
const UPDATE_INTERVAL: u64 = DECAY_ACCESS_WINDOW / 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeCacheStats {
    pub cached_blocks: usize,
    pub hits: u64,
    pub misses: u64,
}

#[derive(Debug, Clone)]
pub struct BiomeCache {
    cached: HashMap<u64, BiomeCacheBlock>,
    access_counter: u64,
    last_update: u64,
    hits: u64,
    misses: u64,
}

impl Default for BiomeCache {
    fn default() -> Self {
        Self {
            cached: HashMap::new(),
            access_counter: 0,
            last_update: 0,
            hits: 0,
            misses: 0,
        }
    }
}

impl BiomeCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stats(&self) -> BiomeCacheStats {
        BiomeCacheStats {
            cached_blocks: self.cached.len(),
            hits: self.hits,
            misses: self.misses,
        }
    }

    pub fn get_biome_index<F>(&mut self, world_x: i32, world_z: i32, make_block: F) -> u8
    where
        F: FnOnce(i32, i32) -> [u8; CHUNK_AREA],
    {
        let indices = self.get_block_indices(world_x, world_z, make_block);
        indices[local_index(world_x, world_z)]
    }

    pub fn get_block_indices<F>(
        &mut self,
        world_x: i32,
        world_z: i32,
        make_block: F,
    ) -> [u8; CHUNK_AREA]
    where
        F: FnOnce(i32, i32) -> [u8; CHUNK_AREA],
    {
        let block_x = world_x.div_euclid(CHUNK_SIDE_I32);
        let block_z = world_z.div_euclid(CHUNK_SIDE_I32);
        let slot = make_slot(block_x, block_z);
        let access_tick = self.next_access_tick();

        if let Some(block) = self.cached.get_mut(&slot) {
            self.hits = self.hits.saturating_add(1);
            block.last_use = access_tick;
            let biome_indices = block.biome_indices;
            self.update();
            return biome_indices;
        }

        self.misses = self.misses.saturating_add(1);
        let biome_indices = make_block(block_x, block_z);

        self.cached.insert(
            slot,
            BiomeCacheBlock {
                last_use: access_tick,
                biome_indices,
            },
        );

        self.update();
        biome_indices
    }

    fn update(&mut self) {
        let elapsed = self.access_counter.wrapping_sub(self.last_update);
        if elapsed < UPDATE_INTERVAL {
            return;
        }

        self.last_update = self.access_counter;
        let now = self.access_counter;

        self.cached.retain(|_, block| {
            let inactive = now.wrapping_sub(block.last_use);
            inactive <= DECAY_ACCESS_WINDOW
        });
    }

    fn next_access_tick(&mut self) -> u64 {
        self.access_counter = self.access_counter.wrapping_add(1);
        self.access_counter
    }
}

#[derive(Debug, Clone)]
struct BiomeCacheBlock {
    last_use: u64,
    biome_indices: [u8; CHUNK_AREA],
}

fn make_slot(x: i32, z: i32) -> u64 {
    u64::from(x as u32) | (u64::from(z as u32) << 32)
}

fn local_index(world_x: i32, world_z: i32) -> usize {
    let local_x = usize::try_from(world_x.rem_euclid(CHUNK_SIDE_I32)).expect("local x in range");
    let local_z = usize::try_from(world_z.rem_euclid(CHUNK_SIDE_I32)).expect("local z in range");
    local_z * CHUNK_SIDE_USIZE + local_x
}

pub mod biome;
pub mod biome_cache;
pub mod level_source;
pub mod noise;

pub use biome::{
    BiomeDecorator, BiomeId, BiomeMap, BiomeSource, DecorationFeature, DecorationPlacement,
};
pub use biome_cache::BiomeCacheStats;
pub use level_source::{
    BEDROCK_BLOCK_ID, DIRT_BLOCK_ID, END_STONE_BLOCK_ID, GRASS_BLOCK_ID, GeneratedChunk,
    HellRandomLevelSource, NETHERRACK_BLOCK_ID, RandomLevelSource, SAND_BLOCK_ID, SEA_LEVEL,
    STONE_BLOCK_ID, TheEndLevelRandomLevelSource, WATER_BLOCK_ID,
};
pub use noise::{PerlinNoise, SimplexNoise};

pub const CHUNK_SIDE_I32: i32 = 16;
pub const CHUNK_SIDE_USIZE: usize = 16;
pub const CHUNK_AREA: usize = CHUNK_SIDE_USIZE * CHUNK_SIDE_USIZE;
pub const WORLD_HEIGHT: i32 = 128;

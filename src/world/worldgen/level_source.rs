use super::biome::{
    BiomeDecorator, BiomeId, BiomeMap, BiomeSource, DecorationFeature, DecorationPlacement,
};
use super::noise::{PerlinNoise, SimplexNoise};
use super::{CHUNK_AREA, CHUNK_SIDE_I32, CHUNK_SIDE_USIZE, WORLD_HEIGHT};
use crate::world::{BlockPos, ChunkPos};

pub const SEA_LEVEL: i32 = 63;

pub const BEDROCK_BLOCK_ID: u16 = 7;
pub const WATER_BLOCK_ID: u16 = 9;
pub const STONE_BLOCK_ID: u16 = 1;
pub const GRASS_BLOCK_ID: u16 = 2;
pub const DIRT_BLOCK_ID: u16 = 3;
pub const SAND_BLOCK_ID: u16 = 12;
pub const NETHERRACK_BLOCK_ID: u16 = 87;
pub const END_STONE_BLOCK_ID: u16 = 121;

#[derive(Debug, Clone, Copy)]
struct EndSpikeLayout {
    spike_chunk_x: i32,
    spike_chunk_z: i32,
    center_x: i32,
    center_z: i32,
}

const END_SPIKE_LAYOUT: [EndSpikeLayout; 8] = [
    EndSpikeLayout {
        spike_chunk_x: 2,
        spike_chunk_z: -1,
        center_x: 40,
        center_z: 0,
    },
    EndSpikeLayout {
        spike_chunk_x: 1,
        spike_chunk_z: 1,
        center_x: 28,
        center_z: 28,
    },
    EndSpikeLayout {
        spike_chunk_x: -1,
        spike_chunk_z: 2,
        center_x: 0,
        center_z: 40,
    },
    EndSpikeLayout {
        spike_chunk_x: -2,
        spike_chunk_z: 1,
        center_x: -28,
        center_z: 28,
    },
    EndSpikeLayout {
        spike_chunk_x: -3,
        spike_chunk_z: -1,
        center_x: -40,
        center_z: 0,
    },
    EndSpikeLayout {
        spike_chunk_x: -2,
        spike_chunk_z: -2,
        center_x: -28,
        center_z: -28,
    },
    EndSpikeLayout {
        spike_chunk_x: -1,
        spike_chunk_z: -3,
        center_x: 0,
        center_z: -40,
    },
    EndSpikeLayout {
        spike_chunk_x: 1,
        spike_chunk_z: -2,
        center_x: 28,
        center_z: -28,
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedChunk {
    pub chunk: ChunkPos,
    pub biome_map: BiomeMap,
    pub surface_heights: [i32; CHUNK_AREA],
    pub blocks: Vec<(BlockPos, u16)>,
    pub decorations: Vec<DecorationPlacement>,
}

#[derive(Debug, Clone)]
pub struct RandomLevelSource {
    seed: i64,
    base_terrain_noise: PerlinNoise,
    detail_terrain_noise: SimplexNoise,
    biome_source: BiomeSource,
    biome_decorator: BiomeDecorator,
}

impl RandomLevelSource {
    pub fn new(seed: i64) -> Self {
        Self {
            seed,
            base_terrain_noise: PerlinNoise::new(seed.wrapping_mul(13).wrapping_add(0x632BE5AB)),
            detail_terrain_noise: SimplexNoise::new(seed.wrapping_mul(29).wrapping_add(0x85157AF5)),
            biome_source: BiomeSource::new(seed),
            biome_decorator: BiomeDecorator::new(seed),
        }
    }

    pub fn seed(&self) -> i64 {
        self.seed
    }

    pub fn biome_source(&self) -> &BiomeSource {
        &self.biome_source
    }

    pub fn surface_height_at(&self, world_x: i32, world_z: i32) -> i32 {
        let biome = self.biome_source.biome_at(world_x, world_z);
        self.surface_height_for(world_x, world_z, biome)
    }

    pub fn generate_chunk(&self, chunk: ChunkPos) -> GeneratedChunk {
        let biome_map = self.biome_source.chunk_biomes(chunk);
        let mut surface_heights = [0; CHUNK_AREA];
        let mut blocks = Vec::new();

        for local_z in 0..CHUNK_SIDE_USIZE {
            for local_x in 0..CHUNK_SIDE_USIZE {
                let world_x = chunk.x * CHUNK_SIDE_I32 + local_x as i32;
                let world_z = chunk.z * CHUNK_SIDE_I32 + local_z as i32;
                let biome = biome_map.at(local_x, local_z);
                let surface_height = self.surface_height_for(world_x, world_z, biome);
                surface_heights[chunk_index(local_x, local_z)] = surface_height;

                append_overworld_column(&mut blocks, world_x, world_z, surface_height, biome);
            }
        }

        let decorations = self.biome_decorator.decorate_chunk(
            chunk,
            &biome_map,
            |x, z| (self.surface_height_at(x, z) + 1).clamp(1, WORLD_HEIGHT - 1),
            |x, z| {
                let surface_height = self.surface_height_at(x, z);
                let heightmap = if surface_height < SEA_LEVEL {
                    SEA_LEVEL + 1
                } else {
                    surface_height + 1
                };
                heightmap.clamp(0, WORLD_HEIGHT - 1)
            },
        );

        GeneratedChunk {
            chunk,
            biome_map,
            surface_heights,
            blocks,
            decorations,
        }
    }

    fn surface_height_for(&self, world_x: i32, world_z: i32, biome: BiomeId) -> i32 {
        let x = f64::from(world_x);
        let z = f64::from(world_z);

        let base_height = match biome {
            BiomeId::Ocean => 56.0,
            BiomeId::Plains => 67.0,
            BiomeId::Desert => 69.0,
            BiomeId::Forest => 71.0,
            BiomeId::Taiga => 74.0,
            BiomeId::Hell => 40.0,
            BiomeId::TheEnd => 58.0,
        };

        let shape = self.base_terrain_noise.sample2d(x * 0.015, z * 0.015) * 9.0;
        let detail = self.detail_terrain_noise.sample2d(x * 0.04, z * 0.04) * 3.0;
        (base_height + shape + detail)
            .round()
            .clamp(32.0, f64::from(WORLD_HEIGHT - 1)) as i32
    }
}

#[derive(Debug, Clone)]
pub struct HellRandomLevelSource {
    seed: i64,
    height_noise: PerlinNoise,
    biome_decorator: BiomeDecorator,
}

impl HellRandomLevelSource {
    pub fn new(seed: i64) -> Self {
        Self {
            seed,
            height_noise: PerlinNoise::new(seed.wrapping_mul(37).wrapping_add(0xA1B2C3D4)),
            biome_decorator: BiomeDecorator::new(seed),
        }
    }

    pub fn seed(&self) -> i64 {
        self.seed
    }

    pub fn surface_height_at(&self, world_x: i32, world_z: i32) -> i32 {
        (42.0
            + self
                .height_noise
                .sample2d(f64::from(world_x) * 0.02, f64::from(world_z) * 0.02)
                * 10.0)
            .round()
            .clamp(20.0, f64::from(WORLD_HEIGHT - 1)) as i32
    }

    pub fn generate_chunk(&self, chunk: ChunkPos) -> GeneratedChunk {
        let biome_map = BiomeMap::filled(BiomeId::Hell);
        let mut surface_heights = [0; CHUNK_AREA];
        let mut blocks = Vec::new();

        for local_z in 0..CHUNK_SIDE_USIZE {
            for local_x in 0..CHUNK_SIDE_USIZE {
                let world_x = chunk.x * CHUNK_SIDE_I32 + local_x as i32;
                let world_z = chunk.z * CHUNK_SIDE_I32 + local_z as i32;
                let height = self.surface_height_at(world_x, world_z);

                surface_heights[chunk_index(local_x, local_z)] = height;
                append_uniform_column(&mut blocks, world_x, world_z, height, NETHERRACK_BLOCK_ID);
            }
        }

        let decorations = self.biome_decorator.decorate_chunk(
            chunk,
            &biome_map,
            |x, z| (self.surface_height_at(x, z) + 1).clamp(1, WORLD_HEIGHT - 1),
            |x, z| (self.surface_height_at(x, z) + 1).clamp(1, WORLD_HEIGHT - 1),
        );

        GeneratedChunk {
            chunk,
            biome_map,
            surface_heights,
            blocks,
            decorations,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TheEndLevelRandomLevelSource {
    seed: i64,
    island_noise: SimplexNoise,
    biome_decorator: BiomeDecorator,
}

impl TheEndLevelRandomLevelSource {
    pub fn new(seed: i64) -> Self {
        Self {
            seed,
            island_noise: SimplexNoise::new(seed.wrapping_mul(41).wrapping_add(0x10203040)),
            biome_decorator: BiomeDecorator::new(seed),
        }
    }

    pub fn seed(&self) -> i64 {
        self.seed
    }

    pub fn surface_height_at(&self, world_x: i32, world_z: i32) -> i32 {
        let radial_distance = ((world_x * world_x + world_z * world_z) as f64).sqrt();
        let radial_falloff = (1.0 - radial_distance / 1_200.0).max(0.0);
        let island_shape =
            self.island_noise
                .sample3d(f64::from(world_x) * 0.01, 0.0, f64::from(world_z) * 0.01);

        (50.0 + radial_falloff * 20.0 + island_shape * 8.0)
            .round()
            .clamp(8.0, f64::from(WORLD_HEIGHT - 1)) as i32
    }

    fn spike_base_height(&self) -> i32 {
        END_SPIKE_LAYOUT
            .iter()
            .map(|spike| {
                let world_x = spike.spike_chunk_x * CHUNK_SIDE_I32 + 8;
                let world_z = spike.spike_chunk_z * CHUNK_SIDE_I32 + 8;
                (self.surface_height_at(world_x, world_z) + 1).clamp(1, WORLD_HEIGHT - 1)
            })
            .max()
            .unwrap_or(WORLD_HEIGHT / 2)
    }

    fn append_special_decorations(
        &self,
        chunk: ChunkPos,
        decorations: &mut Vec<DecorationPlacement>,
    ) {
        let spike_base_y = self.spike_base_height();
        for spike in END_SPIKE_LAYOUT {
            if chunk.x == spike.spike_chunk_x && chunk.z == spike.spike_chunk_z {
                decorations.push(DecorationPlacement {
                    feature: DecorationFeature::EndSpike,
                    block: BlockPos::new(spike.center_x, spike_base_y, spike.center_z),
                });
            }
        }

        if chunk.x == 0 && chunk.z == 0 {
            decorations.push(DecorationPlacement {
                feature: DecorationFeature::EndDragonSpawn,
                block: BlockPos::new(0, WORLD_HEIGHT, 0),
            });
        }

        if chunk.x == -1 && chunk.z == -1 {
            decorations.push(DecorationPlacement {
                feature: DecorationFeature::EndPodium,
                block: BlockPos::new(0, WORLD_HEIGHT / 2, 0),
            });
        }
    }

    pub fn generate_chunk(&self, chunk: ChunkPos) -> GeneratedChunk {
        let biome_map = BiomeMap::filled(BiomeId::TheEnd);
        let mut surface_heights = [0; CHUNK_AREA];
        let mut blocks = Vec::new();

        for local_z in 0..CHUNK_SIDE_USIZE {
            for local_x in 0..CHUNK_SIDE_USIZE {
                let world_x = chunk.x * CHUNK_SIDE_I32 + local_x as i32;
                let world_z = chunk.z * CHUNK_SIDE_I32 + local_z as i32;
                let height = self.surface_height_at(world_x, world_z);

                surface_heights[chunk_index(local_x, local_z)] = height;
                append_uniform_column(&mut blocks, world_x, world_z, height, END_STONE_BLOCK_ID);
            }
        }

        let mut decorations = self
            .biome_decorator
            .decorate_chunk(
                chunk,
                &biome_map,
                |x, z| (self.surface_height_at(x, z) + 1).clamp(1, WORLD_HEIGHT - 1),
                |x, z| (self.surface_height_at(x, z) + 1).clamp(1, WORLD_HEIGHT - 1),
            )
            .into_iter()
            .filter(|placement| is_ore_feature(placement.feature))
            .collect::<Vec<_>>();
        self.append_special_decorations(chunk, &mut decorations);

        GeneratedChunk {
            chunk,
            biome_map,
            surface_heights,
            blocks,
            decorations,
        }
    }
}

fn is_ore_feature(feature: DecorationFeature) -> bool {
    matches!(
        feature,
        DecorationFeature::DirtOre
            | DecorationFeature::GravelOre
            | DecorationFeature::CoalOre
            | DecorationFeature::IronOre
            | DecorationFeature::GoldOre
            | DecorationFeature::RedstoneOre
            | DecorationFeature::DiamondOre
            | DecorationFeature::LapisOre
    )
}

fn append_overworld_column(
    blocks: &mut Vec<(BlockPos, u16)>,
    world_x: i32,
    world_z: i32,
    surface_height: i32,
    biome: BiomeId,
) {
    let height = surface_height.clamp(1, WORLD_HEIGHT - 1);
    let top_block = top_block_for_biome(biome);
    let filler_block = filler_block_for_biome(biome);

    for y in 0..=height {
        let block_id = if y == 0 {
            BEDROCK_BLOCK_ID
        } else if y == height {
            top_block
        } else if y >= height - 3 {
            filler_block
        } else {
            STONE_BLOCK_ID
        };

        blocks.push((BlockPos::new(world_x, y, world_z), block_id));
    }

    if height < SEA_LEVEL {
        for y in (height + 1)..=SEA_LEVEL.min(WORLD_HEIGHT - 1) {
            blocks.push((BlockPos::new(world_x, y, world_z), WATER_BLOCK_ID));
        }
    }
}

fn append_uniform_column(
    blocks: &mut Vec<(BlockPos, u16)>,
    world_x: i32,
    world_z: i32,
    surface_height: i32,
    fill_block: u16,
) {
    let height = surface_height.clamp(1, WORLD_HEIGHT - 1);

    for y in 0..=height {
        let block_id = if y == 0 { BEDROCK_BLOCK_ID } else { fill_block };
        blocks.push((BlockPos::new(world_x, y, world_z), block_id));
    }
}

fn top_block_for_biome(biome: BiomeId) -> u16 {
    match biome {
        BiomeId::Ocean | BiomeId::Desert => SAND_BLOCK_ID,
        BiomeId::Plains | BiomeId::Forest | BiomeId::Taiga => GRASS_BLOCK_ID,
        BiomeId::Hell => NETHERRACK_BLOCK_ID,
        BiomeId::TheEnd => END_STONE_BLOCK_ID,
    }
}

fn filler_block_for_biome(biome: BiomeId) -> u16 {
    match biome {
        BiomeId::Ocean | BiomeId::Desert => SAND_BLOCK_ID,
        BiomeId::Plains | BiomeId::Forest | BiomeId::Taiga => DIRT_BLOCK_ID,
        BiomeId::Hell => NETHERRACK_BLOCK_ID,
        BiomeId::TheEnd => END_STONE_BLOCK_ID,
    }
}

fn chunk_index(local_x: usize, local_z: usize) -> usize {
    assert!(local_x < CHUNK_SIDE_USIZE, "local_x out of chunk range");
    assert!(local_z < CHUNK_SIDE_USIZE, "local_z out of chunk range");
    local_z * CHUNK_SIDE_USIZE + local_x
}

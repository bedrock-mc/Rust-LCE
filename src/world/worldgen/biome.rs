use std::cell::RefCell;

use super::biome_cache::{BiomeCache, BiomeCacheStats};
use super::noise::{PerlinNoise, SimplexNoise};
use super::{CHUNK_AREA, CHUNK_SIDE_I32, CHUNK_SIDE_USIZE, WORLD_HEIGHT};
use crate::world::{BlockPos, ChunkPos};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeId {
    Ocean,
    Plains,
    Desert,
    Forest,
    Taiga,
    Hell,
    TheEnd,
}

impl BiomeId {
    pub const fn as_index(self) -> u8 {
        match self {
            Self::Ocean => 0,
            Self::Plains => 1,
            Self::Desert => 2,
            Self::Forest => 3,
            Self::Taiga => 4,
            Self::Hell => 5,
            Self::TheEnd => 6,
        }
    }

    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Ocean),
            1 => Some(Self::Plains),
            2 => Some(Self::Desert),
            3 => Some(Self::Forest),
            4 => Some(Self::Taiga),
            5 => Some(Self::Hell),
            6 => Some(Self::TheEnd),
            _ => None,
        }
    }

    pub const fn temperature(self) -> f32 {
        match self {
            Self::Ocean => 0.48,
            Self::Plains => 0.78,
            Self::Desert => 1.0,
            Self::Forest => 0.72,
            Self::Taiga => 0.23,
            Self::Hell => 1.0,
            Self::TheEnd => 0.5,
        }
    }

    pub const fn downfall(self) -> f32 {
        match self {
            Self::Ocean => 0.9,
            Self::Plains => 0.4,
            Self::Desert => 0.0,
            Self::Forest => 0.82,
            Self::Taiga => 0.8,
            Self::Hell => 0.0,
            Self::TheEnd => 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiomeMap {
    values: [BiomeId; CHUNK_AREA],
}

impl BiomeMap {
    pub fn filled(biome: BiomeId) -> Self {
        Self {
            values: [biome; CHUNK_AREA],
        }
    }

    pub fn from_indices(indices: &[u8]) -> Option<Self> {
        if indices.len() != CHUNK_AREA {
            return None;
        }

        let mut values = [BiomeId::Plains; CHUNK_AREA];
        for (slot, index) in indices.iter().enumerate() {
            values[slot] = BiomeId::from_index(*index)?;
        }

        Some(Self { values })
    }

    pub fn at(&self, local_x: usize, local_z: usize) -> BiomeId {
        self.values[chunk_index(local_x, local_z)]
    }

    pub fn set(&mut self, local_x: usize, local_z: usize, biome: BiomeId) {
        self.values[chunk_index(local_x, local_z)] = biome;
    }

    pub fn values(&self) -> &[BiomeId; CHUNK_AREA] {
        &self.values
    }

    pub fn to_indices(&self) -> [u8; CHUNK_AREA] {
        let mut indices = [0u8; CHUNK_AREA];
        for (slot, biome) in self.values.iter().enumerate() {
            indices[slot] = biome.as_index();
        }
        indices
    }
}

#[derive(Debug, Clone)]
pub struct BiomeSource {
    seed: i64,
    temperature_noise: SimplexNoise,
    moisture_noise: SimplexNoise,
    continental_noise: PerlinNoise,
    cache: RefCell<BiomeCache>,
}

impl BiomeSource {
    pub fn new(seed: i64) -> Self {
        let temperature_seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let moisture_seed = temperature_seed.wrapping_add(0x5DEECE66D);
        let continental_seed = moisture_seed.wrapping_add(0xB5297A4D);

        Self {
            seed,
            temperature_noise: SimplexNoise::new(temperature_seed),
            moisture_noise: SimplexNoise::new(moisture_seed),
            continental_noise: PerlinNoise::new(continental_seed),
            cache: RefCell::new(BiomeCache::new()),
        }
    }

    pub fn seed(&self) -> i64 {
        self.seed
    }

    pub fn cache_stats(&self) -> BiomeCacheStats {
        self.cache.borrow().stats()
    }

    pub fn biome_at(&self, world_x: i32, world_z: i32) -> BiomeId {
        let biome_index =
            self.cache
                .borrow_mut()
                .get_biome_index(world_x, world_z, |block_x, block_z| {
                    self.build_zoomed_block_indices(block_x, block_z)
                });

        BiomeId::from_index(biome_index).unwrap_or(BiomeId::Plains)
    }

    pub fn raw_biome_at(&self, world_x: i32, world_z: i32) -> BiomeId {
        self.sample_raw_biome(world_x, world_z)
    }

    pub fn temperature_at(&self, world_x: i32, world_z: i32) -> f32 {
        self.biome_at(world_x, world_z).temperature()
    }

    pub fn downfall_at(&self, world_x: i32, world_z: i32) -> f32 {
        self.biome_at(world_x, world_z).downfall()
    }

    pub fn chunk_biomes(&self, chunk: ChunkPos) -> BiomeMap {
        let world_x = chunk.x * CHUNK_SIDE_I32;
        let world_z = chunk.z * CHUNK_SIDE_I32;
        let indices =
            self.biome_index_block(world_x, world_z, CHUNK_SIDE_USIZE, CHUNK_SIDE_USIZE, true);
        BiomeMap::from_indices(&indices).expect("chunk biome indices should always be valid")
    }

    pub fn biome_index_block(
        &self,
        world_x: i32,
        world_z: i32,
        width: usize,
        height: usize,
        use_cache: bool,
    ) -> Vec<u8> {
        if use_cache
            && width == CHUNK_SIDE_USIZE
            && height == CHUNK_SIDE_USIZE
            && world_x.rem_euclid(CHUNK_SIDE_I32) == 0
            && world_z.rem_euclid(CHUNK_SIDE_I32) == 0
        {
            return self
                .cache
                .borrow_mut()
                .get_block_indices(world_x, world_z, |block_x, block_z| {
                    self.build_zoomed_block_indices(block_x, block_z)
                })
                .to_vec();
        }

        let mut indices = vec![BiomeId::Plains.as_index(); width * height];
        for z in 0..height {
            for x in 0..width {
                let sample_x = world_x + x as i32;
                let sample_z = world_z + z as i32;
                let biome = self.sample_zoomed_biome(sample_x, sample_z);
                indices[z * width + x] = biome.as_index();
            }
        }

        indices
    }

    pub fn raw_biome_index_block(
        &self,
        world_x: i32,
        world_z: i32,
        width: usize,
        height: usize,
    ) -> Vec<u8> {
        let mut indices = vec![BiomeId::Plains.as_index(); width * height];
        for z in 0..height {
            for x in 0..width {
                let sample_x = world_x + x as i32;
                let sample_z = world_z + z as i32;
                let biome = self.sample_raw_biome(sample_x, sample_z);
                indices[z * width + x] = biome.as_index();
            }
        }

        indices
    }

    fn build_zoomed_block_indices(&self, block_x: i32, block_z: i32) -> [u8; CHUNK_AREA] {
        self.build_block_indices(block_x, block_z, Self::sample_zoomed_biome)
    }

    fn build_block_indices(
        &self,
        block_x: i32,
        block_z: i32,
        sample: fn(&BiomeSource, i32, i32) -> BiomeId,
    ) -> [u8; CHUNK_AREA] {
        let mut indices = [BiomeId::Plains.as_index(); CHUNK_AREA];

        for local_z in 0..CHUNK_SIDE_USIZE {
            for local_x in 0..CHUNK_SIDE_USIZE {
                let world_x = block_x * CHUNK_SIDE_I32 + local_x as i32;
                let world_z = block_z * CHUNK_SIDE_I32 + local_z as i32;
                let biome = sample(self, world_x, world_z);
                indices[chunk_index(local_x, local_z)] = biome.as_index();
            }
        }

        indices
    }

    fn sample_zoomed_biome(&self, world_x: i32, world_z: i32) -> BiomeId {
        self.sample_biome_internal(world_x, world_z)
    }

    fn sample_raw_biome(&self, world_x: i32, world_z: i32) -> BiomeId {
        self.sample_biome_internal(world_x.div_euclid(4), world_z.div_euclid(4))
    }

    fn sample_biome_internal(&self, world_x: i32, world_z: i32) -> BiomeId {
        let x = f64::from(world_x) * 0.0028;
        let z = f64::from(world_z) * 0.0028;

        let temperature = ((self.temperature_noise.sample2d(x, z) + 1.0) * 0.5).clamp(0.0, 1.0);
        let moisture =
            ((self.moisture_noise.sample2d(x + 137.0, z - 91.0) + 1.0) * 0.5).clamp(0.0, 1.0);
        let continentalness = self.continental_noise.sample2d(x * 0.5, z * 0.5);

        if continentalness < -0.35 {
            BiomeId::Ocean
        } else if temperature > 0.75 && moisture < 0.35 {
            BiomeId::Desert
        } else if temperature < 0.28 {
            BiomeId::Taiga
        } else if moisture > 0.62 {
            BiomeId::Forest
        } else {
            BiomeId::Plains
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecorationFeature {
    DirtOre,
    GravelOre,
    CoalOre,
    IronOre,
    GoldOre,
    RedstoneOre,
    DiamondOre,
    LapisOre,
    Tree,
    Flower,
    TallGrass,
    DeadBush,
    Reed,
    BrownMushroom,
    RedMushroom,
    WaterLily,
    WaterSpring,
    LavaSpring,
    SandPatch,
    ClayPatch,
    GravelPatch,
    Cactus,
    Pumpkin,
    DesertWell,
    EndSpike,
    EndPodium,
    EndDragonSpawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecorationPlacement {
    pub feature: DecorationFeature,
    pub block: BlockPos,
}

#[derive(Debug, Clone)]
pub struct BiomeDecorator {
    seed: i64,
}

impl BiomeDecorator {
    pub fn new(seed: i64) -> Self {
        Self { seed }
    }

    pub fn decorate_chunk<FTop, FHeight>(
        &self,
        chunk: ChunkPos,
        biomes: &BiomeMap,
        top_solid_block_at: FTop,
        heightmap_at: FHeight,
    ) -> Vec<DecorationPlacement>
    where
        FTop: Fn(i32, i32) -> i32,
        FHeight: Fn(i32, i32) -> i32,
    {
        let dominant_biome = biomes.at(CHUNK_SIDE_USIZE / 2, CHUNK_SIDE_USIZE / 2);
        let settings = DecoratorSettings::for_biome(dominant_biome);
        let mut random = JavaRandom::new(chunk_decoration_seed(self.seed, chunk));
        let mut placements = Vec::new();

        place_ore_depth_span_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::DirtOre,
            20,
            0,
            WORLD_HEIGHT,
        );
        place_ore_depth_span_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::GravelOre,
            10,
            0,
            WORLD_HEIGHT,
        );
        place_ore_depth_span_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::CoalOre,
            20,
            0,
            WORLD_HEIGHT,
        );
        place_ore_depth_span_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::IronOre,
            20,
            0,
            WORLD_HEIGHT / 2,
        );
        place_ore_depth_span_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::GoldOre,
            2,
            0,
            WORLD_HEIGHT / 4,
        );
        place_ore_depth_span_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::RedstoneOre,
            8,
            0,
            WORLD_HEIGHT / 8,
        );
        place_ore_depth_span_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::DiamondOre,
            1,
            0,
            WORLD_HEIGHT / 8,
        );
        place_ore_depth_average_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::LapisOre,
            1,
            WORLD_HEIGHT / 8,
            WORLD_HEIGHT / 8,
        );

        place_top_solid_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            &top_solid_block_at,
            DecorationFeature::SandPatch,
            settings.sand_count,
        );
        place_top_solid_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            &top_solid_block_at,
            DecorationFeature::ClayPatch,
            settings.clay_count,
        );
        place_top_solid_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            &top_solid_block_at,
            DecorationFeature::GravelPatch,
            settings.gravel_count,
        );

        let mut tree_attempts = settings.tree_count;
        if random.next_i32_bound(10) == 0 {
            tree_attempts = tree_attempts.saturating_add(1);
        }

        place_heightmap_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            &heightmap_at,
            DecorationFeature::Tree,
            tree_attempts.max(0),
        );

        place_depth_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::Flower,
            settings.flower_count.max(0),
        );
        place_depth_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::TallGrass,
            settings.grass_count.max(0),
        );
        place_depth_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::DeadBush,
            settings.dead_bush_count.max(0),
        );

        place_waterlily_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            &heightmap_at,
            DecorationFeature::WaterLily,
            settings.waterlily_count.max(0),
        );

        for _ in 0..settings.mushroom_count.max(0) {
            if random.next_i32_bound(4) == 0 {
                place_heightmap_feature_attempts(
                    &mut placements,
                    &mut random,
                    chunk,
                    &heightmap_at,
                    DecorationFeature::BrownMushroom,
                    1,
                );
            }
            if random.next_i32_bound(8) == 0 {
                place_depth_feature_attempts(
                    &mut placements,
                    &mut random,
                    chunk,
                    DecorationFeature::RedMushroom,
                    1,
                );
            }
        }

        if random.next_i32_bound(4) == 0 {
            place_depth_feature_attempts(
                &mut placements,
                &mut random,
                chunk,
                DecorationFeature::BrownMushroom,
                1,
            );
        }

        if random.next_i32_bound(8) == 0 {
            place_depth_feature_attempts(
                &mut placements,
                &mut random,
                chunk,
                DecorationFeature::RedMushroom,
                1,
            );
        }

        place_depth_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::Reed,
            settings.reeds_count.max(0),
        );
        place_depth_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::Reed,
            10,
        );

        if random.next_i32_bound(32) == 0 {
            place_depth_feature_attempts(
                &mut placements,
                &mut random,
                chunk,
                DecorationFeature::Pumpkin,
                1,
            );
        }

        place_depth_feature_attempts(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::Cactus,
            settings.cactus_count.max(0),
        );

        if settings.liquids {
            place_water_spring_feature_attempts(
                &mut placements,
                &mut random,
                chunk,
                DecorationFeature::WaterSpring,
                50,
            );
            place_lava_spring_feature_attempts(
                &mut placements,
                &mut random,
                chunk,
                DecorationFeature::LavaSpring,
                20,
            );
        }

        if dominant_biome == BiomeId::Desert && random.next_i32_bound(1_000) == 0 {
            let world_x = chunk.x * CHUNK_SIDE_I32
                + random.next_i32_bound(CHUNK_SIDE_I32)
                + DECORATOR_EDGE_OFFSET;
            let world_z = chunk.z * CHUNK_SIDE_I32
                + random.next_i32_bound(CHUNK_SIDE_I32)
                + DECORATOR_EDGE_OFFSET;
            let y = (heightmap_at(world_x, world_z) + 1).clamp(1, WORLD_HEIGHT - 1);

            placements.push(DecorationPlacement {
                feature: DecorationFeature::DesertWell,
                block: BlockPos::new(world_x, y, world_z),
            });
        }

        placements
    }
}

#[derive(Debug, Clone, Copy)]
struct DecoratorSettings {
    waterlily_count: i32,
    tree_count: i32,
    flower_count: i32,
    grass_count: i32,
    dead_bush_count: i32,
    mushroom_count: i32,
    reeds_count: i32,
    cactus_count: i32,
    gravel_count: i32,
    sand_count: i32,
    clay_count: i32,
    liquids: bool,
}

impl DecoratorSettings {
    fn for_biome(biome: BiomeId) -> Self {
        let mut settings = Self {
            waterlily_count: 0,
            tree_count: 0,
            flower_count: 2,
            grass_count: 1,
            dead_bush_count: 0,
            mushroom_count: 0,
            reeds_count: 0,
            cactus_count: 0,
            gravel_count: 1,
            sand_count: 3,
            clay_count: 1,
            liquids: true,
        };

        match biome {
            BiomeId::Forest => {
                settings.tree_count = 10;
                settings.grass_count = 2;
            }
            BiomeId::Plains => {
                settings.tree_count = -999;
                settings.flower_count = 4;
                settings.grass_count = 10;
            }
            BiomeId::Desert => {
                settings.tree_count = -999;
                settings.flower_count = 0;
                settings.grass_count = 0;
                settings.dead_bush_count = 2;
                settings.reeds_count = 50;
                settings.cactus_count = 10;
            }
            BiomeId::Taiga => {
                settings.tree_count = 10;
                settings.grass_count = 1;
            }
            BiomeId::Ocean => {
                settings.tree_count = -999;
                settings.flower_count = 0;
                settings.grass_count = 0;
                settings.cactus_count = 0;
            }
            BiomeId::Hell | BiomeId::TheEnd => {
                settings.tree_count = -999;
                settings.flower_count = 0;
                settings.grass_count = 0;
                settings.dead_bush_count = 0;
                settings.reeds_count = 0;
                settings.cactus_count = 0;
                settings.sand_count = 0;
                settings.clay_count = 0;
                settings.gravel_count = 0;
                settings.liquids = false;
            }
        }

        settings
    }
}

const DECORATOR_EDGE_OFFSET: i32 = 8;

fn place_ore_depth_span_attempts(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut JavaRandom,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
    y0: i32,
    y1: i32,
) {
    if y1 <= y0 {
        return;
    }

    let y_span = y1 - y0;
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * CHUNK_SIDE_I32 + random.next_i32_bound(CHUNK_SIDE_I32);
        let y = random.next_i32_bound(y_span) + y0;
        let world_z = chunk.z * CHUNK_SIDE_I32 + random.next_i32_bound(CHUNK_SIDE_I32);

        placements.push(DecorationPlacement {
            feature,
            block: BlockPos::new(world_x, y, world_z),
        });
    }
}

fn place_ore_depth_average_attempts(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut JavaRandom,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
    y_mid: i32,
    y_span: i32,
) {
    if y_span <= 0 {
        return;
    }

    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * CHUNK_SIDE_I32 + random.next_i32_bound(CHUNK_SIDE_I32);
        let y = random.next_i32_bound(y_span) + random.next_i32_bound(y_span) + (y_mid - y_span);
        let world_z = chunk.z * CHUNK_SIDE_I32 + random.next_i32_bound(CHUNK_SIDE_I32);

        placements.push(DecorationPlacement {
            feature,
            block: BlockPos::new(world_x, y, world_z),
        });
    }
}

fn place_top_solid_feature_attempts<F>(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut JavaRandom,
    chunk: ChunkPos,
    top_solid_block_at: &F,
    feature: DecorationFeature,
    attempts: i32,
) where
    F: Fn(i32, i32) -> i32,
{
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;
        let world_z = chunk.z * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;
        let y = top_solid_block_at(world_x, world_z).clamp(1, WORLD_HEIGHT - 1);

        placements.push(DecorationPlacement {
            feature,
            block: BlockPos::new(world_x, y, world_z),
        });
    }
}

fn place_heightmap_feature_attempts<F>(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut JavaRandom,
    chunk: ChunkPos,
    heightmap_at: &F,
    feature: DecorationFeature,
    attempts: i32,
) where
    F: Fn(i32, i32) -> i32,
{
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;
        let world_z = chunk.z * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;
        let y = heightmap_at(world_x, world_z).clamp(1, WORLD_HEIGHT - 1);

        placements.push(DecorationPlacement {
            feature,
            block: BlockPos::new(world_x, y, world_z),
        });
    }
}

fn place_waterlily_feature_attempts<F>(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut JavaRandom,
    chunk: ChunkPos,
    heightmap_at: &F,
    feature: DecorationFeature,
    attempts: i32,
) where
    F: Fn(i32, i32) -> i32,
{
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;
        let world_z = chunk.z * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;
        let mut y = random.next_i32_bound(WORLD_HEIGHT);
        let heightmap_y = heightmap_at(world_x, world_z).clamp(0, WORLD_HEIGHT - 1);
        if y > heightmap_y {
            y = heightmap_y;
        }

        placements.push(DecorationPlacement {
            feature,
            block: BlockPos::new(world_x, y, world_z),
        });
    }
}

fn place_depth_feature_attempts(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut JavaRandom,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
) {
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;
        let y = random.next_i32_bound(WORLD_HEIGHT);
        let world_z = chunk.z * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;

        placements.push(DecorationPlacement {
            feature,
            block: BlockPos::new(world_x, y, world_z),
        });
    }
}

fn place_water_spring_feature_attempts(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut JavaRandom,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
) {
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;
        let y_upper = random.next_i32_bound(WORLD_HEIGHT - 8) + 8;
        let y = random.next_i32_bound(y_upper);
        let world_z = chunk.z * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;

        placements.push(DecorationPlacement {
            feature,
            block: BlockPos::new(world_x, y, world_z),
        });
    }
}

fn place_lava_spring_feature_attempts(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut JavaRandom,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
) {
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;
        let y_outer = random.next_i32_bound(WORLD_HEIGHT - 16) + 8;
        let y_mid = random.next_i32_bound(y_outer) + 8;
        let y = random.next_i32_bound(y_mid);
        let world_z = chunk.z * CHUNK_SIDE_I32
            + random.next_i32_bound(CHUNK_SIDE_I32)
            + DECORATOR_EDGE_OFFSET;

        placements.push(DecorationPlacement {
            feature,
            block: BlockPos::new(world_x, y, world_z),
        });
    }
}

fn chunk_decoration_seed(seed: i64, chunk: ChunkPos) -> i64 {
    let mut random = JavaRandom::new(seed);
    let x_seed = (random.next_i64() >> 1 << 1) + 1;
    let z_seed = (random.next_i64() >> 1 << 1) + 1;

    (i64::from(chunk.x)
        .wrapping_mul(x_seed)
        .wrapping_add(i64::from(chunk.z).wrapping_mul(z_seed)))
        ^ seed
}

const JAVA_MULTIPLIER: u64 = 0x5DEECE66D;
const JAVA_ADDEND: u64 = 0xB;
const JAVA_MASK: u64 = (1u64 << 48) - 1;

#[derive(Debug, Clone)]
struct JavaRandom {
    state: u64,
}

impl JavaRandom {
    fn new(seed: i64) -> Self {
        Self {
            state: (u64::from_ne_bytes(seed.to_ne_bytes()) ^ JAVA_MULTIPLIER) & JAVA_MASK,
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.state = (self
            .state
            .wrapping_mul(JAVA_MULTIPLIER)
            .wrapping_add(JAVA_ADDEND))
            & JAVA_MASK;

        (self.state >> (48 - bits)) as u32
    }

    fn next_i32_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");

        if (bound & -bound) == bound {
            return (((i64::from(bound)) * i64::from(self.next_bits(31))) >> 31) as i32;
        }

        let bound_i64 = i64::from(bound);

        loop {
            let bits = i64::from(self.next_bits(31));
            let value = bits % bound_i64;
            if bits - value + (bound_i64 - 1) >= 0 {
                return value as i32;
            }
        }
    }

    fn next_i64(&mut self) -> i64 {
        let high = u64::from(self.next_bits(32));
        let low = u64::from(self.next_bits(32));
        i64::from_ne_bytes(((high << 32) | low).to_ne_bytes())
    }
}

fn chunk_index(local_x: usize, local_z: usize) -> usize {
    assert!(local_x < CHUNK_SIDE_USIZE, "local_x out of chunk range");
    assert!(local_z < CHUNK_SIDE_USIZE, "local_z out of chunk range");
    local_z * CHUNK_SIDE_USIZE + local_x
}

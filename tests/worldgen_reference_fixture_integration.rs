use lce_rust::world::ChunkPos;
use lce_rust::world::worldgen::{
    BiomeDecorator, BiomeId, BiomeMap, DecorationFeature, DecorationPlacement, SEA_LEVEL,
};

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0001_0000_01b3;

const FOREST_FIXTURE_SIGNATURE: u64 = 12_929_510_298_300_529_021;
const PLAINS_FIXTURE_SIGNATURE: u64 = 12_351_481_780_528_278_895;
const DESERT_FIXTURE_SIGNATURE: u64 = 6_631_132_580_923_923_344;
const TAIGA_FIXTURE_SIGNATURE: u64 = 118_249_654_693_268_710;
const HELL_FIXTURE_SIGNATURE: u64 = 3_922_985_679_157_671_802;

#[test]
fn decorator_matches_seeded_reference_fixtures() {
    let cases = [
        FixtureCase {
            label: "forest",
            seed: 2_147_001,
            chunk: ChunkPos::new(3, -2),
            biome: BiomeId::Forest,
            fixture_signature: FOREST_FIXTURE_SIGNATURE,
        },
        FixtureCase {
            label: "plains",
            seed: 2_147_001,
            chunk: ChunkPos::new(3, -2),
            biome: BiomeId::Plains,
            fixture_signature: PLAINS_FIXTURE_SIGNATURE,
        },
        FixtureCase {
            label: "desert",
            seed: 9_911,
            chunk: ChunkPos::new(-4, 7),
            biome: BiomeId::Desert,
            fixture_signature: DESERT_FIXTURE_SIGNATURE,
        },
        FixtureCase {
            label: "taiga",
            seed: -321_007,
            chunk: ChunkPos::new(11, 5),
            biome: BiomeId::Taiga,
            fixture_signature: TAIGA_FIXTURE_SIGNATURE,
        },
        FixtureCase {
            label: "hell",
            seed: 51,
            chunk: ChunkPos::new(-1, -1),
            biome: BiomeId::Hell,
            fixture_signature: HELL_FIXTURE_SIGNATURE,
        },
    ];

    for case in cases {
        let biomes = BiomeMap::filled(case.biome);

        let expected = reference_decorate_chunk(case.seed, case.chunk, case.biome);
        let actual = BiomeDecorator::new(case.seed).decorate_chunk(
            case.chunk,
            &biomes,
            |x, z| fixture_top_solid_block(case.seed, x, z),
            |x, z| fixture_heightmap(case.seed, x, z),
        );

        assert_eq!(
            actual, expected,
            "{} case diverged from reference",
            case.label
        );

        let signature = placement_signature(&actual);
        assert_eq!(
            signature, case.fixture_signature,
            "{} signature changed",
            case.label
        );
    }
}

#[derive(Clone, Copy)]
struct FixtureCase {
    label: &'static str,
    seed: i64,
    chunk: ChunkPos,
    biome: BiomeId,
    fixture_signature: u64,
}

fn fixture_surface_height(seed: i64, world_x: i32, world_z: i32) -> i32 {
    let chunk_x = world_x.div_euclid(16);
    let chunk_z = world_z.div_euclid(16);
    let local_x = world_x.rem_euclid(16);
    let local_z = world_z.rem_euclid(16);
    let mix = i64::from(chunk_x) * 31
        + i64::from(chunk_z) * 17
        + i64::from(local_x) * 11
        + i64::from(local_z) * 7
        + seed.rem_euclid(97);

    48 + mix.rem_euclid(48) as i32
}

fn fixture_top_solid_block(seed: i64, world_x: i32, world_z: i32) -> i32 {
    fixture_surface_height(seed, world_x, world_z) + 1
}

fn fixture_heightmap(seed: i64, world_x: i32, world_z: i32) -> i32 {
    let surface_height = fixture_surface_height(seed, world_x, world_z);
    if surface_height < SEA_LEVEL {
        SEA_LEVEL + 1
    } else {
        surface_height + 1
    }
}

fn reference_decorate_chunk(
    seed: i64,
    chunk: ChunkPos,
    dominant_biome: BiomeId,
) -> Vec<DecorationPlacement> {
    let settings = reference_settings_for_biome(dominant_biome);
    let mut random = ReferenceJavaRandom::new(reference_chunk_seed(seed, chunk));
    let mut placements = Vec::new();

    reference_place_ore_depth_span(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::DirtOre,
        20,
        0,
        128,
    );
    reference_place_ore_depth_span(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::GravelOre,
        10,
        0,
        128,
    );
    reference_place_ore_depth_span(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::CoalOre,
        20,
        0,
        128,
    );
    reference_place_ore_depth_span(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::IronOre,
        20,
        0,
        64,
    );
    reference_place_ore_depth_span(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::GoldOre,
        2,
        0,
        32,
    );
    reference_place_ore_depth_span(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::RedstoneOre,
        8,
        0,
        16,
    );
    reference_place_ore_depth_span(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::DiamondOre,
        1,
        0,
        16,
    );
    reference_place_ore_depth_average(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::LapisOre,
        1,
        16,
        16,
    );

    reference_place_top_solid(
        &mut placements,
        &mut random,
        seed,
        chunk,
        DecorationFeature::SandPatch,
        settings.sand_count,
    );
    reference_place_top_solid(
        &mut placements,
        &mut random,
        seed,
        chunk,
        DecorationFeature::ClayPatch,
        settings.clay_count,
    );
    reference_place_top_solid(
        &mut placements,
        &mut random,
        seed,
        chunk,
        DecorationFeature::GravelPatch,
        settings.gravel_count,
    );

    let mut tree_count = settings.tree_count;
    if random.next_i32_bound(10) == 0 {
        tree_count = tree_count.saturating_add(1);
    }
    reference_place_heightmap(
        &mut placements,
        &mut random,
        seed,
        chunk,
        DecorationFeature::Tree,
        tree_count.max(0),
    );

    reference_place_depth(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::Flower,
        settings.flower_count.max(0),
    );
    reference_place_depth(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::TallGrass,
        settings.grass_count.max(0),
    );
    reference_place_depth(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::DeadBush,
        settings.dead_bush_count.max(0),
    );
    reference_place_waterlily(
        &mut placements,
        &mut random,
        seed,
        chunk,
        DecorationFeature::WaterLily,
        settings.waterlily_count.max(0),
    );

    for _ in 0..settings.mushroom_count.max(0) {
        if random.next_i32_bound(4) == 0 {
            reference_place_heightmap(
                &mut placements,
                &mut random,
                seed,
                chunk,
                DecorationFeature::BrownMushroom,
                1,
            );
        }

        if random.next_i32_bound(8) == 0 {
            reference_place_depth(
                &mut placements,
                &mut random,
                chunk,
                DecorationFeature::RedMushroom,
                1,
            );
        }
    }

    if random.next_i32_bound(4) == 0 {
        reference_place_depth(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::BrownMushroom,
            1,
        );
    }

    if random.next_i32_bound(8) == 0 {
        reference_place_depth(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::RedMushroom,
            1,
        );
    }

    reference_place_depth(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::Reed,
        settings.reeds_count.max(0),
    );
    reference_place_depth(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::Reed,
        10,
    );

    if random.next_i32_bound(32) == 0 {
        reference_place_depth(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::Pumpkin,
            1,
        );
    }

    reference_place_depth(
        &mut placements,
        &mut random,
        chunk,
        DecorationFeature::Cactus,
        settings.cactus_count.max(0),
    );

    if settings.liquids {
        reference_place_water_springs(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::WaterSpring,
            50,
        );
        reference_place_lava_springs(
            &mut placements,
            &mut random,
            chunk,
            DecorationFeature::LavaSpring,
            20,
        );
    }

    if dominant_biome == BiomeId::Desert && random.next_i32_bound(1_000) == 0 {
        let world_x = chunk.x * 16 + random.next_i32_bound(16) + 8;
        let world_z = chunk.z * 16 + random.next_i32_bound(16) + 8;
        let y = (fixture_heightmap(seed, world_x, world_z) + 1).clamp(1, 127);

        placements.push(DecorationPlacement {
            feature: DecorationFeature::DesertWell,
            block: lce_rust::world::BlockPos::new(world_x, y, world_z),
        });
    }

    placements
}

fn reference_place_ore_depth_span(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut ReferenceJavaRandom,
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
        let world_x = chunk.x * 16 + random.next_i32_bound(16);
        let y = random.next_i32_bound(y_span) + y0;
        let world_z = chunk.z * 16 + random.next_i32_bound(16);

        placements.push(DecorationPlacement {
            feature,
            block: lce_rust::world::BlockPos::new(world_x, y, world_z),
        });
    }
}

fn reference_place_ore_depth_average(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut ReferenceJavaRandom,
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
        let world_x = chunk.x * 16 + random.next_i32_bound(16);
        let y = random.next_i32_bound(y_span) + random.next_i32_bound(y_span) + (y_mid - y_span);
        let world_z = chunk.z * 16 + random.next_i32_bound(16);

        placements.push(DecorationPlacement {
            feature,
            block: lce_rust::world::BlockPos::new(world_x, y, world_z),
        });
    }
}

fn reference_place_top_solid(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut ReferenceJavaRandom,
    seed: i64,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
) {
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * 16 + random.next_i32_bound(16) + 8;
        let world_z = chunk.z * 16 + random.next_i32_bound(16) + 8;
        let y = fixture_top_solid_block(seed, world_x, world_z).clamp(1, 127);

        placements.push(DecorationPlacement {
            feature,
            block: lce_rust::world::BlockPos::new(world_x, y, world_z),
        });
    }
}

fn reference_place_heightmap(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut ReferenceJavaRandom,
    seed: i64,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
) {
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * 16 + random.next_i32_bound(16) + 8;
        let world_z = chunk.z * 16 + random.next_i32_bound(16) + 8;
        let y = fixture_heightmap(seed, world_x, world_z).clamp(1, 127);

        placements.push(DecorationPlacement {
            feature,
            block: lce_rust::world::BlockPos::new(world_x, y, world_z),
        });
    }
}

fn reference_place_waterlily(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut ReferenceJavaRandom,
    seed: i64,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
) {
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * 16 + random.next_i32_bound(16) + 8;
        let world_z = chunk.z * 16 + random.next_i32_bound(16) + 8;
        let mut y = random.next_i32_bound(128);
        let heightmap = fixture_heightmap(seed, world_x, world_z).clamp(0, 127);
        if y > heightmap {
            y = heightmap;
        }

        placements.push(DecorationPlacement {
            feature,
            block: lce_rust::world::BlockPos::new(world_x, y, world_z),
        });
    }
}

fn reference_place_depth(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut ReferenceJavaRandom,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
) {
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * 16 + random.next_i32_bound(16) + 8;
        let y = random.next_i32_bound(128);
        let world_z = chunk.z * 16 + random.next_i32_bound(16) + 8;

        placements.push(DecorationPlacement {
            feature,
            block: lce_rust::world::BlockPos::new(world_x, y, world_z),
        });
    }
}

fn reference_place_water_springs(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut ReferenceJavaRandom,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
) {
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * 16 + random.next_i32_bound(16) + 8;
        let y_upper = random.next_i32_bound(128 - 8) + 8;
        let y = random.next_i32_bound(y_upper);
        let world_z = chunk.z * 16 + random.next_i32_bound(16) + 8;

        placements.push(DecorationPlacement {
            feature,
            block: lce_rust::world::BlockPos::new(world_x, y, world_z),
        });
    }
}

fn reference_place_lava_springs(
    placements: &mut Vec<DecorationPlacement>,
    random: &mut ReferenceJavaRandom,
    chunk: ChunkPos,
    feature: DecorationFeature,
    attempts: i32,
) {
    for _ in 0..attempts.max(0) {
        let world_x = chunk.x * 16 + random.next_i32_bound(16) + 8;
        let y_outer = random.next_i32_bound(128 - 16) + 8;
        let y_mid = random.next_i32_bound(y_outer) + 8;
        let y = random.next_i32_bound(y_mid);
        let world_z = chunk.z * 16 + random.next_i32_bound(16) + 8;

        placements.push(DecorationPlacement {
            feature,
            block: lce_rust::world::BlockPos::new(world_x, y, world_z),
        });
    }
}

#[derive(Clone, Copy)]
struct ReferenceDecoratorSettings {
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

fn reference_settings_for_biome(biome: BiomeId) -> ReferenceDecoratorSettings {
    let mut settings = ReferenceDecoratorSettings {
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

fn reference_chunk_seed(seed: i64, chunk: ChunkPos) -> i64 {
    let mut random = ReferenceJavaRandom::new(seed);
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

#[derive(Clone)]
struct ReferenceJavaRandom {
    state: u64,
}

impl ReferenceJavaRandom {
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

fn placement_signature(placements: &[DecorationPlacement]) -> u64 {
    let mut state = FNV_OFFSET_BASIS;

    for placement in placements {
        state = fnv1a_u64(state, feature_to_u64(placement.feature));
        state = fnv1a_u64(state, placement.block.x as u64);
        state = fnv1a_u64(state, placement.block.y as u64);
        state = fnv1a_u64(state, placement.block.z as u64);
    }

    state
}

fn fnv1a_u64(state: u64, value: u64) -> u64 {
    (state ^ value).wrapping_mul(FNV_PRIME)
}

fn feature_to_u64(feature: DecorationFeature) -> u64 {
    match feature {
        DecorationFeature::DirtOre => 11,
        DecorationFeature::GravelOre => 12,
        DecorationFeature::CoalOre => 13,
        DecorationFeature::IronOre => 14,
        DecorationFeature::GoldOre => 15,
        DecorationFeature::RedstoneOre => 16,
        DecorationFeature::DiamondOre => 17,
        DecorationFeature::LapisOre => 18,
        DecorationFeature::Tree => 19,
        DecorationFeature::Flower => 20,
        DecorationFeature::TallGrass => 21,
        DecorationFeature::DeadBush => 22,
        DecorationFeature::Reed => 23,
        DecorationFeature::BrownMushroom => 24,
        DecorationFeature::RedMushroom => 25,
        DecorationFeature::WaterLily => 26,
        DecorationFeature::WaterSpring => 27,
        DecorationFeature::LavaSpring => 28,
        DecorationFeature::SandPatch => 29,
        DecorationFeature::ClayPatch => 30,
        DecorationFeature::GravelPatch => 31,
        DecorationFeature::Cactus => 32,
        DecorationFeature::Pumpkin => 33,
        DecorationFeature::DesertWell => 34,
        DecorationFeature::EndSpike => 35,
        DecorationFeature::EndPodium => 36,
        DecorationFeature::EndDragonSpawn => 37,
    }
}

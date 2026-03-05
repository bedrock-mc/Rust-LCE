use lce_rust::world::ChunkPos;
use lce_rust::world::worldgen::{
    BiomeId, GeneratedChunk, HellRandomLevelSource, PerlinNoise, RandomLevelSource, SimplexNoise,
    TheEndLevelRandomLevelSource,
};

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0001_0000_01b3;
const PERLIN_SEED_13337_FIXTURE: u64 = 11_242_637_641_106_292_887;
const SIMPLEX_SEED_NEG_91245_FIXTURE: u64 = 7_604_815_158_596_070_335;
const RANDOM_LEVEL_CHUNK_FIXTURE: u64 = 10_024_521_834_533_616_142;

#[test]
fn perlin_noise_seeded_fixture_is_stable() {
    let first = perlin_signature(13_337);
    let second = perlin_signature(13_337);
    let different_seed = perlin_signature(13_338);

    assert_eq!(first, second);
    assert_ne!(first, different_seed);
    assert_eq!(first, PERLIN_SEED_13337_FIXTURE);
}

#[test]
fn simplex_noise_seeded_fixture_is_stable() {
    let first = simplex_signature(-91_245);
    let second = simplex_signature(-91_245);
    let different_seed = simplex_signature(-91_244);

    assert_eq!(first, second);
    assert_ne!(first, different_seed);
    assert_eq!(first, SIMPLEX_SEED_NEG_91245_FIXTURE);
}

#[test]
fn random_level_source_chunk_generation_is_deterministic() {
    let source = RandomLevelSource::new(77_881);
    let first = source.generate_chunk(ChunkPos::new(0, 0));
    let second = source.generate_chunk(ChunkPos::new(0, 0));

    assert_eq!(first, second);

    let fixture_signature = chunk_signature(&first);
    let different_seed_signature =
        chunk_signature(&RandomLevelSource::new(77_882).generate_chunk(ChunkPos::new(0, 0)));

    assert_ne!(fixture_signature, different_seed_signature);
    assert_eq!(fixture_signature, RANDOM_LEVEL_CHUNK_FIXTURE);
}

#[test]
fn hell_and_end_level_sources_anchor_expected_biomes() {
    let hell_chunk = HellRandomLevelSource::new(5_123).generate_chunk(ChunkPos::new(2, -1));
    assert!(
        hell_chunk
            .biome_map
            .values()
            .iter()
            .all(|biome| *biome == BiomeId::Hell)
    );

    let end_chunk = TheEndLevelRandomLevelSource::new(5_123).generate_chunk(ChunkPos::new(-3, 4));
    assert!(
        end_chunk
            .biome_map
            .values()
            .iter()
            .all(|biome| *biome == BiomeId::TheEnd)
    );
}

fn perlin_signature(seed: i64) -> u64 {
    let noise = PerlinNoise::new(seed);
    let samples = [
        noise.sample3d(0.0, 0.0, 0.0),
        noise.sample3d(17.25, 63.5, -9.0),
        noise.sample2d(-128.5, 42.25),
        noise.sample3d(512.0, 12.0, 512.0),
        noise.sample2d(24.125, -64.875),
    ];

    hash_f64_samples(&samples)
}

fn simplex_signature(seed: i64) -> u64 {
    let noise = SimplexNoise::new(seed);
    let samples = [
        noise.sample2d(0.0, 0.0),
        noise.sample2d(3.75, -11.5),
        noise.sample2d(42.25, 78.625),
        noise.sample3d(1.0, 2.0, 3.0),
        noise.sample3d(-40.5, 12.25, 5.875),
    ];

    hash_f64_samples(&samples)
}

fn chunk_signature(chunk: &GeneratedChunk) -> u64 {
    let mut state = FNV_OFFSET_BASIS;
    state = fnv1a_u64(state, chunk.chunk.x as u64);
    state = fnv1a_u64(state, chunk.chunk.z as u64);

    for biome in chunk.biome_map.values() {
        state = fnv1a_u64(state, biome_to_u64(*biome));
    }

    for height in chunk.surface_heights {
        state = fnv1a_u64(state, height as u64);
    }

    for (position, block_id) in &chunk.blocks {
        state = fnv1a_u64(state, position.x as u64);
        state = fnv1a_u64(state, position.y as u64);
        state = fnv1a_u64(state, position.z as u64);
        state = fnv1a_u64(state, u64::from(*block_id));
    }

    for placement in &chunk.decorations {
        state = fnv1a_u64(state, feature_to_u64(placement.feature));
        state = fnv1a_u64(state, placement.block.x as u64);
        state = fnv1a_u64(state, placement.block.y as u64);
        state = fnv1a_u64(state, placement.block.z as u64);
    }

    state
}

fn hash_f64_samples(samples: &[f64]) -> u64 {
    let mut state = FNV_OFFSET_BASIS;
    for sample in samples {
        state = fnv1a_u64(state, sample.to_bits());
    }

    state
}

fn fnv1a_u64(state: u64, value: u64) -> u64 {
    (state ^ value).wrapping_mul(FNV_PRIME)
}

fn biome_to_u64(biome: BiomeId) -> u64 {
    match biome {
        BiomeId::Ocean => 1,
        BiomeId::Plains => 2,
        BiomeId::Desert => 3,
        BiomeId::Forest => 4,
        BiomeId::Taiga => 5,
        BiomeId::Hell => 6,
        BiomeId::TheEnd => 7,
    }
}

fn feature_to_u64(feature: lce_rust::world::worldgen::DecorationFeature) -> u64 {
    match feature {
        lce_rust::world::worldgen::DecorationFeature::DirtOre => 11,
        lce_rust::world::worldgen::DecorationFeature::GravelOre => 12,
        lce_rust::world::worldgen::DecorationFeature::CoalOre => 13,
        lce_rust::world::worldgen::DecorationFeature::IronOre => 14,
        lce_rust::world::worldgen::DecorationFeature::GoldOre => 15,
        lce_rust::world::worldgen::DecorationFeature::RedstoneOre => 16,
        lce_rust::world::worldgen::DecorationFeature::DiamondOre => 17,
        lce_rust::world::worldgen::DecorationFeature::LapisOre => 18,
        lce_rust::world::worldgen::DecorationFeature::Tree => 19,
        lce_rust::world::worldgen::DecorationFeature::Flower => 20,
        lce_rust::world::worldgen::DecorationFeature::TallGrass => 21,
        lce_rust::world::worldgen::DecorationFeature::DeadBush => 22,
        lce_rust::world::worldgen::DecorationFeature::Reed => 23,
        lce_rust::world::worldgen::DecorationFeature::BrownMushroom => 24,
        lce_rust::world::worldgen::DecorationFeature::RedMushroom => 25,
        lce_rust::world::worldgen::DecorationFeature::WaterLily => 26,
        lce_rust::world::worldgen::DecorationFeature::WaterSpring => 27,
        lce_rust::world::worldgen::DecorationFeature::LavaSpring => 28,
        lce_rust::world::worldgen::DecorationFeature::SandPatch => 29,
        lce_rust::world::worldgen::DecorationFeature::ClayPatch => 30,
        lce_rust::world::worldgen::DecorationFeature::GravelPatch => 31,
        lce_rust::world::worldgen::DecorationFeature::Cactus => 32,
        lce_rust::world::worldgen::DecorationFeature::Pumpkin => 33,
        lce_rust::world::worldgen::DecorationFeature::DesertWell => 34,
        lce_rust::world::worldgen::DecorationFeature::EndSpike => 35,
        lce_rust::world::worldgen::DecorationFeature::EndPodium => 36,
        lce_rust::world::worldgen::DecorationFeature::EndDragonSpawn => 37,
    }
}

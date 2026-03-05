use lce_rust::world::ChunkPos;
use lce_rust::world::worldgen::{
    BiomeDecorator, BiomeId, BiomeMap, DecorationFeature, RandomLevelSource, WORLD_HEIGHT,
};

#[test]
fn decorator_output_is_deterministic_per_seed_and_chunk() {
    let source = RandomLevelSource::new(19_870_102);
    let first = source.generate_chunk(ChunkPos::new(4, -3));
    let second = source.generate_chunk(ChunkPos::new(4, -3));

    assert_eq!(first.decorations, second.decorations);
}

#[test]
fn plains_tree_count_respects_negative_sentinel_behavior() {
    let decorator = BiomeDecorator::new(42);
    let chunk = ChunkPos::new(0, 0);
    let biomes = BiomeMap::filled(BiomeId::Plains);

    let placements = decorator.decorate_chunk(chunk, &biomes, |_, _| 70, |_, _| 70);
    let trees = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::Tree)
        .count();

    assert_eq!(trees, 0);
}

#[test]
fn ore_attempt_counts_and_depth_ranges_match_cxx_decorate_ores() {
    let decorator = BiomeDecorator::new(123_456);
    let chunk = ChunkPos::new(0, 0);
    let placements = decorator.decorate_chunk(
        chunk,
        &BiomeMap::filled(BiomeId::Forest),
        |_, _| 70,
        |_, _| 70,
    );

    let dirt = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::DirtOre)
        .collect::<Vec<_>>();
    let gravel = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::GravelOre)
        .collect::<Vec<_>>();
    let coal = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::CoalOre)
        .collect::<Vec<_>>();
    let iron = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::IronOre)
        .collect::<Vec<_>>();
    let gold = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::GoldOre)
        .collect::<Vec<_>>();
    let redstone = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::RedstoneOre)
        .collect::<Vec<_>>();
    let diamond = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::DiamondOre)
        .collect::<Vec<_>>();
    let lapis = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::LapisOre)
        .collect::<Vec<_>>();

    assert_eq!(dirt.len(), 20);
    assert_eq!(gravel.len(), 10);
    assert_eq!(coal.len(), 20);
    assert_eq!(iron.len(), 20);
    assert_eq!(gold.len(), 2);
    assert_eq!(redstone.len(), 8);
    assert_eq!(diamond.len(), 1);
    assert_eq!(lapis.len(), 1);

    for ore in dirt.iter().chain(gravel.iter()).chain(coal.iter()) {
        assert!((0..WORLD_HEIGHT).contains(&ore.block.y));
    }
    for ore in iron {
        assert!((0..(WORLD_HEIGHT / 2)).contains(&ore.block.y));
    }
    for ore in gold {
        assert!((0..(WORLD_HEIGHT / 4)).contains(&ore.block.y));
    }
    for ore in redstone.iter().chain(diamond.iter()) {
        assert!((0..(WORLD_HEIGHT / 8)).contains(&ore.block.y));
    }
    for ore in lapis {
        assert!((0..(WORLD_HEIGHT / 4)).contains(&ore.block.y));
    }
}

#[test]
fn forest_decorator_emits_more_tree_attempts_than_plains() {
    let decorator = BiomeDecorator::new(73);
    let chunk = ChunkPos::new(3, 2);

    let forest = decorator.decorate_chunk(
        chunk,
        &BiomeMap::filled(BiomeId::Forest),
        |_, _| 72,
        |_, _| 72,
    );
    let plains = decorator.decorate_chunk(
        chunk,
        &BiomeMap::filled(BiomeId::Plains),
        |_, _| 72,
        |_, _| 72,
    );

    let forest_tree_count = forest
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::Tree)
        .count();
    let plains_tree_count = plains
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::Tree)
        .count();

    assert!(forest_tree_count > plains_tree_count);
}

#[test]
fn desert_decorator_emits_cactus_attempts_and_no_trees() {
    let decorator = BiomeDecorator::new(-1122);
    let chunk = ChunkPos::new(-1, 7);
    let placements = decorator.decorate_chunk(
        chunk,
        &BiomeMap::filled(BiomeId::Desert),
        |_, _| 68,
        |_, _| 68,
    );

    let cactus_count = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::Cactus)
        .count();
    let tree_count = placements
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::Tree)
        .count();

    assert!(cactus_count >= 10);
    assert_eq!(tree_count, 0);
}

#[test]
fn pumpkin_spawn_is_optional_and_single_attempt_per_chunk() {
    let biomes = [
        BiomeId::Forest,
        BiomeId::Plains,
        BiomeId::Desert,
        BiomeId::Taiga,
        BiomeId::Ocean,
        BiomeId::Hell,
        BiomeId::TheEnd,
    ];
    let chunk = ChunkPos::new(2, -1);
    let mut found_pumpkin = false;

    for seed in 0_i64..512 {
        let decorator = BiomeDecorator::new(seed);
        for biome in biomes {
            let placements =
                decorator.decorate_chunk(chunk, &BiomeMap::filled(biome), |_, _| 70, |_, _| 70);
            let pumpkin_count = placements
                .iter()
                .filter(|placement| placement.feature == DecorationFeature::Pumpkin)
                .count();

            assert!(pumpkin_count <= 1);
            if pumpkin_count == 1 {
                found_pumpkin = true;
            }
        }

        if found_pumpkin {
            break;
        }
    }

    assert!(
        found_pumpkin,
        "expected at least one pumpkin attempt in scan range"
    );
}

#[test]
fn desert_well_spawn_is_desert_only_and_single_attempt_per_chunk() {
    let chunks = [
        ChunkPos::new(0, 0),
        ChunkPos::new(5, -3),
        ChunkPos::new(-7, 9),
    ];
    let mut found_desert_well = false;

    for seed in 0_i64..5_000 {
        let decorator = BiomeDecorator::new(seed);

        for chunk in chunks {
            let desert = decorator.decorate_chunk(
                chunk,
                &BiomeMap::filled(BiomeId::Desert),
                |_, _| 68,
                |_, _| 69,
            );
            let desert_well_count = desert
                .iter()
                .filter(|placement| placement.feature == DecorationFeature::DesertWell)
                .count();
            assert!(desert_well_count <= 1);

            if desert_well_count == 1 {
                found_desert_well = true;
                let well = desert
                    .iter()
                    .find(|placement| placement.feature == DecorationFeature::DesertWell)
                    .expect("desert well placement should exist");
                assert_eq!(well.block.y, 70);
            }

            let forest = decorator.decorate_chunk(
                chunk,
                &BiomeMap::filled(BiomeId::Forest),
                |_, _| 68,
                |_, _| 69,
            );
            let forest_well_count = forest
                .iter()
                .filter(|placement| placement.feature == DecorationFeature::DesertWell)
                .count();
            assert_eq!(forest_well_count, 0);
        }

        if found_desert_well {
            break;
        }
    }

    assert!(
        found_desert_well,
        "expected at least one desert well attempt in scan range"
    );
}

#[test]
fn surface_decorator_placements_are_clamped_to_world_height_bounds() {
    let decorator = BiomeDecorator::new(93);
    let chunk = ChunkPos::new(0, 0);

    let placements = decorator.decorate_chunk(
        chunk,
        &BiomeMap::filled(BiomeId::Forest),
        |x, z| {
            if (x + z).rem_euclid(2) == 0 {
                WORLD_HEIGHT + 128
            } else {
                -32
            }
        },
        |x, z| {
            if (x + z).rem_euclid(2) == 0 {
                WORLD_HEIGHT + 128
            } else {
                -32
            }
        },
    );

    for placement in placements {
        match placement.feature {
            DecorationFeature::SandPatch
            | DecorationFeature::ClayPatch
            | DecorationFeature::GravelPatch
            | DecorationFeature::Tree
            | DecorationFeature::WaterLily => {
                assert!((1..WORLD_HEIGHT).contains(&placement.block.y));
            }
            _ => {}
        }
    }
}

#[test]
fn decorator_placements_follow_cxx_windows_for_ore_and_surface_paths() {
    let decorator = BiomeDecorator::new(7_777);
    let chunks = [
        ChunkPos::new(0, 0),
        ChunkPos::new(3, -4),
        ChunkPos::new(-9, 12),
    ];
    let biomes = [
        BiomeId::Forest,
        BiomeId::Plains,
        BiomeId::Desert,
        BiomeId::Taiga,
        BiomeId::Ocean,
        BiomeId::Hell,
        BiomeId::TheEnd,
    ];

    for chunk in chunks {
        for biome in biomes {
            let placements =
                decorator.decorate_chunk(chunk, &BiomeMap::filled(biome), |_, _| 70, |_, _| 70);

            let ore_min_x = chunk.x * 16;
            let ore_max_x = ore_min_x + 15;
            let ore_min_z = chunk.z * 16;
            let ore_max_z = ore_min_z + 15;
            let deco_min_x = chunk.x * 16 + 8;
            let deco_max_x = deco_min_x + 15;
            let deco_min_z = chunk.z * 16 + 8;
            let deco_max_z = deco_min_z + 15;

            for placement in placements {
                let is_ore = matches!(
                    placement.feature,
                    DecorationFeature::DirtOre
                        | DecorationFeature::GravelOre
                        | DecorationFeature::CoalOre
                        | DecorationFeature::IronOre
                        | DecorationFeature::GoldOre
                        | DecorationFeature::RedstoneOre
                        | DecorationFeature::DiamondOre
                        | DecorationFeature::LapisOre
                );

                if is_ore {
                    assert!((ore_min_x..=ore_max_x).contains(&placement.block.x));
                    assert!((ore_min_z..=ore_max_z).contains(&placement.block.z));
                } else {
                    assert!((deco_min_x..=deco_max_x).contains(&placement.block.x));
                    assert!((deco_min_z..=deco_max_z).contains(&placement.block.z));
                }
                assert!((0..WORLD_HEIGHT).contains(&placement.block.y));
            }
        }
    }
}

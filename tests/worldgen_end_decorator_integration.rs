use lce_rust::world::ChunkPos;
use lce_rust::world::worldgen::{
    DecorationFeature, HellRandomLevelSource, TheEndLevelRandomLevelSource, WORLD_HEIGHT,
};

const END_SPIKE_CHUNKS: [(ChunkPos, i32, i32); 8] = [
    (ChunkPos::new(2, -1), 40, 0),
    (ChunkPos::new(1, 1), 28, 28),
    (ChunkPos::new(-1, 2), 0, 40),
    (ChunkPos::new(-2, 1), -28, 28),
    (ChunkPos::new(-3, -1), -40, 0),
    (ChunkPos::new(-2, -2), -28, -28),
    (ChunkPos::new(-1, -3), 0, -40),
    (ChunkPos::new(1, -2), 28, -28),
];

#[test]
fn end_spike_markers_emit_in_expected_chunks() {
    let source = TheEndLevelRandomLevelSource::new(8_121);

    for (chunk, expected_x, expected_z) in END_SPIKE_CHUNKS {
        let generated = source.generate_chunk(chunk);
        let spikes = generated
            .decorations
            .iter()
            .filter(|placement| placement.feature == DecorationFeature::EndSpike)
            .collect::<Vec<_>>();

        assert_eq!(spikes.len(), 1);
        assert_eq!(spikes[0].block.x, expected_x);
        assert_eq!(spikes[0].block.z, expected_z);
        assert!((1..WORLD_HEIGHT).contains(&spikes[0].block.y));
    }

    let non_spike = source.generate_chunk(ChunkPos::new(3, 3));
    let non_spike_count = non_spike
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::EndSpike)
        .count();
    assert_eq!(non_spike_count, 0);
}

#[test]
fn end_podium_and_dragon_markers_match_cxx_chunk_triggers() {
    let source = TheEndLevelRandomLevelSource::new(8_122);

    let origin_chunk = source.generate_chunk(ChunkPos::new(0, 0));
    let dragon_markers = origin_chunk
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::EndDragonSpawn)
        .collect::<Vec<_>>();
    assert_eq!(dragon_markers.len(), 1);
    assert_eq!(dragon_markers[0].block.x, 0);
    assert_eq!(dragon_markers[0].block.y, WORLD_HEIGHT);
    assert_eq!(dragon_markers[0].block.z, 0);

    let podium_chunk = source.generate_chunk(ChunkPos::new(-1, -1));
    let podium_markers = podium_chunk
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::EndPodium)
        .collect::<Vec<_>>();
    assert_eq!(podium_markers.len(), 1);
    assert_eq!(podium_markers[0].block.x, 0);
    assert_eq!(podium_markers[0].block.y, WORLD_HEIGHT / 2);
    assert_eq!(podium_markers[0].block.z, 0);
}

#[test]
fn end_decorations_keep_ore_pass_and_exclude_non_end_surface_features() {
    let source = TheEndLevelRandomLevelSource::new(12_345);
    let generated = source.generate_chunk(ChunkPos::new(0, 0));

    let ore_count = generated
        .decorations
        .iter()
        .filter(|placement| is_ore_feature(placement.feature))
        .count();
    assert_eq!(ore_count, 82);

    let pumpkin_count = generated
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::Pumpkin)
        .count();
    assert_eq!(pumpkin_count, 0);
}

#[test]
fn hell_level_source_emits_ore_decorator_attempts() {
    let source = HellRandomLevelSource::new(9_991);
    let generated = source.generate_chunk(ChunkPos::new(0, 0));

    let dirt = generated
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::DirtOre)
        .count();
    let gravel = generated
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::GravelOre)
        .count();
    let coal = generated
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::CoalOre)
        .count();
    let iron = generated
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::IronOre)
        .count();
    let gold = generated
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::GoldOre)
        .count();
    let redstone = generated
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::RedstoneOre)
        .count();
    let diamond = generated
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::DiamondOre)
        .count();
    let lapis = generated
        .decorations
        .iter()
        .filter(|placement| placement.feature == DecorationFeature::LapisOre)
        .count();

    assert_eq!(dirt, 20);
    assert_eq!(gravel, 10);
    assert_eq!(coal, 20);
    assert_eq!(iron, 20);
    assert_eq!(gold, 2);
    assert_eq!(redstone, 8);
    assert_eq!(diamond, 1);
    assert_eq!(lapis, 1);
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

use lce_rust::world::{BlockPos, BlockWorld, ChunkPos};

#[test]
fn returns_blocks_for_specific_chunk_only() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(1, 64, 1), 1);
    world.place_block(BlockPos::new(17, 64, 1), 2);

    let chunk_zero = world.blocks_in_chunk(ChunkPos::new(0, 0));
    let chunk_one = world.blocks_in_chunk(ChunkPos::new(1, 0));

    assert_eq!(chunk_zero.len(), 1);
    assert_eq!(chunk_zero[0], (BlockPos::new(1, 64, 1), 1));
    assert_eq!(chunk_one.len(), 1);
    assert_eq!(chunk_one[0], (BlockPos::new(17, 64, 1), 2));
}

#[test]
fn unload_chunk_removes_its_blocks() {
    let mut world = BlockWorld::new();
    let target = BlockPos::new(-2, 64, -2);
    world.place_block(target, 5);

    let chunk = ChunkPos::from_block(target);
    assert_eq!(world.block_id(target), 5);

    world.unload_chunk(chunk);

    assert_eq!(world.blocks_in_chunk(chunk).len(), 0);
    assert_eq!(world.block_id(target), 0);
}

#[test]
fn chunk_block_count_tracks_per_chunk_population() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 0), 1);
    world.place_block(BlockPos::new(15, 65, 15), 2);
    world.place_block(BlockPos::new(16, 64, 0), 3);

    assert_eq!(world.chunk_block_count(ChunkPos::new(0, 0)), 2);
    assert_eq!(world.chunk_block_count(ChunkPos::new(1, 0)), 1);
    assert_eq!(world.chunk_block_count(ChunkPos::new(2, 0)), 0);
}

#[test]
fn for_each_block_in_chunk_visits_only_target_chunk_blocks() {
    let mut world = BlockWorld::new();
    let a = BlockPos::new(1, 64, 1);
    let b = BlockPos::new(15, 70, 15);
    let c = BlockPos::new(16, 64, 1);
    world.place_block(a, 4);
    world.place_block(b, 5);
    world.place_block(c, 6);

    let mut visited = Vec::new();
    world.for_each_block_in_chunk(ChunkPos::new(0, 0), |position, block_id| {
        visited.push((position, block_id));
    });
    visited.sort_by_key(|(position, _)| (position.y, position.z, position.x));

    assert_eq!(visited, vec![(a, 4), (b, 5)]);
}

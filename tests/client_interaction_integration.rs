use lce_rust::client::interaction::{
    BlockAction, INTERACTION_DISTANCE_BLOCKS, apply_block_action, forward_vector_from_yaw_pitch,
    movement_axes_from_yaw, placement_intersects_player_collider, raycast_first_non_air_block,
    raycast_first_solid_block, target_block_from_direction, target_block_in_front,
    target_chunk_for_block,
};
use lce_rust::world::{BlockPos, BlockWorld, ChunkPos, Vec3, WATER_SOURCE_BLOCK_ID};

#[test]
fn calculates_target_block_in_front_of_player() {
    let target = target_block_in_front(Vec3::new(1.2, 64.9, -3.1));

    assert_eq!(target, BlockPos::new(1, 64, 0));
}

#[test]
fn targets_block_from_camera_direction() {
    let origin = Vec3::new(0.5, 65.0, 0.5);
    let direction = forward_vector_from_yaw_pitch(0.0, 0.0);

    let target = target_block_from_direction(origin, direction, INTERACTION_DISTANCE_BLOCKS as f32);

    assert_eq!(target, BlockPos::new(0, 65, 4));
}

#[test]
fn movement_axes_follow_current_strafe_handedness() {
    let (world_x_left, world_z_left) = movement_axes_from_yaw(0.0, -1.0, 0.0);
    assert!((world_x_left - 1.0).abs() < 0.0001);
    assert!(world_z_left.abs() < 0.0001);

    let ninety_degrees = std::f32::consts::FRAC_PI_2;
    let (world_x_forward, world_z_forward) = movement_axes_from_yaw(ninety_degrees, 0.0, 1.0);
    assert!((world_x_forward + 1.0).abs() < 0.0001);
    assert!(world_z_forward.abs() < 0.0001);
}

#[test]
fn raycast_hits_first_solid_block_and_returns_adjacent_air_cell() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 3), 1);

    let hit = raycast_first_solid_block(
        &world,
        Vec3::new(0.5, 64.5, 0.5),
        Vec3::new(0.0, 0.0, 1.0),
        INTERACTION_DISTANCE_BLOCKS as f32,
    )
    .expect("raycast should hit the block");

    assert_eq!(hit.block, BlockPos::new(0, 64, 3));
    assert_eq!(hit.adjacent_air_block, BlockPos::new(0, 64, 2));
}

#[test]
fn raycast_returns_none_when_no_solid_block_is_in_range() {
    let world = BlockWorld::new();

    let hit = raycast_first_solid_block(
        &world,
        Vec3::new(0.5, 64.5, 0.5),
        Vec3::new(0.0, 0.0, 1.0),
        INTERACTION_DISTANCE_BLOCKS as f32,
    );

    assert!(hit.is_none());
}

#[test]
fn raycast_skips_fluids_and_hits_next_targetable_block() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 2), WATER_SOURCE_BLOCK_ID);
    world.place_block(BlockPos::new(0, 64, 3), 1);

    let hit = raycast_first_solid_block(
        &world,
        Vec3::new(0.5, 64.5, 0.5),
        Vec3::new(0.0, 0.0, 1.0),
        INTERACTION_DISTANCE_BLOCKS as f32,
    )
    .expect("raycast should hit non-fluid block behind water");

    assert_eq!(hit.block, BlockPos::new(0, 64, 3));
    assert_eq!(hit.adjacent_air_block, BlockPos::new(0, 64, 2));
}

#[test]
fn raycast_non_air_hits_fluid_when_it_is_closest_block() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 2), WATER_SOURCE_BLOCK_ID);

    let hit = raycast_first_non_air_block(
        &world,
        Vec3::new(0.5, 64.5, 0.5),
        Vec3::new(0.0, 0.0, 1.0),
        INTERACTION_DISTANCE_BLOCKS as f32,
    )
    .expect("raycast should hit water as non-air block");

    assert_eq!(hit.block, BlockPos::new(0, 64, 2));
}

#[test]
fn place_action_requires_air_and_nonzero_id() {
    let mut world = BlockWorld::new();
    let target = BlockPos::new(0, 64, 2);

    assert!(apply_block_action(
        &mut world,
        target,
        BlockAction::Place { block_id: 1 }
    ));
    assert_eq!(world.block_id(target), 1);

    assert!(!apply_block_action(
        &mut world,
        target,
        BlockAction::Place { block_id: 2 }
    ));
    assert!(!apply_block_action(
        &mut world,
        BlockPos::new(1, 64, 2),
        BlockAction::Place { block_id: 0 }
    ));
}

#[test]
fn place_action_can_replace_fluid_blocks() {
    let mut world = BlockWorld::new();
    let target = BlockPos::new(0, 64, 2);
    world.place_block(target, WATER_SOURCE_BLOCK_ID);

    assert!(apply_block_action(
        &mut world,
        target,
        BlockAction::Place { block_id: 1 }
    ));
    assert_eq!(world.block_id(target), 1);
}

#[test]
fn break_action_returns_false_for_air_and_true_for_blocks() {
    let mut world = BlockWorld::new();
    let target = BlockPos::new(5, 70, 5);

    assert!(!apply_block_action(&mut world, target, BlockAction::Break));

    world.place_block(target, 4);
    assert!(apply_block_action(&mut world, target, BlockAction::Break));
    assert_eq!(world.block_id(target), 0);
}

#[test]
fn break_action_does_not_remove_fluid_blocks() {
    let mut world = BlockWorld::new();
    let target = BlockPos::new(5, 70, 5);
    world.place_block(target, WATER_SOURCE_BLOCK_ID);

    assert!(!apply_block_action(&mut world, target, BlockAction::Break));
    assert_eq!(world.block_id(target), WATER_SOURCE_BLOCK_ID);
}

#[test]
fn target_chunk_mapping_matches_chunk_grid() {
    assert_eq!(
        target_chunk_for_block(BlockPos::new(31, 64, 31)),
        ChunkPos::new(1, 1)
    );
    assert_eq!(
        target_chunk_for_block(BlockPos::new(-1, 64, -1)),
        ChunkPos::new(-1, -1)
    );
}

#[test]
fn placement_collision_check_blocks_player_overlap_and_allows_clear_space() {
    let player_feet = Vec3::new(0.0, 64.0, 0.0);

    assert!(placement_intersects_player_collider(
        player_feet,
        BlockPos::new(0, 64, 0)
    ));
    assert!(placement_intersects_player_collider(
        player_feet,
        BlockPos::new(0, 65, 0)
    ));

    assert!(!placement_intersects_player_collider(
        player_feet,
        BlockPos::new(1, 64, 0)
    ));
    assert!(!placement_intersects_player_collider(
        player_feet,
        BlockPos::new(0, 66, 0)
    ));
}

use std::collections::BTreeSet;

use lce_rust::world::{
    BlockPos, BlockWorld, DEFAULT_GROUND_Y, MovementInput, OfflineGameSession,
    OfflineWorldBootstrap, WATER_SOURCE_BLOCK_ID, is_solid_block_for_player_collision,
};

const TEST_FLOOR_Y: i32 = 63;

#[test]
fn forward_motion_stops_when_hitting_solid_block() {
    let mut game = new_session("CollisionWall", 11);
    let wall = BlockPos::new(0, 64, 1);

    let mut solid_blocks = BTreeSet::new();
    solid_blocks.insert(wall);

    for _ in 0..40 {
        game.tick_with_collision(
            MovementInput {
                strafe: 0.0,
                forward: 1.0,
                jump: false,
                sneak: false,
            },
            |block| block.y <= TEST_FLOOR_Y || solid_blocks.contains(&block),
        );
    }

    let z = game.player().position.z;
    assert!(z > 0.5);
    assert!(z < 0.71);
}

#[test]
fn jump_respects_low_ceiling_collision() {
    let mut game = new_session("CollisionCeiling", 22);
    let ceiling = BlockPos::new(0, 66, 0);

    let mut solid_blocks = BTreeSet::new();
    solid_blocks.insert(ceiling);

    let mut peak = game.player().position.y;
    for tick in 0..12 {
        game.tick_with_collision(
            MovementInput {
                strafe: 0.0,
                forward: 0.0,
                jump: tick == 0,
                sneak: false,
            },
            |block| block.y <= TEST_FLOOR_Y || solid_blocks.contains(&block),
        );

        peak = peak.max(game.player().position.y);
    }

    assert!(peak < 64.25);
}

#[test]
fn fluids_are_non_solid_for_player_collision_probe() {
    let mut game = new_session("CollisionWaterPass", 33);

    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 1), WATER_SOURCE_BLOCK_ID);
    world.place_block(BlockPos::new(0, 64, 2), 1);

    for _ in 0..50 {
        game.tick_with_collision(
            MovementInput {
                strafe: 0.0,
                forward: 1.0,
                jump: false,
                sneak: false,
            },
            |block| {
                block.y <= TEST_FLOOR_Y
                    || is_solid_block_for_player_collision(world.block_id(block))
            },
        );
    }

    let z = game.player().position.z;
    assert!(z > 0.75);
    assert!(z < 1.71);
}

#[test]
fn collision_mode_does_not_force_flat_ground_plane_clamp() {
    let mut game = new_session("CollisionNoGroundClamp", 44);

    for _ in 0..40 {
        game.tick_with_collision(MovementInput::default(), |_| false);
    }

    assert!(
        game.player().position.y < DEFAULT_GROUND_Y - 0.25,
        "expected collision-mode player y below flat ground clamp, got {}",
        game.player().position.y
    );
}

fn new_session(name: &str, seed: i64) -> OfflineGameSession {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world(name, seed)
        .expect("world should be created")
        .clone();

    OfflineGameSession::new(world)
}

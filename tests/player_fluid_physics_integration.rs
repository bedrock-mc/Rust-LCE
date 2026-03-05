use lce_rust::world::{
    BlockPos, BlockWorld, DEFAULT_GROUND_Y, MovementInput, OfflineGameSession,
    OfflineWorldBootstrap, WATER_FLOWING_BLOCK_ID, WATER_SOURCE_BLOCK_ID,
    is_solid_block_for_player_collision,
};

const TEST_FLOOR_Y: i32 = 63;

#[test]
fn swim_jump_moves_player_upward_when_submerged() {
    let mut game = new_session("WaterJump", 7_101);
    let mut world = BlockWorld::new();
    fill_water_strip(&mut world, 0..=2);

    game.tick_with_collision_and_water(
        MovementInput {
            strafe: 0.0,
            forward: 0.0,
            jump: true,
        },
        |block| {
            block.y <= TEST_FLOOR_Y || is_solid_block_for_player_collision(world.block_id(block))
        },
        |block| is_water_block_id(world.block_id(block)),
    );

    assert!(
        game.player().position.y > DEFAULT_GROUND_Y,
        "expected submerged jump to raise player above waterline baseline"
    );
}

#[test]
fn water_drag_reduces_forward_speed_vs_dry_land() {
    let mut dry_game = new_session("DrySpeed", 7_102);
    let mut wet_game = new_session("WetSpeed", 7_103);
    let mut wet_world = BlockWorld::new();
    fill_water_strip(&mut wet_world, 0..=64);

    for _ in 0..20 {
        dry_game.tick_with_collision_and_water(
            MovementInput {
                strafe: 0.0,
                forward: 1.0,
                jump: false,
            },
            |block| block.y <= TEST_FLOOR_Y,
            |_| false,
        );

        wet_game.tick_with_collision_and_water(
            MovementInput {
                strafe: 0.0,
                forward: 1.0,
                jump: false,
            },
            |block| {
                block.y <= TEST_FLOOR_Y
                    || is_solid_block_for_player_collision(wet_world.block_id(block))
            },
            |block| is_water_block_id(wet_world.block_id(block)),
        );
    }

    assert!(
        wet_game.player().position.z < dry_game.player().position.z * 0.75,
        "expected water drag to slow forward movement: dry_z={}, wet_z={}",
        dry_game.player().position.z,
        wet_game.player().position.z,
    );
}

fn fill_water_strip(world: &mut BlockWorld, z_range: std::ops::RangeInclusive<i32>) {
    for z in z_range {
        for y in 64..=65 {
            for x in -1..=1 {
                world.place_block(BlockPos::new(x, y, z), WATER_SOURCE_BLOCK_ID);
            }
        }
    }
}

fn is_water_block_id(block_id: u16) -> bool {
    block_id == WATER_SOURCE_BLOCK_ID || block_id == WATER_FLOWING_BLOCK_ID
}

fn new_session(name: &str, seed: i64) -> OfflineGameSession {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world(name, seed)
        .expect("world should be created")
        .clone();

    OfflineGameSession::new(world)
}

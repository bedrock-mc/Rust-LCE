use crate::world::{
    BlockPos, BlockWorld, ChunkPos, LAVA_FLOWING_BLOCK_ID, LAVA_SOURCE_BLOCK_ID,
    PLAYER_COLLIDER_HALF_WIDTH, PLAYER_COLLIDER_HEIGHT, Vec3, WATER_FLOWING_BLOCK_ID,
    WATER_SOURCE_BLOCK_ID,
};

pub const INTERACTION_DISTANCE_BLOCKS: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockRaycastHit {
    pub block: BlockPos,
    pub adjacent_air_block: BlockPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockAction {
    Place { block_id: u16 },
    Break,
}

pub fn target_block_in_front(position: Vec3) -> BlockPos {
    target_block_from_direction(
        position,
        Vec3::new(0.0, 0.0, 1.0),
        INTERACTION_DISTANCE_BLOCKS as f32,
    )
}

pub fn target_block_from_direction(origin: Vec3, direction: Vec3, distance: f32) -> BlockPos {
    let direction_len =
        (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z).sqrt();

    let normalized = if direction_len <= f32::EPSILON {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(
            direction.x / direction_len,
            direction.y / direction_len,
            direction.z / direction_len,
        )
    };

    let sample = Vec3::new(
        origin.x + normalized.x * distance,
        origin.y + normalized.y * distance,
        origin.z + normalized.z * distance,
    );

    BlockPos::new(
        sample.x.floor() as i32,
        sample.y.floor() as i32,
        sample.z.floor() as i32,
    )
}

pub fn raycast_first_solid_block(
    world: &BlockWorld,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<BlockRaycastHit> {
    raycast_first_matching_block(world, origin, direction, max_distance, |block_id| {
        is_targetable_block(block_id)
    })
}

pub fn raycast_first_non_air_block(
    world: &BlockWorld,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<BlockRaycastHit> {
    raycast_first_matching_block(world, origin, direction, max_distance, |block_id| {
        block_id != 0
    })
}

fn raycast_first_matching_block<F>(
    world: &BlockWorld,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    is_target: F,
) -> Option<BlockRaycastHit>
where
    F: Fn(u16) -> bool,
{
    if max_distance <= 0.0 {
        return None;
    }

    let direction_len =
        (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z).sqrt();
    if direction_len <= f32::EPSILON {
        return None;
    }

    let direction = Vec3::new(
        direction.x / direction_len,
        direction.y / direction_len,
        direction.z / direction_len,
    );

    let mut x = origin.x.floor() as i32;
    let mut y = origin.y.floor() as i32;
    let mut z = origin.z.floor() as i32;
    let mut previous = BlockPos::new(x, y, z);

    let step_x = direction.x.signum() as i32;
    let step_y = direction.y.signum() as i32;
    let step_z = direction.z.signum() as i32;

    let t_delta_x = axis_t_delta(direction.x);
    let t_delta_y = axis_t_delta(direction.y);
    let t_delta_z = axis_t_delta(direction.z);

    let mut t_max_x = axis_t_max(origin.x, x, direction.x, step_x);
    let mut t_max_y = axis_t_max(origin.y, y, direction.y, step_y);
    let mut t_max_z = axis_t_max(origin.z, z, direction.z, step_z);

    loop {
        let current = BlockPos::new(x, y, z);
        if is_target(world.block_id(current)) {
            return Some(BlockRaycastHit {
                block: current,
                adjacent_air_block: previous,
            });
        }

        previous = current;

        let next = t_max_x.min(t_max_y.min(t_max_z));
        if !next.is_finite() || next > max_distance {
            break;
        }

        if t_max_x <= next {
            x += step_x;
            t_max_x += t_delta_x;
        }
        if t_max_y <= next {
            y += step_y;
            t_max_y += t_delta_y;
        }
        if t_max_z <= next {
            z += step_z;
            t_max_z += t_delta_z;
        }
    }

    None
}

pub fn forward_vector_from_yaw_pitch(yaw_radians: f32, pitch_radians: f32) -> Vec3 {
    let yaw_sin = yaw_radians.sin();
    let yaw_cos = yaw_radians.cos();
    let pitch_sin = pitch_radians.sin();
    let pitch_cos = pitch_radians.cos();

    Vec3::new(-yaw_sin * pitch_cos, pitch_sin, yaw_cos * pitch_cos)
}

pub fn movement_axes_from_yaw(
    yaw_radians: f32,
    local_strafe: f32,
    local_forward: f32,
) -> (f32, f32) {
    let length = (local_strafe * local_strafe + local_forward * local_forward).sqrt();
    if length <= f32::EPSILON {
        return (0.0, 0.0);
    }

    let strafe = -local_strafe / length;
    let forward = local_forward / length;

    let yaw_sin = yaw_radians.sin();
    let yaw_cos = yaw_radians.cos();

    let world_x = strafe * yaw_cos - forward * yaw_sin;
    let world_z = strafe * yaw_sin + forward * yaw_cos;
    (world_x, world_z)
}

pub fn target_chunk_for_block(target: BlockPos) -> ChunkPos {
    ChunkPos::from_block(target)
}

pub fn apply_block_action(world: &mut BlockWorld, target: BlockPos, action: BlockAction) -> bool {
    match action {
        BlockAction::Place { block_id } => {
            if block_id == 0 {
                return false;
            }

            let existing = world.block_id(target);
            if existing != 0 && !is_fluid_block(existing) {
                return false;
            }

            world.place_block(target, block_id);
            true
        }
        BlockAction::Break => {
            let block_id = world.block_id(target);
            if block_id == 0 || is_fluid_block(block_id) {
                return false;
            }

            world.break_block(target)
        }
    }
}

pub fn placement_intersects_player_collider(player_feet: Vec3, target: BlockPos) -> bool {
    let player_min_x = player_feet.x - PLAYER_COLLIDER_HALF_WIDTH;
    let player_max_x = player_feet.x + PLAYER_COLLIDER_HALF_WIDTH;
    let player_min_y = player_feet.y;
    let player_max_y = player_feet.y + PLAYER_COLLIDER_HEIGHT;
    let player_min_z = player_feet.z - PLAYER_COLLIDER_HALF_WIDTH;
    let player_max_z = player_feet.z + PLAYER_COLLIDER_HALF_WIDTH;

    let block_min_x = target.x as f32;
    let block_max_x = block_min_x + 1.0;
    let block_min_y = target.y as f32;
    let block_max_y = block_min_y + 1.0;
    let block_min_z = target.z as f32;
    let block_max_z = block_min_z + 1.0;

    ranges_overlap(player_min_x, player_max_x, block_min_x, block_max_x)
        && ranges_overlap(player_min_y, player_max_y, block_min_y, block_max_y)
        && ranges_overlap(player_min_z, player_max_z, block_min_z, block_max_z)
}

fn ranges_overlap(a_min: f32, a_max: f32, b_min: f32, b_max: f32) -> bool {
    a_min < b_max && b_min < a_max
}

fn axis_t_delta(direction_axis: f32) -> f32 {
    if direction_axis.abs() <= f32::EPSILON {
        f32::INFINITY
    } else {
        1.0 / direction_axis.abs()
    }
}

fn axis_t_max(origin_axis: f32, block_axis: i32, direction_axis: f32, step_axis: i32) -> f32 {
    if step_axis > 0 {
        (((block_axis + 1) as f32) - origin_axis) / direction_axis
    } else if step_axis < 0 {
        (origin_axis - (block_axis as f32)) / -direction_axis
    } else {
        f32::INFINITY
    }
}

fn is_targetable_block(block_id: u16) -> bool {
    block_id != 0 && !is_fluid_block(block_id)
}

fn is_fluid_block(block_id: u16) -> bool {
    matches!(
        block_id,
        WATER_SOURCE_BLOCK_ID
            | WATER_FLOWING_BLOCK_ID
            | LAVA_SOURCE_BLOCK_ID
            | LAVA_FLOWING_BLOCK_ID
    )
}

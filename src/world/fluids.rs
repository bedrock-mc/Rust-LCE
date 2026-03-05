use std::collections::BTreeSet;

use crate::world::redstone::{
    LEVER_BLOCK_ID, REDSTONE_TORCH_OFF_BLOCK_ID, REDSTONE_TORCH_ON_BLOCK_ID,
    REDSTONE_WIRE_BLOCK_ID, REDSTONE_WIRE_POWERED_BLOCK_ID, REPEATER_OFF_BLOCK_ID,
    REPEATER_ON_BLOCK_ID, STONE_BUTTON_BLOCK_ID,
};
use crate::world::{BlockPos, BlockWorld, ChunkPos, ScheduledTick, ScheduledTickKind};

pub const WATER_SOURCE_BLOCK_ID: u16 = 8;
pub const WATER_FLOWING_BLOCK_ID: u16 = 9;
pub const LAVA_SOURCE_BLOCK_ID: u16 = 10;
pub const LAVA_FLOWING_BLOCK_ID: u16 = 11;

pub const WATER_TICK_DELAY: u32 = 5;
pub const LAVA_TICK_DELAY: u32 = 30;

const MAX_HORIZONTAL_FLUID_DEPTH: i32 = 8;
const MAX_SLOPE_PASS: i32 = 4;
const DISTANCE_BLOCKED: i32 = 1000;

const WOOD_DOOR_BLOCK_ID: u16 = 64;
const LADDER_BLOCK_ID: u16 = 65;
const WALL_SIGN_BLOCK_ID: u16 = 68;
const IRON_DOOR_BLOCK_ID: u16 = 71;
const REEDS_BLOCK_ID: u16 = 83;
const PORTAL_BLOCK_ID: u16 = 90;
const STANDING_SIGN_BLOCK_ID: u16 = 63;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidKind {
    Water,
    Lava,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluidScheduledTick {
    pub block: BlockPos,
    pub payload_id: u16,
    pub delay_ticks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FluidTickOutcome {
    pub changed_blocks: Vec<BlockPos>,
    pub changed_chunks: BTreeSet<ChunkPos>,
    pub scheduled_ticks: Vec<FluidScheduledTick>,
}

pub fn is_fluid_block(block_id: u16) -> bool {
    fluid_kind_for_block(block_id).is_some()
}

pub fn fluid_kind_for_block(block_id: u16) -> Option<FluidKind> {
    match block_id {
        WATER_SOURCE_BLOCK_ID | WATER_FLOWING_BLOCK_ID => Some(FluidKind::Water),
        LAVA_SOURCE_BLOCK_ID | LAVA_FLOWING_BLOCK_ID => Some(FluidKind::Lava),
        _ => None,
    }
}

pub fn fluid_tick_delay(block_id: u16) -> Option<u32> {
    match block_id {
        WATER_SOURCE_BLOCK_ID => Some(WATER_TICK_DELAY),
        LAVA_SOURCE_BLOCK_ID => Some(LAVA_TICK_DELAY),
        _ => None,
    }
}

pub fn fluid_tick_for_placement(block: BlockPos, block_id: u16) -> Option<FluidScheduledTick> {
    let payload_id = tick_payload_for_block(block_id)?;
    let delay_ticks = fluid_tick_delay(payload_id)?;
    Some(FluidScheduledTick {
        block,
        payload_id,
        delay_ticks,
    })
}

pub fn fluid_ticks_for_block_change(
    world: &BlockWorld,
    changed_block: BlockPos,
    placed_block_id: Option<u16>,
) -> Vec<FluidScheduledTick> {
    let mut scheduled = BTreeSet::new();

    if let Some(placed_block_id) = placed_block_id
        && let Some(payload_id) = tick_payload_for_block(placed_block_id)
    {
        scheduled.insert((changed_block, payload_id));
    }

    for position in cross_neighbors_with_center(changed_block) {
        let block_id = world.block_id(position);
        if let Some(payload_id) = tick_payload_for_block(block_id) {
            scheduled.insert((position, payload_id));
        }
    }

    let mut ticks = Vec::with_capacity(scheduled.len());
    for (block, payload_id) in scheduled {
        let Some(delay_ticks) = fluid_tick_delay(payload_id) else {
            continue;
        };

        ticks.push(FluidScheduledTick {
            block,
            payload_id,
            delay_ticks,
        });
    }

    ticks
}

pub fn process_scheduled_fluid_tick(
    world: &mut BlockWorld,
    tick: ScheduledTick,
) -> Option<FluidTickOutcome> {
    if tick.kind != ScheduledTickKind::Block {
        return None;
    }

    process_fluid_tick_internal(world, tick.block, tick.payload_id, tick.execute_at)
}

pub fn process_fluid_tick(
    world: &mut BlockWorld,
    block: BlockPos,
    payload_id: u16,
) -> Option<FluidTickOutcome> {
    process_fluid_tick_internal(world, block, payload_id, 0)
}

fn process_fluid_tick_internal(
    world: &mut BlockWorld,
    block: BlockPos,
    payload_id: u16,
    world_tick: u64,
) -> Option<FluidTickOutcome> {
    let current_block_id = world.block_id(block);
    let kind =
        fluid_kind_for_block(current_block_id).or_else(|| fluid_kind_for_block(payload_id))?;

    if !is_fluid_block(current_block_id) {
        return None;
    }

    let mut outcome = FluidTickOutcome::default();

    let mut depth = get_depth(world, block, kind);
    if depth < 0 {
        return None;
    }

    let drop_off = drop_off_for_kind(kind);
    let mut become_static = true;

    if depth > 0 {
        let mut max_count = 0;
        let mut highest = -100;
        for neighbor in horizontal_neighbors(block) {
            highest = get_highest(world, neighbor, kind, highest, &mut max_count);
        }

        let mut new_depth = highest + drop_off;
        if new_depth >= MAX_HORIZONTAL_FLUID_DEPTH || highest < 0 {
            new_depth = -1;
        }

        let above = get_depth(world, BlockPos::new(block.x, block.y + 1, block.z), kind);
        if above >= 0 {
            new_depth = if above >= MAX_HORIZONTAL_FLUID_DEPTH {
                above
            } else {
                above + MAX_HORIZONTAL_FLUID_DEPTH
            };
        }

        if max_count >= 2 && kind == FluidKind::Water {
            let below = BlockPos::new(block.x, block.y - 1, block.z);
            let below_kind = fluid_kind_for_block(world.block_id(below));
            if is_water_blocking(world, below)
                || (below_kind == Some(kind) && get_depth(world, below, kind) == 0)
            {
                new_depth = 0;
            }
        }

        if kind == FluidKind::Lava
            && depth < MAX_HORIZONTAL_FLUID_DEPTH
            && new_depth < MAX_HORIZONTAL_FLUID_DEPTH
            && new_depth > depth
            && should_delay_lava_update(block, depth, new_depth, world_tick)
        {
            new_depth = depth;
            become_static = false;
        }

        if new_depth == depth {
            if become_static {
                set_static(world, block, kind, &mut outcome);
            }
        } else {
            depth = new_depth;
            if depth < 0 {
                world.break_block(block);
                record_changed_block(&mut outcome, block);
                schedule_neighbor_fluid_ticks(world, block, &mut outcome);
            }

            if depth >= 0 {
                world.place_block(block, dynamic_block_id(kind));
                world.set_block_data(block, depth as u8);
                record_changed_block(&mut outcome, block);
                schedule_tick_for_position(world, block, &mut outcome);
                schedule_neighbor_fluid_ticks(world, block, &mut outcome);
            }
        }
    } else {
        set_static(world, block, kind, &mut outcome);
    }

    let below = BlockPos::new(block.x, block.y - 1, block.z);
    if can_spread_to(world, below, kind) {
        if kind == FluidKind::Lava
            && fluid_kind_for_block(world.block_id(below)) == Some(FluidKind::Water)
        {
            world.place_block(below, 1);
            world.set_block_data(below, 0);
            record_changed_block(&mut outcome, below);
            schedule_neighbor_fluid_ticks(world, below, &mut outcome);
            return Some(outcome);
        }

        let spread_depth = if depth >= MAX_HORIZONTAL_FLUID_DEPTH {
            depth
        } else {
            depth + MAX_HORIZONTAL_FLUID_DEPTH
        };
        let _ = try_spread_to(world, below, kind, spread_depth, &mut outcome);
    } else if depth >= 0 && (depth == 0 || is_water_blocking(world, below)) {
        let spreads = get_spread(world, block, kind);
        let neighbor_depth = if depth >= MAX_HORIZONTAL_FLUID_DEPTH {
            1
        } else {
            depth + drop_off
        };

        if neighbor_depth < MAX_HORIZONTAL_FLUID_DEPTH {
            let neighbors = horizontal_neighbors(block);
            for (index, neighbor) in neighbors.iter().enumerate() {
                if spreads[index] {
                    let _ = try_spread_to(world, *neighbor, kind, neighbor_depth, &mut outcome);
                }
            }
        }
    }

    Some(outcome)
}

fn set_static(
    world: &mut BlockWorld,
    block: BlockPos,
    kind: FluidKind,
    outcome: &mut FluidTickOutcome,
) {
    let static_id = static_block_id(kind);
    if world.block_id(block) != static_id {
        world.place_block(block, static_id);
        record_changed_block(outcome, block);
    }
}

fn try_spread_to(
    world: &mut BlockWorld,
    target: BlockPos,
    kind: FluidKind,
    depth: i32,
    outcome: &mut FluidTickOutcome,
) -> bool {
    if !can_spread_to(world, target, kind) {
        return false;
    }

    world.place_block(target, dynamic_block_id(kind));
    world.set_block_data(target, depth.clamp(0, 15) as u8);
    record_changed_block(outcome, target);
    schedule_tick_for_position(world, target, outcome);
    schedule_neighbor_fluid_ticks(world, target, outcome);
    true
}

fn get_highest(
    world: &BlockWorld,
    block: BlockPos,
    kind: FluidKind,
    current: i32,
    max_count: &mut i32,
) -> i32 {
    let mut depth = get_depth(world, block, kind);
    if depth < 0 {
        return current;
    }

    if depth == 0 {
        *max_count += 1;
    }

    if depth >= MAX_HORIZONTAL_FLUID_DEPTH {
        depth = 0;
    }

    if current < 0 || depth < current {
        depth
    } else {
        current
    }
}

fn get_spread(world: &BlockWorld, block: BlockPos, kind: FluidKind) -> [bool; 4] {
    let neighbors = horizontal_neighbors(block);
    let mut dist = [DISTANCE_BLOCKED; 4];

    for (index, neighbor) in neighbors.iter().enumerate() {
        if is_water_blocking(world, *neighbor)
            || (fluid_kind_for_block(world.block_id(*neighbor)) == Some(kind)
                && get_depth(world, *neighbor, kind) == 0)
        {
            continue;
        }

        let below = BlockPos::new(neighbor.x, neighbor.y - 1, neighbor.z);
        if is_water_blocking(world, below) {
            dist[index] = get_slope_distance(world, *neighbor, kind, 1, index as i32);
        } else {
            dist[index] = 0;
        }
    }

    let lowest = *dist.iter().min().unwrap_or(&DISTANCE_BLOCKED);
    let mut result = [false; 4];
    for (index, value) in dist.iter().enumerate() {
        result[index] = *value == lowest;
    }
    result
}

fn get_slope_distance(
    world: &BlockWorld,
    block: BlockPos,
    kind: FluidKind,
    pass: i32,
    from: i32,
) -> i32 {
    let mut lowest = DISTANCE_BLOCKED;
    let neighbors = horizontal_neighbors(block);

    for (index, neighbor) in neighbors.iter().enumerate() {
        if is_opposite_direction(index as i32, from) {
            continue;
        }

        if is_water_blocking(world, *neighbor)
            || (fluid_kind_for_block(world.block_id(*neighbor)) == Some(kind)
                && get_depth(world, *neighbor, kind) == 0)
        {
            continue;
        }

        let below = BlockPos::new(neighbor.x, neighbor.y - 1, neighbor.z);
        if is_water_blocking(world, below) {
            if pass < MAX_SLOPE_PASS {
                let candidate = get_slope_distance(world, *neighbor, kind, pass + 1, index as i32);
                if candidate < lowest {
                    lowest = candidate;
                }
            }
        } else {
            return pass;
        }
    }

    lowest
}

fn is_opposite_direction(direction: i32, from: i32) -> bool {
    matches!((direction, from), (0, 1) | (1, 0) | (2, 3) | (3, 2))
}

fn can_spread_to(world: &BlockWorld, block: BlockPos, kind: FluidKind) -> bool {
    let target_id = world.block_id(block);
    let target_kind = fluid_kind_for_block(target_id);

    if target_kind == Some(kind) {
        return false;
    }

    if target_kind == Some(FluidKind::Lava) {
        return false;
    }

    !is_water_blocking(world, block)
}

fn is_water_blocking(world: &BlockWorld, block: BlockPos) -> bool {
    let block_id = world.block_id(block);
    if block_id == 0 {
        return false;
    }

    if matches!(
        block_id,
        WOOD_DOOR_BLOCK_ID
            | IRON_DOOR_BLOCK_ID
            | STANDING_SIGN_BLOCK_ID
            | WALL_SIGN_BLOCK_ID
            | LADDER_BLOCK_ID
            | REEDS_BLOCK_ID
            | PORTAL_BLOCK_ID
    ) {
        return true;
    }

    block_blocks_motion(block_id)
}

fn block_blocks_motion(block_id: u16) -> bool {
    if block_id == 0 || is_fluid_block(block_id) {
        return false;
    }

    !matches!(
        block_id,
        REDSTONE_WIRE_BLOCK_ID
            | REDSTONE_WIRE_POWERED_BLOCK_ID
            | REDSTONE_TORCH_OFF_BLOCK_ID
            | REDSTONE_TORCH_ON_BLOCK_ID
            | REPEATER_OFF_BLOCK_ID
            | REPEATER_ON_BLOCK_ID
            | LEVER_BLOCK_ID
            | STONE_BUTTON_BLOCK_ID
    )
}

fn get_depth(world: &BlockWorld, block: BlockPos, kind: FluidKind) -> i32 {
    if fluid_kind_for_block(world.block_id(block)) != Some(kind) {
        return -1;
    }

    i32::from(world.block_data(block).min(15))
}

fn schedule_neighbor_fluid_ticks(
    world: &BlockWorld,
    block: BlockPos,
    outcome: &mut FluidTickOutcome,
) {
    for position in adjacent_neighbors(block) {
        schedule_tick_for_position(world, position, outcome);
    }
}

fn schedule_tick_for_position(world: &BlockWorld, block: BlockPos, outcome: &mut FluidTickOutcome) {
    let block_id = world.block_id(block);
    let Some(payload_id) = tick_payload_for_block(block_id) else {
        return;
    };

    let Some(delay_ticks) = fluid_tick_delay(payload_id) else {
        return;
    };

    outcome.scheduled_ticks.push(FluidScheduledTick {
        block,
        payload_id,
        delay_ticks,
    });
}

fn tick_payload_for_block(block_id: u16) -> Option<u16> {
    match block_id {
        WATER_SOURCE_BLOCK_ID | WATER_FLOWING_BLOCK_ID => Some(WATER_SOURCE_BLOCK_ID),
        LAVA_SOURCE_BLOCK_ID | LAVA_FLOWING_BLOCK_ID => Some(LAVA_SOURCE_BLOCK_ID),
        _ => None,
    }
}

fn dynamic_block_id(kind: FluidKind) -> u16 {
    match kind {
        FluidKind::Water => WATER_SOURCE_BLOCK_ID,
        FluidKind::Lava => LAVA_SOURCE_BLOCK_ID,
    }
}

fn static_block_id(kind: FluidKind) -> u16 {
    match kind {
        FluidKind::Water => WATER_FLOWING_BLOCK_ID,
        FluidKind::Lava => LAVA_FLOWING_BLOCK_ID,
    }
}

fn drop_off_for_kind(kind: FluidKind) -> i32 {
    match kind {
        FluidKind::Water => 1,
        FluidKind::Lava => 2,
    }
}

fn should_delay_lava_update(block: BlockPos, depth: i32, new_depth: i32, world_tick: u64) -> bool {
    let mut hash = i64::from(block.x)
        .wrapping_mul(3_129_871)
        .wrapping_add(i64::from(block.z).wrapping_mul(116_129_781))
        .wrapping_add(i64::from(block.y).wrapping_mul(423_178_61))
        .wrapping_add(i64::from(depth).wrapping_mul(19_399_663))
        .wrapping_add(i64::from(new_depth).wrapping_mul(83_492_791))
        .wrapping_add(world_tick as i64);
    hash ^= hash >> 13;
    (hash & 3) != 0
}

fn horizontal_neighbors(origin: BlockPos) -> [BlockPos; 4] {
    [
        BlockPos::new(origin.x - 1, origin.y, origin.z),
        BlockPos::new(origin.x + 1, origin.y, origin.z),
        BlockPos::new(origin.x, origin.y, origin.z - 1),
        BlockPos::new(origin.x, origin.y, origin.z + 1),
    ]
}

fn cross_neighbors_with_center(origin: BlockPos) -> [BlockPos; 7] {
    [
        origin,
        BlockPos::new(origin.x - 1, origin.y, origin.z),
        BlockPos::new(origin.x + 1, origin.y, origin.z),
        BlockPos::new(origin.x, origin.y, origin.z - 1),
        BlockPos::new(origin.x, origin.y, origin.z + 1),
        BlockPos::new(origin.x, origin.y - 1, origin.z),
        BlockPos::new(origin.x, origin.y + 1, origin.z),
    ]
}

fn adjacent_neighbors(origin: BlockPos) -> [BlockPos; 6] {
    [
        BlockPos::new(origin.x - 1, origin.y, origin.z),
        BlockPos::new(origin.x + 1, origin.y, origin.z),
        BlockPos::new(origin.x, origin.y, origin.z - 1),
        BlockPos::new(origin.x, origin.y, origin.z + 1),
        BlockPos::new(origin.x, origin.y - 1, origin.z),
        BlockPos::new(origin.x, origin.y + 1, origin.z),
    ]
}

fn record_changed_block(outcome: &mut FluidTickOutcome, block: BlockPos) {
    outcome.changed_blocks.push(block);
    outcome.changed_chunks.insert(ChunkPos::from_block(block));
}

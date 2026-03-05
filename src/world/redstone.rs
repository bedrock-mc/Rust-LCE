use std::collections::BTreeSet;

use crate::world::{BlockPos, BlockWorld, ChunkPos, ScheduledTick, ScheduledTickKind};

pub const REDSTONE_WIRE_BLOCK_ID: u16 = 55;
pub const REDSTONE_WIRE_POWERED_BLOCK_ID: u16 = 1000;
pub const REDSTONE_TORCH_OFF_BLOCK_ID: u16 = 75;
pub const REDSTONE_TORCH_ON_BLOCK_ID: u16 = 76;
pub const REPEATER_OFF_BLOCK_ID: u16 = 93;
pub const REPEATER_ON_BLOCK_ID: u16 = 94;
pub const LEVER_BLOCK_ID: u16 = 69;
pub const STONE_BUTTON_BLOCK_ID: u16 = 77;

pub const REDSTONE_WIRE_TICK_DELAY: u32 = 1;
pub const REDSTONE_TORCH_TICK_DELAY: u32 = 2;
pub const REPEATER_TICK_DELAY: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedstoneComponentKind {
    Wire,
    Torch,
    Repeater,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedstoneScheduledTick {
    pub block: BlockPos,
    pub payload_id: u16,
    pub delay_ticks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RedstoneTickOutcome {
    pub changed_blocks: Vec<BlockPos>,
    pub changed_chunks: BTreeSet<ChunkPos>,
    pub scheduled_ticks: Vec<RedstoneScheduledTick>,
}

pub fn is_redstone_block(block_id: u16) -> bool {
    is_redstone_component(block_id) || is_direct_power_source(block_id)
}

pub fn is_redstone_component(block_id: u16) -> bool {
    redstone_component_kind(block_id).is_some()
}

pub fn redstone_component_kind(block_id: u16) -> Option<RedstoneComponentKind> {
    match block_id {
        REDSTONE_WIRE_BLOCK_ID | REDSTONE_WIRE_POWERED_BLOCK_ID => {
            Some(RedstoneComponentKind::Wire)
        }
        REDSTONE_TORCH_OFF_BLOCK_ID | REDSTONE_TORCH_ON_BLOCK_ID => {
            Some(RedstoneComponentKind::Torch)
        }
        REPEATER_OFF_BLOCK_ID | REPEATER_ON_BLOCK_ID => Some(RedstoneComponentKind::Repeater),
        _ => None,
    }
}

pub fn redstone_tick_delay(block_id: u16) -> Option<u32> {
    let kind = redstone_component_kind(block_id)?;
    Some(tick_delay_for_kind(kind))
}

pub fn redstone_tick_for_placement(
    block: BlockPos,
    block_id: u16,
) -> Option<RedstoneScheduledTick> {
    let delay_ticks = redstone_tick_delay(block_id)?;
    Some(RedstoneScheduledTick {
        block,
        payload_id: block_id,
        delay_ticks,
    })
}

pub fn redstone_ticks_for_block_change(
    world: &BlockWorld,
    changed_block: BlockPos,
    placed_block_id: Option<u16>,
) -> Vec<RedstoneScheduledTick> {
    let mut queued = BTreeSet::new();

    if let Some(block_id) = placed_block_id {
        queue_component_tick(&mut queued, changed_block, block_id);
    }

    for neighbor in adjacent_blocks(changed_block) {
        let neighbor_block_id = world.block_id(neighbor);
        queue_component_tick(&mut queued, neighbor, neighbor_block_id);
    }

    queued
        .into_iter()
        .map(|(block, payload_id, delay_ticks)| RedstoneScheduledTick {
            block,
            payload_id,
            delay_ticks,
        })
        .collect()
}

pub fn process_scheduled_redstone_tick(
    world: &mut BlockWorld,
    tick: ScheduledTick,
) -> Option<RedstoneTickOutcome> {
    if tick.kind != ScheduledTickKind::Block {
        return None;
    }

    process_redstone_tick(world, tick.block, tick.payload_id)
}

pub fn process_redstone_tick(
    world: &mut BlockWorld,
    block: BlockPos,
    payload_id: u16,
) -> Option<RedstoneTickOutcome> {
    let current_block_id = world.block_id(block);
    let component_id = if is_redstone_component(current_block_id) {
        current_block_id
    } else if is_redstone_component(payload_id) {
        payload_id
    } else {
        return None;
    };

    let kind = redstone_component_kind(component_id)?;
    let target_block_id = match kind {
        RedstoneComponentKind::Wire => {
            if wire_network_has_direct_source(world, block) {
                REDSTONE_WIRE_POWERED_BLOCK_ID
            } else {
                REDSTONE_WIRE_BLOCK_ID
            }
        }
        RedstoneComponentKind::Torch => {
            if has_adjacent_emitted_power(world, block) {
                REDSTONE_TORCH_OFF_BLOCK_ID
            } else {
                REDSTONE_TORCH_ON_BLOCK_ID
            }
        }
        RedstoneComponentKind::Repeater => {
            if has_adjacent_emitted_power(world, block) {
                REPEATER_ON_BLOCK_ID
            } else {
                REPEATER_OFF_BLOCK_ID
            }
        }
    };

    let mut outcome = RedstoneTickOutcome::default();
    if current_block_id != target_block_id {
        world.place_block(block, target_block_id);
        record_changed_block(&mut outcome, block);

        let scheduled = redstone_ticks_for_block_change(world, block, Some(target_block_id));
        outcome.scheduled_ticks.extend(scheduled);
    }

    if outcome.changed_blocks.is_empty() && outcome.scheduled_ticks.is_empty() {
        None
    } else {
        Some(outcome)
    }
}

fn queue_component_tick(
    queued: &mut BTreeSet<(BlockPos, u16, u32)>,
    block: BlockPos,
    block_id: u16,
) {
    let Some(delay_ticks) = redstone_tick_delay(block_id) else {
        return;
    };

    queued.insert((block, block_id, delay_ticks));
}

fn tick_delay_for_kind(kind: RedstoneComponentKind) -> u32 {
    match kind {
        RedstoneComponentKind::Wire => REDSTONE_WIRE_TICK_DELAY,
        RedstoneComponentKind::Torch => REDSTONE_TORCH_TICK_DELAY,
        RedstoneComponentKind::Repeater => REPEATER_TICK_DELAY,
    }
}

fn wire_network_has_direct_source(world: &BlockWorld, start: BlockPos) -> bool {
    let mut open = vec![start];
    let mut visited = BTreeSet::new();

    while let Some(block) = open.pop() {
        if !visited.insert(block) {
            continue;
        }

        for neighbor in adjacent_blocks(block) {
            let neighbor_id = world.block_id(neighbor);
            if is_wire_block(neighbor_id) {
                open.push(neighbor);
                continue;
            }

            if is_direct_power_source(neighbor_id) {
                return true;
            }
        }
    }

    false
}

fn has_adjacent_emitted_power(world: &BlockWorld, block: BlockPos) -> bool {
    adjacent_blocks(block)
        .into_iter()
        .map(|neighbor| world.block_id(neighbor))
        .any(is_emitted_power_source)
}

fn is_wire_block(block_id: u16) -> bool {
    matches!(
        block_id,
        REDSTONE_WIRE_BLOCK_ID | REDSTONE_WIRE_POWERED_BLOCK_ID
    )
}

fn is_direct_power_source(block_id: u16) -> bool {
    matches!(
        block_id,
        LEVER_BLOCK_ID | STONE_BUTTON_BLOCK_ID | REDSTONE_TORCH_ON_BLOCK_ID
    )
}

fn is_emitted_power_source(block_id: u16) -> bool {
    is_direct_power_source(block_id)
        || matches!(
            block_id,
            REDSTONE_WIRE_POWERED_BLOCK_ID | REPEATER_ON_BLOCK_ID
        )
}

fn adjacent_blocks(origin: BlockPos) -> [BlockPos; 6] {
    [
        BlockPos::new(origin.x + 1, origin.y, origin.z),
        BlockPos::new(origin.x - 1, origin.y, origin.z),
        BlockPos::new(origin.x, origin.y + 1, origin.z),
        BlockPos::new(origin.x, origin.y - 1, origin.z),
        BlockPos::new(origin.x, origin.y, origin.z + 1),
        BlockPos::new(origin.x, origin.y, origin.z - 1),
    ]
}

fn record_changed_block(outcome: &mut RedstoneTickOutcome, block: BlockPos) {
    outcome.changed_blocks.push(block);
    outcome.changed_chunks.insert(ChunkPos::from_block(block));
}

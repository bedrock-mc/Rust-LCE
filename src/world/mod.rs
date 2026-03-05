pub mod blocks;
pub mod crafting;
pub mod entities;
pub mod fluids;
pub mod inventory;
pub mod item_use;
pub mod lifecycle;
pub mod redstone;
pub mod simulation;
pub mod worldgen;

pub use blocks::{BlockPos, BlockWorld, ChunkLoadOutcome, ChunkPos};
pub use crafting::{CraftOutcome, CraftRecipe, RECIPES, craft_recipe, recipe_by_id};
pub use entities::{
    DEFAULT_MOB_MAX_HEALTH, EntityId, EntityKind, EntityState, EntityWorld,
    MOB_WALK_SPEED_BLOCKS_PER_SECOND, MobKind,
};
pub use fluids::{
    FluidKind, FluidScheduledTick, FluidTickOutcome, LAVA_FLOWING_BLOCK_ID, LAVA_SOURCE_BLOCK_ID,
    LAVA_TICK_DELAY, WATER_FLOWING_BLOCK_ID, WATER_SOURCE_BLOCK_ID, WATER_TICK_DELAY,
    fluid_kind_for_block, fluid_tick_delay, fluid_tick_for_placement, fluid_ticks_for_block_change,
    is_fluid_block, process_fluid_tick, process_scheduled_fluid_tick,
};
pub use inventory::{HOTBAR_SLOTS, INVENTORY_SLOTS, InventoryError, ItemStack, PlayerInventory};
pub use item_use::{block_id_for_item, use_selected_item_for_placement};
pub use lifecycle::{
    ChunkLifecycleController, ChunkLifecycleEvent, DAY_LENGTH_TICKS, ScheduledTick,
    ScheduledTickKind, TimeState, WeatherKind, WeatherState,
};
pub use redstone::{
    LEVER_BLOCK_ID, REDSTONE_TORCH_OFF_BLOCK_ID, REDSTONE_TORCH_ON_BLOCK_ID,
    REDSTONE_TORCH_TICK_DELAY, REDSTONE_WIRE_BLOCK_ID, REDSTONE_WIRE_POWERED_BLOCK_ID,
    REDSTONE_WIRE_TICK_DELAY, REPEATER_OFF_BLOCK_ID, REPEATER_ON_BLOCK_ID, REPEATER_TICK_DELAY,
    RedstoneComponentKind, RedstoneScheduledTick, RedstoneTickOutcome, STONE_BUTTON_BLOCK_ID,
    is_redstone_block, is_redstone_component, process_redstone_tick,
    process_scheduled_redstone_tick, redstone_component_kind, redstone_tick_delay,
    redstone_tick_for_placement, redstone_ticks_for_block_change,
};
pub use simulation::{
    DEFAULT_GROUND_Y, MovementInput, OfflineGameSession, PLAYER_COLLIDER_HALF_WIDTH,
    PLAYER_COLLIDER_HEIGHT, PLAYER_MAX_HEALTH, PlayerState, SPRINT_SPEED_MULTIPLIER, Vec3,
    WALK_SPEED_BLOCKS_PER_SECOND, is_solid_block_for_player_collision,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSession {
    pub name: String,
    pub seed: i64,
    pub tick_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSnapshot {
    pub name: String,
    pub seed: i64,
    pub tick_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldBootstrapError {
    MissingActiveWorld,
    InvalidWorldName,
}

#[derive(Default)]
pub struct OfflineWorldBootstrap {
    active_world: Option<WorldSession>,
}

impl OfflineWorldBootstrap {
    pub fn new() -> Self {
        Self { active_world: None }
    }

    pub fn create_world(
        &mut self,
        name: impl Into<String>,
        seed: i64,
    ) -> Result<&WorldSession, WorldBootstrapError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(WorldBootstrapError::InvalidWorldName);
        }

        self.active_world = Some(WorldSession {
            name,
            seed,
            tick_count: 0,
        });

        Ok(self.active_world.as_ref().expect("world was just created"))
    }

    pub fn load_world(&mut self, snapshot: WorldSnapshot) -> &WorldSession {
        self.active_world = Some(WorldSession {
            name: snapshot.name,
            seed: snapshot.seed,
            tick_count: snapshot.tick_count,
        });

        self.active_world.as_ref().expect("world was just loaded")
    }

    pub fn tick_active_world(&mut self, ticks: u32) -> Result<(), WorldBootstrapError> {
        let world = self
            .active_world
            .as_mut()
            .ok_or(WorldBootstrapError::MissingActiveWorld)?;

        world.tick_count = world.tick_count.saturating_add(u64::from(ticks));
        Ok(())
    }

    pub fn save_active_world(&self) -> Result<WorldSnapshot, WorldBootstrapError> {
        let world = self
            .active_world
            .as_ref()
            .ok_or(WorldBootstrapError::MissingActiveWorld)?;

        Ok(WorldSnapshot {
            name: world.name.clone(),
            seed: world.seed,
            tick_count: world.tick_count,
        })
    }

    pub fn active_world(&self) -> Option<&WorldSession> {
        self.active_world.as_ref()
    }
}

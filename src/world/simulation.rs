use std::time::Duration;

use crate::core::timing::tick_duration;
use crate::world::entities::{EntityId, EntityWorld, MobKind};
use crate::world::fluids::{
    LAVA_FLOWING_BLOCK_ID, LAVA_SOURCE_BLOCK_ID, WATER_FLOWING_BLOCK_ID, WATER_SOURCE_BLOCK_ID,
};
use crate::world::redstone::{
    LEVER_BLOCK_ID, REDSTONE_TORCH_OFF_BLOCK_ID, REDSTONE_TORCH_ON_BLOCK_ID,
    REDSTONE_WIRE_BLOCK_ID, REDSTONE_WIRE_POWERED_BLOCK_ID, REPEATER_OFF_BLOCK_ID,
    REPEATER_ON_BLOCK_ID, STONE_BUTTON_BLOCK_ID,
};
use crate::world::{BlockPos, PlayerInventory, WorldSession, WorldSnapshot};

pub const DEFAULT_GROUND_Y: f32 = 64.0;
pub const WALK_SPEED_BLOCKS_PER_SECOND: f32 = 4.317;
pub const JUMP_VELOCITY_BLOCKS_PER_SECOND: f32 = 5.0;
pub const GRAVITY_BLOCKS_PER_SECOND_SQUARED: f32 = 14.7;
pub const TICK_SECONDS: f32 = 0.05;
pub const PLAYER_MAX_HEALTH: i16 = 20;
pub const PLAYER_JUMP_VELOCITY_BLOCKS_PER_SECOND: f32 = 0.42 / TICK_SECONDS;
pub const FLY_VERTICAL_SPEED_BLOCKS_PER_SECOND: f32 = WALK_SPEED_BLOCKS_PER_SECOND;
pub const PLAYER_COLLIDER_HALF_WIDTH: f32 = 0.3;
pub const PLAYER_COLLIDER_HEIGHT: f32 = 1.8;
pub const SPRINT_SPEED_MULTIPLIER: f32 = 1.3;
pub const SPRINT_DURATION_TICKS: u16 = 20 * 30;

const COLLISION_EPSILON: f32 = 0.001;
const COLLISION_SOLVER_STEPS: u8 = 10;
const FLY_TAP_WINDOW_TICKS: u8 = 10;
const PLAYER_GRAVITY_VELOCITY_DELTA_PER_TICK: f32 = 0.08 / TICK_SECONDS;
const PLAYER_AIR_DRAG_PER_TICK: f32 = 0.98;
const WATER_GRAVITY_VELOCITY_DELTA_PER_TICK: f32 = 0.02 / TICK_SECONDS;
const WATER_SWIM_UP_VELOCITY_DELTA_PER_TICK: f32 = 0.04 / TICK_SECONDS;
const WATER_DRAG_PER_TICK: f32 = 0.8;
const WATER_MOVE_SPEED_MULTIPLIER: f32 = 0.35;
const WATER_MAX_UPWARD_VELOCITY_PER_TICK: f32 = 0.3 / TICK_SECONDS;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MovementInput {
    pub strafe: f32,
    pub forward: f32,
    pub jump: bool,
}

impl MovementInput {
    fn normalized_axis(self) -> (f32, f32) {
        let length = (self.strafe * self.strafe + self.forward * self.forward).sqrt();
        if length <= f32::EPSILON {
            (0.0, 0.0)
        } else {
            (self.strafe / length, self.forward / length)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub allow_flight: bool,
    pub is_flying: bool,
    pub is_sprinting: bool,
    pub inventory: PlayerInventory,
    pub health: i16,
    pub max_health: i16,
    pub is_dead: bool,
}

impl PlayerState {
    pub fn at_spawn() -> Self {
        Self {
            position: Vec3::new(0.0, DEFAULT_GROUND_Y, 0.0),
            velocity: Vec3::ZERO,
            on_ground: true,
            allow_flight: true,
            is_flying: false,
            is_sprinting: false,
            inventory: PlayerInventory::new(),
            health: PLAYER_MAX_HEALTH,
            max_health: PLAYER_MAX_HEALTH,
            is_dead: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OfflineGameSession {
    world: WorldSession,
    player: PlayerState,
    ground_y: f32,
    entities: EntityWorld,
    jump_trigger_time: u8,
    sprint_time: u16,
    two_jumps_registered: bool,
    jump_tap_latched: bool,
    jump_was_pressed: bool,
}

impl OfflineGameSession {
    pub fn new(world: WorldSession) -> Self {
        Self::with_ground(world, DEFAULT_GROUND_Y)
    }

    pub fn with_ground(world: WorldSession, ground_y: f32) -> Self {
        let mut player = PlayerState::at_spawn();
        player.position.y = ground_y;

        let mut entities = EntityWorld::new(world.seed);
        entities.spawn_mob(MobKind::Pig, Vec3::new(4.0, ground_y, 4.0));

        Self {
            world,
            player,
            ground_y,
            entities,
            jump_trigger_time: 0,
            sprint_time: 0,
            two_jumps_registered: false,
            jump_tap_latched: false,
            jump_was_pressed: false,
        }
    }

    pub fn world(&self) -> &WorldSession {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut WorldSession {
        &mut self.world
    }

    pub fn player(&self) -> &PlayerState {
        &self.player
    }

    pub fn world_snapshot(&self) -> WorldSnapshot {
        WorldSnapshot {
            name: self.world.name.clone(),
            seed: self.world.seed,
            tick_count: self.world.tick_count,
        }
    }

    pub fn entities(&self) -> &EntityWorld {
        &self.entities
    }

    pub fn entities_mut(&mut self) -> &mut EntityWorld {
        &mut self.entities
    }

    pub fn spawn_mob(&mut self, kind: MobKind, position: Vec3) -> EntityId {
        self.entities.spawn_mob(kind, position)
    }

    pub fn apply_entity_damage(&mut self, entity_id: EntityId, amount: i16) -> bool {
        self.entities.apply_damage(entity_id, amount)
    }

    pub fn player_mut(&mut self) -> &mut PlayerState {
        &mut self.player
    }

    pub fn set_player_allow_flight(&mut self, allow_flight: bool) {
        self.player.allow_flight = allow_flight;
        if !allow_flight {
            self.player.is_flying = false;
            self.player.is_sprinting = false;
            self.jump_trigger_time = 0;
            self.sprint_time = 0;
            self.two_jumps_registered = false;
        }
    }

    pub fn set_player_sprinting(&mut self, sprinting: bool) {
        if !sprinting {
            self.player.is_sprinting = false;
            self.sprint_time = 0;
            return;
        }

        self.player.is_sprinting = true;
        self.sprint_time = SPRINT_DURATION_TICKS;
    }

    pub fn player_allows_flight(&self) -> bool {
        self.player.allow_flight
    }

    pub fn register_jump_tap(&mut self) {
        self.jump_tap_latched = true;
    }

    pub fn tick_duration() -> Duration {
        tick_duration()
    }

    pub fn tick(&mut self, input: MovementInput) {
        self.tick_with_dt(input, Self::tick_duration());
    }

    pub fn tick_with_collision<F>(&mut self, input: MovementInput, is_solid_block: F)
    where
        F: FnMut(BlockPos) -> bool,
    {
        self.tick_with_dt_and_collision_internal(
            input,
            Self::tick_duration(),
            is_solid_block,
            |_| false,
            false,
        );
    }

    pub fn tick_with_collision_and_water<F, W>(
        &mut self,
        input: MovementInput,
        is_solid_block: F,
        is_water_block: W,
    ) where
        F: FnMut(BlockPos) -> bool,
        W: FnMut(BlockPos) -> bool,
    {
        self.tick_with_dt_and_collision_internal(
            input,
            Self::tick_duration(),
            is_solid_block,
            is_water_block,
            false,
        );
    }

    pub fn tick_with_dt(&mut self, input: MovementInput, dt: Duration) {
        self.tick_with_dt_and_collision_internal(input, dt, |_| false, |_| false, true);
    }

    pub fn tick_with_dt_and_collision<F>(
        &mut self,
        input: MovementInput,
        dt: Duration,
        is_solid_block: F,
    ) where
        F: FnMut(BlockPos) -> bool,
    {
        self.tick_with_dt_and_collision_internal(input, dt, is_solid_block, |_| false, false);
    }

    fn tick_with_dt_and_collision_internal<F, W>(
        &mut self,
        input: MovementInput,
        dt: Duration,
        mut is_solid_block: F,
        mut is_water_block: W,
        enforce_ground_plane: bool,
    ) where
        F: FnMut(BlockPos) -> bool,
        W: FnMut(BlockPos) -> bool,
    {
        let dt_seconds = dt.as_secs_f32();
        if dt_seconds <= 0.0 {
            return;
        }

        let jump_pressed = input.jump;
        let jump_pressed_edge = jump_pressed && !self.jump_was_pressed;
        let jump_tap = self.jump_tap_latched || jump_pressed_edge;
        self.jump_tap_latched = false;

        if self.jump_trigger_time > 0 {
            self.jump_trigger_time = self.jump_trigger_time.saturating_sub(1);
        }

        if self.sprint_time > 0 {
            self.sprint_time = self.sprint_time.saturating_sub(1);
            if self.sprint_time == 0 {
                self.player.is_sprinting = false;
            }
        }

        if self.player.is_dead {
            self.player.is_sprinting = false;
            self.sprint_time = 0;
            self.entities
                .tick_mobs(self.world.tick_count, self.ground_y);
            self.world.tick_count = self.world.tick_count.saturating_add(1);
            self.jump_was_pressed = jump_pressed;
            return;
        }

        if self.player.allow_flight {
            if jump_tap {
                if self.jump_trigger_time == 0 {
                    self.jump_trigger_time = FLY_TAP_WINDOW_TICKS;
                    self.two_jumps_registered = false;
                } else {
                    self.two_jumps_registered = true;
                }
            } else if !jump_pressed && self.jump_trigger_time > 0 && self.two_jumps_registered {
                self.player.is_flying = !self.player.is_flying;
                self.player.on_ground = false;
                self.player.velocity.y = 0.0;
                self.jump_trigger_time = 0;
                self.two_jumps_registered = false;
            }
        } else {
            self.player.is_flying = false;
            self.player.is_sprinting = false;
            self.jump_trigger_time = 0;
            self.sprint_time = 0;
            self.two_jumps_registered = false;
        }

        let sprint_multiplier = if self.player.is_sprinting {
            SPRINT_SPEED_MULTIPLIER
        } else {
            1.0
        };

        let in_water = !self.player.is_flying
            && player_intersects_water(self.player.position, &mut is_water_block);
        let (strafe, forward) = input.normalized_axis();
        let mut horizontal_speed = WALK_SPEED_BLOCKS_PER_SECOND * sprint_multiplier;
        if in_water {
            horizontal_speed *= WATER_MOVE_SPEED_MULTIPLIER;
        }
        self.player.velocity.x = strafe * horizontal_speed;
        self.player.velocity.z = forward * horizontal_speed;

        if self.player.allow_flight && self.player.is_flying {
            self.player.velocity.y = if jump_pressed {
                FLY_VERTICAL_SPEED_BLOCKS_PER_SECOND * sprint_multiplier
            } else {
                0.0
            };
            self.player.on_ground = false;
        } else if in_water {
            if jump_pressed {
                let tick_scale = (dt_seconds / TICK_SECONDS).max(0.0);
                self.player.velocity.y += WATER_SWIM_UP_VELOCITY_DELTA_PER_TICK * tick_scale;
                self.player.velocity.y = self
                    .player
                    .velocity
                    .y
                    .min(WATER_MAX_UPWARD_VELOCITY_PER_TICK);
            }
        } else {
            if jump_pressed && self.player.on_ground {
                self.player.velocity.y = PLAYER_JUMP_VELOCITY_BLOCKS_PER_SECOND;
                self.player.on_ground = false;
            }
        }

        let x_delta = self.player.velocity.x * dt_seconds;
        let y_delta = self.player.velocity.y * dt_seconds;
        let z_delta = self.player.velocity.z * dt_seconds;

        let (_, hit_x) = resolve_player_axis_movement(
            &mut self.player.position,
            CollisionAxis::X,
            x_delta,
            &mut is_solid_block,
        );
        if hit_x {
            self.player.velocity.x = 0.0;
        }

        let (_, hit_y) = resolve_player_axis_movement(
            &mut self.player.position,
            CollisionAxis::Y,
            y_delta,
            &mut is_solid_block,
        );
        if hit_y {
            self.player.velocity.y = 0.0;
        }

        let (_, hit_z) = resolve_player_axis_movement(
            &mut self.player.position,
            CollisionAxis::Z,
            z_delta,
            &mut is_solid_block,
        );
        if hit_z {
            self.player.velocity.z = 0.0;
        }

        if self.player.is_flying {
            self.player.on_ground = false;
        } else {
            let touching_ground = hit_y && y_delta < 0.0
                || has_supporting_collision_below(self.player.position, &mut is_solid_block);
            self.player.on_ground = touching_ground;
        }

        if enforce_ground_plane && self.player.position.y <= self.ground_y {
            self.player.position.y = self.ground_y;
            if self.player.is_flying {
                self.player.velocity.y = 0.0;
                self.player.on_ground = false;
            } else {
                self.player.velocity.y = 0.0;
                self.player.on_ground = true;
            }
        }

        if !self.player.is_flying && !self.player.on_ground {
            let tick_scale = (dt_seconds / TICK_SECONDS).max(0.0);
            if in_water {
                self.player.velocity.y = (self.player.velocity.y
                    - WATER_GRAVITY_VELOCITY_DELTA_PER_TICK * tick_scale)
                    * WATER_DRAG_PER_TICK.powf(tick_scale);
            } else {
                self.player.velocity.y = (self.player.velocity.y
                    - PLAYER_GRAVITY_VELOCITY_DELTA_PER_TICK * tick_scale)
                    * PLAYER_AIR_DRAG_PER_TICK.powf(tick_scale);
            }
        }

        self.entities
            .tick_mobs(self.world.tick_count, self.ground_y);

        self.world.tick_count = self.world.tick_count.saturating_add(1);
        self.jump_was_pressed = jump_pressed;
    }

    pub fn apply_player_damage(&mut self, amount: i16) -> bool {
        if amount <= 0 || self.player.is_dead {
            return false;
        }

        self.player.health = (self.player.health - amount).max(0);
        if self.player.health == 0 {
            self.player.is_dead = true;
            self.player.velocity = Vec3::ZERO;
            self.player.is_flying = false;
            self.player.is_sprinting = false;
            self.sprint_time = 0;
            self.jump_trigger_time = 0;
            self.two_jumps_registered = false;
            self.jump_tap_latched = false;
            return true;
        }

        false
    }

    pub fn heal_player(&mut self, amount: i16) {
        if amount <= 0 || self.player.is_dead {
            return;
        }

        self.player.health = (self.player.health + amount).min(self.player.max_health);
    }

    pub fn respawn_player(&mut self) {
        self.player.position = Vec3::new(0.0, self.ground_y, 0.0);
        self.player.velocity = Vec3::ZERO;
        self.player.on_ground = true;
        self.player.is_flying = false;
        self.player.is_sprinting = false;
        self.sprint_time = 0;
        self.player.health = self.player.max_health;
        self.player.is_dead = false;
        self.jump_trigger_time = 0;
        self.two_jumps_registered = false;
        self.jump_tap_latched = false;
        self.jump_was_pressed = false;
    }
}

#[derive(Clone, Copy)]
enum CollisionAxis {
    X,
    Y,
    Z,
}

fn resolve_player_axis_movement<F>(
    position: &mut Vec3,
    axis: CollisionAxis,
    delta: f32,
    is_solid_block: &mut F,
) -> (f32, bool)
where
    F: FnMut(BlockPos) -> bool,
{
    if delta.abs() <= f32::EPSILON {
        return (0.0, false);
    }

    let mut candidate = *position;
    add_axis(&mut candidate, axis, delta);
    if !player_collides_with_world(candidate, is_solid_block) {
        *position = candidate;
        return (delta, false);
    }

    let sign = delta.signum();
    let mut min_clear = 0.0;
    let mut max_blocked = delta.abs();

    for _ in 0..COLLISION_SOLVER_STEPS {
        let test = (min_clear + max_blocked) * 0.5;
        let mut step_candidate = *position;
        add_axis(&mut step_candidate, axis, sign * test);

        if player_collides_with_world(step_candidate, is_solid_block) {
            max_blocked = test;
        } else {
            min_clear = test;
        }
    }

    if min_clear > COLLISION_EPSILON {
        add_axis(position, axis, sign * min_clear);
    }

    (sign * min_clear, true)
}

fn player_collides_with_world<F>(position: Vec3, is_solid_block: &mut F) -> bool
where
    F: FnMut(BlockPos) -> bool,
{
    let min_x = position.x - PLAYER_COLLIDER_HALF_WIDTH + COLLISION_EPSILON;
    let max_x = position.x + PLAYER_COLLIDER_HALF_WIDTH - COLLISION_EPSILON;
    let min_y = position.y + COLLISION_EPSILON;
    let max_y = position.y + PLAYER_COLLIDER_HEIGHT - COLLISION_EPSILON;
    let min_z = position.z - PLAYER_COLLIDER_HALF_WIDTH + COLLISION_EPSILON;
    let max_z = position.z + PLAYER_COLLIDER_HALF_WIDTH - COLLISION_EPSILON;

    let (x_start, x_end) = block_range(min_x, max_x);
    let (y_start, y_end) = block_range(min_y, max_y);
    let (z_start, z_end) = block_range(min_z, max_z);

    for x in x_start..=x_end {
        for y in y_start..=y_end {
            for z in z_start..=z_end {
                if is_solid_block(BlockPos::new(x, y, z)) {
                    return true;
                }
            }
        }
    }

    false
}

fn player_intersects_water<F>(position: Vec3, is_water_block: &mut F) -> bool
where
    F: FnMut(BlockPos) -> bool,
{
    let min_x = position.x - PLAYER_COLLIDER_HALF_WIDTH + COLLISION_EPSILON;
    let max_x = position.x + PLAYER_COLLIDER_HALF_WIDTH - COLLISION_EPSILON;
    let min_y = position.y + COLLISION_EPSILON;
    let max_y = position.y + PLAYER_COLLIDER_HEIGHT - COLLISION_EPSILON;
    let min_z = position.z - PLAYER_COLLIDER_HALF_WIDTH + COLLISION_EPSILON;
    let max_z = position.z + PLAYER_COLLIDER_HALF_WIDTH - COLLISION_EPSILON;

    let (x_start, x_end) = block_range(min_x, max_x);
    let (y_start, y_end) = block_range(min_y, max_y);
    let (z_start, z_end) = block_range(min_z, max_z);

    for x in x_start..=x_end {
        for y in y_start..=y_end {
            for z in z_start..=z_end {
                if is_water_block(BlockPos::new(x, y, z)) {
                    return true;
                }
            }
        }
    }

    false
}

fn has_supporting_collision_below<F>(position: Vec3, is_solid_block: &mut F) -> bool
where
    F: FnMut(BlockPos) -> bool,
{
    let probe_position = Vec3::new(position.x, position.y - COLLISION_EPSILON, position.z);
    player_collides_with_world(probe_position, is_solid_block)
}

fn block_range(min: f32, max: f32) -> (i32, i32) {
    let start = min.floor() as i32;
    let end = max.floor() as i32;
    if end < start {
        (start, start)
    } else {
        (start, end)
    }
}

fn add_axis(position: &mut Vec3, axis: CollisionAxis, delta: f32) {
    match axis {
        CollisionAxis::X => position.x += delta,
        CollisionAxis::Y => position.y += delta,
        CollisionAxis::Z => position.z += delta,
    }
}

pub fn is_solid_block_for_player_collision(block_id: u16) -> bool {
    if block_id == 0 {
        return false;
    }

    !matches!(
        block_id,
        WATER_SOURCE_BLOCK_ID
            | WATER_FLOWING_BLOCK_ID
            | LAVA_SOURCE_BLOCK_ID
            | LAVA_FLOWING_BLOCK_ID
            | REDSTONE_WIRE_BLOCK_ID
            | REDSTONE_WIRE_POWERED_BLOCK_ID
            | REDSTONE_TORCH_OFF_BLOCK_ID
            | REDSTONE_TORCH_ON_BLOCK_ID
            | REPEATER_OFF_BLOCK_ID
            | REPEATER_ON_BLOCK_ID
            | LEVER_BLOCK_ID
            | STONE_BUTTON_BLOCK_ID
    )
}

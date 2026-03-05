use std::collections::BTreeMap;

use crate::world::simulation::{
    GRAVITY_BLOCKS_PER_SECOND_SQUARED, JUMP_VELOCITY_BLOCKS_PER_SECOND, TICK_SECONDS, Vec3,
};

pub const MOB_WALK_SPEED_BLOCKS_PER_SECOND: f32 = 1.1;
pub const DEFAULT_MOB_MAX_HEALTH: i16 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(u64);

impl EntityId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobKind {
    Pig,
    Zombie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Mob(MobKind),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityState {
    pub id: EntityId,
    pub kind: EntityKind,
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub health: i16,
    pub max_health: i16,
    pub is_dead: bool,
}

impl EntityState {
    pub fn new_mob(id: EntityId, kind: MobKind, position: Vec3) -> Self {
        Self {
            id,
            kind: EntityKind::Mob(kind),
            position,
            velocity: Vec3::ZERO,
            on_ground: true,
            health: DEFAULT_MOB_MAX_HEALTH,
            max_health: DEFAULT_MOB_MAX_HEALTH,
            is_dead: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityWorld {
    seed: i64,
    next_id: u64,
    entities: BTreeMap<EntityId, EntityState>,
}

impl EntityWorld {
    pub fn new(seed: i64) -> Self {
        Self {
            seed,
            next_id: 1,
            entities: BTreeMap::new(),
        }
    }

    pub fn spawn_mob(&mut self, kind: MobKind, position: Vec3) -> EntityId {
        let id = EntityId::new(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.entities
            .insert(id, EntityState::new_mob(id, kind, position));
        id
    }

    pub fn entity(&self, id: EntityId) -> Option<&EntityState> {
        self.entities.get(&id)
    }

    pub fn entities(&self) -> impl Iterator<Item = &EntityState> {
        self.entities.values()
    }

    pub fn mob_count(&self) -> usize {
        self.entities
            .values()
            .filter(|entity| matches!(entity.kind, EntityKind::Mob(_)) && !entity.is_dead)
            .count()
    }

    pub fn apply_damage(&mut self, id: EntityId, amount: i16) -> bool {
        if amount <= 0 {
            return false;
        }

        let Some(entity) = self.entities.get_mut(&id) else {
            return false;
        };

        if entity.is_dead {
            return false;
        }

        entity.health = (entity.health - amount).max(0);
        if entity.health == 0 {
            entity.is_dead = true;
            entity.velocity = Vec3::ZERO;
            return true;
        }

        false
    }

    pub fn tick_mobs(&mut self, world_tick: u64, ground_y: f32) {
        for entity in self.entities.values_mut() {
            if entity.is_dead {
                continue;
            }

            let random = mix64(self.seed as u64 ^ entity.id.value() ^ world_tick);
            let x_axis = normalize_axis((random & 0xffff) as u16);
            let z_axis = normalize_axis(((random >> 16) & 0xffff) as u16);

            entity.velocity.x = x_axis * MOB_WALK_SPEED_BLOCKS_PER_SECOND;
            entity.velocity.z = z_axis * MOB_WALK_SPEED_BLOCKS_PER_SECOND;

            if entity.on_ground && ((random >> 40) & 0x1f) == 0 {
                entity.velocity.y = JUMP_VELOCITY_BLOCKS_PER_SECOND;
                entity.on_ground = false;
            }

            if !entity.on_ground {
                entity.velocity.y -= GRAVITY_BLOCKS_PER_SECOND_SQUARED * TICK_SECONDS;
            }

            entity.position.x += entity.velocity.x * TICK_SECONDS;
            entity.position.y += entity.velocity.y * TICK_SECONDS;
            entity.position.z += entity.velocity.z * TICK_SECONDS;

            if entity.position.y <= ground_y {
                entity.position.y = ground_y;
                entity.velocity.y = 0.0;
                entity.on_ground = true;
            }
        }
    }
}

fn normalize_axis(value: u16) -> f32 {
    (f32::from(value) / 32767.5) - 1.0
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::world::{BlockPos, ChunkPos};

pub const DAY_LENGTH_TICKS: u64 = 24_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherKind {
    Clear,
    Rain,
    Thunder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeatherState {
    pub kind: WeatherKind,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            kind: WeatherKind::Clear,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeState {
    pub total_ticks: u64,
    pub day_time: u64,
}

impl Default for TimeState {
    fn default() -> Self {
        Self {
            total_ticks: 0,
            day_time: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScheduledTickKind {
    Block,
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScheduledTick {
    pub id: u64,
    pub kind: ScheduledTickKind,
    pub block: BlockPos,
    pub chunk: ChunkPos,
    pub payload_id: u16,
    pub execute_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ScheduledTickLookupKey {
    kind: ScheduledTickKind,
    block: BlockPos,
    payload_id: u16,
}

impl ScheduledTickLookupKey {
    fn from_tick(tick: ScheduledTick) -> Self {
        Self {
            kind: tick.kind,
            block: tick.block,
            payload_id: tick.payload_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkLifecycleEvent {
    ChunkLoaded {
        chunk: ChunkPos,
    },
    ChunkActivated {
        chunk: ChunkPos,
    },
    TimeAdvanced {
        total_ticks: u64,
        day_time: u64,
    },
    ChunkTicked {
        chunk: ChunkPos,
        world_tick: u64,
        chunk_tick_count: u64,
    },
    WeatherChanged {
        from: WeatherKind,
        to: WeatherKind,
    },
    TickScheduled {
        tick: ScheduledTick,
    },
    TickTriggered {
        tick: ScheduledTick,
    },
    ChunkDeactivated {
        chunk: ChunkPos,
    },
    ChunkUnloaded {
        chunk: ChunkPos,
    },
}

#[derive(Debug, Default)]
pub struct ChunkLifecycleController {
    loaded_chunks: BTreeSet<ChunkPos>,
    active_chunks: BTreeSet<ChunkPos>,
    chunk_tick_counts: HashMap<ChunkPos, u64>,
    time: TimeState,
    weather: WeatherState,
    scheduled_ticks: BTreeMap<(u64, u64), ScheduledTick>,
    scheduled_tick_lookup: HashMap<ScheduledTickLookupKey, (u64, u64)>,
    scheduled_tick_keys_by_chunk: HashMap<ChunkPos, BTreeSet<(u64, u64)>>,
    triggered_ticks: Vec<ScheduledTick>,
    next_scheduled_tick_id: u64,
    events: Vec<ChunkLifecycleEvent>,
}

impl ChunkLifecycleController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_total_ticks(total_ticks: u64) -> Self {
        let mut controller = Self::default();
        controller.set_total_ticks(total_ticks);
        controller
    }

    pub fn time(&self) -> TimeState {
        self.time
    }

    pub fn weather(&self) -> WeatherState {
        self.weather
    }

    pub fn set_total_ticks(&mut self, total_ticks: u64) {
        self.time.total_ticks = total_ticks;
        self.time.day_time = total_ticks % DAY_LENGTH_TICKS;
    }

    pub fn loaded_chunks(&self) -> &BTreeSet<ChunkPos> {
        &self.loaded_chunks
    }

    pub fn active_chunks(&self) -> &BTreeSet<ChunkPos> {
        &self.active_chunks
    }

    pub fn chunk_tick_count(&self, chunk: ChunkPos) -> u64 {
        self.chunk_tick_counts.get(&chunk).copied().unwrap_or(0)
    }

    pub fn pending_scheduled_tick_count(&self) -> usize {
        self.scheduled_ticks.len()
    }

    pub fn schedule_block_tick(&mut self, block: BlockPos, block_id: u16, delay_ticks: u32) -> u64 {
        self.schedule_tick(ScheduledTickKind::Block, block, block_id, delay_ticks)
    }

    pub fn schedule_tile_tick(&mut self, block: BlockPos, tile_id: u16, delay_ticks: u32) -> u64 {
        self.schedule_tick(ScheduledTickKind::Tile, block, tile_id, delay_ticks)
    }

    pub fn load_chunk(&mut self, chunk: ChunkPos) -> bool {
        if !self.loaded_chunks.insert(chunk) {
            return false;
        }

        self.events.push(ChunkLifecycleEvent::ChunkLoaded { chunk });
        true
    }

    pub fn unload_chunk(&mut self, chunk: ChunkPos) -> bool {
        let mut changed = false;

        if self.active_chunks.remove(&chunk) {
            self.events
                .push(ChunkLifecycleEvent::ChunkDeactivated { chunk });
            changed = true;
        }

        if self.loaded_chunks.remove(&chunk) {
            self.events
                .push(ChunkLifecycleEvent::ChunkUnloaded { chunk });
            changed = true;
        }

        if changed {
            self.chunk_tick_counts.remove(&chunk);

            if let Some(chunk_keys) = self.scheduled_tick_keys_by_chunk.remove(&chunk) {
                for key in chunk_keys {
                    if let Some(tick) = self.scheduled_ticks.remove(&key) {
                        self.scheduled_tick_lookup
                            .remove(&ScheduledTickLookupKey::from_tick(tick));
                    }
                }
            }

            self.triggered_ticks.retain(|tick| tick.chunk != chunk);
        }

        changed
    }

    pub fn set_chunk_active(&mut self, chunk: ChunkPos, active: bool) -> bool {
        if active {
            if !self.loaded_chunks.contains(&chunk) {
                return false;
            }

            if !self.active_chunks.insert(chunk) {
                return false;
            }

            self.events
                .push(ChunkLifecycleEvent::ChunkActivated { chunk });
            true
        } else if self.active_chunks.remove(&chunk) {
            self.events
                .push(ChunkLifecycleEvent::ChunkDeactivated { chunk });
            true
        } else {
            false
        }
    }

    pub fn set_weather(&mut self, next_weather: WeatherKind) -> bool {
        let previous = self.weather.kind;
        if previous == next_weather {
            return false;
        }

        self.weather.kind = next_weather;
        self.events.push(ChunkLifecycleEvent::WeatherChanged {
            from: previous,
            to: next_weather,
        });
        true
    }

    pub fn tick_once(&mut self) {
        self.time.total_ticks = self.time.total_ticks.saturating_add(1);
        self.time.day_time = self.time.total_ticks % DAY_LENGTH_TICKS;
        self.events.push(ChunkLifecycleEvent::TimeAdvanced {
            total_ticks: self.time.total_ticks,
            day_time: self.time.day_time,
        });

        let active_chunks: Vec<_> = self.active_chunks.iter().copied().collect();
        for chunk in active_chunks {
            let counter = self.chunk_tick_counts.entry(chunk).or_default();
            *counter = counter.saturating_add(1);

            self.events.push(ChunkLifecycleEvent::ChunkTicked {
                chunk,
                world_tick: self.time.total_ticks,
                chunk_tick_count: *counter,
            });
        }

        self.trigger_due_ticks();
    }

    pub fn tick_many(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.tick_once();
        }
    }

    pub fn drain_events(&mut self) -> Vec<ChunkLifecycleEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn drain_triggered_ticks(&mut self) -> Vec<ScheduledTick> {
        std::mem::take(&mut self.triggered_ticks)
    }

    fn schedule_tick(
        &mut self,
        kind: ScheduledTickKind,
        block: BlockPos,
        payload_id: u16,
        delay_ticks: u32,
    ) -> u64 {
        let execute_at = self
            .time
            .total_ticks
            .saturating_add(u64::from(delay_ticks.max(1)));

        let lookup_key = ScheduledTickLookupKey {
            kind,
            block,
            payload_id,
        };

        if let Some(&(existing_execute_at, existing_id)) =
            self.scheduled_tick_lookup.get(&lookup_key)
        {
            if existing_execute_at <= execute_at {
                return existing_id;
            }

            let existing_key = (existing_execute_at, existing_id);
            if let Some(mut rescheduled_tick) = self.scheduled_ticks.remove(&existing_key) {
                self.remove_chunk_tick_key(rescheduled_tick.chunk, existing_key);

                rescheduled_tick.execute_at = execute_at;
                let rescheduled_key = (execute_at, rescheduled_tick.id);
                self.scheduled_ticks
                    .insert(rescheduled_key, rescheduled_tick);
                self.scheduled_tick_lookup
                    .insert(lookup_key, rescheduled_key);
                self.insert_chunk_tick_key(rescheduled_tick.chunk, rescheduled_key);

                self.events.push(ChunkLifecycleEvent::TickScheduled {
                    tick: rescheduled_tick,
                });
                return rescheduled_tick.id;
            }

            self.scheduled_tick_lookup.remove(&lookup_key);
        }

        let id = self.next_scheduled_tick_id;
        self.next_scheduled_tick_id = self.next_scheduled_tick_id.saturating_add(1);

        let tick = ScheduledTick {
            id,
            kind,
            block,
            chunk: ChunkPos::from_block(block),
            payload_id,
            execute_at,
        };

        let scheduled_key = (execute_at, id);
        self.scheduled_ticks.insert(scheduled_key, tick);
        self.scheduled_tick_lookup.insert(lookup_key, scheduled_key);
        self.insert_chunk_tick_key(tick.chunk, scheduled_key);
        self.events
            .push(ChunkLifecycleEvent::TickScheduled { tick });
        id
    }

    fn trigger_due_ticks(&mut self) {
        let now = self.time.total_ticks;

        while let Some((&(execute_at, _), _)) = self.scheduled_ticks.first_key_value() {
            if execute_at > now {
                break;
            }

            let Some((key, tick)) = self.scheduled_ticks.pop_first() else {
                break;
            };

            self.remove_chunk_tick_key(tick.chunk, key);
            self.scheduled_tick_lookup
                .remove(&ScheduledTickLookupKey::from_tick(tick));

            self.events
                .push(ChunkLifecycleEvent::TickTriggered { tick });
            self.triggered_ticks.push(tick);
        }
    }

    fn insert_chunk_tick_key(&mut self, chunk: ChunkPos, key: (u64, u64)) {
        self.scheduled_tick_keys_by_chunk
            .entry(chunk)
            .or_default()
            .insert(key);
    }

    fn remove_chunk_tick_key(&mut self, chunk: ChunkPos, key: (u64, u64)) {
        let mut remove_chunk_slot = false;

        if let Some(keys) = self.scheduled_tick_keys_by_chunk.get_mut(&chunk) {
            keys.remove(&key);
            remove_chunk_slot = keys.is_empty();
        }

        if remove_chunk_slot {
            self.scheduled_tick_keys_by_chunk.remove(&chunk);
        }
    }
}
